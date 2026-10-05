//! The theories of the reference run for buttons and for menu items: the
//! button cases come first, the menu item cases (`*_menu_item`) after them.
//!
//! The last part of the file holds the tests of hot keys on a control that
//! is a command source without being a button.

use crate::i_clickable_control::as_clickable_control;
use crate::i_command_source::register_command_source;
use crate::test_command::TestCommand;
use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Button, ContentControl, Control, ControlImpl, HotKeyManager, Menu, MenuItem, Window};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::raw::{RawKeyEventArgs, RawKeyEventType};
use ferroui_base::input::{
    ICommand, ICommandSource, IInputDevice, IKeyboardDevice, InputElementImpl, Key, KeyDeviceType, KeyGesture, KeyModifiers, KeyboardDevice,
    PhysicalKey, RawInputModifiers,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    Ref, StyledElementImpl, StyledElementImplExt, StyledProperty, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

/// The services of the reference tests: only the windowing platform.
fn windowing_platform() -> UnitTestApplicationScope {
    UnitTestApplication::start(TestServices::mock_windowing_platform())
}

fn create_window_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

fn apply_templates(root: &Window) {
    root.set_template(Some(create_window_template()));
    root.apply_template();
    root.presenter().expect("the template has a presenter").apply_template();
}

fn press(target: &Rc<KeyboardDevice>, root: &Window, key: Key, physical_key: PhysicalKey, symbol: &str) {
    target.process_raw_event(&RawKeyEventArgs::new(
        target.clone(),
        0,
        root.input_root(),
        RawKeyEventType::KeyDown,
        key,
        RawInputModifiers::CONTROL,
        physical_key,
        Some(symbol.to_string()),
        KeyDeviceType::Keyboard,
    ));
}

fn press_ctrl_a(target: &Rc<KeyboardDevice>, root: &Window) {
    press(target, root, Key::A, PhysicalKey::A, "a");
}

fn command(execute: impl Fn(Option<&BoxedValue>) + 'static) -> Option<Rc<dyn ICommand>> {
    TestCommand::with_can_execute_and_execute(|_| true, execute).as_command()
}

fn make_button(expected_parameter: i32, action: impl Fn(Option<&BoxedValue>) + 'static, root: &Window) -> Ref<Button> {
    let button = Button::new();
    button.set_command(command(action));
    let parameter: BoxedValue = Rc::new(expected_parameter);
    button.set_command_parameter(Some(parameter));

    root.set_content(Some(Control::boxed(&button)));
    button
}

fn make_button_without_command(root: &Window) -> Ref<Button> {
    let button = Button::new();

    root.set_content(Some(Control::boxed(&button)));
    button
}

fn first_gesture(root: &Window) -> Option<KeyGesture> {
    root.key_bindings().try_get(0).and_then(|binding| binding.gesture())
}

#[test]
fn hot_key_manager_should_register_and_unregister_key_binding() {
    let _app = windowing_platform();
    let gesture1 = KeyGesture::new(Key::A, KeyModifiers::CONTROL);
    let gesture2 = KeyGesture::new(Key::B, KeyModifiers::CONTROL);

    let tl = Window::new();
    let button = Button::new();
    tl.set_content(Some(Control::boxed(&button)));
    apply_templates(&tl);

    HotKeyManager::set_hot_key(&button, Some(gesture1));

    assert_eq!(first_gesture(&tl), Some(gesture1));

    HotKeyManager::set_hot_key(&button, Some(gesture2));
    assert_eq!(first_gesture(&tl), Some(gesture2));

    tl.set_content(None);
    tl.presenter().unwrap().apply_template();

    assert!(tl.key_bindings().is_empty());

    tl.set_content(Some(Control::boxed(&button)));
    tl.presenter().unwrap().apply_template();

    assert_eq!(first_gesture(&tl), Some(gesture2));

    HotKeyManager::set_hot_key(&button, None);
    assert!(tl.key_bindings().is_empty());
}

#[test]
fn hot_key_manager_should_use_command_parameter() {
    let _app = windowing_platform();
    let target = KeyboardDevice::new();
    let command_result = Rc::new(Cell::new(0));
    let expected_parameter = 1;

    let gesture = KeyGesture::new(Key::A, KeyModifiers::CONTROL);

    let result = command_result.clone();
    let root = Window::new();
    let element = make_button(
        expected_parameter,
        move |parameter| {
            if let Some(value) = parameter.and_then(|parameter| parameter.downcast_ref::<i32>()) {
                result.set(*value);
            }
        },
        &root,
    );

    apply_templates(&root);

    HotKeyManager::set_hot_key(&element, Some(gesture));

    press_ctrl_a(&target, &root);

    assert_eq!(expected_parameter, command_result.get(), "Button HotKey did not carry the CommandParameter.");
}

#[test]
fn hot_key_manager_should_do_not_executed_when_is_enabled_false() {
    let _app = windowing_platform();
    let target = KeyboardDevice::new();
    let is_executed = Rc::new(Cell::new(false));

    let gesture = KeyGesture::new(Key::A, KeyModifiers::CONTROL);

    let executed = is_executed.clone();
    let root = Window::new();
    let element = make_button(0, move |_| executed.set(true), &root);

    element.set_is_enabled(false);

    apply_templates(&root);

    HotKeyManager::set_hot_key(&element, Some(gesture));

    press_ctrl_a(&target, &root);

    assert!(!is_executed.get(), "Button Execution raised when IsEnabled is false.");
}

#[test]
fn hot_key_manager_should_invoke_event_click_when_command_is_null() {
    let _app = windowing_platform();
    let target = KeyboardDevice::new();
    let click_executed_count = Rc::new(Cell::new(0));

    let gesture = KeyGesture::new(Key::A, KeyModifiers::CONTROL);

    let root = Window::new();
    let element = make_button_without_command(&root);
    let clickable = as_clickable_control(&element).expect("a button is clickable");
    let count = click_executed_count.clone();
    clickable.click(Rc::new(move |_| count.set(count.get() + 1)));

    apply_templates(&root);

    HotKeyManager::set_hot_key(&element, Some(gesture));

    press_ctrl_a(&target, &root);

    element.set_is_enabled(false);

    press_ctrl_a(&target, &root);

    assert_eq!(click_executed_count.get(), 1, "Button Execution raised when IsEnabled is false.");
}

#[test]
fn hot_key_manager_should_not_invoke_event_click_when_command_is_not_null() {
    let _app = windowing_platform();
    let target = KeyboardDevice::new();
    let click_executed_count = Rc::new(Cell::new(0));
    let command_executed_count = Rc::new(Cell::new(0));

    let gesture = KeyGesture::new(Key::A, KeyModifiers::CONTROL);

    let root = Window::new();
    let executed = command_executed_count.clone();
    let element = make_button(0, move |_| executed.set(executed.get() + 1), &root);
    let clickable = as_clickable_control(&element).expect("a button is clickable");
    let count = click_executed_count.clone();
    clickable.click(Rc::new(move |_| count.set(count.get() + 1)));

    apply_templates(&root);

    HotKeyManager::set_hot_key(&element, Some(gesture));

    press_ctrl_a(&target, &root);

    element.set_is_enabled(false);

    press_ctrl_a(&target, &root);

    assert_eq!(command_executed_count.get(), 1, "Button Execution raised when IsEnabled is false.");
    assert_eq!(click_executed_count.get(), 0, "Button Execution raised event Click.");
}

// --- the theories for menu items ----------------------------------------------

fn make_menu(expected_parameter: i32, action: impl Fn(Option<&BoxedValue>) + 'static, root: &Window) -> Ref<MenuItem> {
    let menuitem = MenuItem::new();
    menuitem.set_command(command(action));
    let parameter: BoxedValue = Rc::new(expected_parameter);
    menuitem.set_command_parameter(Some(parameter));
    let root_menu = Menu::new();

    root_menu.items().add(Some(Control::boxed(&menuitem)));

    root.set_content(Some(Control::boxed(&root_menu)));
    menuitem
}

fn make_menu_without_command(root: &Window) -> Ref<MenuItem> {
    let menuitem = MenuItem::new();
    let root_menu = Menu::new();

    root_menu.items().add(Some(Control::boxed(&menuitem)));

    root.set_content(Some(Control::boxed(&root_menu)));
    menuitem
}

#[test]
fn hot_key_manager_should_use_command_parameter_menu_item() {
    let _app = windowing_platform();
    let target = KeyboardDevice::new();
    let command_result = Rc::new(Cell::new(0));
    let expected_parameter = 1;

    let gesture = KeyGesture::new(Key::A, KeyModifiers::CONTROL);

    let result = command_result.clone();
    let root = Window::new();
    let element = make_menu(
        expected_parameter,
        move |parameter| {
            if let Some(value) = parameter.and_then(|parameter| parameter.downcast_ref::<i32>()) {
                result.set(*value);
            }
        },
        &root,
    );

    apply_templates(&root);

    HotKeyManager::set_hot_key(&element, Some(gesture));

    press_ctrl_a(&target, &root);

    assert_eq!(expected_parameter, command_result.get(), "MenuItem HotKey did not carry the CommandParameter.");
}

#[test]
fn hot_key_manager_should_do_not_executed_when_is_enabled_false_menu_item() {
    let _app = windowing_platform();
    let target = KeyboardDevice::new();
    let is_executed = Rc::new(Cell::new(false));

    let gesture = KeyGesture::new(Key::A, KeyModifiers::CONTROL);

    let executed = is_executed.clone();
    let root = Window::new();
    let element = make_menu(0, move |_| executed.set(true), &root);

    element.set_is_enabled(false);

    apply_templates(&root);

    HotKeyManager::set_hot_key(&element, Some(gesture));

    press_ctrl_a(&target, &root);

    assert!(!is_executed.get(), "MenuItem Execution raised when IsEnabled is false.");
}

#[test]
fn hot_key_manager_should_invoke_event_click_when_command_is_null_menu_item() {
    let _app = windowing_platform();
    let target = KeyboardDevice::new();
    let click_executed_count = Rc::new(Cell::new(0));

    let gesture = KeyGesture::new(Key::A, KeyModifiers::CONTROL);

    let root = Window::new();
    let element = make_menu_without_command(&root);
    let clickable = as_clickable_control(&element).expect("a menu item is clickable");
    let count = click_executed_count.clone();
    clickable.click(Rc::new(move |_| count.set(count.get() + 1)));

    apply_templates(&root);

    HotKeyManager::set_hot_key(&element, Some(gesture));

    press_ctrl_a(&target, &root);

    element.set_is_enabled(false);

    press_ctrl_a(&target, &root);

    assert_eq!(click_executed_count.get(), 1, "MenuItem Execution raised when IsEnabled is false.");
}

#[test]
fn hot_key_manager_should_not_invoke_event_click_when_command_is_not_null_menu_item() {
    let _app = windowing_platform();
    let target = KeyboardDevice::new();
    let click_executed_count = Rc::new(Cell::new(0));
    let command_executed_count = Rc::new(Cell::new(0));

    let gesture = KeyGesture::new(Key::A, KeyModifiers::CONTROL);

    let root = Window::new();
    let executed = command_executed_count.clone();
    let element = make_menu(0, move |_| executed.set(executed.get() + 1), &root);
    let clickable = as_clickable_control(&element).expect("a menu item is clickable");
    let count = click_executed_count.clone();
    clickable.click(Rc::new(move |_| count.set(count.get() + 1)));

    apply_templates(&root);

    HotKeyManager::set_hot_key(&element, Some(gesture));

    press_ctrl_a(&target, &root);

    element.set_is_enabled(false);

    press_ctrl_a(&target, &root);

    assert_eq!(command_executed_count.get(), 1, "MenuItem Execution raised when IsEnabled is false.");
    assert_eq!(click_executed_count.get(), 0, "MenuItem Execution raised event Click.");
}

// --- hot keyed controls -----------------------------------------------------

/// A focusable control that is a command source: its command focuses it.
/// Stands in for the text box subclass of the reference test.
#[repr(C)]
struct HotKeyedControl {
    base: Control,
    hotkey: Cell<Option<KeyGesture>>,
}

ferro_class!(HotKeyedControl: Control);
ferro_impl_classes!(HotKeyedControl: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for HotKeyedControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        register_command_source::<HotKeyedControl>(|control| Rc::new(HotKeyedControlHandle(control)));
    }
}

impl StyledElementImpl for HotKeyedControl {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        if let Some(hotkey) = this.hotkey.get() {
            this.set_value(Self::hot_key_property(), Some(hotkey));
        }

        Self::parent_on_attached_to_logical_tree(this, e);
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        if let Some(hotkey) = this.hot_key() {
            this.hotkey.set(Some(hotkey));
            this.set_value(Self::hot_key_property(), None);
        }

        Self::parent_on_detached_from_logical_tree(this, e);
    }
}

struct HotKeyedControlHandle(Ref<HotKeyedControl>);

impl ICommandSource for HotKeyedControlHandle {
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        let weak = self.0.downgrade();
        command(move |_| {
            if let Some(control) = weak.upgrade() {
                control.focus();
            }
        })
    }

    fn command_parameter(&self) -> Option<BoxedValue> {
        None
    }

    fn can_execute_changed(&self) {}

    fn is_effectively_enabled(&self) -> bool {
        self.0.is_effectively_enabled()
    }
}

impl HotKeyedControl {
    ferro_property!(
        fn hot_key_property() -> StyledProperty<Option<KeyGesture>> {
            HotKeyManager::hot_key_property().add_owner::<HotKeyedControl>()
        }
    );

    fn new() -> Ref<Self> {
        let control = instantiate(Self { base: Control::construct(), hotkey: Cell::new(None) });
        control.set_focusable(true);
        control
    }

    fn hot_key(&self) -> Option<KeyGesture> {
        self.get_value(Self::hot_key_property())
    }

    fn set_hot_key(&self, value: Option<KeyGesture>) {
        self.set_value(Self::hot_key_property(), value)
    }
}

fn prepared_window() -> Ref<Window> {
    let w = Window::new();
    w.apply_template();
    w
}

fn create_services_with_focus() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)),
    )
}

#[test]
fn hot_keyed_text_box_focus_performed_on_hotkey() {
    let _app = create_services_with_focus();

    let keyboard_device = KeyboardDevice::new();
    let hot_keyed_text_box = HotKeyedControl::new();
    hot_keyed_text_box.set_hot_key(Some(KeyGesture::new(Key::F, KeyModifiers::CONTROL)));
    let root = prepared_window();
    root.set_content(Some(Control::boxed(&hot_keyed_text_box)));
    root.show();

    assert!(!hot_keyed_text_box.is_focused());

    press(&keyboard_device, &root, Key::F, PhysicalKey::F, "f");

    assert!(hot_keyed_text_box.is_focused());
}
