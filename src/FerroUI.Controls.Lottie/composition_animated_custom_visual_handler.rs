use crate::CompositionAnimation;
use crate::feedback::{Feedback, Report};
use crate::playback::Playback;
use crate::visual_payload::{PlaybackSettings, VisualCommand, VisualPayload};
use ferroui_base::media::{ImmediateDrawingContext, MediaExtensions};
use ferroui_base::rendering::composition::{
    CompositionCustomVisualHandler, ICompositionCustomVisualHandler,
};
use ferroui_base::{Rect, Size};
use ferroui_skia::{ISkiaApiLease, ISkiaApiLeaseFeature};
use skia_safe::{AutoCanvasRestore, Canvas};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

pub(crate) struct CompositionAnimatedCustomVisualHandler {
    base: CompositionCustomVisualHandler,
    state: RefCell<State>,
    feedback: Arc<Feedback>,
}

struct State {
    animation: Option<Box<dyn CompositionAnimation>>,
    playback: Playback,
    settings: PlaybackSettings,
    generation: u64,
    revision: u64,
    renderer_available: Option<bool>,
}

impl CompositionAnimatedCustomVisualHandler {
    pub fn new(settings: PlaybackSettings, feedback: Arc<Feedback>) -> Self {
        Self {
            base: CompositionCustomVisualHandler::new(),
            state: RefCell::new(State {
                animation: None,
                playback: Playback::default(),
                settings,
                generation: 0,
                revision: 0,
                renderer_available: None,
            }),
            feedback,
        }
    }
}

impl State {
    fn report_frame(&self, feedback: &Feedback, delta: Duration) {
        feedback.push(
            self.generation,
            Report::Frame {
                revision: self.revision,
                position: self.playback.position,
                delta,
            },
        );
    }

    fn is_animating(&self) -> bool {
        self.playback.running && self.settings.playback_rate > 0.0
    }

    fn dispose_animation(&mut self) {
        if let Some(mut animation) = self.animation.take() {
            animation.on_disposed();
        }
    }
}

impl Drop for CompositionAnimatedCustomVisualHandler {
    fn drop(&mut self) {
        self.state.get_mut().dispose_animation();
    }
}

struct CanvasLease(Rc<dyn ISkiaApiLease>);

impl Drop for CanvasLease {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

impl ICompositionCustomVisualHandler for CompositionAnimatedCustomVisualHandler {
    fn handler_base(&self) -> &CompositionCustomVisualHandler {
        &self.base
    }

    fn on_message(&self, message: Arc<dyn Any + Send + Sync>) {
        let Some(payload) = message.downcast_ref::<VisualPayload>() else {
            return;
        };
        let mut state = self.state.borrow_mut();
        if state.settings.playback_rate != payload.settings.playback_rate {
            state.playback.reset_clock();
        }
        state.settings = payload.settings;
        state.revision = payload.revision;
        let repeats = state.settings.repeat_count;

        match &payload.command {
            VisualCommand::Replace(factory, autoplay) => {
                state.dispose_animation();
                state.generation = payload.generation;
                state.renderer_available = None;
                state.playback.stop();
                if let Some(factory) = factory {
                    match factory() {
                        Ok(mut animation) => {
                            self.feedback.push(
                                state.generation,
                                Report::Ready {
                                    size: animation.source_size(),
                                    duration: animation.duration().unwrap_or_default(),
                                },
                            );
                            if *autoplay {
                                state.playback.start(repeats);
                                animation.on_started();
                                self.feedback.push(state.generation, Report::Started);
                            }
                            state.animation = Some(animation);
                        }
                        Err(error) => self
                            .feedback
                            .push(state.generation, Report::LoadError(error)),
                    }
                }
            }
            VisualCommand::Start if state.animation.is_some() => {
                state.playback.start(repeats);
                if let Some(animation) = state.animation.as_mut() {
                    animation.on_started();
                }
                self.feedback.push(state.generation, Report::Started);
                state.report_frame(&self.feedback, Duration::ZERO);
            }
            VisualCommand::Stop => {
                state.playback.stop();
                if let Some(animation) = state.animation.as_mut() {
                    animation.on_stopped();
                    self.feedback.push(state.generation, Report::Stopped);
                }
                state.report_frame(&self.feedback, Duration::ZERO);
            }
            VisualCommand::Pause => {
                state.playback.pause();
                state.report_frame(&self.feedback, Duration::ZERO);
            }
            VisualCommand::Resume if state.animation.is_some() => {
                state.playback.resume(repeats);
                state.report_frame(&self.feedback, Duration::ZERO);
            }
            VisualCommand::Seek(position) => {
                if let Some(animation) = state.animation.as_ref() {
                    let duration = animation.duration();
                    state.playback.seek(*position, duration);
                    state.report_frame(&self.feedback, Duration::ZERO);
                }
            }
            VisualCommand::Start
            | VisualCommand::Resume
            | VisualCommand::Update
            | VisualCommand::Redraw => {}
        }
        self.base.invalidate();
    }

    fn on_animation_frame_update(&self) {
        if self.state.borrow().is_animating() {
            self.base.invalidate();
        }
    }

    fn on_render(&self, context: &mut ImmediateDrawingContext<'_>) {
        let mut state = self.state.borrow_mut();
        if state.animation.is_none() {
            return;
        }
        let feature = context.try_get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>());
        let feature = feature
            .as_ref()
            .and_then(|feature| feature.downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>());
        if state.renderer_available != Some(feature.is_some()) {
            state.renderer_available = Some(feature.is_some());
            self.feedback
                .push(state.generation, Report::Renderer(feature.is_some()));
        }
        let Some(feature) = feature else {
            state.playback.reset_clock();
            return;
        };

        let viewport = self.get_render_bounds();
        let State {
            animation,
            playback,
            settings,
            ..
        } = &mut *state;
        let Some(animation) = animation.as_mut() else {
            return;
        };
        let Some(destination) = destination_rect(viewport, animation.source_size(), *settings)
        else {
            playback.reset_clock();
            return;
        };

        let mut delta = None;
        if playback.running && settings.playback_rate > 0.0 {
            delta = playback.advance(
                self.base.composition_now(),
                settings.playback_rate,
                settings.repeat_count,
                &mut **animation,
            );
            if let Some(delta) = delta {
                animation.on_update(delta);
            }
        }
        draw_with_lease(
            &**feature,
            viewport,
            destination,
            &mut **animation,
            playback.position,
            playback.running,
        );

        if let Some(delta) = delta {
            state.report_frame(&self.feedback, delta);
        }
        if state.is_animating() {
            self.base.register_for_next_animation_frame_update();
        }
    }
}

fn destination_rect(viewport: Rect, source: Size, settings: PlaybackSettings) -> Option<Rect> {
    if viewport.width <= 0.0
        || viewport.height <= 0.0
        || source.width <= 0.0
        || source.height <= 0.0
    {
        return None;
    }
    let size = MediaExtensions::calculate_size(
        settings.stretch,
        viewport.size(),
        source,
        settings.stretch_direction,
    );
    Some(viewport.center_rect(Rect::from_size(size)))
}

pub(crate) fn draw_with_lease(
    feature: &dyn ISkiaApiLeaseFeature,
    viewport: Rect,
    destination: Rect,
    animation: &mut dyn CompositionAnimation,
    position: Duration,
    running: bool,
) {
    let lease = CanvasLease(feature.lease());
    let opacity = lease.0.current_opacity();
    lease.0.with_sk_canvas(&mut |canvas| {
        draw_animation(
            canvas,
            viewport,
            destination,
            opacity,
            animation,
            position,
            running,
        );
    });
}

pub(crate) fn draw_animation(
    canvas: &Canvas,
    viewport: Rect,
    destination: Rect,
    opacity: f64,
    animation: &mut dyn CompositionAnimation,
    position: Duration,
    running: bool,
) {
    let _restore = AutoCanvasRestore::guard(canvas, true);
    let clip = skia_safe::Rect::new(
        viewport.x as f32,
        viewport.y as f32,
        viewport.right() as f32,
        viewport.bottom() as f32,
    );
    canvas.clip_rect(clip, None, false);
    if opacity < 1.0 {
        canvas.save_layer_alpha_f(clip, opacity as f32);
    }
    animation.on_render(canvas, destination, position, running);
}
