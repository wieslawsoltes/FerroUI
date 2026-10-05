use super::popup_positioning::{
    CustomPopupPlacementCallback, PopupAnchor, PopupGravity, PopupPositionRequest, PopupPositionerConstraintAdjustment,
};
use super::{
    IPopupHost, LightDismissOverlayLayer, OverlayPopupHost, PopupRoot, TemplateAppliedEventArgs, TemplatedControl,
};
use crate::diagnostics::IPopupHostProvider;
use crate::presenters::ContentPresenter;
use ferroui_base::input::text_input::TransformTrackingHelper;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::utilities::CancelEventArgs;
use ferroui_base::ElementRef;
use crate::{Control, ControlImpl, PixelPointEventArgs, PlacementMode, TopLevel, Window, WindowBase};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::raw::{IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{InputElement, InputElementImpl, InputManager, PointerPressedEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::media::{ScaleTransform, Transform};
use ferroui_base::reactive::{AnonymousObserver, CompositeDisposable, Disposable, IDisposable, ObservableExt};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, DirectProperty,
    FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    IFerroDependencyResolver, Matrix, Nullable, ObjectType, PixelPoint, Point, Rect, Ref, Size, StyledElement,
    StyledElementImpl, StyledElementImplExt, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The value of the `CustomPopupPlacementCallback` property: a callback,
/// compared by identity.
#[derive(Clone)]
pub struct CustomPopupPlacementCallbackValue(pub CustomPopupPlacementCallback);

impl PartialEq for CustomPopupPlacementCallbackValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Displays a popup window.
#[repr(C)]
pub struct Popup {
    base: Control,
    dependency_resolver: RefCell<Option<Rc<dyn IFerroDependencyResolver>>>,
    is_open_requested: Cell<bool>,
    ignore_is_open_changed: Cell<bool>,
    is_using_overlay_layer: Cell<bool>,
    open_state: RefCell<Option<Rc<PopupOpenState>>>,
    popup_host_changed_handler: HandlerList<dyn Fn(Option<&Rc<dyn IPopupHost>>)>,
    opened_popups: RefCell<Vec<Ref<Popup>>>,
    closed: HandlerList<dyn Fn()>,
    opened: HandlerList<dyn Fn()>,
    closing: HandlerList<dyn Fn(&CancelEventArgs)>,
}

ferro_class!(Popup: Control);
ferroui_base::ferro_class_info!(Popup { new: Popup::new });
ferro_impl_classes!(Popup: InteractiveImpl, InputElementImpl);

impl ControlImpl for Popup {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::PopupAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for Popup {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let Some(open_state) = this.open_state() else { return };

        let property = change.property();
        if property == Layoutable::width_property().as_property()
            || property == Layoutable::min_width_property().as_property()
            || property == Layoutable::max_width_property().as_property()
            || property == Layoutable::height_property().as_property()
            || property == Layoutable::min_height_property().as_property()
            || property == Layoutable::max_height_property().as_property()
        {
            this.update_host_sizing(&open_state.popup_host, open_state.top_level(), &open_state.placement_target());
        } else if property == Self::placement_target_property().as_property()
            || property == Self::placement_property().as_property()
            || property == Self::horizontal_offset_property().as_property()
            || property == Self::vertical_offset_property().as_property()
            || property == Self::placement_anchor_property().as_property()
            || property == Self::placement_constraint_adjustment_property().as_property()
            || property == Self::placement_rect_property().as_property()
        {
            if property == Self::placement_target_property().as_property() {
                let new_target = ElementRef::resolve(&change.get_new_value::<Option<ElementRef<Control>>>())
                    .or_else(|| find_logical_ancestor_of_type::<Control>(this));

                let Some(new_target) = new_target else {
                    this.close();
                    return;
                };
                if TopLevel::get_top_level(Some(&new_target)) != open_state.top_level() {
                    this.close();
                    return;
                }

                open_state.set_placement_target(new_target);
            }

            this.update_host_position(&open_state.popup_host, &open_state.placement_target());
        } else if property == Self::topmost_property().as_property() {
            open_state.popup_host.set_topmost(change.get_new_value::<bool>());
        } else if property == InputElement::is_hit_test_visible_property().as_property() {
            open_state.popup_host.set_is_hit_test_visible(change.get_new_value::<bool>());
        }
    }
}

impl StyledElementImpl for Popup {
    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_logical_tree(this, e);
        this.close();
    }
}

impl VisualImpl for Popup {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        if this.is_open_requested.get() {
            this.open();
        }
    }
}

impl LayoutableImpl for Popup {
    /// Measures the control: a size of 0,0 as the popup itself takes up no
    /// space.
    fn measure_core(_this: &Self, _available_size: Size) -> Size {
        Size::default()
    }
}

/// Resets the flag that makes the popup ignore changes of `IsOpen`.
struct IgnoreIsOpenScope<'a> {
    owner: &'a Popup,
}

impl Drop for IgnoreIsOpenScope<'_> {
    fn drop(&mut self) {
        self.owner.ignore_is_open_changed.set(false);
    }
}

ferroui_base::ferro_properties! { impl Popup {
    ferro_property!(
        /// Defines the `WindowManagerAddShadowHint` property.
        pub fn window_manager_add_shadow_hint_property() -> StyledProperty<bool> {
            FerroProperty::register::<Popup, _>("WindowManagerAddShadowHint", false)
        }
    );

    ferro_property!(
        /// Defines the `Child` property.
        pub fn child_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<Popup, _>("Child", None)
        }
    );

    ferro_property!(
        /// Defines the `InheritsTransform` property.
        pub fn inherits_transform_property() -> StyledProperty<bool> {
            FerroProperty::register::<Popup, _>("InheritsTransform", false)
        }
    );

    ferro_property!(
        /// Defines the `IsOpen` property.
        pub fn is_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<Popup, _>("IsOpen", false)
        }
    );

    ferro_property!(
        /// Defines the `PlacementAnchor` property.
        pub fn placement_anchor_property() -> StyledProperty<PopupAnchor> {
            FerroProperty::register::<Popup, _>("PlacementAnchor", PopupAnchor::NONE)
        }
    );

    ferro_property!(
        /// Defines the `PlacementConstraintAdjustment` property.
        pub fn placement_constraint_adjustment_property() -> StyledProperty<PopupPositionerConstraintAdjustment> {
            FerroProperty::register::<Popup, _>(
                "PlacementConstraintAdjustment",
                PopupPositionerConstraintAdjustment::FLIP_X
                    | PopupPositionerConstraintAdjustment::FLIP_Y
                    | PopupPositionerConstraintAdjustment::SLIDE_X
                    | PopupPositionerConstraintAdjustment::SLIDE_Y
                    | PopupPositionerConstraintAdjustment::RESIZE_X
                    | PopupPositionerConstraintAdjustment::RESIZE_Y,
            )
        }
    );

    ferro_property!(
        /// Defines the `PlacementGravity` property.
        pub fn placement_gravity_property() -> StyledProperty<PopupGravity> {
            FerroProperty::register::<Popup, _>("PlacementGravity", PopupGravity::NONE)
        }
    );

    ferro_property!(
        /// Defines the `Placement` property.
        pub fn placement_property() -> StyledProperty<PlacementMode> {
            FerroProperty::register::<Popup, _>("Placement", PlacementMode::Bottom)
        }
    );

    ferro_property!(
        /// Defines the `PlacementRect` property.
        pub fn placement_rect_property() -> StyledProperty<Option<Rect>> {
            FerroProperty::register::<Popup, _>("PlacementRect", None)
        }
    );

    ferro_property!(
        /// Defines the `PlacementTarget` property.
        ///
        /// The popup does not own its placement target (it is usually an
        /// ancestor of the popup), so the value is an element reference.
        pub fn placement_target_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<Popup, _>("PlacementTarget", None)
        }
    );

    ferro_property!(
        /// Defines the `CustomPopupPlacementCallback` property.
        pub fn custom_popup_placement_callback_property() -> StyledProperty<Option<CustomPopupPlacementCallbackValue>> {
            FerroProperty::register::<Popup, _>("CustomPopupPlacementCallback", None)
        }
    );

    ferro_property!(
        /// Defines the `OverlayDismissEventPassThrough` property.
        pub fn overlay_dismiss_event_pass_through_property() -> StyledProperty<bool> {
            FerroProperty::register::<Popup, _>("OverlayDismissEventPassThrough", false)
        }
    );

    ferro_property!(
        /// Defines the `OverlayInputPassThroughElement` property.
        pub fn overlay_input_pass_through_element_property() -> StyledProperty<Option<ElementRef<InputElement>>> {
            ValueTypes::register_element_ref::<InputElement>();
            FerroProperty::register::<Popup, _>("OverlayInputPassThroughElement", None)
        }
    );

    ferro_property!(
        /// Defines the `HorizontalOffset` property.
        pub fn horizontal_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<Popup, _>("HorizontalOffset", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `IsLightDismissEnabled` property.
        pub fn is_light_dismiss_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<Popup, _>("IsLightDismissEnabled", false)
        }
    );

    ferro_property!(
        /// Defines the `VerticalOffset` property.
        pub fn vertical_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<Popup, _>("VerticalOffset", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `Topmost` property.
        pub fn topmost_property() -> StyledProperty<bool> {
            FerroProperty::register::<Popup, _>("Topmost", false)
        }
    );

    ferro_property!(
        /// Defines the `TakesFocusFromNativeControl` property.
        pub fn takes_focus_from_native_control_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<Popup, Control, _>("TakesFocusFromNativeControl", true)
        }
    );

    ferro_property!(
        /// Defines the `ShouldUseOverlayLayer` property.
        pub fn should_use_overlay_layer_property() -> StyledProperty<bool> {
            FerroProperty::register::<Popup, _>("ShouldUseOverlayLayer", false)
        }
    );

    ferro_property!(
        /// Defines the `IsUsingOverlayLayer` property.
        pub fn is_using_overlay_layer_property() -> DirectProperty<Popup, bool> {
            FerroProperty::register_direct::<Popup, _>("IsUsingOverlayLayer", |o| o.is_using_overlay_layer(), None, false)
        }
    );
} }

impl Popup {
    fn static_constructor() {
        Self::child_property().changed().add_class_handler::<Popup>(|x, e| x.child_changed(e));
        Self::is_open_property().changed().add_class_handler::<Popup>(|x, e| x.is_open_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            dependency_resolver: RefCell::new(None),
            is_open_requested: Cell::new(false),
            ignore_is_open_changed: Cell::new(false),
            is_using_overlay_layer: Cell::new(false),
            open_state: RefCell::new(None),
            popup_host_changed_handler: HandlerList::new(),
            opened_popups: RefCell::new(Vec::new()),
            closed: HandlerList::new(),
            opened: HandlerList::new(),
            closing: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn subscribe<F: ?Sized + 'static>(&self, select: fn(&Popup) -> &HandlerList<F>, handler: Rc<F>) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }

    /// Raised when the popup closes. Disposing the returned handle
    /// unsubscribes.
    pub fn closed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn()>(|popup| &popup.closed, Rc::new(handler))
    }

    /// Raised when the popup opens. Disposing the returned handle
    /// unsubscribes.
    pub fn opened(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn()>(|popup| &popup.opened, Rc::new(handler))
    }

    /// Raised before the popup closes; the close can be cancelled (internal
    /// upstream). Disposing the returned handle unsubscribes.
    pub fn closing(&self, handler: impl Fn(&CancelEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&CancelEventArgs)>(|popup| &popup.closing, Rc::new(handler))
    }

    /// Raised when the popup host changes (the popup host provider
    /// contract). Disposing the returned handle unsubscribes.
    pub fn popup_host_changed(
        &self,
        handler: impl Fn(Option<&Rc<dyn IPopupHost>>) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(Option<&Rc<dyn IPopupHost>>)>(|popup| &popup.popup_host_changed_handler, Rc::new(handler))
    }

    fn open_state(&self) -> Option<Rc<PopupOpenState>> {
        self.open_state.borrow().clone()
    }

    /// The host of the open popup (internal upstream).
    pub fn host(&self) -> Option<Rc<dyn IPopupHost>> {
        self.open_state().map(|open_state| open_state.popup_host.clone())
    }

    /// The popups that are currently open directly inside this popup, in
    /// the order they were opened.
    pub fn opened_popups(&self) -> Vec<Ref<Popup>> {
        self.opened_popups.borrow().clone()
    }

    /// A hint to the window manager that a shadow should be added to the
    /// popup.
    pub fn window_manager_add_shadow_hint(&self) -> bool {
        self.get_value(Self::window_manager_add_shadow_hint_property())
    }

    pub fn set_window_manager_add_shadow_hint(&self, value: bool) {
        self.set_value(Self::window_manager_add_shadow_hint_property(), value)
    }

    /// The control to display in the popup.
    pub fn child(&self) -> Option<Ref<Control>> {
        self.get_value(Self::child_property())
    }

    pub fn set_child(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::child_property(), value.into().0)
    }

    /// A dependency resolver for the [`PopupRoot`].
    ///
    /// This property allows a client to customize the behaviour of the
    /// popup by injecting a specialized dependency resolver into the
    /// [`PopupRoot`]'s constructor.
    pub fn dependency_resolver(&self) -> Option<Rc<dyn IFerroDependencyResolver>> {
        self.dependency_resolver.borrow().clone()
    }

    pub fn set_dependency_resolver(&self, value: Option<Rc<dyn IFerroDependencyResolver>>) {
        *self.dependency_resolver.borrow_mut() = value;
    }

    /// Determines whether the popup inherits the render transform from its
    /// placement target. Defaults to false.
    pub fn inherits_transform(&self) -> bool {
        self.get_value(Self::inherits_transform_property())
    }

    pub fn set_inherits_transform(&self, value: bool) {
        self.set_value(Self::inherits_transform_property(), value)
    }

    /// Determines how the popup can be dismissed.
    ///
    /// Light dismiss is when the user taps on any area other than the
    /// popup.
    pub fn is_light_dismiss_enabled(&self) -> bool {
        self.get_value(Self::is_light_dismiss_enabled_property())
    }

    pub fn set_is_light_dismiss_enabled(&self, value: bool) {
        self.set_value(Self::is_light_dismiss_enabled_property(), value)
    }

    /// Whether the popup is currently open.
    pub fn is_open(&self) -> bool {
        self.get_value(Self::is_open_property())
    }

    pub fn set_is_open(&self, value: bool) {
        self.set_value(Self::is_open_property(), value)
    }

    /// The anchor point on the placement rect when the placement is
    /// [`PlacementMode::AnchorAndGravity`].
    pub fn placement_anchor(&self) -> PopupAnchor {
        self.get_value(Self::placement_anchor_property())
    }

    pub fn set_placement_anchor(&self, value: PopupAnchor) {
        self.set_value(Self::placement_anchor_property(), value)
    }

    /// Describes how the popup position will be adjusted if the unadjusted
    /// position would result in the popup being partly constrained.
    pub fn placement_constraint_adjustment(&self) -> PopupPositionerConstraintAdjustment {
        self.get_value(Self::placement_constraint_adjustment_property())
    }

    pub fn set_placement_constraint_adjustment(&self, value: PopupPositionerConstraintAdjustment) {
        self.set_value(Self::placement_constraint_adjustment_property(), value)
    }

    /// Defines in what direction the popup should open when the placement
    /// is [`PlacementMode::AnchorAndGravity`].
    pub fn placement_gravity(&self) -> PopupGravity {
        self.get_value(Self::placement_gravity_property())
    }

    pub fn set_placement_gravity(&self, value: PopupGravity) {
        self.set_value(Self::placement_gravity_property(), value)
    }

    /// The desired placement of the popup in relation to the placement
    /// target.
    pub fn placement(&self) -> PlacementMode {
        self.get_value(Self::placement_property())
    }

    pub fn set_placement(&self, value: PlacementMode) {
        self.set_value(Self::placement_property(), value)
    }

    /// The anchor rectangle within the parent that the popup will be placed
    /// relative to when the placement is
    /// [`PlacementMode::AnchorAndGravity`].
    ///
    /// The placement rect defines a rectangle relative to the placement
    /// target around which the popup will be opened, with the placement
    /// anchor determining which edge of the placement target is used.
    ///
    /// If unset, the anchor rectangle will be the bounds of the placement
    /// target.
    pub fn placement_rect(&self) -> Option<Rect> {
        self.get_value(Self::placement_rect_property())
    }

    pub fn set_placement_rect(&self, value: Option<Rect>) {
        self.set_value(Self::placement_rect_property(), value)
    }

    /// The control that is used to determine the popup's position.
    pub fn placement_target(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::placement_target_property()))
    }

    pub fn set_placement_target(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::placement_target_property(), ElementRef::from_nullable(value.into().0))
    }

    /// The callback that positions the popup when the placement is
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

    /// Whether the event that closes the popup is passed through to the
    /// parent window.
    ///
    /// When light dismiss is enabled, clicks outside the popup cause the
    /// popup to close. When this is false, these clicks will be handled by
    /// the popup and not be registered by the parent window. When true, the
    /// events will be passed through to the parent window.
    pub fn overlay_dismiss_event_pass_through(&self) -> bool {
        self.get_value(Self::overlay_dismiss_event_pass_through_property())
    }

    pub fn set_overlay_dismiss_event_pass_through(&self, value: bool) {
        self.set_value(Self::overlay_dismiss_event_pass_through_property(), value)
    }

    /// An element that should receive pointer input events even when
    /// underneath the popup's overlay.
    pub fn overlay_input_pass_through_element(&self) -> Option<Ref<InputElement>> {
        ElementRef::resolve(&self.get_value(Self::overlay_input_pass_through_element_property()))
    }

    pub fn set_overlay_input_pass_through_element(&self, value: impl Into<Nullable<InputElement>>) {
        self.set_value(Self::overlay_input_pass_through_element_property(), ElementRef::from_nullable(value.into().0))
    }

    /// The horizontal offset of the popup in relation to the placement
    /// target.
    pub fn horizontal_offset(&self) -> f64 {
        self.get_value(Self::horizontal_offset_property())
    }

    pub fn set_horizontal_offset(&self, value: f64) {
        self.set_value(Self::horizontal_offset_property(), value)
    }

    /// The vertical offset of the popup in relation to the placement
    /// target.
    pub fn vertical_offset(&self) -> f64 {
        self.get_value(Self::vertical_offset_property())
    }

    pub fn set_vertical_offset(&self, value: f64) {
        self.set_value(Self::vertical_offset_property(), value)
    }

    /// Whether this popup appears on top of all other windows.
    pub fn topmost(&self) -> bool {
        self.get_value(Self::topmost_property())
    }

    pub fn set_topmost(&self, value: bool) {
        self.set_value(Self::topmost_property(), value)
    }

    /// Whether the popup, on show, transfers focus from any focused native
    /// control to the toolkit. The default is `true`.
    ///
    /// This property only applies to advanced native control embedding
    /// scenarios. By default, if a popup is shown when a native control is
    /// focused, focus is transferred back to the toolkit in order for the
    /// popup to receive input. If this property is set to `false`, then the
    /// shown popup will not receive input until it receives an interaction
    /// which explicitly focuses the popup, such as a mouse click.
    pub fn takes_focus_from_native_control(&self) -> bool {
        self.get_value(Self::takes_focus_from_native_control_property())
    }

    pub fn set_takes_focus_from_native_control(&self, value: bool) {
        self.set_value(Self::takes_focus_from_native_control_property(), value)
    }

    /// Whether the popup should be shown in the overlay layer of the parent
    /// window.
    ///
    /// When this is false the implementation depends on the platform. Use
    /// [`is_using_overlay_layer`](Self::is_using_overlay_layer) to get the
    /// actual popup behavior. This is an equivalent of the overlay popups
    /// platform option, but settable independently per each popup.
    pub fn should_use_overlay_layer(&self) -> bool {
        self.get_value(Self::should_use_overlay_layer_property())
    }

    pub fn set_should_use_overlay_layer(&self, value: bool) {
        self.set_value(Self::should_use_overlay_layer_property(), value)
    }

    /// Whether the popup is shown in the overlay layer of the parent
    /// window.
    pub fn is_using_overlay_layer(&self) -> bool {
        self.is_using_overlay_layer.get()
    }

    fn set_is_using_overlay_layer(&self, value: bool) {
        self.set_and_raise_cell(Self::is_using_overlay_layer_property(), &self.is_using_overlay_layer, value);
    }

    /// Opens the popup.
    pub fn open(&self) {
        // Popup is currently open
        if self.open_state.borrow().is_some() {
            return;
        }

        let Some(placement_target) =
            self.placement_target().or_else(|| find_logical_ancestor_of_type::<Control>(self))
        else {
            self.is_open_requested.set(true);
            return;
        };

        let Some(top_level) = TopLevel::get_top_level(Some(&placement_target)) else {
            self.is_open_requested.set(true);
            return;
        };

        self.is_open_requested.set(false);

        let this = self.to_ref();
        let weak = this.downgrade();
        let popup_host = OverlayPopupHost::create_popup_host(
            &placement_target,
            self.dependency_resolver(),
            self.should_use_overlay_layer(),
        );
        let handler_cleanup = Rc::new(CompositeDisposable::with_capacity(7));

        self.update_host_sizing(&popup_host, Some(top_level.clone()), &placement_target);
        popup_host.set_topmost(self.topmost());
        popup_host.set_is_hit_test_visible(self.is_hit_test_visible());
        popup_host.set_child(self.child());
        popup_host.as_control().set_parent(this.clone());

        if self.inherits_transform() {
            let weak = weak.clone();
            handler_cleanup.add(TransformTrackingHelper::track(&placement_target.clone().upcast(), true, move |v, m| {
                if let Some(this) = weak.upgrade() {
                    this.placement_target_transform_changed(v, m);
                }
            }));
        } else {
            popup_host.set_transform(None);
        }

        if let Some(top_level_popup) = popup_host.as_popup_root() {
            handler_cleanup.add(top_level_popup.bind_typed_value(
                ThemeVariant::actual_theme_variant_property(),
                self.get_binding_observable(ThemeVariant::actual_theme_variant_property()),
                BindingPriority::LocalValue,
            ));
        }

        self.update_host_position(&popup_host, &placement_target);

        let template_applied_subscription = popup_host.template_applied(Rc::new({
            let weak = weak.clone();
            move |e| {
                if let Some(this) = weak.upgrade() {
                    this.root_template_applied(e);
                }
            }
        }));
        handler_cleanup.add(template_applied_subscription.clone());

        handler_cleanup.add(placement_target.detached_from_visual_tree({
            let weak = weak.clone();
            move |e| {
                if let Some(this) = weak.upgrade() {
                    this.target_detached(e);
                }
            }
        }));

        self.subscribe_to_top_level(&top_level, &placement_target, &handler_cleanup);

        if let Some(input_manager) = InputManager::instance() {
            let weak = weak.clone();
            handler_cleanup.add(input_manager.process().subscribe(Rc::new(AnonymousObserver::new(
                move |e: Rc<dyn IRawInputEventArgs>| {
                    if let Some(this) = weak.upgrade() {
                        this.listen_for_non_client_click(&*e);
                    }
                },
            ))));
        }

        let cleanup_popup = Disposable::create({
            let popup_host = popup_host.clone();
            let handler_cleanup = handler_cleanup.clone();
            move || {
                handler_cleanup.dispose();

                popup_host.set_child(None);
                popup_host.hide();

                popup_host.as_control().set_parent(None);
                popup_host.dispose();
            }
        });

        if self.is_light_dismiss_enabled() {
            let dismiss_layer = LightDismissOverlayLayer::get_light_dismiss_overlay_layer(&placement_target);

            if let Some(dismiss_layer) = dismiss_layer {
                handler_cleanup.add(dismiss_layer.register(self.overlay_input_pass_through_element().as_ref()));

                let token = dismiss_layer.add_handler(InputElement::pointer_pressed_event(), {
                    let weak = weak.clone();
                    move |_, e: &PointerPressedEventArgs| {
                        if let Some(this) = weak.upgrade() {
                            this.pointer_pressed_dismiss_overlay(e);
                        }
                    }
                });
                let weak_layer = dismiss_layer.downgrade();
                handler_cleanup.add(Disposable::create(move || {
                    if let Some(dismiss_layer) = weak_layer.upgrade() {
                        dismiss_layer.remove_handler(InputElement::pointer_pressed_event(), token);
                    }
                }));
            }
        }

        let parent_popup = find_parent_popup(&placement_target);
        let open_state = Rc::new(PopupOpenState::new(
            placement_target,
            &top_level,
            popup_host.clone(),
            cleanup_popup,
            parent_popup.as_ref(),
            template_applied_subscription,
        ));
        *self.open_state.borrow_mut() = Some(open_state);

        if let Some(parent_popup) = parent_popup {
            parent_popup.add_opened_popup(&this);
        } else {
            top_level.add_opened_popup(&this);
        }

        window_manager_add_shadow_hint_changed(&popup_host, self.window_manager_add_shadow_hint());

        popup_host.show();
        self.set_is_using_overlay_layer(popup_host.as_overlay_popup_host().is_some());

        if self.takes_focus_from_native_control() {
            popup_host.take_focus();
        }

        {
            let _ignore = self.begin_ignoring_is_open();
            self.set_current_value(Self::is_open_property(), true);
        }

        for (_, handler) in self.opened.snapshot().iter() {
            handler();
        }

        let host = self.host();
        for (_, handler) in self.popup_host_changed_handler.snapshot().iter() {
            handler(host.as_ref());
        }
    }

    /// Subscribes to the notifications of the top-level of the placement
    /// target that close or reposition the popup.
    fn subscribe_to_top_level(
        &self,
        top_level: &Ref<TopLevel>,
        placement_target: &Ref<Control>,
        handler_cleanup: &CompositeDisposable,
    ) {
        let weak = self.to_ref().downgrade();

        if let Some(window) = top_level.cast::<Window>() {
            let window: &WindowBase = &window;
            if window.platform_impl().is_some() {
                handler_cleanup.add(window.deactivated({
                    let weak = weak.clone();
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.window_deactivated();
                        }
                    }
                }));

                handler_cleanup.add(top_level.platform_lost_focus({
                    let weak = weak.clone();
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.window_lost_focus();
                        }
                    }
                }));

                // Recalculate popup position on parent moved/resized, but not if placement was on pointer
                if self.placement() != PlacementMode::Pointer {
                    handler_cleanup.add(top_level.platform_position_changed({
                        let weak = weak.clone();
                        move |pp| {
                            if let Some(this) = weak.upgrade() {
                                this.window_position_changed(pp);
                            }
                        }
                    }));

                    // If the placement target is moved, update the popup position
                    handler_cleanup.add(placement_target.layout_updated({
                        let weak = weak.clone();
                        move || {
                            if let Some(this) = weak.upgrade() {
                                this.placement_target_layout_updated();
                            }
                        }
                    }));
                }

                return;
            }
        }

        if let Some(parent_popup_root) = top_level.cast::<PopupRoot>() {
            handler_cleanup.add(parent_popup_root.position_changed({
                let weak = weak.clone();
                move |e| {
                    if let Some(this) = weak.upgrade() {
                        this.parent_popup_position_changed(e);
                    }
                }
            }));

            if let Some(popup) = parent_popup_root.parent().and_then(|parent| parent.cast::<Popup>()) {
                handler_cleanup.add(popup.closed({
                    let weak = weak.clone();
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.parent_closed();
                        }
                    }
                }));
            }
        } else if let Some(platform_impl) = top_level.platform_impl() {
            if platform_impl.as_window_base_impl().is_some() {
                handler_cleanup.add(top_level.platform_lost_focus({
                    let weak = weak.clone();
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.top_level_lost_platform_focus();
                        }
                    }
                }));

                handler_cleanup.add(top_level.platform_deactivated({
                    let weak = weak.clone();
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.window_base_deactivated();
                        }
                    }
                }));
            } else {
                handler_cleanup.add(top_level.platform_lost_focus({
                    let weak = weak.clone();
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.top_level_lost_platform_focus();
                        }
                    }
                }));
            }
        }
    }

    /// Closes the popup.
    pub fn close(&self) {
        self.close_core()
    }

    /// Gets the value of the `TakesFocusFromNativeControl` attached
    /// property on the specified control.
    pub fn get_takes_focus_from_native_control(control: &Control) -> bool {
        control.get_value(Self::takes_focus_from_native_control_property())
    }

    /// Sets the value of the `TakesFocusFromNativeControl` attached
    /// property on the specified control.
    pub fn set_takes_focus_from_native_control_on(control: &Control, value: bool) {
        control.set_value(Self::takes_focus_from_native_control_property(), value)
    }

    /// Helper method to set popup's styling and templated parent (internal
    /// upstream).
    pub fn set_popup_parent(&self, new_parent: Option<&Ref<Control>>) {
        let new_parent_element: Option<Ref<StyledElement>> = new_parent.map(|parent| parent.clone().upcast());
        if self.parent().is_some() && self.parent() != new_parent_element {
            self.set_parent(None);
        }

        if self.parent().is_none() || self.placement_target().as_ref() != new_parent {
            self.set_parent(Nullable(new_parent_element));
            self.set_templated_parent(Nullable(new_parent.and_then(|parent| parent.templated_parent())));
        }
    }

    fn update_host_position(&self, popup_host: &Rc<dyn IPopupHost>, placement_target: &Ref<Control>) {
        popup_host.configure_position(PopupPositionRequest::new_with(
            placement_target.clone().upcast(),
            self.placement(),
            Point::new(self.horizontal_offset(), self.vertical_offset()),
            self.placement_anchor(),
            self.placement_gravity(),
            self.placement_constraint_adjustment(),
            Some(self.placement_rect().unwrap_or_else(|| Rect::from_size(placement_target.bounds().size()))),
            self.custom_popup_placement_callback(),
        ));
    }

    fn update_host_sizing(
        &self,
        popup_host: &Rc<dyn IPopupHost>,
        top_level: Option<Ref<TopLevel>>,
        placement_target: &Ref<Control>,
    ) {
        let mut scale_x = 1.0;
        let mut scale_y = 1.0;

        let matrix = if self.inherits_transform() {
            top_level.and_then(|top_level| placement_target.transform_to_visual(&top_level))
        } else {
            None
        };

        if let Some(m) = matrix {
            scale_x = (m.m11 * m.m11 + m.m12 * m.m12).sqrt();
            scale_y = (m.m11 * m.m11 + m.m12 * m.m12).sqrt();

            // Ideally we'd only assign a ScaleTransform here when the scale != 1, but there's
            // an issue with LayoutTransformControl in that it sets its LayoutTransform property
            // with LocalValue priority in ArrangeOverride in certain cases when LayoutTransform
            // is null, which breaks TemplateBindings to this property.
            let transform: Ref<Transform> = ScaleTransform::with_scale(scale_x, scale_y).upcast();
            popup_host.set_transform(Some(transform));
        } else {
            popup_host.set_transform(None);
        }

        popup_host.set_width(self.width() * scale_x);
        popup_host.set_min_width(self.min_width() * scale_x);
        popup_host.set_max_width(self.max_width() * scale_x);
        popup_host.set_height(self.height() * scale_y);
        popup_host.set_min_height(self.min_height() * scale_y);
        popup_host.set_max_height(self.max_height() * scale_y);
    }

    fn handle_position_change(&self) {
        if let Some(open_state) = self.open_state() {
            self.update_host_position(&open_state.popup_host, &open_state.placement_target());
        }
    }

    /// Called when the `IsOpen` property changes.
    fn is_open_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if !self.ignore_is_open_changed.get() {
            if e.get_new_value::<bool>() {
                self.open();
            } else {
                self.close();
            }
        }
    }

    /// Called when the `Child` property changes.
    fn child_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = e.get_old_and_new_value::<Option<Ref<Control>>>();

        self.logical_children().clear();

        if let Some(old_value) = old_value {
            old_value.set_parent(None);
        }

        if let Some(new_value) = new_value {
            new_value.set_parent(self.to_ref());
            self.logical_children().add(new_value.upcast());
        }
    }

    fn close_core(&self) {
        let closing_args = CancelEventArgs::new();
        for (_, handler) in self.closing.snapshot().iter() {
            handler(&closing_args);
        }
        if closing_args.cancel() {
            return;
        }

        self.is_open_requested.set(false);
        let Some(open_state) = self.open_state() else {
            let _ignore = self.begin_ignoring_is_open();
            self.set_current_value(Self::is_open_property(), false);
            return;
        };

        let this = self.to_ref();
        if let Some(parent_popup) = open_state.parent_popup() {
            parent_popup.remove_opened_popup(&this);
        } else if let Some(top_level) = open_state.top_level() {
            top_level.remove_opened_popup(&this);
        }

        open_state.dispose();
        *self.open_state.borrow_mut() = None;

        for (_, handler) in self.popup_host_changed_handler.snapshot().iter() {
            handler(None);
        }

        {
            let _ignore = self.begin_ignoring_is_open();
            self.set_current_value(Self::is_open_property(), false);
        }

        for (_, handler) in self.closed.snapshot().iter() {
            handler();
        }
    }

    fn listen_for_non_client_click(&self, e: &dyn IRawInputEventArgs) {
        let mouse = e.downcast_ref::<RawPointerEventArgs>();

        if self.is_light_dismiss_enabled()
            && mouse.is_some_and(|mouse| mouse.type_() == RawPointerEventType::NonClientLeftButtonDown)
        {
            self.close_core();
        }
    }

    fn pointer_pressed_dismiss_overlay(&self, e: &PointerPressedEventArgs) {
        if !self.is_light_dismiss_enabled() {
            return;
        }
        let Some(v) = e.source().and_then(|source| source.cast::<Visual>()) else { return };

        if !self.is_child_or_this(&v) {
            if self.overlay_dismiss_event_pass_through() {
                pass_through_event(e);
            }

            // Ensure the popup is closed if it was not closed by a pass-through event handler
            if self.is_open() {
                self.close_core();
            }
        }
    }

    fn target_detached(&self, _e: &VisualTreeAttachmentEventArgs) {
        self.close();
    }

    fn root_template_applied(&self, _e: &TemplateAppliedEventArgs) {
        let Some(open_state) = self.open_state() else { return };

        let popup_host = open_state.popup_host.clone();

        open_state.unsubscribe_template_applied();

        open_state.set_presenter_subscription(None);

        // If the Popup appears in a control template, then the child controls
        // that appear in the popup host need to have their TemplatedParent
        // properties set.
        if self.templated_parent().is_some() {
            if let Some(presenter) = popup_host.presenter() {
                presenter.apply_template();

                let weak = self.to_ref().downgrade();
                let presenter_object: &FerroObject = &presenter;
                let presenter_subscription = FerroObjectExtensions::get_observable(
                    presenter_object,
                    ContentPresenter::child_property(),
                )
                .subscribe_fn(
                    move |control: Option<Ref<Control>>| {
                        if let Some(this) = weak.upgrade() {
                            this.set_templated_parent_and_apply_child_templates(control.as_ref());
                        }
                    },
                );

                open_state.set_presenter_subscription(Some(presenter_subscription));
            }
        }
    }

    fn set_templated_parent_and_apply_child_templates(&self, control: Option<&Ref<Control>>) {
        if let Some(control) = control {
            TemplatedControl::apply_templated_parent(control, self.templated_parent().as_ref());
        }
    }

    fn is_child_or_this(&self, child: &Visual) -> bool {
        let Some(open_state) = self.open_state() else { return false };

        let popup_host: Ref<Visual> = open_state.popup_host.as_control().upcast();

        let mut root = child.visual_root();

        while let Some(current) = root {
            let Some(element) = current.downcast_ref::<InputElement>() else { break };
            let Some(hosted_root) = element.as_hosted_visual_tree_root() else { break };

            if current == popup_host {
                return true;
            }

            root = hosted_root.host().and_then(|host| host.visual_root());
        }

        false
    }

    /// Whether `visual` is inside the open popup.
    pub fn is_inside_popup(&self, visual: &Visual) -> bool {
        let Some(open_state) = self.open_state() else { return false };

        let popup_host = open_state.popup_host.as_control();

        popup_host.is_visual_ancestor_of(visual)
    }

    /// Whether the pointer is over the open popup.
    pub fn is_pointer_over_popup(&self) -> bool {
        self.open_state().is_some_and(|open_state| open_state.popup_host.as_control().is_pointer_over())
    }

    fn window_deactivated(&self) {
        if self.is_light_dismiss_enabled() {
            self.close();
        }
    }

    fn window_base_deactivated(&self) {
        if self.is_light_dismiss_enabled() {
            self.close();
        }
    }

    fn parent_closed(&self) {
        if self.is_light_dismiss_enabled() {
            self.close();
        }
    }

    fn top_level_lost_platform_focus(&self) {
        if self.is_light_dismiss_enabled() {
            self.close();
        }
    }

    fn placement_target_transform_changed(&self, _v: &Ref<Visual>, _matrix: Option<Matrix>) {
        if let Some(open_state) = self.open_state() {
            self.update_host_sizing(&open_state.popup_host, open_state.top_level(), &open_state.placement_target());
        }
    }

    fn window_lost_focus(&self) {
        if self.is_light_dismiss_enabled() {
            self.close();
        }
    }

    fn window_position_changed(&self, _pp: PixelPoint) {
        self.handle_position_change();
    }

    fn placement_target_layout_updated(&self) {
        let Some(open_state) = self.open_state() else { return };

        // A LayoutUpdated event is raised for the whole visual tree:
        // the bounds of the PlacementTarget might not have effectively changed.
        let new_bounds = open_state.placement_target().bounds();
        if new_bounds == open_state.last_placement_target_bounds.get() {
            return;
        }

        open_state.last_placement_target_bounds.set(new_bounds);
        self.update_host_position(&open_state.popup_host, &open_state.placement_target());
    }

    fn parent_popup_position_changed(&self, _e: &PixelPointEventArgs) {
        self.handle_position_change();
    }

    fn begin_ignoring_is_open(&self) -> IgnoreIsOpenScope<'_> {
        self.ignore_is_open_changed.set(true);
        IgnoreIsOpenScope { owner: self }
    }

    /// The popup as a popup host provider (the diagnostics contract it
    /// implements upstream).
    pub fn to_popup_host_provider(&self) -> Rc<dyn IPopupHostProvider> {
        Rc::new(PopupHostProvider(self.to_ref()))
    }

    /// Records a popup opened directly inside this popup (internal
    /// upstream).
    pub fn add_opened_popup(&self, popup: &Ref<Popup>) {
        self.opened_popups.borrow_mut().push(popup.clone());
    }

    /// Forgets a popup opened directly inside this popup (internal
    /// upstream).
    pub fn remove_opened_popup(&self, popup: &Ref<Popup>) {
        let mut opened_popups = self.opened_popups.borrow_mut();
        if let Some(index) = opened_popups.iter().position(|p| p == popup) {
            opened_popups.remove(index);
        }
    }
}

/// The first logical ancestor of `element` that is a `T`.
fn find_logical_ancestor_of_type<T: ObjectType>(element: &StyledElement) -> Option<Ref<T>> {
    let mut parent = element.parent();
    while let Some(current) = parent {
        if let Some(result) = current.cast::<T>() {
            return Some(result);
        }
        parent = current.parent();
    }
    None
}

fn window_manager_add_shadow_hint_changed(host: &Rc<dyn IPopupHost>, hint: bool) {
    if let Some(pr) = host.as_popup_root() {
        pr.set_window_manager_add_shadow_hint(hint);
    }
}

fn pass_through_event(e: &PointerPressedEventArgs) {
    let Some(layer) = e.source().and_then(|source| source.cast::<LightDismissOverlayLayer>()) else { return };
    let Some(root) = layer.visual_root().and_then(|root| root.cast::<InputElement>()) else { return };

    let p = e.get_current_point(Some(&root));
    let layer_visual: Ref<Visual> = layer.upcast();
    let hit = root.input_hit_test_filtered(p.position, &|x: &Visual| !x.to_ref().ptr_eq(&layer_visual), true);

    if let Some(hit) = hit {
        e.pointer().capture(Some(&hit));
        hit.raise_event(e);
        e.set_handled(true);
    }
}

fn find_parent_popup(placement_target: &Visual) -> Option<Ref<Popup>> {
    for visual in placement_target.get_self_and_visual_ancestors() {
        if visual.is::<PopupRoot>() || visual.is::<OverlayPopupHost>() {
            return visual
                .cast::<StyledElement>()
                .and_then(|element| element.parent())
                .and_then(|parent| parent.cast::<Popup>());
        }
    }

    None
}

/// The state of an open popup.
struct PopupOpenState {
    cleanup: Rc<dyn IDisposable>,
    presenter_cleanup: RefCell<Option<Rc<dyn IDisposable>>>,
    template_applied_subscription: Rc<dyn IDisposable>,
    placement_target: RefCell<Ref<Control>>,
    last_placement_target_bounds: Cell<Rect>,
    top_level: WeakRef<TopLevel>,
    parent_popup: Option<WeakRef<Popup>>,
    popup_host: Rc<dyn IPopupHost>,
}

impl PopupOpenState {
    fn new(
        placement_target: Ref<Control>,
        top_level: &Ref<TopLevel>,
        popup_host: Rc<dyn IPopupHost>,
        cleanup: Rc<dyn IDisposable>,
        parent_popup: Option<&Ref<Popup>>,
        template_applied_subscription: Rc<dyn IDisposable>,
    ) -> Self {
        Self {
            cleanup,
            presenter_cleanup: RefCell::new(None),
            template_applied_subscription,
            last_placement_target_bounds: Cell::new(placement_target.bounds()),
            placement_target: RefCell::new(placement_target),
            top_level: top_level.downgrade(),
            parent_popup: parent_popup.map(Ref::downgrade),
            popup_host,
        }
    }

    /// The top-level of the placement target; `None` once it is gone.
    fn top_level(&self) -> Option<Ref<TopLevel>> {
        self.top_level.upgrade()
    }

    fn parent_popup(&self) -> Option<Ref<Popup>> {
        self.parent_popup.as_ref().and_then(WeakRef::upgrade)
    }

    fn placement_target(&self) -> Ref<Control> {
        self.placement_target.borrow().clone()
    }

    fn set_placement_target(&self, value: Ref<Control>) {
        self.last_placement_target_bounds.set(value.bounds());
        *self.placement_target.borrow_mut() = value;
    }

    /// Stops listening for the template of the host being applied.
    fn unsubscribe_template_applied(&self) {
        self.template_applied_subscription.dispose();
    }

    fn set_presenter_subscription(&self, presenter_cleanup: Option<Rc<dyn IDisposable>>) {
        let old = self.presenter_cleanup.replace(presenter_cleanup);
        if let Some(old) = old {
            old.dispose();
        }
    }
}

impl IDisposable for PopupOpenState {
    fn dispose(&self) {
        let presenter_cleanup = self.presenter_cleanup.borrow().clone();
        if let Some(presenter_cleanup) = presenter_cleanup {
            presenter_cleanup.dispose();
        }

        self.cleanup.dispose();
    }
}

/// The popup host provider contract of a popup.
struct PopupHostProvider(Ref<Popup>);

impl IPopupHostProvider for PopupHostProvider {
    fn popup_host(&self) -> Option<Rc<dyn IPopupHost>> {
        self.0.host()
    }

    fn popup_host_changed(&self, handler: Rc<dyn Fn(Option<&Rc<dyn IPopupHost>>)>) -> Rc<dyn IDisposable> {
        self.0.popup_host_changed(move |host| handler(host))
    }
}
