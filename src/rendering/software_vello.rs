//! Provides a software renderer that can be used without a GPU, using Vello CPU.

use super::{
    FillShader, FontKind, Scene, SceneManager, Transform,
    consts::SHADOW_OFFSET,
    default_text_engine::{Font, Label, TextEngine},
    entity::Entity,
    resource::{self, ResourceAllocator},
};
use crate::{layout::LayoutState, rendering::Background, settings, settings::ImageCache};
use alloc::{rc::Rc, sync::Arc};
use core::mem;
use vello_cpu::{
    Image as VelloImageBrush, ImageSource, Pixmap, RenderContext, RenderSettings,
    color::{AlphaColor, PremulRgba8, Srgb},
    kurbo::{Affine, BezPath, Point, Rect, Shape, Stroke},
    peniko::{
        BlendMode, ColorStop, ColorStops, Compose, Extend, Fill, Gradient, ImageQuality,
        ImageSampler, LinearGradientPosition, Mix,
    },
};

#[cfg(feature = "image")]
use crate::settings::{BLUR_FACTOR, BackgroundImage};
#[cfg(feature = "image")]
use image::{ImageBuffer, imageops::FilterType};
#[cfg(feature = "image")]
pub use image::{self, RgbaImage};

struct VelloPathBuilder(BezPath);

type VelloPath = Rc<BezPath>;
type VelloImage = Rc<Image>;
type VelloFont = Font;
type VelloLabel = Label<VelloPath>;

type PaintType = vello_cpu::PaintType;

struct Image {
    pixmap: Arc<Pixmap>,
    aspect_ratio: f32,
}

impl resource::Image for VelloImage {
    fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }
}

impl resource::PathBuilder for VelloPathBuilder {
    type Path = VelloPath;

    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(Point::new(x as f64, y as f64))
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(Point::new(x as f64, y as f64))
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(
            Point::new(x1 as f64, y1 as f64),
            Point::new(x as f64, y as f64),
        )
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.curve_to(
            Point::new(x1 as f64, y1 as f64),
            Point::new(x2 as f64, y2 as f64),
            Point::new(x as f64, y as f64),
        )
    }

    fn close(&mut self) {
        self.0.close_path()
    }

    fn finish(self) -> Self::Path {
        Rc::new(self.0)
    }
}

const fn convert_color(&[r, g, b, a]: &[f32; 4]) -> AlphaColor<Srgb> {
    AlphaColor::new([r, g, b, a])
}

const fn convert_transform(transform: &Transform) -> Affine {
    Affine::new([
        transform.scale_x as f64,
        0.0,
        0.0,
        transform.scale_y as f64,
        transform.x as f64,
        transform.y as f64,
    ])
}

struct VelloAllocator {
    text_engine: TextEngine<VelloPath>,
}

impl ResourceAllocator for VelloAllocator {
    type PathBuilder = VelloPathBuilder;
    type Path = VelloPath;
    type Image = VelloImage;
    type Font = VelloFont;
    type Label = VelloLabel;

    fn path_builder(&mut self) -> Self::PathBuilder {
        VelloPathBuilder(BezPath::new())
    }

    fn create_image(&mut self, data: &[u8]) -> Option<Self::Image> {
        #[cfg(feature = "image")]
        {
            let mut buf = image::load_from_memory(data).ok()?.to_rgba8();

            // Premultiplication
            for [r, g, b, a] in bytemuck::cast_slice_mut::<u8, [u8; 4]>(&mut buf) {
                let a = *a as u16;
                *r = ((*r as u16 * a) / 255) as u8;
                *g = ((*g as u16 * a) / 255) as u8;
                *b = ((*b as u16 * a) / 255) as u8;
            }

            let (width, height) = (buf.width(), buf.height());
            let width_u16 = u16::try_from(width).ok()?;
            let height_u16 = u16::try_from(height).ok()?;

            let mut pixels = Vec::with_capacity(width as usize * height as usize);
            for chunk in buf.chunks_exact(4) {
                pixels.push(PremulRgba8 {
                    r: chunk[0],
                    g: chunk[1],
                    b: chunk[2],
                    a: chunk[3],
                });
            }

            let pixmap = Pixmap::from_parts(pixels, width_u16, height_u16);

            Some(Rc::new(Image {
                pixmap: Arc::new(pixmap),
                aspect_ratio: width as f32 / height as f32,
            }))
        }
        #[cfg(not(feature = "image"))]
        {
            None
        }
    }

    fn create_font(&mut self, font: Option<&settings::Font>, kind: FontKind) -> Self::Font {
        self.text_engine.create_font(font, kind)
    }

    fn create_label(
        &mut self,
        text: &str,
        font: &mut Self::Font,
        max_width: Option<f32>,
    ) -> Self::Label {
        self.text_engine
            .create_label(VelloPathBuilder::default, text, font, max_width)
    }

    fn update_label(
        &mut self,
        label: &mut Self::Label,
        text: &str,
        font: &mut Self::Font,
        max_width: Option<f32>,
    ) {
        self.text_engine
            .update_label(VelloPathBuilder::default, label, text, font, max_width)
    }
}

impl Default for VelloPathBuilder {
    fn default() -> Self {
        Self(BezPath::new())
    }
}

fn render_settings_for_size(width: u16, height: u16) -> RenderSettings {
    const THREADING_MIN_PIXELS: u32 = 512 * 512;
    let pixels = u32::from(width) * u32::from(height);
    let mut settings = RenderSettings::default();
    if pixels < THREADING_MIN_PIXELS {
        settings.num_threads = 0;
    }
    settings
}

/// The software renderer allows rendering layouts entirely on the CPU. This is
/// surprisingly fast and can be considered the default renderer.
pub struct Renderer {
    allocator: VelloAllocator,
    scene_manager: SceneManager<VelloPath, VelloImage, VelloFont, VelloLabel>,
    #[cfg(feature = "image")]
    blurred_background_image: Option<(BackgroundImage<usize>, Arc<Pixmap>)>,
    background_pixmap: Pixmap,
    frame_pixmap: Pixmap,
    background_context: RenderContext,
    top_context: RenderContext,
    min_y: f32,
    max_y: f32,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    /// Creates a new software renderer.
    pub fn new() -> Self {
        let mut allocator = VelloAllocator {
            text_engine: TextEngine::new(),
        };
        let scene_manager = SceneManager::new(&mut allocator);
        let background_pixmap = Pixmap::new(1, 1);
        let frame_pixmap = Pixmap::new(1, 1);
        Self {
            allocator,
            scene_manager,
            #[cfg(feature = "image")]
            blurred_background_image: None,
            background_pixmap,
            frame_pixmap,
            background_context: RenderContext::new_with(1, 1, render_settings_for_size(1, 1)),
            top_context: RenderContext::new_with(1, 1, render_settings_for_size(1, 1)),
            min_y: f32::INFINITY,
            max_y: f32::NEG_INFINITY,
        }
    }

    /// Renders the layout state provided with the chosen resolution. It may
    /// detect that the layout got resized. In that case it returns the new
    /// ideal size. This is just a hint and can be ignored entirely. The image
    /// is always rendered with the resolution provided. By default the renderer
    /// will try not to redraw parts of the image that haven't changed.
    pub fn render(
        &mut self,
        state: &LayoutState,
        image_cache: &ImageCache,
        [width, height]: [u32; 2],
    ) -> Option<[f32; 2]> {
        let width_u16 = u16::try_from(width).ok()?;
        let height_u16 = u16::try_from(height).ok()?;

        if self.background_pixmap.width() != width_u16 || self.background_pixmap.height() != height_u16 {
            self.background_pixmap = Pixmap::new(width_u16, height_u16);
            self.frame_pixmap = Pixmap::new(width_u16, height_u16);
            let settings = render_settings_for_size(width_u16, height_u16);
            self.background_context = RenderContext::new_with(width_u16, height_u16, settings);
            self.top_context = RenderContext::new_with(width_u16, height_u16, settings);
            #[cfg(feature = "image")]
            {
                self.blurred_background_image = None;
            }
        }

        let new_resolution = self.scene_manager.update_scene(
            &mut self.allocator,
            [width as _, height as _],
            state,
            image_cache,
        );

        let scene = self.scene_manager.scene();
        let rectangle = scene.rectangle();
        let rectangle = rectangle.as_ref();

        let bottom_layer_changed = scene.bottom_layer_changed();

        if bottom_layer_changed {
            if fill_background_fast_path(
                scene,
                width_u16,
                height_u16,
                self.background_pixmap.data_as_u8_slice_mut(),
            ) {
                // Background already filled and there is no bottom layer to draw.
            } else {
                self.background_context.reset();
                fill_background(
                    scene,
                    #[cfg(feature = "image")]
                    &mut self.blurred_background_image,
                    &mut self.background_context,
                    width as f64,
                    height as f64,
                    rectangle,
                );
                render_layer(&mut self.background_context, scene.bottom_layer(), rectangle);
                self.background_context.flush();
                self.background_context.render_to_pixmap(&mut self.background_pixmap);
            }
        }

        let top_layer = scene.top_layer();
        let [min_y, max_y] = calculate_bounds(top_layer);
        let row_bounds = row_range(min_y, max_y, height);

        let min_y = mem::replace(&mut self.min_y, min_y).min(min_y);
        let max_y = mem::replace(&mut self.max_y, max_y).max(max_y);

        if bottom_layer_changed {
            self.frame_pixmap
                .data_as_u8_slice_mut()
                .copy_from_slice(self.background_pixmap.data_as_u8_slice());
        } else if let Some((min_row, max_row)) = row_range(min_y, max_y, height) {
            copy_rows(
                self.background_pixmap.data_as_u8_slice(),
                self.frame_pixmap.data_as_u8_slice_mut(),
                width as usize,
                min_row,
                max_row,
            );
        }

        if row_bounds.is_some() {
            self.top_context.reset();
            render_layer(&mut self.top_context, top_layer, rectangle);
            self.top_context.flush();
            self.top_context.render_to_pixmap(&mut self.frame_pixmap);
        }

        new_resolution
    }

    /// Accesses the image as a byte slice of RGBA8 encoded pixels (red, green,
    /// blue, alpha with each channel being an u8).
    pub fn image_data(&self) -> &[u8] {
        self.frame_pixmap.data_as_u8_slice()
    }

    /// Turns the whole renderer into the underlying image buffer of RGBA8
    /// encoded pixels (red, green, blue, alpha with each channel being an u8).
    pub fn into_image_data(self) -> Vec<u8> {
        self.frame_pixmap.data_as_u8_slice().to_vec()
    }

    /// Accesses the image.
    #[cfg(feature = "image")]
    pub fn image(&self) -> ImageBuffer<image::Rgba<u8>, &[u8]> {
        ImageBuffer::from_raw(
            self.frame_pixmap.width().into(),
            self.frame_pixmap.height().into(),
            self.frame_pixmap.data_as_u8_slice(),
        )
        .unwrap()
    }

    /// Turns the whole renderer into the underlying image.
    #[cfg(feature = "image")]
    pub fn into_image(self) -> image::RgbaImage {
        image::RgbaImage::from_raw(
            self.frame_pixmap.width().into(),
            self.frame_pixmap.height().into(),
            self.frame_pixmap.data_as_u8_slice().to_vec(),
        )
        .unwrap()
    }
}

fn render_layer(
    context: &mut RenderContext,
    layer: &[Entity<VelloPath, VelloImage, VelloLabel>],
    rectangle: &BezPath,
) {
    for entity in layer {
        match entity {
            Entity::FillPath(path, shader, transform) => {
                if path.is_empty() {
                    continue;
                }
                let paint = convert_shader(
                    shader,
                    path,
                    |path| {
                        let bounds = path.bounding_box();
                        [bounds.y0 as f32, bounds.y1 as f32]
                    },
                    |path| {
                        let bounds = path.bounding_box();
                        [bounds.x0 as f32, bounds.x1 as f32]
                    },
                );

                context.set_transform(convert_transform(transform));
                context.set_paint_transform(Affine::IDENTITY);
                context.set_fill_rule(Fill::NonZero);
                context.set_paint(paint);
                context.fill_path(path.as_ref());
            }
            Entity::StrokePath(path, stroke_width, color, transform) => {
                if path.is_empty() {
                    continue;
                }
                context.set_transform(convert_transform(transform));
                context.set_paint_transform(Affine::IDENTITY);
                context.set_paint(convert_color(color));
                context.set_stroke(Stroke::new(*stroke_width as f64));
                context.stroke_path(path.as_ref());
            }
            Entity::Image(image, transform) => {
                let pixmap = &image.pixmap;
                let paint = VelloImageBrush {
                    image: ImageSource::Pixmap(pixmap.clone()),
                    sampler: ImageSampler {
                        x_extend: Extend::Pad,
                        y_extend: Extend::Pad,
                        quality: ImageQuality::Medium,
                        alpha: 1.0,
                    },
                };

                context.set_transform(convert_transform(transform));
                context.set_paint(paint);
                context.set_paint_transform(Affine::scale_non_uniform(
                    1.0 / pixmap.width() as f64,
                    1.0 / pixmap.height() as f64,
                ));
                context.set_fill_rule(Fill::NonZero);
                context.fill_path(rectangle);
                context.set_paint_transform(Affine::IDENTITY);
            }
            Entity::Label(label, shader, text_shadow, transform) => {
                let label = &*label.read().unwrap();

                let paint = convert_shader(
                    shader,
                    label,
                    |label| {
                        let (mut top, mut bottom) = (f32::INFINITY, f32::NEG_INFINITY);
                        for glyph in label.glyphs() {
                            let bounds = glyph.path.bounding_box();
                            top = top.min(bounds.y0 as f32);
                            bottom = bottom.max(bounds.y1 as f32);
                        }
                        if bottom < top {
                            [0.0, 0.0]
                        } else {
                            [top, bottom]
                        }
                    },
                    |label| {
                        let (mut left, mut right) = (f32::INFINITY, f32::NEG_INFINITY);
                        for glyph in label.glyphs() {
                            let bounds = glyph.path.bounding_box();
                            left = left.min(bounds.x0 as f32);
                            right = right.max(bounds.x1 as f32);
                        }
                        if right < left {
                            [0.0, 0.0]
                        } else {
                            [left, right]
                        }
                    },
                );

                if let Some(text_shadow) = text_shadow {
                    let alpha = match shader {
                        FillShader::SolidColor([.., a]) => *a,
                        FillShader::VerticalGradient([.., a1], [.., a2])
                        | FillShader::HorizontalGradient([.., a1], [.., a2]) => 0.5 * (a1 + a2),
                    };
                    let mut shadow_color = *text_shadow;
                    shadow_color[3] *= alpha;
                    let color = convert_color(&shadow_color);
                    let transform = transform.pre_translate(SHADOW_OFFSET, SHADOW_OFFSET);

                    for glyph in label.glyphs() {
                        if glyph.path.is_empty() {
                            continue;
                        }
                        let transform = transform
                            .pre_translate(glyph.x, glyph.y)
                            .pre_scale(glyph.scale, glyph.scale);

                        context.set_transform(convert_transform(&transform));
                        context.set_paint_transform(Affine::IDENTITY);
                        context.set_fill_rule(Fill::NonZero);
                        context.set_paint(color);
                        context.fill_path(glyph.path.as_ref());
                    }
                }

                for glyph in label.glyphs() {
                    if glyph.path.is_empty() {
                        continue;
                    }
                    let transform = transform
                        .pre_translate(glyph.x, glyph.y)
                        .pre_scale(glyph.scale, glyph.scale);

                    context.set_transform(convert_transform(&transform));
                    context.set_paint_transform(Affine::IDENTITY);
                    context.set_fill_rule(Fill::NonZero);

                    if let Some(color) = &glyph.color {
                        context.set_paint(convert_color(color));
                    } else {
                        context.set_paint(paint.clone());
                    }

                    context.fill_path(glyph.path.as_ref());
                }
            }
        }
    }
}

fn convert_shader<T>(
    shader: &FillShader,
    has_bounds: &T,
    calculate_top_bottom: impl FnOnce(&T) -> [f32; 2],
    calculate_left_right: impl FnOnce(&T) -> [f32; 2],
) -> PaintType {
    match shader {
        FillShader::SolidColor(color) => convert_color(color).into(),
        FillShader::VerticalGradient(top, bottom) => {
            let [bound_top, bound_bottom] = calculate_top_bottom(has_bounds);
            let stops = [
                ColorStop::from((0.0, convert_color(top))),
                ColorStop::from((1.0, convert_color(bottom))),
            ];
            Gradient {
                kind: LinearGradientPosition {
                    start: Point::new(0.0, bound_top as f64),
                    end: Point::new(0.0, bound_bottom as f64),
                }
                .into(),
                stops: ColorStops::from(stops.as_slice()),
                extend: Extend::Pad,
                ..Default::default()
            }
            .into()
        }
        FillShader::HorizontalGradient(left, right) => {
            let [bound_left, bound_right] = calculate_left_right(has_bounds);
            let stops = [
                ColorStop::from((0.0, convert_color(left))),
                ColorStop::from((1.0, convert_color(right))),
            ];
            Gradient {
                kind: LinearGradientPosition {
                    start: Point::new(bound_left as f64, 0.0),
                    end: Point::new(bound_right as f64, 0.0),
                }
                .into(),
                stops: ColorStops::from(stops.as_slice()),
                extend: Extend::Pad,
                ..Default::default()
            }
            .into()
        }
    }
}

fn fill_background(
    scene: &Scene<VelloPath, VelloImage, VelloLabel>,
    #[cfg(feature = "image")] blurred_background_image: &mut Option<(BackgroundImage<usize>, Arc<Pixmap>)>,
    context: &mut RenderContext,
    width: f64,
    height: f64,
    rectangle: &BezPath,
) {
    #[cfg(feature = "image")]
    update_blurred_background_image(scene, blurred_background_image);

    match scene.background() {
        Some(background) => match background {
            Background::Shader(shader) => {
                let background_rect = Rect::new(0.0, 0.0, width, height);
                context.set_transform(Affine::IDENTITY);
                context.set_paint_transform(Affine::IDENTITY);
                context.set_fill_rule(Fill::NonZero);
                context.set_blend_mode(BlendMode::from(Compose::Copy));

                match shader {
                    FillShader::SolidColor(color) => {
                        context.set_paint(convert_color(color));
                        context.fill_rect(&background_rect);
                    }
                    FillShader::VerticalGradient(top, bottom) => {
                        let stops = [
                            ColorStop::from((0.0, convert_color(top))),
                            ColorStop::from((1.0, convert_color(bottom))),
                        ];
                        let gradient = Gradient {
                            kind: LinearGradientPosition {
                                start: Point::new(0.0, 0.0),
                                end: Point::new(0.0, height),
                            }
                            .into(),
                            stops: ColorStops::from(stops.as_slice()),
                            extend: Extend::Pad,
                            ..Default::default()
                        };
                        context.set_paint(gradient);
                        context.fill_rect(&background_rect);
                    }
                    FillShader::HorizontalGradient(left, right) => {
                        let stops = [
                            ColorStop::from((0.0, convert_color(left))),
                            ColorStop::from((1.0, convert_color(right))),
                        ];
                        let gradient = Gradient {
                            kind: LinearGradientPosition {
                                start: Point::new(0.0, 0.0),
                                end: Point::new(width, 0.0),
                            }
                            .into(),
                            stops: ColorStops::from(stops.as_slice()),
                            extend: Extend::Pad,
                            ..Default::default()
                        };
                        context.set_paint(gradient);
                        context.fill_rect(&background_rect);
                    }
                }

                context.set_blend_mode(BlendMode::default());
            }
            Background::Image(image, transform) => {
                #[cfg(feature = "image")]
                let pixmap = if image.blur != 0.0 {
                    blurred_background_image
                        .as_ref()
                        .map(|(_, pixmap)| pixmap.clone())
                        .unwrap()
                } else {
                    image.image.pixmap.clone()
                };
                #[cfg(not(feature = "image"))]
                let pixmap = image.image.pixmap.clone();

                context.set_transform(convert_transform(transform));
                context.set_paint(VelloImageBrush {
                    image: ImageSource::Pixmap(pixmap.clone()),
                    sampler: ImageSampler {
                        x_extend: Extend::Pad,
                        y_extend: Extend::Pad,
                        quality: ImageQuality::Medium,
                        alpha: image.opacity,
                    },
                });
                context.set_paint_transform(Affine::scale_non_uniform(
                    1.0 / pixmap.width() as f64,
                    1.0 / pixmap.height() as f64,
                ));
                context.set_fill_rule(Fill::NonZero);
                context.set_blend_mode(BlendMode::from(Compose::Copy));
                context.fill_path(rectangle);
                context.set_paint_transform(Affine::IDENTITY);

                if image.brightness != 1.0 {
                    let brightness = image.brightness;
                    let color = AlphaColor::<Srgb>::new([brightness, brightness, brightness, 1.0]);
                    context.set_paint(color);
                    context.set_blend_mode(BlendMode::new(Mix::Multiply, Compose::SrcOver));
                    context.fill_path(rectangle);
                }

                context.set_blend_mode(BlendMode::default());
            }
        },
        None => {
            context.set_transform(Affine::IDENTITY);
            context.set_paint_transform(Affine::IDENTITY);
            context.set_fill_rule(Fill::NonZero);
            context.set_blend_mode(BlendMode::from(Compose::Copy));
            context.set_paint(AlphaColor::<Srgb>::new([0.0, 0.0, 0.0, 0.0]));
            context.fill_rect(&Rect::new(0.0, 0.0, width, height));
            context.set_blend_mode(BlendMode::default());
        }
    }
}

fn fill_background_fast_path(
    scene: &Scene<VelloPath, VelloImage, VelloLabel>,
    width: u16,
    height: u16,
    target: &mut [u8],
) -> bool {
    if !scene.bottom_layer().is_empty() {
        return false;
    }

    let Some(Background::Shader(FillShader::SolidColor(color))) = scene.background() else {
        return false;
    };

    if target.len() != (width as usize) * (height as usize) * 4 {
        return false;
    }

    let alpha = color[3].clamp(0.0, 1.0);
    let premul = PremulRgba8 {
        r: (color[0].clamp(0.0, 1.0) * alpha * 255.0) as u8,
        g: (color[1].clamp(0.0, 1.0) * alpha * 255.0) as u8,
        b: (color[2].clamp(0.0, 1.0) * alpha * 255.0) as u8,
        a: (alpha * 255.0) as u8,
    };
    let pixel = premul.to_u8_array();
    for chunk in target.chunks_exact_mut(4) {
        chunk.copy_from_slice(&pixel);
    }

    true
}

#[cfg(feature = "image")]
fn update_blurred_background_image(
    scene: &Scene<VelloPath, VelloImage, VelloLabel>,
    blurred_background_image: &mut Option<(BackgroundImage<usize>, Arc<Pixmap>)>,
) {
    match scene.background() {
        Some(Background::Image(image, _)) if image.blur != 0.0 => {
            let current_key = image.map(image.image.id);
            if !blurred_background_image
                .as_ref()
                .is_some_and(|(key, _)| &current_key == key)
            {
                let original_image = ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                    image.image.pixmap.width().into(),
                    image.image.pixmap.height().into(),
                    image.image.pixmap.data_as_u8_slice(),
                )
                .unwrap();

                // Formula to calculate the sigma as specified
                let dim = original_image.width().max(original_image.height()) as f32;
                let sigma = BLUR_FACTOR * image.blur * dim;

                // For large blurs the calculation is actually very expensive,
                // but we can get around that because large blurs don't require
                // high resolutions in the first place. So we simply scale down
                // the image based on the sigma to a smaller size and then blur
                // the image. For the scaled down image we always use a sigma of
                // 2.0, so scaling the image by 2.0 / sigma should resulting in
                // the same amount of blur. Of course we never want to scale the
                // image up, so in case the scale factor would end up in >= 1x,
                // we simply don't do any scaling and keep the original sigma.
                const SIGMA_WHEN_SCALED: f32 = 2.0;
                let scale = SIGMA_WHEN_SCALED / sigma;

                let scaled;
                let (image, sigma) = if scale < 1.0 {
                    // The image needs to at least be 1x1. A triangle filter is
                    // probably fine, the blur will hide most scaling artifacts
                    // anyway.
                    scaled = image::imageops::resize(
                        &original_image,
                        ((scale * original_image.width() as f32) as u32).max(1),
                        ((scale * original_image.height() as f32) as u32).max(1),
                        FilterType::Triangle,
                    );
                    (
                        ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                            scaled.width(),
                            scaled.height(),
                            &*scaled,
                        )
                        .unwrap(),
                        SIGMA_WHEN_SCALED,
                    )
                } else {
                    (original_image, sigma)
                };

                let image_buffer = image::imageops::blur(&image, sigma);
                let (width, height) = image_buffer.dimensions();
                let pixmap = pixmap_from_premultiplied_rgba(
                    image_buffer.into_raw(),
                    width,
                    height,
                )
                .unwrap();
                *blurred_background_image = Some((current_key, Arc::new(pixmap)));
            }
        }
        _ => {
            *blurred_background_image = None;
        }
    }
}

fn calculate_bounds(layer: &[Entity<VelloPath, VelloImage, VelloLabel>]) -> [f32; 2] {
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for entity in layer.iter() {
        match entity {
            Entity::FillPath(path, _, transform) => {
                if path.is_empty() {
                    continue;
                }
                let bounds = path.bounding_box();
                for y in [bounds.y0 as f32, bounds.y1 as f32] {
                    let transformed_y = transform.transform_y(y);
                    min_y = min_y.min(transformed_y);
                    max_y = max_y.max(transformed_y);
                }
            }
            Entity::StrokePath(path, radius, _, transform) => {
                if path.is_empty() {
                    continue;
                }
                let radius = transform.scale_y * radius;
                let bounds = path.bounding_box();
                for y in [bounds.y0 as f32, bounds.y1 as f32] {
                    let transformed_y = transform.transform_y(y);
                    min_y = min_y.min(transformed_y - radius);
                    max_y = max_y.max(transformed_y + radius);
                }
            }
            Entity::Image(_, transform) => {
                for y in [0.0, 1.0] {
                    let transformed_y = transform.transform_y(y);
                    min_y = min_y.min(transformed_y);
                    max_y = max_y.max(transformed_y);
                }
            }
            Entity::Label(label, _, text_shadow, transform) => {
                let label = &*label.read().unwrap();

                if text_shadow.is_some() {
                    let transform = transform.pre_translate(SHADOW_OFFSET, SHADOW_OFFSET);

                    for glyph in label.glyphs() {
                        if glyph.path.is_empty() {
                            continue;
                        }
                        let transform = transform
                            .pre_translate(glyph.x, glyph.y)
                            .pre_scale(glyph.scale, glyph.scale);

                        let bounds = glyph.path.bounding_box();
                        for y in [bounds.y0 as f32, bounds.y1 as f32] {
                            let transformed_y = transform.transform_y(y);
                            min_y = min_y.min(transformed_y);
                            max_y = max_y.max(transformed_y);
                        }
                    }
                }

                for glyph in label.glyphs() {
                    if glyph.path.is_empty() {
                        continue;
                    }
                    let transform = transform
                        .pre_translate(glyph.x, glyph.y)
                        .pre_scale(glyph.scale, glyph.scale);

                    let bounds = glyph.path.bounding_box();
                    for y in [bounds.y0 as f32, bounds.y1 as f32] {
                        let transformed_y = transform.transform_y(y);
                        min_y = min_y.min(transformed_y);
                        max_y = max_y.max(transformed_y);
                    }
                }
            }
        }
    }
    [min_y, max_y]
}

fn row_range(min_y: f32, max_y: f32, height: u32) -> Option<(u32, u32)> {
    if min_y > max_y {
        return None;
    }
    let min_row = (min_y - 1.0).floor() as i32;
    let max_row = (max_y + 2.0).ceil() as i32;
    let min_row = min_row.max(0) as u32;
    let max_row = max_row.min(height as i32).max(min_row as i32) as u32;
    if min_row >= max_row {
        None
    } else {
        Some((min_row, max_row))
    }
}

fn copy_rows(src: &[u8], dst: &mut [u8], width: usize, min_row: u32, max_row: u32) {
    let row_len = width * 4;
    for y in min_row..max_row {
        let start = y as usize * row_len;
        let end = start + row_len;
        dst[start..end].copy_from_slice(&src[start..end]);
    }
}


#[cfg(feature = "image")]
fn pixmap_from_premultiplied_rgba(raw: Vec<u8>, width: u32, height: u32) -> Option<Pixmap> {
    let width_u16 = u16::try_from(width).ok()?;
    let height_u16 = u16::try_from(height).ok()?;
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for chunk in raw.chunks_exact(4) {
        pixels.push(PremulRgba8 {
            r: chunk[0],
            g: chunk[1],
            b: chunk[2],
            a: chunk[3],
        });
    }
    Some(Pixmap::from_parts(pixels, width_u16, height_u16))
}
