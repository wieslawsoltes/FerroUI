//! The tests of the button family of the reference that need the text
//! controls: the letter spacing tests of the button, check box and radio
//! button tests, and the access key test of the button tests.
//!
//! The reference tests that take the template of the button from the theme
//! set a template with a content presenter here, and a `TestRoot` stands in
//! for the window (the access key test hosts the button in a top-level, as
//! the reference does).

use crate::presenters::ContentPresenter;
use crate::primitives::AccessText;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_command::TestCommand;
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::platform::ITopLevelImpl;
use crate::primitives::TemplatedControlImpl;
use crate::testing::{MockImplKind, MockWindowImpl, TestServices, UnitTestApplication};
use crate::{
    Button, CheckBox, ContentControl, ContentControlImpl, Control, ControlImpl, RadioButton, TextBlock, TopLevel,
    TopLevelImpl,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroLocator, FerroObjectImpl, LocatorExtensions, StyledElementImpl,
    VisualImpl,
};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{
    AccessKeyHandler, IAccessKeyHandler, IKeyboardDevice, InputElement, InputElementImpl, Key, KeyEventArgs,
    KeyModifiers, KeyboardDevice, NavigationMethod,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{AnyValue, BoxedValue, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;

/// The part of the button template of the theme these tests rely on.
fn create_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::for_type::<Button>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let content = ContentControl::content_property().as_property();
        presenter.bind_binding(content, &TemplateBinding::new(content));
        let content_template = ContentControl::content_template_property().as_property();
        presenter.bind_binding(content_template, &TemplateBinding::new(content_template));
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

fn templated_button(content: &str, letter_spacing: f64) -> Ref<Button> {
    let button = Button::new();
    button.set_template(Some(create_template()));
    button.set_content(boxed_str(content));
    button.set_letter_spacing(letter_spacing);
    button
}

// --- button -----------------------------------------------------------------

#[test]
fn button_letter_spacing_default_value_is_zero() {
    let button = Button::new();
    assert_eq!(0.0, button.letter_spacing());
}

#[test]
fn button_letter_spacing_can_be_set_and_retrieved() {
    let button = Button::new();
    button.set_letter_spacing(2.5);
    assert_eq!(2.5, button.letter_spacing());
}

#[test]
fn button_letter_spacing_can_be_set_to_negative_value() {
    let button = Button::new();
    button.set_letter_spacing(-1.5);
    assert_eq!(-1.5, button.letter_spacing());
}

#[test]
fn button_letter_spacing_can_be_set_to_zero() {
    let button = Button::new();
    button.set_letter_spacing(5.0);
    button.set_letter_spacing(0.0);
    assert_eq!(0.0, button.letter_spacing());
}

#[test]
fn button_letter_spacing_propagates_to_content_presenter() {
    let _scope = test_scope();
    let button = templated_button("Test", 3.0);
    let _root = TestRoot::with_child(&button);

    button.apply_template();

    let presenter = button.presenter().unwrap();
    assert_eq!(3.0, presenter.letter_spacing());
}

#[test]
fn button_letter_spacing_updates_content_presenter_when_changed() {
    let _scope = test_scope();
    let button = templated_button("Test", 1.0);
    let _root = TestRoot::with_child(&button);

    button.apply_template();
    let presenter = button.presenter().unwrap();

    button.set_letter_spacing(5.0);

    assert_eq!(5.0, presenter.letter_spacing());
}

#[test]
fn button_letter_spacing_works_with_large_values() {
    let button = Button::new();
    button.set_letter_spacing(100.0);
    assert_eq!(100.0, button.letter_spacing());
}

#[test]
fn button_letter_spacing_property_inherits_through_visual_tree() {
    let _scope = test_scope();
    let button = templated_button("Test", 2.0);
    let _root = TestRoot::with_child(&button);

    button.apply_template();
    if let Some(presenter) = button.presenter() {
        presenter.update_child();
    }

    // Verify the property value is accessible on the presenter.
    let presenter = button.presenter().unwrap();
    assert_eq!(2.0, presenter.letter_spacing());
}

#[test]
fn button_letter_spacing_affects_text_block_child_in_content_presenter() {
    let _scope = test_scope();
    let button = templated_button("Test Text", 3.5);
    let _root = TestRoot::with_child(&button);

    button.apply_template();
    if let Some(presenter) = button.presenter() {
        presenter.update_child();
    }

    // Find the text block that was created by the content presenter.
    let presenter = button.presenter().unwrap();
    let text_block = presenter.child().and_then(|child| child.cast::<TextBlock>()).unwrap();

    // Verify the letter spacing inherited to the text block.
    assert_eq!(3.5, text_block.letter_spacing());

    // Force a measure to create the text layout.
    text_block.measure(Size::new(f64::INFINITY, f64::INFINITY));

    // Verify the text layout actually has the letter spacing value.
    let text_layout = text_block.text_layout();
    assert_eq!(3.5, text_layout.letter_spacing());
}

/// The top-level of the access key test.
#[repr(C)]
struct TestTopLevel {
    base: TopLevel,
}

ferro_class!(TestTopLevel: TopLevel);
ferro_impl_classes!(
    TestTopLevel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl
);

impl TestTopLevel {
    fn new(platform_impl: Rc<dyn ITopLevelImpl>) -> Ref<Self> {
        instantiate(Self { base: TopLevel::construct(platform_impl) })
    }
}

fn key_event(down: bool, key: Key, key_symbol: Option<&str>, modifiers: KeyModifiers) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(if down { InputElement::key_down_event() } else { InputElement::key_up_event() }));
    e.key = key;
    e.key_symbol = key_symbol.map(str::to_string);
    e.key_modifiers = modifiers;
    e
}

fn raise_access_key(target: &InputElement, access_key: Key, key_symbol: &str) {
    target.raise_event(&key_event(true, Key::LeftAlt, None, KeyModifiers::NONE));
    target.raise_event(&key_event(true, access_key, Some(key_symbol), KeyModifiers::ALT));
    target.raise_event(&key_event(false, access_key, Some(key_symbol), KeyModifiers::ALT));
    target.raise_event(&key_event(false, Key::LeftAlt, None, KeyModifiers::NONE));
}

#[test]
fn raises_click_when_access_key_raised() {
    let raised = Rc::new(Cell::new(0));
    let kd = KeyboardDevice::new();
    // The fonts and the render interface the access text is laid out with.
    let _text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let _app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_render_interface(render_interface)
            .with_access_key_handler(|| Some(AccessKeyHandler::new() as Rc<dyn IAccessKeyHandler>))
            .with_keyboard_device({
                let kd = kd.clone();
                move || Some(kd.clone() as Rc<dyn IKeyboardDevice>)
            }),
    );

    let platform_impl = MockWindowImpl::bare(MockImplKind::TopLevel);

    let counter = raised.clone();
    let command = TestCommand::with_can_execute_and_execute(
        |parameter| {
            parameter.is_some_and(|parameter| {
                let value: &dyn AnyValue = &**parameter;
                value.downcast_ref::<bool>().copied().unwrap_or(false)
            })
        },
        move |_| counter.set(counter.get() + 1),
    );

    let root = TestTopLevel::new(platform_impl);
    root.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let content = ContentControl::content_property().as_property();
        presenter.bind_binding(content, &TemplateBinding::new(content));
        let content_template = ContentControl::content_template_property().as_property();
        presenter.bind_binding(content_template, &TemplateBinding::new(content_template));
        presenter.register_in_name_scope(&**scope).upcast()
    })));
    assert!(root.access_key_handler().is_some());

    let target = Button::new();
    target.set_content(boxed_str("_A"));
    target.set_command(command.as_command());
    target.set_template(Some(FuncControlTemplate::for_type::<Button>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let content = ContentControl::content_property().as_property();
        presenter.bind_binding(content, &TemplateBinding::new(content));
        // As in the reference test, the content template is bound to the
        // content of the button.
        presenter.bind_binding(ContentControl::content_template_property().as_property(), &TemplateBinding::new(content));
        presenter.set_recognizes_access_key(true);
        presenter.register_in_name_scope(&**scope).upcast()
    })));
    root.set_content(Some(Control::boxed(&target)));

    root.apply_template();
    root.presenter().unwrap().update_child();
    target.apply_template();
    target.presenter().unwrap().update_child();
    assert!(target.presenter().unwrap().child().is_some_and(|child| child.is::<AccessText>()));
    kd.set_focused_element(Some(&target.clone().upcast()), NavigationMethod::Unspecified, KeyModifiers::NONE);

    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));

    const ACCESS_KEY: Key = Key::A;
    const ACCESS_KEY_SYMBOL: &str = "a";
    let parameter: BoxedValue = Rc::new(true);
    target.set_command_parameter(Some(parameter));

    raise_access_key(&root, ACCESS_KEY, ACCESS_KEY_SYMBOL);

    assert_eq!(1, raised.get());

    let parameter: BoxedValue = Rc::new(false);
    target.set_command_parameter(Some(parameter));

    raise_access_key(&root, ACCESS_KEY, ACCESS_KEY_SYMBOL);

    assert_eq!(1, raised.get());
}

// --- check box --------------------------------------------------------------

#[test]
fn check_box_letter_spacing_default_value_is_zero() {
    let check_box = CheckBox::new();
    assert_eq!(0.0, check_box.letter_spacing());
}

#[test]
fn check_box_letter_spacing_can_be_set_and_retrieved() {
    let check_box = CheckBox::new();
    check_box.set_letter_spacing(2.5);
    assert_eq!(2.5, check_box.letter_spacing());
}

#[test]
fn check_box_letter_spacing_inherits_from_templated_control() {
    let check_box = CheckBox::new();
    check_box.set_letter_spacing(3.0);
    // The letter spacing is inherited from the templated control.
    assert_eq!(3.0, check_box.letter_spacing());
}

#[test]
fn check_box_letter_spacing_can_be_negative() {
    let check_box = CheckBox::new();
    check_box.set_letter_spacing(-1.5);
    assert_eq!(-1.5, check_box.letter_spacing());
}

// --- radio button -----------------------------------------------------------

#[test]
fn radio_button_letter_spacing_default_value_is_zero() {
    let radio_button = RadioButton::new();
    assert_eq!(0.0, radio_button.letter_spacing());
}

#[test]
fn radio_button_letter_spacing_can_be_set_and_retrieved() {
    let radio_button = RadioButton::new();
    radio_button.set_letter_spacing(2.5);
    assert_eq!(2.5, radio_button.letter_spacing());
}

#[test]
fn radio_button_letter_spacing_inherits_from_templated_control() {
    let radio_button = RadioButton::new();
    radio_button.set_letter_spacing(3.0);
    // The letter spacing is inherited from the templated control.
    assert_eq!(3.0, radio_button.letter_spacing());
}
