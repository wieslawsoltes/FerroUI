//! Port of `Input/InputElement_Focus.cs` (base unit tests). The trees are
//! built from buttons, decorators, panels, stack panels, text blocks, menus
//! and windows, so the tests live with the controls.
//!
//! `TestFocusScope` is a panel that is a focus scope (`IFocusScope`). Upstream's
//! `root.FocusManager` is the focus manager of the root, a `FocusManager`, so
//! `Assert.IsType<FocusManager>` and the casts to it have no counterpart.

use crate::test_support::{boxed_str, TestRoot};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{
    Border, Button, Canvas, Control, ControlImpl, Decorator, Menu, MenuItem, Panel, PanelImpl, StackPanel, TextBlock,
    Window,
};
use ferroui_base::input::navigation::XYFocus;
use ferroui_base::input::{
    AccessKeyEventArgs, FindNextElementOptions, FocusManager, IKeyboardDevice, InputElement, InputElementImpl,
    KeyModifiers, KeyboardDevice, KeyboardNavigation, KeyboardNavigationMode, NavigationDirection, NavigationMethod,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{
    ferro_class, FerroLocator, LocatorExtensions, ferro_impl_classes, instantiate, FerroObjectImpl, IntoRef, Ref, StyledElementImpl, VisualImpl,
};
use std::rc::Rc;

/// C# `private class TestFocusScope : Panel, IFocusScope`.
#[repr(C)]
struct TestFocusScope {
    base: Panel,
}

ferro_class!(TestFocusScope: Panel);
ferro_impl_classes!(
    TestFocusScope: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    PanelImpl
);

impl InputElementImpl for TestFocusScope {
    fn is_focus_scope(_this: &Self) -> bool {
        true
    }
}

impl TestFocusScope {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Panel::construct() })
    }
}

/// The element form of a control, for comparisons with focused elements.
fn el(control: &(impl IntoRef<InputElement> + Clone)) -> Option<Ref<InputElement>> {
    Some(control.clone().into_ref())
}

fn keyboard_focused_element() -> Option<Ref<InputElement>> {
    KeyboardDevice::instance().and_then(|keyboard| keyboard.focused_element())
}

fn button(content: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_focusable(true);
    button.set_content(boxed_str(content));
    button
}

fn named_button(name: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.to_string()));
    button
}

fn stack_panel(children: &[&dyn AsControl]) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    for child in children {
        panel.children().add(child.as_control());
    }
    panel
}

fn panel(children: &[&dyn AsControl]) -> Ref<Panel> {
    let panel = Panel::new();
    for child in children {
        panel.children().add(child.as_control());
    }
    panel
}

/// A control of any class, as a child of a panel.
trait AsControl {
    fn as_control(&self) -> Ref<Control>;
}

impl<T: ferroui_base::ObjectType> AsControl for Ref<T>
where
    Ref<T>: IntoRef<Control>,
{
    fn as_control(&self) -> Ref<Control> {
        self.clone().into_ref()
    }
}

fn focusable_decorator() -> Ref<Decorator> {
    let decorator = Decorator::new();
    decorator.set_focusable(true);
    decorator
}

fn real_focus() -> TestServices {
    TestServices::real_focus()
}

/// `TestServices.RealFocus` with the render interface, fonts and text shaper
/// upstream's set has, which the layout pass lays the text of the buttons out
/// with.
//
// Deviation (DEVIATIONS.md, Tests and test support): the port's `real_focus`
// has no text services, so they come from a text test scope.
fn real_focus_with_text_services() -> (TextTestScope, TestServices) {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    (text, real_focus().with_render_interface(render_interface))
}

fn styled_window_with_keyboard_device() -> TestServices {
    TestServices::styled_window().with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>))
}

#[test]
fn focus_should_set_focus_manager_current() {
    let _app = UnitTestApplication::start(real_focus());
    let target = Button::new();
    let root = TestRoot::with_child(&target);

    target.focus();

    assert_eq!(el(&target), root.focus_manager().get_focused_element());
}

#[test]
fn invisible_controls_should_not_receive_focus() {
    let _app = UnitTestApplication::start(real_focus());
    let target = Button::new();
    target.set_is_visible(false);
    let root = TestRoot::with_child(&target);

    assert!(root.focus_manager().get_focused_element().is_none());

    target.focus();

    assert!(!target.is_focused());
    assert!(!target.is_keyboard_focus_within());

    assert!(root.focus_manager().get_focused_element().is_none());
}

#[test]
fn effectively_invisible_controls_should_not_receive_focus() {
    let target = Button::new();
    let _app = UnitTestApplication::start(real_focus());
    let container = panel(&[&target]);
    container.set_is_visible(false);
    let root = TestRoot::with_child(&container);

    assert!(root.focus_manager().get_focused_element().is_none());

    target.focus();

    assert!(!target.is_focused());
    assert!(!target.is_keyboard_focus_within());

    assert!(root.focus_manager().get_focused_element().is_none());
}

#[test]
fn trying_to_focus_invisible_control_should_not_change_focus() {
    let _app = UnitTestApplication::start(real_focus());
    let first = Button::new();
    let second = Button::new();
    second.set_is_visible(false);
    let root = TestRoot::with_child(&stack_panel(&[&first, &second]));

    first.focus();

    assert_eq!(el(&first), root.focus_manager().get_focused_element());

    second.focus();

    assert_eq!(el(&first), root.focus_manager().get_focused_element());
}

#[test]
fn disabled_controls_should_not_receive_focus() {
    let _app = UnitTestApplication::start(real_focus());
    let target = Button::new();
    target.set_is_enabled(false);
    let root = TestRoot::with_child(&target);

    assert!(root.focus_manager().get_focused_element().is_none());

    target.focus();

    assert!(!target.is_focused());
    assert!(!target.is_keyboard_focus_within());

    assert!(root.focus_manager().get_focused_element().is_none());
}

#[test]
fn effectively_disabled_controls_should_not_receive_focus() {
    let target = Button::new();
    let _app = UnitTestApplication::start(real_focus());
    let container = panel(&[&target]);
    container.set_is_enabled(false);
    let root = TestRoot::with_child(&container);

    assert!(root.focus_manager().get_focused_element().is_none());

    target.focus();

    assert!(!target.is_focused());
    assert!(!target.is_keyboard_focus_within());

    assert!(root.focus_manager().get_focused_element().is_none());
}

#[test]
fn focus_should_not_get_restored_to_enabled_control() {
    let _app = UnitTestApplication::start(real_focus());
    let sp = StackPanel::new();
    let target = Button::new();
    let target1 = Button::new();
    let weak = target.downgrade();
    target.click(move |_, _| {
        if let Some(target) = weak.upgrade() {
            target.set_is_enabled(false);
        }
    });
    let weak = target.downgrade();
    target1.click(move |_, _| {
        if let Some(target) = weak.upgrade() {
            target.set_is_enabled(true);
        }
    });
    sp.children().add(target.clone());
    sp.children().add(target1.clone());
    let _root = TestRoot::with_child(&sp);

    target.focus();
    target.raise_event(&AccessKeyEventArgs::new("b1", false));
    assert!(!target.is_enabled());
    assert!(!target.is_focused());
    target1.raise_event(&AccessKeyEventArgs::new("b2", false));
    assert!(target.is_enabled());
    assert!(!target.is_focused());
}

#[test]
fn focus_should_be_cleared_when_control_is_hidden() {
    let _app = UnitTestApplication::start(real_focus());
    let target = Button::new();
    let root = TestRoot::with_child(&target);

    target.focus();
    target.set_is_visible(false);

    assert!(root.focus_manager().get_focused_element().is_none());
}

#[test]
fn focus_should_be_cleared_when_control_is_effectively_hidden() {
    let _app = UnitTestApplication::start(real_focus());
    let target = Button::new();
    let container = Border::new();
    container.set_child(Some(target.clone().upcast()));
    let root = TestRoot::with_child(&container);

    target.focus();
    container.set_is_visible(false);

    assert!(root.focus_manager().get_focused_element().is_none());
}

#[test]
fn focus_should_be_cleared_when_control_is_disabled() {
    let _app = UnitTestApplication::start(real_focus());
    let target = Button::new();
    let root = TestRoot::with_child(&target);

    target.focus();
    target.set_is_enabled(false);

    assert!(root.focus_manager().get_focused_element().is_none());
}

#[test]
fn focus_should_be_cleared_when_control_is_effectively_disabled() {
    let _app = UnitTestApplication::start(real_focus());
    let target = Button::new();
    let container = Border::new();
    container.set_child(Some(target.clone().upcast()));
    let root = TestRoot::with_child(&container);

    target.focus();
    container.set_is_enabled(false);

    assert!(root.focus_manager().get_focused_element().is_none());
}

#[test]
fn focus_should_be_cleared_when_control_is_removed_from_visual_tree() {
    let _app = UnitTestApplication::start(real_focus());
    let target = Button::new();
    let root = TestRoot::with_child(&target);

    target.focus();
    root.set_child(None);

    assert!(root.focus_manager().get_focused_element().is_none());
}

#[test]
fn focus_pseudoclass_should_be_applied_on_focus() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = focusable_decorator();
    let target2 = focusable_decorator();
    let _root = TestRoot::with_child(&stack_panel(&[&target1, &target2]));

    target1.apply_template();
    target2.apply_template();

    target1.focus();
    assert!(target1.is_focused());
    assert!(target1.classes().contains(":focus"));
    assert!(!target2.is_focused());
    assert!(!target2.classes().contains(":focus"));

    target2.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
    assert!(!target1.is_focused());
    assert!(!target1.classes().contains(":focus"));
    assert!(target2.is_focused());
    assert!(target2.classes().contains(":focus"));
}

#[test]
fn control_focus_vsisible_pseudoclass_should_be_applied_on_tab_and_directional_focus() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = focusable_decorator();
    let target2 = focusable_decorator();
    let _root = TestRoot::with_child(&stack_panel(&[&target1, &target2]));

    target1.apply_template();
    target2.apply_template();

    target1.focus();
    assert!(target1.is_focused());
    assert!(!target1.classes().contains(":focus-visible"));
    assert!(!target2.is_focused());
    assert!(!target2.classes().contains(":focus-visible"));

    target2.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
    assert!(!target1.is_focused());
    assert!(!target1.classes().contains(":focus-visible"));
    assert!(target2.is_focused());
    assert!(target2.classes().contains(":focus-visible"));

    target1.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
    assert!(target1.is_focused());
    assert!(target1.classes().contains(":focus-visible"));
    assert!(!target2.is_focused());
    assert!(!target2.classes().contains(":focus-visible"));
}

#[test]
fn control_focus_within_pseudo_class_should_be_applied() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = focusable_decorator();
    let target2 = focusable_decorator();
    let root = TestRoot::with_child(&stack_panel(&[&target1, &target2]));

    target1.apply_template();
    target2.apply_template();

    target1.focus();
    let root_child = root.child().unwrap();
    assert!(target1.is_focused());
    assert!(target1.classes().contains(":focus-within"));
    assert!(target1.is_keyboard_focus_within());
    assert!(root_child.classes().contains(":focus-within"));
    assert!(root_child.is_keyboard_focus_within());
    assert!(root.classes().contains(":focus-within"));
    assert!(root.is_keyboard_focus_within());
}

#[test]
fn control_focus_within_pseudo_class_should_be_applied_and_removed() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = focusable_decorator();
    let target2 = focusable_decorator();
    let panel1 = panel(&[&target1]);
    let panel2 = panel(&[&target2]);
    let root = TestRoot::with_child(&stack_panel(&[&panel1, &panel2]));

    target1.apply_template();
    target2.apply_template();

    target1.focus();
    let root_child = root.child().unwrap();
    assert!(target1.is_focused());
    assert!(target1.classes().contains(":focus-within"));
    assert!(target1.is_keyboard_focus_within());
    assert!(panel1.classes().contains(":focus-within"));
    assert!(panel1.is_keyboard_focus_within());
    assert!(root_child.classes().contains(":focus-within"));
    assert!(root_child.is_keyboard_focus_within());
    assert!(root.classes().contains(":focus-within"));
    assert!(root.is_keyboard_focus_within());

    target2.focus();

    assert!(!target1.is_focused());
    assert!(!target1.classes().contains(":focus-within"));
    assert!(!target1.is_keyboard_focus_within());
    assert!(!panel1.classes().contains(":focus-within"));
    assert!(!panel1.is_keyboard_focus_within());
    assert!(root_child.classes().contains(":focus-within"));
    assert!(root_child.is_keyboard_focus_within());
    assert!(root.classes().contains(":focus-within"));
    assert!(root.is_keyboard_focus_within());

    assert!(target2.is_focused());
    assert!(target2.classes().contains(":focus-within"));
    assert!(target2.is_keyboard_focus_within());
    assert!(panel2.classes().contains(":focus-within"));
    assert!(panel2.is_keyboard_focus_within());
}

#[test]
fn control_focus_within_pseudoclass_should_be_removed_when_removed_from_tree() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = focusable_decorator();
    let target2 = focusable_decorator();
    let root = TestRoot::with_child(&stack_panel(&[&target1, &target2]));

    target1.apply_template();
    target2.apply_template();

    target1.focus();
    let root_child = root.child().unwrap();
    assert!(target1.is_focused());
    assert!(target1.classes().contains(":focus-within"));
    assert!(target1.is_keyboard_focus_within());
    assert!(root_child.classes().contains(":focus-within"));
    assert!(root_child.is_keyboard_focus_within());
    assert!(root.classes().contains(":focus-within"));
    assert!(root.is_keyboard_focus_within());

    let keyboard_device = KeyboardDevice::instance().unwrap();
    assert_eq!(keyboard_device.focused_element(), el(&target1));

    root.set_child(None);

    assert!(keyboard_device.focused_element().is_none());

    assert!(!target1.is_focused());
    assert!(!target1.classes().contains(":focus-within"));
    assert!(!target1.is_keyboard_focus_within());
    assert!(!root.classes().contains(":focus-within"));
    assert!(!root.is_keyboard_focus_within());
}

#[test]
fn control_focus_within_pseudoclass_should_be_removed_focus_moves_to_different_root() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = focusable_decorator();
    let target2 = focusable_decorator();

    let root1 = TestRoot::with_child(&stack_panel(&[&target1]));

    let root2 = TestRoot::with_child(&stack_panel(&[&target2]));

    target1.apply_template();
    target2.apply_template();

    target1.focus();
    let root1_child = root1.child().unwrap();
    let root2_child = root2.child().unwrap();
    assert!(target1.is_focused());
    assert!(target1.classes().contains(":focus-within"));
    assert!(target1.is_keyboard_focus_within());
    assert!(root1_child.classes().contains(":focus-within"));
    assert!(root1_child.is_keyboard_focus_within());
    assert!(root1.classes().contains(":focus-within"));
    assert!(root1.is_keyboard_focus_within());

    assert_eq!(KeyboardDevice::instance().unwrap().focused_element(), el(&target1));

    target2.focus();

    assert!(!target1.is_focused());
    assert!(!target1.classes().contains(":focus-within"));
    assert!(!target1.is_keyboard_focus_within());
    assert!(!root1_child.classes().contains(":focus-within"));
    assert!(!root1_child.is_keyboard_focus_within());
    assert!(!root1.classes().contains(":focus-within"));
    assert!(!root1.is_keyboard_focus_within());

    assert!(target2.is_focused());
    assert!(target2.classes().contains(":focus-within"));
    assert!(target2.is_keyboard_focus_within());
    assert!(root2_child.classes().contains(":focus-within"));
    assert!(root2_child.is_keyboard_focus_within());
    assert!(root2.classes().contains(":focus-within"));
    assert!(root2.is_keyboard_focus_within());
}

#[test]
fn can_clear_focus() {
    let _app = UnitTestApplication::start(real_focus());
    let target = Button::new();
    let root = TestRoot::with_child(&target);

    target.focus();
    root.focus_manager().focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);

    assert!(root.focus_manager().get_focused_element().is_none());
}

#[test]
fn removing_focused_element_inside_focus_scope_activates_root_focus_scope() {
    // Issue #13325
    let _app = UnitTestApplication::start(real_focus());
    let inner_button = Button::new();
    let intermediate_button = Button::new();
    let outer_button = Button::new();
    let inner_scope = TestFocusScope::new();
    inner_scope.children().add(inner_button.clone());
    // Intermediate focus scope to make sure that the root focus scope gets
    // activated, not this one.
    let intermediate_scope = TestFocusScope::new();
    intermediate_scope.children().add(inner_scope.clone());
    intermediate_scope.children().add(intermediate_button.clone());
    let root = TestRoot::with_child(&stack_panel(&[&intermediate_scope, &outer_button]));

    // Focus a control in each scope, ending with the innermost one.
    outer_button.focus();
    intermediate_button.focus();
    inner_button.focus();

    // Remove the focused control from the tree.
    inner_button.parent().unwrap().cast::<Panel>().unwrap().children().remove(inner_button.clone());

    let focus_manager = root.focus_manager();
    assert_eq!(el(&outer_button), focus_manager.get_focused_element());
    assert!(focus_manager.get_focused_element_in_scope(&inner_scope).is_none());
}

#[test]
fn removing_focus_scope_activates_root_focus_scope() {
    let _app = UnitTestApplication::start(real_focus());
    let inner_button = Button::new();
    let outer_button = Button::new();
    let inner_scope = TestFocusScope::new();
    inner_scope.children().add(inner_button.clone());
    let root = TestRoot::with_child(&stack_panel(&[&inner_scope, &outer_button]));

    // Focus a control in the top-level and inner focus scopes.
    outer_button.focus();
    inner_button.focus();

    // Remove the inner focus scope.
    inner_scope.parent().unwrap().cast::<Panel>().unwrap().children().remove(inner_scope.clone());

    let focus_manager = root.focus_manager();
    assert_eq!(el(&outer_button), focus_manager.get_focused_element());
}

#[test]
fn switching_focus_scope_changes_focus() {
    let _app = UnitTestApplication::start(real_focus());
    let inner_button = Button::new();
    let outer_button = Button::new();
    let inner_scope = TestFocusScope::new();
    inner_scope.children().add(inner_button.clone());
    let root = TestRoot::with_child(&stack_panel(&[&inner_scope, &outer_button]));

    // Focus a control in the top-level and inner focus scopes.
    outer_button.focus();
    inner_button.focus();

    let focus_manager = root.focus_manager();
    assert_eq!(el(&inner_button), focus_manager.get_focused_element());

    focus_manager.set_focus_scope(&root.clone().upcast());
    assert_eq!(el(&outer_button), focus_manager.get_focused_element());

    focus_manager.set_focus_scope(&inner_scope.clone().upcast());
    assert_eq!(el(&inner_button), focus_manager.get_focused_element());
}

// Upstream issue 13134.
#[test]
fn set_focus_scope_on_non_focusable_scope_changes_scope() {
    let _app = UnitTestApplication::start(real_focus());

    let inner_scope = TestFocusScope::new();
    let outer_button = Button::new();
    let stack = stack_panel(&[&inner_scope, &outer_button]);
    stack.set_focusable(false);
    let root = TestRoot::with_child(&stack);

    outer_button.focus();

    let focus_manager = root.focus_manager();
    assert_eq!(el(&outer_button), focus_manager.get_focused_element());

    // Switch to a scope that has no previously focused element and isn't focusable itself.
    // TestFocusScope is a Panel (Focusable = false) + IFocusScope.
    focus_manager.set_focus_scope(&inner_scope.clone().upcast());

    // Focus must be cleared: the scope is not focusable and has no prior focused element.
    // Before the fix this was a no-op and outerButton would still be reported as focused.
    assert!(focus_manager.get_focused_element().is_none());
}

#[test]
fn can_get_first_focusable_element() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let target2 = button("2");
    let target3 = button("3");
    let target4 = button("4");
    let container = stack_panel(&[&target1, &target2, &target3, &target4]);
    let root = TestRoot::with_child(&container);

    let first_focusable = FocusManager::find_first_focusable_element_in(&container.clone().upcast());

    assert_eq!(el(&target1), first_focusable);

    let first_focusable = root.focus_manager().find_first_focusable_element();

    assert_eq!(el(&target1), first_focusable);
}

#[test]
fn get_first_focusable_element_skips_unfocusable_elements() {
    let _app = UnitTestApplication::start(real_focus());
    let skip1 = button("s1");
    skip1.set_focusable(false);
    let skip2 = TextBlock::new();
    skip2.set_text(Some("s2"));
    let skip3 = StackPanel::new();
    let target4 = button("4");
    let target5 = button("5");
    let container = stack_panel(&[&skip1, &skip2, &skip3, &target4, &target5]);
    let root = TestRoot::with_child(&container);

    let first_focusable = FocusManager::find_first_focusable_element_in(&container.clone().upcast());

    assert_eq!(el(&target4), first_focusable);

    let first_focusable = root.focus_manager().find_first_focusable_element();

    assert_eq!(el(&target4), first_focusable);
}

#[test]
fn can_get_last_focusable_element() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let target2 = button("2");
    let target3 = button("3");
    let target4 = button("4");
    let container = stack_panel(&[&target1, &target2, &target3, &target4]);
    let root = TestRoot::with_child(&container);

    let last_focusable = FocusManager::find_last_focusable_element_in(&container.clone().upcast());

    assert_eq!(el(&target4), last_focusable);

    let last_focusable = root.focus_manager().find_last_focusable_element();

    assert_eq!(el(&target4), last_focusable);
}

#[test]
fn get_last_focusable_element_skips_unfocusable_elements() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let target2 = button("2");
    let skip3 = button("s3");
    skip3.set_focusable(false);
    let skip4 = TextBlock::new();
    skip4.set_text(Some("s4"));
    let skip5 = StackPanel::new();
    let container = stack_panel(&[&target1, &target2, &skip3, &skip4, &skip5]);
    let root = TestRoot::with_child(&container);

    let last_focusable = FocusManager::find_last_focusable_element_in(&container.clone().upcast());

    assert_eq!(el(&target2), last_focusable);

    let last_focusable = root.focus_manager().find_last_focusable_element();

    assert_eq!(el(&target2), last_focusable);
}

#[test]
fn can_get_next_element() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let target2 = button("2");
    let target3 = button("3");
    let target4 = button("4");
    let container = stack_panel(&[&target1, &target2, &target3, &target4]);
    let _root = TestRoot::with_child(&container);

    let focus_manager = FocusManager::get_focus_manager(&container);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();
    target1.focus();

    let next = focus_manager.find_next_element(NavigationDirection::Next, None);

    assert_eq!(next, el(&target2));
}

#[test]
fn can_get_next_element_out_of_container_with_tab_navigation_once() {
    let _app = UnitTestApplication::start(real_focus());
    let inside = button("inside");
    let after = button("after");

    // The focused element has to sit at least one level below the Once container:
    // GetFocusParent(focused) must resolve to something *deeper* than that container,
    // otherwise the reset below happens to land on the correct node and the walk
    // terminates by accident.
    let once = stack_panel(&[&stack_panel(&[&inside])]);
    KeyboardNavigation::set_tab_navigation(&once, KeyboardNavigationMode::Once);
    let _root = TestRoot::with_child(&stack_panel(&[&once, &after]));

    let focus_manager = FocusManager::get_focus_manager(&inside);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();
    inside.focus();

    // Before the fix this call never returned: on every Once hit the parent walk was
    // reset to the focused element's parent, so it oscillated between the same two
    // nodes forever and burned 100% CPU on the UI thread.
    let next = focus_manager.find_next_element(NavigationDirection::Next, None);

    assert_eq!(el(&after), next);
}

#[test]
fn can_get_previous_element_out_of_container_with_tab_navigation_once() {
    let _app = UnitTestApplication::start(real_focus());
    let before = button("before");
    before.set_name(Some("before".to_string()));
    let inside = button("inside");
    inside.set_name(Some("inside".to_string()));

    // Same shape as the Next case. The Once container itself must stay unfocusable,
    // otherwise GetPreviousTabStop returns it before reaching the faulty branch.
    let once = stack_panel(&[&stack_panel(&[&inside])]);
    KeyboardNavigation::set_tab_navigation(&once, KeyboardNavigationMode::Once);
    let _root = TestRoot::with_child(&stack_panel(&[&before, &once]));

    let focus_manager = FocusManager::get_focus_manager(&inside);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();
    inside.focus();

    // Before the fixes this call never returned. The walk must leave the Once
    // container and land on the element preceding it.
    let previous = focus_manager.find_next_element(NavigationDirection::Previous, None);

    assert_eq!(el(&before), previous);
}

#[test]
fn can_get_previous_element() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let target2 = button("2");
    let target3 = button("3");
    let target4 = button("4");
    let container = stack_panel(&[&target1, &target2, &target3, &target4]);
    let _root = TestRoot::with_child(&container);

    let focus_manager = FocusManager::get_focus_manager(&container);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();
    target3.focus();

    // Must return the closest preceding sibling: not target1 (which merely comes
    // first) and not target4 (which comes after the focused element).
    let previous = focus_manager.find_next_element(NavigationDirection::Previous, None);

    assert_eq!(el(&target2), previous);
}

#[test]
fn previous_wraps_to_last_element_in_cycle_container() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let target2 = button("2");
    let target3 = button("3");
    let cycle = stack_panel(&[&target1, &target2, &target3]);
    KeyboardNavigation::set_tab_navigation(&cycle, KeyboardNavigationMode::Cycle);
    let _root = TestRoot::with_child(&cycle);

    let focus_manager = FocusManager::get_focus_manager(&target1);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();
    target1.focus();

    // Wrapping backwards inside a Cycle scope must land on the last focusable
    // element, mirroring the forward wrap (last -> first). It used to take the
    // FIRST element - the focused element itself - making Previous a no-op.
    let previous = focus_manager.find_next_element(NavigationDirection::Previous, None);

    assert_eq!(el(&target3), previous);
}

#[test]
fn can_get_next_element_with_focused_element_option() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let target2 = button("2");
    let target3 = button("3");
    let target4 = button("4");
    let container = stack_panel(&[&target1, &target2, &target3, &target4]);
    let _root = TestRoot::with_child(&container);

    let focus_manager = FocusManager::get_focus_manager(&container);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();
    assert!(focus_manager.get_focused_element().is_none());

    let next = focus_manager.find_next_element(
        NavigationDirection::Next,
        Some(&FindNextElementOptions { focused_element: el(&target1), ..Default::default() }),
    );

    assert_eq!(next, el(&target2));
}

#[test]
fn can_get_directional_next_element_with_options() {
    let (_text, services) = real_focus_with_text_services();
    let _app = UnitTestApplication::start(services);
    let target1 = button("1");
    let target2 = button("2");
    let target3 = button("3");
    let target4 = button("4");
    let target5 = button("5");
    let seach_stack = stack_panel(&[&target3, &target4]);
    let container = stack_panel(&[&target1, &target2, &seach_stack, &target5]);
    container.set_orientation(Orientation::Horizontal);
    let root = TestRoot::with_child(&container);

    root.invalidate_measure();
    root.execute_initial_layout_pass();

    let focus_manager = FocusManager::get_focus_manager(&container);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();
    target1.focus();

    let options = FindNextElementOptions { search_root: el(&seach_stack), ..Default::default() };

    // Search root is right of the current focus, should return the first focusable element in the search root
    let next = focus_manager.find_next_element(NavigationDirection::Right, Some(&options));

    assert_eq!(next, el(&target3));

    target5.focus();

    // Search root is right of the current focus, should return the first focusable element in the search root
    let next = focus_manager.find_next_element(NavigationDirection::Left, Some(&options));

    assert_eq!(next, el(&target3));

    // Search root isn't to the right of the current focus, should return null
    let next = focus_manager.find_next_element(NavigationDirection::Right, Some(&options));

    assert!(next.is_none());
}

#[test]
fn can_get_directional_next_element_with_focused_element_option() {
    let (_text, services) = real_focus_with_text_services();
    let _app = UnitTestApplication::start(services);
    let target1 = button("1");
    let target2 = button("2");
    let target3 = button("3");
    let target4 = button("4");
    let target5 = button("5");
    let search_stack = stack_panel(&[&target3, &target4]);
    let container = stack_panel(&[&target1, &target2, &search_stack, &target5]);
    container.set_orientation(Orientation::Horizontal);
    let root = TestRoot::with_child(&container);

    root.invalidate_measure();
    root.execute_initial_layout_pass();

    let focus_manager = FocusManager::get_focus_manager(&container);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();
    assert!(focus_manager.get_focused_element().is_none());

    // Search root is right of the specified focused element, should return the first focusable element in the search root
    let next = focus_manager.find_next_element(
        NavigationDirection::Right,
        Some(&FindNextElementOptions {
            search_root: el(&search_stack),
            focused_element: el(&target1),
            ..Default::default()
        }),
    );

    assert_eq!(next, el(&target3));

    // Search root is left of the specified focused element, should return the first focusable element in the search root
    let next = focus_manager.find_next_element(
        NavigationDirection::Left,
        Some(&FindNextElementOptions {
            search_root: el(&search_stack),
            focused_element: el(&target5),
            ..Default::default()
        }),
    );

    assert_eq!(next, el(&target3));

    // Search root isn't to the right of the specified focused element, should return null
    let next = focus_manager.find_next_element(
        NavigationDirection::Right,
        Some(&FindNextElementOptions {
            search_root: el(&search_stack),
            focused_element: el(&target5),
            ..Default::default()
        }),
    );

    assert!(next.is_none());
}

#[test]
fn focus_should_move_according_to_direction() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let target2 = button("2");
    let target3 = button("3");
    let target4 = button("4");
    let container = stack_panel(&[&target1, &target2, &target3, &target4]);
    let _root = TestRoot::with_child(&container);

    let focus_manager = FocusManager::get_focus_manager(&container);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();

    let has_moved = focus_manager.try_move_focus(NavigationDirection::Next, None);

    assert!(target1.is_focused());
    assert!(has_moved);

    let has_moved = focus_manager.try_move_focus(NavigationDirection::Previous, None);

    assert!(target4.is_focused());
    assert!(has_moved);
}

/// The common part of the three skip tests: three moves to the next element
/// land on `target1`, `target2` and `target3`.
fn assert_moves_through(container: &Ref<StackPanel>, targets: [&Ref<Button>; 3]) {
    let focus_manager = FocusManager::get_focus_manager(container);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();

    for target in targets {
        let has_moved = focus_manager.try_move_focus(NavigationDirection::Next, None);

        assert!(target.is_focused());
        assert!(has_moved);
    }
}

#[test]
fn focus_should_skip_elements_with_focusable_equal_false() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let skip1 = button("s1");
    skip1.set_focusable(false);
    let target2 = button("2");
    let skip2 = button("s2");
    skip2.set_focusable(false);
    let target3 = button("3");
    let container = stack_panel(&[&target1, &skip1, &target2, &skip2, &target3]);
    let _root = TestRoot::with_child(&container);

    assert_moves_through(&container, [&target1, &target2, &target3]);
}

#[test]
fn focus_should_skip_elements_with_is_tab_stop_equal_false() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let skip1 = button("s1");
    skip1.set_is_tab_stop(false);
    let target2 = button("2");
    let skip2 = button("s2");
    skip2.set_is_tab_stop(false);
    let target3 = button("3");
    let container = stack_panel(&[&target1, &skip1, &target2, &skip2, &target3]);
    let _root = TestRoot::with_child(&container);

    assert_moves_through(&container, [&target1, &target2, &target3]);
}

#[test]
fn focus_should_skip_text_block_elements() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let skip1 = TextBlock::new();
    skip1.set_focusable(false);
    skip1.set_text(Some("s1"));
    let target2 = button("2");
    let skip2 = TextBlock::new();
    skip2.set_focusable(false);
    skip2.set_text(Some("s2"));
    let target3 = button("3");
    let container = stack_panel(&[&target1, &skip1, &target2, &skip2, &target3]);
    let _root = TestRoot::with_child(&container);

    assert_moves_through(&container, [&target1, &target2, &target3]);
}

#[test]
fn focus_should_skip_empty_containers() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let skip1 = StackPanel::new();
    let target2 = button("2");
    let skip2 = StackPanel::new();
    let target3 = button("3");
    let container = stack_panel(&[&target1, &skip1, &target2, &skip2, &target3]);
    let _root = TestRoot::with_child(&container);

    assert_moves_through(&container, [&target1, &target2, &target3]);
}

#[test]
fn focus_should_move_according_to_xy_direction() {
    let _app = UnitTestApplication::start(real_focus());
    let target1 = button("1");
    let target2 = button("2");
    let target3 = button("3");
    let target4 = button("4");
    let center = Button::new();
    XYFocus::set_left(&center, el(&target1));
    XYFocus::set_right(&center, el(&target2));
    XYFocus::set_up(&center, el(&target3));
    XYFocus::set_down(&center, el(&target4));
    let container = Canvas::new();
    for child in [&target1, &target2, &target3, &target4, &center] {
        container.children().add(child.clone());
    }

    let _root = TestRoot::with_child(&container);

    let focus_manager = FocusManager::get_focus_manager(&container);
    assert!(focus_manager.is_some());
    let focus_manager = focus_manager.unwrap();

    center.focus();

    let options = FindNextElementOptions { search_root: el(&container), ..Default::default() };

    let has_moved = focus_manager.try_move_focus(NavigationDirection::Up, Some(&options));
    assert!(target3.is_focused());
    assert!(has_moved);
}

#[test]
fn focus_in_scope_should_not_change_when_focus_canceled() {
    let _app = UnitTestApplication::start(real_focus());
    let first = named_button("First");
    let second = named_button("Second");

    let root = TestRoot::with_child(&stack_panel(&[&first, &second]));

    let focus_manager = root.focus_manager();

    // Focus the first element
    first.focus();
    assert_eq!(el(&first), focus_manager.get_focused_element_in_scope(&root));

    // Cancel focus change
    second.add_handler(&InputElement::getting_focus_event(), |_, e| {
        e.try_cancel();
    });

    // Move the focus to the second element: it should fail
    let focus_result = focus_manager.focus(el(&second).as_ref(), NavigationMethod::Unspecified, KeyModifiers::NONE);
    assert!(!focus_result);
    assert_eq!(el(&first), keyboard_focused_element());

    // FocusedElement for the scope should remain the same
    let new_focused_element_in_scope = focus_manager.get_focused_element_in_scope(&root);
    assert_eq!(el(&first), new_focused_element_in_scope);
}

#[test]
fn focus_in_scope_should_match_redirected_element_when_focus_redirected() {
    let _app = UnitTestApplication::start(real_focus());
    let first = named_button("First");
    let second = named_button("Second");
    let third = named_button("Third");

    let root = TestRoot::with_child(&stack_panel(&[&first, &second, &third]));

    let focus_manager = root.focus_manager();

    // Focus the first element
    first.focus();
    assert_eq!(el(&first), focus_manager.get_focused_element_in_scope(&root));

    // Redirect focus change
    let redirect = el(&third);
    second.add_handler(&InputElement::getting_focus_event(), move |_, e| {
        e.try_set_new_focused_element(redirect.as_ref());
    });

    // Move the focus to the second element: it should fail
    let focus_result = focus_manager.focus(el(&second).as_ref(), NavigationMethod::Unspecified, KeyModifiers::NONE);
    assert!(!focus_result);
    assert_eq!(el(&third), keyboard_focused_element());

    // FocusedElement for the scope should have moved to the redirected element
    let new_focused_element_in_scope = focus_manager.get_focused_element_in_scope(&root);
    assert_eq!(el(&third), new_focused_element_in_scope);
}

/// C# `window.PlatformImpl?.Activated?.Invoke()`.
fn activate(window: &Window) {
    if let Some(activated) = window.platform_impl().and_then(|platform| platform.activated()) {
        activated();
    }
}

#[test]
fn focus_should_return_to_first_window_when_second_is_closed() {
    let _app = UnitTestApplication::start(styled_window_with_keyboard_device());
    let first = named_button("FirstButton");
    let second = named_button("SecondButton");

    let window1 = Window::new();
    window1.set_content(Some(Control::boxed(first.clone())));

    let window2 = Window::new();
    window2.set_content(Some(Control::boxed(second.clone())));

    window1.show();

    // Focus the first button in the first window
    first.focus();
    assert_eq!(el(&first), keyboard_focused_element());
    assert_eq!(el(&first), window1.focus_manager().get_focused_element());

    window2.show();

    // Focus the second button in the second window
    second.focus();
    assert_eq!(el(&second), keyboard_focused_element());
    assert_eq!(el(&second), window2.focus_manager().get_focused_element());

    // Close the second window, focus should be lost
    window2.close();
    assert!(keyboard_focused_element().is_none());
    assert!(window2.focus_manager().get_focused_element().is_none());

    // Activate the first window again
    activate(&window1);

    // Focus should have moved back to the first button in the first window
    assert_eq!(el(&first), keyboard_focused_element());
    assert_eq!(el(&first), window1.focus_manager().get_focused_element());
}

#[test]
fn focus_should_stay_in_active_window() {
    let _app = UnitTestApplication::start(styled_window_with_keyboard_device());
    let first = named_button("FirstButton");
    let second = named_button("SecondButton");

    let window1 = Window::new();
    window1.set_content(Some(Control::boxed(first.clone())));

    let window2 = Window::new();
    window2.set_content(Some(Control::boxed(second.clone())));

    window1.show();

    // Focus the first button in the first window
    first.focus();
    assert_eq!(el(&first), keyboard_focused_element());
    assert_eq!(el(&first), window1.focus_manager().get_focused_element());

    window2.show();

    // Focus the second button in the second window
    second.focus();
    assert_eq!(el(&second), keyboard_focused_element());
    assert_eq!(el(&second), window2.focus_manager().get_focused_element());

    // Activate the first window again
    activate(&window1);

    // Focus should have moved back to the first button in the first window
    assert_eq!(el(&first), keyboard_focused_element());
    assert_eq!(el(&first), window1.focus_manager().get_focused_element());

    // Close the second window
    window2.close();

    // Focus should still be in the first window
    assert_eq!(el(&first), keyboard_focused_element());
    assert_eq!(el(&first), window1.focus_manager().get_focused_element());
}

#[test]
fn focus_should_not_be_restored_to_detached_control() {
    let _app = UnitTestApplication::start(styled_window_with_keyboard_device());
    let button = named_button("Button");
    let child_menu = MenuItem::new();
    child_menu.set_header(boxed_str("Bar"));
    let top_level_menu = MenuItem::new();
    top_level_menu.set_header(boxed_str("Foo"));
    top_level_menu.items().add(Some(Control::boxed(child_menu.clone())));
    let menu = Menu::new();
    menu.items().add(Some(Control::boxed(top_level_menu.clone())));
    let panel = StackPanel::new();
    panel.children().add(button.clone());
    panel.children().add(menu.clone());

    let window = Window::new();
    window.set_content(Some(Control::boxed(panel.clone())));
    window.set_name(Some("Window1".to_string()));

    window.show();

    // Focus the button
    button.focus();
    assert_eq!(el(&button), keyboard_focused_element());
    assert_eq!(el(&button), window.focus_manager().get_focused_element());

    // Open the menu and focus the child menu
    menu.open();
    top_level_menu.set_is_sub_menu_open(true);
    child_menu.focus();

    // Remove the previously focused button.
    panel.children().remove(button.clone());

    // Close the menus.
    menu.close();
    top_level_menu.close();

    activate(&window);

    // When window is activated, focus should be empty
    assert!(keyboard_focused_element().is_none());
    assert!(window.focus_manager().get_focused_element().is_none());
}
