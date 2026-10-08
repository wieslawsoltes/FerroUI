use super::DiagnosticTextRenderer;
use crate::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use crate::media::{BoxShadows, Brushes};
use crate::platform::{self, IDrawingContextImpl, IGeometryImpl, IPlatformRenderInterface, IStreamGeometryImpl};
use crate::{Matrix, Point, Rect, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

const HEADER_PADDING: f64 = 2.0;

/// Represents a simple time graph for diagnostics purpose, used to show layout and render times.
pub struct FrameTimeGraph {
    render_interface: Rc<dyn IPlatformRenderInterface>,
    border_brush: ImmutableSolidColorBrush,
    graph_pen: ImmutablePen,
    frame_values: RefCell<Box<[f64]>>,
    size: Size,
    header_size: Size,
    graph_size: Size,
    default_max_y: f64,
    title: String,
    text_renderer: Rc<DiagnosticTextRenderer>,

    start_frame_index: Cell<usize>,
    frame_count: Cell<usize>,
}

impl FrameTimeGraph {
    pub fn size(&self) -> Size {
        self.size
    }

    /// Creates a graph using the platform render interface registered in the
    /// service locator.
    pub fn new(
        max_frames: i32,
        size: Size,
        default_max_y: f64,
        title: &str,
        text_renderer: Rc<DiagnosticTextRenderer>,
    ) -> Self {
        Self::with_render_interface(
            platform::render_interface(),
            max_frames,
            size,
            default_max_y,
            title,
            text_renderer,
        )
    }

    /// Creates a graph that builds its geometry with `render_interface`
    /// instead of looking the render interface up in the service locator.
    pub fn with_render_interface(
        render_interface: Rc<dyn IPlatformRenderInterface>,
        max_frames: i32,
        size: Size,
        default_max_y: f64,
        title: &str,
        text_renderer: Rc<DiagnosticTextRenderer>,
    ) -> Self {
        debug_assert!(max_frames >= 1);
        debug_assert!(size.width > 0.0);
        debug_assert!(size.height > 0.0);

        let header_size = Size::new(size.width, text_renderer.get_max_height() + HEADER_PADDING * 2.0);
        Self {
            render_interface,
            border_brush: ImmutableSolidColorBrush::from_uint32(0x80808080),
            graph_pen: ImmutablePen::with_brush(Some(Brushes::blue()), 1.0),
            frame_values: RefCell::new(vec![0.0; max_frames.max(0) as usize].into_boxed_slice()),
            size,
            header_size,
            graph_size: Size::new(size.width, size.height - header_size.height),
            default_max_y,
            title: title.to_string(),
            text_renderer,
            start_frame_index: Cell::new(0),
            frame_count: Cell::new(0),
        }
    }

    pub fn add_frame_value(&self, value: f64) {
        let mut frame_values = self.frame_values.borrow_mut();
        if self.frame_count.get() < frame_values.len() {
            frame_values[self.start_frame_index.get() + self.frame_count.get()] = value;
            self.frame_count.set(self.frame_count.get() + 1);
        } else {
            // overwrite oldest value
            frame_values[self.start_frame_index.get()] = value;
            self.start_frame_index.set(self.start_frame_index.get() + 1);
            if self.start_frame_index.get() == frame_values.len() {
                self.start_frame_index.set(0);
            }
        }
    }

    pub fn reset(&self) {
        self.start_frame_index.set(0);
        self.frame_count.set(0);
    }

    /// Draws the graph at the origin of the current transform of `context`.
    pub fn render(&self, context: &mut dyn IDrawingContextImpl) {
        context.push_clip(Rect::from_size(self.size));

        let no_shadows = BoxShadows::default();
        context.draw_rectangle(Some(&self.border_brush), None, Rect::from_size(self.size).into(), &no_shadows);
        context.draw_rectangle(Some(&self.border_brush), None, Rect::from_size(self.header_size).into(), &no_shadows);

        let outer_transform = context.transform();
        context.set_transform(Matrix::create_translation(HEADER_PADDING, HEADER_PADDING) * outer_transform);
        self.text_renderer.draw_ascii_text(context, &self.title, &*Brushes::black());

        if self.frame_count.get() > 0 {
            let (min, avg, max) = self.get_y_values();

            self.draw_labelled_value(context, "Min", min, self.header_size.width * 0.19);
            self.draw_labelled_value(context, "Avg", avg, self.header_size.width * 0.46);
            self.draw_labelled_value(context, "Max", max, self.header_size.width * 0.73);

            let header_transform = context.transform();
            context.set_transform(Matrix::create_translation(0.0, self.header_size.height) * header_transform);
            let geometry = self.build_graph_geometry(max.max(self.default_max_y));
            let geometry: &dyn IGeometryImpl = &*geometry;
            context.draw_geometry(None, Some(&self.graph_pen), geometry);
            context.set_transform(header_transform);
        }

        context.set_transform(outer_transform);
        context.pop_clip();
    }

    fn draw_labelled_value(&self, context: &mut dyn IDrawingContextImpl, label: &str, value: f64, left: f64) {
        let old_transform = context.transform();
        context.set_transform(Matrix::create_translation(left + HEADER_PADDING, HEADER_PADDING) * old_transform);

        let brush = if value <= self.default_max_y { Brushes::black() } else { Brushes::red() };

        // The text is written into a 24 character buffer; when it does not
        // fit, nothing is written at all.
        let mut text = format!("{label}: {:>5}ms", format_f2(value));
        if text.encode_utf16().count() > 24 {
            text.clear();
        }
        self.text_renderer.draw_ascii_text(context, &text, &*brush);

        context.set_transform(old_transform);
    }

    fn build_graph_geometry(&self, max_y: f64) -> Arc<dyn IStreamGeometryImpl> {
        debug_assert!(self.frame_count.get() > 0);

        let graph_geometry = self.render_interface.create_stream_geometry();
        let mut geometry_context = graph_geometry.open();

        let frame_values = self.frame_values.borrow();
        let x_ratio = self.graph_size.width / frame_values.len() as f64;
        let y_ratio = self.graph_size.height / max_y;

        geometry_context
            .begin_figure(Point::new(0.0, self.graph_size.height - self.get_frame_value(&frame_values, 0) * y_ratio), false);

        for i in 1..self.frame_count.get() {
            let x = (i as f64 * x_ratio).round_ties_even();
            let y = self.graph_size.height - self.get_frame_value(&frame_values, i) * y_ratio;
            geometry_context.line_to(Point::new(x, y), true);
        }

        geometry_context.end_figure(false);
        geometry_context.dispose();
        graph_geometry
    }

    fn get_y_values(&self) -> (f64, f64, f64) {
        debug_assert!(self.frame_count.get() > 0);

        let frame_values = self.frame_values.borrow();
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut total = 0.0;

        for i in 0..self.frame_count.get() {
            let y = self.get_frame_value(&frame_values, i);

            total += y;

            if y < min {
                min = y;
            }

            if y > max {
                max = y;
            }
        }

        (min, total / self.frame_count.get() as f64, max)
    }

    #[inline]
    fn get_frame_value(&self, frame_values: &[f64], frame_offset: usize) -> f64 {
        frame_values[(self.start_frame_index.get() + frame_offset) % frame_values.len()]
    }
}

/// Formats `value` with two decimals the way the invariant culture `F2`
/// format does. Finite values come out the same as with the standard
/// formatter (correctly rounded, exact ties to even); only the non-finite
/// values are spelled differently: `NaN`, `Infinity` and `-Infinity`.
fn format_f2(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }

    format!("{value:.2}")
}

#[cfg(test)]
mod tests {
    use super::super::diagnostic_text_renderer::tests::create_renderer;
    use super::super::dirty_rects::test_mocks::{MockDrawingContext, MockRenderInterface};
    use super::*;

    fn create_graph(max_frames: i32) -> (FrameTimeGraph, Rc<MockRenderInterface>) {
        let platform = Rc::new(MockRenderInterface::default());
        // The mock glyphs are at most 14 high: the header is 18 high and
        // the graph area 360 x 46.
        let graph = FrameTimeGraph::with_render_interface(
            platform.clone(),
            max_frames,
            Size::new(360.0, 64.0),
            20.0,
            "Render",
            Rc::new(create_renderer()),
        );
        (graph, platform)
    }

    /// Splits the glyph log into runs of text, keyed by the y/x origin and brush of their first glyph.
    fn drawn_texts(ctx: &MockDrawingContext) -> Vec<(String, String)> {
        let mut texts: Vec<(String, String, f64)> = Vec::new();
        for line in ctx.log.iter().filter_map(|l| l.strip_prefix("glyph ")) {
            let parts: Vec<&str> = line.split(' ').collect();
            let c = char::from_u32(parts[0].parse::<u32>().unwrap()).unwrap();
            let x: f64 = parts[3].split(',').next().unwrap().parse().unwrap();
            match texts.last_mut() {
                // Mock glyphs are 6 wide: a glyph continues the run when it
                // starts where the previous one ended.
                Some((text, _, next_x)) if *next_x == x => {
                    text.push(c);
                    *next_x += 6.0;
                }
                _ => texts.push((c.to_string(), format!("{} at {}", parts[1], parts[3]), x + 6.0)),
            }
        }
        texts.into_iter().map(|(text, origin, _)| (text, origin)).collect()
    }

    #[test]
    fn format_f2_matches_invariant_fixed_point_formatting() {
        // Reference values produced by the C# formatter.
        assert_eq!("0.00", format_f2(0.0));
        assert_eq!("16.67", format_f2(1000.0 / 60.0));
        assert_eq!("0.12", format_f2(0.125));
        assert_eq!("0.38", format_f2(0.375));
        assert_eq!("2.62", format_f2(2.625));
        assert_eq!("-0.12", format_f2(-0.125));
        assert_eq!("1234.88", format_f2(1234.875));
        assert_eq!("0.99", format_f2(0.994));
        assert_eq!("0.99", format_f2(0.995));
        assert_eq!("2.50", format_f2(2.5));
        assert_eq!("0.01", format_f2(0.005));
        assert_eq!("-0.00", format_f2(-0.0));
        assert_eq!("-0.00", format_f2(-0.001));
        assert_eq!("100000000000000000000.00", format_f2(1.0e20));
        assert_eq!("NaN", format_f2(f64::NAN));
        assert_eq!("Infinity", format_f2(f64::INFINITY));
        assert_eq!("-Infinity", format_f2(f64::NEG_INFINITY));
    }

    #[test]
    fn size_is_the_requested_size() {
        let (graph, _) = create_graph(4);
        assert_eq!(Size::new(360.0, 64.0), graph.size());
    }

    #[test]
    fn values_fill_up_and_then_overwrite_the_oldest() {
        let (graph, _) = create_graph(3);
        graph.add_frame_value(1.0);
        graph.add_frame_value(2.0);
        assert_eq!((1.0, 1.5, 2.0), graph.get_y_values());

        graph.add_frame_value(3.0);
        graph.add_frame_value(4.0);
        // 1.0 was dropped; the window is 2, 3, 4 in that order.
        assert_eq!((2.0, 3.0, 4.0), graph.get_y_values());
        let values = graph.frame_values.borrow();
        assert_eq!(
            vec![2.0, 3.0, 4.0],
            (0..3).map(|i| graph.get_frame_value(&values, i)).collect::<Vec<_>>()
        );
        drop(values);

        graph.add_frame_value(5.0);
        graph.add_frame_value(6.0);
        // The start index wrapped around.
        assert_eq!(0, graph.start_frame_index.get());
        assert_eq!((4.0, 5.0, 6.0), graph.get_y_values());

        graph.reset();
        assert_eq!(0, graph.frame_count.get());
        graph.add_frame_value(9.0);
        assert_eq!((9.0, 9.0, 9.0), graph.get_y_values());
    }

    #[test]
    fn renders_only_the_frame_and_title_without_values() {
        let (graph, platform) = create_graph(4);
        let mut ctx =
            MockDrawingContext { transform: Matrix::create_translation(10.0, 100.0), ..Default::default() };
        graph.render(&mut ctx);

        assert_eq!("push_clip 0, 0, 360, 64", ctx.log[0]);
        assert_eq!("rect #80808080@1 0, 0, 360, 64", ctx.log[1]);
        assert_eq!("rect #80808080@1 0, 0, 360, 18", ctx.log[2]);
        assert_eq!("pop_clip", ctx.log.last().unwrap());
        assert_eq!(vec![("Render".to_string(), "#ff000000@1 at 12,102".to_string())], drawn_texts(&ctx));
        assert!(platform.streams.borrow().is_empty());
        assert_eq!(Matrix::create_translation(10.0, 100.0), ctx.transform);
    }

    #[test]
    fn renders_labels_and_the_graph_geometry() {
        let (graph, platform) = create_graph(4);
        graph.add_frame_value(10.0);
        graph.add_frame_value(40.0);
        graph.add_frame_value(25.125);
        let mut ctx = MockDrawingContext::default();
        graph.render(&mut ctx);

        assert_eq!(
            vec![
                ("Render".to_string(), "#ff000000@1 at 2,2".to_string()),
                // Label origins: header padding twice plus 19% / 46% / 73% of the width.
                ("Min: 10.00ms".to_string(), "#ff000000@1 at 72.4,4".to_string()),
                // The average is above the default maximum of 20: red.
                ("Avg: 25.04ms".to_string(), "#ffff0000@1 at 169.6,4".to_string()),
                ("Max: 40.00ms".to_string(), "#ffff0000@1 at 266.8,4".to_string()),
            ],
            drawn_texts(&ctx)
        );

        // The geometry is drawn below the header, stroked blue.
        assert!(ctx.log.contains(&"geometry none pen #ff0000ff@1 at 2,20".to_string()), "{:?}", ctx.log);
        assert_eq!("pop_clip", ctx.log.last().unwrap());
        assert_eq!(Matrix::IDENTITY, ctx.transform);

        // 4 frames over 360: 90 per frame. Max y is 40 over a height of 46.
        let streams = platform.streams.borrow();
        assert_eq!(1, streams.len());
        assert_eq!(
            vec![
                format!("begin {} false", Point::new(0.0, 46.0 - 10.0 * (46.0 / 40.0))),
                format!("line {} true", Point::new(90.0, 46.0 - 40.0 * (46.0 / 40.0))),
                format!("line {} true", Point::new(180.0, 46.0 - 25.125 * (46.0 / 40.0))),
                "end false".to_string(),
                "dispose".to_string(),
            ],
            *streams[0].lock().unwrap()
        );
    }

    #[test]
    fn graph_is_scaled_to_the_default_maximum_when_values_are_small() {
        let (graph, platform) = create_graph(8);
        graph.add_frame_value(5.0);
        graph.add_frame_value(10.0);
        let mut ctx = MockDrawingContext::default();
        graph.render(&mut ctx);
        let streams = platform.streams.borrow();
        // x = round(1 * 45) = 45; y scale is 46 / 20.
        assert_eq!(format!("begin {} false", Point::new(0.0, 46.0 - 5.0 * 2.3)), streams[0].lock().unwrap()[0]);
        assert_eq!(format!("line {} true", Point::new(45.0, 46.0 - 10.0 * 2.3)), streams[0].lock().unwrap()[1]);
        let texts = drawn_texts(&ctx);
        // The value is right-aligned in 5 characters.
        assert_eq!("Min:  5.00ms", texts[1].0);
    }

    #[test]
    fn x_positions_round_half_to_even() {
        let platform = Rc::new(MockRenderInterface::default());
        // 5 frames over 2.5: ratio 0.5, so x is 0.5, 1, 1.5, 2 -> 0, 1, 2, 2.
        let graph = FrameTimeGraph::with_render_interface(
            platform.clone(),
            5,
            Size::new(2.5, 64.0),
            20.0,
            "",
            Rc::new(create_renderer()),
        );
        for _ in 0..5 {
            graph.add_frame_value(0.0);
        }
        graph.render(&mut MockDrawingContext::default());
        let streams = platform.streams.borrow();
        let xs: Vec<String> =
            streams[0].lock().unwrap().iter().filter(|l| l.starts_with("line")).map(|l| l.to_string()).collect();
        assert_eq!(
            [0.0, 1.0, 2.0, 2.0].iter().map(|x| format!("line {} true", Point::new(*x, 46.0))).collect::<Vec<_>>(),
            xs
        );
    }

    #[test]
    fn labels_that_do_not_fit_the_buffer_are_not_drawn() {
        let (graph, _) = create_graph(4);
        graph.add_frame_value(1.0e20);
        let mut ctx = MockDrawingContext::default();
        graph.render(&mut ctx);
        // "Min: 100000000000000000000.00ms" is longer than 24 characters.
        assert_eq!(1, drawn_texts(&ctx).len());
    }
}
