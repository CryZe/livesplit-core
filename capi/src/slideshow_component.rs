// TODO: Class Doc

use super::{output_vec, Json};
use crate::component::OwnedComponent;
use crate::key_value_component_state::OwnedKeyValueComponentState;
use livesplit_core::component::slideshow::Component as SlideshowComponent;
use livesplit_core::{GeneralLayoutSettings, Timer};

/// type
pub type OwnedSlideshowComponent = Box<SlideshowComponent>;

/// Creates a new Slideshow Component.
#[no_mangle]
pub extern "C" fn SlideshowComponent_new() -> OwnedSlideshowComponent {
    Box::new(SlideshowComponent::new())
}

/// drop
#[no_mangle]
pub extern "C" fn SlideshowComponent_drop(this: OwnedSlideshowComponent) {
    drop(this);
}

/// Converts the component into a generic component suitable for using with a
/// layout.
#[no_mangle]
pub extern "C" fn SlideshowComponent_into_generic(this: OwnedSlideshowComponent) -> OwnedComponent {
    Box::new((*this).into())
}

/// Encodes the component's state information as JSON.
#[no_mangle]
pub extern "C" fn SlideshowComponent_state_as_json(
    this: &mut SlideshowComponent,
    timer: &Timer,
    layout_settings: &GeneralLayoutSettings,
) -> Json {
    output_vec(|o| {
        this.state(timer, layout_settings).write_json(o).unwrap();
    })
}

/// Calculates the component's state based on the timer provided.
#[no_mangle]
pub extern "C" fn SlideshowComponent_state(
    this: &mut SlideshowComponent,
    timer: &Timer,
    layout_settings: &GeneralLayoutSettings,
) -> OwnedKeyValueComponentState {
    Box::new(this.state(timer, layout_settings))
}
