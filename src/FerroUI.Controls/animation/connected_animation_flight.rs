//! The composition engine of a connected animation: the port of WinUI's `CConnectedAnimation`
//! (`dxaml/xcp/components/animation/ConnectedAnimation.cpp` of microsoft-ui-xaml) and of the
//! effect property set of `GravityConnectedAnimationConfiguration`.
//!
//! A flight shows the source and the destination through brushes on sprite visuals in the
//! connected animation layer of the window (WinUI's `ConnectedAnimationRoot`), hides the two
//! elements while it runs, and animates the offset, scale, clip and cross-fade of the sprites on
//! the compositor. Its states and the order of its steps are those of `CConnectedAnimation`:
//! `prepare` takes the source, `try_start` names the destination, and the first frame after the
//! destination's layout (`update_pre_commit`, run from the service before the commit of a frame)
//! builds the destination brush and starts the animations in a scoped batch whose completion
//! ends the flight.

use super::{
    BasicConnectedAnimationConfiguration, ConnectedAnimationConfiguration, ConnectedAnimationService,
    DirectConnectedAnimationConfiguration, GravityConnectedAnimationConfiguration,
};
use crate::TopLevel;
use ferroui_base::animation::easings::Easing;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::{Color, Colors};
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::animations::{CompositionEasingFunction, ICompositionAnimation};
use ferroui_base::rendering::composition::{
    CompositionBatchTypes, CompositionClip, CompositionGetValueStatus,
    CompositionPropertySet, CompositionScopedBatch, CompositionVisual, Compositor, ICompositionObjectAnimations,
    ICompositionSurface, LoadedImageSurface,
};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{Rect, Ref, Vector, Vector3D, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// `s_timeout` of `ConnectedAnimation.cpp`: a prepared animation that is not started within
/// this time is cancelled.
const PREPARED_TIMEOUT: Duration = Duration::from_millis(2000);

/// The control points of the default easing of the service (`s_ControlPoint1/2`).
const DEFAULT_CONTROL_POINT_1: Vector2 = Vector2::new(0.8, 0.0);
const DEFAULT_CONTROL_POINT_2: Vector2 = Vector2::new(0.2, 1.0);

/// `DirectConnectedAnimationConfiguration`: 200 ms and a decelerating curve.
const DIRECT_DURATION: Duration = Duration::from_millis(200);
const DIRECT_CONTROL_POINT_1: Vector2 = Vector2::new(0.1, 0.9);
const DIRECT_CONTROL_POINT_2: Vector2 = Vector2::new(0.2, 1.0);

/// `GravityConnectedAnimationConfiguration`: the dip of the flight.
const GRAVITY_DIPPED_Z: f32 = 32.0;
const GRAVITY_DIPPED_Y: f32 = 80.0;
const GRAVITY_DIPPED_SCALE: f32 = 1.1;
const GRAVITY_PEAK_TIME: f32 = 0.66;
const GRAVITY_IN_CONTROL_POINT_1: Vector2 = Vector2::new(0.35, 0.0);
const GRAVITY_IN_CONTROL_POINT_2: Vector2 = Vector2::new(0.55, 1.0);
const GRAVITY_OUT_CONTROL_POINT_1: Vector2 = Vector2::new(0.45, 0.0);
const GRAVITY_OUT_CONTROL_POINT_2: Vector2 = Vector2::new(0.55, 1.0);

/// The parts of a connected animation that a custom animation can replace
/// (`Microsoft.UI.Xaml.Media.Animation.ConnectedAnimationComponent`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConnectedAnimationComponent {
    /// The horizontal offset of the flight.
    OffsetX = 0,
    /// The vertical offset of the flight.
    OffsetY = 1,
    /// The cross-fade from the source image to the destination (and the clip).
    CrossFade = 2,
    /// The scale of the flight.
    Scale = 3,
}

/// How a flight ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlightEnd {
    /// The animations ran to their end.
    Completed,
    /// The flight was cancelled after it started (the destination left the tree, ...).
    Cancelled,
    /// The flight was not started in time, or could not start.
    Expired,
}

/// `ConnectedAnimationState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlightState {
    /// Prepared: the source image is in the layer, the source hidden.
    Prepared,
    /// `try_start` was called; the animations start with the next frame.
    Started,
    /// The animations run.
    Running,
    Canceled,
    Complete,
}

/// `ConnectedAnimationElementInfo`: an element, its rectangles in the coordinates of the window,
/// and the sprite that shows it.
#[derive(Default)]
struct ElementInfo {
    element: Option<Ref<Visual>>,
    /// Whether the flight hides the element (a request of `add_composition_hide_request`).
    hidden: bool,
    clipped: Rect,
    unclipped: Rect,
    sprite: Option<Rc<CompositionVisual>>,
}

/// The connected animation layer of a window: a container visual above its content.
fn layer_of(top_level: &TopLevel, compositor: &Rc<Compositor>) -> Rc<CompositionVisual> {
    if let Some(layer) = top_level.composition_layer_visual() {
        if Rc::ptr_eq(layer.compositor(), compositor) {
            return layer;
        }
    }
    let layer = compositor.create_container_visual();
    layer.set_clip_to_bounds(false);
    top_level.set_composition_layer_visual(Some(layer.clone()));
    layer
}

/// The rectangles of `element` in the coordinates of `root`: unclipped (the bounds of the
/// element), and clipped by the ancestors that clip to their bounds
/// (`GetTightGlobalBounds` with and without clipping).
fn element_rects(element: &Visual, root: &Visual) -> Option<(Rect, Rect)> {
    let transform = element.transform_to_visual(root)?;
    let unclipped = Rect::from_size(element.bounds().size()).transform_to_aabb(transform);
    let mut clipped = unclipped;
    let mut current = element.visual_parent();
    while let Some(ancestor) = current {
        if std::ptr::eq(&*ancestor, root) {
            break;
        }
        if ancestor.clip_to_bounds() {
            if let Some(transform) = ancestor.transform_to_visual(root) {
                clipped = clipped.intersect(Rect::from_size(ancestor.bounds().size()).transform_to_aabb(transform));
            }
        }
        current = ancestor.visual_parent();
    }
    // The window clips everything.
    clipped = clipped.intersect(Rect::from_size(root.bounds().size()));
    Some((clipped, unclipped))
}

fn v3(x: f64, y: f64) -> Vector3D {
    Vector3D::new(x, y, 0.0)
}

/// The engine of one connected animation.
pub(crate) struct ConnectedAnimationFlight {
    this: Weak<ConnectedAnimationFlight>,
    service: Ref<ConnectedAnimationService>,
    compositor: Rc<Compositor>,
    top_level: WeakRef<TopLevel>,
    state: Cell<FlightState>,
    /// The animation this one coordinates with (`m_baseAnimation`), for an element of
    /// `try_start_with_coordinated_elements`.
    base: Option<Rc<ConnectedAnimationFlight>>,
    source: RefCell<ElementInfo>,
    destination: RefCell<ElementInfo>,
    /// The host visual: a container in the layer that holds the sprites (`m_hostVisual`).
    host: RefCell<Option<Rc<CompositionVisual>>>,
    inset_clip: RefCell<Option<Rc<CompositionClip>>>,
    /// The cross-fade weight (`Crossfade.Weight` of the cross-fade effect in WinUI).
    cross_fade: RefCell<Option<Rc<CompositionPropertySet>>>,
    effect_property_set: RefCell<Option<Rc<CompositionPropertySet>>>,
    primary_property_set: RefCell<Option<Rc<CompositionPropertySet>>>,
    batch: RefCell<Option<Rc<CompositionScopedBatch>>>,
    batch_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    timeout: RefCell<Option<Rc<dyn IDisposable>>>,
    is_scale_animation_enabled: Cell<bool>,
    components: RefCell<[Option<Rc<dyn ICompositionAnimation>>; 4]>,
    configuration: RefCell<Option<Rc<dyn ConnectedAnimationConfiguration>>>,
    /// Called once when the flight ends.
    on_finished: RefCell<Option<Box<dyn FnOnce(FlightEnd)>>>,
}

impl ConnectedAnimationFlight {
    /// `Prepare`: records the rectangles of `source`, renders it into the source image, puts the
    /// image in the layer where the source is and hides the source. `None` when the source is
    /// not in a window rendered by a compositor (the caller then uses the flight of the overlay
    /// layer).
    pub(crate) fn prepare(
        service: &Ref<ConnectedAnimationService>,
        source: &Visual,
        snapshot: Option<Rc<RenderTargetBitmap>>,
    ) -> Option<Rc<ConnectedAnimationFlight>> {
        let top_level = TopLevel::get_top_level(Some(source))?;
        let compositor = source.composition_visual()?.compositor().clone();
        let flight = Self::create(service, &top_level, &compositor, None);
        let (clipped, unclipped) = element_rects(source, &top_level)?;
        {
            let mut info = flight.source.borrow_mut();
            info.element = Some(source.to_ref());
            info.clipped = clipped;
            info.unclipped = unclipped;
        }

        // `CreateSnapshotBrush(m_source, false, true)`: the source image, frozen.
        if let Some(snapshot) = snapshot {
            if unclipped.width >= 0.5 && unclipped.height >= 0.5 {
                let surface = LoadedImageSurface::from_bitmap(&compositor, &snapshot);
                let brush = compositor.create_surface_brush_with_surface(surface as Rc<dyn ICompositionSurface>);
                let sprite = compositor.create_sprite_visual();
                sprite.set_brush(Some((*brush).clone()));
                sprite.set_size(Vector::new(unclipped.width, unclipped.height));
                flight.source.borrow_mut().sprite = Some(sprite.visual().clone());
                let host = flight.ensure_host();
                host.children().add(sprite.visual().clone());
                flight.update_sprite_opacities();
            }
        }
        flight.hide(&flight.source);
        flight.state.set(FlightState::Prepared);

        // `TimeoutIfNecessary`.
        let weak = flight.this.clone();
        let timeout = DispatcherTimer::run_once(
            move || {
                if let Some(flight) = weak.upgrade() {
                    if flight.state.get() == FlightState::Prepared {
                        flight.cancel();
                        flight.finish(FlightEnd::Expired);
                    }
                }
            },
            PREPARED_TIMEOUT,
            DispatcherPriority::BACKGROUND,
        );
        *flight.timeout.borrow_mut() = Some(timeout);
        Some(flight)
    }

    fn create(
        service: &Ref<ConnectedAnimationService>,
        top_level: &TopLevel,
        compositor: &Rc<Compositor>,
        base: Option<Rc<ConnectedAnimationFlight>>,
    ) -> Rc<ConnectedAnimationFlight> {
        Rc::new_cyclic(|this| ConnectedAnimationFlight {
            this: this.clone(),
            service: service.clone(),
            compositor: compositor.clone(),
            top_level: top_level.to_ref().downgrade(),
            state: Cell::new(FlightState::Prepared),
            base,
            source: RefCell::new(ElementInfo::default()),
            destination: RefCell::new(ElementInfo::default()),
            host: RefCell::new(None),
            inset_clip: RefCell::new(None),
            cross_fade: RefCell::new(None),
            effect_property_set: RefCell::new(None),
            primary_property_set: RefCell::new(None),
            batch: RefCell::new(None),
            batch_subscription: RefCell::new(None),
            timeout: RefCell::new(None),
            is_scale_animation_enabled: Cell::new(true),
            components: RefCell::new([None, None, None, None]),
            configuration: RefCell::new(None),
            on_finished: RefCell::new(None),
        })
    }

    pub(crate) fn set_on_finished(&self, on_finished: Box<dyn FnOnce(FlightEnd)>) {
        *self.on_finished.borrow_mut() = Some(on_finished);
    }

    pub(crate) fn is_scale_animation_enabled(&self) -> bool {
        self.is_scale_animation_enabled.get()
    }

    pub(crate) fn set_is_scale_animation_enabled(&self, value: bool) {
        self.is_scale_animation_enabled.set(value);
    }

    pub(crate) fn set_animation_component(
        &self,
        component: ConnectedAnimationComponent,
        animation: Option<Rc<dyn ICompositionAnimation>>,
    ) {
        self.components.borrow_mut()[component as usize] = animation;
    }

    /// `TryStart`: names the destination. The animations start with the next frame.
    pub(crate) fn try_start(
        &self,
        destination: &Visual,
        configuration: Option<Rc<dyn ConnectedAnimationConfiguration>>,
    ) -> bool {
        let is_source = self.source.borrow().element.as_ref().is_some_and(|source| std::ptr::eq(&**source, destination));
        if is_source {
            self.cancel();
            return false;
        }
        if self.state.get() != FlightState::Prepared {
            return false;
        }
        *self.configuration.borrow_mut() = configuration;
        self.destination.borrow_mut().element = Some(destination.to_ref());
        self.hide(&self.destination);
        self.state.set(FlightState::Started);
        if let Some(timeout) = self.timeout.borrow_mut().take() {
            timeout.dispose();
        }
        self.service.request_flight_tick(&self.compositor);
        true
    }

    /// `AnimateCoordinatedEntrance`: a flight of `element` that enters from where it would be
    /// relative to the source as it is relative to the destination. `None` when this flight
    /// cannot coordinate (it is not prepared or started).
    pub(crate) fn animate_coordinated_entrance(self: &Rc<Self>, element: &Visual) -> Option<Rc<ConnectedAnimationFlight>> {
        if !matches!(self.state.get(), FlightState::Prepared | FlightState::Started) {
            return None;
        }
        let top_level = self.top_level.upgrade()?;
        let coordinated = Self::create(&self.service, &top_level, &self.compositor, Some(self.clone()));
        *coordinated.configuration.borrow_mut() = self.configuration.borrow().clone();
        coordinated.destination.borrow_mut().element = Some(element.to_ref());
        coordinated.hide(&coordinated.destination);
        coordinated.state.set(FlightState::Started);
        self.service.request_flight_tick(&self.compositor);
        Some(coordinated)
    }

    /// `Cancel`: removes the sprites and shows the elements again.
    pub(crate) fn cancel(&self) {
        self.reset();
        self.state.set(FlightState::Canceled);
    }

    /// Cancels the flight and reports it: as cancelled when it had started, else as expired.
    fn cancel_and_notify(&self) {
        let was_running = matches!(self.state.get(), FlightState::Running | FlightState::Started);
        self.cancel();
        self.finish(if was_running { FlightEnd::Cancelled } else { FlightEnd::Expired });
    }

    fn finish(&self, end: FlightEnd) {
        self.service.remove_ended_flights();
        let on_finished = self.on_finished.borrow_mut().take();
        if let Some(on_finished) = on_finished {
            on_finished(end);
        }
    }

    /// Whether the flight was cancelled or completed.
    pub(crate) fn has_ended(&self) -> bool {
        matches!(self.state.get(), FlightState::Canceled | FlightState::Complete)
    }

    /// Whether the flight has started (`try_start` was called) and not ended.
    pub(crate) fn is_in_flight(&self) -> bool {
        matches!(self.state.get(), FlightState::Started | FlightState::Running)
    }

    fn hide(&self, info: &RefCell<ElementInfo>) {
        let mut info = info.borrow_mut();
        if info.hidden {
            return;
        }
        if let Some(element) = &info.element {
            element.add_composition_hide_request();
            info.hidden = true;
        }
    }

    /// `ClearElementInfo`.
    fn clear_element_info(info: &RefCell<ElementInfo>) {
        let mut info = info.borrow_mut();
        if info.hidden {
            if let Some(element) = &info.element {
                element.remove_composition_hide_request();
            }
        }
        *info = ElementInfo::default();
    }

    /// `Reset`.
    fn reset(&self) {
        Self::clear_element_info(&self.source);
        Self::clear_element_info(&self.destination);
        if let Some(host) = self.host.borrow_mut().take() {
            if let Some(layer) = host.parent() {
                layer.children().remove(&host);
            }
        }
        *self.inset_clip.borrow_mut() = None;
        *self.cross_fade.borrow_mut() = None;
        *self.effect_property_set.borrow_mut() = None;
        *self.primary_property_set.borrow_mut() = None;
        if let Some(subscription) = self.batch_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        *self.batch.borrow_mut() = None;
        if let Some(timeout) = self.timeout.borrow_mut().take() {
            timeout.dispose();
        }
    }

    /// The host visual, created in the layer of the window at the unclipped rectangle of the
    /// source (`CreateConnectedAnimationVisual` and the start of `CreateSnapshotBrush`).
    fn ensure_host(&self) -> Rc<CompositionVisual> {
        if let Some(host) = self.host.borrow().clone() {
            return host;
        }
        let host = self.compositor.create_container_visual();
        host.set_clip_to_bounds(false);
        let source = self.source.borrow().unclipped;
        host.set_offset(v3(source.x, source.y));
        host.set_size(Vector::new(source.width, source.height));
        if let Some(top_level) = self.top_level.upgrade() {
            layer_of(&top_level, &self.compositor).children().insert_at_top(host.clone());
        }
        // The clip is needed when the source is clipped; it starts at the source's insets.
        let (clipped, unclipped) = {
            let source = self.source.borrow();
            (source.clipped, source.unclipped)
        };
        if clipped != unclipped && unclipped.width > 0.0 {
            let clip = self.compositor.create_inset_clip_with_insets(
                (clipped.x - unclipped.x) as f32,
                (clipped.y - unclipped.y) as f32,
                (unclipped.right() - clipped.right()) as f32,
                (unclipped.bottom() - clipped.bottom()) as f32,
            );
            let clip: Rc<CompositionClip> = (*clip).clone();
            host.set_composition_clip(Some(clip.clone()));
            *self.inset_clip.borrow_mut() = Some(clip);
        }
        *self.host.borrow_mut() = Some(host.clone());
        host
    }

    /// Which sprites show: the opacities of the cross-fade (`CreateSnapshotBrush`, the end).
    fn update_sprite_opacities(&self) {
        let source = self.source.borrow().sprite.clone();
        let destination = self.destination.borrow().sprite.clone();
        if let (Some(source), Some(destination)) = (&source, &destination) {
            // Both: cross-fade by the weight.
            let cross_fade = self.cross_fade.borrow_mut().get_or_insert_with(|| {
                let set = CompositionPropertySet::new(&self.compositor);
                set.insert_scalar("Weight", 0.0);
                set
            }).clone();
            let fade_out = self.compositor.create_expression_animation_with("1 - crossfade.Weight");
            fade_out.set_reference_parameter("crossfade", cross_fade.clone());
            source.start_animation("Opacity", &*fade_out);
            let fade_in = self.compositor.create_expression_animation_with("crossfade.Weight");
            fade_in.set_reference_parameter("crossfade", cross_fade);
            destination.start_animation("Opacity", &*fade_in);
            return;
        }
        if self.state.get() == FlightState::Running {
            return;
        }
        if let Some(host) = self.host.borrow().clone() {
            // The source alone fades out; the destination alone fades in.
            host.set_opacity(if source.is_some() { 1.0 } else { 0.0 });
        }
    }

    /// `UpdatePreCommit`: called before the commit of each frame while the service has flights.
    /// Returns whether the flight is done and can be removed from the service.
    pub(crate) fn update_pre_commit(&self) -> bool {
        // A coordinated flight follows its base.
        if self.base.as_ref().is_some_and(|base| base.state.get() == FlightState::Canceled) {
            self.cancel();
        }
        match self.state.get() {
            FlightState::Started => {
                if self.base.as_ref().is_some_and(|base| base.state.get() != FlightState::Running) {
                    self.service.request_flight_tick(&self.compositor);
                    return false;
                }
            }
            FlightState::Prepared | FlightState::Running => return false,
            FlightState::Canceled | FlightState::Complete => return true,
        }

        let Some(top_level) = self.top_level.upgrade() else {
            self.cancel_and_notify();
            return true;
        };
        let destination = self.destination.borrow().element.clone();
        let Some(destination) = destination.filter(|destination| destination.is_attached_to_visual_tree()) else {
            self.cancel_and_notify();
            return true;
        };

        // `GetSnapshotTransformInfo(m_destination)`. A destination that is not laid out yet waits
        // for the next frame.
        let Some((clipped, unclipped)) = element_rects(&destination, &top_level) else {
            self.service.request_flight_tick(&self.compositor);
            return false;
        };
        {
            let mut info = self.destination.borrow_mut();
            info.clipped = clipped;
            info.unclipped = unclipped;
        }

        if let Some(base) = &self.base {
            let base_destination = base.destination.borrow().clipped;
            if base_destination.width == 0.0 || base_destination.height == 0.0 {
                self.cancel_and_notify();
                return true;
            }
            self.base_animation_source_info(base);
        }

        // The destination brush: a live image of the destination's visual.
        if self.destination.borrow().sprite.is_none() {
            self.create_destination_sprite(&destination);
        }

        if self.host.borrow().is_none() {
            // Neither image could be made: complete at once.
            self.state.set(FlightState::Complete);
            self.reset();
            self.finish(FlightEnd::Completed);
            return true;
        }

        if !self.is_scale_animation_enabled.get() {
            self.layout_clipped_sprites();
        }

        let batch = self.compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        self.start_sprite_animations();
        batch.end();
        let weak = self.this.clone();
        let subscription = batch.completed(move |_| {
            if let Some(flight) = weak.upgrade() {
                flight.try_to_complete();
            }
        });
        *self.batch.borrow_mut() = Some(batch);
        *self.batch_subscription.borrow_mut() = Some(subscription);
        self.state.set(FlightState::Running);
        false
    }

    fn create_destination_sprite(&self, destination: &Visual) {
        let Some(visual) = destination.composition_visual() else { return };
        let unclipped = self.destination.borrow().unclipped;
        if unclipped.width < 0.5 || unclipped.height < 0.5 {
            return;
        }
        let surface = self.compositor.create_visual_surface();
        surface.set_source_visual(Some((*visual).clone()));
        let brush = self.compositor.create_surface_brush_with_surface(surface as Rc<dyn ICompositionSurface>);
        let sprite = self.compositor.create_sprite_visual();
        sprite.set_brush(Some((*brush).clone()));
        let source = self.source.borrow().unclipped;
        sprite.set_size(Vector::new(source.width, source.height));
        self.destination.borrow_mut().sprite = Some(sprite.visual().clone());
        let host = self.ensure_host();
        host.children().add(sprite.visual().clone());
        self.update_sprite_opacities();
    }

    /// `GetBaseAnimationSourceInfo`: the source of a coordinated flight is where the element
    /// would be relative to the base's source as it is relative to the base's destination.
    fn base_animation_source_info(&self, base: &ConnectedAnimationFlight) {
        self.is_scale_animation_enabled.set(base.is_scale_animation_enabled.get());
        let scale_enabled = self.is_scale_animation_enabled.get();
        let (source_base, destination_base) = {
            let source = base.source.borrow();
            let destination = base.destination.borrow();
            if scale_enabled {
                (source.unclipped, destination.unclipped)
            } else {
                (source.clipped, destination.clipped)
            }
        };
        let destination = {
            let info = self.destination.borrow();
            if scale_enabled {
                info.unclipped
            } else {
                info.clipped
            }
        };
        let scale_x = if scale_enabled { source_base.width / destination_base.width } else { 1.0 };
        let scale_y = if scale_enabled { source_base.height / destination_base.height } else { 1.0 };
        let offset_x = destination.x - destination_base.x;
        let offset_y = destination.y - destination_base.y;
        let rect = Rect::new(
            source_base.x + offset_x * scale_x,
            source_base.y + offset_y * scale_y,
            destination.width * scale_x,
            destination.height * scale_y,
        );
        let mut source = self.source.borrow_mut();
        source.clipped = rect;
        source.unclipped = rect;
    }

    /// `ConvertSourceBrushToClipped` and `UpdateVisualRectForClippedBrushes`: without the scale
    /// animation the sprites show only the clipped parts of the elements, centred in a host of
    /// the larger of the two clipped sizes.
    fn layout_clipped_sprites(&self) {
        let (source_clipped, source_unclipped) = {
            let source = self.source.borrow();
            (source.clipped, source.unclipped)
        };
        let (destination_clipped, destination_unclipped) = {
            let destination = self.destination.borrow();
            (destination.clipped, destination.unclipped)
        };
        let width = source_clipped.width.max(destination_clipped.width);
        let height = source_clipped.height.max(destination_clipped.height);
        let Some(host) = self.host.borrow().clone() else { return };
        host.set_size(Vector::new(width, height));
        host.set_composition_clip(None);
        *self.inset_clip.borrow_mut() = None;
        let place = |sprite: &Rc<CompositionVisual>, clipped: Rect, unclipped: Rect| {
            // The sprite keeps the unclipped image and is moved so that its clipped part is
            // centred; a clip on the sprite cuts the rest.
            let left = (width - clipped.width) / 2.0 - (clipped.x - unclipped.x);
            let top = (height - clipped.height) / 2.0 - (clipped.y - unclipped.y);
            sprite.set_offset(v3(left, top));
            sprite.set_size(Vector::new(unclipped.width, unclipped.height));
            let clip = self.compositor.create_inset_clip_with_insets(
                (clipped.x - unclipped.x) as f32,
                (clipped.y - unclipped.y) as f32,
                (unclipped.right() - clipped.right()) as f32,
                (unclipped.bottom() - clipped.bottom()) as f32,
            );
            sprite.set_composition_clip(Some((*clip).clone()));
        };
        if let Some(sprite) = self.source.borrow().sprite.clone() {
            place(&sprite, source_clipped, source_unclipped);
        }
        if let Some(sprite) = self.destination.borrow().sprite.clone() {
            place(&sprite, destination_clipped, destination_unclipped);
        }
    }

    /// `ComputeAnimationOffset`.
    fn compute_animation_offset(&self, of_destination: bool) -> Vector3D {
        let (info, other) = {
            let source = self.source.borrow();
            let destination = self.destination.borrow();
            if of_destination {
                ((destination.clipped, destination.unclipped), (source.clipped, source.unclipped))
            } else {
                ((source.clipped, source.unclipped), (destination.clipped, destination.unclipped))
            }
        };
        if self.is_scale_animation_enabled.get() {
            return v3(info.1.x, info.1.y);
        }
        let mut x = info.0.x;
        let mut y = info.0.y;
        match &self.base {
            None => {
                if info.0.width < other.0.width {
                    x -= (other.0.width - info.0.width) / 2.0;
                }
                if info.0.height < other.0.height {
                    y -= (other.0.height - info.0.height) / 2.0;
                }
            }
            Some(base) => {
                if !of_destination {
                    let base_source = base.source.borrow().clipped;
                    let base_destination = base.destination.borrow().clipped;
                    x += (base_source.width - base_destination.width) / 2.0;
                    y += (base_source.height - base_destination.height) / 2.0;
                }
            }
        }
        v3(x, y)
    }

    /// The duration and easing of the flight (`GetEffectiveDuration`,
    /// `GetEffectiveEasingFunction`), and whether the configuration is the gravity one with or
    /// without a shadow.
    fn timing(&self) -> (Duration, Easing, Option<bool>) {
        let configuration = self.configuration.borrow().clone();
        let configuration = configuration.as_ref().map(|configuration| configuration.as_any());
        let service_easing = || {
            self.service.default_easing_function().unwrap_or_else(|| {
                CompositionEasingFunction::create_cubic_bezier_easing_function(
                    &self.compositor,
                    DEFAULT_CONTROL_POINT_1,
                    DEFAULT_CONTROL_POINT_2,
                )
            })
        };
        let service_duration = Duration::from_secs_f64(self.service.default_duration().total_seconds().max(0.001));
        if let Some(direct) =
            configuration.and_then(|configuration| configuration.downcast_ref::<DirectConnectedAnimationConfiguration>())
        {
            let duration = direct
                .duration()
                .map(|duration| Duration::from_secs_f64(duration.total_seconds().max(0.001)))
                .unwrap_or(DIRECT_DURATION);
            let easing = CompositionEasingFunction::create_cubic_bezier_easing_function(
                &self.compositor,
                DIRECT_CONTROL_POINT_1,
                DIRECT_CONTROL_POINT_2,
            );
            return (duration, easing, None);
        }
        if configuration.is_some_and(|configuration| configuration.is::<BasicConnectedAnimationConfiguration>()) {
            return (service_duration, service_easing(), None);
        }
        // The default configuration is the gravity one (`ConnectedAnimationConfiguration_GetDefault`).
        let shadow = configuration
            .and_then(|configuration| configuration.downcast_ref::<GravityConnectedAnimationConfiguration>())
            .map_or(true, |gravity| gravity.is_shadow_enabled());
        (service_duration, service_easing(), Some(shadow))
    }

    /// `StartSpriteAnimations`.
    fn start_sprite_animations(&self) {
        let (duration, easing, gravity) = self.timing();
        let Some(host) = self.host.borrow().clone() else { return };

        let source_offset = self.compute_animation_offset(false);
        let destination_offset = self.compute_animation_offset(true);
        let (source_unclipped, destination_unclipped) =
            (self.source.borrow().unclipped, self.destination.borrow().unclipped);
        let mut scale = Vector3D::new(1.0, 1.0, 1.0);
        if self.is_scale_animation_enabled.get() && source_unclipped.width > 0.0 && source_unclipped.height > 0.0 {
            scale = Vector3D::new(
                destination_unclipped.width / source_unclipped.width,
                destination_unclipped.height / source_unclipped.height,
                1.0,
            );
        }

        // The configuration may add an effect: the gravity dip. Its property set is combined
        // with a primary property set by expressions on the host, and the primary set is
        // animated instead of the host.
        let mut offset_on_primary = false;
        let mut scale_on_primary = false;
        if let Some(shadow) = gravity {
            let effect = self.gravity_effect_property_set(duration, scale);
            let primary = CompositionPropertySet::new(&self.compositor);
            primary.insert_vector3("Offset", source_offset.into());
            let offset = self.compositor.create_expression_animation_with("PrimaryPS.Offset + EffectPS.Offset");
            offset.set_reference_parameter("PrimaryPS", primary.clone());
            offset.set_reference_parameter("EffectPS", effect.clone());
            host.start_animation("Offset", &*offset);
            offset_on_primary = true;
            if effect.try_get_vector3("Scale").0 == CompositionGetValueStatus::Succeeded {
                primary.insert_vector3("Scale", Vector3::new(1.0, 1.0, 1.0));
                let scale_expression = self.compositor.create_expression_animation_with("PrimaryPS.Scale * EffectPS.Scale");
                scale_expression.set_reference_parameter("PrimaryPS", primary.clone());
                scale_expression.set_reference_parameter("EffectPS", effect.clone());
                host.start_animation("Scale", &*scale_expression);
                scale_on_primary = true;
            }
            // Deviation (winui-composition.md): WinUI casts the shadow of a coordinated element
            // too, shaped by the alpha of its image; the shadow here is the rectangle of the
            // host, which a text element would show as a band, so coordinated flights have none.
            if shadow && self.base.is_none() {
                self.add_shadow(&host, &effect);
            }
            *self.effect_property_set.borrow_mut() = Some(effect);
            *self.primary_property_set.borrow_mut() = Some(primary);
        }
        let primary = self.primary_property_set.borrow().clone();

        // The offset.
        let (offset_x, offset_y) = {
            let components = self.components.borrow();
            (components[0].clone(), components[1].clone())
        };
        let offset_target: &dyn ICompositionObjectAnimations = match (&primary, offset_on_primary) {
            (Some(primary), true) => &**primary,
            _ => &*host,
        };
        if offset_x.is_none() && offset_y.is_none() {
            self.start_vector3_animation(offset_target, "Offset", source_offset, destination_offset, duration, &easing, offset_on_primary);
        } else {
            self.start_custom_animation(offset_target, "Offset.X", ConnectedAnimationComponent::OffsetX, source_offset.x, destination_offset.x, duration, &easing);
            self.start_custom_animation(offset_target, "Offset.Y", ConnectedAnimationComponent::OffsetY, source_offset.y, destination_offset.y, duration, &easing);
        }

        // The scale.
        if scale.x != 1.0 || scale.y != 1.0 {
            let scale_target: &dyn ICompositionObjectAnimations = match (&primary, scale_on_primary) {
                (Some(primary), true) => &**primary,
                _ => &*host,
            };
            if self.components.borrow()[ConnectedAnimationComponent::Scale as usize].is_none() {
                self.start_vector3_animation(scale_target, "Scale", Vector3D::new(1.0, 1.0, 1.0), scale, duration, &easing, scale_on_primary);
            } else {
                self.start_custom_animation(scale_target, "Scale.X", ConnectedAnimationComponent::Scale, 1.0, scale.x, duration, &easing);
                self.start_custom_animation(scale_target, "Scale.Y", ConnectedAnimationComponent::Scale, 1.0, scale.y, duration, &easing);
            }
        }

        // The clip, from the insets of the source to those of the destination at its scale.
        if let Some(clip) = self.inset_clip.borrow().clone() {
            let (source_clipped, destination_clipped) =
                (self.source.borrow().clipped, self.destination.borrow().clipped);
            let from = [
                source_clipped.x - source_unclipped.x,
                source_clipped.y - source_unclipped.y,
                source_unclipped.right() - source_clipped.right(),
                source_unclipped.bottom() - source_clipped.bottom(),
            ];
            let to = [
                (destination_clipped.x - destination_unclipped.x) / scale.x,
                (destination_clipped.y - destination_unclipped.y) / scale.y,
                (destination_unclipped.right() - destination_clipped.right()) / scale.x,
                (destination_unclipped.bottom() - destination_clipped.bottom()) / scale.y,
            ];
            for (index, name) in ["LeftInset", "TopInset", "RightInset", "BottomInset"].into_iter().enumerate() {
                self.start_custom_animation(&*clip, name, ConnectedAnimationComponent::CrossFade, from[index], to[index], duration, &easing);
            }
        }

        // The cross-fade: the weight when both images show, else the opacity of the host.
        if let Some(cross_fade) = self.cross_fade.borrow().clone() {
            self.start_custom_animation(&*cross_fade, "Weight", ConnectedAnimationComponent::CrossFade, 0.0, 1.0, duration, &easing);
        } else {
            let start = host.opacity() as f64;
            let end = if start == 0.0 { 1.0 } else { 0.0 };
            self.start_custom_animation(&*host, "Opacity", ConnectedAnimationComponent::CrossFade, start, end, duration, &easing);
        }
    }

    /// `GravityConnectedAnimationConfiguration::GetEffectPropertySet`.
    fn gravity_effect_property_set(&self, duration: Duration, scale_factors: Vector3D) -> Rc<CompositionPropertySet> {
        let compositor = &self.compositor;
        let set = CompositionPropertySet::new(compositor);
        let ease_in = CompositionEasingFunction::create_cubic_bezier_easing_function(
            compositor,
            GRAVITY_IN_CONTROL_POINT_1,
            GRAVITY_IN_CONTROL_POINT_2,
        );
        let ease_out = CompositionEasingFunction::create_cubic_bezier_easing_function(
            compositor,
            GRAVITY_OUT_CONTROL_POINT_1,
            GRAVITY_OUT_CONTROL_POINT_2,
        );

        let natural_offset = Vector3::new(0.0, 0.0, 0.0);
        let dipped_offset = Vector3::new(0.0, GRAVITY_DIPPED_Y, GRAVITY_DIPPED_Z);
        set.insert_vector3("Offset", natural_offset);
        let offset = compositor.create_vector3_key_frame_animation();
        offset.set_duration(duration);
        offset.insert_key_frame(0.0, natural_offset);
        offset.insert_key_frame_with_easing(GRAVITY_PEAK_TIME, dipped_offset, ease_in.as_rc().clone());
        offset.insert_key_frame_with_easing(1.0, natural_offset, ease_out.as_rc().clone());
        set.start_animation("Offset", &*offset);

        // The scale dips too, unless the flight scales already.
        if scale_factors.x == 1.0 && scale_factors.y == 1.0 {
            let natural_scale = Vector3::new(1.0, 1.0, 1.0);
            let dipped_scale = Vector3::new(GRAVITY_DIPPED_SCALE, GRAVITY_DIPPED_SCALE, 1.0);
            set.insert_vector3("Scale", natural_scale);
            let scale = compositor.create_vector3_key_frame_animation();
            scale.set_duration(duration);
            scale.insert_key_frame(0.0, natural_scale);
            scale.insert_key_frame_with_easing(GRAVITY_PEAK_TIME, dipped_scale, ease_in.as_rc().clone());
            scale.insert_key_frame_with_easing(1.0, natural_scale, ease_out.as_rc().clone());
            set.start_animation("Scale", &*scale);
        }
        set
    }

    /// The shadow of the gravity configuration. WinUI makes the host a caster of a
    /// `ThemeShadow`, whose depth follows the Z of the dip; here a drop shadow under the images
    /// grows and fades with that Z.
    fn add_shadow(&self, host: &Rc<CompositionVisual>, effect: &Rc<CompositionPropertySet>) {
        let compositor = &self.compositor;
        let size = host.size();
        let shadow = compositor.create_drop_shadow();
        shadow.set_color(Colors::BLACK);
        shadow.set_opacity(0.0);
        let sprite = compositor.create_sprite_visual();
        sprite.set_size(size);
        sprite.set_brush(Some((*compositor.create_color_brush_with_color(Color::from_argb(0, 0, 0, 0))).clone()));
        sprite.set_shadow(Some(shadow.clone()));
        host.children().insert_at_bottom(sprite.visual().clone());

        let blur = compositor.create_expression_animation_with("effect.Offset.Z * 0.75");
        blur.set_reference_parameter("effect", effect.clone());
        shadow.start_animation("BlurRadius", &*blur);
        let opacity = compositor.create_expression_animation_with("effect.Offset.Z / 32 * 0.4");
        opacity.set_reference_parameter("effect", effect.clone());
        shadow.start_animation("Opacity", &*opacity);
        let offset = compositor.create_expression_animation_with("Vector3(0, effect.Offset.Z * 0.25, 0)");
        offset.set_reference_parameter("effect", effect.clone());
        shadow.start_animation("Offset", &*offset);
    }

    /// `StartCustomAnimation`: the custom animation of `component` with `StartingValue` and
    /// `FinalValue` parameters, or a key frame animation from `from` to `to`.
    #[allow(clippy::too_many_arguments)]
    fn start_custom_animation(
        &self,
        target: &dyn ICompositionObjectAnimations,
        property: &str,
        component: ConnectedAnimationComponent,
        from: f64,
        to: f64,
        duration: Duration,
        easing: &Easing,
    ) {
        let custom = self.components.borrow()[component as usize].clone();
        let Some(custom) = custom else {
            self.start_scalar_animation(target, property, from as f32, to as f32, duration, easing);
            return;
        };
        if let Some(animation) = custom.composition_animation() {
            animation.set_scalar_parameter("StartingValue", from as f32);
            animation.set_scalar_parameter("FinalValue", to as f32);
        }
        target.start_animation(property, &*custom);
    }

    /// `StartScalarAnimation`.
    fn start_scalar_animation(
        &self,
        target: &dyn ICompositionObjectAnimations,
        property: &str,
        from: f32,
        to: f32,
        duration: Duration,
        easing: &Easing,
    ) {
        if from == to {
            return;
        }
        let animation = self.compositor.create_scalar_key_frame_animation();
        animation.set_duration(duration);
        animation.insert_key_frame(0.0, from);
        animation.insert_key_frame_with_easing(1.0, to, easing.as_rc().clone());
        target.start_animation(property, &*animation);
    }

    /// `StartVector3Animation`. The entries of a property set are `Vector3` (the type of WinUI),
    /// the properties of a visual `Vector3D`.
    #[allow(clippy::too_many_arguments)]
    fn start_vector3_animation(
        &self,
        target: &dyn ICompositionObjectAnimations,
        property: &str,
        from: Vector3D,
        to: Vector3D,
        duration: Duration,
        easing: &Easing,
        on_property_set: bool,
    ) {
        if from.x == to.x && from.y == to.y {
            return;
        }
        if on_property_set {
            let animation = self.compositor.create_vector3_key_frame_animation();
            animation.set_duration(duration);
            animation.insert_key_frame(0.0, from.into());
            animation.insert_key_frame_with_easing(1.0, to.into(), easing.as_rc().clone());
            target.start_animation(property, &*animation);
            return;
        }
        let animation = self.compositor.create_vector3d_key_frame_animation();
        animation.set_duration(duration);
        animation.insert_key_frame(0.0, from);
        animation.insert_key_frame_with_easing(1.0, to, easing.as_rc().clone());
        target.start_animation(property, &*animation);
    }

    /// `OnAnimationCompleted` and `TryToCompleteAnimation`.
    fn try_to_complete(&self) {
        if self.state.get() != FlightState::Running {
            return;
        }
        self.state.set(FlightState::Complete);
        self.reset();
        self.finish(FlightEnd::Completed);
    }
}
