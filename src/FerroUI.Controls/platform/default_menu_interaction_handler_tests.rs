//! Tests of the default menu interaction handler, driven through recording
//! mocks of the menu contracts.

use super::default_menu_interaction_handler::{
    DefaultMenuInteractionHandler, DefaultMenuInteractionHandlerOverrides, MenuDelayRun,
};
use super::IMenuInteractionHandler;
use crate::i_menu::{register_menu, IMenu};
use crate::i_menu_element::IMenuElement;
use crate::i_menu_item::{register_menu_item, IMenuItem};
use crate::primitives::Popup;
use crate::test_support::test_scope;
use crate::{Control, ControlImpl, Menu, MenuItem, MenuItemToggleType, TopLevel};
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingMode, BindingPriority, ReflectionBinding};
use ferroui_base::input::{
    IMainMenu, IPointer, InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers, MouseButton,
    NavigationDirection, NavigationMethod, Pointer, PointerPointProperties, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_model, instantiate, BoxedValue, FerroObjectImpl, Point,
    Ref, StyledElementImpl, Visual, VisualImpl,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Recording mocks
// ---------------------------------------------------------------------------

/// A call made on a mock through one of the menu contracts.
#[derive(Clone, PartialEq)]
enum Call {
    Open,
    Close,
    MoveSelection(NavigationDirection, bool),
    RaiseClick,
    /// The element of the item the selection was set to.
    SetSelectedItem(Option<Ref<InputElement>>),
    SetIsSubMenuOpen(bool),
    SetIsChecked(bool),
    #[allow(dead_code)]
    SetStaysOpenOnClick(bool),
    /// `Focus` with any navigation method and any key modifiers.
    Focus,
    MainMenuOpen,
    MainMenuClose,
}

/// A configured result of `move_selection`, with the callback run when the
/// call is made.
struct MoveSelectionSetup {
    direction: NavigationDirection,
    wrap: bool,
    result: bool,
    callback: Option<Rc<dyn Fn()>>,
}

/// The configured values and the recorded calls of a mock. It lives in the
/// control instance, so every handle of the control sees the same state.
#[derive(Default)]
struct MockState {
    is_top_level: Cell<bool>,
    has_sub_menu: Cell<bool>,
    is_sub_menu_open: Cell<bool>,
    is_pointer_over_sub_menu: Cell<bool>,
    is_open: Cell<bool>,
    parent: RefCell<Option<Rc<dyn IMenuElement>>>,
    /// Whether the selected item is a settable property that remembers its
    /// value.
    tracks_selected_item: Cell<bool>,
    selected_item: RefCell<Option<Rc<dyn IMenuItem>>>,
    /// The configured result of the selected item getter, which takes
    /// precedence over the remembered value.
    selected_item_getter: RefCell<Option<Option<Rc<dyn IMenuItem>>>>,
    sub_items: RefCell<Vec<Rc<dyn IMenuItem>>>,
    move_selection: RefCell<Vec<MoveSelectionSetup>>,
    invocations: RefCell<Vec<Call>>,
}

impl MockState {
    fn record(&self, call: Call) {
        self.invocations.borrow_mut().push(call);
    }
}

macro_rules! mock_class {
    ($name:ident, $register:expr) => {
        #[repr(C)]
        struct $name {
            base: Control,
            state: Rc<MockState>,
        }

        ferro_class!($name: Control);
        ferro_class_info!($name { new: $name::new });
        ferro_impl_classes!(
            $name: FerroObjectImpl,
            StyledElementImpl,
            VisualImpl,
            InteractiveImpl,
            InputElementImpl,
            LayoutableImpl,
            ControlImpl
        );

        impl $name {
            fn static_constructor() {
                $register
            }

            fn new() -> Ref<Self> {
                instantiate(Self { base: Control::construct(), state: Rc::new(MockState::default()) })
            }
        }
    };
}

// A control that is a menu item.
mock_class!(
    MockMenuItem,
    register_menu_item::<MockMenuItem>(|item| {
        Rc::new(MockHandle { kind: Kind::MenuItem, state: item.state.clone(), element: item.upcast() })
    })
);

// A control that is a menu.
mock_class!(
    MockMenu,
    register_menu::<MockMenu>(|menu| {
        Rc::new(MockHandle { kind: Kind::Menu, state: menu.state.clone(), element: menu.upcast() })
    })
);

// A control that is a menu element and a main menu, but not a menu.
mock_class!(MockMainMenu, ());

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    MenuItem,
    Menu,
    MainMenu,
}

/// The view of a mock control through the menu contracts.
struct MockHandle {
    kind: Kind,
    element: Ref<InputElement>,
    state: Rc<MockState>,
}

impl MockHandle {
    fn duplicate(&self) -> Rc<MockHandle> {
        Rc::new(MockHandle { kind: self.kind, element: self.element.clone(), state: self.state.clone() })
    }
}

impl IMenuElement for MockHandle {
    fn element(&self) -> Ref<InputElement> {
        self.element.clone()
    }

    fn selected_item(&self) -> Option<Rc<dyn IMenuItem>> {
        if let Some(configured) = self.state.selected_item_getter.borrow().clone() {
            return configured;
        }
        self.state.selected_item.borrow().clone()
    }

    fn set_selected_item(&self, value: Option<Rc<dyn IMenuItem>>) {
        self.state.record(Call::SetSelectedItem(value.as_ref().map(|item| item.element())));
        if self.state.tracks_selected_item.get() {
            *self.state.selected_item.borrow_mut() = value;
        }
    }

    fn sub_items(&self) -> Vec<Rc<dyn IMenuItem>> {
        self.state.sub_items.borrow().clone()
    }

    fn open(&self) {
        self.state.record(Call::Open);
    }

    fn close(&self) {
        self.state.record(Call::Close);
    }

    fn move_selection(&self, direction: NavigationDirection, wrap: bool) -> bool {
        self.state.record(Call::MoveSelection(direction, wrap));
        let setup = self
            .state
            .move_selection
            .borrow()
            .iter()
            .find(|setup| setup.direction == direction && setup.wrap == wrap)
            .map(|setup| (setup.result, setup.callback.clone()));
        match setup {
            Some((result, callback)) => {
                if let Some(callback) = callback {
                    callback();
                }
                result
            }
            None => false,
        }
    }

    fn focus_with(&self, _method: NavigationMethod, _key_modifiers: KeyModifiers) -> bool {
        self.state.record(Call::Focus);
        false
    }

    fn as_menu(&self) -> Option<Rc<dyn IMenu>> {
        if self.kind == Kind::Menu {
            Some(self.duplicate())
        } else {
            None
        }
    }

    fn as_menu_item(&self) -> Option<Rc<dyn IMenuItem>> {
        if self.kind == Kind::MenuItem {
            Some(self.duplicate())
        } else {
            None
        }
    }

    fn as_main_menu(&self) -> Option<Rc<dyn IMainMenu>> {
        if self.kind == Kind::MainMenu {
            Some(self.duplicate())
        } else {
            None
        }
    }
}

impl IMenuItem for MockHandle {
    fn has_sub_menu(&self) -> bool {
        self.state.has_sub_menu.get()
    }

    fn is_pointer_over_sub_menu(&self) -> bool {
        self.state.is_pointer_over_sub_menu.get()
    }

    fn is_sub_menu_open(&self) -> bool {
        self.state.is_sub_menu_open.get()
    }

    fn set_is_sub_menu_open(&self, value: bool) {
        self.state.record(Call::SetIsSubMenuOpen(value));
    }

    fn stays_open_on_click(&self) -> bool {
        false
    }

    fn set_stays_open_on_click(&self, value: bool) {
        self.state.record(Call::SetStaysOpenOnClick(value));
    }

    fn is_top_level(&self) -> bool {
        self.state.is_top_level.get()
    }

    fn parent(&self) -> Option<Rc<dyn IMenuElement>> {
        self.state.parent.borrow().clone()
    }

    fn toggle_type(&self) -> MenuItemToggleType {
        MenuItemToggleType::default()
    }

    fn group_name(&self) -> Option<String> {
        None
    }

    fn is_checked(&self) -> bool {
        false
    }

    fn set_is_checked(&self, value: bool) {
        self.state.record(Call::SetIsChecked(value));
    }

    fn raise_click(&self) {
        self.state.record(Call::RaiseClick);
    }
}

impl IMenu for MockHandle {
    fn interaction_handler(&self) -> Rc<dyn IMenuInteractionHandler> {
        DefaultMenuInteractionHandler::new(false)
    }

    fn is_open(&self) -> bool {
        self.state.is_open.get()
    }

    fn top_level(&self) -> Option<Ref<TopLevel>> {
        None
    }
}

impl IMainMenu for MockHandle {
    fn is_open(&self) -> bool {
        self.state.is_open.get()
    }

    fn close(&self) {
        self.state.record(Call::MainMenuClose);
    }

    fn open(&self) {
        self.state.record(Call::MainMenuOpen);
    }

    fn closed(&self, _handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

/// A mock control together with its state: what the tests configure and
/// verify.
struct Mock {
    control: Ref<Control>,
    handle: Rc<MockHandle>,
}

impl Mock {
    fn of(kind: Kind, control: Ref<Control>, state: Rc<MockState>) -> Self {
        let handle = Rc::new(MockHandle { kind, element: control.clone().upcast(), state });
        Self { control, handle }
    }

    fn state(&self) -> &MockState {
        &self.handle.state
    }

    /// The mocked object as a menu item.
    fn item(&self) -> Rc<dyn IMenuItem> {
        assert!(self.handle.kind == Kind::MenuItem);
        self.handle.clone()
    }

    /// The mocked object as a menu.
    fn menu(&self) -> Rc<dyn IMenu> {
        assert!(self.handle.kind == Kind::Menu);
        self.handle.clone()
    }

    /// The mocked object as a menu element.
    fn element(&self) -> Rc<dyn IMenuElement> {
        self.handle.clone()
    }

    fn input_element(&self) -> Ref<InputElement> {
        self.handle.element.clone()
    }

    fn visual(&self) -> Ref<Visual> {
        self.control.clone().upcast()
    }

    /// Configures the result of the selected item getter.
    fn setup_get_selected_item(&self, value: Option<&Mock>) {
        *self.state().selected_item_getter.borrow_mut() = Some(value.map(|mock| mock.item()));
    }

    fn setup_move_selection(&self, direction: NavigationDirection, wrap: bool, result: bool) {
        self.state().move_selection.borrow_mut().push(MoveSelectionSetup { direction, wrap, result, callback: None });
    }

    fn setup_move_selection_with_callback(
        &self,
        direction: NavigationDirection,
        wrap: bool,
        result: bool,
        callback: impl Fn() + 'static,
    ) {
        self.state().move_selection.borrow_mut().push(MoveSelectionSetup {
            direction,
            wrap,
            result,
            callback: Some(Rc::new(callback)),
        });
    }

    fn was_called(&self, call: &Call) -> bool {
        self.state().invocations.borrow().contains(call)
    }

    /// Asserts that the call was made at least once.
    fn verify(&self, call: Call) {
        assert!(self.was_called(&call), "the expected call was not made");
    }

    /// Asserts that the call was never made.
    fn verify_never(&self, call: Call) {
        assert!(!self.was_called(&call), "a call that should never be made was made");
    }

    /// Asserts that the selected item was set to the given item.
    fn verify_set_selected_item(&self, value: Option<&Mock>) {
        self.verify(Call::SetSelectedItem(value.map(Mock::input_element)));
    }

    /// Asserts that the selected item was never set to the given item.
    fn verify_set_selected_item_never(&self, value: Option<&Mock>) {
        self.verify_never(Call::SetSelectedItem(value.map(Mock::input_element)));
    }

    fn clear_invocations(&self) {
        self.state().invocations.borrow_mut().clear();
    }
}

/// A menu mock whose properties remember the values they are set to.
fn mock_of_menu() -> Mock {
    let mock = new_mock_menu();
    mock.state().tracks_selected_item.set(true);
    mock
}

/// A menu mock with nothing set up.
fn new_mock_menu() -> Mock {
    let menu = MockMenu::new();
    let state = menu.state.clone();
    Mock::of(Kind::Menu, menu.upcast(), state)
}

fn create_mock_main_menu() -> Mock {
    let menu = MockMainMenu::new();
    let state = menu.state.clone();
    Mock::of(Kind::MainMenu, menu.upcast(), state)
}

/// The arguments of `create_mock_menu_item`.
#[derive(Default)]
struct ItemSetup {
    is_top_level: bool,
    has_sub_menu: bool,
    is_sub_menu_open: bool,
    parent: Option<Rc<dyn IMenuElement>>,
}

fn create_mock_menu_item(setup: ItemSetup) -> Mock {
    let item = MockMenuItem::new();
    let state = item.state.clone();
    state.is_top_level.set(setup.is_top_level);
    state.has_sub_menu.set(setup.has_sub_menu);
    state.is_sub_menu_open.set(setup.is_sub_menu_open);
    *state.parent.borrow_mut() = setup.parent;
    state.tracks_selected_item.set(true);
    Mock::of(Kind::MenuItem, item.upcast(), state)
}

/// Holds the delayed action of the handler until the test runs it.
#[derive(Clone, Default)]
struct TestTimer {
    action: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
}

impl TestTimer {
    fn new() -> Self {
        Self::default()
    }

    fn action_is_queued(&self) -> bool {
        self.action.borrow().is_some()
    }

    fn pulse(&self) {
        let action = self.action.borrow().clone();
        let action = action.expect("an action is queued");
        action();
        *self.action.borrow_mut() = None;
    }

    fn run_once(&self) -> MenuDelayRun {
        let slot = self.action.clone();
        Rc::new(move |action: Box<dyn Fn()>, _time_span: Duration| {
            if slot.borrow().is_some() {
                panic!("Action already set.");
            }

            *slot.borrow_mut() = Some(Rc::from(action));
        })
    }
}

struct FakePointer {
    id: i32,
    captured: RefCell<Option<Ref<InputElement>>>,
}

impl FakePointer {
    fn new() -> Rc<Self> {
        Rc::new(Self { id: Pointer::get_next_free_id(), captured: RefCell::new(None) })
    }
}

impl IPointer for FakePointer {
    fn id(&self) -> i32 {
        self.id
    }

    fn capture(&self, control: Option<&Ref<InputElement>>) {
        *self.captured.borrow_mut() = control.cloned();
    }

    fn captured(&self) -> Option<Ref<InputElement>> {
        self.captured.borrow().clone()
    }

    fn type_(&self) -> PointerType {
        PointerType::Mouse
    }

    fn is_primary(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn create_pressed(source: &Ref<Control>) -> PointerPressedEventArgs {
    let visual: Ref<Visual> = source.clone().upcast();
    PointerPressedEventArgs::new(
        source,
        FakePointer::new(),
        &visual,
        Point::default(),
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE,
        1,
    )
}

fn create_released(source: &Ref<Control>) -> PointerReleasedEventArgs {
    let visual: Ref<Visual> = source.clone().upcast();
    PointerReleasedEventArgs::new(
        source,
        FakePointer::new(),
        &visual,
        Point::default(),
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased),
        KeyModifiers::NONE,
        MouseButton::Left,
    )
}

fn key_event(key: Key, source: &Ref<Control>) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_source(source);
    e
}

// ---------------------------------------------------------------------------
// TopLevel
// ---------------------------------------------------------------------------

mod top_level {
    use super::*;

    #[test]
    fn up_opens_menu_item_with_sub_menu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let item = create_mock_menu_item(ItemSetup { is_top_level: true, has_sub_menu: true, ..Default::default() });
        let e = key_event(Key::Up, &item.control);

        target.key_down(&e);

        item.verify(Call::Open);
        item.verify(Call::MoveSelection(NavigationDirection::First, true));
        assert!(e.handled());
    }

    #[test]
    fn down_opens_menu_item_with_sub_menu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let item = create_mock_menu_item(ItemSetup { is_top_level: true, has_sub_menu: true, ..Default::default() });
        let e = key_event(Key::Down, &item.control);

        target.key_down_item(Some(item.item()), &e);

        item.verify(Call::Open);
        item.verify(Call::MoveSelection(NavigationDirection::First, true));
        assert!(e.handled());
    }

    #[test]
    fn down_selects_first_item_of_already_opened_submenu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let item = create_mock_menu_item(ItemSetup {
            is_top_level: true,
            has_sub_menu: true,
            is_sub_menu_open: true,
            ..Default::default()
        });
        let e = key_event(Key::Down, &item.control);

        target.key_down(&e);

        item.verify(Call::MoveSelection(NavigationDirection::First, true));
        assert!(e.handled());
    }

    #[test]
    fn right_selects_next_menu_item() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        menu.setup_move_selection(NavigationDirection::Right, true, true);
        let item =
            create_mock_menu_item(ItemSetup { is_top_level: true, parent: Some(menu.element()), ..Default::default() });
        let e = key_event(Key::Right, &item.control);

        target.key_down(&e);

        menu.verify(Call::MoveSelection(NavigationDirection::Right, true));
        assert!(e.handled());
    }

    #[test]
    fn left_selects_previous_menu_item() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        menu.setup_move_selection(NavigationDirection::Left, true, true);
        let item =
            create_mock_menu_item(ItemSetup { is_top_level: true, parent: Some(menu.element()), ..Default::default() });
        let e = key_event(Key::Left, &item.control);

        target.key_down(&e);

        menu.verify(Call::MoveSelection(NavigationDirection::Left, true));
        assert!(e.handled());
    }

    #[test]
    fn enter_on_item_with_no_sub_menu_causes_click() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let item =
            create_mock_menu_item(ItemSetup { is_top_level: true, parent: Some(menu.element()), ..Default::default() });
        let e = key_event(Key::Enter, &item.control);

        target.key_down(&e);

        item.verify(Call::RaiseClick);
        menu.verify(Call::Close);
        assert!(e.handled());
    }

    #[test]
    fn enter_on_item_with_sub_menu_opens_sub_menu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let item = create_mock_menu_item(ItemSetup {
            is_top_level: true,
            has_sub_menu: true,
            parent: Some(menu.element()),
            ..Default::default()
        });
        let e = key_event(Key::Enter, &item.control);

        target.key_down(&e);

        item.verify(Call::Open);
        item.verify(Call::MoveSelection(NavigationDirection::First, true));
        assert!(e.handled());
    }

    #[test]
    fn escape_closes_parent_menu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let item =
            create_mock_menu_item(ItemSetup { is_top_level: true, parent: Some(menu.element()), ..Default::default() });
        let e = key_event(Key::Escape, &item.control);

        target.key_down(&e);

        menu.verify(Call::Close);
        assert!(e.handled());
    }

    #[test]
    fn click_on_top_level_calls_main_menu_open() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = create_mock_main_menu();
        let item = create_mock_menu_item(ItemSetup {
            is_top_level: true,
            has_sub_menu: true,
            parent: Some(menu.element()),
            ..Default::default()
        });

        let e = create_pressed(&item.control);

        target.pointer_pressed(Some(&item.visual()), &e);
        menu.verify(Call::MainMenuOpen);
    }

    #[test]
    fn click_on_open_top_level_menu_closes_menu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let item = create_mock_menu_item(ItemSetup {
            is_top_level: true,
            has_sub_menu: true,
            is_sub_menu_open: true,
            parent: Some(menu.element()),
        });

        let e = create_pressed(&item.control);

        target.pointer_pressed(Some(&item.visual()), &e);
        menu.verify(Call::Close);
    }

    #[test]
    fn pointer_entered_opens_item_when_old_item_is_open() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = new_mock_menu();
        let item = create_mock_menu_item(ItemSetup {
            is_top_level: true,
            has_sub_menu: true,
            is_sub_menu_open: true,
            parent: Some(menu.element()),
        });
        let next_item = create_mock_menu_item(ItemSetup {
            is_top_level: true,
            has_sub_menu: true,
            parent: Some(menu.element()),
            ..Default::default()
        });
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_entered_item_event(), &next_item.control);

        menu.setup_get_selected_item(Some(&item));

        target.pointer_entered(&e);

        item.verify(Call::Close);
        menu.verify_set_selected_item(Some(&next_item));
        next_item.verify(Call::Open);
        next_item.verify_never(Call::MoveSelection(NavigationDirection::First, true));
        assert!(!e.handled());
    }

    #[test]
    fn pointer_exited_deselects_item_when_menu_not_open() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = new_mock_menu();
        let item =
            create_mock_menu_item(ItemSetup { is_top_level: true, parent: Some(menu.element()), ..Default::default() });
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_exited_item_event(), &item.control);

        menu.setup_get_selected_item(Some(&item));
        target.pointer_exited(&e);

        menu.verify_set_selected_item(None);
        assert!(!e.handled());
    }

    #[test]
    fn pointer_exited_doesnt_deselect_item_when_menu_open() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = new_mock_menu();
        let item =
            create_mock_menu_item(ItemSetup { is_top_level: true, parent: Some(menu.element()), ..Default::default() });
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_exited_item_event(), &item.control);

        menu.state().is_open.set(true);
        menu.setup_get_selected_item(Some(&item));
        target.pointer_exited(&e);

        menu.verify_set_selected_item_never(None);
        assert!(!e.handled());
    }

    #[test]
    fn doesnt_throw_on_menu_keypress() {
        // Issue #3459
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let _item =
            create_mock_menu_item(ItemSetup { is_top_level: true, parent: Some(menu.element()), ..Default::default() });
        let e = key_event(Key::Tab, &menu.control);

        target.key_down(&e);
    }

    struct MenuItemVM {
        is_checked: Cell<bool>,
        is_sub_menu_open: Cell<bool>,
        property_changed: Event<str>,
    }

    impl MenuItemVM {
        fn new() -> Rc<Self> {
            Model::new_model(Self {
                is_checked: Cell::new(false),
                is_sub_menu_open: Cell::new(false),
                property_changed: Event::new(),
            })
        }

        fn is_checked(&self) -> bool {
            self.is_checked.get()
        }

        fn set_is_checked(&self, value: bool) {
            self.is_checked.set(value);
            self.property_changed.raise("IsChecked");
        }

        fn is_sub_menu_open(&self) -> bool {
            self.is_sub_menu_open.get()
        }

        fn set_is_sub_menu_open(&self, value: bool) {
            self.is_sub_menu_open.set(value);
            self.property_changed.raise("IsSubMenuOpen");
        }
    }

    impl INotifyPropertyChanged for MenuItemVM {
        fn property_changed(&self) -> &Event<str> {
            &self.property_changed
        }
    }

    ferro_model!(MenuItemVM, |b| b
        .notify_property_changed()
        .property::<Value<bool>>("IsChecked", |vm| vm.is_checked(), |vm, v| vm.set_is_checked(v))
        .property::<Value<bool>>("IsSubMenuOpen", |vm| vm.is_sub_menu_open(), |vm, v| vm.set_is_sub_menu_open(v)));

    fn style_two_way_binding(path: &str) -> Rc<ReflectionBinding> {
        let binding = ReflectionBinding::new(path).with_mode(BindingMode::TwoWay);
        binding.set_priority(BindingPriority::Style);
        binding
    }

    #[test]
    fn doesnt_replace_is_checked_binding() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = Menu::new();
        let vm = MenuItemVM::new();

        let item = MenuItem::new();
        let data_context: BoxedValue = vm.clone();
        item.set_data_context(Some(data_context));
        item.bind_binding(MenuItem::is_checked_property().as_property(), &*style_two_way_binding("IsChecked"));
        item.set_toggle_type(MenuItemToggleType::CheckBox);
        menu.items().add(Some(Control::boxed(item.clone())));

        let menu_control: Ref<Control> = menu.clone().upcast();
        target.key_down_item(Some(item.to_menu_item()), &key_event(Key::Enter, &menu_control));

        assert!(item.is_checked());

        vm.set_is_checked(false);

        assert!(!item.is_checked());
    }

    #[test]
    fn doesnt_replace_is_sub_menu_open_binding() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = Menu::new();
        let vm = MenuItemVM::new();

        let item = MenuItem::new();
        let data_context: BoxedValue = vm.clone();
        item.set_data_context(Some(data_context));
        item.bind_binding(MenuItem::is_sub_menu_open_property().as_property(), &*style_two_way_binding("IsSubMenuOpen"));
        item.items().add(Some(Control::boxed(MenuItem::new())));

        let menu_control: Ref<Control> = menu.clone().upcast();
        target.key_down_item(Some(item.to_menu_item()), &key_event(Key::Enter, &menu_control));

        assert!(item.is_sub_menu_open());

        vm.set_is_sub_menu_open(false);

        assert!(!item.is_sub_menu_open());
    }
}

// ---------------------------------------------------------------------------
// NonTopLevel
// ---------------------------------------------------------------------------

mod non_top_level {
    use super::*;

    fn top_level_parent() -> Mock {
        create_mock_menu_item(ItemSetup { is_top_level: true, has_sub_menu: true, ..Default::default() })
    }

    fn top_level_parent_in(menu: &Mock) -> Mock {
        create_mock_menu_item(ItemSetup {
            is_top_level: true,
            has_sub_menu: true,
            parent: Some(menu.element()),
            ..Default::default()
        })
    }

    fn child_of(parent: &Mock) -> Mock {
        create_mock_menu_item(ItemSetup { parent: Some(parent.element()), ..Default::default() })
    }

    fn child_with_sub_menu_of(parent: &Mock) -> Mock {
        create_mock_menu_item(ItemSetup { has_sub_menu: true, parent: Some(parent.element()), ..Default::default() })
    }

    #[test]
    fn up_selects_previous_menu_item() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let parent_item = top_level_parent();
        let item = child_of(&parent_item);
        let e = key_event(Key::Up, &item.control);

        target.key_down_item(Some(item.item()), &e);

        parent_item.verify(Call::MoveSelection(NavigationDirection::Up, true));
        assert!(e.handled());
    }

    #[test]
    fn down_selects_next_menu_item() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let parent_item = top_level_parent();
        let item = child_of(&parent_item);
        let e = key_event(Key::Down, &item.control);

        target.key_down_item(Some(item.item()), &e);

        parent_item.verify(Call::MoveSelection(NavigationDirection::Down, true));
        assert!(e.handled());
    }

    #[test]
    fn left_closes_parent_sub_menu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let parent_item =
            create_mock_menu_item(ItemSetup { has_sub_menu: true, is_sub_menu_open: true, ..Default::default() });
        let item = child_of(&parent_item);
        let e = key_event(Key::Left, &item.control);

        target.key_down_item(Some(item.item()), &e);

        parent_item.verify(Call::Close);
        parent_item.verify(Call::Focus);
        assert!(e.handled());
    }

    #[test]
    fn right_with_sub_menu_items_opens_sub_menu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let parent_item = top_level_parent();
        let item = child_with_sub_menu_of(&parent_item);
        let e = key_event(Key::Right, &item.control);

        target.key_down_item(Some(item.item()), &e);

        item.verify(Call::Open);
        item.verify(Call::MoveSelection(NavigationDirection::First, true));
        assert!(e.handled());
    }

    #[test]
    fn right_on_top_level_child_navigates_top_level_selection() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = new_mock_menu();
        let parent_item = create_mock_menu_item(ItemSetup {
            is_top_level: true,
            has_sub_menu: true,
            is_sub_menu_open: true,
            parent: Some(menu.element()),
        });
        let next_item = top_level_parent_in(&menu);
        let item = child_of(&parent_item);
        let e = key_event(Key::Right, &item.control);

        {
            let menu_state = menu.handle.state.clone();
            let next = next_item.item();
            menu.setup_move_selection_with_callback(NavigationDirection::Right, true, true, move || {
                *menu_state.selected_item_getter.borrow_mut() = Some(Some(next.clone()));
            });
        }

        target.key_down_item(Some(item.item()), &e);

        menu.verify(Call::MoveSelection(NavigationDirection::Right, true));
        parent_item.verify(Call::Close);
        next_item.verify(Call::Open);
        next_item.verify(Call::MoveSelection(NavigationDirection::First, true));
        assert!(e.handled());

        // The callback keeps the menu state and the menu state the callback.
        menu.state().move_selection.borrow_mut().clear();
    }

    #[test]
    fn enter_on_item_with_no_sub_menu_causes_click() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let parent_item = top_level_parent_in(&menu);
        let item = child_of(&parent_item);
        let e = key_event(Key::Enter, &item.control);

        target.key_down_item(Some(item.item()), &e);

        item.verify(Call::RaiseClick);
        menu.verify(Call::Close);
        assert!(e.handled());
    }

    #[test]
    fn enter_on_item_with_sub_menu_opens_sub_menu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let parent_item = top_level_parent();
        let item = child_with_sub_menu_of(&parent_item);
        let e = key_event(Key::Enter, &item.control);

        target.key_down_item(Some(item.item()), &e);

        item.verify(Call::Open);
        item.verify(Call::MoveSelection(NavigationDirection::First, true));
        assert!(e.handled());
    }

    #[test]
    fn escape_closes_parent_menu_item() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let parent_item = top_level_parent();
        let item = child_of(&parent_item);
        let e = key_event(Key::Escape, &item.control);

        target.key_down_item(Some(item.item()), &e);

        parent_item.verify(Call::Close);
        parent_item.verify(Call::Focus);
        assert!(e.handled());
    }

    #[test]
    fn pointer_entered_selects_item() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let parent_item = top_level_parent_in(&menu);
        let item = child_of(&parent_item);
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_entered_item_event(), &item.control);

        target.pointer_entered(&e);

        parent_item.verify_set_selected_item(Some(&item));
        assert!(!e.handled());
    }

    #[test]
    fn pointer_entered_opens_submenu_after_delay() {
        let _scope = test_scope();
        let timer = TestTimer::new();
        let target = DefaultMenuInteractionHandler::with(false, None, timer.run_once());
        let menu = mock_of_menu();
        let parent_item = top_level_parent_in(&menu);
        let item = child_with_sub_menu_of(&parent_item);
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_entered_item_event(), &item.control);

        target.pointer_entered(&e);
        item.verify_never(Call::Open);

        timer.pulse();
        item.verify(Call::Open);

        assert!(!e.handled());
    }

    #[test]
    fn pointer_entered_closes_sibling_submenu_after_delay() {
        let _scope = test_scope();
        let timer = TestTimer::new();
        let target = DefaultMenuInteractionHandler::with(false, None, timer.run_once());
        let menu = mock_of_menu();
        let parent_item = top_level_parent_in(&menu);
        let item = child_of(&parent_item);
        let sibling = create_mock_menu_item(ItemSetup {
            has_sub_menu: true,
            is_sub_menu_open: true,
            parent: Some(parent_item.element()),
            ..Default::default()
        });
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_entered_item_event(), &item.control);

        *parent_item.state().sub_items.borrow_mut() = vec![item.item(), sibling.item()];

        target.pointer_entered(&e);
        sibling.verify_never(Call::Close);

        timer.pulse();
        sibling.verify(Call::Close);

        assert!(!e.handled());

        // The items keep their parent and the parent kept its items.
        parent_item.state().sub_items.borrow_mut().clear();
        *parent_item.state().selected_item.borrow_mut() = None;
    }

    #[test]
    fn pointer_exited_deselects_item() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let parent_item = top_level_parent_in(&menu);
        let item = child_of(&parent_item);
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_exited_item_event(), &item.control);

        parent_item.setup_get_selected_item(Some(&item));
        target.pointer_exited(&e);

        parent_item.verify_set_selected_item(None);
        assert!(!e.handled());

        *parent_item.state().selected_item_getter.borrow_mut() = None;
    }

    #[test]
    fn pointer_exited_doesnt_deselect_sibling() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let parent_item = top_level_parent_in(&menu);
        let item = child_of(&parent_item);
        let sibling = child_of(&parent_item);
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_exited_item_event(), &item.control);

        parent_item.setup_get_selected_item(Some(&sibling));
        target.pointer_exited(&e);

        parent_item.verify_set_selected_item_never(None);
        assert!(!e.handled());

        *parent_item.state().selected_item_getter.borrow_mut() = None;
    }

    #[test]
    fn pointer_exited_doesnt_deselect_item_if_pointer_over_submenu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let parent_item = top_level_parent_in(&menu);
        let item = child_with_sub_menu_of(&parent_item);
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_exited_item_event(), &item.control);

        item.state().is_pointer_over_sub_menu.set(true);
        target.pointer_exited(&e);

        parent_item.verify_set_selected_item_never(None);
        assert!(!e.handled());
    }

    #[test]
    fn pointer_released_on_item_with_no_sub_menu_causes_click() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let parent_item = top_level_parent_in(&menu);
        let item = child_of(&parent_item);
        let e = create_released(&item.control);

        target.pointer_released(&e);

        item.verify(Call::RaiseClick);
        menu.verify(Call::Close);
        assert!(e.handled());
    }

    #[test]
    fn selection_is_correct_when_pointer_temporarily_exits_item_to_select_sub_item() {
        let _scope = test_scope();
        let timer = TestTimer::new();
        let target = DefaultMenuInteractionHandler::with(false, None, timer.run_once());
        let menu = mock_of_menu();
        let parent_item = top_level_parent_in(&menu);
        let item = child_with_sub_menu_of(&parent_item);
        let child_item = child_of(&item);
        let enter = RoutedEventArgs::with_event_and_source(MenuItem::pointer_entered_item_event(), &item.control);
        let leave = RoutedEventArgs::with_event_and_source(MenuItem::pointer_exited_item_event(), &item.control);

        // Pointer enters item; item is selected.
        target.pointer_entered(&enter);
        assert!(timer.action_is_queued());
        parent_item.verify_set_selected_item(Some(&item));
        parent_item.clear_invocations();

        // SubMenu shown after a delay.
        timer.pulse();
        item.verify(Call::Open);
        item.state().is_sub_menu_open.set(true);
        item.clear_invocations();

        // Pointer briefly exits item, but submenu remains open.
        target.pointer_exited(&leave);
        item.verify_never(Call::Close);
        item.clear_invocations();

        // Pointer enters child item; is selected.
        enter.set_source(&child_item.control);
        target.pointer_entered(&enter);
        item.verify_set_selected_item(Some(&child_item));
        parent_item.verify_set_selected_item(Some(&item));

        // The items keep their parents and the parents kept their selection;
        // the queued action keeps the item.
        *item.state().selected_item.borrow_mut() = None;
        *parent_item.state().selected_item.borrow_mut() = None;
        *menu.state().selected_item.borrow_mut() = None;
        *timer.action.borrow_mut() = None;
    }

    #[test]
    fn pointer_pressed_on_item_with_sub_menu_causes_opens_submenu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let parent_item = top_level_parent_in(&menu);
        let item = child_with_sub_menu_of(&parent_item);
        let e = create_pressed(&item.control);

        target.pointer_pressed(Some(&item.visual()), &e);

        item.verify(Call::Open);
        item.verify_never(Call::MoveSelection(NavigationDirection::First, true));
        assert!(e.handled());
    }

    #[test]
    fn pointer_pressed_on_disabled_item_doesnt_close_sub_menu() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(false);
        let menu = mock_of_menu();
        let parent_item = create_mock_menu_item(ItemSetup {
            is_top_level: true,
            has_sub_menu: true,
            is_sub_menu_open: true,
            parent: Some(menu.element()),
        });
        let popup = Popup::new();
        let popup_control: Ref<Control> = popup.clone().upcast();
        let e = create_pressed(&popup_control);

        popup.set_parent(&parent_item.control);
        target.pointer_pressed(Some(&parent_item.visual()), &e);

        parent_item.verify_never(Call::Close);
        assert!(e.handled());
    }
}

// ---------------------------------------------------------------------------
// ContextMenu
// ---------------------------------------------------------------------------

mod context_menu {
    use super::*;

    #[test]
    fn down_selects_selects_first_menu_item_when_no_selection() {
        let _scope = test_scope();
        let target = DefaultMenuInteractionHandler::new(true);
        let context_menu = mock_of_menu();
        context_menu.setup_move_selection(NavigationDirection::Down, true, true);
        let e = key_event(Key::Down, &context_menu.control);

        target.attach_core(context_menu.menu());
        target.key_down(&e);

        context_menu.verify(Call::MoveSelection(NavigationDirection::Down, true));
        assert!(e.handled());
    }
}

// ---------------------------------------------------------------------------
// Not a port: the overridable members of the handler
// ---------------------------------------------------------------------------

mod overrides {
    use super::*;

    /// Counts the pointer-entered notifications and forwards them to the
    /// default behaviour only when asked to.
    struct CountingOverrides {
        pointer_entered: Cell<i32>,
        call_base: Cell<bool>,
    }

    impl DefaultMenuInteractionHandlerOverrides for CountingOverrides {
        fn pointer_entered(&self, handler: &DefaultMenuInteractionHandler, e: &RoutedEventArgs) {
            self.pointer_entered.set(self.pointer_entered.get() + 1);

            if self.call_base.get() {
                handler.base_pointer_entered(e);
            }
        }
    }

    #[test]
    fn override_replaces_pointer_entered_and_can_call_the_base_behaviour() {
        let _scope = test_scope();
        let timer = TestTimer::new();
        let overrides = Rc::new(CountingOverrides { pointer_entered: Cell::new(0), call_base: Cell::new(false) });
        let target = DefaultMenuInteractionHandler::with_overrides(false, None, timer.run_once(), overrides.clone());
        let menu = mock_of_menu();
        let item =
            create_mock_menu_item(ItemSetup { is_top_level: true, parent: Some(menu.element()), ..Default::default() });

        target.attach_core(menu.menu());

        // The event reaches the handler through the menu it is attached to;
        // the override runs instead of the default behaviour.
        item.control.set_parent(&menu.control);
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_entered_item_event(), &item.control);
        menu.control.raise_event(&e);

        assert_eq!(1, overrides.pointer_entered.get());
        menu.verify_set_selected_item_never(Some(&item));

        // Members that are not overridden keep the default behaviour.
        let key = key_event(Key::Escape, &item.control);
        target.key_down(&key);
        menu.verify(Call::Close);
        assert!(key.handled());

        // The override can run the default behaviour.
        overrides.call_base.set(true);
        let e = RoutedEventArgs::with_event_and_source(MenuItem::pointer_entered_item_event(), &item.control);
        target.pointer_entered(&e);

        assert_eq!(2, overrides.pointer_entered.get());
        menu.verify_set_selected_item(Some(&item));

        target.detach_core(&*menu.menu());
        *menu.state().selected_item.borrow_mut() = None;
    }
}
