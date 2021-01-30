use crate::output_vec;
use livesplit_core::layout::LayoutState;

#[cfg(feature = "buffer-rendering")]
use livesplit_core::rendering::buffer_rendering::BufferRenderer;

#[cfg(not(feature = "buffer-rendering"))]
pub struct BufferRenderer;
#[cfg(not(feature = "buffer-rendering"))]
impl BufferRenderer {
    fn new() -> Self {
        Self
    }

    fn render(&mut self, _: &LayoutState, _: &mut Vec<u8>, _: [f32; 2]) {}
}

/// type
pub type OwnedBufferRenderer = Box<BufferRenderer>;

#[no_mangle]
pub extern "C" fn BufferRenderer_new() -> OwnedBufferRenderer {
    Box::new(BufferRenderer::new())
}

#[no_mangle]
pub extern "C" fn BufferRenderer_drop(this: OwnedBufferRenderer) {
    drop(this);
}

#[no_mangle]
pub unsafe extern "C" fn BufferRenderer_render(
    this: &mut BufferRenderer,
    layout_state: &LayoutState,
    width: f32,
    height: f32,
) -> *const u8 {
    output_vec(|buf| {
        this.render(layout_state, buf, [width, height]);
    })
    .cast()
}
