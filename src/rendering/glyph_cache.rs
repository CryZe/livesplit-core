use super::{
    font::Font,
    mesh::{fill_builder, Mesh},
    Backend,
};
use hashbrown::HashMap;
use lyon::{
    path::{self, math::point, Path},
    tessellation::{FillOptions, FillTessellator},
};
use ttf_parser::OutlineBuilder;

struct PathBuilder<PB>(PB);

impl<PB: super::PathBuilder> OutlineBuilder for PathBuilder<PB> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, -y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, -y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, -y1, x, -y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.curve_to(x1, -y1, x2, -y2, x, -y);
    }
    fn close(&mut self) {
        self.0.close();
    }
}

pub struct GlyphCache<P> {
    glyphs: HashMap<u32, P>,
}

impl<P> Default for GlyphCache<P> {
    fn default() -> Self {
        Self {
            glyphs: Default::default(),
        }
    }
}

impl<P> GlyphCache<P> {
    pub fn new() -> Self {
        Default::default()
    }

    #[cfg(feature = "font-loading")]
    pub fn clear(&mut self, backend: &mut impl Backend<Path = P>) {
        for (_, path) in self.glyphs.drain() {
            backend.free_path(path);
        }
    }

    pub fn lookup_or_insert(
        &mut self,
        font: &Font<'_>,
        glyph: u32,
        backend: &mut impl Backend<Path = P>,
    ) -> &P {
        self.glyphs.entry(glyph).or_insert_with(|| {
            let mut builder = PathBuilder(backend.build_path());
            font.outline_glyph(glyph, &mut builder);
            super::PathBuilder::finish(builder.0)
        })
    }
}
