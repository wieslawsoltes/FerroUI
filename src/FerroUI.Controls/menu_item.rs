use crate::definition_base::{DefinitionBase, SharedSizeScope};
use crate::generators::RecycleKey;
use crate::i_clickable_control::register_clickable_control;
use crate::i_command_source::register_command_source;
use crate::i_menu::as_menu;
use crate::i_menu_element::{as_menu_element, IMenuElement};
use crate::i_menu_item::{as_menu_item, register_menu_item, IMenuItem};
use crate::i_selectable::{register_selectable, ISelectable};
use crate::menu_item_access_key_handler::MenuItemAccessKeyHandler;
use crate::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use crate::mixins::{PressedMixin, SelectableMixin};
use crate::platform::default_menu_interaction_handler::find_logical_ancestor_of_type;
use crate::platform::{FeedbackType, IMenuInteractionHandler, PlatformFeedback};
use crate::primitives::{
    HeaderedSelectingItemsControl, HeaderedSelectingItemsControlImpl, Popup, SelectingItemsControl,
    SelectingItemsControlImpl, TemplateAppliedEventArgs, TemplatedControlImpl,
};
use crate::radio_button_group_manager::{register_radio_button, IRadioButton};
use crate::templates::{FuncTemplate, ITemplateOf};
use crate::{
    Button, Control, ControlImpl, HotKeyManager, ItemsControl, ItemsControlImpl, Menu, MenuBase, MenuItemToggleType,
    Panel, RadioButton, StackPanel,
};
use ferroui_base::data::{BindingError, BindingPriority, BindingValueType};
use ferroui_base::input::{
    AccessKeyHandler, AccessKeyPressedEventArgs, FocusChangedEventArgs, IAccessKeyHandler, IClickableControl,
    ICommand, ICommandSource, InputElement, InputElementImpl, InputElementImplExt, KeyEventArgs, KeyGesture, NavigationDirection,
    PointerEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::utilities::WeakEvents;
use ferroui_base::reactive::{Disposable, IDisposable, IObserver, LightweightSubject, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate, BoxedValue,
    FerroLocator, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, IFerroDependencyResolver, Ref, ServiceHandle, StaticType, StyledElement,
    StyledElementImpl, StyledElementImplExt, StyledProperty, StyledPropertyMetadata, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A menu item control.
#[repr(C)]
pub struct MenuItem {
    base: HeaderedSelectingItemsControl,
    can_execute_change_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    command_can_execute: Cell<bool>,
    command_binding_error: Cell<bool>,
    popup: RefCell<Option<Ref<Popup>>>,
    /// The subscriptions to the `Opened` and `Closed` events of the popup.
    popup_events: RefCell<Option<(Rc<dyn IDisposable>, Rc<dyn IDisposable>)>>,
    hotkey: Cell<Option<KeyGesture>>,
    is_embedded_in_menu: Cell<bool>,
    /// The shared size scope of the visual parent; see `constructed`.
    parent_shared_size_scope: LightweightSubject<Option<Rc<SharedSizeScope>>>,
    parent_shared_size_scope_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    MenuItem: HeaderedSelectingItemsControl, virtuals MenuItemImpl: HeaderedSelectingItemsControlImpl {
        /// Invoked when an unhandled `Click` event reaches an element in
        /// its route that is derived from this class. Implement this method
        /// to add class handling for this event.
        fn on_click(this, e: &RoutedEventArgs);
        /// Invoked when an unhandled `SubmenuOpened` event reaches an
        /// element in its route that is derived from this class. Implement
        /// this method to add class handling for this event.
        fn on_submenu_opened(this, e: &RoutedEventArgs);
    }
}

ferro_class_info!(MenuItem { new: MenuItem::new });

ferro_impl_classes!(MenuItem: LayoutableImpl, InteractiveImpl, SelectingItemsControlImpl, HeaderedSelectingItemsControlImpl);

impl ControlImpl for MenuItem {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::MenuItemAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for MenuItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        // HACK: This nasty but it's all WPF's fault. Grid uses an inherited
        // attached property to store SharedSizeGroup state, except property
        // inheritance is done down the logical tree. In this case, the
        // control which is setting Grid.IsSharedSizeScope="True" is not in
        // the logical tree. Instead of fixing the way Grid stores shared
        // size state, the developers of WPF just created a binding of the
        // internal state of the visual parent to the menu item. We don't
        // have much choice but to do the same for now unless we want to
        // refactor Grid.
        //
        // In addition to the hack from WPF, we also make sure to return null
        // when we have no parent. If we don't do this, inheritance falls
        // back to the logical tree, causing the shared size scope in the
        // parent MenuItem to be used, breaking menu layout.
        let parent_shared_size_scope: Rc<LightweightSubject<Option<Rc<SharedSizeScope>>>> =
            Rc::new(this.parent_shared_size_scope.clone());

        this.bind(
            DefinitionBase::private_shared_size_scope_property(),
            parent_shared_size_scope,
            BindingPriority::LocalValue,
        );
        this.update_parent_shared_size_scope(None);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Visual::visual_parent_property().as_property() {
            this.update_parent_shared_size_scope(change.get_new_value::<Option<Ref<Visual>>>());
        }

        if property == HeaderedSelectingItemsControl::header_property().as_property() {
            this.header_changed(change);
        } else if property == Self::icon_property().as_property() {
            this.icon_changed(change);
        } else if property == SelectingItemsControl::is_selected_property().as_property() {
            this.is_selected_changed(change);
        } else if property == Self::is_sub_menu_open_property().as_property() {
            this.sub_menu_open_changed(change);
        } else if property == Self::command_property().as_property() {
            this.command_changed(change);
        } else if property == Self::command_parameter_property().as_property() {
            this.command_parameter_changed(change);
        } else if property == Self::is_checked_property().as_property() {
            this.is_checked_changed(change);
        } else if property == Self::toggle_type_property().as_property() {
            this.toggle_type_changed(change);
        } else if property == Self::group_name_property().as_property() {
            this.group_name_changed(change);
        } else if property == ItemsControl::item_count_property().as_property() {
            // A menu item with no sub-menu is effectively disabled if its
            // command binding failed: this means that the effectively
            // enabled state depends on whether the number of items in the
            // menu is 0 or not.
            let (o, n) = change.get_old_and_new_value::<i32>();
            if o == 0 || n == 0 {
                this.update_is_effectively_enabled();
            }
        }
    }

    // TODO: This is confusing for some ppl. Need to think about alternatives here.
    fn update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&BindingError>,
    ) {
        Self::parent_update_data_validation(this, property, state, error);

        if property == Self::command_property().as_property() {
            this.command_binding_error.set(state == BindingValueType::BINDING_ERROR);
            if this.command_binding_error.get() && this.command_can_execute.get() {
                this.command_can_execute.set(false);
                this.update_is_effectively_enabled();
            }
        }
    }
}

impl StyledElementImpl for MenuItem {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        // Control attached again, set Hotkey to create a hotkey manager for
        // this control
        if let Some(hotkey) = this.hotkey.get() {
            this.set_current_value(Self::hot_key_property(), Some(hotkey));
        }

        Self::parent_on_attached_to_logical_tree(this, e);

        let (command, parameter) = (this.command(), this.command_parameter());
        if let Some(command) = &command {
            this.subscribe_can_execute_changed(command);
        }

        this.try_update_can_execute_with(command.as_ref(), parameter.as_ref());

        let mut parent = this.parent();

        while let Some(current) = parent.clone().filter(|parent| parent.is::<MenuItem>()) {
            parent = current.parent();
        }

        let mut is_embedded_in_menu = false;
        let mut current = parent;
        while let Some(logical) = current {
            if as_menu(&logical).is_some() {
                is_embedded_in_menu = true;
                break;
            }
            current = logical.parent();
        }
        this.is_embedded_in_menu.set(is_embedded_in_menu);
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        // This will cause the hotkey manager to dispose the observer and
        // the reference to this control
        if let Some(hotkey) = this.hot_key() {
            this.hotkey.set(Some(hotkey));
            this.set_current_value(Self::hot_key_property(), None);
        }

        Self::parent_on_detached_from_logical_tree(this, e);

        if this.command().is_some() {
            this.unsubscribe_can_execute_changed();
        }
    }
}

impl VisualImpl for MenuItem {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        this.try_update_can_execute();
        this.register_in_menu_interaction_handler();
    }
}

impl InputElementImpl for MenuItem {
    fn is_enabled_core(this: &Self) -> bool {
        this.is_enabled() && (this.has_sub_menu() || this.command_can_execute.get())
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        if !this.is_embedded_in_menu.get() {
            // Normally the Menu's IMenuInteractionHandler is sending the
            // click events for us. However when the item is not embedded
            // into a menu we need to send them ourselves.
            this.raise_event(&RoutedEventArgs::with_event(Self::click_event()));
        }
    }

    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);

        let container: Ref<Control> = this.to_ref().upcast();
        if let Some(owner) = SelectingItemsControl::selecting_items_control_from_item_container(&container) {
            owner.update_selection_from_event(&container, e);
        }
    }

    fn on_key_down(_this: &Self, _e: &KeyEventArgs) {
        // Don't handle here: let event bubble up to menu.
    }

    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);
        this.raise_event(&RoutedEventArgs::with_event(Self::pointer_entered_item_event()));
    }

    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        this.raise_event(&RoutedEventArgs::with_event(Self::pointer_exited_item_event()));
    }
}

impl TemplatedControlImpl for MenuItem {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let old_popup = this.popup.borrow_mut().take();
        if let Some(old_popup) = old_popup {
            let popup_events = this.popup_events.borrow_mut().take();
            if let Some((opened, closed)) = popup_events {
                opened.dispose();
                closed.dispose();
            }
            old_popup.set_dependency_resolver(None);
        }

        let popup = e.name_scope().find_as::<Popup>("PART_Popup");
        *this.popup.borrow_mut() = popup.clone();

        if let Some(popup) = popup {
            popup.set_dependency_resolver(Some(DependencyResolver::instance()));

            let weak = this.to_ref().downgrade();
            let opened = popup.opened({
                let weak = weak.clone();
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.popup_opened();
                    }
                }
            });
            let closed = popup.closed(move || {
                if let Some(this) = weak.upgrade() {
                    this.popup_closed();
                }
            });
            *this.popup_events.borrow_mut() = Some((opened, closed));
        }
    }
}

impl ItemsControlImpl for MenuItem {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        MenuItem::new().upcast()
    }

    fn needs_container_override(_this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        MenuBase::needs_menu_container(item)
    }
}

impl MenuItemImpl for MenuItem {
    fn on_click(this: &Self, e: &RoutedEventArgs) {
        let (command, parameter) = (this.command(), this.command_parameter());
        if let Some(command) = command {
            if !e.handled() && command.can_execute(parameter.as_ref()) {
                command.execute(parameter.as_ref());
                e.set_handled(true);
            }
        }
    }

    fn on_submenu_opened(this: &Self, e: &RoutedEventArgs) {
        let menu_item = e.source().and_then(|source| source.cast::<MenuItem>());

        if let Some(menu_item) = menu_item {
            let this_element: &StyledElement = this;
            if menu_item.parent().is_some_and(|parent| std::ptr::eq::<StyledElement>(&*parent, this_element)) {
                for child in this.to_menu_item().sub_items() {
                    if child.element() != menu_item.clone().upcast::<InputElement>() && child.is_sub_menu_open() {
                        child.set_is_sub_menu_open(false);
                    }
                }
            }
        }
    }
}

impl ISelectable for MenuItem {
    fn is_selected(&self) -> bool {
        MenuItem::is_selected(self)
    }

    fn set_is_selected(&self, value: bool) {
        MenuItem::set_is_selected(self, value)
    }
}

/// The menu item viewed through the interfaces it implements.
struct MenuItemHandle(Ref<MenuItem>);

impl IMenuElement for MenuItemHandle {
    fn element(&self) -> Ref<InputElement> {
        self.0.clone().upcast()
    }

    fn selected_item(&self) -> Option<Rc<dyn IMenuItem>> {
        let index = self.0.selected_index();
        if index != -1 {
            self.0.container_from_index(index).and_then(|container| as_menu_item(&container))
        } else {
            None
        }
    }

    fn set_selected_item(&self, value: Option<Rc<dyn IMenuItem>>) {
        let index = match value.and_then(|value| value.element().cast::<Control>()) {
            Some(c) => self.0.index_from_container(&c),
            None => -1,
        };
        self.0.set_selected_index(index);
    }

    fn sub_items(&self) -> Vec<Rc<dyn IMenuItem>> {
        StyledElement::logical_children(&self.0).to_vec().iter().filter_map(|child| as_menu_item(child)).collect()
    }

    fn open(&self) {
        self.0.open();
    }

    fn close(&self) {
        self.0.close();
    }

    fn move_selection(&self, direction: NavigationDirection, wrap: bool) -> bool {
        self.0.move_selection(direction, wrap, false)
    }

    fn as_menu_item(&self) -> Option<Rc<dyn IMenuItem>> {
        Some(Rc::new(MenuItemHandle(self.0.clone())))
    }
}

impl IMenuItem for MenuItemHandle {
    fn has_sub_menu(&self) -> bool {
        self.0.has_sub_menu()
    }

    fn is_pointer_over_sub_menu(&self) -> bool {
        self.0.popup.borrow().as_ref().is_some_and(|popup| popup.is_pointer_over_popup())
    }

    fn is_sub_menu_open(&self) -> bool {
        self.0.is_sub_menu_open()
    }

    fn set_is_sub_menu_open(&self, value: bool) {
        self.0.set_current_value(MenuItem::is_sub_menu_open_property(), value);
    }

    fn stays_open_on_click(&self) -> bool {
        self.0.stays_open_on_click()
    }

    fn set_stays_open_on_click(&self, value: bool) {
        self.0.set_current_value(MenuItem::stays_open_on_click_property(), value);
    }

    fn is_top_level(&self) -> bool {
        self.0.is_top_level()
    }

    fn parent(&self) -> Option<Rc<dyn IMenuElement>> {
        self.0.parent().and_then(|parent| as_menu_element(&parent))
    }

    fn toggle_type(&self) -> MenuItemToggleType {
        self.0.toggle_type()
    }

    fn group_name(&self) -> Option<String> {
        self.0.group_name()
    }

    fn is_checked(&self) -> bool {
        self.0.is_checked()
    }

    fn set_is_checked(&self, value: bool) {
        self.0.set_current_value(MenuItem::is_checked_property(), value);
    }

    fn raise_click(&self) {
        self.0.raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
    }
}

impl IRadioButton for MenuItemHandle {
    fn logical(&self) -> Ref<StyledElement> {
        self.0.clone().upcast()
    }

    fn group_name(&self) -> Option<String> {
        self.0.group_name()
    }

    fn toggle_type(&self) -> MenuItemToggleType {
        self.0.toggle_type()
    }

    fn is_checked(&self) -> bool {
        self.0.is_checked()
    }

    fn set_is_checked(&self, value: bool) {
        self.0.set_current_value(MenuItem::is_checked_property(), value);
    }
}

impl ICommandSource for MenuItemHandle {
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.0.command()
    }

    fn command_parameter(&self) -> Option<BoxedValue> {
        self.0.command_parameter()
    }

    fn can_execute_changed(&self) {
        self.0.can_execute_changed();
    }

    fn is_effectively_enabled(&self) -> bool {
        self.0.is_effectively_enabled()
    }
}

impl IClickableControl for MenuItemHandle {
    fn click(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.0.click(move |_, e| handler(e));
        let menu_item = self.0.downgrade();
        Disposable::create(move || {
            if let Some(menu_item) = menu_item.upgrade() {
                menu_item.remove_handler(MenuItem::click_event(), token);
            }
        })
    }

    fn raise_click(&self) {
        if self.0.is_effectively_enabled() {
            self.0.raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
        }
    }

    fn is_effectively_enabled(&self) -> bool {
        self.0.is_effectively_enabled()
    }
}

/// A dependency resolver which returns a [`MenuItemAccessKeyHandler`].
struct DependencyResolver;

impl DependencyResolver {
    /// Gets the default instance of [`DependencyResolver`].
    fn instance() -> Rc<dyn IFerroDependencyResolver> {
        thread_local! {
            static INSTANCE: Rc<DependencyResolver> = Rc::new(DependencyResolver);
        }
        INSTANCE.with(|instance| instance.clone())
    }
}

impl IFerroDependencyResolver for DependencyResolver {
    /// Gets a service of the specified type.
    fn get_service_untyped(&self, service_type: TypeId) -> Option<ServiceHandle> {
        if service_type == TypeId::of::<dyn IAccessKeyHandler>() {
            let handler: Rc<dyn IAccessKeyHandler> = MenuItemAccessKeyHandler::new();
            let handle: ServiceHandle = Rc::new(handler);
            Some(handle)
        } else {
            FerroLocator::current().get_service_untyped(service_type)
        }
    }
}

ferro_properties! {
    impl MenuItem {
        /// Defines the `Command` property.
        pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            Button::command_property()
                .add_owner_with::<MenuItem>(StyledPropertyMetadata::new(None).with_enable_data_validation(true))
        }

        /// Defines the `HotKey` property.
        pub fn hot_key_property() -> StyledProperty<Option<KeyGesture>> {
            HotKeyManager::hot_key_property().add_owner::<MenuItem>()
        }

        /// Defines the `CommandParameter` property.
        pub fn command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            Button::command_parameter_property().add_owner::<MenuItem>()
        }

        /// Defines the `Icon` property.
        pub fn icon_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<MenuItem, _>("Icon", None)
        }

        /// Defines the `InputGesture` property.
        pub fn input_gesture_property() -> StyledProperty<Option<KeyGesture>> {
            FerroProperty::register::<MenuItem, _>("InputGesture", None)
        }

        /// Defines the `IsSubMenuOpen` property.
        pub fn is_sub_menu_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<MenuItem, _>("IsSubMenuOpen", false)
        }

        /// Defines the `StaysOpenOnClick` property.
        pub fn stays_open_on_click_property() -> StyledProperty<bool> {
            FerroProperty::register::<MenuItem, _>("StaysOpenOnClick", false)
        }

        /// Defines the `ToggleType` property.
        pub fn toggle_type_property() -> StyledProperty<MenuItemToggleType> {
            FerroProperty::register::<MenuItem, _>("ToggleType", MenuItemToggleType::None)
        }

        /// Defines the `IsChecked` property.
        pub fn is_checked_property() -> StyledProperty<bool> {
            FerroProperty::register::<MenuItem, _>("IsChecked", false)
        }

        /// Defines the `GroupName` property.
        pub fn group_name_property() -> StyledProperty<Option<String>> {
            RadioButton::group_name_property().add_owner::<MenuItem>()
        }
    }
}

impl MenuItem {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] =
        &[TemplatePartAttribute::new("PART_Popup", <Popup as StaticType>::TYPE)];

    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[
        ":separator",
        ":radio",
        ":toggle",
        ":checked",
        ":icon",
        ":open",
        ":pressed",
        ":selected",
    ]);

    ferro_routed_event!(
        /// Defines the `Click` event.
        pub fn click_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<MenuItem, _>("Click", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerEnteredItem` event.
        pub fn pointer_entered_item_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<MenuItem, _>("PointerEnteredItem", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerExitedItem` event.
        pub fn pointer_exited_item_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<MenuItem, _>("PointerExitedItem", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `SubmenuOpened` event.
        pub fn submenu_opened_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<MenuItem, _>("SubmenuOpened", RoutingStrategies::BUBBLE)
        }
    );

    /// Initializes static members of the [`MenuItem`] class.
    fn static_constructor() {
        register_menu_item::<MenuItem>(|menu_item| Rc::new(MenuItemHandle(menu_item)));
        register_radio_button::<MenuItem>(|menu_item| Rc::new(MenuItemHandle(menu_item)));
        register_command_source::<MenuItem>(|menu_item| Rc::new(MenuItemHandle(menu_item)));
        register_clickable_control::<MenuItem>(|menu_item| Rc::new(MenuItemHandle(menu_item)));
        register_selectable::<MenuItem>();

        // The default value for the `ItemsPanel` property.
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(StackPanel::new().upcast::<Panel>()));

        SelectableMixin::attach::<MenuItem>(SelectingItemsControl::is_selected_property());
        PressedMixin::attach::<MenuItem>();
        InputElement::focusable_property().override_default_value::<MenuItem>(true);
        ItemsControl::items_panel_property().override_default_value::<MenuItem>(default_panel);
        Self::click_event().add_class_handler::<MenuItem>(|x, e| x.on_click(e));
        Self::submenu_opened_event().add_class_handler::<MenuItem>(|x, e| x.on_submenu_opened(e));
        crate::automation::AutomationProperties::is_offscreen_behavior_property()
            .override_default_value::<MenuItem>(crate::automation::IsOffscreenBehavior::FromClip);
        AccessKeyHandler::access_key_pressed_event().add_class_handler::<MenuItem>(Self::on_access_key_pressed);
        PlatformFeedback::feedback_type_property().override_default_value::<MenuItem>(FeedbackType::Auto);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: HeaderedSelectingItemsControl::construct(),
            can_execute_change_subscription: RefCell::new(None),
            command_can_execute: Cell::new(true),
            command_binding_error: Cell::new(false),
            popup: RefCell::new(None),
            popup_events: RefCell::new(None),
            hotkey: Cell::new(None),
            is_embedded_in_menu: Cell::new(false),
            parent_shared_size_scope: LightweightSubject::new(),
            parent_shared_size_scope_subscription: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Occurs when a [`MenuItem`] without a submenu is clicked.
    pub fn click(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::click_event(), handler)
    }

    /// Occurs when the pointer enters a menu item.
    ///
    /// A bubbling version of the pointer entered event for menu items.
    pub fn pointer_entered_item(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::pointer_entered_item_event(), handler)
    }

    /// Raised when the pointer leaves a menu item.
    ///
    /// A bubbling version of the pointer exited event for menu items.
    pub fn pointer_exited_item(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::pointer_exited_item_event(), handler)
    }

    /// Occurs when a [`MenuItem`]'s submenu is opened.
    pub fn submenu_opened(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::submenu_opened_event(), handler)
    }

    /// Gets or sets the command associated with the menu item.
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value)
    }

    /// Gets or sets a key gesture associated with this control.
    pub fn hot_key(&self) -> Option<KeyGesture> {
        self.get_value(Self::hot_key_property())
    }

    pub fn set_hot_key(&self, value: Option<KeyGesture>) {
        self.set_value(Self::hot_key_property(), value)
    }

    /// Gets or sets the parameter to pass to the `Command` property of a
    /// [`MenuItem`].
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value)
    }

    /// Gets or sets the icon that appears in a [`MenuItem`].
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }

    pub fn set_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), value)
    }

    /// Gets or sets the input gesture that will be displayed in the menu
    /// item.
    ///
    /// Setting this property does not cause the input gesture to be handled
    /// by the menu item, it simply displays the gesture text in the menu.
    pub fn input_gesture(&self) -> Option<KeyGesture> {
        self.get_value(Self::input_gesture_property())
    }

    pub fn set_input_gesture(&self, value: Option<KeyGesture>) {
        self.set_value(Self::input_gesture_property(), value)
    }

    /// Gets or sets a value indicating whether the [`MenuItem`] is
    /// currently selected.
    pub fn is_selected(&self) -> bool {
        self.get_value(SelectingItemsControl::is_selected_property())
    }

    pub fn set_is_selected(&self, value: bool) {
        self.set_value(SelectingItemsControl::is_selected_property(), value)
    }

    /// Gets or sets a value that indicates whether the submenu of the
    /// [`MenuItem`] is open.
    pub fn is_sub_menu_open(&self) -> bool {
        self.get_value(Self::is_sub_menu_open_property())
    }

    pub fn set_is_sub_menu_open(&self, value: bool) {
        self.set_value(Self::is_sub_menu_open_property(), value)
    }

    /// Gets or sets a value that indicates the submenu that this
    /// [`MenuItem`] is within should not close when this item is clicked.
    pub fn stays_open_on_click(&self) -> bool {
        self.get_value(Self::stays_open_on_click_property())
    }

    pub fn set_stays_open_on_click(&self, value: bool) {
        self.set_value(Self::stays_open_on_click_property(), value)
    }

    /// Gets or sets the toggle type of the menu item.
    pub fn toggle_type(&self) -> MenuItemToggleType {
        self.get_value(Self::toggle_type_property())
    }

    pub fn set_toggle_type(&self, value: MenuItemToggleType) {
        self.set_value(Self::toggle_type_property(), value)
    }

    /// Gets or sets if menu item is checked when the toggle type is
    /// [`MenuItemToggleType::CheckBox`] or [`MenuItemToggleType::Radio`].
    pub fn is_checked(&self) -> bool {
        self.get_value(Self::is_checked_property())
    }

    pub fn set_is_checked(&self, value: bool) {
        self.set_value(Self::is_checked_property(), value)
    }

    /// Gets or sets the menu item group name when the toggle type is
    /// [`MenuItemToggleType::Radio`].
    pub fn group_name(&self) -> Option<String> {
        self.get_value(Self::group_name_property())
    }

    pub fn set_group_name(&self, value: Option<String>) {
        self.set_value(Self::group_name_property(), value)
    }

    /// Gets a value that indicates whether the [`MenuItem`] has a submenu.
    pub fn has_sub_menu(&self) -> bool {
        !self.classes().contains(":empty")
    }

    /// Gets a value that indicates whether the [`MenuItem`] is a top-level
    /// main menu item.
    pub fn is_top_level(&self) -> bool {
        self.parent().is_some_and(|parent| parent.is::<Menu>())
    }

    /// The interaction handler of the menu the item is within.
    pub(crate) fn menu_interaction_handler(&self) -> Option<Rc<dyn IMenuInteractionHandler>> {
        let this: Ref<StyledElement> = self.to_ref().upcast();
        find_logical_ancestor_of_type::<MenuBase>(&this, false)
            .or_else(|| self.find_ancestor_of_type::<MenuBase>(false))
            .map(|menu| menu.interaction_handler())
    }

    /// Opens the submenu.
    ///
    /// This has the same effect as setting `IsSubMenuOpen` to true.
    pub fn open(&self) {
        self.set_current_value(Self::is_sub_menu_open_property(), true);
    }

    /// Closes the submenu.
    ///
    /// This has the same effect as setting `IsSubMenuOpen` to false.
    pub fn close(&self) {
        self.set_current_value(Self::is_sub_menu_open_property(), false);
    }

    /// The menu item viewed as [`IMenuItem`].
    pub(crate) fn to_menu_item(&self) -> Rc<dyn IMenuItem> {
        Rc::new(MenuItemHandle(self.to_ref()))
    }

    /// Feeds the shared size scope of the visual parent to the binding
    /// created in `constructed`.
    fn update_parent_shared_size_scope(&self, visual_parent: Option<Ref<Visual>>) {
        let subscription = self.parent_shared_size_scope_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        match visual_parent.and_then(|parent| parent.cast::<Control>()) {
            Some(parent) => {
                let subject = self.parent_shared_size_scope.clone();
                let parent: &ferroui_base::FerroObject = &parent;
                let subscription = FerroObjectExtensions::get_observable(
                    parent,
                    DefinitionBase::private_shared_size_scope_property(),
                )
                    .subscribe_fn(move |scope| subject.on_next(scope));
                *self.parent_shared_size_scope_subscription.borrow_mut() = Some(subscription);
            }
            None => self.parent_shared_size_scope.on_next(None),
        }
    }

    /// Closes all submenus of the menu item.
    fn close_submenus(&self) {
        for child in self.to_menu_item().sub_items() {
            child.set_is_sub_menu_open(false);
        }
    }

    /// Called when the `Command` property changes.
    fn command_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_command, new_command) = e.get_old_and_new_value::<Option<Rc<dyn ICommand>>>();

        if self.is_attached_to_logical_tree() {
            if old_command.is_some() {
                self.unsubscribe_can_execute_changed();
            }

            if let Some(new_command) = &new_command {
                self.subscribe_can_execute_changed(new_command);
            }
        }
        self.try_update_can_execute_with(new_command.as_ref(), self.command_parameter().as_ref());
    }

    fn on_access_key_pressed(sender: &MenuItem, e: &AccessKeyPressedEventArgs) {
        if e.handled() || e.target().is_some() {
            return;
        }

        e.set_target(Some(sender.to_ref().upcast()));
        e.set_handled(true);
    }

    /// Called when the `CommandParameter` property changes.
    fn command_parameter_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (command, parameter) = (self.command(), e.get_new_value::<Option<BoxedValue>>());
        self.try_update_can_execute_with(command.as_ref(), parameter.as_ref());
    }

    fn subscribe_can_execute_changed(&self, command: &Rc<dyn ICommand>) {
        // A weak subscription: it ends with this control, so a command
        // that outlives the control keeps no handler of it.
        let subscription = WeakEvents::command_can_execute_changed(command, &self.to_ref(), |this| {
            this.can_execute_changed()
        });
        let old = self.can_execute_change_subscription.replace(Some(subscription));
        if let Some(old) = old {
            old.dispose();
        }
    }

    fn unsubscribe_can_execute_changed(&self) {
        let subscription = self.can_execute_change_subscription.take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    /// Called when the "can execute changed" event of the command fires.
    fn can_execute_changed(&self) {
        self.try_update_can_execute();
    }

    /// Tries to evaluate CanExecute value of a Command if menu is opened
    fn try_update_can_execute(&self) {
        self.try_update_can_execute_with(self.command().as_ref(), self.command_parameter().as_ref());
    }

    fn try_update_can_execute_with(&self, command: Option<&Rc<dyn ICommand>>, parameter: Option<&BoxedValue>) {
        let Some(command) = command else {
            self.command_can_execute.set(!self.command_binding_error.get());
            self.update_is_effectively_enabled();
            return;
        };

        // Perf optimization - only raise CanExecute event if the menu is open
        if !self.is_attached_to_logical_tree()
            || self
                .parent()
                .and_then(|parent| parent.cast::<MenuItem>())
                .is_some_and(|parent| !parent.is_sub_menu_open())
        {
            return;
        }

        let can_execute = command.can_execute(parameter);
        if can_execute != self.command_can_execute.get() {
            self.command_can_execute.set(can_execute);
            self.update_is_effectively_enabled();
        }
    }

    /// The default menu interaction handler of the menu the item is within,
    /// if the menu uses it.
    fn with_default_menu_interaction_handler(&self, f: impl FnOnce(&crate::platform::DefaultMenuInteractionHandler)) {
        if let Some(handler) = self.menu_interaction_handler() {
            if let Some(handler) = handler.as_default_menu_interaction_handler() {
                f(handler);
            }
        }
    }

    /// Called when the `GroupName` property changes.
    fn group_name_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let old_group_name = e.get_old_value::<Option<String>>().flatten();
        self.with_default_menu_interaction_handler(|handler| {
            handler.on_group_or_type_changed(&MenuItemHandle(self.to_ref()), old_group_name.as_deref())
        });
    }

    /// Called when the `ToggleType` property changes.
    fn toggle_type_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let new_value = e.get_new_value::<MenuItemToggleType>();
        self.pseudo_classes().set(":radio", new_value == MenuItemToggleType::Radio);
        self.pseudo_classes().set(":toggle", new_value == MenuItemToggleType::CheckBox);

        self.with_default_menu_interaction_handler(|handler| {
            handler.on_group_or_type_changed(&MenuItemHandle(self.to_ref()), self.group_name().as_deref())
        });
    }

    /// Called when the `IsChecked` property changes.
    fn is_checked_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let new_value = e.get_new_value::<bool>();
        self.pseudo_classes().set(":checked", new_value);

        if new_value {
            self.with_default_menu_interaction_handler(|handler| {
                handler.on_checked_changed(&MenuItemHandle(self.to_ref()))
            });
        }
    }

    fn is_separator_header(value: &Option<BoxedValue>) -> bool {
        value.as_ref().is_some_and(|value| {
            let value: &dyn ferroui_base::AnyValue = &**value;
            value.downcast_ref::<String>().is_some_and(|text| text == "-")
                || value.downcast_ref::<&'static str>().is_some_and(|text| *text == "-")
        })
    }

    /// Called when the `Header` property changes.
    fn header_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = e.get_old_and_new_value::<Option<BoxedValue>>();
        if Self::is_separator_header(&new_value) {
            self.pseudo_classes().add_pseudo(":separator");
            self.set_focusable(false);
        } else if Self::is_separator_header(&old_value) {
            self.pseudo_classes().remove_pseudo(":separator");
            self.set_focusable(true);
        }
    }

    /// Called when the `Icon` property changes.
    fn icon_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = e.get_old_and_new_value::<Option<BoxedValue>>();

        if let Some(old_value) = &old_value {
            if let Some(old_logical) = Control::logical_from_boxed(old_value) {
                StyledElement::logical_children(self).remove(&old_logical);
            }

            self.pseudo_classes().remove_pseudo(":icon");
        }

        if let Some(new_value) = &new_value {
            if let Some(new_logical) = Control::logical_from_boxed(new_value) {
                StyledElement::logical_children(self).add(new_logical);
            }

            self.pseudo_classes().add_pseudo(":icon");
        }
    }

    /// Called when the `IsSelected` property changes.
    fn is_selected_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let parent_menu = self.parent().and_then(|parent| parent.cast::<Menu>());

        if e.get_new_value::<bool>() && parent_menu.is_none_or(|parent_menu| parent_menu.is_open()) {
            self.focus();
        }
    }

    /// Called when the `IsSubMenuOpen` property changes.
    fn sub_menu_open_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let value = e.get_new_value::<bool>();

        if value {
            let items: Vec<Ref<MenuItem>> = self
                .items_view()
                .iter()
                .filter_map(|item| item.as_ref().and_then(Control::from_boxed))
                .filter_map(|control| control.cast::<MenuItem>())
                .collect();

            for item in items {
                item.try_update_can_execute();
            }

            self.raise_event(&RoutedEventArgs::with_event(Self::submenu_opened_event()));
            self.set_current_value(SelectingItemsControl::is_selected_property(), true);
            self.pseudo_classes().add_pseudo(":open");
        } else {
            self.close_submenus();
            self.set_selected_index(-1);
            self.pseudo_classes().remove_pseudo(":open");
        }
    }

    /// Called when the submenu's popup is opened.
    fn popup_opened(&self) {
        // If we're using overlay popups, there's a chance we need to do a
        // layout pass before the child items are added to the visual tree.
        // If we don't do this here, then selection breaks.
        if self.presenter().is_some_and(|presenter| !presenter.is_attached_to_visual_tree()) {
            self.update_layout();
        }

        let selected = self.selected_index();

        if selected != -1 {
            if let Some(container) = self.container_from_index(selected) {
                container.focus();
            }
        }
    }

    /// Called when the submenu's popup is closed.
    fn popup_closed(&self) {
        self.set_selected_item(None);
    }

    fn register_in_menu_interaction_handler(&self) {
        if self.toggle_type() != MenuItemToggleType::Radio {
            return;
        }

        self.with_default_menu_interaction_handler(|handler| {
            handler.on_group_or_type_changed(&MenuItemHandle(self.to_ref()), None)
        });
    }
}
