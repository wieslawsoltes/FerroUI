use super::{
    BasicConnectedAnimationConfiguration, ConnectedAnimationConfiguration, ConnectedAnimationService,
    DirectConnectedAnimationConfiguration, GravityConnectedAnimationConfiguration,
};
use crate::presenters::ContentPresenter;
use crate::primitives::{OverlayLayer, TemplatedControl};
use crate::{Border, Canvas, ControlImpl, Panel, TopLevel};
use ferroui_base::animation::easings::{Easing, LinearEasing, SplineEasing};
use ferroui_base::animation::{Animation, Cue, FillMode, KeyFrame, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::{
    BoxShadow, BoxShadows, Color, IBrush, IImageBrushSource, ImageBrush, ScaleTransform, SolidColorBrush,
    Stretch, TransformGroup, TranslateTransform,
};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::Setter;
use ferroui_base::threading::{CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, CornerRadius, FerroLocator,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, LocatorExtensions, PixelSize, Point, Rect, Ref, RelativePoint, RelativeUnit, Size,
    StyledElementImpl, StyledProperty, Thickness, Vector, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// Provides data for the [`ConnectedAnimation::completed`] event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectedAnimationCompletedEventArgs {
    cancelled: bool,
}

impl ConnectedAnimationCompletedEventArgs {
    pub(crate) fn new(cancelled: bool) -> Self {
        Self { cancelled }
    }

    /// Gets a value indicating whether the animation was cancelled before it
    /// completed. When `true` the destination element's opacity has already
    /// been restored but no visual transition was shown.
    pub fn cancelled(&self) -> bool {
        self.cancelled
    }
}

type CompletedHandler = dyn Fn(&ConnectedAnimationCompletedEventArgs);

const COORDINATED_FADE_START_THRESHOLD: f64 = 0.6;
const COORDINATED_FADE_RANGE: f64 = 0.4;

thread_local! {
    static DIRECT_EASING: Easing = Easing::new(SplineEasing::with_points(0.0, 0.0, 0.58, 1.0));
    static BASIC_EASING: Easing = Easing::new(SplineEasing::with_points(0.42, 0.0, 0.58, 1.0));
    static GRAVITY_EASING: Easing = Easing::new(SplineEasing::with_points(0.1, 0.9, 0.2, 1.0));
}

/// The timing and style an animation runs with
/// ([`ConnectedAnimation::resolve_timing_and_easing`]).
#[derive(Clone)]
pub(crate) struct ResolvedTiming {
    pub(crate) duration: TimeSpan,
    pub(crate) easing: Easing,
    pub(crate) use_gravity_dip: bool,
    pub(crate) use_shadow: bool,
}

/// Animates an element seamlessly between two views during navigation by
/// flying a proxy over the [`OverlayLayer`].
///
/// Obtain an instance via
/// [`ConnectedAnimationService::prepare_to_animate`], then start it with
/// [`try_start`](Self::try_start) after navigation.
///
/// The animation auto-disposes after three seconds if not consumed
/// (matching UWP behaviour).
pub struct ConnectedAnimation {
    this: Weak<ConnectedAnimation>,
    key: String,
    service: Ref<ConnectedAnimationService>,

    source_bounds: Cell<Rect>,
    source_corner_radius: CornerRadius,
    source_background: Option<Rc<dyn IBrush>>,
    source_border_thickness: Thickness,
    source_border_brush: Option<Rc<dyn IBrush>>,
    source_snapshot: RefCell<Option<Rc<RenderTargetBitmap>>>,

    is_consumed: Cell<bool>,
    disposed: Cell<bool>,

    timeout_cts: RefCell<Option<CancellationTokenSource>>,
    timeout_timer_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    animation_cts: RefCell<Option<CancellationTokenSource>>,
    animation_timer: RefCell<Option<Rc<DispatcherTimer>>>,

    // Active-flight state used by `dispose` to clean up if cancelled
    // mid-animation.
    active_destination: RefCell<Option<Ref<Visual>>>,
    active_dest_original_opacity: Cell<f64>,
    active_proxy: RefCell<Option<Ref<ConnectedAnimationProxy>>>,
    active_overlay_layer: RefCell<Option<Ref<OverlayLayer>>>,

    configuration: RefCell<Option<Rc<dyn ConnectedAnimationConfiguration>>>,
    completed: HandlerList<CompletedHandler>,
}

/// Animations compare by reference.
impl PartialEq for ConnectedAnimation {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ConnectedAnimation {
    pub(crate) fn new(key: &str, source: &Visual, service: &Ref<ConnectedAnimationService>) -> Rc<Self> {
        let animation = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            key: key.to_owned(),
            service: service.clone(),
            source_bounds: Cell::new(Rect::default()),
            source_corner_radius: Self::get_corner_radius(source),
            source_background: Self::get_background(source),
            source_border_thickness: Self::get_border_thickness(source),
            source_border_brush: Self::get_border_brush(source),
            source_snapshot: RefCell::new(None),
            is_consumed: Cell::new(false),
            disposed: Cell::new(false),
            timeout_cts: RefCell::new(None),
            timeout_timer_disposable: RefCell::new(None),
            animation_cts: RefCell::new(None),
            animation_timer: RefCell::new(None),
            active_destination: RefCell::new(None),
            active_dest_original_opacity: Cell::new(0.0),
            active_proxy: RefCell::new(None),
            active_overlay_layer: RefCell::new(None),
            configuration: RefCell::new(None),
            completed: HandlerList::new(),
        });

        let top_level = source.find_ancestor_of_type::<TopLevel>(false);
        if let Some(top_level) = top_level {
            if source.bounds().width > 0.0 && source.bounds().height > 0.0 {
                if let Some(transform) = source.transform_to_visual(&top_level) {
                    animation.source_bounds.set(Rect::from_position_size(
                        transform.transform(Point::new(0.0, 0.0)),
                        Size::new(source.bounds().width, source.bounds().height),
                    ));
                }

                animation.capture_snapshot(source, &top_level);
            }
        }

        // Auto-dispose after 3 s if not consumed (matches UWP behaviour).
        let timeout_cts = CancellationTokenSource::new();
        let token = timeout_cts.token();
        *animation.timeout_cts.borrow_mut() = Some(timeout_cts);
        let this = animation.clone();
        let timeout_timer_disposable = DispatcherTimer::run_once(
            move || {
                if !token.is_cancellation_requested() && !this.is_consumed.get() {
                    this.dispose();
                }
            },
            Duration::from_secs(3),
            DispatcherPriority::BACKGROUND,
        );
        *animation.timeout_timer_disposable.borrow_mut() = Some(timeout_timer_disposable);

        animation
    }

    /// Gets the key that identifies this animation.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Gets a value indicating whether [`try_start`](Self::try_start) has
    /// been called.
    pub fn is_consumed(&self) -> bool {
        self.is_consumed.get()
    }

    /// Gets the configuration that controls timing and visual style.
    pub fn configuration(&self) -> Option<Rc<dyn ConnectedAnimationConfiguration>> {
        self.configuration.borrow().clone()
    }

    /// Sets the configuration that controls timing and visual style. Set
    /// this before calling [`try_start`](Self::try_start).
    pub fn set_configuration(&self, value: Option<Rc<dyn ConnectedAnimationConfiguration>>) {
        *self.configuration.borrow_mut() = value;
    }

    /// Raised when the animation finishes or is cancelled. Check
    /// [`ConnectedAnimationCompletedEventArgs::cancelled`] to distinguish the
    /// cases. Disposing the returned handle unsubscribes.
    pub fn completed(
        &self,
        handler: impl Fn(&ConnectedAnimationCompletedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let id = self.completed.add(Rc::new(handler));
        let this = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.completed.remove(id);
            }
        })
    }

    fn raise_completed(&self, cancelled: bool) {
        let e = ConnectedAnimationCompletedEventArgs::new(cancelled);
        for (_, handler) in self.completed.snapshot().iter() {
            handler(&e);
        }
    }

    /// Starts the animation towards `destination`. Returns `false` if the
    /// animation has already been consumed or disposed.
    pub fn try_start(&self, destination: &Visual) -> bool {
        self.try_start_with_coordinated_elements(destination, &[])
    }

    /// Starts the animation towards `destination` with
    /// `coordinated_elements` that fade in during the last 40 % of the
    /// animation. Returns `false` if the animation has already been consumed
    /// or disposed.
    pub fn try_start_with_coordinated_elements(&self, destination: &Visual, coordinated_elements: &[Ref<Visual>]) -> bool {
        if self.is_consumed.get() || self.disposed.get() {
            return false;
        }

        self.is_consumed.set(true);
        self.cancel_timeout();

        let this = self.this.upgrade().expect("the animation is alive while it is used");
        let _ = crate::page::start_async(this.run_animation_async(destination.to_ref(), coordinated_elements.to_vec()));
        true
    }

    // Exposed to the crate so tests can verify the disposal state.
    pub(crate) fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    /// Releases all resources and cancels the animation if it is in flight.
    /// The [`completed`](Self::completed) event is raised with `cancelled`
    /// set only when the animation was actively running at dispose time.
    pub fn dispose(&self) {
        if self.disposed.get() {
            return;
        }
        self.disposed.set(true);

        self.cancel_timeout();
        self.service.remove_animation(&self.key);
        let animation_cts = self.animation_cts.borrow_mut().take();
        if let Some(animation_cts) = animation_cts {
            animation_cts.cancel();
        }
        let animation_timer = self.animation_timer.borrow_mut().take();
        if let Some(animation_timer) = animation_timer {
            animation_timer.stop();
        }

        let active_destination = self.active_destination.borrow_mut().take();
        let was_mid_flight = active_destination.is_some();

        if let Some(active_destination) = active_destination {
            active_destination.set_opacity(self.active_dest_original_opacity.get());
        }

        let active_proxy = self.active_proxy.borrow().clone();
        let active_overlay_layer = self.active_overlay_layer.borrow().clone();
        if let (Some(active_proxy), Some(active_overlay_layer)) = (active_proxy, active_overlay_layer) {
            active_overlay_layer.children().remove(&active_proxy);
            *self.active_proxy.borrow_mut() = None;
            *self.active_overlay_layer.borrow_mut() = None;
        }

        let source_snapshot = self.source_snapshot.borrow_mut().take();
        if let Some(source_snapshot) = source_snapshot {
            source_snapshot.dispose();
        }

        if was_mid_flight {
            self.raise_completed(true);
        }
    }

    fn capture_snapshot(&self, source: &Visual, top_level: &TopLevel) {
        // The reference catches what creating and rendering the bitmap
        // throws; the failure it can meet without a defect is a missing
        // render interface, which is checked here.
        if FerroLocator::current().get_service::<dyn IPlatformRenderInterface>().is_none() {
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
                let exception = "Unable to locate the platform render interface.";
                logger.log_with_values(
                    None,
                    "ConnectedAnimation snapshot failed for key '{Key}': {Exception}",
                    &[&self.key, &exception],
                );
            }
            return;
        }

        let dpi = top_level.render_scaling();
        let w = (source.bounds().width * dpi).ceil() as i32;
        let h = (source.bounds().height * dpi).ceil() as i32;
        if w > 0 && h > 0 {
            let snapshot = Rc::new(RenderTargetBitmap::with_dpi(PixelSize::new(w, h), Vector::new(96.0 * dpi, 96.0 * dpi)));
            snapshot.render(&source.to_ref());
            *self.source_snapshot.borrow_mut() = Some(snapshot);
        }
    }

    fn cancel_timeout(&self) {
        let timeout_timer_disposable = self.timeout_timer_disposable.borrow_mut().take();
        if let Some(timeout_timer_disposable) = timeout_timer_disposable {
            timeout_timer_disposable.dispose();
        }
        let timeout_cts = self.timeout_cts.borrow_mut().take();
        if let Some(timeout_cts) = timeout_cts {
            timeout_cts.cancel();
        }
    }

    async fn run_animation_async(self: Rc<Self>, destination: Ref<Visual>, coordinated_elements: Vec<Ref<Visual>>) {
        // Cancellation completes the awaited operations normally, and the
        // cleanup is done by `dispose`; a failed animation is reported here.
        if let Err(exception) = self.clone().run_animation_core_async(destination, coordinated_elements).await {
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
                logger.log_with_values(
                    None,
                    "ConnectedAnimation failed for key '{Key}': {Exception}",
                    &[&self.key, &exception],
                );
            }
            self.dispose();
        }
    }

    async fn run_animation_core_async(
        self: Rc<Self>,
        destination: Ref<Visual>,
        coordinated_elements: Vec<Ref<Visual>>,
    ) -> Result<(), String> {
        let ResolvedTiming { duration, easing, use_gravity_dip, use_shadow } =
            self.resolve_timing_and_easing(&self.service);

        let Some(top_level) = destination.find_ancestor_of_type::<TopLevel>(false) else {
            self.on_animation_complete();
            return Ok(());
        };

        let Some(overlay_layer) = OverlayLayer::get_overlay_layer(&top_level) else {
            self.run_fallback_animation_async(
                &destination,
                &coordinated_elements,
                &top_level,
                duration,
                easing,
                use_gravity_dip,
                use_shadow,
            )
            .await;
            return Ok(());
        };

        // Wait for destination layout if bounds are not yet valid.
        if destination.bounds().width <= 0.0
            || destination.bounds().height <= 0.0
            || destination.transform_to_visual(&top_level).is_none()
        {
            if let Some(layoutable) = destination.to_ref().cast::<Layoutable>() {
                let layout_tcs = Completion::new();
                let handler: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));

                let subscription = layoutable.layout_updated({
                    let destination = destination.clone();
                    let top_level = top_level.clone();
                    let handler = handler.clone();
                    let layout_tcs = layout_tcs.clone();
                    move || {
                        if destination.bounds().width > 0.0
                            && destination.bounds().height > 0.0
                            && destination.transform_to_visual(&top_level).is_some()
                        {
                            let subscription = handler.borrow_mut().take();
                            if let Some(subscription) = subscription {
                                subscription.dispose();
                            }
                            layout_tcs.try_set(CompletionOutcome::Result(true));
                        }
                    }
                });
                *handler.borrow_mut() = Some(subscription);

                // The 500 ms timeout of the reference.
                let timeout = DispatcherTimer::run_once(
                    {
                        let handler = handler.clone();
                        let layout_tcs = layout_tcs.clone();
                        move || {
                            let subscription = handler.borrow_mut().take();
                            if let Some(subscription) = subscription {
                                subscription.dispose();
                            }
                            layout_tcs.try_set(CompletionOutcome::Result(false));
                        }
                    },
                    Duration::from_millis(500),
                    DispatcherPriority::NORMAL,
                );

                layout_tcs.wait().await;
                timeout.dispose();
            }
        }

        let Some(dest_transform) = destination.transform_to_visual(&top_level) else {
            self.on_animation_complete();
            return Ok(());
        };

        let dest_bounds = Rect::from_position_size(
            dest_transform.transform(Point::new(0.0, 0.0)),
            Size::new(destination.bounds().width, destination.bounds().height),
        );

        let dest_corner_radius = Self::get_corner_radius(&destination);
        let dest_border_thickness = Self::get_border_thickness(&destination);
        let dest_border_brush = Self::get_border_brush(&destination);

        let source_bounds = self.source_bounds.get();
        let proxy = ConnectedAnimationProxy::new();
        proxy.set_width(source_bounds.width);
        proxy.set_height(source_bounds.height);
        proxy.set_corner_radius(self.source_corner_radius);
        proxy.set_border_thickness(self.source_border_thickness);
        proxy.set_border_brush(self.source_border_brush.clone());
        proxy.set_clip_to_bounds(true);
        proxy.set_is_hit_test_visible(false);

        let source_snapshot = self.source_snapshot.borrow().clone();
        if let Some(source_background) = &self.source_background {
            proxy.set_background(Some(source_background.clone()));
        } else if let Some(source_snapshot) = source_snapshot {
            let brush = ImageBrush::with_source(Some(source_snapshot as Rc<dyn IImageBrushSource>));
            brush.set_stretch(Stretch::Fill);
            proxy.set_background(Some(brush.into()));
        }

        Canvas::set_left(&proxy, source_bounds.x);
        Canvas::set_top(&proxy, source_bounds.y);

        let dest_original_opacity = destination.opacity();
        destination.set_opacity(0.0);

        *self.active_destination.borrow_mut() = Some(destination.clone());
        self.active_dest_original_opacity.set(dest_original_opacity);
        *self.active_proxy.borrow_mut() = Some(proxy.clone());
        *self.active_overlay_layer.borrow_mut() = Some(overlay_layer.clone());

        let mut original_opacities = vec![0.0; coordinated_elements.len()];
        for (i, element) in coordinated_elements.iter().enumerate() {
            if element == &destination {
                continue;
            }
            original_opacities[i] = element.opacity();
            element.set_opacity(0.0);
        }

        let dest_background = Self::get_background(&destination);
        let needs_cross_fade = match (&dest_background, &self.source_background) {
            (Some(dest_background), Some(source_background)) => !Self::brushes_equal(source_background, dest_background),
            _ => false,
        };

        overlay_layer.children().add(proxy.clone());

        let mut cross_fade_overlay: Option<Ref<Border>> = None;
        if needs_cross_fade {
            let overlay = Border::new();
            overlay.set_background(dest_background.clone());
            overlay.set_opacity(0.0);
            overlay.set_is_hit_test_visible(false);
            proxy.set_child(Some(overlay.clone().upcast()));
            cross_fade_overlay = Some(overlay);
        }

        let (start_x, end_x) = (source_bounds.x, dest_bounds.x);
        let (start_y, end_y) = (source_bounds.y, dest_bounds.y);
        let (start_w, end_w) = (source_bounds.width, dest_bounds.width);
        let (start_h, end_h) = (source_bounds.height, dest_bounds.height);

        let src_tl = self.source_corner_radius.top_left;
        let src_tr = self.source_corner_radius.top_right;
        let src_br = self.source_corner_radius.bottom_right;
        let src_bl = self.source_corner_radius.bottom_left;
        let dst_tl = dest_corner_radius.top_left;
        let dst_tr = dest_corner_radius.top_right;
        let dst_br = dest_corner_radius.bottom_right;
        let dst_bl = dest_corner_radius.bottom_left;

        let src_bt = self.source_border_thickness;
        let dst_bt = dest_border_thickness;

        let solid_color = |brush: &Option<Rc<dyn IBrush>>| {
            brush.as_ref().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color()))
        };
        let can_lerp_border_brush =
            solid_color(&self.source_border_brush).is_some() && solid_color(&dest_border_brush).is_some();
        let src_bc = solid_color(&self.source_border_brush).unwrap_or_default();
        let dst_bc = solid_color(&dest_border_brush).unwrap_or_default();
        let lerp_brush = can_lerp_border_brush.then(|| SolidColorBrush::with_color(src_bc));
        let snap_border_brush = Cell::new(!can_lerp_border_brush && dest_border_brush.is_some());

        let (mut dip_amplitude, mut scale_amplitude) = (0.0, 0.0);
        if use_gravity_dip {
            let travel = f64::max((end_x - start_x).abs(), (end_y - start_y).abs());
            dip_amplitude = (travel * 0.12).clamp(8.0, 50.0);
            scale_amplitude = 0.05;
        }

        let animate_shadow = use_shadow && use_gravity_dip;

        let animation_cts = CancellationTokenSource::new();
        let animation_token = animation_cts.token();
        *self.animation_cts.borrow_mut() = Some(animation_cts);

        let weak_proxy = proxy.downgrade();
        let coordinated = coordinated_elements.clone();
        let progress_opacities = original_opacities.clone();
        let progress_destination = destination.clone();
        proxy.set_progress_callback(Some(Rc::new(move |progress: f64| {
            let Some(proxy) = weak_proxy.upgrade() else { return };
            let ep = easing.ease(progress);

            let bx = start_x + (end_x - start_x) * ep;
            let by = start_y + (end_y - start_y) * ep;
            let bw = start_w + (end_w - start_w) * ep;
            let bh = start_h + (end_h - start_h) * ep;

            if use_gravity_dip {
                let dip_curve = (std::f64::consts::PI * progress).sin();
                let scale_boost = 1.0 + scale_amplitude * dip_curve;
                let sw = bw * scale_boost;
                let sh = bh * scale_boost;

                Canvas::set_left(&proxy, bx - (sw - bw) / 2.0);
                Canvas::set_top(&proxy, by - (sh - bh) / 2.0 + dip_amplitude * dip_curve);
                proxy.set_width(f64::max(1.0, sw));
                proxy.set_height(f64::max(1.0, sh));

                if animate_shadow {
                    let alpha = (100.0 * dip_curve) as u8;
                    let blur = 24.0 * dip_curve;
                    let offset_y = 10.0 * dip_curve;
                    proxy.set_box_shadow(BoxShadows::new(BoxShadow {
                        offset_x: 0.0,
                        offset_y,
                        blur,
                        color: Color::from_argb(alpha, 0, 0, 0),
                        ..BoxShadow::default()
                    }));
                }
            } else {
                Canvas::set_left(&proxy, bx);
                Canvas::set_top(&proxy, by);
                proxy.set_width(f64::max(1.0, bw));
                proxy.set_height(f64::max(1.0, bh));
            }

            proxy.set_corner_radius(CornerRadius::new(
                src_tl + (dst_tl - src_tl) * ep,
                src_tr + (dst_tr - src_tr) * ep,
                src_br + (dst_br - src_br) * ep,
                src_bl + (dst_bl - src_bl) * ep,
            ));

            proxy.set_border_thickness(Thickness::new(
                src_bt.left + (dst_bt.left - src_bt.left) * ep,
                src_bt.top + (dst_bt.top - src_bt.top) * ep,
                src_bt.right + (dst_bt.right - src_bt.right) * ep,
                src_bt.bottom + (dst_bt.bottom - src_bt.bottom) * ep,
            ));

            if let Some(lerp_brush) = &lerp_brush {
                lerp_brush.set_color(Color::from_argb(
                    lerp_channel(src_bc.a, dst_bc.a, ep),
                    lerp_channel(src_bc.r, dst_bc.r, ep),
                    lerp_channel(src_bc.g, dst_bc.g, ep),
                    lerp_channel(src_bc.b, dst_bc.b, ep),
                ));
                proxy.set_border_brush(Some(lerp_brush.clone().into()));
            } else if snap_border_brush.get() && progress >= 0.5 {
                proxy.set_border_brush(dest_border_brush.clone());
                snap_border_brush.set(false);
            }

            if let Some(cross_fade_overlay) = &cross_fade_overlay {
                cross_fade_overlay.set_opacity(ep);
            }

            if progress > COORDINATED_FADE_START_THRESHOLD {
                let cp = (progress - COORDINATED_FADE_START_THRESHOLD) / COORDINATED_FADE_RANGE;
                for (j, element) in coordinated.iter().enumerate() {
                    if element == &progress_destination {
                        continue;
                    }
                    element.set_opacity(progress_opacities[j] * cp);
                }
            }
        })));

        let animation = Animation::new();
        animation.set_duration(duration);
        animation.set_easing(LinearEasing::new());
        animation.set_fill_mode(FillMode::Forward);
        animation.children().add(KeyFrame::with_cue(
            Cue::new(0.0),
            [Setter::new(ConnectedAnimationProxy::progress_property(), 0.0) as _],
        ));
        animation.children().add(KeyFrame::with_cue(
            Cue::new(1.0),
            [Setter::new(ConnectedAnimationProxy::progress_property(), 1.0) as _],
        ));

        animation.run_async(&proxy, animation_token).await.map_err(|error| error.to_string())?;

        *self.animation_cts.borrow_mut() = None;

        destination.set_opacity(dest_original_opacity);

        *self.active_destination.borrow_mut() = None;
        *self.active_proxy.borrow_mut() = None;
        *self.active_overlay_layer.borrow_mut() = None;

        overlay_layer.children().remove(&proxy);

        for (i, element) in coordinated_elements.iter().enumerate() {
            if element == &destination {
                continue;
            }
            element.set_opacity(original_opacities[i]);
        }

        let source_snapshot = self.source_snapshot.borrow_mut().take();
        if let Some(source_snapshot) = source_snapshot {
            source_snapshot.dispose();
        }

        self.on_animation_complete();
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_fallback_animation_async(
        self: &Rc<Self>,
        destination: &Ref<Visual>,
        coordinated_elements: &[Ref<Visual>],
        top_level: &TopLevel,
        duration: TimeSpan,
        easing: Easing,
        use_gravity_dip: bool,
        use_shadow: bool,
    ) {
        let Some(dest_transform) = destination.transform_to_visual(top_level) else {
            self.on_animation_complete();
            return;
        };

        let dest_bounds = Rect::from_position_size(
            dest_transform.transform(Point::new(0.0, 0.0)),
            Size::new(destination.bounds().width, destination.bounds().height),
        );

        let source_bounds = self.source_bounds.get();
        let dx = source_bounds.x - dest_bounds.x;
        let dy = source_bounds.y - dest_bounds.y;
        let sx = if source_bounds.width > 0.0 && dest_bounds.width > 0.0 {
            source_bounds.width / dest_bounds.width
        } else {
            1.0
        };
        let sy = if source_bounds.height > 0.0 && dest_bounds.height > 0.0 {
            source_bounds.height / dest_bounds.height
        } else {
            1.0
        };

        let group = TransformGroup::new();
        let scale_t = ScaleTransform::with_scale(sx, sy);
        let trans_t = TranslateTransform::with_offset(dx, dy);
        group.children().add(scale_t.clone().upcast());
        group.children().add(trans_t.clone().upcast());

        let orig_transform = destination.render_transform();
        let orig_origin = destination.render_transform_origin();
        destination.set_render_transform_origin(RelativePoint::new(0.0, 0.0, RelativeUnit::Absolute));
        destination.set_render_transform(Some(group.into()));

        let (mut dip_amp, mut scale_amp) = (0.0, 0.0);
        if use_gravity_dip {
            let travel = f64::max(dx.abs(), dy.abs());
            dip_amp = (travel * 0.12).clamp(8.0, 50.0);
            scale_amp = 0.05;
        }

        let shadow_border =
            if use_shadow && use_gravity_dip { destination.to_ref().cast::<Border>() } else { None };
        let orig_shadow = shadow_border.as_ref().map(|border| border.box_shadow()).unwrap_or_default();

        let mut original_opacities = vec![0.0; coordinated_elements.len()];
        for (i, element) in coordinated_elements.iter().enumerate() {
            if element == destination {
                continue;
            }
            original_opacities[i] = element.opacity();
            element.set_opacity(0.0);
        }

        // The timestamp of the reference is the clock of the dispatcher here.
        let start_timestamp = Dispatcher::ui_thread().now();
        let tcs = Completion::new();

        let animation_timer = DispatcherTimer::with_priority(DispatcherPriority::RENDER);
        animation_timer.set_interval(Duration::from_millis(16));
        *self.animation_timer.borrow_mut() = Some(animation_timer.clone());

        let _tick = animation_timer.tick({
            let this = self.clone();
            let shadow_border = shadow_border.clone();
            let coordinated_elements = coordinated_elements.to_vec();
            let original_opacities = original_opacities.clone();
            let destination = destination.clone();
            let tcs = tcs.clone();
            move |_| {
                let elapsed = (Dispatcher::ui_thread().now() - start_timestamp) as f64;
                let progress = f64::min(1.0, elapsed / duration.total_milliseconds());
                let ep = easing.ease(progress);

                let bsx = sx + (1.0 - sx) * ep;
                let bsy = sy + (1.0 - sy) * ep;
                let btx = dx * (1.0 - ep);
                let bty = dy * (1.0 - ep);

                if use_gravity_dip {
                    let dip_curve = (std::f64::consts::PI * progress).sin();
                    let scale_boost = 1.0 + scale_amp * dip_curve;
                    scale_t.set_scale_x(bsx * scale_boost);
                    scale_t.set_scale_y(bsy * scale_boost);
                    trans_t.set_x(btx);
                    trans_t.set_y(bty + dip_amp * dip_curve);

                    if let Some(shadow_border) = &shadow_border {
                        let alpha = (100.0 * dip_curve) as u8;
                        let blur = 24.0 * dip_curve;
                        let offset_y = 10.0 * dip_curve;
                        shadow_border.set_box_shadow(BoxShadows::new(BoxShadow {
                            offset_x: 0.0,
                            offset_y,
                            blur,
                            color: Color::from_argb(alpha, 0, 0, 0),
                            ..BoxShadow::default()
                        }));
                    }
                } else {
                    scale_t.set_scale_x(bsx);
                    scale_t.set_scale_y(bsy);
                    trans_t.set_x(btx);
                    trans_t.set_y(bty);
                }

                if progress > COORDINATED_FADE_START_THRESHOLD {
                    let cp = (progress - COORDINATED_FADE_START_THRESHOLD) / COORDINATED_FADE_RANGE;
                    for (j, element) in coordinated_elements.iter().enumerate() {
                        if element == &destination {
                            continue;
                        }
                        element.set_opacity(original_opacities[j] * cp);
                    }
                }

                if progress >= 1.0 {
                    let animation_timer = this.animation_timer.borrow_mut().take();
                    if let Some(animation_timer) = animation_timer {
                        animation_timer.stop();
                    }
                    tcs.try_set(CompletionOutcome::Result(true));
                }
            }
        });

        let animation_cts = CancellationTokenSource::new();
        let reg = animation_cts.token().register({
            let tcs = tcs.clone();
            move || tcs.try_set(CompletionOutcome::Canceled)
        });
        *self.animation_cts.borrow_mut() = Some(animation_cts);

        animation_timer.start();

        let cancelled = tcs.wait().await == CompletionOutcome::Canceled;
        reg.dispose();
        let animation_timer = self.animation_timer.borrow_mut().take();
        if let Some(animation_timer) = animation_timer {
            animation_timer.stop();
        }
        *self.animation_cts.borrow_mut() = None;

        destination.set_render_transform(orig_transform);
        destination.set_render_transform_origin(orig_origin);

        if let Some(shadow_border) = &shadow_border {
            shadow_border.set_box_shadow(orig_shadow);
        }

        for (i, element) in coordinated_elements.iter().enumerate() {
            if element == destination {
                continue;
            }
            element.set_opacity(original_opacities[i]);
        }

        let source_snapshot = self.source_snapshot.borrow_mut().take();
        if let Some(source_snapshot) = source_snapshot {
            source_snapshot.dispose();
        }

        if cancelled {
            self.raise_completed(true);
            return;
        }

        self.on_animation_complete();
    }

    /// The duration, easing, gravity dip and shadow of the animation for its
    /// configuration, with the defaults of `service`.
    pub(crate) fn resolve_timing_and_easing(&self, service: &ConnectedAnimationService) -> ResolvedTiming {
        let configuration = self.configuration();
        let configuration = configuration.as_ref().map(|configuration| configuration.as_any());

        if let Some(direct) =
            configuration.and_then(|configuration| configuration.downcast_ref::<DirectConnectedAnimationConfiguration>())
        {
            ResolvedTiming {
                duration: direct.duration().unwrap_or_else(|| service.default_duration()),
                easing: DIRECT_EASING.with(Easing::clone),
                use_gravity_dip: false,
                use_shadow: false,
            }
        } else if configuration.is_some_and(|configuration| configuration.is::<BasicConnectedAnimationConfiguration>()) {
            ResolvedTiming {
                duration: service.default_duration(),
                easing: service.default_easing_function().unwrap_or_else(|| BASIC_EASING.with(Easing::clone)),
                use_gravity_dip: false,
                use_shadow: false,
            }
        } else {
            ResolvedTiming {
                duration: service.default_duration(),
                easing: service.default_easing_function().unwrap_or_else(|| GRAVITY_EASING.with(Easing::clone)),
                use_gravity_dip: true,
                use_shadow: match configuration
                    .and_then(|configuration| configuration.downcast_ref::<GravityConnectedAnimationConfiguration>())
                {
                    Some(g) => g.is_shadow_enabled(),
                    None => true,
                },
            }
        }
    }

    fn on_animation_complete(&self) {
        self.service.remove_animation(&self.key);
        self.raise_completed(false);
    }

    fn get_background(visual: &Visual) -> Option<Rc<dyn IBrush>> {
        if let Some(b) = visual.downcast_ref::<Border>() {
            b.background()
        } else if let Some(p) = visual.downcast_ref::<Panel>() {
            p.background()
        } else if let Some(cp) = visual.downcast_ref::<ContentPresenter>() {
            cp.background()
        } else if let Some(tc) = visual.downcast_ref::<TemplatedControl>() {
            // Covers the content control case of the reference, which reads
            // the same property.
            tc.background()
        } else {
            None
        }
    }

    fn get_corner_radius(visual: &Visual) -> CornerRadius {
        if let Some(b) = visual.downcast_ref::<Border>() {
            b.corner_radius()
        } else if let Some(tc) = visual.downcast_ref::<TemplatedControl>() {
            tc.corner_radius()
        } else if let Some(cp) = visual.downcast_ref::<ContentPresenter>() {
            cp.corner_radius()
        } else {
            CornerRadius::default()
        }
    }

    fn get_border_thickness(visual: &Visual) -> Thickness {
        if let Some(b) = visual.downcast_ref::<Border>() {
            b.border_thickness()
        } else if let Some(tc) = visual.downcast_ref::<TemplatedControl>() {
            tc.border_thickness()
        } else if let Some(cp) = visual.downcast_ref::<ContentPresenter>() {
            cp.border_thickness()
        } else {
            Thickness::default()
        }
    }

    fn get_border_brush(visual: &Visual) -> Option<Rc<dyn IBrush>> {
        if let Some(b) = visual.downcast_ref::<Border>() {
            b.border_brush()
        } else if let Some(tc) = visual.downcast_ref::<TemplatedControl>() {
            tc.border_brush()
        } else if let Some(cp) = visual.downcast_ref::<ContentPresenter>() {
            cp.border_brush()
        } else {
            None
        }
    }

    fn brushes_equal(a: &Rc<dyn IBrush>, b: &Rc<dyn IBrush>) -> bool {
        if a.reference_id() == b.reference_id() {
            return true;
        }
        if let (Some(sa), Some(sb)) = (a.as_solid_color_brush(), b.as_solid_color_brush()) {
            return sa.color() == sb.color() && (a.opacity() - b.opacity()).abs() < 0.001;
        }
        false
    }
}

/// `(byte)(from + (to - from) * progress)`: the channel interpolation of the
/// reference, truncated towards zero.
fn lerp_channel(from: u8, to: u8, progress: f64) -> u8 {
    (f64::from(from) + (f64::from(to) - f64::from(from)) * progress) as u8
}

/// The outcome of a [`Completion`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletionOutcome {
    Result(bool),
    Canceled,
}

/// A task completion source whose continuation runs asynchronously: it is
/// completed once, from any thread, and the awaiting future resumes from its
/// next poll by the scheduler that runs it.
#[derive(Clone)]
struct Completion(Arc<Mutex<CompletionState>>);

#[derive(Default)]
struct CompletionState {
    outcome: Option<CompletionOutcome>,
    waker: Option<Waker>,
}

impl Completion {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(CompletionState::default())))
    }

    /// Completes the task unless it has completed already.
    fn try_set(&self, outcome: CompletionOutcome) {
        let waker = {
            let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if state.outcome.is_some() {
                return;
            }
            state.outcome = Some(outcome);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn wait(&self) -> CompletionFuture {
        CompletionFuture(self.clone())
    }
}

struct CompletionFuture(Completion);

impl Future for CompletionFuture {
    type Output = CompletionOutcome;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<CompletionOutcome> {
        let mut state = self.0 .0.lock().unwrap_or_else(PoisonError::into_inner);
        match state.outcome {
            Some(outcome) => Poll::Ready(outcome),
            None => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// The element that flies over the overlay layer: a border whose
/// `Progress`, animated from 0 to 1, drives the progress callback.
#[repr(C)]
pub(crate) struct ConnectedAnimationProxy {
    base: Border,
    progress_callback: RefCell<Option<Rc<dyn Fn(f64)>>>,
}

ferro_class!(ConnectedAnimationProxy: Border);
ferro_class_info!(ConnectedAnimationProxy { new: ConnectedAnimationProxy::new });
ferro_impl_classes!(
    ConnectedAnimationProxy: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for ConnectedAnimationProxy {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::progress_property().as_property() {
            let callback = this.progress_callback.borrow().clone();
            if let Some(callback) = callback {
                callback(change.get_new_value::<f64>());
            }
        }
    }
}

ferro_properties! {
    impl ConnectedAnimationProxy {
        /// Defines the `Progress` property.
        pub fn progress_property() -> StyledProperty<f64> {
            FerroProperty::register::<ConnectedAnimationProxy, _>("Progress", 0.0)
        }
    }
}

impl ConnectedAnimationProxy {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub(crate) fn construct() -> Self {
        Self { base: Border::construct(), progress_callback: RefCell::new(None) }
    }

    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the progress of the flight.
    #[allow(dead_code)] // The accessors of the reference; the animation sets the property itself.
    pub(crate) fn progress(&self) -> f64 {
        self.get_value(Self::progress_property())
    }

    /// Sets the progress of the flight.
    #[allow(dead_code)]
    pub(crate) fn set_progress(&self, value: f64) {
        self.set_value(Self::progress_property(), value);
    }

    pub(crate) fn set_progress_callback(&self, value: Option<Rc<dyn Fn(f64)>>) {
        *self.progress_callback.borrow_mut() = value;
    }
}
