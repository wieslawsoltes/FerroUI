use crate::platform::{DefaultMenuInteractionHandler, IMenuInteractionHandler};
use crate::primitives::{SelectingItemsControlImpl, TemplatedControlImpl};
use crate::templates::{FuncTemplate, ITemplateOf};
use crate::{Control, ControlImpl, ItemsControl, ItemsControlImpl, ItemsControlImplExt, MenuBase, MenuBaseImpl, MenuItem, Panel, StackPanel, TopLevel};
use ferroui_base::input::{
    AccessKeyHandler, AccessKeyPressedEventArgs, IAccessKeyHandler, IMainMenu, InputElementImpl, KeyboardNavigation,
    KeyboardNavigationMode,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElement,
    StyledElementImpl, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A top-level menu control.
#[repr(C)]
pub struct Menu {
    base: MenuBase,
    access_key_handler: RefCell<Option<Rc<dyn IAccessKeyHandler>>>,
    /// The handle given to the access key handler as its main menu.
    main_menu: RefCell<Option<Rc<dyn IMainMenu>>>,
}

ferro_class!(Menu: MenuBase);
ferro_class_info!(Menu { new: Menu::new });

ferro_impl_classes!(
    Menu: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl
);

impl VisualImpl for Menu {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let access_key_handler = TopLevel::get_top_level(Some(this)).and_then(|top_level| top_level.access_key_handler());
        *this.access_key_handler.borrow_mut() = access_key_handler.clone();
        if let Some(access_key_handler) = access_key_handler {
            let main_menu = this.to_main_menu();
            *this.main_menu.borrow_mut() = Some(main_menu.clone());
            access_key_handler.set_main_menu(Some(main_menu));
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        let access_key_handler = this.access_key_handler.borrow_mut().take();
        let main_menu = this.main_menu.borrow_mut().take();
        if let (Some(access_key_handler), Some(main_menu)) = (access_key_handler, main_menu) {
            if access_key_handler.main_menu().is_some_and(|current| Rc::ptr_eq(&current, &main_menu)) {
                access_key_handler.set_main_menu(None);
            }
        }

        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl ItemsControlImpl for Menu {
    fn prepare_container_for_item_override(this: &Self, element: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        Self::parent_prepare_container_for_item_override(this, element, item, index);

        // Child menu items should not inherit the menu's ItemContainerTheme
        // as that is specific for top-level menu items.
        if let Some(menu_item) = element.downcast_ref::<MenuItem>() {
            if menu_item.item_container_theme() == this.item_container_theme() {
                element.clear_value(ItemsControl::item_container_theme_property());
            }
        }
    }
}

impl MenuBaseImpl for Menu {
    fn close(this: &Self) {
        if !this.is_open() {
            return;
        }

        for i in this.to_menu().sub_items() {
            i.close();
        }

        this.set_is_open(false);
        this.set_selected_index(-1);

        this.raise_event(&RoutedEventArgs::with_event_and_source(MenuBase::closed_event(), this.to_ref()));
    }

    fn open(this: &Self) {
        if this.is_open() {
            return;
        }

        this.set_is_open(true);

        this.raise_event(&RoutedEventArgs::with_event_and_source(MenuBase::opened_event(), this.to_ref()));
    }
}

/// The menu viewed as the main menu of a window.
struct MainMenuHandle(Ref<Menu>);

impl IMainMenu for MainMenuHandle {
    fn is_open(&self) -> bool {
        self.0.is_open()
    }

    fn close(&self) {
        self.0.close();
    }

    fn open(&self) {
        self.0.open();
    }

    fn closed(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.0.closed(move |_, e| handler(e));
        let menu = self.0.downgrade();
        Disposable::create(move || {
            if let Some(menu) = menu.upgrade() {
                menu.remove_handler(MenuBase::closed_event(), token);
            }
        })
    }
}

impl Menu {
    fn static_constructor() {
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| {
            let panel = StackPanel::new();
            panel.set_orientation(Orientation::Horizontal);
            Some(panel.upcast::<Panel>())
        });

        ItemsControl::items_panel_property().override_default_value::<Menu>(default_panel);
        KeyboardNavigation::tab_navigation_property().override_default_value::<Menu>(KeyboardNavigationMode::Once);
        crate::automation::AutomationProperties::accessibility_view_property()
            .override_default_value::<Menu>(crate::automation::AccessibilityView::Control);
        crate::automation::AutomationProperties::control_type_override_property()
            .override_default_value::<Menu>(Some(crate::automation::peers::AutomationControlType::Menu));
        AccessKeyHandler::access_key_pressed_event().add_class_handler::<Menu>(Self::on_access_key_pressed);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: MenuBase::construct(), access_key_handler: RefCell::new(None), main_menu: RefCell::new(None) }
    }

    /// Creates the class data of a menu that uses `interaction_handler`.
    pub fn construct_with(interaction_handler: Rc<dyn IMenuInteractionHandler>) -> Self {
        Self { base: MenuBase::construct_with(interaction_handler), access_key_handler: RefCell::new(None), main_menu: RefCell::new(None) }
    }

    /// Initializes a new instance of the [`Menu`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Initializes a new instance of the [`Menu`] class that uses
    /// `interaction_handler`.
    pub fn with_interaction_handler(interaction_handler: Rc<dyn IMenuInteractionHandler>) -> Ref<Self> {
        instantiate(Self::construct_with(interaction_handler))
    }

    /// The menu viewed as [`IMainMenu`].
    pub(crate) fn to_main_menu(&self) -> Rc<dyn IMainMenu> {
        Rc::new(MainMenuHandle(self.to_ref()))
    }

    fn on_access_key_pressed(_sender: &Menu, e: &AccessKeyPressedEventArgs) {
        if e.handled() {
            return;
        }
        let Some(target) = e.source().and_then(|source| source.cast::<StyledElement>()) else { return };

        e.set_target(DefaultMenuInteractionHandler::get_menu_item_core(Some(target)).map(|item| item.element()));
        e.set_handled(true);
    }
}
