//! Provides a software renderer that can be used without a GPU. The rendering
//! is much slower than with a normal GPU, but might be sufficient for
//! situations where you want to create a screenshot of the layout.

use super::{Backend, Mesh, Renderer, Rgba as LSColor, Shader, Transform};
use crate::layout::LayoutState;
use tiny_skia::{Canvas, Pixmap};

pub use image::{self, RgbaImage};

// struct SoftwareBackend {
//     color: AlphaBlended,
// }

// impl Backend for SoftwareBackend {
//     type Mesh = Vec<Vertex>;
//     type Texture = Texture;

//     fn create_mesh(&mut self, mesh: &Mesh) -> Self::Mesh {
//         let vertices = mesh.vertices();
//         mesh.indices()
//             .iter()
//             .map(|&index| {
//                 let v = vertices[index as usize];
//                 Vertex {
//                     position: Vec2::new(v.x, v.y),
//                     texcoord: Vec2::new(v.u, v.v),
//                 }
//             })
//             .collect()
//     }

//     fn render_mesh(
//         &mut self,
//         mesh: &Self::Mesh,
//         transform: Transform,
//         [tl, tr, br, bl]: [LSColor; 4],
//         texture: Option<&Self::Texture>,
//     ) {
//         let [x1, y1, z1, x2, y2, z2] = transform.to_column_major_array();
//         MyPipeline {
//             transform: Mat3::new(x1, x2, 0.0, y1, y2, 0.0, z1, z2, 0.0),
//             color_tl: Rgba::new(tl[0], tl[1], tl[2], tl[3]),
//             color_tr: Rgba::new(tr[0], tr[1], tr[2], tr[3]),
//             color_bl: Rgba::new(bl[0], bl[1], bl[2], bl[3]),
//             color_br: Rgba::new(br[0], br[1], br[2], br[3]),
//             texture,
//         }
//         .draw::<rasterizer::Triangles<'_, (f32,), BackfaceCullingDisabled>, _>(
//             mesh,
//             &mut self.color,
//             None,
//         );
//     }

//     fn free_mesh(&mut self, _: Self::Mesh) {}

//     fn create_texture(&mut self, width: u32, height: u32, data: &[u8]) -> Self::Texture {
//         Texture {
//             data: data.to_owned(),
//             width: width as f32,
//             height: height as f32,
//             stride: width as usize * 4,
//         }
//     }
//     fn free_texture(&mut self, _: Self::Texture) {}

//     fn resize(&mut self, _: f32, _: f32) {}
// }

// struct AlphaBlended(Buffer2d<Rgba<f32>>);

// impl Target for AlphaBlended {
//     type Item = Rgba<f32>;

//     fn size(&self) -> [usize; 2] {
//         self.0.size()
//     }

//     unsafe fn set(&mut self, pos: [usize; 2], src: Self::Item) {
//         debug_assert!(!src.r.is_nan());
//         debug_assert!(!src.g.is_nan());
//         debug_assert!(!src.b.is_nan());
//         debug_assert!(!src.a.is_nan());

//         let dst = self.0.get(pos);
//         self.0.set(
//             pos,
//             Rgba::new(
//                 src.a * src.r + (1.0 - src.a) * dst.r,
//                 src.a * src.g + (1.0 - src.a) * dst.g,
//                 src.a * src.b + (1.0 - src.a) * dst.b,
//                 src.a + (1.0 - src.a) * dst.a,
//             ),
//         );
//     }

//     unsafe fn get(&self, pos: [usize; 2]) -> Self::Item {
//         self.0.get(pos)
//     }

//     fn clear(&mut self, fill: Self::Item) {
//         self.0.clear(fill)
//     }
// }

// struct Texture {
//     data: Vec<u8>,
//     width: f32,
//     height: f32,
//     stride: usize,
// }

// struct MyPipeline<'a> {
//     transform: Mat3<f32>,
//     color_tl: Rgba<f32>,
//     color_tr: Rgba<f32>,
//     color_bl: Rgba<f32>,
//     color_br: Rgba<f32>,
//     texture: Option<&'a Texture>,
// }

// struct Vertex {
//     position: Vec2<f32>,
//     texcoord: Vec2<f32>,
// }

// #[derive(Clone, Mul, Add)]
// struct VsOut {
//     color: Rgba<f32>,
//     texcoord: Vec2<f32>,
// }

// impl Interpolate for VsOut {
//     #[inline(always)]
//     fn lerp2(fx: Self, fy: Self, x: f32, y: f32) -> Self {
//         fx * x + fy * y
//     }
//     #[inline(always)]
//     fn lerp3(fx: Self, fy: Self, fz: Self, x: f32, y: f32, z: f32) -> Self {
//         fx * x + fy * y + fz * z
//     }
// }

// impl Pipeline for MyPipeline<'_> {
//     type Vertex = Vertex;
//     type VsOut = VsOut;
//     type Pixel = Rgba<f32>;

//     fn vert(&self, vertex: &Self::Vertex) -> ([f32; 4], Self::VsOut) {
//         let left = self.color_tl * (1.0 - vertex.texcoord.y) + self.color_bl * vertex.texcoord.y;
//         let right = self.color_tr * (1.0 - vertex.texcoord.y) + self.color_br * vertex.texcoord.y;
//         let color = left * (1.0 - vertex.texcoord.x) + right * vertex.texcoord.x;

//         let pos = Vec3::new(vertex.position.x, vertex.position.y, 1.0) * self.transform;

//         (
//             [2.0 * pos.x - 1.0, -2.0 * pos.y + 1.0, 0.0, 1.0],
//             VsOut {
//                 color,
//                 texcoord: vertex.texcoord,
//             },
//         )
//     }

//     fn frag(&self, vsout: &Self::VsOut) -> Self::Pixel {
//         if let Some(texture) = self.texture {
//             let x = vsout.texcoord.x * texture.width;
//             let y = vsout.texcoord.y * texture.height;
//             let pixel = &texture.data[texture.stride * y as usize + x as usize * 4..];
//             return Rgba::new(
//                 f32::from(pixel[0]) / 255.0,
//                 f32::from(pixel[1]) / 255.0,
//                 f32::from(pixel[2]) / 255.0,
//                 f32::from(pixel[3]) / 255.0,
//             ) * vsout.color;
//         }
//         vsout.color
//     }

//     fn get_depth_strategy(&self) -> DepthStrategy {
//         DepthStrategy::None
//     }
// }

struct SkiaBuilder(tiny_skia::PathBuilder);

impl super::PathBuilder for SkiaBuilder {
    type Path = Option<tiny_skia::Path>;

    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y)
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y)
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, y1, x, y)
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, y1, x2, y2, x, y)
    }

    fn push_circle(&mut self, x: f32, y: f32, r: f32) {
        self.0.push_circle(x, y, r)
    }

    fn close(&mut self) {
        self.0.close()
    }

    fn finish(self) -> Self::Path {
        self.0.finish()
    }
}

fn convert_color([r, g, b, a]: [f32; 4]) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(r, g, b, a).unwrap()
}

struct SoftwareBackend<'a> {
    canvas: Canvas<'a>,
}

impl Backend for SoftwareBackend<'_> {
    type PathBuilder = SkiaBuilder;
    type Path = Option<tiny_skia::Path>;
    type Texture = Option<tiny_skia::Pixmap>;

    fn build_path(&mut self) -> Self::PathBuilder {
        SkiaBuilder(tiny_skia::PathBuilder::new())
    }

    fn render_path(
        &mut self,
        path: &Self::Path,
        stroke: Option<f32>,
        transform: Transform,
        shader: Shader,
        texture: Option<&Self::Texture>,
    ) {
        if let Some(path) = path {
            let pixmap = self.canvas.pixmap();
            let (w, h) = (pixmap.width() as _, pixmap.height() as _);

            let [sx, ky, kx, sy, tx, ty] = transform.to_row_major_array();
            let transform = tiny_skia::Transform::from_row(sx, ky, kx, sy, tx, ty)
                .unwrap()
                .post_scale(w, h)
                .unwrap();

            let shader = if let Some(Some(texture)) = texture {
                tiny_skia::Pattern::new(
                    texture.as_ref(),
                    tiny_skia::SpreadMode::Pad,
                    tiny_skia::FilterQuality::Bilinear,
                    1.0,
                    tiny_skia::Transform::from_scale(
                        1.0 / texture.width() as f32,
                        1.0 / texture.height() as f32,
                    )
                    .unwrap(),
                )
            } else {
                match shader {
                    Shader::SolidColor(col) => tiny_skia::Shader::SolidColor(convert_color(col)),
                    Shader::VerticalGradient(top, bottom) => {
                        let bounds = path.bounds();
                        tiny_skia::LinearGradient::new(
                            tiny_skia::Point::from_xy(0.0, bounds.top()),
                            tiny_skia::Point::from_xy(0.0, bounds.bottom()),
                            vec![
                                tiny_skia::GradientStop::new(0.0, convert_color(top)),
                                tiny_skia::GradientStop::new(1.0, convert_color(bottom)),
                            ],
                            tiny_skia::SpreadMode::Pad,
                            tiny_skia::Transform::identity(),
                        )
                        .unwrap()
                    }
                    Shader::HorizontalGradient(left, right) => {
                        let bounds = path.bounds();
                        tiny_skia::LinearGradient::new(
                            tiny_skia::Point::from_xy(bounds.left(), 0.0),
                            tiny_skia::Point::from_xy(bounds.right(), 0.0),
                            vec![
                                tiny_skia::GradientStop::new(0.0, convert_color(left)),
                                tiny_skia::GradientStop::new(1.0, convert_color(right)),
                            ],
                            tiny_skia::SpreadMode::Pad,
                            tiny_skia::Transform::identity(),
                        )
                        .unwrap()
                    }
                }
            };

            self.canvas.set_transform(transform);

            if let Some(stroke_width) = stroke {
                self.canvas.stroke_path(
                    path,
                    &tiny_skia::Paint {
                        shader,
                        anti_alias: true,
                        ..Default::default()
                    },
                    &tiny_skia::Stroke {
                        width: stroke_width,
                        ..Default::default()
                    },
                );
            } else {
                self.canvas.fill_path(
                    path,
                    &tiny_skia::Paint {
                        shader,
                        anti_alias: true,
                        ..Default::default()
                    },
                    tiny_skia::FillRule::Winding,
                );
            }
        }
    }

    fn free_path(&mut self, _: Self::Path) {}

    fn create_texture(&mut self, width: u32, height: u32, data: &[u8]) -> Self::Texture {
        let mut texture = tiny_skia::Pixmap::new(width, height)?;
        for (d, &[r, g, b, a]) in texture
            .pixels_mut()
            .iter_mut()
            .zip(bytemuck::cast_slice::<_, [u8; 4]>(data))
        {
            *d = tiny_skia::ColorU8::from_rgba(r, g, b, a).premultiply();
        }
        Some(texture)
    }

    fn free_texture(&mut self, _: Self::Texture) {}

    fn resize(&mut self, _: f32, _: f32) {}
}

pub struct SoftwareRenderer {
    renderer: Renderer<Option<tiny_skia::Path>, Option<tiny_skia::Pixmap>>,
    pixmap: Pixmap,
}

impl SoftwareRenderer {
    pub fn new() -> Self {
        Self {
            renderer: Renderer::new(),
            pixmap: Pixmap::new(1, 1).unwrap(),
        }
    }

    /// Renders the layout state provided into an image of the selected resolution.
    /// The final render will have pixelated edges as there is not going to be any
    /// anti aliasing. Use [`render_anti_aliased`] if you want it to be anti
    /// aliased. Note that this is software rendered and thus will be much slower
    /// than rendering on the GPU.
    ///
    /// [`render_anti_aliased`]: fn.render_anti_aliased.html
    pub fn render(&mut self, state: &LayoutState, [width, height]: [u32; 2]) {
        if width != self.pixmap.width() || height != self.pixmap.height() {
            self.pixmap = Pixmap::new(width, height).unwrap();
        } else {
            // FIXME: .fill() once it's stable.
            for b in self.pixmap.data_mut() {
                *b = 0;
            }
        }

        let mut backend = SoftwareBackend {
            canvas: Canvas::from(self.pixmap.as_mut()),
        };

        self.renderer
            .render(&mut backend, (width as _, height as _), &state);
    }

    pub fn image(&self) -> image::ImageBuffer<image::Rgba<u8>, &[u8]> {
        image::ImageBuffer::from_raw(
            self.pixmap.width(),
            self.pixmap.height(),
            self.pixmap.data(),
        )
        .unwrap()
    }

    pub fn into_image(self) -> image::RgbaImage {
        RgbaImage::from_raw(
            self.pixmap.width(),
            self.pixmap.height(),
            self.pixmap.take(),
        )
        .unwrap()
    }
}
