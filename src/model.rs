use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct RequestedPayment {
    #[serde(with = "rust_decimal::serde::float")]
    pub amount: Decimal,
    pub correlation_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestingPayment {
    #[serde(with = "rust_decimal::serde::float")]
    pub amount: Decimal,
    pub correlation_id: String,
    pub requested_at: DateTime<Utc>,
}

#[derive(Default, Copy, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestsSummary {
    pub default: ServerSummary,
    pub fallback: ServerSummary,
}
#[derive(Default, Copy, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSummary {
    pub total_requests: u64,
    #[serde(with = "rust_decimal::serde::float")]
    pub total_amount: Decimal,
}
