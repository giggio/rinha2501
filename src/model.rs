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

