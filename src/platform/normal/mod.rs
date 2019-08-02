#![allow(missing_docs)]

pub use chrono::{DateTime, Duration, Local, Utc};
pub use palette;
pub use std::time::Instant;

pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}
