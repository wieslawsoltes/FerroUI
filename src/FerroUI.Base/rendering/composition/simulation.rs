//! Public composition-engine simulation contract.
//!
//! This module defines a domain-agnostic simulation pipeline integrated directly
//! into FerroUI's [`Compositor`] and visual/effect pipeline:
//!
//! - [`ICompositionSimulation`] is implemented by any simulation engine — rigid-body
//!   or soft-body physics (such as `ferroui-physics`), spring/inertia choreography,
//!   harmonic wave fields, audio-reactive visualizers, particle systems, or custom
//!   procedural simulations.
//! - [`Compositor::register_simulation`] binds a simulation to the compositor's frame
//!   clock (`MediaContext` animation frames), stepping right before each composition
//!   batch commit and automatically idling when all simulations report
//!   [`SimulationStepResult::Idle`].
//! - [`SimulationVisualPatch`] applies per-frame transform, opacity, and GPU
//!   shader/effect updates ([`IImmutableEffect`] / [`ImmutableCustomEffect`]) directly
//!   to either a pure [`CompositionVisual`] or any UI-tree [`Visual`] (via
//!   [`ElementComposition::get_element_visual`]) without triggering layout passes.
//! - [`HarmonicWaveSimulation`] ships as a built-in, zero-physics reference mode
//!   demonstrating multi-mode extensibility directly inside `ferroui-base`.

use super::{CompositionVisual, Compositor, ElementComposition};
use crate::animation::SignedDuration;
use crate::media::effects::{CustomEffect, EffectExtensions, IImmutableEffect, ImmutableCustomEffect};
use crate::media::MediaContext;
use crate::{FerroLocator, LocatorExtensions, Ref, Vector, Vector3D, Visual};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// Unique identifier for a simulation registered with a [`Compositor`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SimulationId(u64);

impl SimulationId {
    /// Raw numeric identifier.
    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SimulationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sim#{}", self.0)
    }
}

/// High-level simulation mode tag reported by an [`ICompositionSimulation`].
///
/// The composition simulation contract is intentionally open-ended: physics is one
/// first-class mode alongside spring choreography, harmonic wave fields, audio
/// reactivity, particle systems, and arbitrary custom modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SimulationMode {
    /// Rigid-body, soft-body, cloth, fluid, or constraint physics simulation.
    Physics,
    /// Spring, damper, and inertial UI choreography.
    SpringDynamics,
    /// Procedural harmonic / wave / orbital visual motion.
    Harmonic,
    /// Audio-reactive visual and shader modulation.
    AudioReactive,
    /// Particle, flocking, or field-driven simulation.
    Particles,
    /// Domain-specific custom mode provided by an external crate or application.
    Custom(&'static str),
}

impl SimulationMode {
    /// Human-readable label for telemetry, tooling, and diagnostics.
    #[inline]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Physics => "Physics",
            Self::SpringDynamics => "Spring Dynamics",
            Self::Harmonic => "Harmonic Wave",
            Self::AudioReactive => "Audio Reactive",
            Self::Particles => "Particles",
            Self::Custom(name) => name,
        }
    }
}

impl fmt::Display for SimulationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Timing metadata for a single composition simulation step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationFrame {
    /// Clamped frame delta time in seconds (`> 0.0`).
    pub dt: f64,
    /// Total elapsed time on the compositor clock.
    pub elapsed: Duration,
    /// Monotonically increasing simulation frame counter for this compositor.
    pub frame_index: u64,
}

/// Lifecycle hint returned by [`ICompositionSimulation::step`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationStepResult {
    /// The simulation is still in motion; keep scheduling compositor frames.
    Active,
    /// The simulation has reached equilibrium; stop scheduling frames until
    /// woken via [`CompositionSimulationHandle::wake`] or
    /// [`Compositor::wake_simulations`].
    Idle,
}

/// How a [`SimulationVisualPatch`] updates the visual's effect / GPU shader slot.
#[derive(Clone, Default)]
pub enum SimulationEffectUpdate {
    /// Leave the visual's current effect untouched.
    #[default]
    Keep,
    /// Clear the visual's effect (`None`).
    Clear,
    /// Apply or update an immutable effect (including GPU WGSL/SkSL [`ImmutableCustomEffect`]).
    Set(Arc<dyn IImmutableEffect>),
}

impl fmt::Debug for SimulationEffectUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keep => f.write_str("Keep"),
            Self::Clear => f.write_str("Clear"),
            Self::Set(_) => f.write_str("Set(<IImmutableEffect>)"),
        }
    }
}

/// Per-frame visual & shader state written by an [`ICompositionSimulation`] to a
/// [`CompositionVisual`] or UI-tree [`Visual`].
///
/// Only populated (`Some`) fields are written, allowing multiple properties or
/// layout-managed properties (such as `offset`) to compose cleanly without fighting
/// FerroUI's layout engine.
#[derive(Clone, Debug, Default)]
pub struct SimulationVisualPatch {
    /// Composition-layer translation in DIPs (additive with layout `offset`, zero layout cost).
    pub translation: Option<Vector3D>,
    /// Composition-layer offset in DIPs (useful for pure `CompositionVisual` trees).
    pub offset: Option<Vector3D>,
    /// Rotation angle around Z in radians (rotates around `center_point`).
    pub rotation_angle: Option<f32>,
    /// Scale vector around `center_point`.
    pub scale: Option<Vector3D>,
    /// Transform pivot in local coordinates.
    pub center_point: Option<Vector3D>,
    /// Visual opacity (`0.0..=1.0`).
    pub opacity: Option<f32>,
    /// Optional effect / GPU shader update for this visual.
    pub effect: SimulationEffectUpdate,
}

impl SimulationVisualPatch {
    /// Creates an empty patch (no-op until fields are configured).
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets 2D/3D composition translation (additive with layout position).
    #[inline]
    pub fn with_translation(mut self, x: f64, y: f64, z: f64) -> Self {
        self.translation = Some(Vector3D::new(x, y, z));
        self
    }

    /// Sets 2D composition translation (`z = 0`).
    #[inline]
    pub fn with_translation_2d(self, x: f64, y: f64) -> Self {
        self.with_translation(x, y, 0.0)
    }

    /// Sets composition offset directly (typically for pure `CompositionVisual` nodes).
    #[inline]
    pub fn with_offset(mut self, x: f64, y: f64, z: f64) -> Self {
        self.offset = Some(Vector3D::new(x, y, z));
        self
    }

    /// Sets rotation angle in radians around the visual's center point.
    #[inline]
    pub fn with_rotation_angle(mut self, radians: f32) -> Self {
        self.rotation_angle = Some(radians);
        self
    }

    /// Sets scale around the visual's center point.
    #[inline]
    pub fn with_scale(mut self, sx: f64, sy: f64, sz: f64) -> Self {
        self.scale = Some(Vector3D::new(sx, sy, sz));
        self
    }

    /// Sets 2D scale (`sz = 1.0`).
    #[inline]
    pub fn with_scale_2d(self, sx: f64, sy: f64) -> Self {
        self.with_scale(sx, sy, 1.0)
    }

    /// Sets the transform pivot in local visual coordinates.
    #[inline]
    pub fn with_center_point(mut self, x: f64, y: f64, z: f64) -> Self {
        self.center_point = Some(Vector3D::new(x, y, z));
        self
    }

    /// Sets visual opacity (`0.0..=1.0`).
    #[inline]
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity.clamp(0.0, 1.0));
        self
    }

    /// Sets an immutable effect (including GPU shaders) on the visual.
    #[inline]
    pub fn with_effect(mut self, effect: Arc<dyn IImmutableEffect>) -> Self {
        self.effect = SimulationEffectUpdate::Set(effect);
        self
    }

    /// Convenience helper wrapping any [`CustomEffect`] value into an
    /// [`ImmutableCustomEffect`] snapshot for GPU/CPU shader execution.
    #[inline]
    pub fn with_custom_effect<E: CustomEffect>(self, effect: E) -> Self {
        let immutable: Arc<dyn IImmutableEffect> = Arc::new(ImmutableCustomEffect::new(effect));
        self.with_effect(immutable)
    }

    /// Clears any effect currently attached to the visual.
    #[inline]
    pub fn without_effect(mut self) -> Self {
        self.effect = SimulationEffectUpdate::Clear;
        self
    }

    /// Applies this patch directly to a [`CompositionVisual`].
    pub fn apply_to_composition_visual(&self, target: &CompositionVisual) {
        if let Some(center) = self.center_point {
            if target.center_point() != center {
                target.set_center_point(center);
            }
        }
        if let Some(offset) = self.offset {
            if target.offset() != offset {
                target.set_offset(offset);
            }
        }
        if let Some(translation) = self.translation {
            if target.translation() != translation {
                target.set_translation(translation);
            }
        }
        if let Some(angle) = self.rotation_angle {
            if (target.rotation_angle() - angle).abs() > 1e-6 {
                target.set_rotation_angle(angle);
            }
        }
        if let Some(scale) = self.scale {
            if target.scale() != scale {
                target.set_scale(scale);
            }
        }
        if let Some(opacity) = self.opacity {
            if (target.opacity() - opacity).abs() > 1e-5 {
                target.set_opacity(opacity);
            }
        }
        match &self.effect {
            SimulationEffectUpdate::Keep => {}
            SimulationEffectUpdate::Clear => {
                if target.effect().is_some() {
                    target.set_effect(None);
                }
            }
            SimulationEffectUpdate::Set(effect) => {
                let current = target.effect();
                if !EffectExtensions::effect_equals(current.as_deref(), Some(effect.as_ref())) {
                    target.set_effect(Some(effect.clone()));
                }
            }
        }
    }

    /// Applies this patch to a UI-tree [`Visual`]:
    ///
    /// - Composition transforms (`center_point`, `translation`, `rotation_angle`,
    ///   `scale`, `offset`, `opacity`) are written directly to the backing
    ///   [`CompositionVisual`] via [`ElementComposition::get_element_visual`] so
    ///   no UI layout pass is triggered.
    /// - Effect / GPU shader updates are written directly to the backing
    ///   [`CompositionVisual`] with `simulation_effect_override` enabled, avoiding
    ///   UI-thread draw-list re-recording (`invalidate_visual`) and property-store
    ///   allocations while preventing unrelated control repaints from clobbering
    ///   the simulation's live shader. Clearing restores `visual.effect()`.
    ///
    /// Returns `true` if the visual currently has a backing [`CompositionVisual`].
    pub fn apply_to_visual(&self, visual: &Visual) -> bool {
        let Some(comp) = ElementComposition::get_element_visual(visual) else {
            return false;
        };

        // Write transforms straight to the compositor visual (zero layout passes).
        if let Some(center) = self.center_point {
            if comp.center_point() != center {
                comp.set_center_point(center);
            }
        }
        if let Some(offset) = self.offset {
            if comp.offset() != offset {
                comp.set_offset(offset);
            }
        }
        if let Some(translation) = self.translation {
            if comp.translation() != translation {
                comp.set_translation(translation);
            }
        }
        if let Some(angle) = self.rotation_angle {
            if (comp.rotation_angle() - angle).abs() > 1e-6 {
                comp.set_rotation_angle(angle);
            }
        }
        if let Some(scale) = self.scale {
            if comp.scale() != scale {
                comp.set_scale(scale);
            }
        }
        if let Some(opacity) = self.opacity {
            if (comp.opacity() - opacity).abs() > 1e-5 {
                comp.set_opacity(opacity);
            }
        }

        // Write effect / GPU shader straight to CompositionVisual with ZERO UI-thread
        // draw-list re-recording and ZERO property-store allocations. `simulation_effect_override`
        // ensures unrelated control repaints (hover/focus/click) never clobber the live shader.
        match &self.effect {
            SimulationEffectUpdate::Keep => {}
            SimulationEffectUpdate::Clear => {
                comp.set_simulation_effect_override(false);
                let base_effect = visual.effect().as_ref().map(EffectExtensions::to_immutable);
                let current_comp = comp.effect();
                let same = match (current_comp.as_deref(), base_effect.as_deref()) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.equals(Some(b)),
                    _ => false,
                };
                if !same {
                    comp.set_effect(base_effect);
                }
            }
            SimulationEffectUpdate::Set(effect) => {
                comp.set_simulation_effect_override(true);
                let current_comp = comp.effect();
                if !EffectExtensions::effect_equals(current_comp.as_deref(), Some(effect.as_ref())) {
                    comp.set_effect(Some(effect.clone()));
                }
            }
        }

        true
    }
}

/// Target visual driven by an [`ICompositionSimulation`].
///
/// Supports both pure [`CompositionVisual`] nodes (`SpriteVisual`, `ShapeVisual`,
/// `CompositionContainerVisual`, etc.) and any UI-tree [`Visual`] (`Border`,
/// `Button`, `StackPanel`, custom controls).
#[derive(Clone)]
pub enum SimulationTarget {
    /// A direct composition-layer visual.
    Composition(Rc<CompositionVisual>),
    /// A FerroUI UI-tree visual whose backing [`CompositionVisual`] is resolved
    /// lazily via [`ElementComposition::get_element_visual`].
    Visual(Ref<Visual>),
}

impl SimulationTarget {
    /// Creates a target wrapping a pure [`CompositionVisual`].
    #[inline]
    pub fn composition(visual: Rc<CompositionVisual>) -> Self {
        Self::Composition(visual)
    }

    /// Creates a target wrapping a UI-tree [`Visual`].
    #[inline]
    pub fn visual(visual: Ref<Visual>) -> Self {
        Self::Visual(visual)
    }

    /// Resolves the backing [`CompositionVisual`], if currently attached.
    #[inline]
    pub fn composition_visual(&self) -> Option<Rc<CompositionVisual>> {
        match self {
            Self::Composition(cv) => Some(cv.clone()),
            Self::Visual(v) => ElementComposition::get_element_visual(v),
        }
    }

    /// Returns the visual's current size in DIPs, if available.
    pub fn size(&self) -> Option<Vector> {
        match self {
            Self::Composition(cv) => Some(cv.size()),
            Self::Visual(v) => {
                let bounds = v.bounds();
                if bounds.width > 0.0 || bounds.height > 0.0 {
                    Some(Vector::new(bounds.width, bounds.height))
                } else {
                    ElementComposition::get_element_visual(v).map(|cv| cv.size())
                }
            }
        }
    }

    /// Applies `patch` to this target, returning `true` when the visual is
    /// attached and updated.
    pub fn apply(&self, patch: &SimulationVisualPatch) -> bool {
        match self {
            Self::Composition(cv) => {
                patch.apply_to_composition_visual(cv);
                true
            }
            Self::Visual(v) => patch.apply_to_visual(v),
        }
    }

    /// Resets composition-layer overrides (`translation`, `rotation_angle`, `scale`,
    /// `opacity`, and optionally clears `effect`) back to identity defaults.
    pub fn reset_to_identity(&self, clear_effect: bool) {
        let mut patch = SimulationVisualPatch::new()
            .with_translation(0.0, 0.0, 0.0)
            .with_rotation_angle(0.0)
            .with_scale(1.0, 1.0, 1.0)
            .with_opacity(1.0);
        if clear_effect {
            patch = patch.without_effect();
        }
        let _ = self.apply(&patch);
    }
}

/// Execution context handed to [`ICompositionSimulation::step`] on each compositor frame.
pub struct CompositionSimulationContext<'a> {
    compositor: &'a Rc<Compositor>,
    frame: SimulationFrame,
    applied_updates: usize,
}

impl<'a> CompositionSimulationContext<'a> {
    #[inline]
    pub(crate) fn new(compositor: &'a Rc<Compositor>, frame: SimulationFrame) -> Self {
        Self { compositor, frame, applied_updates: 0 }
    }

    /// Frame timing metadata (`dt`, `elapsed`, `frame_index`).
    #[inline]
    pub fn frame(&self) -> SimulationFrame {
        self.frame
    }

    /// The owning [`Compositor`].
    #[inline]
    pub fn compositor(&self) -> &'a Rc<Compositor> {
        self.compositor
    }

    /// Applies `patch` to `target` and records the update.
    #[inline]
    pub fn apply(&mut self, target: &SimulationTarget, patch: &SimulationVisualPatch) -> bool {
        if target.apply(patch) {
            self.applied_updates += 1;
            true
        } else {
            false
        }
    }

    /// Applies `patch` directly to a [`CompositionVisual`].
    #[inline]
    pub fn apply_composition(&mut self, visual: &CompositionVisual, patch: &SimulationVisualPatch) {
        patch.apply_to_composition_visual(visual);
        self.applied_updates += 1;
    }

    /// Applies `patch` to a UI-tree [`Visual`].
    #[inline]
    pub fn apply_visual(&mut self, visual: &Visual, patch: &SimulationVisualPatch) -> bool {
        if patch.apply_to_visual(visual) {
            self.applied_updates += 1;
            true
        } else {
            false
        }
    }

    /// Number of visual patches applied during this simulation step.
    #[inline]
    pub fn applied_updates(&self) -> usize {
        self.applied_updates
    }
}

/// Public simulation contract integrated into FerroUI's [`Compositor`].
///
/// Any simulation engine (rigid-body/soft-body physics, spring choreography,
/// harmonic wave fields, audio visualizers, particle systems, etc.) implements
/// this trait and registers with [`Compositor::register_simulation`].
pub trait ICompositionSimulation: 'static {
    /// Semantic mode of this simulation.
    fn mode(&self) -> SimulationMode;

    /// Human-readable name used in diagnostics and developer tooling.
    fn name(&self) -> &str;

    /// Called once when registered on a [`Compositor`].
    fn on_attached(&mut self, _compositor: &Rc<Compositor>) {}

    /// Called once when unregistered or dropped from a [`Compositor`].
    fn on_detached(&mut self) {}

    /// Advances the simulation for the current composition frame and writes visual
    /// / shader updates through `ctx`.
    ///
    /// Return [`SimulationStepResult::Active`] while work remains, or
    /// [`SimulationStepResult::Idle`] to let the compositor sleep with zero CPU usage
    /// until woken via [`CompositionSimulationHandle::wake`].
    fn step(&mut self, ctx: &mut CompositionSimulationContext<'_>) -> SimulationStepResult;
}

struct RegisteredSimulation {
    id: SimulationId,
    mode: SimulationMode,
    paused: bool,
    awake: bool,
    simulation: Box<dyn ICompositionSimulation>,
}

/// RAII handle returned by [`Compositor::register_simulation`].
///
/// Dropping the handle unregisters the simulation unless [`detach`](Self::detach)
/// was called.
pub struct CompositionSimulationHandle {
    compositor: Weak<Compositor>,
    id: SimulationId,
    mode: SimulationMode,
    detached: bool,
}

impl CompositionSimulationHandle {
    /// Unique identifier of this simulation registration.
    #[inline]
    pub fn id(&self) -> SimulationId {
        self.id
    }

    /// The simulation's reported [`SimulationMode`].
    #[inline]
    pub fn mode(&self) -> SimulationMode {
        self.mode
    }

    /// Returns `true` if this simulation is still registered on its compositor.
    pub fn is_registered(&self) -> bool {
        self.compositor.upgrade().is_some_and(|c| c.simulations().contains(self.id))
    }

    /// Returns `true` if this simulation is currently paused.
    pub fn is_paused(&self) -> bool {
        self.compositor.upgrade().is_some_and(|c| c.simulations().is_paused(self.id))
    }

    /// Pauses stepping for this simulation without unregistering it.
    pub fn pause(&self) {
        self.set_paused(true);
    }

    /// Resumes stepping for this simulation and wakes the compositor loop.
    pub fn resume(&self) {
        self.set_paused(false);
    }

    /// Sets the paused state of this simulation.
    pub fn set_paused(&self, paused: bool) {
        if let Some(compositor) = self.compositor.upgrade() {
            compositor.simulations().set_paused(&compositor, self.id, paused);
        }
    }

    /// Wakes an idle simulation so it steps on the next composition frame.
    pub fn wake(&self) {
        if let Some(compositor) = self.compositor.upgrade() {
            compositor.simulations().wake_one(&compositor, self.id);
        }
    }

    /// Unregisters this simulation immediately and invokes [`ICompositionSimulation::on_detached`].
    pub fn unregister(&mut self) {
        self.detached = true;
        if let Some(compositor) = self.compositor.upgrade() {
            compositor.simulations().unregister(self.id);
        }
    }

    /// Detaches the handle so the simulation remains registered for the lifetime
    /// of the compositor even after the handle is dropped.
    pub fn detach(mut self) {
        self.detached = true;
    }
}

impl Drop for CompositionSimulationHandle {
    fn drop(&mut self) {
        if !self.detached {
            if let Some(compositor) = self.compositor.upgrade() {
                compositor.simulations().unregister(self.id);
            }
        }
    }
}

/// Internal simulation scheduler owned by [`Compositor`].
pub(crate) struct CompositionSimulationHost {
    next_id: Cell<u64>,
    entries: RefCell<Vec<RegisteredSimulation>>,
    frame_scheduled: Cell<bool>,
    stepping: Cell<bool>,
    last_frame_time: Cell<Option<SignedDuration>>,
    frame_index: Cell<u64>,
}

impl CompositionSimulationHost {
    /// Maximum frame delta clamped into simulations so debugger breaks or window
    /// drags never destabilize integrators (50 ms = 20 FPS floor).
    const MAX_DT_SECONDS: f64 = 1.0 / 20.0;
    /// Minimum positive delta passed to simulations.
    const MIN_DT_SECONDS: f64 = 1.0 / 240.0;

    pub(crate) fn new() -> Self {
        Self {
            next_id: Cell::new(1),
            entries: RefCell::new(Vec::new()),
            frame_scheduled: Cell::new(false),
            stepping: Cell::new(false),
            last_frame_time: Cell::new(None),
            frame_index: Cell::new(0),
        }
    }

    pub(crate) fn register(
        &self,
        compositor: &Rc<Compositor>,
        mut simulation: Box<dyn ICompositionSimulation>,
    ) -> CompositionSimulationHandle {
        let id = SimulationId(self.next_id.get());
        self.next_id.set(id.0 + 1);
        let mode = simulation.mode();
        simulation.on_attached(compositor);

        self.entries.borrow_mut().push(RegisteredSimulation {
            id,
            mode,
            paused: false,
            awake: true,
            simulation,
        });
        self.ensure_frame_scheduled(compositor);

        CompositionSimulationHandle {
            compositor: Rc::downgrade(compositor),
            id,
            mode,
            detached: false,
        }
    }

    pub(crate) fn unregister(&self, id: SimulationId) {
        let removed = {
            let mut entries = self.entries.borrow_mut();
            entries.iter().position(|e| e.id == id).map(|idx| entries.remove(idx))
        };
        if let Some(mut entry) = removed {
            entry.simulation.on_detached();
        }
    }

    pub(crate) fn contains(&self, id: SimulationId) -> bool {
        self.entries.borrow().iter().any(|e| e.id == id)
    }

    pub(crate) fn is_paused(&self, id: SimulationId) -> bool {
        self.entries.borrow().iter().find(|e| e.id == id).is_some_and(|e| e.paused)
    }

    pub(crate) fn count(&self) -> usize {
        self.entries.borrow().len()
    }

    pub(crate) fn active_modes(&self) -> Vec<SimulationMode> {
        self.entries.borrow().iter().filter(|e| !e.paused).map(|e| e.mode).collect()
    }

    pub(crate) fn set_paused(&self, compositor: &Rc<Compositor>, id: SimulationId, paused: bool) {
        let mut wake_needed = false;
        if let Some(entry) = self.entries.borrow_mut().iter_mut().find(|e| e.id == id) {
            entry.paused = paused;
            if !paused {
                entry.awake = true;
                wake_needed = true;
            }
        }
        if wake_needed {
            self.last_frame_time.set(None);
            self.ensure_frame_scheduled(compositor);
        }
    }

    pub(crate) fn wake_one(&self, compositor: &Rc<Compositor>, id: SimulationId) {
        let mut found = false;
        if let Some(entry) = self.entries.borrow_mut().iter_mut().find(|e| e.id == id) {
            entry.awake = true;
            found = !entry.paused;
        }
        if found {
            self.last_frame_time.set(None);
            self.ensure_frame_scheduled(compositor);
        }
    }

    pub(crate) fn wake_all(&self, compositor: &Rc<Compositor>) {
        let mut any_active = false;
        for entry in self.entries.borrow_mut().iter_mut() {
            entry.awake = true;
            if !entry.paused {
                any_active = true;
            }
        }
        if any_active {
            self.last_frame_time.set(None);
            self.ensure_frame_scheduled(compositor);
        }
    }

    pub(crate) fn has_awake_simulations(&self) -> bool {
        self.entries.borrow().iter().any(|e| !e.paused && e.awake)
    }

    /// Manually steps all non-paused awake simulations by `dt_seconds`.
    ///
    /// Useful for deterministic headless testing, offline rendering, or custom
    /// frame loops.
    pub(crate) fn step_all(&self, compositor: &Rc<Compositor>, dt_seconds: f64, elapsed: Duration) -> SimulationStepResult {
        if self.stepping.get() {
            return SimulationStepResult::Active;
        }
        self.stepping.set(true);

        let dt = if dt_seconds.is_finite() && dt_seconds > 0.0 {
            dt_seconds.clamp(Self::MIN_DT_SECONDS, Self::MAX_DT_SECONDS)
        } else {
            1.0 / 60.0
        };

        let frame_index = self.frame_index.get().wrapping_add(1);
        self.frame_index.set(frame_index);
        let frame = SimulationFrame { dt, elapsed, frame_index };

        // Swap entries out while stepping so a simulation's callbacks can safely
        // register, wake, or unregister simulations without RefCell borrow conflicts.
        let mut active_entries = std::mem::take(&mut *self.entries.borrow_mut());
        let mut any_active = false;
        let mut total_updates = 0usize;

        for entry in &mut active_entries {
            if entry.paused || !entry.awake {
                continue;
            }
            let mut ctx = CompositionSimulationContext::new(compositor, frame);
            let result = entry.simulation.step(&mut ctx);
            total_updates += ctx.applied_updates();
            match result {
                SimulationStepResult::Active => {
                    entry.awake = true;
                    any_active = true;
                }
                SimulationStepResult::Idle => {
                    entry.awake = false;
                }
            }
        }

        // Merge back any simulations that were added re-entrantly during `step`.
        {
            let mut newly_added = self.entries.borrow_mut();
            active_entries.append(&mut newly_added);
            *newly_added = active_entries;
        }

        self.stepping.set(false);

        if total_updates > 0 {
            compositor.ensure_commit_requested();
        }

        if any_active {
            SimulationStepResult::Active
        } else {
            SimulationStepResult::Idle
        }
    }

    pub(crate) fn ensure_frame_scheduled(&self, compositor: &Rc<Compositor>) {
        if self.frame_scheduled.get() || !self.has_awake_simulations() {
            return;
        }
        // Only hook into MediaContext when one is registered on this thread
        // (headless unit tests may construct a standalone Compositor without it).
        let Some(media_context) = FerroLocator::current().get_service::<MediaContext>() else {
            return;
        };

        self.frame_scheduled.set(true);
        let weak_compositor = Rc::downgrade(compositor);
        media_context.request_animation_frame(move |now| {
            let Some(compositor) = weak_compositor.upgrade() else {
                return;
            };
            let host = compositor.simulations();
            host.frame_scheduled.set(false);

            if !host.has_awake_simulations() {
                host.last_frame_time.set(None);
                return;
            }

            let dt = match host.last_frame_time.get() {
                Some(prev) => {
                    let delta_ticks = now.ticks().saturating_sub(prev.ticks());
                    let secs = delta_ticks as f64 / SignedDuration::TICKS_PER_SECOND as f64;
                    if secs > 0.0 && secs.is_finite() { secs } else { 1.0 / 60.0 }
                }
                None => 1.0 / 60.0,
            };
            host.last_frame_time.set(Some(now));

            let result = host.step_all(&compositor, dt, compositor.clock_elapsed());
            if result == SimulationStepResult::Active || host.has_awake_simulations() {
                host.ensure_frame_scheduled(&compositor);
            } else {
                host.last_frame_time.set(None);
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Built-in non-physics reference mode: HarmonicWaveSimulation
// ---------------------------------------------------------------------------

/// Optional callback invoked by [`HarmonicWaveSimulation`] per target per frame
/// to synthesize an effect/shader (for example, a velocity- or phase-reactive
/// [`ImmutableCustomEffect`]).
pub type HarmonicEffectDriver = Box<dyn Fn(usize, f32, f32, f32) -> SimulationEffectUpdate>;

/// Built-in, zero-physics [`ICompositionSimulation`] (`SimulationMode::Harmonic`)
/// that drives a set of [`SimulationTarget`]s with smooth phase-staggered harmonic
/// waves directly on the [`Compositor`].
///
/// Serves both as a ready-to-use UI choreography mode and as a reference proving
/// that the composition simulation contract is completely decoupled from physics.
pub struct HarmonicWaveSimulation {
    name: String,
    targets: Vec<SimulationTarget>,
    amplitude_y: f64,
    amplitude_x: f64,
    tilt_radians: f32,
    pulse_scale: f64,
    frequency_hz: f64,
    phase_step: f64,
    time: f64,
    effect_driver: Option<HarmonicEffectDriver>,
}

impl HarmonicWaveSimulation {
    /// Creates a harmonic wave simulation targeting `targets`.
    pub fn new(name: impl Into<String>, targets: Vec<SimulationTarget>) -> Self {
        Self {
            name: name.into(),
            targets,
            amplitude_y: 26.0,
            amplitude_x: 10.0,
            tilt_radians: 0.08,
            pulse_scale: 0.045,
            frequency_hz: 0.55,
            phase_step: 0.85,
            time: 0.0,
            effect_driver: None,
        }
    }

    /// Configures X/Y wave amplitudes in DIPs.
    pub fn with_amplitudes(mut self, amplitude_x: f64, amplitude_y: f64) -> Self {
        self.amplitude_x = amplitude_x;
        self.amplitude_y = amplitude_y;
        self
    }

    /// Configures rotation tilt amplitude in radians and breathing scale factor.
    pub fn with_tilt_and_pulse(mut self, tilt_radians: f32, pulse_scale: f64) -> Self {
        self.tilt_radians = tilt_radians;
        self.pulse_scale = pulse_scale;
        self
    }

    /// Configures wave frequency (Hz) and per-visual phase offset (radians).
    pub fn with_frequency(mut self, frequency_hz: f64, phase_step: f64) -> Self {
        self.frequency_hz = frequency_hz;
        self.phase_step = phase_step;
        self
    }

    /// Attaches an optional per-visual effect/shader generator receiving
    /// `(visual_index, phase_radians, normalized_velocity_y, energy_0_to_1)`.
    pub fn with_effect_driver(
        mut self,
        driver: impl Fn(usize, f32, f32, f32) -> SimulationEffectUpdate + 'static,
    ) -> Self {
        self.effect_driver = Some(Box::new(driver));
        self
    }
}

impl ICompositionSimulation for HarmonicWaveSimulation {
    fn mode(&self) -> SimulationMode {
        SimulationMode::Harmonic
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn on_detached(&mut self) {
        for target in &self.targets {
            target.reset_to_identity(true);
        }
    }

    fn step(&mut self, ctx: &mut CompositionSimulationContext<'_>) -> SimulationStepResult {
        if self.targets.is_empty() {
            return SimulationStepResult::Idle;
        }
        self.time += ctx.frame().dt;
        let omega = std::f64::consts::TAU * self.frequency_hz;

        for (idx, target) in self.targets.iter().enumerate() {
            let phase = self.time * omega + idx as f64 * self.phase_step;
            let s = phase.sin();
            let c = phase.cos();

            let tx = c * self.amplitude_x;
            let ty = s * self.amplitude_y;
            let rot = (c as f32) * self.tilt_radians;
            let scale = 1.0 + s * self.pulse_scale;

            let mut patch = SimulationVisualPatch::new()
                .with_translation_2d(tx, ty)
                .with_rotation_angle(rot)
                .with_scale_2d(scale, scale);

            if let Some(size) = target.size() {
                if size.x > 0.0 && size.y > 0.0 {
                    patch = patch.with_center_point(size.x * 0.5, size.y * 0.5, 0.0);
                }
            }

            if let Some(driver) = &self.effect_driver {
                let vel_norm = c as f32;
                let energy = ((s * 0.5 + 0.5) as f32).clamp(0.0, 1.0);
                patch.effect = driver(idx, phase as f32, vel_norm, energy);
            }

            ctx.apply(target, &patch);
        }

        SimulationStepResult::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::effects::ImmutableBlurEffect;
    use crate::rendering::composition::test_compositor::TestCompositor;
    use std::cell::Cell;

    struct CountingSimulation {
        mode: SimulationMode,
        target: SimulationTarget,
        steps: Rc<Cell<u32>>,
        max_steps: u32,
        apply_blur: bool,
    }

    impl ICompositionSimulation for CountingSimulation {
        fn mode(&self) -> SimulationMode {
            self.mode
        }

        fn name(&self) -> &str {
            "counting-test"
        }

        fn step(&mut self, ctx: &mut CompositionSimulationContext<'_>) -> SimulationStepResult {
            let n = self.steps.get() + 1;
            self.steps.set(n);
            let mut patch = SimulationVisualPatch::new()
                .with_translation_2d(n as f64 * 12.0, n as f64 * -4.0)
                .with_rotation_angle(n as f32 * 0.1)
                .with_scale_2d(1.0 + n as f64 * 0.05, 1.0);
            if self.apply_blur {
                let blur: Arc<dyn IImmutableEffect> = Arc::new(ImmutableBlurEffect::new(n as f64 * 2.0));
                patch = patch.with_effect(blur);
            }
            ctx.apply(&self.target, &patch);
            if n >= self.max_steps {
                SimulationStepResult::Idle
            } else {
                SimulationStepResult::Active
            }
        }
    }

    #[test]
    fn compositor_simulation_drives_visual_and_effect_and_idles() {
        let test = TestCompositor::new();
        let compositor = &test.compositor;
        let visual = compositor.create_solid_color_visual();
        visual.set_size(Vector::new(120.0, 80.0));

        let steps = Rc::new(Cell::new(0));
        let handle = compositor.register_simulation(CountingSimulation {
            mode: SimulationMode::Custom("TestMode"),
            target: SimulationTarget::composition((*visual).clone()),
            steps: steps.clone(),
            max_steps: 3,
            apply_blur: true,
        });

        assert_eq!(compositor.simulation_count(), 1);
        assert_eq!(compositor.active_simulation_modes(), vec![SimulationMode::Custom("TestMode")]);
        assert!(handle.is_registered());

        assert_eq!(compositor.step_simulations(1.0 / 60.0), SimulationStepResult::Active);
        assert_eq!(compositor.step_simulations(1.0 / 60.0), SimulationStepResult::Active);
        assert_eq!(compositor.step_simulations(1.0 / 60.0), SimulationStepResult::Idle);
        assert_eq!(steps.get(), 3);

        // Idle simulation does not step again until woken.
        assert_eq!(compositor.step_simulations(1.0 / 60.0), SimulationStepResult::Idle);
        assert_eq!(steps.get(), 3);

        assert!((visual.translation().x - 36.0).abs() < 1e-6);
        assert!((visual.translation().y - (-12.0)).abs() < 1e-6);
        assert!((visual.rotation_angle() - 0.3).abs() < 1e-5);
        assert!(visual.effect().is_some());

        // Waking resumes stepping immediately.
        handle.wake();
        assert_eq!(compositor.step_simulations(1.0 / 60.0), SimulationStepResult::Idle);
        assert_eq!(steps.get(), 4);

        // Dropping the handle unregisters cleanly.
        drop(handle);
        assert_eq!(compositor.simulation_count(), 0);
    }

    #[test]
    fn multi_mode_simulations_coexist_on_single_compositor() {
        let test = TestCompositor::new();
        let compositor = &test.compositor;

        let v1 = compositor.create_solid_color_visual();
        v1.set_size(Vector::new(100.0, 60.0));
        let v2 = compositor.create_solid_color_visual();
        v2.set_size(Vector::new(100.0, 60.0));

        let harmonic_handle = compositor.register_simulation(
            HarmonicWaveSimulation::new("wave", vec![SimulationTarget::composition((*v1).clone())])
                .with_amplitudes(15.0, 30.0),
        );
        let steps = Rc::new(Cell::new(0));
        let physics_handle = compositor.register_simulation(CountingSimulation {
            mode: SimulationMode::Physics,
            target: SimulationTarget::composition((*v2).clone()),
            steps: steps.clone(),
            max_steps: 10,
            apply_blur: false,
        });

        assert_eq!(compositor.simulation_count(), 2);
        let modes = compositor.active_simulation_modes();
        assert!(modes.contains(&SimulationMode::Harmonic));
        assert!(modes.contains(&SimulationMode::Physics));

        assert_eq!(compositor.step_simulations(0.016), SimulationStepResult::Active);
        assert!(v1.translation().x.abs() > 0.0 || v1.translation().y.abs() > 0.0);
        assert!((v1.center_point().x - 50.0).abs() < 1e-6);
        assert_eq!(steps.get(), 1);

        // Pausing one mode leaves the other mode running.
        physics_handle.pause();
        assert!(physics_handle.is_paused());
        assert_eq!(compositor.step_simulations(0.016), SimulationStepResult::Active);
        assert_eq!(steps.get(), 1);

        drop(harmonic_handle);
        drop(physics_handle);
        assert_eq!(compositor.simulation_count(), 0);
        // HarmonicWaveSimulation resets its target on detach.
        assert_eq!(v1.translation(), Vector3D::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn simulation_effect_override_toggles_cleanly_on_composition_visual() {
        let test = TestCompositor::new();
        let compositor = &test.compositor;
        let visual = compositor.create_solid_color_visual();

        assert!(!visual.has_simulation_effect_override());
        visual.set_simulation_effect_override(true);
        let blur: Arc<dyn IImmutableEffect> = Arc::new(ImmutableBlurEffect::new(6.0));
        SimulationVisualPatch::new().with_effect(blur).apply_to_composition_visual(&visual);
        assert!(visual.has_simulation_effect_override());
        assert!(visual.effect().is_some());

        visual.set_simulation_effect_override(false);
        SimulationVisualPatch::new().without_effect().apply_to_composition_visual(&visual);
        assert!(!visual.has_simulation_effect_override());
        assert!(visual.effect().is_none());
    }
}
