//! The commands of a frame drawn through the drawing context, which every render backend
//! implements on its GPU. Geometry and brushes are kept between frames: a path that does not
//! change is built once, and a frame of a moving transform only pushes a matrix.

use crate::evaluate::{FrameCommand, FrameCommands, GradientKind, MaskMode, Paint, ShapeDraw, StrokeStyle};
use crate::model::MatteMode;
use crate::path::{PathCommand, PathData};
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::media::immutable::{
    ImmutableDashStyle, ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen,
    ImmutableRadialGradientBrush, ImmutableSolidColorBrush,
};
use ferroui_base::media::{
    Color, DrawingContext, FillRule, Geometry, GradientSpreadMethod, IBrush, IEffect, IImmutableBrush, IPen,
    ImmutableLuminanceToAlphaEffect, PenLineCap, PenLineJoin, PushedState, StreamGeometry,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{Matrix, Point, Rect, Ref, RelativePoint, RelativeScalar, RelativeUnit};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// The geometries kept. Past it, those no recent frame used go.
const MOST_GEOMETRIES: usize = 1024;
/// How many frames a geometry stays without being used, once there are too many.
const KEPT_FRAMES: u64 = 120;
/// How far the outside of an inverted mask reaches.
const OUTSIDE: f64 = 1_000_000.0;

/// What a push of a frame leaves to its `Pop`.
enum Scope {
    State(PushedState),
    /// A group whose content is cut, when it ends, by what `coverage` fills.
    Cut { group: PushedState, blend: BitmapBlendingMode, coverage: Vec<(Ref<Geometry>, f64)> },
}

struct KeptGeometry {
    path: PathData,
    even_odd: bool,
    geometry: Ref<Geometry>,
    used: u64,
}

/// Draws the frames of an animated visual, keeping what they share.
#[derive(Default)]
pub struct VisualRenderer {
    geometries: HashMap<u64, KeptGeometry>,
    solid_brushes: HashMap<u32, Rc<ImmutableSolidColorBrush>>,
    frame: u64,
    built: u64,
}

fn key_of(path: &PathData, even_odd: bool) -> u64 {
    let mut hasher = DefaultHasher::new();
    even_odd.hash(&mut hasher);
    let mut point = |p: Point| {
        p.x.to_bits().hash(&mut hasher);
        p.y.to_bits().hash(&mut hasher);
    };
    for command in &path.commands {
        match *command {
            PathCommand::MoveTo(a) => point(a),
            PathCommand::LineTo(a) => point(a),
            PathCommand::CubicTo(a, b, c) => {
                point(a);
                point(b);
                point(c);
            }
            PathCommand::Close => point(Point::new(f64::NAN, 0.0)),
        }
    }
    hasher.finish()
}

fn build_geometry(path: &PathData, even_odd: bool, filled: bool) -> Ref<Geometry> {
    let geometry = StreamGeometry::new();
    {
        let mut context = geometry.open();
        context.set_fill_rule(if even_odd { FillRule::EvenOdd } else { FillRule::NonZero });
        let mut open = false;
        for command in &path.commands {
            match *command {
                PathCommand::MoveTo(point) => {
                    if open {
                        context.end_figure(false);
                    }
                    context.begin_figure(point, filled);
                    open = true;
                }
                PathCommand::LineTo(point) if open => context.line_to(point, true),
                PathCommand::CubicTo(a, b, c) if open => context.cubic_bezier_to(a, b, c, true),
                PathCommand::Close if open => {
                    context.end_figure(true);
                    open = false;
                }
                _ => {}
            }
        }
        if open {
            context.end_figure(false);
        }
    }
    geometry.upcast()
}

fn with_opacity(color: Color, opacity: f64) -> Color {
    let alpha = (f64::from(color.a) * opacity.clamp(0.0, 1.0)).round() as u8;
    Color::from_argb(alpha, color.r, color.g, color.b)
}

fn absolute(point: Point) -> RelativePoint {
    RelativePoint::new(point.x, point.y, RelativeUnit::Absolute)
}

fn pen_of(style: &StrokeStyle, brush: Rc<dyn IImmutableBrush>) -> Rc<dyn IPen> {
    let cap = match style.cap {
        2 => PenLineCap::Round,
        3 => PenLineCap::Square,
        _ => PenLineCap::Flat,
    };
    let join = match style.join {
        1 => PenLineJoin::Miter,
        3 => PenLineJoin::Bevel,
        _ => PenLineJoin::Round,
    };
    // The lengths of a dash style are in widths of the pen.
    let dash = (!style.dashes.is_empty() && style.width > 0.0).then(|| {
        let dashes: Vec<f64> = style.dashes.iter().map(|length| length / style.width).collect();
        Rc::new(ImmutableDashStyle::new(Some(&dashes), style.dash_offset / style.width))
    });
    Rc::new(ImmutablePen::new(Some(brush), style.width, dash, cap, join, style.miter.max(0.0)))
}

impl VisualRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many geometries were built so far: a frame that changes no path builds none.
    pub fn geometries_built(&self) -> u64 {
        self.built
    }

    /// Draws `commands`, in the space of a composition of `width` by `height`.
    pub fn render(&mut self, commands: &FrameCommands, width: f64, height: f64, context: &mut DrawingContext<'_>) {
        self.frame += 1;
        let bounds = Rect::new(0.0, 0.0, width, height);
        self.render_commands(&commands.commands, bounds, context);
        if self.geometries.len() > MOST_GEOMETRIES {
            let frame = self.frame;
            self.geometries.retain(|_, kept| frame - kept.used <= KEPT_FRAMES);
        }
    }

    fn geometry(&mut self, path: &PathData, even_odd: bool) -> Ref<Geometry> {
        let frame = self.frame;
        let key = key_of(path, even_odd);
        if let Some(kept) = self.geometries.get_mut(&key) {
            if kept.even_odd == even_odd && kept.path == *path {
                kept.used = frame;
                return kept.geometry.clone();
            }
        }
        let geometry = build_geometry(path, even_odd, true);
        self.built += 1;
        self.geometries.insert(key, KeptGeometry { path: path.clone(), even_odd, geometry: geometry.clone(), used: frame });
        geometry
    }

    fn solid(&mut self, color: Color) -> Rc<ImmutableSolidColorBrush> {
        let key = u32::from_be_bytes([color.a, color.r, color.g, color.b]);
        self.solid_brushes.entry(key).or_insert_with(|| Rc::new(ImmutableSolidColorBrush::new(color))).clone()
    }

    fn paint(&mut self, paint: &Paint, opacity: f64) -> Rc<dyn IImmutableBrush> {
        match paint {
            Paint::Solid(color) => self.solid(with_opacity(*color, opacity)),
            Paint::Gradient { kind, start, end, stops } => {
                let stops: Vec<ImmutableGradientStop> =
                    stops.iter().map(|(offset, color)| ImmutableGradientStop::new(*offset, *color)).collect();
                let spread = GradientSpreadMethod::Pad;
                match kind {
                    GradientKind::Linear => Rc::new(ImmutableLinearGradientBrush::new(
                        &stops,
                        opacity,
                        None,
                        None,
                        spread,
                        Some(absolute(*start)),
                        Some(absolute(*end)),
                        None,
                    )),
                    GradientKind::Radial => {
                        let radius = RelativeScalar::new((end.x - start.x).hypot(end.y - start.y), RelativeUnit::Absolute);
                        Rc::new(ImmutableRadialGradientBrush::new(
                            &stops,
                            opacity,
                            None,
                            None,
                            spread,
                            Some(absolute(*start)),
                            Some(absolute(*start)),
                            Some(radius),
                            Some(radius),
                            None,
                        ))
                    }
                }
            }
        }
    }

    fn draw(&mut self, draw: &ShapeDraw, context: &mut DrawingContext<'_>) {
        if draw.opacity <= 0.0 {
            return;
        }
        let geometry = self.geometry(&draw.path, draw.even_odd);
        let brush = self.paint(&draw.paint, draw.opacity);
        match &draw.stroke {
            Some(style) if style.width > 0.0 => {
                let pen = pen_of(style, brush);
                context.draw_geometry(None, Some(&pen), &geometry);
            }
            Some(_) => {}
            None => {
                let brush: Rc<dyn IBrush> = brush;
                context.draw_geometry(Some(&brush), None, &geometry);
            }
        }
    }

    /// The area of a mask: its path, or for an inverted mask everything but it.
    fn mask_area(&mut self, path: &PathData, inverted: bool) -> Ref<Geometry> {
        if !inverted {
            return self.geometry(path, false);
        }
        let mut outside = PathData::rectangle(Point::new(0.0, 0.0), OUTSIDE * 2.0, OUTSIDE * 2.0, 0.0);
        outside.append(path);
        self.geometry(&outside, true)
    }

    /// The clip of a mask of full opacity: its area, or what a subtracted mask leaves.
    fn mask_geometry(&mut self, path: &PathData, mode: MaskMode, inverted: bool) -> Ref<Geometry> {
        self.mask_area(path, (mode == MaskMode::Subtract) != inverted)
    }

    /// Opens a group that `coverage` cuts when it ends. A backend without compositing layers has
    /// no such group: it gets `None`, and what the group would cut stays.
    fn begin_cut(
        blend: BitmapBlendingMode,
        coverage: Vec<(Ref<Geometry>, f64)>,
        bounds: Rect,
        context: &mut DrawingContext<'_>,
    ) -> Option<Scope> {
        let group = context.try_push_compositing_layer(BitmapBlendingMode::SourceOver, 1.0, bounds)?;
        Some(Scope::Cut { group, blend, coverage })
    }

    fn end_scope(&mut self, scope: Scope, bounds: Rect, context: &mut DrawingContext<'_>) {
        match scope {
            Scope::State(state) => context.pop(state),
            Scope::Cut { group, blend, coverage } => {
                if let Some(cut) = context.try_push_compositing_layer(blend, 1.0, bounds) {
                    for (geometry, opacity) in &coverage {
                        let brush: Rc<dyn IBrush> = self.solid(with_opacity(Color::from_rgb(255, 255, 255), *opacity));
                        context.draw_geometry(Some(&brush), None, geometry);
                    }
                    context.pop(cut);
                }
                context.pop(group);
            }
        }
    }

    fn render_commands(&mut self, commands: &[FrameCommand], bounds: Rect, context: &mut DrawingContext<'_>) {
        // Each scope keeps the area of the composition in the space outside it.
        let mut scopes: Vec<(Scope, Rect)> = Vec::new();
        let mut bounds = bounds;
        let mut index = 0;
        while index < commands.len() {
            match &commands[index] {
                FrameCommand::PushTransform(matrix) => {
                    scopes.push((Scope::State(context.push_transform(*matrix)), bounds));
                    bounds = area_inside(bounds, *matrix);
                }
                FrameCommand::PushOpacity(opacity) => scopes.push((Scope::State(context.push_opacity(*opacity)), bounds)),
                FrameCommand::PushMask { path, mode, inverted, opacity } => {
                    let cut = (*opacity < 1.0).then(|| {
                        let area = self.mask_area(path, *inverted);
                        let blend = if *mode == MaskMode::Subtract {
                            BitmapBlendingMode::DestinationOut
                        } else {
                            BitmapBlendingMode::DestinationIn
                        };
                        Self::begin_cut(blend, vec![(area, *opacity)], bounds, context)
                    });
                    let scope = match cut.flatten() {
                        Some(scope) => scope,
                        None => {
                            let geometry = self.mask_geometry(path, *mode, *inverted);
                            Scope::State(context.push_geometry_clip(&geometry))
                        }
                    };
                    scopes.push((scope, bounds));
                }
                FrameCommand::PushCoverage(paths) => {
                    let coverage: Vec<_> =
                        paths.iter().map(|(path, opacity)| (self.geometry(path, false), *opacity)).collect();
                    let scope = match Self::begin_cut(BitmapBlendingMode::DestinationIn, coverage, bounds, context) {
                        Some(scope) => scope,
                        None => {
                            let mut union = PathData::default();
                            for (path, _) in paths {
                                union.append(path);
                            }
                            let geometry = self.geometry(&union, false);
                            Scope::State(context.push_geometry_clip(&geometry))
                        }
                    };
                    scopes.push((scope, bounds));
                }
                FrameCommand::Draw(draw) => self.draw(draw, context),
                FrameCommand::Pop => {
                    if let Some((scope, outer)) = scopes.pop() {
                        self.end_scope(scope, outer, context);
                        bounds = outer;
                    }
                }
                FrameCommand::BeginMatte(mode) => {
                    let (source_end, end) = matte_range(commands, index);
                    let source = &commands[index + 1..source_end];
                    let content = &commands[source_end + 1..end];
                    self.render_matted(*mode, source, content, bounds, context);
                    index = end;
                }
                FrameCommand::EndMatteSource => {}
            }
            index += 1;
        }
        while let Some((scope, outer)) = scopes.pop() {
            self.end_scope(scope, outer, context);
        }
    }

    /// The content in a group, then its matte composed into the group: destination-in keeps what
    /// the matte covers, destination-out what it does not. A luma matte passes through the
    /// luminance mask first. These are layers of the backend, drawn on its GPU; nothing is read
    /// back. A backend without compositing layers cannot cut the content, so it does not draw it.
    fn render_matted(
        &mut self,
        mode: MatteMode,
        source: &[FrameCommand],
        content: &[FrameCommand],
        bounds: Rect,
        context: &mut DrawingContext<'_>,
    ) {
        let Some(group) = context.try_push_compositing_layer(BitmapBlendingMode::SourceOver, 1.0, bounds) else {
            return;
        };
        self.render_commands(content, bounds, context);
        let blend = match mode {
            MatteMode::Alpha | MatteMode::Luma => BitmapBlendingMode::DestinationIn,
            MatteMode::AlphaInverted | MatteMode::LumaInverted => BitmapBlendingMode::DestinationOut,
        };
        if let Some(matte) = context.try_push_compositing_layer(blend, 1.0, bounds) {
            if matches!(mode, MatteMode::Luma | MatteMode::LumaInverted) {
                let effect: Rc<dyn IEffect> = Rc::new(ImmutableLuminanceToAlphaEffect);
                let luminance = context.push_effect(&effect, bounds);
                self.render_commands(source, bounds, context);
                context.pop(luminance);
            } else {
                self.render_commands(source, bounds, context);
            }
            context.pop(matte);
        }
        context.pop(group);
    }
}

/// `bounds` seen from inside `matrix`. A matrix that cannot be inverted draws nothing.
fn area_inside(bounds: Rect, matrix: Matrix) -> Rect {
    matrix.try_invert().map_or(Rect::default(), |inverse| bounds.transform_to_aabb(inverse))
}

/// The index of the `EndMatteSource` and of the `Pop` of the matte that begins at `begin`.
fn matte_range(commands: &[FrameCommand], begin: usize) -> (usize, usize) {
    let mut depth = 0usize;
    let mut source_end = commands.len();
    for (index, command) in commands.iter().enumerate().skip(begin + 1) {
        match command {
            FrameCommand::PushTransform(_)
            | FrameCommand::PushOpacity(_)
            | FrameCommand::PushMask { .. }
            | FrameCommand::PushCoverage(_)
            | FrameCommand::BeginMatte(_) => depth += 1,
            FrameCommand::EndMatteSource if depth == 0 => source_end = index,
            FrameCommand::Pop if depth == 0 => return (source_end.min(index), index),
            FrameCommand::Pop => depth -= 1,
            _ => {}
        }
    }
    (source_end.min(commands.len().saturating_sub(1)), commands.len().saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AnimatedVisual;
    use ferroui_base::media::PlatformDrawingContext;
    use ferroui_base::platform::{IPlatformRenderInterface, IRenderTargetBitmapImpl, PixelFormat};
    use ferroui_base::{FerroLocator, PixelSize, Vector};
    use ferroui_vello::{PlatformRenderInterface, RenderTargetBitmapImpl, VelloRenderingMode};
    use serde_json::{json, Value};
    use std::sync::Arc;

    const SIZE: i32 = 40;

    fn visual(layers: Value) -> AnimatedVisual {
        AnimatedVisual::parse(&json!({"fr": 30, "ip": 0, "op": 30, "w": SIZE, "h": SIZE, "layers": layers}).to_string()).unwrap()
    }

    fn square(x: f64, y: f64, side: f64, color: [f64; 3]) -> Value {
        json!([
            {"ty": "rc", "p": {"k": [x, y]}, "s": {"k": [side, side]}, "r": {"k": 0}},
            {"ty": "fl", "c": {"k": [color[0], color[1], color[2], 1]}, "o": {"k": 100}}
        ])
    }

    /// The frame drawn in `mode` of the Vello backend.
    fn draw_in(
        mode: VelloRenderingMode,
        renderer: &mut VisualRenderer,
        visual: &AnimatedVisual,
        frame: f64,
    ) -> Arc<dyn IRenderTargetBitmapImpl> {
        let interface: Rc<dyn IPlatformRenderInterface> = Rc::new(PlatformRenderInterface::default());
        let _scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>().to_constant(interface.clone());
        let target: Arc<dyn IRenderTargetBitmapImpl> =
            Arc::new(RenderTargetBitmapImpl::new(PixelSize::new(SIZE, SIZE), Vector::new(96.0, 96.0), vec![mode]));
        let mut commands = FrameCommands::default();
        visual.evaluate(frame, &mut commands);
        // An owned core: disposing the context ends the frame of the target.
        let mut context = DrawingContext::owned(Box::new(PlatformDrawingContext::new(target.create_drawing_context())));
        renderer.render(&commands, visual.width, visual.height, &mut context);
        context.dispose();
        target
    }

    fn draw(renderer: &mut VisualRenderer, visual: &AnimatedVisual, frame: f64) -> Arc<dyn IRenderTargetBitmapImpl> {
        draw_in(VelloRenderingMode::Cpu, renderer, visual, frame)
    }

    fn pixel(target: &Arc<dyn IRenderTargetBitmapImpl>, x: i32, y: i32) -> (u8, u8, u8, u8) {
        let framebuffer = target.lock();
        let offset = (y * framebuffer.row_bytes() + x * 4) as usize;
        let mut bytes = [0u8; 4];
        framebuffer.with_data(&mut |data| bytes.copy_from_slice(&data[offset..offset + 4]));
        let format = framebuffer.format();
        framebuffer.dispose();
        if format == PixelFormat::BGRA8888 {
            (bytes[2], bytes[1], bytes[0], bytes[3])
        } else {
            (bytes[0], bytes[1], bytes[2], bytes[3])
        }
    }

    #[test]
    fn a_moving_layer_is_drawn_where_it_is_and_builds_its_path_once() {
        let document = visual(json!([{"ty": 4, "ind": 1, "ip": 0, "op": 30,
            "ks": {"p": {"k": [{"t": 0, "s": [10, 20], "e": [30, 20]}, {"t": 10}]}},
            "shapes": square(0.0, 0.0, 10.0, [1.0, 0.0, 0.0])}]));
        let mut renderer = VisualRenderer::new();
        let first = draw(&mut renderer, &document, 0.0);
        assert_eq!(pixel(&first, 10, 20), (255, 0, 0, 255));
        assert_eq!(pixel(&first, 30, 20).3, 0);
        let last = draw(&mut renderer, &document, 10.0);
        assert_eq!(pixel(&last, 30, 20), (255, 0, 0, 255));
        assert_eq!(pixel(&last, 10, 20).3, 0);
        assert_eq!(renderer.geometries_built(), 1, "a moving transform builds no new path");
    }

    #[test]
    fn a_mask_cuts_its_layer_and_a_subtracted_mask_keeps_the_rest() {
        let mask = |mode: &str| json!([{"mode": mode, "inv": false, "o": {"k": 100}, "pt": {"k": {"c": true,
            "v": [[0, 0], [20, 0], [20, 40], [0, 40]], "i": [[0, 0], [0, 0], [0, 0], [0, 0]], "o": [[0, 0], [0, 0], [0, 0], [0, 0]]}}}]);
        let layer = |mode: &str| json!([{"ty": 4, "ind": 1, "ip": 0, "op": 30, "ks": {}, "hasMask": true,
            "masksProperties": mask(mode), "shapes": square(20.0, 20.0, 40.0, [0.0, 1.0, 0.0])}]);
        let mut renderer = VisualRenderer::new();
        let added = draw(&mut renderer, &visual(layer("a")), 0.0);
        assert_eq!(pixel(&added, 10, 20), (0, 255, 0, 255));
        assert_eq!(pixel(&added, 30, 20).3, 0);
        let subtracted = draw(&mut renderer, &visual(layer("s")), 0.0);
        assert_eq!(pixel(&subtracted, 10, 20).3, 0);
        assert_eq!(pixel(&subtracted, 30, 20), (0, 255, 0, 255));
    }

    /// A matte of a white square on the left and a black one on the right, over a blue layer.
    fn matted(mode: u8) -> AnimatedVisual {
        let matte = square(10.0, 20.0, 20.0, [1.0, 1.0, 1.0]);
        let black = square(30.0, 20.0, 20.0, [0.0, 0.0, 0.0]);
        let shapes = json!([
            {"ty": "gr", "it": [matte[0], matte[1], {"ty": "tr"}]},
            {"ty": "gr", "it": [black[0], black[1], {"ty": "tr"}]},
        ]);
        visual(json!([
            {"ty": 4, "ind": 1, "td": 1, "ip": 0, "op": 30, "ks": {}, "shapes": shapes},
            {"ty": 4, "ind": 2, "tt": mode, "ip": 0, "op": 30, "ks": {}, "shapes": square(20.0, 20.0, 40.0, [0.0, 0.0, 1.0])}
        ]))
    }

    /// Which halves of the blue layer each kind of matte keeps.
    const MATTES: [(u8, bool, bool); 4] = [(1, true, true), (2, false, false), (3, true, false), (4, false, true)];

    fn assert_mattes(mode: VelloRenderingMode) {
        for (kind, left, right) in MATTES {
            let mut renderer = VisualRenderer::new();
            let target = draw_in(mode, &mut renderer, &matted(kind), 0.0);
            let expect = |kept: bool| if kept { (0, 0, 255, 255) } else { (0, 0, 0, 0) };
            assert_eq!(pixel(&target, 10, 20), expect(left), "{mode:?}, matte {kind}, left");
            assert_eq!(pixel(&target, 30, 20), expect(right), "{mode:?}, matte {kind}, right");
            // Outside the matte: only the inverted kinds keep it.
            let outside = if kind % 2 == 0 { (0, 0, 255, 255) } else { (0, 0, 0, 0) };
            assert_eq!(pixel(&target, 20, 2), outside, "{mode:?}, matte {kind}, outside");
        }
    }

    #[test]
    fn each_kind_of_matte_keeps_its_part_of_the_layer() {
        assert_mattes(VelloRenderingMode::Cpu);
    }

    /// The same frames on the device: the layers and the luminance pass of the GPU modes. Without
    /// an adapter there is nothing to run them on.
    #[cfg(feature = "device-tests")]
    #[test]
    fn each_kind_of_matte_keeps_its_part_of_the_layer_on_the_device() {
        if let Err(error) = ferroui_vello::gpu::VelloWgpuDevice::headless() {
            println!("skipped: no adapter: {error}");
            return;
        }
        assert_mattes(VelloRenderingMode::Hybrid);
        assert_mattes(VelloRenderingMode::Gpu);
    }

    #[test]
    fn a_mask_at_half_opacity_keeps_half_of_what_it_covers() {
        let mask = |mode: &str| json!([{"mode": mode, "inv": false, "o": {"k": 50}, "pt": {"k": {"c": true,
            "v": [[0, 0], [20, 0], [20, 40], [0, 40]], "i": [[0, 0], [0, 0], [0, 0], [0, 0]], "o": [[0, 0], [0, 0], [0, 0], [0, 0]]}}}]);
        let layer = |mode: &str| json!([{"ty": 4, "ind": 1, "ip": 0, "op": 30, "ks": {}, "hasMask": true,
            "masksProperties": mask(mode), "shapes": square(20.0, 20.0, 40.0, [0.0, 1.0, 0.0])}]);
        let mut renderer = VisualRenderer::new();
        let added = draw(&mut renderer, &visual(layer("a")), 0.0);
        assert!(pixel(&added, 10, 20).3.abs_diff(128) <= 1, "{:?}", pixel(&added, 10, 20));
        assert_eq!(pixel(&added, 30, 20).3, 0);
        let subtracted = draw(&mut renderer, &visual(layer("s")), 0.0);
        assert!(pixel(&subtracted, 10, 20).3.abs_diff(128) <= 1, "{:?}", pixel(&subtracted, 10, 20));
        assert_eq!(pixel(&subtracted, 30, 20).3, 255);
    }

    #[test]
    fn the_range_of_a_matte_skips_what_is_nested_in_it() {
        let commands = [
            FrameCommand::BeginMatte(MatteMode::Alpha),
            FrameCommand::PushOpacity(0.5),
            FrameCommand::Pop,
            FrameCommand::EndMatteSource,
            FrameCommand::PushOpacity(0.5),
            FrameCommand::Pop,
            FrameCommand::Pop,
        ];
        assert_eq!(matte_range(&commands, 0), (3, 6));
    }
}
