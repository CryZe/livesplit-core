#![allow(missing_docs)]

pub use chrono::{DateTime, Duration, Local, Utc};
pub use indexmap;
pub use palette;
pub use std::time::Instant;
pub mod prelude {}

pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}
