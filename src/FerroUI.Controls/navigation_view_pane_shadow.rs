//! The shadow of the overlaying pane of a `NavigationView` and its motion: WinUI's `ShadowCaster`,
//! `SetDropShadow`, `UnsetDropShadow`, `ShadowCasterEaseOutStoryboard_Completed` and the
//! `VisualTransition`s of `PaneOverlayGroup` (`controls/dev/NavigationView/NavigationView.cpp` and
//! `NavigationView.xaml` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md).
//!
//! The caster is the empty `ShadowCaster` of the template, as wide as the open pane, beside the
//! split view. While the pane overlays the content it casts a `ThemeShadow` at the elevation
//! `PaneOverlayShadowDepth`. The storyboards of WinUI move it with the pane and fade it; here they
//! are composition key frame animations of the `Offset.X` and the `Opacity` of the caster's
//! composition visual, so the motion runs on the compositor of the renderer, and the shadow
//! visual, which shares the caster's transform parent and opacity, moves and fades with it.

use crate::Control;
use ferroui_base::media::{Shadow, ThemeShadow};
use ferroui_base::numerics::Vector2;
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::composition::{
    CompositionBatchTypes, CompositionScopedBatch, CompositionVisual, ElementComposition, ICompositionObjectAnimations,
};
use ferroui_base::{FerroLocator, LocatorExtensions, Ref, Vector3D, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// `c_paneOverlayShadowDepth`: the elevation of the shadow when the application resources have no
/// `PaneOverlayShadowDepth`.
pub(crate) const PANE_OVERLAY_SHADOW_DEPTH: f64 = 16.0;
/// The `VisualTransition` from `PaneNotOverlaying` to `PaneOverlaying`: 350 ms, key spline
/// 0.1,0.9 0.2,1.0 for the translation, linear for the opacity.
pub(crate) const PANE_SHADOW_OPEN_DURATION: Duration = Duration::from_millis(350);
/// `ShadowCasterEaseOutStoryboard`, from `PaneOverlaying` to `PaneNotOverlaying`: 120 ms.
pub(crate) const PANE_SHADOW_CLOSE_DURATION: Duration = Duration::from_millis(120);

#[derive(Default)]
pub(crate) struct PaneShadow {
    state: Rc<RefCell<State>>,
}

#[derive(Default)]
struct State {
    caster: Option<WeakRef<Control>>,
    /// The theme shadow the caster casts (WinUI creates a `ThemeShadow` in `SetDropShadow`; one is
    /// kept per navigation view).
    shadow: Option<Ref<ThemeShadow>>,
    /// The state of `PaneOverlayGroup` the caster shows; `None` before the first update.
    overlaying: Option<bool>,
    /// `UnsetDropShadow` waits for the end of the ease out before the shadow is removed.
    clear_pending: bool,
    generation: u64,
    running: Option<Running>,
}

struct Running {
    visual: Rc<CompositionVisual>,
    overlaying: bool,
    _batch: Rc<CompositionScopedBatch>,
    _completed: Subscription,
}

impl State {
    fn caster(&self) -> Option<Ref<Control>> {
        self.caster.as_ref().and_then(WeakRef::upgrade)
    }

    /// The end state of the transition to `overlaying`: the caster at its place, fully shown while
    /// the pane overlays (the opacity 0 of `PaneNotOverlaying` otherwise), and the shadow removed
    /// when `UnsetDropShadow` waited for the ease out.
    fn settle(&mut self, overlaying: bool, visual: Option<&Rc<CompositionVisual>>) {
        let caster = self.caster();
        let opacity = if overlaying { 1.0 } else { 0.0 };
        if let Some(visual) = visual {
            if let Some(caster) = &caster {
                visual.set_offset(Vector3D::new(caster.bounds().x, caster.bounds().y, 0.0));
            }
            visual.set_opacity(opacity as f32);
        }
        if let Some(caster) = &caster {
            caster.set_opacity(opacity);
            if !overlaying && std::mem::take(&mut self.clear_pending) {
                caster.set_shadow(None);
            }
        }
    }

    fn finish(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(running) = self.running.take() {
            self.settle(running.overlaying, Some(&running.visual));
        }
    }
}

impl PaneShadow {
    /// The caster of a new template (`m_shadowCaster`): an animation of the old one ends.
    pub(crate) fn set_caster(&self, caster: Option<Ref<Control>>) {
        let mut state = self.state.borrow_mut();
        state.finish();
        state.caster = caster.as_ref().map(Ref::downgrade);
        state.overlaying = None;
        state.clear_pending = false;
        if let Some(caster) = &caster {
            // PaneNotOverlaying until the first update.
            caster.set_opacity(0.0);
        }
    }

    /// `SetDropShadow`: in the compact and the minimal display mode, the caster casts the theme
    /// shadow at the elevation `depth` (the Z of its translation).
    pub(crate) fn set_drop_shadow(&self, depth: f64) {
        let mut state = self.state.borrow_mut();
        let Some(caster) = state.caster() else { return };
        state.clear_pending = false;
        let shadow = state.shadow.get_or_insert_with(ThemeShadow::new).clone();
        let shadow: Ref<Shadow> = shadow.upcast();
        if caster.shadow().as_ref() != Some(&shadow) {
            caster.set_shadow(Some(shadow));
        }
        let translation = caster.translation();
        caster.set_translation(Vector3D::new(translation.x, translation.y, depth));
    }

    /// `UnsetDropShadow`: the shadow is removed when the ease out of the caster has completed
    /// (`ShadowCasterEaseOutStoryboard_Completed`), at once when none runs.
    pub(crate) fn unset_drop_shadow(&self) {
        let mut state = self.state.borrow_mut();
        let Some(caster) = state.caster() else { return };
        if caster.shadow().is_none() {
            return;
        }
        if state.running.as_ref().is_some_and(|running| !running.overlaying) {
            state.clear_pending = true;
        } else {
            caster.set_shadow(None);
        }
    }

    /// `UpdatePaneOverlayGroup`'s `GoToState` with transitions: on a change of `PaneOverlayGroup`
    /// the caster slides by `distance` (the `NegativeOpenPaneLengthMinusCompactLength` of the split
    /// view) and fades, in on `PaneOverlaying` and out on `PaneNotOverlaying`. The first state, the
    /// states before the control is loaded (`animate` false: the adaptive layout of the first
    /// measure), disabled animations and a caster without a composition visual settle at once.
    pub(crate) fn update(&self, overlaying: bool, distance: f64, animate: bool) {
        let mut state = self.state.borrow_mut();
        let previous = state.overlaying.replace(overlaying);
        if previous == Some(overlaying) {
            return;
        }
        state.finish();
        let enabled = animate
            && FerroLocator::current()
                .get_service::<dyn IPlatformSettings>()
                .is_none_or(|settings| settings.animations_enabled());
        let caster = state.caster();
        let visual = caster
            .as_ref()
            .and_then(|caster| ElementComposition::get_element_visual(caster));
        let (Some(caster), Some(visual), Some(_), true, true) =
            (caster, visual, previous, enabled, distance.is_finite())
        else {
            state.settle(overlaying, None);
            return;
        };
        // The opacity of the caster changes when the batch completes: written now, it would replace
        // the animation of the composition visual's opacity.
        let base = caster.bounds().x;
        let compositor = visual.compositor();
        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        let spline = compositor.create_cubic_bezier_easing_function(Vector2::new(0.1, 0.9), Vector2::new(0.2, 1.0));
        let linear = compositor.create_linear_easing_function();
        let (from, to, duration) = if overlaying {
            (base + distance, base, PANE_SHADOW_OPEN_DURATION)
        } else {
            (base, base + distance, PANE_SHADOW_CLOSE_DURATION)
        };
        let offset = compositor.create_scalar_key_frame_animation();
        offset.insert_key_frame(0.0, from as f32);
        offset.insert_key_frame_with_easing(1.0, to as f32, spline);
        offset.set_duration(duration);
        let opacity = compositor.create_scalar_key_frame_animation();
        opacity.insert_key_frame(0.0, if overlaying { 0.0 } else { 1.0 });
        opacity.insert_key_frame_with_easing(1.0, if overlaying { 1.0 } else { 0.0 }, linear);
        opacity.set_duration(duration);
        visual.start_animation("Offset.X", &*offset);
        visual.start_animation("Opacity", &*opacity);
        let generation = state.generation;
        let weak = Rc::downgrade(&self.state);
        let completed = batch.completed(move |_| {
            let Some(state) = weak.upgrade() else { return };
            let mut state = state.borrow_mut();
            if state.generation == generation {
                state.finish();
            }
        });
        state.running = Some(Running {
            visual,
            overlaying,
            _batch: batch.clone(),
            _completed: completed,
        });
        drop(state);
        batch.end();
    }

    /// Ends a running motion (the template goes away or the control leaves the tree).
    pub(crate) fn clear(&self) {
        self.state.borrow_mut().finish();
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.state.borrow().running.is_some()
    }
}

impl Drop for PaneShadow {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.finish();
        }
    }
}
