mod mesh;

use lyon_tessellation::{
    path::path, BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions,
    StrokeTessellator, StrokeVertex,
};

pub use self::mesh::{Mesh, Vertex};

use super::{FillShader, Rgba, Transform};

/// The rendering backend for the Renderer is abstracted out into the Backend
/// trait such that the rendering isn't tied to a specific rendering framework.
pub trait Backend {
    /// The type the backend uses for meshes.
    type Mesh;
    /// The type the backend uses for textures.
    type Texture;

    /// Instructs the backend to create a mesh. The mesh consists out of a
    /// vertex buffer and an index buffer that describes pairs of three indices
    /// of the vertex buffer that form a triangle each.
    fn create_mesh(&mut self, mesh: &Mesh) -> Self::Mesh;

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
    fn render_mesh(
        &mut self,
        mesh: &Self::Mesh,
        transform: Transform,
        colors: [Rgba; 4],
        texture: Option<&Self::Texture>,
    );

    /// Instructs the backend to free a mesh as it is not needed anymore.
    fn free_mesh(&mut self, mesh: Self::Mesh);

    /// Instructs the backend to create a texture out of the texture data
    /// provided. The texture's resolution is provided as well. The data is an
    /// array of chunks of RGBA8 encoded pixels (red, green, blue, alpha with
    /// each channel being an u8).
    fn create_texture(&mut self, width: u32, height: u32, data: &[u8]) -> Self::Texture;

    /// Instructs the backend to free a texture as it is not needed anymore.
    fn free_texture(&mut self, texture: Self::Texture);
}

pub struct FillBuilder(path::Builder);

impl<B: Backend> super::PathBuilder<B> for FillBuilder {
    type Path = <B as Backend>::Mesh;

    fn move_to(&mut self, x: f32, y: f32) {
        self.0.begin((x, y).into());
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((x, y).into());
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quadratic_bezier_to((x1, y1).into(), (x, y).into());
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0
            .cubic_bezier_to((x1, y1).into(), (x2, y2).into(), (x, y).into());
    }

    fn close(&mut self) {
        self.0.close();
    }

    fn finish(self, backend: &mut B) -> Self::Path {
        let path = self.0.build();
        let mut tessellator = FillTessellator::new();
        let mut mesh = Mesh::new();
        let _ = tessellator.tessellate_path(
            &path,
            &FillOptions::tolerance(0.005),
            &mut BuffersBuilder::new(&mut mesh.buffers, |v: FillVertex<'_>| Vertex {
                x: v.position().x,
                y: v.position().y,
                u: 0.0,
                v: 0.0,
            }),
        );
        Backend::create_mesh(backend, &mesh)
    }
}

pub struct StrokeBuilder(path::Builder, f32);

impl<B: Backend> super::PathBuilder<B> for StrokeBuilder {
    type Path = <B as Backend>::Mesh;

    fn move_to(&mut self, x: f32, y: f32) {
        self.0.begin((x, y).into());
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((x, y).into());
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quadratic_bezier_to((x1, y1).into(), (x, y).into());
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0
            .cubic_bezier_to((x1, y1).into(), (x2, y2).into(), (x, y).into());
    }

    fn close(&mut self) {
        self.0.close();
    }

    fn finish(self, backend: &mut B) -> Self::Path {
        let path = self.0.build();
        let mut tessellator = StrokeTessellator::new();
        let mut mesh = Mesh::new();
        let _ = tessellator.tessellate_path(
            &path,
            &StrokeOptions::default().with_line_width(self.1),
            &mut BuffersBuilder::new(&mut mesh.buffers, |v: StrokeVertex<'_, '_>| Vertex {
                x: v.position().x,
                y: v.position().y,
                u: 0.0,
                v: 0.0,
            }),
        );
        Backend::create_mesh(backend, &mesh)
    }
}

impl<T: Backend> super::Backend for T {
    type FillBuilder = FillBuilder;
    type StrokeBuilder = StrokeBuilder;
    type Path = <Self as Backend>::Mesh;
    type Image = <Self as Backend>::Texture;

    fn fill_builder(&mut self) -> Self::FillBuilder {
        FillBuilder(path::Builder::new())
    }

    fn stroke_builder(&mut self, stroke_width: f32) -> Self::StrokeBuilder {
        StrokeBuilder(path::Builder::new(), stroke_width)
    }

    fn render_fill_path(&mut self, path: &Self::Path, shader: FillShader, transform: Transform) {
        let colors = match shader {
            FillShader::SolidColor(c) => [c; 4],
            FillShader::VerticalGradient(t, b) => [t, t, b, b],
            FillShader::HorizontalGradient(l, r) => [l, r, r, l],
        };
        Backend::render_mesh(self, path, transform, colors, None)
    }

    fn render_stroke_path(
        &mut self,
        path: &Self::Path,
        _stroke_width: f32,
        color: Rgba,
        transform: Transform,
    ) {
        Backend::render_mesh(self, path, transform, [color; 4], None)
    }

    fn render_image(&mut self, image: &Self::Image, rectangle: &Self::Path, transform: Transform) {
        Backend::render_mesh(self, rectangle, transform, [[1.0; 4]; 4], Some(image))
    }

    fn free_path(&mut self, path: Self::Path) {
        Backend::free_mesh(self, path)
    }

    fn create_image(&mut self, width: u32, height: u32, data: &[u8]) -> Self::Image {
        Backend::create_texture(self, width, height, data)
    }

    fn free_image(&mut self, image: Self::Image) {
        Backend::free_texture(self, image)
    }
}
