//! The zoom between the two views of a [`SemanticZoom`](super::SemanticZoom) on the compositor.
//!
//! WinUI zooms the element that holds both presenters from 1 to 0.5 (to the zoomed-out view)
//! or from 0.5 to 1 (`ScrollViewer::BringIntoViewport` with an animation in
//! `SemanticZoom::ChangeViews`); the zoomed-out presenter is scaled by 2 within it, so both
//! views shrink when the semantic zoom zooms out and grow when it zooms in, while the
//! `ZoomInView` and `ZoomOutView` states cross-fade them (`FadeInThemeAnimation`,
//! `FadeOutThemeAnimation`). The port runs the same motion as key frames of the composition
//! visuals of the two presenters (`Scale.X`, `Scale.Y` about their centres, and `Opacity`), so
//! it runs on the compositor of the renderer and the presenters' content is not drawn again.
//! The end of the animations (a scoped batch) is WinUI's `ViewChangeAnimationFinished`.

use crate::Control;
use ferroui_base::numerics::Vector2;
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::composition::{
    CompositionBatchTypes, CompositionScopedBatch, CompositionVisual, ElementComposition, ICompositionObjectAnimations,
};
use ferroui_base::{FerroLocator, LocatorExtensions, Ref, Vector3D};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The duration of the zoom and the cross-fade (WinUI takes the zoom from DirectManipulation and
/// the fades from the platform's animation library; the port's duration).
pub(crate) const ZOOM_DURATION: Duration = Duration::from_millis(300);

/// The scale of the zoomed-out view against the zoomed-in view (WinUI's `_zoomMin` of 0.5: the
/// `MinZoomFactor` of the template's ScrollViewer).
pub(crate) const ZOOM_MIN: f32 = 0.5;

type Completion = Box<dyn FnOnce()>;

#[derive(Default)]
struct State {
    generation: u64,
    running: Option<Running>,
}

struct Running {
    source: Rc<CompositionVisual>,
    destination: Rc<CompositionVisual>,
    completion: Option<Completion>,
    _batch: Rc<CompositionScopedBatch>,
    _completed: Subscription,
}

impl State {
    /// Ends the running zoom: both visuals at their scale, the source hidden, and the completion
    /// to call (outside the borrow).
    fn finish(&mut self) -> Option<Completion> {
        self.generation = self.generation.wrapping_add(1);
        let mut running = self.running.take()?;
        for (visual, opacity) in [(&running.source, 0.0), (&running.destination, 1.0)] {
            visual.set_scale(Vector3D::new(1.0, 1.0, 1.0));
            visual.set_opacity(opacity);
        }
        running.completion.take()
    }
}

/// The zoom of one semantic zoom; at most one runs at a time.
#[derive(Default)]
pub(crate) struct SemanticZoomTransition {
    state: Rc<RefCell<State>>,
}

impl SemanticZoomTransition {
    /// Starts the zoom from `source` to `destination` (the presenters of the views), out to the
    /// zoomed-out view when `to_zoomed_out`. `completion` runs when the animations have ended, or
    /// when another zoom or [`finish`](Self::finish) ends this one. Returns `false`, without
    /// calling `completion`, when nothing animates: a presenter without a composition visual
    /// (not rendered yet) or animations disabled by the platform.
    pub(crate) fn start(
        &self,
        source: &Ref<Control>,
        destination: &Ref<Control>,
        to_zoomed_out: bool,
        completion: impl FnOnce() + 'static,
    ) -> bool {
        self.finish();
        let enabled = FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .is_none_or(|settings| settings.animations_enabled());
        let (Some(source_visual), Some(destination_visual), true) = (
            ElementComposition::get_element_visual(source),
            ElementComposition::get_element_visual(destination),
            enabled,
        ) else {
            return false;
        };
        let compositor = source_visual.compositor();
        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        let spline = compositor.create_cubic_bezier_easing_function(Vector2::new(0.1, 0.9), Vector2::new(0.2, 1.0));
        let linear = compositor.create_linear_easing_function();
        // Zooming out both views shrink: the zoomed-in view from 1 to 0.5, the zoomed-out view from
        // 2 to 1; zooming in they grow back.
        let (source_scale, destination_scale) = if to_zoomed_out {
            ((1.0, ZOOM_MIN), (1.0 / ZOOM_MIN, 1.0))
        } else {
            ((1.0, 1.0 / ZOOM_MIN), (ZOOM_MIN, 1.0))
        };
        for (visual, element, (from, to), (fade_from, fade_to)) in [
            (&source_visual, source, source_scale, (1.0, 0.0)),
            (&destination_visual, destination, destination_scale, (0.0, 1.0)),
        ] {
            let size = element.bounds();
            visual.set_center_point(Vector3D::new(size.width / 2.0, size.height / 2.0, 0.0));
            for property in ["Scale.X", "Scale.Y"] {
                let scale = compositor.create_scalar_key_frame_animation();
                scale.insert_key_frame(0.0, from);
                scale.insert_key_frame_with_easing(1.0, to, spline.clone());
                scale.set_duration(ZOOM_DURATION);
                visual.start_animation(property, &*scale);
            }
            let opacity = compositor.create_scalar_key_frame_animation();
            opacity.insert_key_frame(0.0, fade_from);
            opacity.insert_key_frame_with_easing(1.0, fade_to, linear.clone());
            opacity.set_duration(ZOOM_DURATION);
            visual.start_animation("Opacity", &*opacity);
        }
        let mut state = self.state.borrow_mut();
        let generation = state.generation;
        let weak = Rc::downgrade(&self.state);
        let completed = batch.completed(move |_| {
            let Some(state) = weak.upgrade() else { return };
            let completion = {
                let mut state = state.borrow_mut();
                if state.generation != generation {
                    return;
                }
                state.finish()
            };
            if let Some(completion) = completion {
                completion();
            }
        });
        state.running = Some(Running {
            source: source_visual,
            destination: destination_visual,
            completion: Some(Box::new(completion)),
            _batch: batch.clone(),
            _completed: completed,
        });
        drop(state);
        batch.end();
        true
    }

    /// Ends a running zoom at once and calls its completion.
    pub(crate) fn finish(&self) {
        let completion = self.state.borrow_mut().finish();
        if let Some(completion) = completion {
            completion();
        }
    }

    /// Whether a zoom runs.
    pub(crate) fn is_running(&self) -> bool {
        self.state.borrow().running.is_some()
    }
}

impl Drop for SemanticZoomTransition {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            // The control is gone: nothing is left to complete.
            let _ = state.finish();
        }
    }
}
