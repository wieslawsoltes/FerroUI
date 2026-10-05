use super::IMenuInteractionHandler;
use crate::i_menu::IMenu;
use crate::i_menu_element::{same_menu_element, same_menu_item, IMenuElement};
use crate::i_menu_item::{as_menu_item, IMenuItem};
use crate::primitives::Popup;
use crate::radio_button_group_manager::{as_radio_button, IRadioButton, RadioButtonGroupManager};
use crate::{Control, Menu, MenuBase, MenuItem, MenuItemToggleType, TopLevel, WindowBase};
use ferroui_base::input::raw::{IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{
    AccessKeyEventArgs, AccessKeyHandler, FocusChangedEventArgs, IInputManager, InputElement, InputManager, Key,
    KeyEventArgs, KeyModifiers, MouseButton, NavigationDirection, NavigationMethod, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{ObjectType, Ref, StyledElement, Visual};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Runs an action after a delay: the timer used to open and close submenus
/// when the pointer hovers over a menu item.
pub type MenuDelayRun = Rc<dyn Fn(Box<dyn Fn()>, Duration)>;

thread_local! {
    static MENU_SHOW_DELAY: Cell<Duration> = const { Cell::new(Duration::from_millis(400)) };
}

/// The handlers the interaction handler added to the menu it is attached
/// to.
struct MenuHandlers {
    got_focus: RoutedEventHandlerToken,
    lost_focus: RoutedEventHandlerToken,
    key_down: RoutedEventHandlerToken,
    pointer_pressed: RoutedEventHandlerToken,
    pointer_released: RoutedEventHandlerToken,
    access_key: RoutedEventHandlerToken,
    menu_opened: RoutedEventHandlerToken,
    pointer_entered_item: RoutedEventHandlerToken,
    pointer_exited_item: RoutedEventHandlerToken,
    pointer_moved: RoutedEventHandlerToken,
}

/// The overridable members of [`DefaultMenuInteractionHandler`]: the
/// handlers a class deriving from it can replace. Every method defaults to
/// the behaviour of the default handler (`base_*`), which an override calls
/// to run the base implementation.
///
/// Install the overrides with
/// [`DefaultMenuInteractionHandler::with_overrides`].
pub trait DefaultMenuInteractionHandlerOverrides {
    fn got_focus(&self, handler: &DefaultMenuInteractionHandler, e: &FocusChangedEventArgs) {
        handler.base_got_focus(e)
    }

    fn lost_focus(&self, handler: &DefaultMenuInteractionHandler, e: &RoutedEventArgs) {
        handler.base_lost_focus(e)
    }

    fn key_down(&self, handler: &DefaultMenuInteractionHandler, e: &KeyEventArgs) {
        handler.base_key_down(e)
    }

    fn access_key_pressed(&self, handler: &DefaultMenuInteractionHandler, e: &dyn IRoutedEventArgs) {
        handler.base_access_key_pressed(e)
    }

    fn pointer_entered(&self, handler: &DefaultMenuInteractionHandler, e: &RoutedEventArgs) {
        handler.base_pointer_entered(e)
    }

    fn pointer_moved(&self, handler: &DefaultMenuInteractionHandler, e: &PointerEventArgs) {
        handler.base_pointer_moved(e)
    }

    fn pointer_exited(&self, handler: &DefaultMenuInteractionHandler, e: &RoutedEventArgs) {
        handler.base_pointer_exited(e)
    }

    fn pointer_pressed(&self, handler: &DefaultMenuInteractionHandler, sender: Option<&Visual>, e: &PointerPressedEventArgs) {
        handler.base_pointer_pressed(sender, e)
    }

    fn pointer_released(&self, handler: &DefaultMenuInteractionHandler, e: &PointerReleasedEventArgs) {
        handler.base_pointer_released(e)
    }

    fn menu_opened(&self, handler: &DefaultMenuInteractionHandler, e: &RoutedEventArgs) {
        handler.base_menu_opened(e)
    }

    fn raw_input(&self, handler: &DefaultMenuInteractionHandler, e: &dyn IRawInputEventArgs) {
        handler.base_raw_input(e)
    }

    fn root_pointer_pressed(&self, handler: &DefaultMenuInteractionHandler, e: &PointerPressedEventArgs) {
        handler.base_root_pointer_pressed(e)
    }

    fn window_deactivated(&self, handler: &DefaultMenuInteractionHandler) {
        handler.base_window_deactivated()
    }
}

/// Provides the default keyboard and pointer interaction for menus.
pub struct DefaultMenuInteractionHandler {
    this: Weak<DefaultMenuInteractionHandler>,
    is_context_menu: bool,
    input_manager: Option<Rc<dyn IInputManager>>,
    delay_run: MenuDelayRun,
    overrides: Option<Rc<dyn DefaultMenuInteractionHandlerOverrides>>,
    menu: RefCell<Option<Rc<dyn IMenu>>>,
    menu_handlers: RefCell<Option<MenuHandlers>>,
    input_manager_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    root: RefCell<Option<Ref<TopLevel>>>,
    root_pointer_pressed: Cell<Option<RoutedEventHandlerToken>>,
    window_deactivated: RefCell<Option<Rc<dyn IDisposable>>>,
    top_level_lost_platform_focus: RefCell<Option<Rc<dyn IDisposable>>>,
    group_manager: RefCell<Option<Rc<RadioButtonGroupManager>>>,
}

impl IMenuInteractionHandler for DefaultMenuInteractionHandler {
    fn attach(&self, menu: &Ref<MenuBase>) {
        self.attach_core(menu.to_menu());
    }

    fn detach(&self, menu: &Ref<MenuBase>) {
        self.detach_core(&*menu.to_menu());
    }

    fn as_default_menu_interaction_handler(&self) -> Option<&DefaultMenuInteractionHandler> {
        Some(self)
    }
}

impl DefaultMenuInteractionHandler {
    /// Creates an interaction handler that uses the input manager of the
    /// application and a dispatcher timer for delayed actions.
    pub fn new(is_context_menu: bool) -> Rc<Self> {
        Self::with(is_context_menu, InputManager::instance(), Rc::new(Self::default_delay_run))
    }

    /// Creates an interaction handler.
    pub fn with(is_context_menu: bool, input_manager: Option<Rc<dyn IInputManager>>, delay_run: MenuDelayRun) -> Rc<Self> {
        Self::create(is_context_menu, input_manager, delay_run, None)
    }

    /// Creates an interaction handler whose overridable members are
    /// replaced by `overrides`: the form a class deriving from the default
    /// handler takes.
    pub fn with_overrides(
        is_context_menu: bool,
        input_manager: Option<Rc<dyn IInputManager>>,
        delay_run: MenuDelayRun,
        overrides: Rc<dyn DefaultMenuInteractionHandlerOverrides>,
    ) -> Rc<Self> {
        Self::create(is_context_menu, input_manager, delay_run, Some(overrides))
    }

    fn create(
        is_context_menu: bool,
        input_manager: Option<Rc<dyn IInputManager>>,
        delay_run: MenuDelayRun,
        overrides: Option<Rc<dyn DefaultMenuInteractionHandlerOverrides>>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            is_context_menu,
            input_manager,
            delay_run,
            overrides,
            menu: RefCell::new(None),
            menu_handlers: RefCell::new(None),
            input_manager_subscription: RefCell::new(None),
            root: RefCell::new(None),
            root_pointer_pressed: Cell::new(None),
            window_deactivated: RefCell::new(None),
            top_level_lost_platform_focus: RefCell::new(None),
            group_manager: RefCell::new(None),
        })
    }

    /// The function that runs delayed actions.
    pub fn delay_run(&self) -> &MenuDelayRun {
        &self.delay_run
    }

    /// The input manager whose raw input the handler observes.
    pub fn input_manager(&self) -> Option<Rc<dyn IInputManager>> {
        self.input_manager.clone()
    }

    /// The menu the handler is attached to.
    pub(crate) fn menu(&self) -> Option<Rc<dyn IMenu>> {
        self.menu.borrow().clone()
    }

    /// The delay before a submenu opens or closes when the pointer hovers
    /// over a menu item.
    pub fn menu_show_delay() -> Duration {
        MENU_SHOW_DELAY.with(Cell::get)
    }

    pub fn set_menu_show_delay(value: Duration) {
        MENU_SHOW_DELAY.with(|delay| delay.set(value));
    }

    pub fn got_focus(&self, e: &FocusChangedEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.got_focus(self, e),
            None => self.base_got_focus(e),
        }
    }

    pub fn lost_focus(&self, e: &RoutedEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.lost_focus(self, e),
            None => self.base_lost_focus(e),
        }
    }

    pub fn key_down(&self, e: &KeyEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.key_down(self, e),
            None => self.base_key_down(e),
        }
    }

    pub fn access_key_pressed(&self, e: &dyn IRoutedEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.access_key_pressed(self, e),
            None => self.base_access_key_pressed(e),
        }
    }

    pub fn pointer_entered(&self, e: &RoutedEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.pointer_entered(self, e),
            None => self.base_pointer_entered(e),
        }
    }

    pub fn pointer_moved(&self, e: &PointerEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.pointer_moved(self, e),
            None => self.base_pointer_moved(e),
        }
    }

    pub fn pointer_exited(&self, e: &RoutedEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.pointer_exited(self, e),
            None => self.base_pointer_exited(e),
        }
    }

    pub fn pointer_pressed(&self, sender: Option<&Visual>, e: &PointerPressedEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.pointer_pressed(self, sender, e),
            None => self.base_pointer_pressed(sender, e),
        }
    }

    pub fn pointer_released(&self, e: &PointerReleasedEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.pointer_released(self, e),
            None => self.base_pointer_released(e),
        }
    }

    pub fn menu_opened(&self, e: &RoutedEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.menu_opened(self, e),
            None => self.base_menu_opened(e),
        }
    }

    pub fn raw_input(&self, e: &dyn IRawInputEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.raw_input(self, e),
            None => self.base_raw_input(e),
        }
    }

    pub fn root_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        match &self.overrides {
            Some(overrides) => overrides.root_pointer_pressed(self, e),
            None => self.base_root_pointer_pressed(e),
        }
    }

    pub fn window_deactivated(&self) {
        match &self.overrides {
            Some(overrides) => overrides.window_deactivated(self),
            None => self.base_window_deactivated(),
        }
    }

    /// The default implementation of [`got_focus`](Self::got_focus).
    pub fn base_got_focus(&self, e: &FocusChangedEventArgs) {
        let item = Self::get_menu_item_core(Self::source_of(e));

        if let Some(item) = item {
            if item.parent().is_some() {
                item.set_selected_item(Some(item.clone()));
            }
        }
    }

    /// The default implementation of [`lost_focus`](Self::lost_focus).
    pub fn base_lost_focus(&self, e: &RoutedEventArgs) {
        let item = Self::get_menu_item_core(Self::source_of(e));

        if let Some(item) = item {
            item.set_selected_item(None);
        }
    }

    /// The default implementation of [`key_down`](Self::key_down).
    pub fn base_key_down(&self, e: &KeyEventArgs) {
        self.key_down_item(Self::get_menu_item_core(Self::source_of(e)), e);
    }

    /// The default implementation of [`access_key_pressed`](Self::access_key_pressed).
    pub fn base_access_key_pressed(&self, e: &dyn IRoutedEventArgs) {
        let access_key = e.downcast_ref::<AccessKeyEventArgs>();
        let e = e.as_routed_event_args();
        let Some(item) = Self::get_menu_item_core(Self::source_of(e)) else { return };

        if access_key.is_some_and(AccessKeyEventArgs::is_multiple) {
            // in case we have multiple matches, only focus item and bail
            item.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
            return;
        }

        if item.has_sub_menu() && item.element().is_effectively_enabled() {
            self.open(&item, true);
        } else {
            self.click(&item);
        }

        e.set_handled(true);
    }

    /// The default implementation of [`pointer_entered`](Self::pointer_entered).
    pub fn base_pointer_entered(&self, e: &RoutedEventArgs) {
        let Some(item) = Self::get_menu_item_core(Self::source_of(e)) else { return };
        let Some(parent) = item.parent() else { return };

        if item.is_top_level() {
            let selected = parent.selected_item();
            if !same_menu_item(Some(&item), selected.as_ref()) && selected.as_ref().is_some_and(|s| s.is_sub_menu_open())
            {
                if let Some(selected) = selected {
                    selected.close();
                }
                self.select_item_and_ancestors(&item);
                if item.has_sub_menu() {
                    self.open(&item, false);
                }
            } else {
                self.select_item_and_ancestors(&item);
            }
        } else {
            self.select_item_and_ancestors(&item);

            if item.has_sub_menu() {
                self.open_with_delay(&item);
            } else if let Some(parent) = item.parent() {
                for sibling in parent.sub_items() {
                    if sibling.is_sub_menu_open() {
                        self.close_with_delay(&sibling);
                    }
                }
            }
        }
    }

    /// The default implementation of [`pointer_moved`](Self::pointer_moved).
    pub fn base_pointer_moved(&self, e: &PointerEventArgs) {
        // HACK: #8179 needs to be addressed to correctly implement it in the
        // pointer pressed method.
        let item = Self::get_menu_item_core(Self::source_of(e)).and_then(|item| item.element().cast::<MenuItem>());

        let Some(item) = item else { return };

        let Some(transformed_bounds) = item.get_transformed_bounds() else { return };

        let point = e.get_current_point(None);

        if point.properties.is_left_button_pressed && !transformed_bounds.contains(point.position) {
            e.pointer().capture(None);
        }
    }

    /// The default implementation of [`pointer_exited`](Self::pointer_exited).
    pub fn base_pointer_exited(&self, e: &RoutedEventArgs) {
        let Some(item) = Self::get_menu_item_core(Self::source_of(e)) else { return };
        let Some(parent) = item.parent() else { return };

        if same_menu_item(parent.selected_item().as_ref(), Some(&item)) {
            if item.is_top_level() {
                if !parent.as_menu().expect("the parent of a top-level menu item is a menu").is_open() {
                    parent.set_selected_item(None);
                }
            } else if !item.has_sub_menu() {
                parent.set_selected_item(None);
            } else if !item.is_pointer_over_sub_menu() {
                (self.delay_run)(
                    Box::new(move || {
                        if !item.is_pointer_over_sub_menu() {
                            item.set_is_sub_menu_open(false);
                        }
                    }),
                    Self::menu_show_delay(),
                );
            }
        }
    }

    /// `sender` is the element the handler is attached to: the menu.
    /// The default implementation of [`pointer_pressed`](Self::pointer_pressed).
    pub fn base_pointer_pressed(&self, sender: Option<&Visual>, e: &PointerPressedEventArgs) {
        let item = Self::get_menu_item_core(Self::source_of(e));

        let Some(visual) = sender else { return };
        let Some(item) = item else { return };

        if e.get_current_point(Some(visual)).properties.is_left_button_pressed && item.has_sub_menu() {
            if item.is_sub_menu_open() {
                // PointerPressed events may bubble from disabled items in
                // sub-menus. In this case, keep the sub-menu open.
                let popup = e
                    .source()
                    .and_then(|source| source.cast::<StyledElement>())
                    .and_then(|source| find_logical_ancestor_of_type::<Popup>(&source, false));
                if item.is_top_level() && popup.is_none() {
                    self.close_menu(&item);
                }
            } else {
                if item.is_top_level() {
                    if let Some(main_menu) = item.parent().and_then(|parent| parent.as_main_menu()) {
                        main_menu.open();
                    }
                }

                self.open(&item, false);
            }

            e.set_handled(true);
        }
    }

    /// The default implementation of [`pointer_released`](Self::pointer_released).
    pub fn base_pointer_released(&self, e: &PointerReleasedEventArgs) {
        let item = Self::get_menu_item_core(Self::source_of(e));

        if let Some(item) = item {
            if e.initial_press_mouse_button() == MouseButton::Left && !item.has_sub_menu() {
                self.click(&item);
                e.set_handled(true);
            }
        }
    }

    /// The default implementation of [`menu_opened`](Self::menu_opened).
    pub fn base_menu_opened(&self, e: &RoutedEventArgs) {
        if e.source().is_some_and(|source| source.is::<Menu>()) {
            if let Some(menu) = self.menu() {
                menu.move_selection(NavigationDirection::First, true);
            }
        }
    }

    /// The default implementation of [`raw_input`](Self::raw_input).
    pub fn base_raw_input(&self, e: &dyn IRawInputEventArgs) {
        let mouse = e.downcast_ref::<RawPointerEventArgs>();

        if mouse.is_some_and(|mouse| mouse.type_() == RawPointerEventType::NonClientLeftButtonDown) {
            if let Some(menu) = self.menu() {
                menu.close();
            }
        }
    }

    /// The default implementation of [`root_pointer_pressed`](Self::root_pointer_pressed).
    pub fn base_root_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let Some(menu) = self.menu() else { return };

        if menu.is_open() {
            if let Some(control) = e.source().and_then(|source| source.cast::<StyledElement>()) {
                if !is_logical_ancestor_of(&menu.element(), &control) {
                    menu.close();
                }
            }
        }
    }

    /// The default implementation of [`window_deactivated`](Self::window_deactivated).
    pub fn base_window_deactivated(&self) {
        if let Some(menu) = self.menu() {
            menu.close();
        }
    }

    pub(crate) fn attach_core(&self, menu: Rc<dyn IMenu>) {
        if self.menu.borrow().is_some() {
            panic!("DefaultMenuInteractionHandler is already attached.");
        }

        *self.menu.borrow_mut() = Some(menu.clone());

        let element = menu.element();
        let weak_element = element.downgrade();

        macro_rules! handler {
            (|$this:ident, $e:ident| $body:expr) => {{
                let weak = self.this.clone();
                move |_: &ferroui_base::interactivity::Interactive, $e: &_| {
                    if let Some($this) = weak.upgrade() {
                        $body
                    }
                }
            }};
        }

        let handlers = MenuHandlers {
            got_focus: element.add_handler(InputElement::got_focus_event(), handler!(|this, e| this.got_focus(e))),
            lost_focus: element.add_handler(InputElement::lost_focus_event(), handler!(|this, e| this.lost_focus(e))),
            key_down: element.add_handler(InputElement::key_down_event(), handler!(|this, e| this.key_down(e))),
            pointer_pressed: element.add_handler(InputElement::pointer_pressed_event(), {
                let weak = self.this.clone();
                move |_, e: &PointerPressedEventArgs| {
                    if let (Some(this), Some(sender)) = (weak.upgrade(), weak_element.upgrade()) {
                        this.pointer_pressed(Some(&sender), e);
                    }
                }
            }),
            pointer_released: element
                .add_handler(InputElement::pointer_released_event(), handler!(|this, e| this.pointer_released(e))),
            access_key: element.add_handler(AccessKeyHandler::access_key_event(), {
                let weak = self.this.clone();
                move |_, e: &AccessKeyEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.access_key_pressed(e);
                    }
                }
            }),
            menu_opened: element.add_handler(MenuBase::opened_event(), handler!(|this, e| this.menu_opened(e))),
            pointer_entered_item: element
                .add_handler(MenuItem::pointer_entered_item_event(), handler!(|this, e| this.pointer_entered(e))),
            pointer_exited_item: element
                .add_handler(MenuItem::pointer_exited_item_event(), handler!(|this, e| this.pointer_exited(e))),
            pointer_moved: element
                .add_handler(InputElement::pointer_moved_event(), handler!(|this, e| this.pointer_moved(e))),
        };
        *self.menu_handlers.borrow_mut() = Some(handlers);

        let root = menu.top_level();
        *self.root.borrow_mut() = root.clone();

        if let Some(root) = &root {
            let group_manager = RadioButtonGroupManager::get_or_create_for_root(Visual::presentation_source(root).as_ref());
            *self.group_manager.borrow_mut() = Some(group_manager.clone());
            let element: Rc<dyn IMenuElement> = menu.clone();
            Self::add_menu_item_to_radio_group(&group_manager, &*element);

            let token = root.add_handler_with(
                InputElement::pointer_pressed_event(),
                handler!(|this, e| this.root_pointer_pressed(e)),
                RoutingStrategies::TUNNEL,
                false,
            );
            self.root_pointer_pressed.set(Some(token));

            if let Some(window) = root.downcast_ref::<WindowBase>() {
                let weak = self.this.clone();
                let subscription = window.deactivated(move || {
                    if let Some(this) = weak.upgrade() {
                        this.window_deactivated();
                    }
                });
                *self.window_deactivated.borrow_mut() = Some(subscription);
            }

            if root.platform_impl().is_some() {
                let weak = self.this.clone();
                let subscription = root.platform_lost_focus(move || {
                    if let Some(this) = weak.upgrade() {
                        this.top_level_lost_platform_focus();
                    }
                });
                *self.top_level_lost_platform_focus.borrow_mut() = Some(subscription);
            }
        }

        if let Some(input_manager) = &self.input_manager {
            let weak = self.this.clone();
            let subscription = input_manager.process().subscribe_fn(move |e: Rc<dyn IRawInputEventArgs>| {
                if let Some(this) = weak.upgrade() {
                    this.raw_input(&*e);
                }
            });
            *self.input_manager_subscription.borrow_mut() = Some(subscription);
        }
    }

    pub(crate) fn detach_core(&self, menu: &dyn IMenu) {
        let attached = self.menu();
        let Some(attached) = attached.filter(|attached| same_menu_element(&**attached, menu)) else {
            panic!("DefaultMenuInteractionHandler is not attached to the menu.");
        };

        let element = attached.element();
        let handlers = self.menu_handlers.borrow_mut().take();
        if let Some(handlers) = handlers {
            element.remove_handler(InputElement::got_focus_event(), handlers.got_focus);
            element.remove_handler(InputElement::lost_focus_event(), handlers.lost_focus);
            element.remove_handler(InputElement::key_down_event(), handlers.key_down);
            element.remove_handler(InputElement::pointer_pressed_event(), handlers.pointer_pressed);
            element.remove_handler(InputElement::pointer_released_event(), handlers.pointer_released);
            element.remove_handler(AccessKeyHandler::access_key_event(), handlers.access_key);
            element.remove_handler(MenuBase::opened_event(), handlers.menu_opened);
            element.remove_handler(MenuItem::pointer_entered_item_event(), handlers.pointer_entered_item);
            element.remove_handler(MenuItem::pointer_exited_item_event(), handlers.pointer_exited_item);
            element.remove_handler(InputElement::pointer_moved_event(), handlers.pointer_moved);
        }

        let root = self.root.borrow().clone();

        if root.is_some() {
            let old_manager = self.group_manager.borrow_mut().take();
            if let Some(old_manager) = old_manager {
                Self::remove_menu_item_from_radio_group(&old_manager, menu);
            }
        }

        if let Some(root) = &root {
            if let Some(token) = self.root_pointer_pressed.take() {
                root.remove_handler(InputElement::pointer_pressed_event(), token);
            }
        }

        let window_deactivated = self.window_deactivated.borrow_mut().take();
        if let Some(subscription) = window_deactivated {
            subscription.dispose();
        }

        let lost_platform_focus = self.top_level_lost_platform_focus.borrow_mut().take();
        if let Some(subscription) = lost_platform_focus {
            subscription.dispose();
        }

        let input_manager_subscription = self.input_manager_subscription.borrow_mut().take();
        if let Some(subscription) = input_manager_subscription {
            subscription.dispose();
        }

        let attached = self.menu.borrow_mut().take();
        drop(attached);
        let root = self.root.borrow_mut().take();
        drop(root);
    }

    pub(crate) fn click(&self, item: &Rc<dyn IMenuItem>) {
        if !item.has_sub_menu() {
            if item.toggle_type() == MenuItemToggleType::CheckBox {
                let new_value = !item.is_checked();
                item.set_is_checked(new_value);
            } else if item.toggle_type() == MenuItemToggleType::Radio && !item.is_checked() {
                item.set_is_checked(true);
            }
        }

        item.raise_click();

        if !item.stays_open_on_click() {
            self.close_menu(item);
        }
    }

    pub(crate) fn close_menu(&self, item: &Rc<dyn IMenuItem>) {
        let mut current: Option<Rc<dyn IMenuElement>> = Some(item.clone());

        while let Some(element) = current.clone() {
            if element.as_menu().is_some() {
                break;
            }
            current = element.as_menu_item().and_then(|item| item.parent());
        }

        if let Some(current) = current {
            current.close();
        }
    }

    pub(crate) fn close_with_delay(&self, item: &Rc<dyn IMenuItem>) {
        let item = item.clone();
        let execute = move || {
            let selected = item.parent().and_then(|parent| parent.selected_item());
            if !same_menu_item(selected.as_ref(), Some(&item)) {
                item.close();
            }
        };

        (self.delay_run)(Box::new(execute), Self::menu_show_delay());
    }

    pub(crate) fn key_down_item(&self, item: Option<Rc<dyn IMenuItem>>, e: &KeyEventArgs) {
        let mut default = false;

        match e.key {
            Key::Up | Key::Down => match &item {
                Some(item) if item.is_top_level() && item.has_sub_menu() => {
                    if !item.is_sub_menu_open() {
                        self.open(item, true);
                    } else {
                        item.move_selection(NavigationDirection::First, true);
                    }

                    e.set_handled(true);
                }
                _ => default = true,
            },

            Key::Left => {
                let parent_item = item
                    .as_ref()
                    .and_then(|item| item.parent())
                    .and_then(|parent| parent.as_menu_item())
                    .filter(|parent| !parent.is_top_level() && parent.is_sub_menu_open());

                if let Some(item) =
                    item.as_ref().filter(|item| item.is_sub_menu_open() && item.selected_item().is_none())
                {
                    item.close();
                } else if let Some(parent) = parent_item {
                    parent.close();
                    parent.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE);
                    e.set_handled(true);
                } else {
                    default = true;
                }
            }

            Key::Right => match &item {
                Some(item) if !item.is_top_level() && item.has_sub_menu() => {
                    self.open(item, true);
                    e.set_handled(true);
                }
                _ => default = true,
            },

            Key::Enter => {
                if let Some(item) = &item {
                    if !item.has_sub_menu() {
                        self.click(item);
                    } else {
                        self.open(item, true);
                    }

                    e.set_handled(true);
                }
            }

            Key::Escape => {
                if let Some(parent) = item.as_ref().and_then(|item| item.parent()) {
                    parent.close();
                    parent.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE);
                } else {
                    self.menu().expect("the interaction handler is attached to a menu").close();
                }

                e.set_handled(true);
            }

            _ => default = true,
        }

        if default {
            let direction = e.key.to_navigation_direction(KeyModifiers::NONE);

            if let Some(direction) = direction.filter(|direction| direction.is_directional()) {
                if item.is_none() && self.is_context_menu {
                    if self
                        .menu()
                        .expect("the interaction handler is attached to a menu")
                        .move_selection(direction, true)
                    {
                        e.set_handled(true);
                    }
                } else if let Some(item) = &item {
                    if let Some(parent) = item.parent() {
                        if parent.move_selection(direction, true) {
                            // If the parent is an IMenu which successfully
                            // moved its selection, and the current menu is
                            // open then close the current menu and open the
                            // new menu.
                            if item.is_sub_menu_open() && parent.as_menu().is_some() {
                                if let Some(selected) = parent.selected_item() {
                                    if !same_menu_item(Some(&selected), Some(item)) {
                                        item.close();
                                        self.open(&selected, true);
                                    }
                                }
                            }
                            e.set_handled(true);
                        }
                    }
                }
            }
        }

        if !e.handled() {
            if let Some(parent_item) = item.and_then(|item| item.parent()).and_then(|parent| parent.as_menu_item()) {
                self.key_down_item(Some(parent_item), e);
            }
        }
    }

    pub(crate) fn open(&self, item: &Rc<dyn IMenuItem>, select_first: bool) {
        item.open();

        if select_first {
            item.move_selection(NavigationDirection::First, true);
        }
    }

    pub(crate) fn open_with_delay(&self, item: &Rc<dyn IMenuItem>) {
        let weak = self.this.clone();
        let item = item.clone();
        let execute = move || {
            let selected = item.parent().and_then(|parent| parent.selected_item());
            if same_menu_item(selected.as_ref(), Some(&item)) {
                if let Some(this) = weak.upgrade() {
                    this.open(&item, false);
                }
            }
        };

        (self.delay_run)(Box::new(execute), Self::menu_show_delay());
    }

    pub(crate) fn select_item_and_ancestors(&self, item: &Rc<dyn IMenuItem>) {
        let mut current = Some(item.clone());

        while let Some(item) = current {
            let Some(parent) = item.parent() else { break };
            parent.set_selected_item(Some(item));
            current = parent.as_menu_item();
        }
    }

    pub(crate) fn on_checked_changed(&self, item: &dyn IMenuItem) {
        if let Some(radio_button) = as_radio_button(&item.element()) {
            let group_manager = self.group_manager.borrow().clone();
            if let Some(group_manager) = group_manager {
                group_manager.on_checked_changed(&*radio_button);
            }
        }
    }

    pub(crate) fn on_group_or_type_changed(&self, button: &dyn IRadioButton, old_group_name: Option<&str>) {
        let group_manager = self.group_manager.borrow().clone();
        let Some(group_manager) = group_manager else { return };

        if old_group_name.is_some_and(|name| !name.is_empty()) {
            group_manager.remove(button, old_group_name);
        }
        if button.group_name().is_some_and(|name| !name.is_empty()) {
            group_manager.add(button);
        }
    }

    /// The menu item `item` is, or is within.
    pub(crate) fn get_menu_item_core(item: Option<Ref<StyledElement>>) -> Option<Rc<dyn IMenuItem>> {
        let mut item = item;
        loop {
            let current = item?;
            if let Some(menu_item) = as_menu_item(&current) {
                return Some(menu_item);
            }
            item = current.parent();
        }
    }

    /// The source of a routed event, when it is a control.
    fn source_of(e: &RoutedEventArgs) -> Option<Ref<StyledElement>> {
        e.source().and_then(|source| source.cast::<Control>()).map(|control| control.upcast())
    }

    fn top_level_lost_platform_focus(&self) {
        if let Some(menu) = self.menu() {
            menu.close();
        }
    }

    fn default_delay_run(action: Box<dyn Fn()>, time_span: Duration) {
        DispatcherTimer::run_once(action, time_span, DispatcherPriority::default());
    }

    fn add_menu_item_to_radio_group(manager: &RadioButtonGroupManager, element: &dyn IMenuElement) {
        // Instead add menu item to the group on attached/detached + ensure
        // checked stated on attached.
        if let Some(button) = as_radio_button(&element.element()) {
            manager.add(&*button);
        }

        for sub_item in element.sub_items() {
            let sub_item: Rc<dyn IMenuElement> = sub_item;
            Self::add_menu_item_to_radio_group(manager, &*sub_item);
        }
    }

    fn remove_menu_item_from_radio_group(manager: &RadioButtonGroupManager, element: &dyn IMenuElement) {
        if let Some(button) = as_radio_button(&element.element()) {
            manager.remove(&*button, button.group_name().as_deref());
        }

        for sub_item in element.sub_items() {
            let sub_item: Rc<dyn IMenuElement> = sub_item;
            Self::remove_menu_item_from_radio_group(manager, &*sub_item);
        }
    }
}

/// Finds the first logical ancestor of `element` of type `T`, optionally
/// starting at the element itself.
pub(crate) fn find_logical_ancestor_of_type<T: ObjectType>(element: &Ref<StyledElement>, include_self: bool) -> Option<Ref<T>> {
    let mut current = if include_self { Some(element.clone()) } else { element.parent() };

    while let Some(logical) = current {
        if let Some(result) = logical.clone().cast::<T>() {
            return Some(result);
        }
        current = logical.parent();
    }

    None
}

/// Whether `logical` is a (strict) logical ancestor of `target`.
pub(crate) fn is_logical_ancestor_of(logical: &StyledElement, target: &StyledElement) -> bool {
    let mut current = target.parent();

    while let Some(parent) = current {
        if std::ptr::eq::<StyledElement>(&*parent, logical) {
            return true;
        }
        current = parent.parent();
    }

    false
}
