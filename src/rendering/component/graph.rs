use crate::{
    component::graph::State,
    layout::LayoutState,
    rendering::{
        mesh::{basic_builder, fill_builder, stroke_builder},
        Backend, PathBuilder, RenderContext,
    },
    settings::Gradient,
};
// use lyon::tessellation::{
//     basic_shapes::{fill_circle, fill_polyline, stroke_polyline},
//     FillOptions, FillTessellator, StrokeOptions,
// };
use std::iter;

pub(in crate::rendering) fn render(
    context: &mut RenderContext<'_, impl Backend>,
    [width, height]: [f32; 2],
    component: &State,
    _layout_state: &LayoutState,
) {
    let old_transform = context.transform;
    context.scale(height);
    let width = width / height;

    const GRID_LINE_WIDTH: f32 = 0.015;
    const LINE_WIDTH: f32 = 0.025;
    const CIRCLE_RADIUS: f32 = 0.035;

    context.render_rectangle(
        [0.0, 0.0],
        [width, component.middle],
        &Gradient::Plain(component.top_background_color),
    );
    context.render_rectangle(
        [0.0, component.middle],
        [width, 1.0],
        &Gradient::Plain(component.bottom_background_color),
    );

    for &y in &component.horizontal_grid_lines {
        context.render_rectangle(
            [0.0, y - GRID_LINE_WIDTH],
            [width, y + GRID_LINE_WIDTH],
            &Gradient::Plain(component.grid_lines_color),
        );
    }

    for &x in &component.vertical_grid_lines {
        context.render_rectangle(
            [width * x - GRID_LINE_WIDTH, 0.0],
            [width * x + GRID_LINE_WIDTH, 1.0],
            &Gradient::Plain(component.grid_lines_color),
        );
    }

    let len = if component.is_live_delta_active {
        let p1 = &component.points[component.points.len() - 2];
        let p2 = &component.points[component.points.len() - 1];

        let mut builder = context.backend.build_path();
        builder.move_to(width * p1.x, component.middle);
        builder.line_to(width * p1.x, p1.y);
        builder.line_to(width * p2.x, p2.y);
        builder.line_to(width * p2.x, component.middle);
        builder.close();
        let partial_fill_path = builder.finish();
        context.render_path(&partial_fill_path, component.partial_fill_color);
        context.free_path(partial_fill_path);

        component.points.len() - 1
    } else {
        component.points.len()
    };

    let mut builder = context.backend.build_path();
    builder.move_to(0.0, component.middle);
    for p in &component.points[..len] {
        builder.line_to(width * p.x, p.y);
    }
    builder.line_to(width * component.points[len - 1].x, component.middle);
    builder.close();
    let fill_path = builder.finish();
    context.render_path(&fill_path, component.complete_fill_color);
    context.free_path(fill_path);

    for points in component.points.windows(2) {
        let mut builder = context.backend.build_path();
        builder.move_to(width * points[0].x, points[0].y);
        builder.line_to(width * points[1].x, points[1].y);

        let color = if points[1].is_best_segment {
            component.best_segment_color
        } else {
            component.graph_lines_color
        };

        let line_path = builder.finish();
        context.render_stroke_path(&line_path, color, LINE_WIDTH);
        context.free_path(line_path);
    }

    for (i, point) in component.points.iter().enumerate().skip(1) {
        if i != component.points.len() - 1 || !component.is_live_delta_active {
            let color = if point.is_best_segment {
                component.best_segment_color
            } else {
                component.graph_lines_color
            };

            let circle_path = context
                .backend
                .build_circle(width * point.x, point.y, CIRCLE_RADIUS);
            context.render_path(&circle_path, color);
            context.free_path(circle_path);
        }
    }

    context.transform = old_transform;
}
