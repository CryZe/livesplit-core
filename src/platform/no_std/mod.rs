mod time;
pub use self::time::*;
pub use chrono::{DateTime, Duration, Utc, Utc as Local};
pub mod indexmap;
pub mod palette;
pub mod prelude {
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::string::String;
    pub use alloc::string::ToString;
    pub use alloc::vec::Vec;
    pub use alloc::{format, vec};
}

use chrono::NaiveDateTime;

pub fn utc_now() -> DateTime<Utc> {
    DateTime::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc)
}
