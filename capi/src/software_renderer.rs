use livesplit_core::layout::LayoutState;

#[cfg(feature = "software-rendering")]
use livesplit_core::rendering::software::BorrowedSoftwareRenderer as SoftwareRenderer;

#[cfg(not(feature = "software-rendering"))]
pub struct SoftwareRenderer;
#[cfg(not(feature = "software-rendering"))]
impl SoftwareRenderer {
    fn new() -> Self {
        Self
    }

    fn render(&mut self, _: &LayoutState, _: &mut [u8], _: [u32; 2]) {}
}

/// type
pub type OwnedSoftwareRenderer = Box<SoftwareRenderer>;

#[no_mangle]
pub extern "C" fn SoftwareRenderer_new() -> OwnedSoftwareRenderer {
    Box::new(SoftwareRenderer::new())
}

#[no_mangle]
pub extern "C" fn SoftwareRenderer_drop(this: OwnedSoftwareRenderer) {
    drop(this);
}

#[no_mangle]
pub unsafe extern "C" fn SoftwareRenderer_render(
    this: &mut SoftwareRenderer,
    layout_state: &LayoutState,
    data: *mut u8,
    width: u32,
    height: u32,
    stride: u32,
) {
    this.render(
        layout_state,
        std::slice::from_raw_parts_mut(data, stride as usize * height as usize * 4),
        [width, height],
        stride,
    );
}
