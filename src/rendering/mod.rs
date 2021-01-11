//! The rendering module provides a renderer for layout states that is
//! abstracted over different backends so that it can be used with OpenGL,
//! DirectX, Vulkan, Metal, WebGL or any other rendering framework. An optional
//! software renderer is available behind the `software-rendering` feature.
//! While it is slower than using a proper GPU backend, it might be sufficient
//! for situations where you want to create a screenshot of the layout.

// # Coordinate spaces used in this module
//
// ## Backend Coordinate Space
//
// The backend has its own coordinate space that ranges from 0 to 1 across both
// dimensions. (0, 0) is the top left corner of the rendered layout and (1, 1)
// is the bottom right corner. Since the coordinate space forms a square, the
// aspect ratio of the layout is not respected.
//
// ## Renderer Coordinate Space
//
// The renderer internally uses the so called renderer coordinate space. It has
// the dimensions [width, 1] with the width being chosen such that the renderer
// coordinate space respects the aspect ratio of the render target. This
// coordinate space is mostly an implementation detail.
//
// ## Component Coordinate Space
//
// The component coordinate space is a coordinate space local to a single
// component. This means that (0, 0) is the top left corner and (width, height)
// is the bottom right corner of the component. Width and Height are chosen
// based on various properties. In vertical mode, the height is chosen to be the
// component's actual height, while the width is dynamically adjusted based on
// the other components in the layout and the dimensions of the render target.
// In horizontal mode, the height is always the two row height, while the width
// is dynamically adjusted based the component's width preference. The width
// preference however only serves as a ratio of how much of the total width to
// distribute to the individual components. So similar to vertical mode, the
// width is fairly dynamic.
//
// ## Default Pixel Space
//
// The default pixel space describes a default scaling factor to apply to the
// component coordinate space. Both the original LiveSplit as well as
// livesplit-core internally use this coordinate space to store the component
// settings that influence dimensions of elements drawn on the component, such
// as font sizes and the dimensions of the component itself. It also serves as a
// good default size when choosing the size of a window or an image when the
// preferred size of the layout is unknown. The factor for converting component
// space coordinates to the default pixel space coordinates is 24.
//
// ### Guidelines for Spacing and Sizes in the Component Coordinate Space
//
// The default height of a component in the component coordinate space is 1.
// This is equal to the height of one split or one key value component. The
// default text size is 0.8. There is a padding of 0.35 to the left and right
// side of a component for the contents shown inside a component, such as images
// and texts. The same padding of 0.35 is also used for the minimum spacing
// between text and other content such as an icon or another text. A vertical
// padding of 10% of the height of the available space is chosen unless that is
// larger than the normal padding. If text doesn't fit, it is to be either
// abbreviated or overflown via the use of ellipsis. Numbers and times are
// supposed to be aligned to the right and should be using a monospace text
// layout. Sometimes components are rendered in two row mode. The height of
// these components is 1.8. All components also need to be able to render with
// this height in horizontal mode. Separators have a thickness of 0.1, while
// thin separators have half of this thickness.

mod component;
mod font;
mod glyph_cache;
mod icon;
mod mesh;

#[cfg(feature = "software-rendering")]
pub mod software;

use self::{font::Font, glyph_cache::GlyphCache, icon::Icon};
use crate::{
    layout::{ComponentState, LayoutDirection, LayoutState},
    settings::{Color, FontStretch, FontStyle, FontWeight, Gradient},
};
use alloc::borrow::Cow;
use core::iter;
use euclid::{Transform2D, UnknownUnit};
use rustybuzz::UnicodeBuffer;

pub use self::mesh::{Mesh, Vertex};
pub use euclid;

/// The default font to be used for general text. The font is encoded as TTF.
pub const TEXT_FONT: &[u8] = include_bytes!("fonts/FiraSans-Regular.ttf");
/// The default font to be used for timers. The font is encoded as TTF.
pub const TIMER_FONT: &[u8] = include_bytes!("fonts/Timer.ttf");

/// Describes a coordinate in 2D space.
pub type Pos = [f32; 2];
/// A color encoded as RGBA (red, green, blue, alpha) where each component is
/// stored as a value between 0 and 1.
pub type Rgba = [f32; 4];
/// A transformation matrix to apply to meshes in order to place them into the
/// scene.
pub type Transform = Transform2D<f32, UnknownUnit, UnknownUnit>;

const PADDING: f32 = 0.35;
const BOTH_PADDINGS: f32 = 2.0 * PADDING;
const BOTH_VERTICAL_PADDINGS: f32 = DEFAULT_COMPONENT_HEIGHT - DEFAULT_TEXT_SIZE;
const VERTICAL_PADDING: f32 = BOTH_VERTICAL_PADDINGS / 2.0;
const DEFAULT_COMPONENT_HEIGHT: f32 = 1.0;
const TWO_ROW_HEIGHT: f32 = 2.0 * DEFAULT_TEXT_SIZE + BOTH_VERTICAL_PADDINGS;
const DEFAULT_TEXT_SIZE: f32 = 0.8;
const DEFAULT_TEXT_ASCENT: f32 = 0.6;
const DEFAULT_TEXT_DESCENT: f32 = DEFAULT_TEXT_SIZE - DEFAULT_TEXT_ASCENT;
const TEXT_ALIGN_TOP: f32 = VERTICAL_PADDING + DEFAULT_TEXT_ASCENT;
const TEXT_ALIGN_BOTTOM: f32 = -(VERTICAL_PADDING + DEFAULT_TEXT_DESCENT);
const TEXT_ALIGN_CENTER: f32 = DEFAULT_TEXT_ASCENT - DEFAULT_TEXT_SIZE / 2.0;
const SEPARATOR_THICKNESS: f32 = 0.1;
const THIN_SEPARATOR_THICKNESS: f32 = SEPARATOR_THICKNESS / 2.0;
const PSEUDO_PIXELS: f32 = 1.0 / 24.0;
const DEFAULT_VERTICAL_WIDTH: f32 = 11.5;

fn vertical_padding(height: f32) -> f32 {
    (VERTICAL_PADDING * height).min(PADDING)
}

pub trait PathBuilder {
    type Path;

    /// Appends a MoveTo segment.
    ///
    /// Start of a contour.
    fn move_to(&mut self, x: f32, y: f32);

    /// Appends a LineTo segment.
    fn line_to(&mut self, x: f32, y: f32);

    /// Appends a QuadTo segment.
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32);

    /// Appends a CurveTo segment.
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32);

    /// Appends a ClosePath segment.
    ///
    /// End of a contour.
    fn close(&mut self);

    fn finish(self) -> Self::Path;
}

#[derive(Copy, Clone)]
pub enum Shader {
    SolidColor(Rgba),
    VerticalGradient(Rgba, Rgba),
    HorizontalGradient(Rgba, Rgba),
}

/// The rendering backend for the Renderer is abstracted out into the Backend
/// trait such that the rendering isn't tied to a specific rendering framework.
pub trait Backend {
    /// The type the backend uses for paths.
    type PathBuilder: PathBuilder<Path = Self::Path>;
    type Path;
    /// The type the backend uses for textures.
    type Image;

    fn build_path(&mut self) -> Self::PathBuilder;

    fn build_circle(&mut self, x: f32, y: f32, r: f32) -> Self::Path {
        // Based on https://spencermortensen.com/articles/bezier-circle/
        const C: f64 = 0.551915024494;
        let c = (C * r as f64) as f32;
        let mut builder = self.build_path();
        builder.move_to(x, y - r);
        builder.curve_to(x + c, y - r, x + r, y - c, x + r, y);
        builder.curve_to(x + r, y + c, x + c, y + r, x, y + r);
        builder.curve_to(x - c, y + r, x - r, y + c, x - r, y);
        builder.curve_to(x - r, y - c, x - c, y - r, x, y - r);
        builder.close();
        builder.finish()
    }

    /// Instructs the backend to render out a mesh. The rendering uses no
    /// backface culling or depth buffering. The colors are supposed to be alpha
    /// blended and don't use sRGB. The transform represents a transformation
    /// matrix to be applied to the mesh's vertices in order to place it in the
    /// scene. The scene's coordinates are within 0..1 for both x (left..right)
    /// and y (up..down). There may be a texture that needs to be applied to the
    /// mesh based on its u and v texture coordinates. There also are four
    /// colors for are interpolated between based on the u and v texture
    /// coordinates. The colors are positioned in UV texture space in the
    /// following way:
    /// ```rust,ignore
    /// [
    ///     (0, 0), // Top Left
    ///     (1, 0), // Top Right
    ///     (1, 1), // Bottom Right
    ///     (0, 1), // Bottom Left
    /// ]
    /// ```
    fn render_fill_path(&mut self, path: &Self::Path, shader: Shader, transform: Transform);

    fn render_stroke_path(
        &mut self,
        path: &Self::Path,
        stroke_width: f32,
        color: Rgba,
        transform: Transform,
    );

    fn render_image(&mut self, image: &Self::Image, rectangle: &Self::Path, transform: Transform);

    /// Instructs the backend to free a mesh as it is not needed anymore.
    fn free_path(&mut self, path: Self::Path);

    /// Instructs the backend to create a texture out of the texture data
    /// provided. The texture's resolution is provided as well. The data is an
    /// array of chunks of RGBA8 encoded pixels (red, green, blue, alpha with
    /// each channel being an u8).
    fn create_image(&mut self, width: u32, height: u32, data: &[u8]) -> Self::Image;

    /// Instructs the backend to free a texture as it is not needed anymore.
    fn free_image(&mut self, texture: Self::Image);

    /// Instructs the backend to resize the size of the render target.
    fn resize(&mut self, width: f32, height: f32);
}

enum CachedSize {
    Vertical(f32),
    Horizontal(f32),
}

/// A renderer can be used to render out layout states with the backend chosen.
pub struct Renderer<P, I> {
    #[cfg(feature = "font-loading")]
    timer_font_setting: Option<crate::settings::Font>,
    timer_font: Font<'static>,
    timer_glyph_cache: GlyphCache<P>,
    #[cfg(feature = "font-loading")]
    times_font_setting: Option<crate::settings::Font>,
    times_font: Font<'static>,
    times_glyph_cache: GlyphCache<P>,
    #[cfg(feature = "font-loading")]
    text_font_setting: Option<crate::settings::Font>,
    text_font: Font<'static>,
    text_glyph_cache: GlyphCache<P>,
    rectangle: Option<P>,
    cached_size: Option<CachedSize>,
    icons: IconCache<I>,
    text_buffer: Option<UnicodeBuffer>,
}

struct IconCache<I> {
    game_icon: Option<Icon<I>>,
    split_icons: Vec<Option<Icon<I>>>,
    detailed_timer_icon: Option<Icon<I>>,
}

impl<P, I> Default for Renderer<P, I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P, I> Renderer<P, I> {
    /// Creates a new renderer.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "font-loading")]
            timer_font_setting: None,
            timer_font: Font::from_slice(
                TIMER_FONT,
                0,
                FontStyle::Normal,
                FontWeight::Bold,
                FontStretch::Normal,
            )
            .unwrap(),
            timer_glyph_cache: GlyphCache::new(),
            #[cfg(feature = "font-loading")]
            times_font_setting: None,
            times_font: Font::from_slice(
                TEXT_FONT,
                0,
                FontStyle::Normal,
                FontWeight::Bold,
                FontStretch::Normal,
            )
            .unwrap(),
            times_glyph_cache: GlyphCache::new(),
            #[cfg(feature = "font-loading")]
            text_font_setting: None,
            text_font: Font::from_slice(
                TEXT_FONT,
                0,
                FontStyle::Normal,
                FontWeight::Normal,
                FontStretch::Normal,
            )
            .unwrap(),
            text_glyph_cache: GlyphCache::new(),
            rectangle: None,
            icons: IconCache {
                game_icon: None,
                split_icons: Vec::new(),
                detailed_timer_icon: None,
            },
            cached_size: None,
            text_buffer: None,
        }
    }

    /// Renders the layout state with the backend provided. A resolution needs
    /// to be provided as well so that the contents are rendered according to
    /// aspect ratio of the render target.
    pub fn render<B: Backend<Path = P, Image = I>>(
        &mut self,
        backend: &mut B,
        resolution: (f32, f32),
        state: &LayoutState,
    ) {
        #[cfg(feature = "font-loading")]
        {
            if self.timer_font_setting != state.timer_font {
                self.timer_font = state
                    .timer_font
                    .as_ref()
                    .and_then(Font::load)
                    .unwrap_or_else(|| {
                        Font::from_slice(
                            TIMER_FONT,
                            0,
                            FontStyle::Normal,
                            FontWeight::Bold,
                            FontStretch::Normal,
                        )
                        .unwrap()
                    });
                self.timer_glyph_cache.clear(backend);
                self.timer_font_setting.clone_from(&state.timer_font);
            }

            if self.times_font_setting != state.times_font {
                self.times_font = state
                    .times_font
                    .as_ref()
                    .and_then(Font::load)
                    .unwrap_or_else(|| {
                        Font::from_slice(
                            TEXT_FONT,
                            0,
                            FontStyle::Normal,
                            FontWeight::Bold,
                            FontStretch::Normal,
                        )
                        .unwrap()
                    });
                self.times_glyph_cache.clear(backend);
                self.times_font_setting.clone_from(&state.times_font);
            }

            if self.text_font_setting != state.text_font {
                self.text_font = state
                    .text_font
                    .as_ref()
                    .and_then(Font::load)
                    .unwrap_or_else(|| {
                        Font::from_slice(
                            TEXT_FONT,
                            0,
                            FontStyle::Normal,
                            FontWeight::Normal,
                            FontStretch::Normal,
                        )
                        .unwrap()
                    });
                self.text_glyph_cache.clear(backend);
                self.text_font_setting.clone_from(&state.text_font);
            }
        }

        match state.direction {
            LayoutDirection::Vertical => self.render_vertical(backend, resolution, state),
            LayoutDirection::Horizontal => self.render_horizontal(backend, resolution, state),
        }
    }

    fn render_vertical<B: Backend<Path = P, Image = I>>(
        &mut self,
        backend: &mut B,
        resolution: (f32, f32),
        state: &LayoutState,
    ) {
        let total_height = state.components.iter().map(component_height).sum::<f32>();

        let cached_total_size = self
            .cached_size
            .get_or_insert(CachedSize::Vertical(total_height));
        match cached_total_size {
            CachedSize::Vertical(cached_total_height) => {
                if *cached_total_height != total_height {
                    backend.resize(
                        resolution.0,
                        resolution.1 / *cached_total_height * total_height,
                    );
                    *cached_total_height = total_height;
                }
            }
            CachedSize::Horizontal(_) => {
                let to_pixels = resolution.1 / TWO_ROW_HEIGHT;
                let new_height = total_height * to_pixels;
                let new_width = DEFAULT_VERTICAL_WIDTH * to_pixels;
                backend.resize(new_width, new_height);
                *cached_total_size = CachedSize::Vertical(total_height);
            }
        }

        let aspect_ratio = resolution.0 as f32 / resolution.1 as f32;

        let mut context = RenderContext {
            backend,
            transform: Transform::identity(),
            rectangle: &mut self.rectangle,
            timer_font: &self.timer_font,
            timer_glyph_cache: &mut self.timer_glyph_cache,
            times_font: &mut self.times_font,
            times_glyph_cache: &mut self.times_glyph_cache,
            text_font: &self.text_font,
            text_glyph_cache: &mut self.text_glyph_cache,
            text_buffer: &mut self.text_buffer,
        };

        // Initially we are in Backend Coordinate Space.
        // We can render the background here from (0, 0) to (1, 1) as we just
        // want to fill all of the background. We don't need to know anything
        // about the aspect ratio or specific sizes.
        context.render_background(&state.background);

        // Now we transform the coordinate space to Renderer Coordinate Space by
        // non-uniformly adjusting for the aspect ratio.
        context.scale_non_uniform_x(aspect_ratio.recip());

        // We scale the coordinate space uniformly such that we have the same
        // scaling as the Component Coordinate Space. This also already is the
        // Component Coordinate Space for the component at (0, 0).
        context.scale(total_height.recip());

        // Calculate the width of the components in component space. In vertical
        // mode, all the components have the same width.
        let width = aspect_ratio * total_height;

        for component in &state.components {
            let height = component_height(component);
            let dim = [width, height];
            render_component(&mut context, &mut self.icons, component, state, dim);
            // We translate the coordinate space to the Component Coordinate
            // Space of the next component by shifting by the height of the
            // current component in the Component Coordinate Space.
            context.translate(0.0, height);
        }
    }

    fn render_horizontal<B: Backend<Path = P, Image = I>>(
        &mut self,
        backend: &mut B,
        resolution: (f32, f32),
        state: &LayoutState,
    ) {
        let total_width = state.components.iter().map(component_width).sum::<f32>();

        let cached_total_size = self
            .cached_size
            .get_or_insert(CachedSize::Horizontal(total_width));
        match cached_total_size {
            CachedSize::Vertical(cached_total_height) => {
                let new_height = resolution.1 * TWO_ROW_HEIGHT / *cached_total_height;
                let new_width = total_width * new_height / TWO_ROW_HEIGHT;
                backend.resize(new_width, new_height);
                *cached_total_size = CachedSize::Horizontal(total_width);
            }
            CachedSize::Horizontal(cached_total_width) => {
                if *cached_total_width != total_width {
                    backend.resize(
                        resolution.0 / *cached_total_width * total_width,
                        resolution.1,
                    );
                    *cached_total_width = total_width;
                }
            }
        }

        let aspect_ratio = resolution.0 as f32 / resolution.1 as f32;

        let mut context = RenderContext {
            backend,
            transform: Transform::identity(),
            rectangle: &mut self.rectangle,
            timer_font: &mut self.timer_font,
            timer_glyph_cache: &mut self.timer_glyph_cache,
            times_font: &mut self.times_font,
            times_glyph_cache: &mut self.times_glyph_cache,
            text_font: &mut self.text_font,
            text_glyph_cache: &mut self.text_glyph_cache,
            text_buffer: &mut self.text_buffer,
        };

        // Initially we are in Backend Coordinate Space.
        // We can render the background here from (0, 0) to (1, 1) as we just
        // want to fill all of the background. We don't need to know anything
        // about the aspect ratio or specific sizes.
        context.render_background(&state.background);

        // Now we transform the coordinate space to Renderer Coordinate Space by
        // non-uniformly adjusting for the aspect ratio.
        context.scale_non_uniform_x(aspect_ratio.recip());

        // We scale the coordinate space uniformly such that we have the same
        // scaling as the Component Coordinate Space. This also already is the
        // Component Coordinate Space for the component at (0, 0). Since all the
        // components use the two row height as their height, we scale by the
        // reciprocal of that.
        context.scale(TWO_ROW_HEIGHT.recip());

        // We don't take the component width we calculate. Instead we use the
        // component width as a ratio of how much of the total actual width to
        // distribute to each of the components. This factor is this adjustment.
        let width_scaling = TWO_ROW_HEIGHT * aspect_ratio / total_width;

        for component in &state.components {
            let width = component_width(component) * width_scaling;
            let height = TWO_ROW_HEIGHT;
            let dim = [width, height];
            render_component(&mut context, &mut self.icons, component, state, dim);
            // We translate the coordinate space to the Component Coordinate
            // Space of the next component by shifting by the width of the
            // current component in the Component Coordinate Space.
            context.translate(width, 0.0);
        }
    }
}

fn render_component<B: Backend>(
    context: &mut RenderContext<'_, B>,
    icons: &mut IconCache<B::Image>,
    component: &ComponentState,
    state: &LayoutState,
    dim: [f32; 2],
) {
    match component {
        ComponentState::BlankSpace(state) => component::blank_space::render(context, dim, state),
        ComponentState::DetailedTimer(component) => component::detailed_timer::render(
            context,
            dim,
            component,
            state,
            &mut icons.detailed_timer_icon,
        ),
        ComponentState::Graph(component) => {
            component::graph::render(context, dim, component, state)
        }
        ComponentState::KeyValue(component) => {
            component::key_value::render(context, dim, component, state)
        }
        ComponentState::Separator(component) => {
            component::separator::render(context, dim, component, state)
        }
        ComponentState::Splits(component) => {
            component::splits::render(context, dim, component, state, &mut icons.split_icons)
        }
        ComponentState::Text(component) => component::text::render(context, dim, component, state),
        ComponentState::Timer(component) => {
            component::timer::render(context, dim, component);
        }
        ComponentState::Title(component) => {
            component::title::render(context, dim, component, state, &mut icons.game_icon)
        }
    }
}

struct RenderContext<'b, B: Backend> {
    transform: Transform,
    backend: &'b mut B,
    rectangle: &'b mut Option<B::Path>,
    timer_font: &'b Font<'static>,
    timer_glyph_cache: &'b mut GlyphCache<B::Path>,
    text_font: &'b Font<'static>,
    text_glyph_cache: &'b mut GlyphCache<B::Path>,
    times_font: &'b Font<'static>,
    times_glyph_cache: &'b mut GlyphCache<B::Path>,
    text_buffer: &'b mut Option<UnicodeBuffer>,
}

impl<B: Backend> RenderContext<'_, B> {
    fn backend_render_rectangle(&mut self, [x1, y1]: Pos, [x2, y2]: Pos, shader: Shader) {
        let transform = self
            .transform
            .pre_translate([x1, y1].into())
            .pre_scale(x2 - x1, y2 - y1);

        let rectangle = self.rectangle.get_or_insert_with({
            let backend = &mut self.backend;
            move || {
                let mut builder = backend.build_path();
                builder.move_to(0.0, 0.0);
                builder.line_to(0.0, 1.0);
                builder.line_to(1.0, 1.0);
                builder.line_to(1.0, 0.0);
                builder.close();
                builder.finish()
            }
        });

        self.backend.render_fill_path(rectangle, shader, transform);
    }

    // fn create_path(&mut self, path: &Path) -> B::Path {
    //     self.backend.create_path(path)
    // }

    fn render_path(&mut self, path: &B::Path, color: Color) {
        self.backend
            .render_fill_path(path, solid(&color), self.transform)
    }

    fn render_stroke_path(&mut self, path: &B::Path, color: Color, stroke_width: f32) {
        self.backend
            .render_stroke_path(path, stroke_width, decode_color(&color), self.transform)
    }

    fn create_icon(&mut self, image_data: &[u8]) -> Option<Icon<B::Image>> {
        if image_data.is_empty() {
            return None;
        }

        let image = image::load_from_memory(image_data).ok()?.to_rgba8();
        let texture = self
            .backend
            .create_image(image.width(), image.height(), &image);

        Some(Icon {
            texture,
            aspect_ratio: image.width() as f32 / image.height() as f32,
        })
    }

    fn free_path(&mut self, path: B::Path) {
        self.backend.free_path(path)
    }

    fn scale(&mut self, factor: f32) {
        self.transform = self.transform.pre_scale(factor, factor);
    }

    fn scale_non_uniform_x(&mut self, x: f32) {
        self.transform = self.transform.pre_scale(x, 1.0);
    }

    fn translate(&mut self, x: f32, y: f32) {
        self.transform = self.transform.pre_translate([x, y].into());
    }

    fn render_rectangle(&mut self, top_left: Pos, bottom_right: Pos, gradient: &Gradient) {
        if let Some(colors) = decode_gradient(gradient) {
            self.backend_render_rectangle(top_left, bottom_right, colors);
        }
    }

    fn render_icon(
        &mut self,
        [mut x, mut y]: Pos,
        [mut width, mut height]: Pos,
        icon: &Icon<B::Image>,
    ) {
        let box_aspect_ratio = width / height;
        let aspect_ratio_diff = box_aspect_ratio / icon.aspect_ratio;

        if aspect_ratio_diff > 1.0 {
            let new_width = width / aspect_ratio_diff;
            let diff_width = width - new_width;
            x += 0.5 * diff_width;
            width = new_width;
        } else if aspect_ratio_diff < 1.0 {
            let new_height = height * aspect_ratio_diff;
            let diff_height = height - new_height;
            y += 0.5 * diff_height;
            height = new_height;
        }

        let transform = self
            .transform
            .pre_translate([x, y].into())
            .pre_scale(width, height);

        // TODO: Deduplicate
        let rectangle = self.rectangle.get_or_insert_with({
            let backend = &mut self.backend;
            move || {
                let mut builder = backend.build_path();
                builder.move_to(0.0, 0.0);
                builder.line_to(0.0, 1.0);
                builder.line_to(1.0, 1.0);
                builder.line_to(1.0, 0.0);
                builder.close();
                builder.finish()
            }
        });

        self.backend
            .render_image(&icon.texture, rectangle, transform);
    }

    fn render_background(&mut self, background: &Gradient) {
        self.render_rectangle([0.0, 0.0], [1.0, 1.0], background);
    }

    fn render_key_value_component(
        &mut self,
        key: &str,
        abbreviations: &[Cow<'_, str>],
        value: &str,
        [width, height]: [f32; 2],
        key_color: Color,
        value_color: Color,
        display_two_rows: bool,
    ) {
        let left_of_value_x = self.render_numbers(
            value,
            [width - PADDING, height + TEXT_ALIGN_BOTTOM],
            DEFAULT_TEXT_SIZE,
            solid(&value_color),
        );
        let end_x = if display_two_rows {
            width
        } else {
            left_of_value_x
        };
        let key = self.choose_abbreviation(
            iter::once(key).chain(abbreviations.iter().map(|x| &**x)),
            DEFAULT_TEXT_SIZE,
            end_x - BOTH_PADDINGS,
        );
        self.render_text_ellipsis(
            key,
            [PADDING, TEXT_ALIGN_TOP],
            DEFAULT_TEXT_SIZE,
            solid(&key_color),
            end_x - PADDING,
        );
    }

    fn render_text_ellipsis(
        &mut self,
        text: &str,
        pos: Pos,
        scale: f32,
        shader: Shader,
        max_x: f32,
    ) -> f32 {
        let mut cursor = font::Cursor::new(pos);

        let mut buffer = self.text_buffer.take().unwrap_or_default();
        buffer.push_str(text.trim());
        buffer.guess_segment_properties();

        let font = self.text_font.scale(scale);
        let glyphs = font.shape(buffer);

        font::render(
            glyphs.left_aligned(&mut cursor, max_x),
            shader,
            &font,
            self.text_glyph_cache,
            &self.transform,
            self.backend,
        );

        *self.text_buffer = Some(glyphs.into_buffer());

        cursor.x
    }

    fn render_text_centered(
        &mut self,
        text: &str,
        min_x: f32,
        max_x: f32,
        pos: Pos,
        scale: f32,
        shader: Shader,
    ) {
        let mut cursor = font::Cursor::new(pos);

        let mut buffer = self.text_buffer.take().unwrap_or_default();
        buffer.push_str(text.trim());
        buffer.guess_segment_properties();

        let font = self.text_font.scale(scale);
        let glyphs = font.shape(buffer);

        font::render(
            glyphs.centered(&mut cursor, min_x, max_x),
            shader,
            &font,
            self.text_glyph_cache,
            &self.transform,
            self.backend,
        );

        *self.text_buffer = Some(glyphs.into_buffer());
    }

    fn render_text_right_align(&mut self, text: &str, pos: Pos, scale: f32, shader: Shader) -> f32 {
        let mut cursor = font::Cursor::new(pos);

        let mut buffer = self.text_buffer.take().unwrap_or_default();
        buffer.push_str(text.trim());
        buffer.guess_segment_properties();

        let font = self.text_font.scale(scale);
        let glyphs = font.shape(buffer);

        font::render(
            glyphs.right_aligned(&mut cursor),
            shader,
            &font,
            self.text_glyph_cache,
            &self.transform,
            self.backend,
        );

        *self.text_buffer = Some(glyphs.into_buffer());

        cursor.x
    }

    fn render_text_align(
        &mut self,
        text: &str,
        min_x: f32,
        max_x: f32,
        pos: Pos,
        scale: f32,
        centered: bool,
        shader: Shader,
    ) {
        if centered {
            self.render_text_centered(text, min_x, max_x, pos, scale, shader);
        } else {
            self.render_text_ellipsis(text, pos, scale, shader, max_x);
        }
    }

    fn render_numbers(&mut self, text: &str, pos: Pos, scale: f32, shader: Shader) -> f32 {
        let mut cursor = font::Cursor::new(pos);

        let mut buffer = self.text_buffer.take().unwrap_or_default();
        buffer.push_str(text.trim());
        buffer.guess_segment_properties();

        let font = self.times_font.scale(scale);
        let glyphs = font.shape_tabular_numbers(buffer);

        font::render(
            glyphs.tabular_numbers(&mut cursor),
            shader,
            &font,
            self.times_glyph_cache,
            &self.transform,
            self.backend,
        );

        *self.text_buffer = Some(glyphs.into_buffer());

        cursor.x
    }

    fn render_timer(&mut self, text: &str, pos: Pos, scale: f32, shader: Shader) -> f32 {
        let mut cursor = font::Cursor::new(pos);

        let mut buffer = self.text_buffer.take().unwrap_or_default();
        buffer.push_str(text.trim());
        buffer.guess_segment_properties();

        let font = self.timer_font.scale(scale);
        let glyphs = font.shape_tabular_numbers(buffer);

        font::render(
            glyphs.tabular_numbers(&mut cursor),
            shader,
            &font,
            self.timer_glyph_cache,
            &self.transform,
            self.backend,
        );

        *self.text_buffer = Some(glyphs.into_buffer());

        cursor.x
    }

    fn choose_abbreviation<'a>(
        &mut self,
        abbreviations: impl IntoIterator<Item = &'a str>,
        font_size: f32,
        max_width: f32,
    ) -> &'a str {
        let mut abbreviations = abbreviations.into_iter();
        let abbreviation = abbreviations.next().unwrap_or("");
        let width = self.measure_text(abbreviation, font_size);
        let (mut total_longest, mut total_longest_width) = (abbreviation, width);
        let (mut within_longest, mut within_longest_width) = if width <= max_width {
            (abbreviation, width)
        } else {
            ("", 0.0)
        };

        for abbreviation in abbreviations {
            let width = self.measure_text(abbreviation, font_size);
            if width <= max_width && width > within_longest_width {
                within_longest_width = width;
                within_longest = abbreviation;
            }
            if width > total_longest_width {
                total_longest_width = width;
                total_longest = abbreviation;
            }
        }

        if within_longest.is_empty() {
            total_longest
        } else {
            within_longest
        }
    }

    fn measure_text(&mut self, text: &str, scale: f32) -> f32 {
        let mut buffer = self.text_buffer.take().unwrap_or_default();
        buffer.push_str(text.trim());
        buffer.guess_segment_properties();

        let glyphs = self.text_font.scale(scale).shape(buffer);
        let width = glyphs.width();

        *self.text_buffer = Some(glyphs.into_buffer());

        width
    }

    fn measure_numbers(&mut self, text: &str, scale: f32) -> f32 {
        let mut cursor = font::Cursor::new([0.0; 2]);

        let mut buffer = self.text_buffer.take().unwrap_or_default();
        buffer.push_str(text.trim());
        buffer.guess_segment_properties();

        let glyphs = self.times_font.scale(scale).shape_tabular_numbers(buffer);

        // Iterate over all glyphs, to move the cursor forward.
        glyphs.tabular_numbers(&mut cursor).for_each(drop);

        // Wherever we end up is our width.
        let width = -cursor.x;

        *self.text_buffer = Some(glyphs.into_buffer());

        width
    }
}

fn decode_gradient(gradient: &Gradient) -> Option<Shader> {
    Some(match gradient {
        Gradient::Transparent => return None,
        Gradient::Horizontal(left, right) => {
            let left = decode_color(left);
            let right = decode_color(right);
            Shader::HorizontalGradient(left, right)
        }
        Gradient::Vertical(top, bottom) => {
            let top = decode_color(top);
            let bottom = decode_color(bottom);
            Shader::VerticalGradient(top, bottom)
        }
        Gradient::Plain(plain) => {
            let plain = decode_color(plain);
            Shader::SolidColor(plain)
        }
    })
}

fn decode_color(color: &Color) -> [f32; 4] {
    let (r, g, b, a) = color.rgba.into();
    [r, g, b, a]
}

fn solid(color: &Color) -> Shader {
    Shader::SolidColor(decode_color(color))
}

fn component_width(component: &ComponentState) -> f32 {
    match component {
        ComponentState::BlankSpace(state) => state.size as f32 * PSEUDO_PIXELS,
        ComponentState::DetailedTimer(_) => 7.0,
        ComponentState::Graph(_) => 7.0,
        ComponentState::KeyValue(_) => 6.0,
        ComponentState::Separator(_) => SEPARATOR_THICKNESS,
        ComponentState::Splits(state) => {
            let column_count = 2.0; // FIXME: Not always 2.
            let split_width = 2.0 + column_count * component::splits::COLUMN_WIDTH;
            state.splits.len() as f32 * split_width
        }
        ComponentState::Text(_) => 6.0,
        ComponentState::Timer(_) => 8.25,
        ComponentState::Title(_) => 8.0,
    }
}

fn component_height(component: &ComponentState) -> f32 {
    match component {
        ComponentState::BlankSpace(state) => state.size as f32 * PSEUDO_PIXELS,
        ComponentState::DetailedTimer(_) => 2.5,
        ComponentState::Graph(state) => state.height as f32 * PSEUDO_PIXELS,
        ComponentState::KeyValue(state) => {
            if state.display_two_rows {
                TWO_ROW_HEIGHT
            } else {
                DEFAULT_COMPONENT_HEIGHT
            }
        }
        ComponentState::Separator(_) => SEPARATOR_THICKNESS,
        ComponentState::Splits(state) => {
            state.splits.len() as f32
                * if state.display_two_rows {
                    TWO_ROW_HEIGHT
                } else {
                    DEFAULT_COMPONENT_HEIGHT
                }
                + if state.column_labels.is_some() {
                    DEFAULT_COMPONENT_HEIGHT
                } else {
                    0.0
                }
        }
        ComponentState::Text(state) => {
            if state.display_two_rows {
                TWO_ROW_HEIGHT
            } else {
                DEFAULT_COMPONENT_HEIGHT
            }
        }
        ComponentState::Timer(state) => state.height as f32 * PSEUDO_PIXELS,
        ComponentState::Title(_) => TWO_ROW_HEIGHT,
    }
}
