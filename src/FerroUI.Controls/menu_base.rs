use crate::generators::RecycleKey;
use crate::i_menu::{register_menu, IMenu};
use crate::i_menu_element::IMenuElement;
use crate::i_menu_item::{as_menu_item, IMenuItem};
use crate::platform::{DefaultMenuInteractionHandler, IMenuInteractionHandler};
use crate::primitives::{SelectingItemsControl, SelectingItemsControlImpl, TemplatedControlImpl};
use crate::{Control, ControlImpl, ItemsControl, ItemsControlImpl, Menu, MenuItem, Separator, TopLevel};
use ferroui_base::input::{IFocusScope, IMainMenu, InputElement, InputElementImpl, KeyEventArgs, NavigationDirection};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, BoxedValue,
    DirectProperty, FerroObjectImpl, FerroProperty, Ref, StyledElement, StyledElementImpl, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use std::cell::Cell;
use std::rc::Rc;

/// Base class for menu controls.
#[repr(C)]
pub struct MenuBase {
    base: SelectingItemsControl,
    is_open: Cell<bool>,
    interaction_handler: Rc<dyn IMenuInteractionHandler>,
}

ferro_class! {
    MenuBase: SelectingItemsControl, virtuals MenuBaseImpl: SelectingItemsControlImpl {
        /// Closes the menu.
        fn close(this);
        /// Opens the menu.
        fn open(this);
        /// Called when a submenu opens somewhere in the menu.
        fn on_submenu_opened(this, e: &RoutedEventArgs);
    }
}

ferro_class_info!(MenuBase {});

ferro_impl_classes!(
    MenuBase: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl
);

impl IFocusScope for MenuBase {}

impl VisualImpl for MenuBase {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.interaction_handler.attach(&this.to_ref());
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.interaction_handler.detach(&this.to_ref());
    }
}

impl InputElementImpl for MenuBase {
    fn is_focus_scope(_this: &Self) -> bool {
        true
    }

    fn on_key_down(_this: &Self, _e: &KeyEventArgs) {
        // Don't handle here: let the interaction handler handle it.
    }
}

impl ItemsControlImpl for MenuBase {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        MenuItem::new().upcast()
    }

    fn needs_container_override(_this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        Self::needs_menu_container(item)
    }
}

impl MenuBaseImpl for MenuBase {
    fn close(_this: &Self) {
        panic!("MenuBase is abstract.")
    }

    fn open(_this: &Self) {
        panic!("MenuBase is abstract.")
    }

    fn on_submenu_opened(this: &Self, e: &RoutedEventArgs) {
        let menu_item = e.source().and_then(|source| source.cast::<MenuItem>());

        if let Some(menu_item) = menu_item {
            let this_element: &StyledElement = this;
            if menu_item.parent().is_some_and(|parent| std::ptr::eq::<StyledElement>(&*parent, this_element)) {
                let children: Vec<Ref<MenuItem>> = StyledElement::logical_children(this)
                    .to_vec()
                    .into_iter()
                    .filter_map(|child| child.cast::<MenuItem>())
                    .collect();

                for child in children {
                    if child != menu_item && child.is_sub_menu_open() {
                        child.set_is_sub_menu_open(false);
                    }
                }
            }
        }

        this.set_is_open(true);
    }
}

/// The menu viewed as a menu element and as a menu.
struct MenuBaseHandle(Ref<MenuBase>);

impl IMenuElement for MenuBaseHandle {
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

    fn as_menu(&self) -> Option<Rc<dyn IMenu>> {
        Some(Rc::new(MenuBaseHandle(self.0.clone())))
    }

    fn as_main_menu(&self) -> Option<Rc<dyn IMainMenu>> {
        self.0.clone().cast::<Menu>().map(|menu| menu.to_main_menu())
    }
}

impl IMenu for MenuBaseHandle {
    fn interaction_handler(&self) -> Rc<dyn IMenuInteractionHandler> {
        self.0.interaction_handler()
    }

    fn is_open(&self) -> bool {
        self.0.is_open()
    }

    fn top_level(&self) -> Option<Ref<TopLevel>> {
        TopLevel::get_top_level(Some(&self.0))
    }
}

ferro_properties! {
    impl MenuBase {
        /// Defines the `IsOpen` property.
        pub fn is_open_property() -> DirectProperty<MenuBase, bool> {
            FerroProperty::register_direct::<MenuBase, _>("IsOpen", |o| o.is_open(), None, false)
        }
    }
}

impl MenuBase {
    ferro_routed_event!(
        /// Defines the `Opened` event.
        pub fn opened_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<MenuBase, _>("Opened", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Closed` event.
        pub fn closed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<MenuBase, _>("Closed", RoutingStrategies::BUBBLE)
        }
    );

    /// Initializes static members of the [`MenuBase`] class.
    fn static_constructor() {
        register_menu::<MenuBase>(|menu| Rc::new(MenuBaseHandle(menu)));
        MenuItem::submenu_opened_event().add_class_handler::<MenuBase>(|x, e| x.on_submenu_opened(e));
    }

    /// Creates the class data of a menu that uses the default menu
    /// interaction handler; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self::construct_with(DefaultMenuInteractionHandler::new(false))
    }

    /// Creates the class data of a menu that uses `interaction_handler`.
    pub fn construct_with(interaction_handler: Rc<dyn IMenuInteractionHandler>) -> Self {
        Self { base: SelectingItemsControl::construct(), is_open: Cell::new(false), interaction_handler }
    }

    /// Gets a value indicating whether the menu is open.
    pub fn is_open(&self) -> bool {
        self.is_open.get()
    }

    /// Sets a value indicating whether the menu is open. For deriving
    /// classes.
    pub fn set_is_open(&self, value: bool) {
        self.set_and_raise_cell(Self::is_open_property(), &self.is_open, value);
    }

    /// Gets the interaction handler for the menu.
    pub fn interaction_handler(&self) -> Rc<dyn IMenuInteractionHandler> {
        self.interaction_handler.clone()
    }

    /// Occurs when a menu is opened.
    pub fn opened(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::opened_event(), handler)
    }

    /// Occurs when a menu is closed.
    pub fn closed(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::closed_event(), handler)
    }

    /// The menu viewed as [`IMenu`].
    pub(crate) fn to_menu(&self) -> Rc<dyn IMenu> {
        Rc::new(MenuBaseHandle(self.to_ref()))
    }

    /// The container decision shared by menus and menu items: menu items
    /// and separators are their own containers.
    pub(crate) fn needs_menu_container(item: &Option<BoxedValue>) -> (bool, Option<RecycleKey>) {
        let is_container = item
            .as_ref()
            .and_then(Control::from_boxed)
            .is_some_and(|control| control.is::<MenuItem>() || control.is::<Separator>());

        if is_container {
            (false, None)
        } else {
            (true, Some(ItemsControl::DEFAULT_RECYCLE_KEY))
        }
    }
}
