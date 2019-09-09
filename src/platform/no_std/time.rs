use super::prelude::*;
use core::ops::Sub;
use core::sync::atomic::{self, AtomicPtr};

pub trait Clock: 'static {
    fn now(&self) -> Duration;
}

static CLOCK: AtomicPtr<Box<dyn Clock>> = AtomicPtr::new(core::ptr::null_mut());

pub fn register_clock<C: Clock>(clock: C) {
    let clock: Box<dyn Clock> = Box::new(clock);
    let clock = Box::new(clock);
    // FIXME: Should be compare_and_swap. This is racy.
    if !CLOCK.load(atomic::Ordering::SeqCst).is_null() {
        panic!("The clock has already been registered");
    }
    CLOCK.store(Box::into_raw(clock), atomic::Ordering::SeqCst);
}

pub use core::time::Duration;

#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub struct Instant(Duration);

impl Instant {
    pub fn now() -> Self {
        let clock = CLOCK.load(atomic::Ordering::SeqCst);
        if clock.is_null() {
            panic!("No clock registered");
        }
        let clock = unsafe { &*clock };
        Instant(clock.now())
    }
}

impl Sub for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Duration {
        self.0 - rhs.0
    }
}