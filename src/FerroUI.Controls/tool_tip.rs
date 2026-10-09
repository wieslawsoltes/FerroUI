use crate::diagnostics::IPopupHostProvider;
use crate::metadata::PseudoClassesAttribute;
use crate::primitives::{CustomPopupPlacementCallbackValue, IPopupHost, Popup, TemplatedControlImpl};
use crate::primitives::popup_positioning::CustomPopupPlacementCallback;
use crate::{ContentControl, ContentControlImpl, Control, ControlImpl, PlacementMode};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    CancelRoutedEventArgs, Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken,
    RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, AnyValue, AttachedProperty, BoxedValue,
    FerroObjectExtensions, FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledPropertyOptions, VisualImpl, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

const PC_OPEN: &str = ":open";

/// A control which pops up a hint when a control is hovered.
///
/// You will probably not want to create a [`ToolTip`] control directly: if
/// added to the tree it will act as a simple [`ContentControl`] styled to
/// look like a tooltip. To add a tooltip to a control, use the `Tip`
/// attached property ([`ToolTip::tip_property`]), assigning the content
/// that you want displayed.
#[repr(C)]
pub struct ToolTip {
    base: ContentControl,
    popup: RefCell<Option<Ref<Popup>>>,
    popup_host_changed_handler: HandlerList<dyn Fn(Option<&Rc<dyn IPopupHost>>)>,
    subscriptions: RefCell<Option<Rc<CompositeDisposable>>>,
    adorned_control: RefCell<Option<WeakRef<Control>>>,
    closed: HandlerList<dyn Fn(&ToolTip)>,
}

ferro_class!(ToolTip: ContentControl);
ferroui_base::ferro_class_info!(ToolTip { new: ToolTip::new });
ferro_impl_classes!(
    ToolTip: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl ControlImpl for ToolTip {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ToolTipAutomationPeer::new(this).upcast()
    }
}

ferroui_base::ferro_impl_classes!(ToolTip: FerroObjectImpl);

impl ToolTip {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[PC_OPEN]);
}

ferroui_base::ferro_properties! { impl ToolTip {
    ferro_property!(
        /// Defines the `ToolTip.Tip` attached property.
        pub fn tip_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<ToolTip, Control, _>("Tip", None)
        }
    );

    ferro_property!(
        /// Defines the `ToolTip.IsOpen` attached property.
        pub fn is_open_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<ToolTip, Control, _>("IsOpen", false)
        }
    );

    ferro_property!(
        /// Defines the `ToolTip.Placement` property.
        pub fn placement_property() -> AttachedProperty<PlacementMode> {
            FerroProperty::register_attached::<ToolTip, Control, _>("Placement", PlacementMode::Pointer)
        }
    );

    ferro_property!(
        /// Defines the `ToolTip.HorizontalOffset` property.
        pub fn horizontal_offset_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<ToolTip, Control, _>("HorizontalOffset", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `ToolTip.VerticalOffset` property.
        pub fn vertical_offset_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<ToolTip, Control, _>("VerticalOffset", 20.0)
        }
    );

    ferro_property!(
        /// Defines the `ToolTip.CustomPopupPlacementCallback` property; see
        /// [`Popup::custom_popup_placement_callback_property`].
        pub fn custom_popup_placement_callback_property() -> AttachedProperty<Option<CustomPopupPlacementCallbackValue>> {
            FerroProperty::register_attached::<ToolTip, Control, _>("CustomPopupPlacementCallback", None)
        }
    );

    ferro_property!(
        /// Defines the `ToolTip.ShowDelay` property.
        pub fn show_delay_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<ToolTip, Control, _>("ShowDelay", 400)
        }
    );

    ferro_property!(
        /// Defines the `ToolTip.BetweenShowDelay` property.
        pub fn between_show_delay_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<ToolTip, Control, _>("BetweenShowDelay", 100)
        }
    );

    ferro_property!(
        /// Defines the `ToolTip.ShowOnDisabled` property.
        pub fn show_on_disabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<ToolTip, Control, _>(
                "ShowOnDisabled",
                StyledPropertyOptions::new(false).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `ToolTip.ServiceEnabled` property.
        pub fn service_enabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<ToolTip, Control, _>(
                "ServiceEnabled",
                StyledPropertyOptions::new(true).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `ToolTip.ShouldUseOverlayLayer` property; see
        /// [`Popup::should_use_overlay_layer`].
        pub fn should_use_overlay_layer_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<ToolTip, Control, _>("ShouldUseOverlayLayer", false)
        }
    );

    ferro_property!(
        /// Stores the current [`ToolTip`] instance in the control (internal
        /// upstream).
        pub fn tool_tip_property() -> AttachedProperty<Option<Ref<ToolTip>>> {
            FerroProperty::register_attached::<ToolTip, Control, _>("ToolTip", None)
        }
    );
} }

impl ToolTip {
    ferro_routed_event!(
        /// The event raised when a tooltip is going to be shown on an
        /// element.
        ///
        /// To prevent a tooltip from appearing in the UI, your handler for
        /// `ToolTipOpening` can mark the event data as cancelled. Otherwise,
        /// the tooltip is displayed, using the value of the `Tip` property
        /// as the tooltip content. Another possible scenario is that you
        /// could write a handler that resets the value of the `Tip`
        /// property for the element that is the event source, just before
        /// the tooltip is displayed. `ToolTipOpening` will not be raised if
        /// the value of `Tip` is `None` or otherwise unset. Do not
        /// deliberately set `Tip` to `None` while a tooltip is open or
        /// opening; this will not have the effect of closing the tooltip,
        /// and will instead create an undesirable visual artifact in the UI.
        pub fn tool_tip_opening_event() -> RoutedEvent<CancelRoutedEventArgs> {
            RoutedEvent::register::<ToolTip, CancelRoutedEventArgs>("ToolTipOpening", RoutingStrategies::DIRECT)
        }
    );

    ferro_routed_event!(
        /// The event raised when a tooltip on an element that was shown
        /// should now be hidden.
        ///
        /// Marking the `ToolTipClosing` event as handled does not cancel
        /// closing the tooltip. Once the tooltip is displayed, closing the
        /// tooltip is done only in response to user interaction with the UI.
        pub fn tool_tip_closing_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<ToolTip, RoutedEventArgs>("ToolTipClosing", RoutingStrategies::DIRECT)
        }
    );

    /// Initializes static members of the [`ToolTip`] class.
    fn static_constructor() {
        // The subscription lives as long as the property.
        let _ = Self::is_open_property().changed().subscribe(Self::is_open_changed);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            popup: RefCell::new(None),
            popup_host_changed_handler: HandlerList::new(),
            subscriptions: RefCell::new(None),
            adorned_control: RefCell::new(None),
            closed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the value of the `ToolTip.Tip` attached property: the content
    /// to be displayed in the tooltip of `element`.
    pub fn get_tip(element: &Control) -> Option<BoxedValue> {
        element.get_value(Self::tip_property())
    }

    /// Sets the value of the `ToolTip.Tip` attached property: the content
    /// to be displayed in the tooltip of `element`.
    pub fn set_tip(element: &Control, value: Option<BoxedValue>) {
        element.set_value(Self::tip_property(), value)
    }

    /// Gets the value of the `ToolTip.IsOpen` attached property: whether
    /// the tooltip of `element` is visible.
    pub fn get_is_open(element: &Control) -> bool {
        element.get_value(Self::is_open_property())
    }

    /// Sets the value of the `ToolTip.IsOpen` attached property: whether
    /// the tooltip of `element` is visible.
    pub fn set_is_open(element: &Control, value: bool) {
        element.set_value(Self::is_open_property(), value)
    }

    /// Gets the value of the `ToolTip.Placement` attached property: how the
    /// tooltip of `element` is positioned.
    pub fn get_placement(element: &Control) -> PlacementMode {
        element.get_value(Self::placement_property())
    }

    /// Sets the value of the `ToolTip.Placement` attached property: how the
    /// tooltip of `element` is positioned.
    pub fn set_placement(element: &Control, value: PlacementMode) {
        element.set_value(Self::placement_property(), value)
    }

    /// Gets the value of the `ToolTip.HorizontalOffset` attached property.
    pub fn get_horizontal_offset(element: &Control) -> f64 {
        element.get_value(Self::horizontal_offset_property())
    }

    /// Sets the value of the `ToolTip.HorizontalOffset` attached property.
    pub fn set_horizontal_offset(element: &Control, value: f64) {
        element.set_value(Self::horizontal_offset_property(), value)
    }

    /// Gets the value of the `ToolTip.VerticalOffset` attached property.
    pub fn get_vertical_offset(element: &Control) -> f64 {
        element.get_value(Self::vertical_offset_property())
    }

    /// Sets the value of the `ToolTip.VerticalOffset` attached property.
    pub fn set_vertical_offset(element: &Control, value: f64) {
        element.set_value(Self::vertical_offset_property(), value)
    }

    /// Gets the value of the `ToolTip.ShowDelay` attached property: the
    /// time, in milliseconds, before the tooltip of `element` opens.
    pub fn get_show_delay(element: &Control) -> i32 {
        element.get_value(Self::show_delay_property())
    }

    /// Sets the value of the `ToolTip.ShowDelay` attached property: the
    /// time, in milliseconds, before the tooltip of `element` opens.
    pub fn set_show_delay(element: &Control, value: i32) {
        element.set_value(Self::show_delay_property(), value)
    }

    /// Gets the number of milliseconds since the last tooltip closed during
    /// which the tooltip of `element` will open immediately, or a negative
    /// value indicating that the tooltip will always wait for the show
    /// delay before opening.
    pub fn get_between_show_delay(element: &Control) -> i32 {
        element.get_value(Self::between_show_delay_property())
    }

    /// Sets the number of milliseconds since the last tooltip closed during
    /// which the tooltip of `element` will open immediately.
    ///
    /// Setting a negative value disables the immediate opening behaviour.
    /// The tooltip of `element` will then always wait until the show delay
    /// elapses before showing.
    pub fn set_between_show_delay(element: &Control, value: i32) {
        element.set_value(Self::between_show_delay_property(), value)
    }

    /// Gets whether a control will display a tooltip even if it disabled.
    pub fn get_show_on_disabled(element: &Control) -> bool {
        element.get_value(Self::show_on_disabled_property())
    }

    /// Sets whether a control will display a tooltip even if it disabled.
    pub fn set_show_on_disabled(element: &Control, value: bool) {
        element.set_value(Self::show_on_disabled_property(), value)
    }

    /// Gets whether showing and hiding of a control's tooltip will be
    /// automatically controlled by the tooltip service.
    pub fn get_service_enabled(element: &Control) -> bool {
        element.get_value(Self::service_enabled_property())
    }

    /// Sets whether showing and hiding of a control's tooltip will be
    /// automatically controlled by the tooltip service.
    pub fn set_service_enabled(element: &Control, value: bool) {
        element.set_value(Self::service_enabled_property(), value)
    }

    /// Gets whether the tooltip popup for `element` is shown in the overlay
    /// layer. See [`Popup::should_use_overlay_layer`] for details.
    pub fn get_should_use_overlay_layer(element: &Control) -> bool {
        element.get_value(Self::should_use_overlay_layer_property())
    }

    /// Sets whether the tooltip popup for `element` is shown in the overlay
    /// layer. See [`Popup::should_use_overlay_layer`] for details.
    pub fn set_should_use_overlay_layer(element: &Control, value: bool) {
        element.set_value(Self::should_use_overlay_layer_property(), value)
    }

    /// Adds a handler for the `ToolTipOpening` attached event to `element`.
    pub fn add_tool_tip_opening_handler(
        element: &Control,
        handler: impl Fn(&Interactive, &CancelRoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        element.add_handler(Self::tool_tip_opening_event(), handler)
    }

    /// Removes a handler for the `ToolTipOpening` attached event from
    /// `element`.
    pub fn remove_tool_tip_opening_handler(element: &Control, handler: RoutedEventHandlerToken) {
        element.remove_handler(Self::tool_tip_opening_event(), handler);
    }

    /// Adds a handler for the `ToolTipClosing` attached event to `element`.
    pub fn add_tool_tip_closing_handler(
        element: &Control,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        element.add_handler(Self::tool_tip_closing_event(), handler)
    }

    /// Removes a handler for the `ToolTipClosing` attached event from
    /// `element`.
    pub fn remove_tool_tip_closing_handler(element: &Control, handler: RoutedEventHandlerToken) {
        element.remove_handler(Self::tool_tip_closing_event(), handler);
    }

    /// Gets the value of the `ToolTip.CustomPopupPlacementCallback`
    /// attached property.
    pub fn get_custom_popup_placement_callback(element: &Control) -> Option<CustomPopupPlacementCallback> {
        element.get_value(Self::custom_popup_placement_callback_property()).map(|value| value.0)
    }

    /// Sets the value of the `ToolTip.CustomPopupPlacementCallback`
    /// attached property.
    pub fn set_custom_popup_placement_callback(element: &Control, value: Option<CustomPopupPlacementCallback>) {
        element.set_value(
            Self::custom_popup_placement_callback_property(),
            value.map(CustomPopupPlacementCallbackValue),
        )
    }

    fn is_open_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        let control = e.sender().downcast_ref::<Control>().expect("the tooltip properties are attached to controls");
        let new_value = e.get_new_value::<bool>();

        if new_value {
            let args = CancelRoutedEventArgs::with_event(Self::tool_tip_opening_event());
            control.raise_event(&args);
            if args.cancel() {
                control.set_current_value(Self::is_open_property(), false);
                return;
            }

            let Some(tip) = Self::get_tip(control) else {
                control.set_current_value(Self::is_open_property(), false);
                return;
            };

            let tip_tool_tip = tool_tip_from_boxed(&tip);
            let mut tool_tip = control.get_value(Self::tool_tip_property());
            let is_current = tool_tip.as_ref().is_some_and(|tool_tip| {
                tip_tool_tip.as_ref() == Some(tool_tip)
                    || tool_tip.content().is_some_and(|content| same_value(&content, &tip))
            });
            if !is_current {
                if let Some(tool_tip) = &tool_tip {
                    tool_tip.close();
                }

                let new_tool_tip = tip_tool_tip.unwrap_or_else(|| {
                    let tool_tip = ToolTip::new();
                    tool_tip.set_content(Some(tip));
                    tool_tip
                });
                control.set_value(Self::tool_tip_property(), Some(new_tool_tip.clone()));
                tool_tip = Some(new_tool_tip);
            }

            let tool_tip = tool_tip.expect("the tooltip was just created");
            let control = control.to_ref();
            *tool_tip.adorned_control.borrow_mut() = Some(control.downgrade());
            tool_tip.open(&control);
        } else if let Some(tool_tip) = control.get_value(Self::tool_tip_property()) {
            *tool_tip.adorned_control.borrow_mut() = None;
            tool_tip.close();
        }
    }

    /// The control this tooltip is shown for (internal upstream).
    pub fn adorned_control(&self) -> Option<Ref<Control>> {
        self.adorned_control.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Raised when the popup of the tooltip has closed (internal upstream).
    /// Disposing the returned handle unsubscribes.
    pub fn closed(&self, handler: impl Fn(&ToolTip) + 'static) -> Rc<dyn IDisposable> {
        let token = self.closed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.closed.remove(token);
            }
        })
    }

    /// The host of the popup the tooltip is shown in, while it is open.
    pub fn popup_host(&self) -> Option<Rc<dyn IPopupHost>> {
        let popup = self.popup.borrow().clone();
        popup.and_then(|popup| popup.host())
    }

    /// Raised when the popup host changes (the popup host provider
    /// contract). Disposing the returned handle unsubscribes.
    pub fn popup_host_changed(
        &self,
        handler: impl Fn(Option<&Rc<dyn IPopupHost>>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.popup_host_changed_handler.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.popup_host_changed_handler.remove(token);
            }
        })
    }

    /// The tooltip as a popup host provider (the diagnostics contract it
    /// implements upstream).
    pub fn to_popup_host_provider(&self) -> Rc<dyn IPopupHostProvider> {
        Rc::new(ToolTipPopupHostProvider(self.to_ref()))
    }

    fn open(&self, control: &Ref<Control>) {
        self.close();

        let existing = self.popup.borrow().clone();
        let popup = match existing {
            Some(popup) => popup,
            None => {
                let popup = Popup::new();
                popup.set_takes_focus_from_native_control(false);
                popup.set_window_manager_add_shadow_hint(false);

                // The popup belongs to the tooltip, which unsubscribes by
                // letting go of it.
                let weak = self.to_ref().downgrade();
                let _ = popup.opened({
                    let weak = weak.clone();
                    let weak_popup = popup.downgrade();
                    move || {
                        if let (Some(this), Some(popup)) = (weak.upgrade(), weak_popup.upgrade()) {
                            this.on_popup_opened(&popup);
                        }
                    }
                });
                let _ = popup.closed(move || {
                    if let Some(this) = weak.upgrade() {
                        this.on_popup_closed();
                    }
                });

                *self.popup.borrow_mut() = Some(popup.clone());
                popup
            }
        };

        // The popup holds the tooltip only while it is open: a tooltip and
        // its popup must not keep each other alive.
        popup.set_child(self.to_ref());

        let subscriptions = CompositeDisposable::from_disposables([
            popup.bind_typed_value(
                Popup::horizontal_offset_property(),
                control.get_binding_observable(Self::horizontal_offset_property()),
                BindingPriority::LocalValue,
            ),
            popup.bind_typed_value(
                Popup::vertical_offset_property(),
                control.get_binding_observable(Self::vertical_offset_property()),
                BindingPriority::LocalValue,
            ),
            popup.bind_typed_value(
                Popup::placement_property(),
                control.get_binding_observable(Self::placement_property()),
                BindingPriority::LocalValue,
            ),
            popup.bind_typed_value(
                Popup::custom_popup_placement_callback_property(),
                control.get_binding_observable(Self::custom_popup_placement_callback_property()),
                BindingPriority::LocalValue,
            ),
            popup.bind_typed_value(
                Popup::should_use_overlay_layer_property(),
                control.get_binding_observable(Self::should_use_overlay_layer_property()),
                BindingPriority::LocalValue,
            ),
        ]);
        *self.subscriptions.borrow_mut() = Some(Rc::new(subscriptions));

        popup.set_placement_target(control);
        popup.set_popup_parent(Some(control));

        popup.set_is_open(true);
    }

    fn close(&self) {
        if let Some(adorned_control) = self.adorned_control() {
            if Self::get_is_open(&adorned_control) {
                let args = RoutedEventArgs::with_event(Self::tool_tip_closing_event());
                adorned_control.raise_event(&args);
            }
        }

        let popup = self.popup.borrow().clone();
        if let Some(popup) = popup {
            popup.set_is_open(false);
            popup.set_popup_parent(None);
            popup.set_placement_target(None::<Ref<Control>>);
            if !popup.is_open() {
                popup.set_child(None::<Ref<Control>>);
            }
        }

        let subscriptions = self.subscriptions.borrow().clone();
        if let Some(subscriptions) = subscriptions {
            subscriptions.dispose();
        }
    }

    fn on_popup_closed(&self) {
        // This condition is true, when Popup was closed by any other reason outside of ToolTipService/ToolTip, keeping IsOpen=true.
        if let Some(adorned_control) = self.adorned_control() {
            if Self::get_is_open(&adorned_control) {
                adorned_control.set_current_value(Self::is_open_property(), false);
            }
        }

        for (_, handler) in self.popup_host_changed_handler.snapshot().iter() {
            handler(None);
        }
        self.update_pseudo_classes(false);
        for (_, handler) in self.closed.snapshot().iter() {
            handler(self);
        }
    }

    fn on_popup_opened(&self, popup: &Popup) {
        let host = popup.host();
        for (_, handler) in self.popup_host_changed_handler.snapshot().iter() {
            handler(host.as_ref());
        }
        self.update_pseudo_classes(true);
    }

    fn update_pseudo_classes(&self, new_value: bool) {
        self.pseudo_classes().set(PC_OPEN, new_value);
    }
}

/// The tooltip held by the untyped value of the `Tip` property, if it holds
/// one: a tooltip boxed as a control (see [`Control::boxed`]) or as itself.
fn tool_tip_from_boxed(value: &BoxedValue) -> Option<Ref<ToolTip>> {
    if let Some(control) = Control::from_boxed(value) {
        return control.cast::<ToolTip>();
    }
    let value: &dyn AnyValue = &**value;
    value.downcast_ref::<Ref<ToolTip>>().cloned()
}

/// Whether two untyped values are the same value: the same box, or boxes of
/// equal values.
pub(crate) fn same_value(a: &BoxedValue, b: &BoxedValue) -> bool {
    Rc::ptr_eq(a, b) || **a == **b
}

/// Whether the untyped value of the `Tip` property holds a tooltip.
pub(crate) fn is_tool_tip(value: &BoxedValue) -> bool {
    tool_tip_from_boxed(value).is_some()
}

/// The popup host provider contract of a tooltip.
struct ToolTipPopupHostProvider(Ref<ToolTip>);

impl IPopupHostProvider for ToolTipPopupHostProvider {
    fn popup_host(&self) -> Option<Rc<dyn IPopupHost>> {
        self.0.popup_host()
    }

    fn popup_host_changed(&self, handler: Rc<dyn Fn(Option<&Rc<dyn IPopupHost>>)>) -> Rc<dyn IDisposable> {
        self.0.popup_host_changed(move |host| handler(host))
    }
}
