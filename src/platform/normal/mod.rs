#![allow(missing_docs)]

pub use std::time::Instant;
pub use chrono::{DateTime, Duration, Utc, Local};

pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}
