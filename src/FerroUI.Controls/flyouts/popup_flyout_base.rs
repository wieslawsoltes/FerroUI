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
    Nullable, PixelRect, Rect, Ref, Size, StyledProperty, Thickness, VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// The handlers attached to the target of an open flyout: the subscription
/// to its detachment from the visual tree and the key up handler.
type TargetHandlers = (Rc<dyn IDisposable>, RoutedEventHandlerToken);

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
    ignore_is_open_changed: Cell<bool>,
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
        if !this.is_open.get() {
            return false;
        }

        if can_cancel && this.cancel_closing() {
            return false;
        }

        this.is_open.set(false);
        {
            let _ignore = this.begin_ignoring_is_open();
            this.set_current_value(FlyoutBase::is_open_property(), false);
        }
        let popup = this.popup();
        popup.set_is_open(false);

        popup.set_placement_target(None);
        popup.set_popup_parent(None);

        // Ensure this isn't active
        let transient_disposable = this.transient_disposable.take();
        if let Some(transient_disposable) = transient_disposable {
            transient_disposable.dispose();
        }
        this.enlarged_popup_rect.set(None);
        this.enlarge_popup_rect_screen_pixel_rect.set(None);

        let target_handlers = this.target_handlers.take();
        if let (Some(target), Some((detached, key_up))) = (this.target(), target_handlers) {
            detached.dispose();
            target.remove_handler(InputElement::key_up_event(), key_up);
        }

        this.on_closed();

        this.set_target(None);

        true
    }

    fn show_at_core(this: &Self, placement_target: &Control, show_at_pointer: bool) -> bool {
        let placement_target = placement_target.to_ref();

        *this.last_placement_target.borrow_mut() = Some(placement_target.downgrade());

        if this.is_open.get() {
            if this.target().as_ref() == Some(&placement_target) {
                return false;
            } else {
                // Close before opening a new one
                let _ = this.hide_core(false);
            }
        }

        let popup = this.popup();
        this.set_target(&placement_target);
        popup.set_placement_target(&placement_target);
        popup.set_popup_parent(Some(&placement_target));

        if popup.child().is_none() {
            popup.set_child(this.create_presenter());
        }

        popup.set_overlay_dismiss_event_pass_through(this.overlay_dismiss_event_pass_through());
        popup.set_overlay_input_pass_through_element(this.overlay_input_pass_through_element());

        if this.cancel_opening() {
            return false;
        }

        this.position_popup(show_at_pointer);
        this.is_open.set(true);
        {
            let _ignore = this.begin_ignoring_is_open();
            this.set_current_value(FlyoutBase::is_open_property(), true);
        }
        popup.set_is_open(true);
        this.on_opened();

        let weak = this.to_ref().downgrade();
        let detached = placement_target.detached_from_visual_tree({
            let weak = weak.clone();
            move |e| {
                if let Some(this) = weak.upgrade() {
                    this.placement_target_detached_from_visual_tree(e);
                }
            }
        });
        let key_up = placement_target.add_handler(InputElement::key_up_event(), move |_, e: &KeyEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.on_placement_target_or_popup_key_up(e);
            }
        });
        *this.target_handlers.borrow_mut() = Some((detached, key_up));

        match this.show_mode() {
            FlyoutShowMode::Standard => {
                // Try and focus content inside Flyout
                if let Some(child) = popup.child() {
                    if child.focusable() {
                        child.focus();
                    } else {
                        let next_focus =
                            KeyboardNavigationHandler::get_next(&child.upcast(), NavigationDirection::Next);
                        if let Some(next_focus) = next_focus {
                            next_focus.focus();
                        }
                    }
                }
            }
            FlyoutShowMode::TransientWithDismissOnPointerMoveAway => {
                if let Some(input_manager) = InputManager::instance() {
                    let weak = this.to_ref().downgrade();
                    let subscription = input_manager.process().subscribe(Rc::new(AnonymousObserver::new(
                        move |e: Rc<dyn IRawInputEventArgs>| {
                            if let Some(this) = weak.upgrade() {
                                this.handle_transient_dismiss(&*e);
                            }
                        },
                    )));
                    *this.transient_disposable.borrow_mut() = Some(subscription);
                }
            }
            FlyoutShowMode::Transient => {}
        }

        true
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

/// Resets the flag that makes the flyout ignore changes of `IsOpen`.
struct IgnoreIsOpenScope<'a> {
    owner: &'a PopupFlyoutBase,
}

impl Drop for IgnoreIsOpenScope<'_> {
    fn drop(&mut self) {
        self.owner.ignore_is_open_changed.set(false);
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
            ignore_is_open_changed: Cell::new(false),
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

    fn placement_target_detached_from_visual_tree(&self, _e: &VisualTreeAttachmentEventArgs) {
        let _ = self.hide_core(false);
        *self.last_placement_target.borrow_mut() = None;
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
        self.is_open.set(true);
        {
            let _ignore = self.begin_ignoring_is_open();
            self.set_current_value(FlyoutBase::is_open_property(), true);
        }

        let host = self.popup().host();
        for (_, handler) in self.popup_host_changed_handler.snapshot().iter() {
            handler(host.as_ref());
        }
    }

    fn on_popup_closing(&self, e: &CancelEventArgs) {
        if self.is_open.get() {
            e.set_cancel(self.cancel_closing());
        }
    }

    fn on_popup_closed(&self) {
        self.hide_core(false);

        for (_, handler) in self.popup_host_changed_handler.snapshot().iter() {
            handler(None);
        }
    }

    // This method is handling both popup logical tree and target logical tree.
    fn on_placement_target_or_popup_key_up(&self, e: &KeyEventArgs) {
        if e.handled() || !self.is_open.get() {
            return;
        }

        let this: Ref<FlyoutBase> = self.to_ref().upcast();
        if self.target().and_then(|target| target.context_flyout()) != Some(this) {
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
        if self.ignore_is_open_changed.get() {
            return;
        }

        if e.get_new_value::<bool>() {
            let last_placement_target = self.last_placement_target.borrow().as_ref().and_then(WeakRef::upgrade);
            if let Some(last_placement_target) = last_placement_target {
                if self.show_at_core(&last_placement_target, false) {
                    return;
                }
            }

            // No target, or opening was cancelled: revert so IsOpen stays honest
            let _ignore = self.begin_ignoring_is_open();
            self.set_current_value(FlyoutBase::is_open_property(), false);
        } else if !self.hide_core(true) {
            // Closing was cancelled: revert so IsOpen stays honest
            let _ignore = self.begin_ignoring_is_open();
            self.set_current_value(FlyoutBase::is_open_property(), true);
        }
    }

    fn begin_ignoring_is_open(&self) -> IgnoreIsOpenScope<'_> {
        self.ignore_is_open_changed.set(true);
        IgnoreIsOpenScope { owner: self }
    }

    fn position_popup(&self, show_at_pointer: bool) {
        let popup = self.popup();

        // The child of the popup can't be missing here, it was set in
        // `show_at_core`.
        let child = popup.child().expect("the popup has a child");
        if child.desired_size() == Size::default() {
            // Popup may not have been shown yet. Measure content
            let child: &Layoutable = &child;
            LayoutHelper::measure_child(Some(child), Size::INFINITY, Thickness::default());
        }

        popup.set_vertical_offset(self.vertical_offset());
        popup.set_horizontal_offset(self.horizontal_offset());
        popup.set_placement_anchor(self.placement_anchor());
        popup.set_placement_gravity(self.placement_gravity());
        popup.set_custom_popup_placement_callback(self.custom_popup_placement_callback());
        if show_at_pointer {
            popup.set_placement(PlacementMode::Pointer);
        } else {
            popup.set_placement(self.placement());
            popup.set_placement_constraint_adjustment(self.placement_constraint_adjustment());
        }
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

    fn on_control_context_canceled(sender: &Interactive, e: &RoutedEventArgs) {
        if e.handled() {
            return;
        }
        let Some(control) = sender.downcast_ref::<Control>() else { return };
        let Some(flyout) = control.context_flyout() else { return };

        if flyout.is_open() {
            flyout.hide();
        }
    }

    fn on_control_context_requested(sender: &Interactive, e: &ContextRequestedEventArgs) {
        if e.handled() {
            return;
        }
        let Some(control) = sender.downcast_ref::<Control>() else { return };
        let Some(flyout) = control.context_flyout() else { return };

        if control.context_menu().is_some() {
            if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, "FlyoutBase") {
                let source: &dyn std::any::Any = control;
                logger.log(Some(source), "ContextMenu and ContextFlyout are both set, defaulting to ContextMenu");
            }
            return;
        }

        if let Some(popup_flyout) = flyout.cast::<PopupFlyoutBase>() {
            // Absolute popup positioning is not supported yet, so the point
            // is ignored at this moment.
            let triggered_by_pointer_input = e.try_get_position(None).is_some();
            e.set_handled(popup_flyout.show_at_core(control, triggered_by_pointer_input));

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
