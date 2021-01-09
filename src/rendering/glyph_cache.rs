use super::{font::Font, Backend};
use hashbrown::HashMap;
use tiny_skia::{Canvas, Color, FillRule, Paint, Pixmap, Shader, Transform};
use ttf_parser::OutlineBuilder;

struct ImageBuilder(tiny_skia::PathBuilder);

impl OutlineBuilder for ImageBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, -y)
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, -y)
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, -y1, x, -y)
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, -y1, x2, -y2, x, -y)
    }

    fn close(&mut self) {
        self.0.close()
    }
}

pub struct GlyphCache<T> {
    textures: HashMap<u32, Option<GlyphTexture<T>>>,
}

pub struct GlyphTexture<T> {
    pub texture: T,
    pub width: f32,
    pub x_offset: f32,
}

impl<T> Default for GlyphCache<T> {
    fn default() -> Self {
        Self {
            textures: Default::default(),
        }
    }
}

impl<T> GlyphCache<T> {
    pub fn new() -> Self {
        Default::default()
    }

    #[cfg(feature = "font-loading")]
    pub fn clear(&mut self, backend: &mut impl Backend<Texture = T>) {
        for (_, texture) in self.textures.drain() {
            if let Some(texture) = texture {
                backend.free_texture(texture.texture);
            }
        }
    }

    pub fn lookup_or_insert(
        &mut self,
        font: &Font<'_>,
        glyph: u32,
        backend: &mut impl Backend<Texture = T>,
    ) -> Option<&GlyphTexture<T>> {
        self.textures
            .entry(glyph)
            .or_insert_with(|| {
                let mut builder = ImageBuilder(tiny_skia::PathBuilder::new());
                if !font.outline_glyph(glyph, &mut builder) {
                    return None;
                }
                let path = builder.0.finish()?;
                let bounds = path.bounds();
                let x = bounds.x().floor() as i32;
                // let y = bounds.y().floor() as i32;
                let end_x = (bounds.x() + bounds.width()).ceil() as i32;
                // let end_y = (bounds.y() + bounds.height()).ceil() as i32;
                let width = (end_x - x) as u32;
                // let height = (end_y - y) as u32;

                // TODO: Shouldn't really need a cast
                let height = font.height() as u32;

                let mut image = Pixmap::new(width, height)?;

                // FIXME: slice::fill once its stable
                for p in bytemuck::cast_slice_mut(image.as_mut().data_mut()) {
                    *p = [255u8, 255, 255, 0];
                }

                let mut canvas = Canvas::from(image.as_mut());

                canvas.apply_transform(&Transform::from_translate(-bounds.x(), font.ascender())?);

                canvas.fill_path(
                    &path,
                    &Paint {
                        shader: Shader::SolidColor(Color::WHITE),
                        anti_alias: true,
                        ..Default::default()
                    },
                    FillRule::Winding,
                );

                Some(GlyphTexture {
                    texture: backend.create_texture(width, height, image.data()),
                    width: width as f32,
                    x_offset: x as f32,
                })
            })
            .as_ref()
    }
}
