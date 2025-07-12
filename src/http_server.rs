use std::{io::{self, BufRead}, sync::Arc};
use chrono::{DateTime, Utc};
use may_minihttp::{HttpServer, HttpService, Request, Response};
use url::Url;
use crate::{model::*, shared_memory::*};

#[derive(Clone)]
pub struct RinhaServer {
    server_default: String,
    server_fallback: String,
    mutex: Arc<MutexWrapper<Vec<RequestsSummary>>>,
    start_time: DateTime<Utc>,
}

impl RinhaServer {
    pub fn new(server_default: String, server_fallback: String, mutex: MutexWrapper<Vec<RequestsSummary>>) -> Result<Self, String> {
        Ok(RinhaServer {
            server_default,
            server_fallback,
            mutex: Arc::new(mutex),
            start_time: Utc::now(),
        })
    }

    pub fn start(self, url: &str) -> Result<may::coroutine::JoinHandle<()>, String> {
        let server = HttpServer(self).start(url)
            .map_err(|e| format!("Error starting server: {:?}", e))?;
        Ok(server)
    }
}

impl HttpService for RinhaServer {
    fn call(&mut self, req: Request, res: &mut Response) -> io::Result<()> {
        match req.method() {
            "GET" => {
                let p = req.path();
                let url = Url::parse(p).unwrap();
                if url.path() == "/payments-summary" {
                    let mut query_pairs = url.query_pairs();
                    let mut from_opt: Option<DateTime<Utc>> = None;
                    let mut to_opt: Option<DateTime<Utc>> = None;
                    while let Some((key, value)) = query_pairs.next() {
                        if key == "from" {
                            if let Ok(date) = DateTime::parse_from_rfc3339(&value) {
                                from_opt = Some(date.with_timezone(&Utc));
                                if to_opt.is_some() {
                                    break;
                                }
                            }
                        } else if key == "to" {
                            if let Ok(date) = DateTime::parse_from_rfc3339(&value) {
                                to_opt = Some(date.with_timezone(&Utc));
                                if from_opt.is_some() {
                                    break;
                                }
                            }
                        }
                    }
                    let summary = self.mutex.get_projection(|summaries| {
                        if let Some(from) = from_opt {
                            let to = to_opt.unwrap();
                            let seconds_from_start = (from - self.start_time).num_seconds();
                            let seconds_to_start = (to - self.start_time).num_seconds();
                            let from_summary = summaries.get(seconds_from_start as usize).unwrap();
                            let to_summary = summaries.get(seconds_to_start as usize).unwrap();
                            RequestsSummary {
                                default: ServerSummary {
                                    total_requests: to_summary.default.total_requests - from_summary.default.total_requests,
                                    total_amount: to_summary.default.total_amount - from_summary.default.total_amount,
                                },
                                fallback: ServerSummary {
                                    total_requests: to_summary.fallback.total_requests - from_summary.fallback.total_requests,
                                    total_amount: to_summary.fallback.total_amount - from_summary.fallback.total_amount,
                                },
                            }
                        } else {
                            summaries.last().unwrap().clone()
                        }
                    }).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    let json = serde_json::to_vec(&summary)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to serialize summary: {}", e)))?;
                    res.header("Content-Type: application/json");
                    res.body_vec(json);
                } else if p == "/payments" {
                    res.status_code(405, "");
                }
                else {
                    res.body("Hello, Rinha!");
                }
            }
            "POST" => {
                let p = req.path();
                if p == "/payments" {
                    if let Ok(payment) = serde_json::from_slice::<RequestedPayment>(req.body().fill_buf()?) {
                        let updated_default = true;
                        println!("Got payment: {payment:?}");
                        let requesting_payment_json = serde_json::to_string(&RequestingPayment {
                            amount: payment.amount,
                            correlation_id: payment.correlation_id,
                            requested_at: chrono::Utc::now(),
                        })?;
                        println!("Requesting payment JSON: {requesting_payment_json}");
                        let client = reqwest::blocking::Client::new();
                        let response = client.post(format!("{}/payments", &self.server_default))
                            .header("Content-Type", "application/json")
                            .body(requesting_payment_json)
                            .send()
                            .map_err(|e| {
                                io::Error::new(
                                    io::ErrorKind::Other,
                                    format!("Error sending request to server: {e:?}")
                                )
                            })?;
                        println!("Response from server: {}", response.status());
                        if response.status().is_success() {
                            res.status_code(204, "");
                        } else {
                            res.status_code(response.status().as_u16() as usize, "");
                        }
                        let seconds_from_start = (Utc::now() - self.start_time).num_seconds();
                        fn get_last(summaries: &mut Vec<RequestsSummary>, index: usize) -> &mut RequestsSummary {
                            if summaries.len() < index {
                                let last = if index == 0 {
                                    RequestsSummary::default()
                                } else {
                                    get_last(summaries, index - 1).clone()
                                };
                                summaries.push(last);
                            }
                            &mut summaries[index]
                        }
                        self.mutex.set_fn(|summaries| {
                            let last = get_last(summaries, seconds_from_start as usize);
                            if updated_default {
                                last.default.total_requests += 1;
                                last.default.total_amount += payment.amount;
                            } else {
                                last.fallback.total_requests += 1;
                                last.fallback.total_amount += payment.amount;
                            }
                        })
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    } else {
                        println!("Failed to parse payment from request body.");
                        res.status_code(400, "Invalid payment data");
                    }
                }
                else {
                    res.status_code(405, "");
                }
            }
            _ => {
                res.status_code(405, "");
            }
        }

        Ok(())
    }
}

