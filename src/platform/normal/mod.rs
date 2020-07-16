#![allow(missing_docs)]

pub use indexmap;
pub use time::{Duration, Instant, OffsetDateTime as DateTime};

pub fn utc_now() -> DateTime {
    DateTime::now_utc()
}

pub fn to_local(date_time: DateTime) -> DateTime {
    date_time.to_offset(time::UtcOffset::local_offset_at(date_time))
}
