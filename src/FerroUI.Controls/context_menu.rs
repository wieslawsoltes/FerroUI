use crate::diagnostics::IPopupHostProvider;
use crate::platform::{DefaultMenuInteractionHandler, FeedbackAction, IMenuInteractionHandler, PlatformFeedbackExtensions};
use crate::primitives::popup_positioning::{
    CustomPopupPlacementCallback, PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment,
};
use crate::primitives::{CustomPopupPlacementCallbackValue, IPopupHost, Popup, SelectingItemsControlImpl, TemplatedControlImpl};
use crate::{Application, Control, ControlImpl, ItemsControlImpl, MenuBase, MenuBaseImpl, MenuItem, PlacementMode};
use ferroui_base::input::{ContextRequestedEventArgs, FocusManager, InputElement, InputElementImpl, KeyEventArgs};
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::{ISetterValue, Setter};
use ferroui_base::utilities::{CancelEventArgs, HandlerList};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, ElementRef, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Nullable, Rect, Ref, StyledElement, StyledElementImpl,
    StyledProperty, VisualImpl, VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A control context menu.
#[repr(C)]
pub struct ContextMenu {
    base: MenuBase,
    popup: RefCell<Option<Ref<Popup>>>,
    /// The controls the menu is attached to. The controls own the menu (it
    /// is the value of their `ContextMenu` property), so the references
    /// back are weak.
    attached_controls: RefCell<Option<Vec<WeakRef<Control>>>>,
    /// The element that had the focus when the popup opened. Only written,
    /// as in the reference; held weakly since it is not owned by the menu.
    previous_focus: RefCell<Option<WeakRef<InputElement>>>,
    popup_host_changed_handler: HandlerList<dyn Fn(Option<&Rc<dyn IPopupHost>>)>,
    opening: HandlerList<dyn Fn(&CancelEventArgs)>,
    closing: HandlerList<dyn Fn(&CancelEventArgs)>,
}

ferro_class!(ContextMenu: MenuBase);
ferro_class_info!(ContextMenu { new: ContextMenu::new });

ferro_impl_classes!(
    ContextMenu: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ItemsControlImpl,
    SelectingItemsControlImpl
);

impl FerroObjectImpl for ContextMenu {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::window_manager_add_shadow_hint_property().as_property() {
            let popup = this.popup.borrow().clone();
            if let Some(popup) = popup {
                popup.set_window_manager_add_shadow_hint(change.get_new_value::<bool>());
            }
        }
    }
}

impl MenuBaseImpl for ContextMenu {
    /// Opens the menu.
    fn open(this: &Self) {
        this.open_at(None);
    }

    /// Closes the menu.
    fn close(this: &Self) {
        if !this.is_open() {
            return;
        }

        let popup = this.popup.borrow().clone();
        if let Some(popup) = popup {
            if popup.is_visible() {
                popup.set_is_open(false);
            }
        }
    }
}

impl ISetterValue for ContextMenu {
    fn initialize(&self, setter: &Setter) {
        // ContextMenu can be assigned to the ContextMenu property in a setter. This overrides
        // the behavior defined in Control which requires controls to be wrapped in a <template>.
        if setter.property() != Some(Control::context_menu_property().as_property()) {
            panic!("Cannot use a control as a Setter value. Wrap the control in a <Template>.");
        }
    }
}

ferro_properties! {
    impl ContextMenu {
        /// Defines the `HorizontalOffset` property.
        pub fn horizontal_offset_property() -> StyledProperty<f64> {
            Popup::horizontal_offset_property().add_owner::<ContextMenu>()
        }

        /// Defines the `VerticalOffset` property.
        pub fn vertical_offset_property() -> StyledProperty<f64> {
            Popup::vertical_offset_property().add_owner::<ContextMenu>()
        }

        /// Defines the `PlacementAnchor` property.
        pub fn placement_anchor_property() -> StyledProperty<PopupAnchor> {
            Popup::placement_anchor_property().add_owner::<ContextMenu>()
        }

        /// Defines the `PlacementConstraintAdjustment` property.
        pub fn placement_constraint_adjustment_property() -> StyledProperty<PopupPositionerConstraintAdjustment> {
            Popup::placement_constraint_adjustment_property().add_owner::<ContextMenu>()
        }

        /// Defines the `PlacementGravity` property.
        pub fn placement_gravity_property() -> StyledProperty<PopupGravity> {
            Popup::placement_gravity_property().add_owner::<ContextMenu>()
        }

        /// Defines the `Placement` property.
        pub fn placement_property() -> StyledProperty<PlacementMode> {
            Popup::placement_property().add_owner::<ContextMenu>()
        }

        /// Defines the `PlacementRect` property.
        pub fn placement_rect_property() -> StyledProperty<Option<Rect>> {
            Popup::placement_rect_property().add_owner::<ContextMenu>()
        }

        /// Defines the `WindowManagerAddShadowHint` property.
        pub fn window_manager_add_shadow_hint_property() -> StyledProperty<bool> {
            Popup::window_manager_add_shadow_hint_property().add_owner::<ContextMenu>()
        }

        /// Defines the `PlacementTarget` property.
        ///
        /// The menu does not own its placement target, so the value is an
        /// element reference.
        pub fn placement_target_property() -> StyledProperty<Option<ElementRef<Control>>> {
            Popup::placement_target_property().add_owner::<ContextMenu>()
        }

        /// Defines the `CustomPopupPlacementCallback` property.
        pub fn custom_popup_placement_callback_property() -> StyledProperty<Option<CustomPopupPlacementCallbackValue>> {
            Popup::custom_popup_placement_callback_property().add_owner::<ContextMenu>()
        }
    }
}

impl ContextMenu {
    /// Initializes static members of the [`ContextMenu`] class.
    fn static_constructor() {
        Self::placement_property().override_default_value::<ContextMenu>(PlacementMode::Pointer);
        let _ = Control::context_menu_property().changed().subscribe(Self::context_menu_changed);
        crate::automation::AutomationProperties::accessibility_view_property()
            .override_default_value::<ContextMenu>(crate::automation::AccessibilityView::Control);
        crate::automation::AutomationProperties::control_type_override_property()
            .override_default_value::<ContextMenu>(Some(crate::automation::peers::AutomationControlType::Menu));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self::construct_with(DefaultMenuInteractionHandler::new(true))
    }

    /// Creates the class data of a context menu that uses
    /// `interaction_handler`.
    pub fn construct_with(interaction_handler: Rc<dyn IMenuInteractionHandler>) -> Self {
        Self {
            base: MenuBase::construct_with(interaction_handler),
            popup: RefCell::new(None),
            attached_controls: RefCell::new(None),
            previous_focus: RefCell::new(None),
            popup_host_changed_handler: HandlerList::new(),
            opening: HandlerList::new(),
            closing: HandlerList::new(),
        }
    }

    /// Initializes a new instance of the [`ContextMenu`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Initializes a new instance of the [`ContextMenu`] class that uses
    /// `interaction_handler`.
    pub fn with_interaction_handler(interaction_handler: Rc<dyn IMenuInteractionHandler>) -> Ref<Self> {
        instantiate(Self::construct_with(interaction_handler))
    }

    /// The horizontal offset of the menu in relation to its target; see
    /// [`Popup::horizontal_offset`].
    pub fn horizontal_offset(&self) -> f64 {
        self.get_value(Self::horizontal_offset_property())
    }

    pub fn set_horizontal_offset(&self, value: f64) {
        self.set_value(Self::horizontal_offset_property(), value)
    }

    /// The vertical offset of the menu in relation to its target; see
    /// [`Popup::vertical_offset`].
    pub fn vertical_offset(&self) -> f64 {
        self.get_value(Self::vertical_offset_property())
    }

    pub fn set_vertical_offset(&self, value: f64) {
        self.set_value(Self::vertical_offset_property(), value)
    }

    /// See [`Popup::placement_anchor`].
    pub fn placement_anchor(&self) -> PopupAnchor {
        self.get_value(Self::placement_anchor_property())
    }

    pub fn set_placement_anchor(&self, value: PopupAnchor) {
        self.set_value(Self::placement_anchor_property(), value)
    }

    /// See [`Popup::placement_constraint_adjustment`].
    pub fn placement_constraint_adjustment(&self) -> PopupPositionerConstraintAdjustment {
        self.get_value(Self::placement_constraint_adjustment_property())
    }

    pub fn set_placement_constraint_adjustment(&self, value: PopupPositionerConstraintAdjustment) {
        self.set_value(Self::placement_constraint_adjustment_property(), value)
    }

    /// See [`Popup::placement_gravity`].
    pub fn placement_gravity(&self) -> PopupGravity {
        self.get_value(Self::placement_gravity_property())
    }

    pub fn set_placement_gravity(&self, value: PopupGravity) {
        self.set_value(Self::placement_gravity_property(), value)
    }

    /// See [`Popup::placement`].
    pub fn placement(&self) -> PlacementMode {
        self.get_value(Self::placement_property())
    }

    pub fn set_placement(&self, value: PlacementMode) {
        self.set_value(Self::placement_property(), value)
    }

    /// See [`Popup::window_manager_add_shadow_hint`].
    pub fn window_manager_add_shadow_hint(&self) -> bool {
        self.get_value(Self::window_manager_add_shadow_hint_property())
    }

    pub fn set_window_manager_add_shadow_hint(&self, value: bool) {
        self.set_value(Self::window_manager_add_shadow_hint_property(), value)
    }

    /// See [`Popup::placement_rect`].
    pub fn placement_rect(&self) -> Option<Rect> {
        self.get_value(Self::placement_rect_property())
    }

    pub fn set_placement_rect(&self, value: Option<Rect>) {
        self.set_value(Self::placement_rect_property(), value)
    }

    /// See [`Popup::placement_target`].
    pub fn placement_target(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::placement_target_property()))
    }

    pub fn set_placement_target(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::placement_target_property(), ElementRef::from_nullable(value.into().0))
    }

    /// See [`Popup::custom_popup_placement_callback`].
    pub fn custom_popup_placement_callback(&self) -> Option<CustomPopupPlacementCallback> {
        self.get_value(Self::custom_popup_placement_callback_property()).map(|value| value.0)
    }

    pub fn set_custom_popup_placement_callback(&self, value: Option<CustomPopupPlacementCallback>) {
        self.set_value(
            Self::custom_popup_placement_callback_property(),
            value.map(CustomPopupPlacementCallbackValue),
        )
    }

    fn subscribe<F: ?Sized + 'static>(
        &self,
        select: fn(&ContextMenu) -> &HandlerList<F>,
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

    /// Occurs when the value of the `IsOpen` property is changing from
    /// false to true. Disposing the returned handle unsubscribes.
    pub fn opening(&self, handler: impl Fn(&CancelEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&CancelEventArgs)>(|menu| &menu.opening, Rc::new(handler))
    }

    /// Occurs when the value of the `IsOpen` property is changing from
    /// true to false. Disposing the returned handle unsubscribes.
    pub fn closing(&self, handler: impl Fn(&CancelEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&CancelEventArgs)>(|menu| &menu.closing, Rc::new(handler))
    }

    /// The controls the menu is attached to that are still alive, or
    /// `None` when the list was never created.
    fn attached_controls(&self) -> Option<Vec<Ref<Control>>> {
        self.attached_controls.borrow().as_ref().map(|controls| controls.iter().filter_map(WeakRef::upgrade).collect())
    }

    /// Removes the first occurrence of `control` from the attached
    /// controls.
    fn remove_attached_control(&self, control: &Ref<Control>) {
        if let Some(controls) = self.attached_controls.borrow_mut().as_mut() {
            // Controls that are gone are dropped from the list on the way.
            controls.retain(|c| c.upgrade().is_some());
            if let Some(index) = controls.iter().position(|c| c.upgrade().as_ref() == Some(control)) {
                controls.remove(index);
            }
        }
    }

    /// Called when the `ContextMenu` property changes on a control.
    fn context_menu_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        let control = e.sender().downcast_ref::<Control>().expect("the sender is a control");
        let (old_value, new_value) = e.get_old_and_new_value::<Option<Ref<ContextMenu>>>();

        if let Some(old_menu) = old_value {
            if let Some((context_requested, context_canceled, attached, detached)) =
                control.context_menu_handlers().take()
            {
                control.remove_handler(InputElement::context_requested_event(), context_requested);
                control.remove_handler(InputElement::context_canceled_event(), context_canceled);
                attached.dispose();
                detached.dispose();
            }
            old_menu.remove_attached_control(&control.to_ref());
            let popup = old_menu.popup.borrow().clone();
            if let Some(popup) = popup {
                popup.set_parent(None);
            }
        }

        if new_value.is_some() {
            let context_requested =
                control.add_handler(InputElement::context_requested_event(), Self::control_context_requested);
            let context_canceled =
                control.add_handler(InputElement::context_canceled_event(), Self::control_context_canceled);
            let weak = control.to_ref().downgrade();
            let attached = control.attached_to_visual_tree({
                let weak = weak.clone();
                move |e| {
                    if let Some(control) = weak.upgrade() {
                        Self::control_on_attached_to_visual_tree(&control, e);
                    }
                }
            });
            let detached = control.detached_from_visual_tree(move |e| {
                if let Some(control) = weak.upgrade() {
                    Self::control_detached_from_visual_tree(&control, e);
                }
            });
            control.context_menu_handlers().set(Some((context_requested, context_canceled, attached, detached)));
        }

        if control.is_attached_to_visual_tree() {
            Self::attach_control_to_context_menu(control);
        }
    }

    /// Opens a context menu on the specified control.
    ///
    /// # Panics
    ///
    /// Panics when `control` is `None` and the menu is attached to no
    /// control, and when `control` is not one of the controls the menu is
    /// attached to.
    pub fn open_at(&self, control: Option<&Ref<Control>>) {
        let attached_controls = self.attached_controls();

        if control.is_none() && attached_controls.as_ref().is_none_or(|controls| controls.is_empty()) {
            panic!("Value cannot be null. (Parameter 'control')");
        }

        if let (Some(control), Some(attached_controls)) = (control, &attached_controls) {
            if !attached_controls.contains(control) {
                panic!(
                    "Cannot show ContentMenu on a different control to the one it is attached to. (Parameter 'control')"
                );
            }
        }

        let control = match control {
            Some(control) => control.clone(),
            None => attached_controls.expect("checked above")[0].clone(),
        };
        let placement_target = self.placement_target().unwrap_or_else(|| control.clone());
        self.open_core(&control, &placement_target, self.placement());
    }

    /// The host of the popup the menu is shown in, while it is open.
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
        self.subscribe::<dyn Fn(Option<&Rc<dyn IPopupHost>>)>(|menu| &menu.popup_host_changed_handler, Rc::new(handler))
    }

    /// The menu as a popup host provider (the diagnostics contract it
    /// implements in the reference).
    pub fn to_popup_host_provider(&self) -> Rc<dyn IPopupHostProvider> {
        Rc::new(ContextMenuHostProvider(self.to_ref()))
    }

    fn open_core(&self, control: &Ref<Control>, placement_target: &Ref<Control>, placement: PlacementMode) {
        if self.is_open() {
            return;
        }

        let existing = self.popup.borrow().clone();
        let popup = match existing {
            Some(popup) => popup,
            None => {
                let popup = Popup::new();
                popup.set_is_light_dismiss_enabled(true);
                popup.set_overlay_dismiss_event_pass_through(true);
                popup.set_takes_focus_from_native_control(Popup::get_takes_focus_from_native_control(self));

                // The popup belongs to the menu and lives as long as it:
                // the handlers are never removed.
                let weak = self.to_ref().downgrade();
                let _ = popup.opened({
                    let weak = weak.clone();
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.popup_opened();
                        }
                    }
                });
                let _ = popup.closed({
                    let weak = weak.clone();
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.popup_closed();
                        }
                    }
                });
                let _ = popup.closing({
                    let weak = weak.clone();
                    move |e| {
                        if let Some(this) = weak.upgrade() {
                            this.popup_closing(e);
                        }
                    }
                });
                popup.add_handler(InputElement::key_up_event(), move |_, e: &KeyEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.popup_key_up(e);
                    }
                });

                *self.popup.borrow_mut() = Some(popup.clone());
                popup
            }
        };

        popup.set_popup_parent(Some(control));

        popup.set_placement(placement);

        // Position of the line below is really important.
        // All styles are being applied only when control has logical parent.
        // Line below will add ContextMenu as child to the Popup and this will trigger styles and they would be applied.
        // If you will move line below somewhere else it may cause that ContextMenu will behave differently from what you are expecting.
        popup.set_child(self.to_ref());
        popup.set_placement_target(placement_target);
        popup.set_horizontal_offset(self.horizontal_offset());
        popup.set_vertical_offset(self.vertical_offset());
        popup.set_placement_anchor(self.placement_anchor());
        popup.set_placement_constraint_adjustment(self.placement_constraint_adjustment());
        popup.set_placement_gravity(self.placement_gravity());
        popup.set_placement_rect(self.placement_rect());
        popup.set_custom_popup_placement_callback(self.custom_popup_placement_callback());
        popup.set_window_manager_add_shadow_hint(self.window_manager_add_shadow_hint());
        self.set_is_open(true);
        popup.set_is_open(true);

        self.raise_event(&RoutedEventArgs::with_event_and_source(MenuBase::opened_event(), self.to_ref()));
    }

    fn popup_opened(&self) {
        let previous_focus = FocusManager::get_focus_manager(self).and_then(|manager| manager.get_focused_element());
        *self.previous_focus.borrow_mut() = previous_focus.map(|element| element.downgrade());
        self.focus();

        let host = self.popup_host();
        for (_, handler) in self.popup_host_changed_handler.snapshot().iter() {
            handler(host.as_ref());
        }
    }

    fn popup_closing(&self, e: &CancelEventArgs) {
        e.set_cancel(self.cancel_closing());
    }

    fn popup_closed(&self) {
        let children = StyledElement::logical_children(self).to_vec();
        for i in children {
            if let Some(menu_item) = i.cast::<MenuItem>() {
                menu_item.set_is_sub_menu_open(false);
            }
        }

        self.set_selected_index(-1);
        self.set_is_open(false);

        let popup = self.popup.borrow().clone();

        if self.attached_controls().is_none_or(|controls| controls.is_empty()) {
            if let Some(popup) = &popup {
                popup.set_popup_parent(None);
            }
        }

        self.raise_event(&RoutedEventArgs::with_event_and_source(MenuBase::closed_event(), self.to_ref()));

        for (_, handler) in self.popup_host_changed_handler.snapshot().iter() {
            handler(None);
        }

        // Not in the reference, where the popup keeps the menu as its
        // child: the menu owns the popup, so the popup holds the menu only
        // while it is open (a menu and its popup must not keep each other
        // alive). A handler above may have opened the menu again.
        if let Some(popup) = popup {
            if !popup.is_open() && !self.is_open() {
                popup.set_child(None::<Ref<Control>>);
            }
        }
    }

    fn popup_key_up(&self, e: &KeyEventArgs) {
        if self.is_open() {
            let keymap = Application::current()
                .and_then(|application| application.platform_settings())
                .map(|settings| settings.hotkey_configuration());

            if keymap.is_some_and(|keymap| keymap.open_context_menu.iter().any(|k| k.matches(Some(e))))
                && !self.cancel_closing()
            {
                self.close();
                e.set_handled(true);
            }
        }
    }

    fn control_context_canceled(sender: &Interactive, e: &RoutedEventArgs) {
        if e.handled() {
            return;
        }
        let Some(control) = sender.downcast_ref::<Control>() else { return };
        let Some(context_menu) = control.context_menu() else { return };

        if context_menu.is_open() {
            context_menu.close();
        }
    }

    fn control_context_requested(sender: &Interactive, e: &ContextRequestedEventArgs) {
        let Some(control) = sender.downcast_ref::<Control>() else { return };
        let Some(context_menu) = control.context_menu() else { return };

        if !e.handled() && !context_menu.cancel_opening() {
            let requested_by_pointer = e.try_get_position(None).is_some();
            let control = control.to_ref();
            let placement_target =
                e.source().and_then(|source| source.cast::<Control>()).unwrap_or_else(|| control.clone());
            context_menu.open_core(
                &control,
                &placement_target,
                if requested_by_pointer { context_menu.placement() } else { PlacementMode::Bottom },
            );
            e.set_handled(true);

            if e.is_holding() {
                control.perform_feedback(FeedbackAction::hold());
            }
        }
    }

    fn control_on_attached_to_visual_tree(sender: &Control, _e: &VisualTreeAttachmentEventArgs) {
        Self::attach_control_to_context_menu(sender);
    }

    fn attach_control_to_context_menu(control: &Control) {
        if let Some(context_menu) = control.context_menu() {
            context_menu
                .attached_controls
                .borrow_mut()
                .get_or_insert_with(Vec::new)
                .push(control.to_ref().downgrade());
        }
    }

    fn control_detached_from_visual_tree(control: &Control, _e: &VisualTreeAttachmentEventArgs) {
        if let Some(context_menu) = control.context_menu() {
            let control = control.to_ref();
            let popup = context_menu.popup.borrow().clone();
            if let Some(popup) = popup {
                if popup.parent() == Some(control.clone().upcast::<StyledElement>()) {
                    popup.set_parent(None);
                }
            }

            context_menu.close();
            context_menu.remove_attached_control(&control);
        }
    }

    fn cancel_closing(&self) -> bool {
        let event_args = CancelEventArgs::new();
        for (_, handler) in self.closing.snapshot().iter() {
            handler(&event_args);
        }
        event_args.cancel()
    }

    fn cancel_opening(&self) -> bool {
        let event_args = CancelEventArgs::new();
        for (_, handler) in self.opening.snapshot().iter() {
            handler(&event_args);
        }
        event_args.cancel()
    }
}

/// The popup host provider contract of a context menu.
struct ContextMenuHostProvider(Ref<ContextMenu>);

impl IPopupHostProvider for ContextMenuHostProvider {
    fn popup_host(&self) -> Option<Rc<dyn IPopupHost>> {
        self.0.popup_host()
    }

    fn popup_host_changed(&self, handler: Rc<dyn Fn(Option<&Rc<dyn IPopupHost>>)>) -> Rc<dyn IDisposable> {
        self.0.subscribe::<dyn Fn(Option<&Rc<dyn IPopupHost>>)>(|menu| &menu.popup_host_changed_handler, handler)
    }
}
