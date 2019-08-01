cfg_if::cfg_if! {
    if #[cfg(not(feature = "std"))] {
        use derive_more::{Add, Neg, Sub};
        use ordered_float::OrderedFloat;
        use core::ops::Sub;

        #[derive(Add, Sub, Neg, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        pub struct Duration(i128);
        #[derive(Add, Sub, Neg, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        pub struct Utc {}
        #[derive(Add, Sub, Neg, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        pub struct DateTime<T>(T);
        #[derive(Copy, Clone)]
        pub struct Local;
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

        impl<T> DateTime<T> {
            pub fn with_timezone<U: Copy>(&self, tz: &U) -> DateTime<U> {
                DateTime(*tz)
            }
        }
    } else if #[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))] {
        mod wasm;
        pub use self::wasm::*;
    } else {
        mod normal;
        pub use self::normal::*;
        pub use chrono::{Duration, Utc};
    }
}
