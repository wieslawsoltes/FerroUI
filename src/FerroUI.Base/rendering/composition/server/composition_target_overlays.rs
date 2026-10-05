use super::{DiagnosticTextRenderer, FpsCounter, FrameTimeGraph, ServerCompositionTarget};
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{BoxShadows, Colors};
use crate::platform::{IDrawingContextImpl, IFontManagerImpl};
use crate::rendering::{LayoutPassTiming, RendererDebugOverlays};
use crate::{FerroLocator, LocatorExtensions, Matrix, Rect, RoundedRect, Size};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The debug overlays of a composition target: the frame counter and the
/// time graphs.
///
/// Time is read from the compositor clock.
#[derive(Default)]
pub struct CompositionTargetOverlays {
    fps_counter: OnceCell<Option<FpsCounter>>,
    render_time_graph: OnceCell<Option<FrameTimeGraph>>,
    compositor_update_time_graph: OnceCell<Option<FrameTimeGraph>>,
    update_time_graph: OnceCell<Option<FrameTimeGraph>>,
    layout_time_graph: OnceCell<Option<FrameTimeGraph>>,
    old_fps_counter_rect: Cell<Option<Rect>>,
    update_started: Cell<Duration>,
    debug_overlays: Cell<RendererDebugOverlays>,
    diagnostic_text_renderer: RefCell<Option<Rc<DiagnosticTextRenderer>>>,
}

impl CompositionTargetOverlays {
    pub fn new() -> Self {
        Self::default()
    }

    fn now(target: &ServerCompositionTarget) -> Duration {
        target.compositor().map_or(Duration::ZERO, |compositor| compositor.clock_elapsed())
    }

    fn fps_counter(&self, target: &ServerCompositionTarget) -> Option<&FpsCounter> {
        self.fps_counter
            .get_or_init(|| self.diagnostic_text_renderer().map(|renderer| FpsCounter::new(renderer, Self::now(target))))
            .as_ref()
    }

    fn layout_time_graph(&self) -> Option<&FrameTimeGraph> {
        self.layout_time_graph.get_or_init(|| self.create_time_graph("Layout")).as_ref()
    }

    fn render_time_graph(&self) -> Option<&FrameTimeGraph> {
        self.render_time_graph.get_or_init(|| self.create_time_graph("Render")).as_ref()
    }

    fn compositor_update_time_graph(&self) -> Option<&FrameTimeGraph> {
        self.compositor_update_time_graph.get_or_init(|| self.create_time_graph("GUpdate")).as_ref()
    }

    fn update_time_graph(&self) -> Option<&FrameTimeGraph> {
        self.update_time_graph.get_or_init(|| self.create_time_graph("TUpdate")).as_ref()
    }

    fn diagnostic_text_renderer(&self) -> Option<Rc<DiagnosticTextRenderer>> {
        if self.diagnostic_text_renderer.borrow().is_none() {
            // We are running in some unit test context
            FerroLocator::current().get_service::<dyn IFontManagerImpl>()?;
            *self.diagnostic_text_renderer.borrow_mut() = Some(Rc::new(DiagnosticTextRenderer::create_default()));
        }
        self.diagnostic_text_renderer.borrow().clone()
    }

    pub fn require_layer(&self) -> bool {
        self.debug_overlays.get().intersects(RendererDebugOverlays::DIRTY_RECTS)
    }

    fn create_time_graph(&self, title: &str) -> Option<FrameTimeGraph> {
        let renderer = self.diagnostic_text_renderer()?;
        Some(FrameTimeGraph::new(360, Size::new(360.0, 64.0), 1000.0 / 60.0, title, renderer))
    }

    pub fn on_changed(&self, debug_overlays: RendererDebugOverlays) {
        self.debug_overlays.set(debug_overlays);
        self.old_fps_counter_rect.set(None);

        if !debug_overlays.contains(RendererDebugOverlays::FPS) {
            if let Some(Some(counter)) = self.fps_counter.get() {
                counter.reset();
            }
        }

        if !debug_overlays.contains(RendererDebugOverlays::LAYOUT_TIME_GRAPH) {
            if let Some(Some(graph)) = self.layout_time_graph.get() {
                graph.reset();
            }
        }

        if !debug_overlays.contains(RendererDebugOverlays::RENDER_TIME_GRAPH) {
            for graph in [&self.render_time_graph, &self.compositor_update_time_graph, &self.update_time_graph] {
                if let Some(Some(graph)) = graph.get() {
                    graph.reset();
                }
            }
        }
    }

    fn capture_timing(&self) -> bool {
        self.debug_overlays.get().contains(RendererDebugOverlays::RENDER_TIME_GRAPH)
    }

    fn elapsed_ms(&self, target: &ServerCompositionTarget) -> f64 {
        Self::now(target).saturating_sub(self.update_started.get()).as_secs_f64() * 1000.0
    }

    pub fn draw(&self, target: &ServerCompositionTarget, target_context: &mut dyn IDrawingContextImpl, has_layer: bool) {
        let debug_overlays = self.debug_overlays.get();
        if debug_overlays != RendererDebugOverlays::NONE {
            if self.capture_timing() {
                let elapsed = self.elapsed_ms(target);
                if let Some(graph) = self.render_time_graph() {
                    graph.add_frame_value(elapsed);
                }
            }

            if debug_overlays.contains(RendererDebugOverlays::DIRTY_RECTS) {
                target.dirty_rects().visualize(target_context);
            }

            let scaling = target.scaling();
            target_context.set_transform(Matrix::create_scale(scaling, scaling));

            self.draw_overlays(target, target_context, has_layer, target.size());
        }
    }

    pub fn mark_update_call_start(&self, target: &ServerCompositionTarget) {
        if self.capture_timing() {
            self.update_started.set(Self::now(target));
        }
    }

    pub fn mark_update_call_end(&self, target: &ServerCompositionTarget) {
        if self.capture_timing() {
            let elapsed = self.elapsed_ms(target);
            if let Some(graph) = self.update_time_graph() {
                graph.add_frame_value(elapsed);
            }
        }
    }

    pub fn record_global_compositor_update_time(&self, _target: &ServerCompositionTarget, elapsed: Duration) {
        if self.capture_timing() {
            if let Some(graph) = self.compositor_update_time_graph() {
                graph.add_frame_value(elapsed.as_secs_f64() * 1000.0);
            }
        }
    }

    fn draw_overlays(
        &self,
        target: &ServerCompositionTarget,
        target_context: &mut dyn IDrawingContextImpl,
        has_layer: bool,
        logical_size: Size,
    ) {
        let debug_overlays = self.debug_overlays.get();
        if debug_overlays.contains(RendererDebugOverlays::FPS) {
            if let Some(counter) = self.fps_counter(target) {
                let aux = format!("V:{:04} R:{:04}", target.visited_visuals(), target.rendered_visuals());
                let rect = counter.render_fps(
                    target_context,
                    &aux,
                    has_layer,
                    self.old_fps_counter_rect.get(),
                    Self::now(target),
                );
                self.old_fps_counter_rect.set(Some(rect));
            }
        }

        let mut top = 0.0;
        let base_transform = target_context.transform();

        let mut draw_time_graph = |graph: Option<&FrameTimeGraph>, target_context: &mut dyn IDrawingContextImpl| {
            let Some(graph) = graph else { return };

            let left = logical_size.width - graph.size().width - 8.0;
            top += 8.0;
            if !has_layer {
                target_context.draw_rectangle(
                    Some(&ImmutableSolidColorBrush::new(Colors::WHITE)),
                    None,
                    RoundedRect::from_rect(Rect::new(left, top, graph.size().width, graph.size().height)),
                    &BoxShadows::default(),
                );
            }
            target_context.set_transform(Matrix::create_translation(left, top) * base_transform);
            graph.render(target_context);
            target_context.set_transform(base_transform);
            top += graph.size().height;
        };

        if debug_overlays.contains(RendererDebugOverlays::LAYOUT_TIME_GRAPH) {
            draw_time_graph(self.layout_time_graph(), target_context);
        }

        if debug_overlays.contains(RendererDebugOverlays::RENDER_TIME_GRAPH) {
            draw_time_graph(self.render_time_graph(), target_context);
            draw_time_graph(self.compositor_update_time_graph(), target_context);
            draw_time_graph(self.update_time_graph(), target_context);
        }
    }

    pub fn on_last_layout_pass_timing_changed(&self, _target: &ServerCompositionTarget, last_layout_pass_timing: LayoutPassTiming) {
        if self.debug_overlays.get().contains(RendererDebugOverlays::LAYOUT_TIME_GRAPH) {
            if let Some(graph) = self.layout_time_graph() {
                graph.add_frame_value(last_layout_pass_timing.elapsed.as_secs_f64() * 1000.0);
            }
        }
    }
}
