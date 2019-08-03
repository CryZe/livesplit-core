#![allow(missing_docs)]

pub use chrono::{DateTime, Duration, Local, Utc};
pub use std::time::Instant;
pub use indexmap;
pub mod prelude {}

pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}
