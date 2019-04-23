use std::ops::Sub;

#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub struct Instant;

impl Instant {
    pub fn now() -> Self {
        Self
    }
}

impl Sub for Instant {
    type Output = core::time::Duration;

    fn sub(self, _rhs: Instant) -> core::time::Duration {
        core::time::Duration::new(0, 0)
    }
}

pub use chrono::{DateTime, Duration, Local, Utc};
pub use indexmap;

use chrono::NaiveDateTime;

pub fn utc_now() -> DateTime<Utc> {
    DateTime::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc)
}
