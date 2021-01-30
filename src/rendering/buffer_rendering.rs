use std::{io::Write, mem};

use bincode::{DefaultOptions, Options};
use serde::Serialize;

use crate::layout::LayoutState;

use super::{Backend, FillShader, PathBuilder, Renderer, Rgba, Transform};

type Id = u32;

#[derive(Serialize)]
enum Command<'a> {
    BuildFill(Id, &'a [u8]),
    BuildStroke(Id, f32, &'a [u8]),
    RenderFillPath(Id, FillShader, [f32; 6]),
    RenderStrokePath(Id, f32, Rgba, [f32; 6]),
    RenderImage(Id, Id, [f32; 6]),
    FreePath(Id),
    CreateImage(Id, u32, u32, &'a [u8]),
    FreeImage(Id),
}

#[derive(Serialize)]
enum PathCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CurveTo(f32, f32, f32, f32, f32, f32),
    Close,
}

struct BufferPathBuilder(Vec<u8>, Option<f32>);

impl BufferPathBuilder {
    fn write(&mut self, command: &PathCommand) {
        let _ = DefaultOptions::default()
            .with_fixint_encoding()
            .serialize_into(&mut self.0, command);
    }
}

impl<W: Write> PathBuilder<BufferBackend<'_, W>> for BufferPathBuilder {
    type Path = Id;

    fn move_to(&mut self, x: f32, y: f32) {
        self.write(&PathCommand::MoveTo(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.write(&PathCommand::LineTo(x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.write(&PathCommand::QuadTo(x1, y1, x, y));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.write(&PathCommand::CurveTo(x1, y1, x2, y2, x, y));
    }

    fn close(&mut self) {
        self.write(&PathCommand::Close);
    }

    fn finish(self, backend: &mut BufferBackend<'_, W>) -> Self::Path {
        let id = backend.state.latest_path_idx;
        backend.state.latest_path_idx += 1;

        let command = match self.1 {
            Some(stroke_width) => Command::BuildStroke(id, stroke_width, &self.0),
            None => Command::BuildFill(id, &self.0),
        };
        backend.write(&command);
        backend.state.intermediate_buffer = self.0;

        id
    }
}

struct BufferBackend<'a, W: Write> {
    state: &'a mut State,
    buffer: W,
}

impl<W: Write> BufferBackend<'_, W> {
    fn write(&mut self, command: &Command<'_>) {
        let _ = DefaultOptions::default()
            .with_fixint_encoding()
            .serialize_into(&mut self.buffer, command);
    }
}

impl<W: Write> Backend for BufferBackend<'_, W> {
    type FillBuilder = BufferPathBuilder;
    type StrokeBuilder = BufferPathBuilder;
    type Path = Id;
    type Image = Id;

    fn fill_builder(&mut self) -> Self::FillBuilder {
        let mut buffer = mem::take(&mut self.state.intermediate_buffer);
        buffer.clear();
        BufferPathBuilder(buffer, None)
    }

    fn stroke_builder(&mut self, stroke_width: f32) -> Self::StrokeBuilder {
        let mut buffer = mem::take(&mut self.state.intermediate_buffer);
        buffer.clear();
        BufferPathBuilder(buffer, Some(stroke_width))
    }

    fn render_fill_path(&mut self, path: &Self::Path, shader: FillShader, transform: Transform) {
        self.write(&Command::RenderFillPath(
            *path,
            shader,
            transform.to_array(),
        ));
    }

    fn render_stroke_path(
        &mut self,
        path: &Self::Path,
        stroke_width: f32,
        color: Rgba,
        transform: Transform,
    ) {
        self.write(&Command::RenderStrokePath(
            *path,
            stroke_width,
            color,
            transform.to_array(),
        ));
    }

    fn render_image(&mut self, image: &Self::Image, rectangle: &Self::Path, transform: Transform) {
        self.write(&Command::RenderImage(
            *image,
            *rectangle,
            transform.to_array(),
        ));
    }

    fn free_path(&mut self, path: Self::Path) {
        self.write(&Command::FreePath(path));
    }

    fn create_image(&mut self, width: u32, height: u32, data: &[u8]) -> Self::Image {
        let id = self.state.latest_image_idx;
        self.state.latest_image_idx += 1;

        self.write(&Command::CreateImage(id, width, height, data));

        id
    }

    fn free_image(&mut self, image: Self::Image) {
        self.write(&Command::FreeImage(image));
    }
}

struct State {
    latest_path_idx: Id,
    latest_image_idx: Id,
    intermediate_buffer: Vec<u8>,
}

pub struct BufferRenderer {
    state: State,
    renderer: Renderer<Id, Id>,
}

impl BufferRenderer {
    pub fn new() -> Self {
        Self {
            state: State {
                latest_path_idx: 0,
                latest_image_idx: 0,
                intermediate_buffer: Vec::new(),
            },
            renderer: Renderer::new(),
        }
    }

    pub fn render(
        &mut self,
        state: &LayoutState,
        buffer: impl Write,
        [width, height]: [f32; 2],
    ) -> Option<(f32, f32)> {
        self.renderer.render(
            &mut BufferBackend {
                state: &mut self.state,
                buffer,
            },
            (width as _, height as _),
            &state,
        )
    }
}
