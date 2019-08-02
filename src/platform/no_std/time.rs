use core::marker::PhantomData;
use core::ops::{Add, Sub};
use derive_more::{Add, Neg, Sub};
use ordered_float::OrderedFloat;

#[derive(Copy, Clone, PartialEq, Debug)]
struct FFIDateTime;
pub struct Local;
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct DateTime<T>(PhantomData<T>, FFIDateTime);
pub trait TimeZone: Sized {
    fn datetime_from_str(&self, s: &str, fmt: &str) -> Result<DateTime<Self>, ParseError>;
}
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Utc;
#[derive(Debug)]
pub struct ParseError;

#[derive(Add, Sub, Neg, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Duration(i128);
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub struct Instant(OrderedFloat<f64>);

impl Instant {
    pub fn now() -> Self {
        unimplemented!()
        // Instant(OrderedFloat(unsafe { Instant_now() }))
    }
}

impl Sub for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Duration {
        let total_secs = (self.0).0 - (rhs.0).0;
        let total_nanos = total_secs * 1_000_000_000.0;
        Duration(total_nanos as _)
    }
}

impl Duration {
    pub fn nanoseconds(nanos: i64) -> Self {
        Duration(nanos as _)
    }

    pub fn microseconds(micros: i64) -> Self {
        Duration(micros as i128 * 1_000)
    }

    pub fn num_microseconds(self) -> Option<i128> {
        Some(self.0 / 1_000)
    }

    pub fn from_std(val: core::time::Duration) -> Option<Self> {
        let secs = val.as_secs() as i128;
        let secs_as_nanos = secs * 1_000_000_000;
        Some(Duration(val.subsec_nanos() as i128 + secs_as_nanos))
    }
}

impl TimeZone for Utc {
    fn datetime_from_str(&self, _: &str, _: &str) -> Result<DateTime<Self>, ParseError> {
        Ok(DateTime(PhantomData, Default::default()))
    }
}

impl<T> Add<Duration> for DateTime<T> {
    type Output = DateTime<T>;

    fn add(self, _: Duration) -> DateTime<T> {
        self
    }
}

impl<T> DateTime<T> {
    pub fn signed_duration_since<Tz2: TimeZone>(self, _: DateTime<Tz2>) -> Duration {
        Duration::zero()
    }

    pub fn format(&self, _: &str) -> String {
        format!(
            "{:02}/{:02}/{:04} {:02}:{:02}:{:02}",
            self.1.month, self.1.day, self.1.year, self.1.hours, self.1.minutes, self.1.seconds
        )
    }

    pub fn to_rfc2822(&self) -> &'static str {
        "Tue, 1 Jul 2003 10:52:37 +0200"
    }

    pub fn to_rfc3339(&self) -> &'static str {
        "1996-12-19T16:39:57-08:00"
    }

    pub fn with_timezone<Tz2>(&self, _: &Tz2) -> DateTime<Tz2> {
        DateTime(PhantomData, Default::default())
    }
}

pub fn utc_now() -> DateTime<Utc> {
    DateTime(PhantomData, Default::default())
}
