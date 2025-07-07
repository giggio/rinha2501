extern crate may_minihttp;
// use std::{process::exit, sync::LazyLock};
use std::process::exit;

use std::io::{self, BufRead};
use may_minihttp::{HttpServer, HttpService, Request, Response};
// use regex::Regex;

mod model;
use crate::model::*;

#[derive(Clone)]
struct RinhaServer {
  server_default: String,
  server_fallback: String,
}

impl HttpService for RinhaServer {
  fn call(&mut self, req: Request, res: &mut Response) -> io::Result<()> {
    // static MATCH_1: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\/clientes\/(\d+)\/transacoes.*").unwrap());
    // static MATCH_PAYMENTS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\/payments").unwrap());
    match req.method() {
      "GET" => {
        let p = req.path();
        // if let Some(_) = MATCH_PAYMENTS.captures(p) {
        if p == "/payments" {
          res.status_code(405, "");
        }
        else {
          res.body("Hello, world!");
        }
      }
      "POST" => {
        let p = req.path();
        if p == "/payments" {
          if let Ok(payment) = serde_json::from_slice::<RequestedPayment>(req.body().fill_buf()?) {
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

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let (server_default, server_fallback) = match args.len() {
    1 => {
      ("http://localhost:8001".to_owned(), "http://localhost:8002".to_owned())
    }
    2 => {
      (args[1].clone(), args[1].clone())
    }
    3 => {
      (args[1].clone(), args[2].clone())
    }
    _ => {
      println!("You need to supply from 0 to 2 server urls.");
      exit(1);
    }
  };
  let port: usize = std::env::var("PORT")
        .unwrap_or("9999".to_owned())
        .parse()
        .unwrap_or(9999);
  let rinha_server = RinhaServer {
    server_default: server_default.clone(),
    server_fallback: server_fallback.clone(),
  };
  let server = match HttpServer(rinha_server).start(format!("0.0.0.0:{port}")) {
        Ok(server) => server,
        Err(e) => {
            println!("Error starting server: {:?}", e);
            exit(1);
        }
    };
  println!("Server started on http://localhost:{port}. Servers: {}, {}. Press Ctrl+C to stop.", server_default, server_fallback);
  match server.join() {
    Ok(_) => {
      println!("Server stopped.");
    }
    Err(e) => {
      println!("Error waiting for server: {:?}", e);
      exit(1);
    }
  }
}
