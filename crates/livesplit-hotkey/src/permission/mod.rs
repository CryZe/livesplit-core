//! Native privacy permissions, separate from creating hooks or registering keys.
//!
//! Only the macOS backend currently uses a separate privacy-grant API. Other backends
//! still report resource, device and registration failures when creating hooks;
//! a successful privacy check does not promise that those operations will work.

use core::fmt;

#[cfg(any(all(feature = "std", target_os = "macos"), test))]
pub mod macos;

#[cfg(all(feature = "std", target_os = "macos"))]
pub use crate::macos::permission::{check, request};
#[cfg(not(all(feature = "std", target_os = "macos")))]
mod unrestricted;
#[cfg(not(all(feature = "std", target_os = "macos")))]
pub use unrestricted::{check, request};

/// A native privacy grant required by the selected keyboard hook.
///
/// These are permission identifiers, not localized application messages. They
/// do not describe device-file access, conflicting shortcuts or other failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum Permission {
    /// Monitoring keyboard input with a listen-only tap on macOS 10.15 or later.
    InputMonitoring,
    /// Filtering keyboard events, or legacy macOS keyboard monitoring.
    Accessibility,
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InputMonitoring => "Input Monitoring",
            Self::Accessibility => "Accessibility",
        })
    }
}
