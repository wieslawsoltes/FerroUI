use super::{FlyoutBase, FlyoutBaseImpl, FlyoutShowMode};
use crate::diagnostics::IPopupHostProvider;
use crate::platform::{FeedbackAction, PlatformFeedbackExtensions};
use crate::primitives::popup_positioning::{
    CustomPopupPlacementCallback, PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment,
};
use crate::primitives::{CustomPopupPlacementCallbackValue, IPopupHost, Popup};
use crate::{Application, Control, PlacementMode};
use ferroui_base::controls::Classes;
use ferroui_base::input::raw::{IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{
    ContextRequestedEventArgs, InputElement, InputManager, KeyEventArgs, KeyboardNavigationHandler,
    NavigationDirection,
};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::layout::{LayoutHelper, Layoutable};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::reactive::{AnonymousObserver, Disposable, IDisposable};
use ferroui_base::utilities::{CancelEventArgs, HandlerList};
use ferroui_base::{
    ferro_class, ferro_property, FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs,
    Nullable, PixelRect, Rect, Ref, Size, StyledProperty, Thickness, Visual, WeakRef,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// Registrations belong to the captured target, even when a callback changes
/// the public Target property before the old request returns.
struct TargetHandlers {
    owner: WeakRef<Control>,
    generation: u64,
    detached: Rc<dyn IDisposable>,
    key_up: RoutedEventHandlerToken,
}

/// A closing call temporarily owns the old registrations. Keeping them local
/// allows a canceled underlying Popup close to restore the existing session,
/// while a nested new flyout request installs and keeps its own registrations.
struct FlyoutSessionHandlers {
    target: Option<TargetHandlers>,
    transient: Option<Rc<dyn IDisposable>>,
}

impl Drop for FlyoutSessionHandlers {
    fn drop(&mut self) {
        if let Some(transient) = self.transient.take() {
            transient.dispose();
        }
    }
}

impl Drop for TargetHandlers {
    fn drop(&mut self) {
        self.detached.dispose();
        if let Some(owner) = self.owner.upgrade() {
            owner.remove_handler(InputElement::key_up_event(), self.key_up);
        }
    }
}

/// A request cannot survive a detach/reattach to the same root. Observing the
/// attachment before invoking application code also covers a pending Opening.
struct TargetLifetime {
    target: Ref<Control>,
    root: Option<Ref<Visual>>,
    detached: Rc<Cell<bool>>,
    subscription: Rc<dyn IDisposable>,
}

impl TargetLifetime {
    fn new(target: Ref<Control>) -> Self {
        let root = target.visual_root();
        let detached = Rc::new(Cell::new(false));
        let observed = detached.clone();
        let subscription = target.detached_from_visual_tree(move |_| observed.set(true));
        Self { target, root, detached, subscription }
    }

    fn is_current(&self) -> bool {
        !self.detached.get() && self.target.visual_root() == self.root
    }
}

impl Drop for TargetLifetime {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}

/// The base class of flyouts that are shown in a [`Popup`].
///
/// This class is abstract: only its subclasses can be created.
#[repr(C)]
pub struct PopupFlyoutBase {
    base: FlyoutBase,
    popup_lazy: OnceCell<Ref<Popup>>,
    enlarged_popup_rect: Cell<Option<Rect>>,
    enlarge_popup_rect_screen_pixel_rect: Cell<Option<PixelRect>>,
    transient_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    popup_host_changed_handler: HandlerList<dyn Fn(Option<&Rc<dyn IPopupHost>>)>,
    is_open: Cell<bool>,
    request_generation: Cell<u64>,
    /// Ignore only the matching internal publication, once. A property handler
    /// can synchronously make a different application request.
    ignore_is_open_changed: Cell<Option<bool>>,
    /// The flyout does not own its placement targets: an owner (a button)
    /// holds the flyout, so the reference back is weak.
    last_placement_target: RefCell<Option<WeakRef<Control>>>,
    target_handlers: RefCell<Option<TargetHandlers>>,
    closing: HandlerList<dyn Fn(&CancelEventArgs)>,
    opening: HandlerList<dyn Fn(&CancelEventArgs)>,
}

ferro_class! {
    PopupFlyoutBase: FlyoutBase, virtuals PopupFlyoutBaseImpl: FlyoutBaseImpl {
        /// Hides the flyout; `can_cancel` tells whether the `Closing` event
        /// is raised and may cancel. Returns true if the action was handled.
        fn hide_core(this, can_cancel: bool) -> bool;
        /// Shows the flyout at the given control, at the pointer location
        /// when `show_at_pointer`. Returns true if the action was handled.
        fn show_at_core(this, placement_target: &Control, show_at_pointer: bool) -> bool;
        /// Shows the flyout below an explicit rectangle in the target's
        /// coordinates. The popup positioner applies RTL and screen constraints.
        /// This request does not modify the flyout's placement properties.
        fn show_at_rect_core(this, placement_target: &Control, placement_rect: Rect) -> bool;
        /// Raises the `Opening` event.
        fn on_opening(this, args: &CancelEventArgs);
        /// Raises the `Closing` event.
        fn on_closing(this, args: &CancelEventArgs);
        /// Used to create the content the flyout displays.
        fn create_presenter(this) -> Ref<Control>;
    }
}

ferroui_base::ferro_impl_classes!(PopupFlyoutBase: FerroObjectImpl);

impl FlyoutBaseImpl for PopupFlyoutBase {
    /// Shows the flyout at the given control. Sealed in the reference:
    /// derived classes override `show_at_core`.
    fn show_at(this: &Self, placement_target: &Control) {
        this.show_at_core(placement_target, false);
    }

    /// Hides the flyout. Sealed in the reference: derived classes override
    /// `hide_core`.
    fn hide(this: &Self) {
        this.hide_core(true);
    }
}

impl PopupFlyoutBaseImpl for PopupFlyoutBase {
    fn hide_core(this: &Self, can_cancel: bool) -> bool {
        // Hide also retires an Opening that has not published IsOpen yet.
        let generation = this.next_request();
        let was_open = this.is_open.get();
        if !was_open && this.target().is_none() {
            return false;
        }

        if was_open && can_cancel {
            let canceled = this.cancel_closing();
            if canceled || !this.is_current_request(generation) {
                return false;
            }
        }

        // Take the old registrations before any callback can open a new
        // session. Their cleanup uses their actual owner, never current Target.
        let mut handlers = FlyoutSessionHandlers {
            target: this.target_handlers.take(),
            transient: this.transient_disposable.take(),
        };
        this.is_open.set(false);
        let popup = this.popup();
        // Retire the real host before publishing the flyout's false value.
        // Its property handler may reopen at another target, which needs a new
        // Popup session rather than the old target's detached subscription.
        popup.set_is_open(false);
        if !this.is_current_request(generation) {
            return false;
        }
        if popup.is_open() {
            // An application handler can cancel the real Popup close or call
            // Popup::open during Closing. Its accepted host keeps the existing
            // flyout session, including detachment and transient dismissal.
            let previous = this.target_handlers.replace(handlers.target.take());
            drop(previous);
            let previous = this.transient_disposable.replace(handlers.transient.take());
            if let Some(previous) = previous {
                previous.dispose();
            }
            this.is_open.set(true);
            this.synchronize_is_open();
            return false;
        }
        drop(handlers);
        this.enlarged_popup_rect.set(None);
        this.enlarge_popup_rect_screen_pixel_rect.set(None);
        this.synchronize_is_open();
        if !this.is_current_request(generation) {
            return false;
        }
        popup.set_placement_target(None);
        if !this.is_current_request(generation) {
            return false;
        }
        popup.set_popup_parent(None);
        if !this.is_current_request(generation) {
            return false;
        }

        if was_open {
            this.on_closed();
            if !this.is_current_request(generation) {
                return false;
            }
        }
        this.set_target(None);
        was_open && this.is_current_request(generation)
    }

    fn show_at_core(this: &Self, placement_target: &Control, show_at_pointer: bool) -> bool {
        Self::show_at_with_placement(this, placement_target, show_at_pointer, None)
    }

    fn show_at_rect_core(this: &Self, placement_target: &Control, placement_rect: Rect) -> bool {
        Self::show_at_with_placement(this, placement_target, false, Some(placement_rect))
    }

    fn on_opening(this: &Self, args: &CancelEventArgs) {
        for (_, handler) in this.opening.snapshot().iter() {
            handler(args);
        }
    }

    fn on_closing(this: &Self, args: &CancelEventArgs) {
        for (_, handler) in this.closing.snapshot().iter() {
            handler(args);
        }
    }

    fn create_presenter(_this: &Self) -> Ref<Control> {
        panic!("PopupFlyoutBase is abstract.")
    }
}

impl PopupFlyoutBase {
    fn show_at_with_placement(
        this: &Self,
        placement_target: &Control,
        show_at_pointer: bool,
        placement_rect: Option<Rect>,
    ) -> bool {
        // Once callback-bearing setup starts, a newer request consumes the
        // old routed invocation even though its writes must stop. Keep the
        // ordinary same-target/no-rectangle no-op and Opening cancellation
        // unhandled, as before; do not infer this outcome from current IsOpen.
        let lifetime = TargetLifetime::new(placement_target.to_ref());
        let placement_target = &lifetime.target;
        let mut generation = this.next_request();
        *this.last_placement_target.borrow_mut() = Some(placement_target.downgrade());

        if this.is_open.get() {
            if this.target().as_ref() == Some(placement_target) {
                if placement_rect.is_some() {
                    this.position_popup(generation, &lifetime, false, placement_rect);
                    return true;
                }
                return false;
            } else {
                // Preserve the virtual close extension point. Its default
                // implementation advances the request once. Only that owned
                // close may hand the ticket back; a callback's newer request
                // must win, including a show made by a Closed handler.
                let close_generation = generation.wrapping_add(1);
                let _ = this.hide_core(false);
                if !this.is_current_request(close_generation) {
                    return true;
                }
                if this.is_open.get() || this.target().is_some() || this.popup().is_open() {
                    return false;
                }
                generation = close_generation;
            }
        }

        if !this.accepts_show(generation, &lifetime) {
            return true;
        }
        let popup = this.popup();
        this.set_target(placement_target);
        if !this.accepts_show(generation, &lifetime) {
            return true;
        }
        popup.set_placement_target(placement_target);
        if !this.accepts_show(generation, &lifetime) {
            return true;
        }
        popup.set_popup_parent(Some(placement_target));
        if !this.accepts_show(generation, &lifetime) {
            return true;
        }

        if popup.child().is_none() {
            let presenter = this.create_presenter();
            if !this.accepts_show(generation, &lifetime) {
                return true;
            }
            popup.set_child(presenter);
            if !this.accepts_show(generation, &lifetime) {
                return true;
            }
        }

        popup.set_overlay_dismiss_event_pass_through(this.overlay_dismiss_event_pass_through());
        if !this.accepts_show(generation, &lifetime) {
            return true;
        }
        popup.set_overlay_input_pass_through_element(this.overlay_input_pass_through_element());
        if !this.accepts_show(generation, &lifetime) {
            return true;
        }

        let canceled = this.cancel_opening();
        if !this.accepts_show(generation, &lifetime) {
            return true;
        }
        if canceled {
            this.hide_core(false);
            return false;
        }
        if !this.position_popup(generation, &lifetime, show_at_pointer, placement_rect) {
            return true;
        }

        // Registration precedes property and Opened callbacks: a canceled Hide
        // in one of those callbacks keeps a fully owned open session.
        this.install_target_handlers(placement_target, generation);
        if this.show_mode() == FlyoutShowMode::TransientWithDismissOnPointerMoveAway {
            if let Some(input_manager) = InputManager::instance() {
                let weak = this.to_ref().downgrade();
                let subscription = input_manager.process().subscribe(Rc::new(AnonymousObserver::new(
                    move |e: Rc<dyn IRawInputEventArgs>| {
                        if let Some(this) = weak.upgrade().filter(|this| this.owns_target_handlers(generation)) {
                            this.handle_transient_dismiss(&*e);
                        }
                    },
                )));
                if !this.accepts_show(generation, &lifetime) {
                    subscription.dispose();
                    return true;
                }
                *this.transient_disposable.borrow_mut() = Some(subscription);
            }
        }
        this.is_open.set(true);
        popup.set_is_open(true);
        if !this.accepts_show(generation, &lifetime) {
            return true;
        }
        this.synchronize_is_open();
        if !this.accepts_show(generation, &lifetime) {
            return true;
        }
        this.on_opened();
        if !this.accepts_show(generation, &lifetime) {
            return true;
        }

        match this.show_mode() {
            FlyoutShowMode::Standard => {
                // Try and focus content inside Flyout
                if let Some(child) = popup.child() {
                    if child.focusable() {
                        child.focus();
                    } else {
                        let next_focus =
                            KeyboardNavigationHandler::get_next(&child.upcast(), NavigationDirection::Next);
                        if !this.accepts_show(generation, &lifetime) {
                            return true;
                        }
                        if let Some(next_focus) = next_focus {
                            next_focus.focus();
                        }
                    }
                }
            }
            FlyoutShowMode::TransientWithDismissOnPointerMoveAway | FlyoutShowMode::Transient => {}
        }

        this.accepts_show(generation, &lifetime);
        true
    }
}

/// Resets the flag that makes the flyout ignore changes of `IsOpen`.
struct IgnoreIsOpenScope<'a> {
    owner: &'a PopupFlyoutBase,
    previous: Option<bool>,
}

impl Drop for IgnoreIsOpenScope<'_> {
    fn drop(&mut self) {
        self.owner.ignore_is_open_changed.set(self.previous);
    }
}

ferroui_base::ferro_properties! { impl PopupFlyoutBase {
    ferro_property!(
        /// Defines the `Placement` property.
        pub fn placement_property() -> StyledProperty<PlacementMode> {
            Popup::placement_property().add_owner::<PopupFlyoutBase>()
        }
    );

    ferro_property!(
        /// Defines the `HorizontalOffset` property.
        pub fn horizontal_offset_property() -> StyledProperty<f64> {
            Popup::horizontal_offset_property().add_owner::<PopupFlyoutBase>()
        }
    );

    ferro_property!(
        /// Defines the `VerticalOffset` property.
        pub fn vertical_offset_property() -> StyledProperty<f64> {
            Popup::vertical_offset_property().add_owner::<PopupFlyoutBase>()
        }
    );

    ferro_property!(
        /// Defines the `PlacementAnchor` property.
        pub fn placement_anchor_property() -> StyledProperty<PopupAnchor> {
            Popup::placement_anchor_property().add_owner::<PopupFlyoutBase>()
        }
    );

    ferro_property!(
        /// Defines the `PlacementGravity` property.
        pub fn placement_gravity_property() -> StyledProperty<PopupGravity> {
            Popup::placement_gravity_property().add_owner::<PopupFlyoutBase>()
        }
    );

    ferro_property!(
        /// Defines the `CustomPopupPlacementCallback` property.
        pub fn custom_popup_placement_callback_property() -> StyledProperty<Option<CustomPopupPlacementCallbackValue>> {
            Popup::custom_popup_placement_callback_property().add_owner::<PopupFlyoutBase>()
        }
    );

    ferro_property!(
        /// Defines the `ShowMode` property.
        pub fn show_mode_property() -> StyledProperty<FlyoutShowMode> {
            FerroProperty::register::<PopupFlyoutBase, _>("ShowMode", FlyoutShowMode::Standard)
        }
    );

    ferro_property!(
        /// Defines the `OverlayDismissEventPassThrough` property.
        pub fn overlay_dismiss_event_pass_through_property() -> StyledProperty<bool> {
            Popup::overlay_dismiss_event_pass_through_property().add_owner::<PopupFlyoutBase>()
        }
    );

    ferro_property!(
        /// Defines the `OverlayInputPassThroughElement` property.
        pub fn overlay_input_pass_through_element_property() -> StyledProperty<Option<ferroui_base::ElementRef<InputElement>>> {
            Popup::overlay_input_pass_through_element_property().add_owner::<PopupFlyoutBase>()
        }
    );

    ferro_property!(
        /// Defines the `PlacementConstraintAdjustment` property.
        pub fn placement_constraint_adjustment_property() -> StyledProperty<PopupPositionerConstraintAdjustment> {
            Popup::placement_constraint_adjustment_property().add_owner::<PopupFlyoutBase>()
        }
    );
} }

impl PopupFlyoutBase {
    fn static_constructor() {
        FlyoutBase::is_open_property().changed().add_class_handler::<PopupFlyoutBase>(|x, e| x.is_open_changed(e));
        Control::context_flyout_property()
            .changed()
            .add_class_handler::<Control>(Self::on_context_flyout_property_changed);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FlyoutBase::construct(),
            popup_lazy: OnceCell::new(),
            enlarged_popup_rect: Cell::new(None),
            enlarge_popup_rect_screen_pixel_rect: Cell::new(None),
            transient_disposable: RefCell::new(None),
            popup_host_changed_handler: HandlerList::new(),
            is_open: Cell::new(false),
            request_generation: Cell::new(0),
            ignore_is_open_changed: Cell::new(None),
            last_placement_target: RefCell::new(None),
            target_handlers: RefCell::new(None),
            closing: HandlerList::new(),
            opening: HandlerList::new(),
        }
    }

    /// The popup the flyout is shown in, created on first use. For derived
    /// classes.
    pub fn popup(&self) -> Ref<Popup> {
        self.popup_lazy.get_or_init(|| self.create_popup()).clone()
    }

    /// The desired placement of the flyout in relation to its target.
    pub fn placement(&self) -> PlacementMode {
        self.get_value(Self::placement_property())
    }

    pub fn set_placement(&self, value: PlacementMode) {
        self.set_value(Self::placement_property(), value)
    }

    /// Defines in what direction the flyout should open when the placement
    /// is [`PlacementMode::AnchorAndGravity`].
    pub fn placement_gravity(&self) -> PopupGravity {
        self.get_value(Self::placement_gravity_property())
    }

    pub fn set_placement_gravity(&self, value: PopupGravity) {
        self.set_value(Self::placement_gravity_property(), value)
    }

    /// The anchor point on the placement rect when the placement is
    /// [`PlacementMode::AnchorAndGravity`].
    pub fn placement_anchor(&self) -> PopupAnchor {
        self.get_value(Self::placement_anchor_property())
    }

    pub fn set_placement_anchor(&self, value: PopupAnchor) {
        self.set_value(Self::placement_anchor_property(), value)
    }

    /// The horizontal offset of the flyout in relation to its target.
    pub fn horizontal_offset(&self) -> f64 {
        self.get_value(Self::horizontal_offset_property())
    }

    pub fn set_horizontal_offset(&self, value: f64) {
        self.set_value(Self::horizontal_offset_property(), value)
    }

    /// The vertical offset of the flyout in relation to its target.
    pub fn vertical_offset(&self) -> f64 {
        self.get_value(Self::vertical_offset_property())
    }

    pub fn set_vertical_offset(&self, value: f64) {
        self.set_value(Self::vertical_offset_property(), value)
    }

    /// The callback that positions the flyout when the placement is
    /// [`PlacementMode::Custom`].
    pub fn custom_popup_placement_callback(&self) -> Option<CustomPopupPlacementCallback> {
        self.get_value(Self::custom_popup_placement_callback_property()).map(|value| value.0)
    }

    pub fn set_custom_popup_placement_callback(&self, value: Option<CustomPopupPlacementCallback>) {
        self.set_value(
            Self::custom_popup_placement_callback_property(),
            value.map(CustomPopupPlacementCallbackValue),
        )
    }

    /// The desired show mode.
    pub fn show_mode(&self) -> FlyoutShowMode {
        self.get_value(Self::show_mode_property())
    }

    pub fn set_show_mode(&self, value: FlyoutShowMode) {
        self.set_value(Self::show_mode_property(), value)
    }

    /// Whether the event that closes the flyout is passed through to the
    /// parent window.
    ///
    /// Clicks outside the popup cause the popup to close. When this is
    /// false, these clicks will be handled by the popup and not be
    /// registered by the parent window. When true, the events will be passed
    /// through to the parent window.
    pub fn overlay_dismiss_event_pass_through(&self) -> bool {
        self.get_value(Self::overlay_dismiss_event_pass_through_property())
    }

    pub fn set_overlay_dismiss_event_pass_through(&self, value: bool) {
        self.set_value(Self::overlay_dismiss_event_pass_through_property(), value)
    }

    /// An element that should receive pointer input events even when
    /// underneath the flyout's overlay.
    pub fn overlay_input_pass_through_element(&self) -> Option<Ref<InputElement>> {
        ferroui_base::ElementRef::resolve(&self.get_value(Self::overlay_input_pass_through_element_property()))
    }

    pub fn set_overlay_input_pass_through_element(&self, value: impl Into<Nullable<InputElement>>) {
        self.set_value(
            Self::overlay_input_pass_through_element_property(),
            ferroui_base::ElementRef::from_nullable(value.into().0),
        )
    }

    /// Describes how the flyout position will be adjusted if the unadjusted
    /// position would result in the flyout being partly constrained.
    pub fn placement_constraint_adjustment(&self) -> PopupPositionerConstraintAdjustment {
        self.get_value(Self::placement_constraint_adjustment_property())
    }

    pub fn set_placement_constraint_adjustment(&self, value: PopupPositionerConstraintAdjustment) {
        self.set_value(Self::placement_constraint_adjustment_property(), value)
    }

    /// The flyout as a popup host provider (the diagnostics contract it
    /// implements in the reference).
    pub fn to_popup_host_provider(&self) -> Rc<dyn IPopupHostProvider> {
        Rc::new(PopupFlyoutHostProvider(self.to_ref()))
    }

    fn subscribe<F: ?Sized + 'static>(
        &self,
        select: fn(&PopupFlyoutBase) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }

    /// Raised before the flyout closes; the close can be cancelled.
    /// Disposing the returned handle unsubscribes.
    pub fn closing(&self, handler: impl Fn(&CancelEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&CancelEventArgs)>(|flyout| &flyout.closing, Rc::new(handler))
    }

    /// Raised before the flyout opens; the opening can be cancelled.
    /// Disposing the returned handle unsubscribes.
    pub fn opening(&self, handler: impl Fn(&CancelEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&CancelEventArgs)>(|flyout| &flyout.opening, Rc::new(handler))
    }

    /// Pre-registers a control as the default placement target for this
    /// flyout. Used by owning controls (e.g. the button) so that setting
    /// `IsOpen` to `true` works on first use.
    pub(crate) fn set_default_placement_target(&self, target: Option<&Control>) {
        *self.last_placement_target.borrow_mut() = target.map(|target| target.to_ref().downgrade());
    }

    /// Shows the flyout for the given control at the current pointer
    /// location when `show_at_pointer`, as in a context flyout.
    pub fn show_at_with(&self, placement_target: &Control, show_at_pointer: bool) {
        self.show_at_core(placement_target, show_at_pointer);
    }

    fn next_request(&self) -> u64 {
        let generation = self.request_generation.get().wrapping_add(1);
        self.request_generation.set(generation);
        generation
    }

    fn is_current_request(&self, generation: u64) -> bool {
        self.request_generation.get() == generation
    }

    fn accepts_show(&self, generation: u64, lifetime: &TargetLifetime) -> bool {
        if !self.is_current_request(generation) {
            return false;
        }
        if lifetime.is_current() {
            return true;
        }
        self.clear_last_placement_target(&lifetime.target);
        self.hide_core(false);
        false
    }

    fn clear_last_placement_target(&self, target: &Ref<Control>) {
        let is_target = self.last_placement_target.borrow().as_ref().and_then(WeakRef::upgrade).as_ref() == Some(target);
        if is_target {
            self.last_placement_target.borrow_mut().take();
        }
    }

    fn owns_target_handlers(&self, generation: u64) -> bool {
        self.target_handlers.borrow().as_ref().is_some_and(|handlers| handlers.generation == generation)
    }

    fn install_target_handlers(&self, target: &Ref<Control>, generation: u64) {
        let weak = self.to_ref().downgrade();
        let owner = target.downgrade();
        let detached = target.detached_from_visual_tree({
            let weak = weak.clone();
            let owner = owner.clone();
            move |_| {
                if let Some(this) = weak.upgrade().filter(|this| this.owns_target_handlers(generation)) {
                    if let Some(owner) = owner.upgrade() {
                        // Clear before Hide: Closed may synchronously show at a
                        // different target, whose reopen target must survive.
                        this.clear_last_placement_target(&owner);
                    }
                    this.hide_core(false);
                }
            }
        });
        let key_up = target.add_handler(InputElement::key_up_event(), move |_, e: &KeyEventArgs| {
            if let Some(this) = weak.upgrade().filter(|this| this.owns_target_handlers(generation)) {
                this.on_placement_target_or_popup_key_up(e);
            }
        });
        let old = self.target_handlers.replace(Some(TargetHandlers { owner, generation, detached, key_up }));
        drop(old);
    }

    fn handle_transient_dismiss(&self, args: &dyn IRawInputEventArgs) {
        let Some(p_args) = args.downcast_ref::<RawPointerEventArgs>() else { return };
        if p_args.type_() != RawPointerEventType::Move {
            return;
        }

        // In the show mode that dismisses on pointer move away, the flyout
        // is kept shown as long as the pointer is within a certain px
        // distance from the flyout itself: 100px, which seems about right.
        // The enlarged popup rect is the flyout bounds enlarged 100px.
        // For windowed popups, the enlarged rect is in screen coordinates,
        // for overlay popups, it is in overlay layer coordinates.
        let host = self.popup().host();
        let popup_root = host.as_ref().and_then(|host| host.as_popup_root());
        let overlay_popup_host = host.as_ref().and_then(|host| host.as_overlay_popup_host());

        if self.enlarged_popup_rect.get().is_none() && self.enlarge_popup_rect_screen_pixel_rect.get().is_none() {
            // Only do this once when the flyout opens & cache the result
            if let Some(root) = popup_root {
                // Get the popup root bounds and convert to screen coordinates
                let tmp = root.bounds().inflate(100.0);
                self.enlarge_popup_rect_screen_pixel_rect.set(Some(PixelRect::from_points(
                    root.point_to_screen(tmp.top_left()),
                    root.point_to_screen(tmp.bottom_right()),
                )));
            } else if let Some(host) = overlay_popup_host {
                // Overlay popups are in overlay layer coordinates, just use that
                self.enlarged_popup_rect.set(Some(host.bounds().inflate(100.0)));
            }

            return;
        }

        if popup_root.is_some() {
            // As long as the pointer stays within the enlarged popup rect
            // the flyout stays open. If it leaves, close it.
            // Despite working in screen coordinates, leaving the top-level
            // window will not close this (as pointer events stop).
            let event_root = p_args.root().root_element();
            let pt = event_root.point_to_screen(p_args.position());
            if self.enlarge_popup_rect_screen_pixel_rect.get().is_some_and(|rect| !rect.contains(pt)) {
                self.hide_core(false);
            }
        } else if overlay_popup_host.is_some() {
            // Same as above here, but just different coordinate space
            // so we don't need to translate
            if self.enlarged_popup_rect.get().is_some_and(|rect| !rect.contains(p_args.position())) {
                self.hide_core(false);
            }
        }
    }

    fn create_popup(&self) -> Ref<Popup> {
        let popup = Popup::new();
        popup.set_window_manager_add_shadow_hint(false);
        popup.set_is_light_dismiss_enabled(true);

        // The flyout owns the popup and lives as long as it: the handlers
        // are never removed.
        let weak = self.to_ref().downgrade();
        let _ = popup.opened({
            let weak = weak.clone();
            move || {
                if let Some(this) = weak.upgrade() {
                    this.on_popup_opened();
                }
            }
        });
        let _ = popup.closed({
            let weak = weak.clone();
            move || {
                if let Some(this) = weak.upgrade() {
                    this.on_popup_closed();
                }
            }
        });
        let _ = popup.closing({
            let weak = weak.clone();
            move |e| {
                if let Some(this) = weak.upgrade() {
                    this.on_popup_closing(e);
                }
            }
        });
        popup.add_handler(InputElement::key_up_event(), move |_, e: &KeyEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.on_placement_target_or_popup_key_up(e);
            }
        });
        popup
    }

    fn on_popup_opened(&self) {
        let generation = self.request_generation.get();
        self.is_open.set(true);
        self.synchronize_is_open();
        if !self.is_current_request(generation) {
            return;
        }

        let host = self.popup().host();
        for (_, handler) in self.popup_host_changed_handler.snapshot().iter() {
            handler(host.as_ref());
            if !self.is_current_request(generation) {
                return;
            }
        }
    }

    fn on_popup_closing(&self, e: &CancelEventArgs) {
        if self.is_open.get() {
            e.set_cancel(self.cancel_closing());
        }
    }

    fn on_popup_closed(&self) {
        // An owned close has already retired its session. Do not manufacture
        // another close ticket and prevent a retargeting show from continuing.
        let mut generation = self.request_generation.get();
        if self.is_open.get() {
            self.hide_core(false);
            generation = generation.wrapping_add(1);
        }
        if !self.is_current_request(generation) || self.is_open.get() {
            return;
        }
        self.synchronize_is_open();
        if !self.is_current_request(generation) {
            return;
        }

        for (_, handler) in self.popup_host_changed_handler.snapshot().iter() {
            handler(None);
            if !self.is_current_request(generation) {
                return;
            }
        }
    }

    // This method is handling both popup logical tree and target logical tree.
    fn on_placement_target_or_popup_key_up(&self, e: &KeyEventArgs) {
        if e.handled() || !self.is_open.get() {
            return;
        }

        let this: Ref<FlyoutBase> = self.to_ref().upcast();
        if self.target().and_then(|target| Self::effective_context_flyout(&target)) != Some(this) {
            return;
        }

        let keymap = Application::current()
            .and_then(|application| application.platform_settings())
            .map(|settings| settings.hotkey_configuration());

        if keymap.is_some_and(|keymap| keymap.open_context_menu.iter().any(|k| k.matches(Some(e)))) {
            e.set_handled(self.hide_core(true));
        }
    }

    fn is_open_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let value = e.get_new_value::<bool>();
        if self.ignore_is_open_changed.get() == Some(value) {
            self.ignore_is_open_changed.set(None);
            return;
        }

        if value {
            let last_placement_target = self.last_placement_target.borrow().as_ref().and_then(WeakRef::upgrade);
            if let Some(last_placement_target) = last_placement_target {
                self.show_at_core(&last_placement_target, false);
            } else {
                self.next_request();
            }
        } else {
            self.hide_core(true);
        }
        // Cancellation and reentrant requests may both change the outcome.
        // Publish the accepted state instead of inverting the original input.
        self.synchronize_is_open();
    }

    fn synchronize_is_open(&self) {
        let value = self.is_open.get();
        let _ignore = self.begin_ignoring_is_open(value);
        self.set_current_value(FlyoutBase::is_open_property(), value);
    }

    fn begin_ignoring_is_open(&self, value: bool) -> IgnoreIsOpenScope<'_> {
        let previous = self.ignore_is_open_changed.replace(Some(value));
        IgnoreIsOpenScope { owner: self, previous }
    }

    fn position_popup(
        &self,
        generation: u64,
        lifetime: &TargetLifetime,
        show_at_pointer: bool,
        placement_rect: Option<Rect>,
    ) -> bool {
        let popup = self.popup();

        // The child of the popup can't be missing here, it was set in
        // `show_at_core`.
        let child = popup.child().expect("the popup has a child");
        if child.desired_size() == Size::default() {
            // Popup may not have been shown yet. Measure content
            let child: &Layoutable = &child;
            LayoutHelper::measure_child(Some(child), Size::INFINITY, Thickness::default());
            if !self.accepts_show(generation, lifetime) {
                return false;
            }
        }

        popup.set_vertical_offset(self.vertical_offset());
        if !self.accepts_show(generation, lifetime) {
            return false;
        }
        popup.set_horizontal_offset(self.horizontal_offset());
        if !self.accepts_show(generation, lifetime) {
            return false;
        }
        popup.set_placement_anchor(self.placement_anchor());
        if !self.accepts_show(generation, lifetime) {
            return false;
        }
        popup.set_placement_gravity(self.placement_gravity());
        if !self.accepts_show(generation, lifetime) {
            return false;
        }
        popup.set_custom_popup_placement_callback(self.custom_popup_placement_callback());
        if !self.accepts_show(generation, lifetime) {
            return false;
        }
        // Every opening supplies its own rectangle, including None, so a
        // later ordinary request cannot reuse a previous text-context anchor.
        popup.set_placement_rect(placement_rect);
        if !self.accepts_show(generation, lifetime) {
            return false;
        }
        if placement_rect.is_some() {
            popup.set_placement(PlacementMode::BottomEdgeAlignedLeft);
            if !self.accepts_show(generation, lifetime) {
                return false;
            }
            popup.set_placement_constraint_adjustment(self.placement_constraint_adjustment());
        } else if show_at_pointer {
            popup.set_placement(PlacementMode::Pointer);
        } else {
            popup.set_placement(self.placement());
            if !self.accepts_show(generation, lifetime) {
                return false;
            }
            popup.set_placement_constraint_adjustment(self.placement_constraint_adjustment());
        }
        self.accepts_show(generation, lifetime)
    }

    fn on_context_flyout_property_changed(c: &Control, args: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = args.get_old_and_new_value::<Option<Ref<FlyoutBase>>>();

        if old_value.is_some() {
            if let Some((context_requested, context_canceled)) = c.context_flyout_handlers().take() {
                c.remove_handler(InputElement::context_requested_event(), context_requested);
                c.remove_handler(InputElement::context_canceled_event(), context_canceled);
            }
        }
        if new_value.is_some() {
            let context_requested =
                c.add_handler(InputElement::context_requested_event(), Self::on_control_context_requested);
            let context_canceled =
                c.add_handler(InputElement::context_canceled_event(), Self::on_control_context_canceled);
            c.context_flyout_handlers().set(Some((context_requested, context_canceled)));
        }
    }

    pub(crate) fn on_control_context_canceled(sender: &Interactive, e: &RoutedEventArgs) {
        if e.handled() {
            return;
        }
        let Some(control) = sender.downcast_ref::<Control>() else { return };
        let Some(flyout) = Self::effective_context_flyout(control) else { return };

        if flyout.is_open() {
            flyout.hide();
        }
    }

    pub(crate) fn on_control_context_requested(sender: &Interactive, e: &ContextRequestedEventArgs) {
        if e.handled() {
            return;
        }
        let Some(control) = sender.downcast_ref::<Control>() else { return };
        let Some(flyout) = Self::context_flyout_for_request(control) else { return };

        if control.context_menu().is_some() {
            if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, "FlyoutBase") {
                let source: &dyn std::any::Any = control;
                logger.log(Some(source), "ContextMenu and ContextFlyout are both set, defaulting to ContextMenu");
            }
            return;
        }

        if let Some(popup_flyout) = flyout.cast::<PopupFlyoutBase>() {
            let root = control.visual_root();
            let lifetime = TargetLifetime::new(control.to_ref());
            let popup_generation = popup_flyout.request_generation.get();
            let rich_request = control.downcast_ref::<crate::RichTextBlock>()
                .map(|view| (view, view.context_menu_request_generation()));
            // Converting an explicit anchor reads virtual ITransform values.
            // A nested rich request or a replacement menu owns the new intent;
            // the old request must not reposition its accepted popup afterward.
            let placement_rect = e.try_get_placement_rect(control);
            if e.handled()
                || root != control.visual_root()
                || !lifetime.is_current()
                || !popup_flyout.is_current_request(popup_generation)
                || rich_request.is_some_and(|(view, generation)| {
                    generation != view.context_menu_request_generation()
                })
                || control.context_menu().is_some()
                || Self::context_flyout_for_request(control) != Some(flyout.clone())
            {
                e.set_handled(true);
                return;
            }
            let handled = if let Some(rect) = placement_rect {
                popup_flyout.show_at_rect_core(control, rect)
            } else {
                let triggered_by_pointer_input = e.try_get_position(None).is_some();
                popup_flyout.show_at_core(control, triggered_by_pointer_input)
            };
            e.set_handled(handled);

            if e.handled() && e.is_holding() {
                control.perform_feedback(FeedbackAction::hold());
            }
        } else {
            flyout.show_at(control);
            e.set_handled(true);

            if e.is_holding() {
                control.perform_feedback(FeedbackAction::hold());
            }
        }
    }

    /// Resolves the internal overflow fallback without writing the local
    /// ContextFlyout property or changing the requesting placement target.
    pub(crate) fn effective_context_flyout(control: &Control) -> Option<Ref<FlyoutBase>> {
        control.context_flyout().or_else(|| {
            control
                .downcast_ref::<crate::RichTextBlockOverflow>()
                .and_then(|overflow| overflow.inherited_context_flyout())
        })
    }

    pub(crate) fn context_flyout_for_request(control: &Control) -> Option<Ref<FlyoutBase>> {
        if control.downcast_ref::<crate::RichTextBlockOverflow>()
            .is_some_and(|overflow| !overflow.context_flyout_is_enabled())
        {
            return None;
        }
        Self::effective_context_flyout(control)
    }

    fn cancel_closing(&self) -> bool {
        let event_args = CancelEventArgs::new();
        self.on_closing(&event_args);
        event_args.cancel()
    }

    fn cancel_opening(&self) -> bool {
        let event_args = CancelEventArgs::new();
        self.on_opening(&event_args);
        event_args.cancel()
    }

    /// Makes the classes of `presenter` the given ones, leaving its
    /// pseudoclasses alone.
    pub(crate) fn set_presenter_classes(presenter: Option<&Control>, classes: &Classes) {
        let Some(presenter) = presenter else { return };

        // Remove any classes no longer in use, ignoring pseudo classes
        let presenter_classes = presenter.classes();
        for i in (0..presenter_classes.count()).rev() {
            let name = presenter_classes.get(i);
            if !classes.contains(&name) && !name.contains(':') {
                presenter_classes.remove_at(i);
            }
        }

        // Add new classes
        presenter_classes.add_range(classes.snapshot().iter());
    }
}

/// The popup host provider contract of a flyout.
struct PopupFlyoutHostProvider(Ref<PopupFlyoutBase>);

impl IPopupHostProvider for PopupFlyoutHostProvider {
    fn popup_host(&self) -> Option<Rc<dyn IPopupHost>> {
        self.0.popup().host()
    }

    fn popup_host_changed(&self, handler: Rc<dyn Fn(Option<&Rc<dyn IPopupHost>>)>) -> Rc<dyn IDisposable> {
        self.0.subscribe::<dyn Fn(Option<&Rc<dyn IPopupHost>>)>(|flyout| &flyout.popup_host_changed_handler, handler)
    }
}
