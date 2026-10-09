//! Tests for styles, style collections, control themes and setters.
//!
//! Ported from the style tests of the reference implementation, with the
//! classes of `test_support` in place of controls and observables in place
//! of binding descriptions.

use super::test_support::*;
use super::*;
use crate::controls::{IResourceHost, IResourceNode, ResourceDictionary, ResourceHostRef, ResourceKey, ResourceValue, ResourcesChangedEventArgs};
use crate::data::BindingPriority;
use crate::property_store::FrameType;
use crate::reactive::{Disposable, IDisposable, IObservable, IObserver, LightweightSubject, Observable};
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn foo_setter(value: &str) -> Rc<Setter> {
    Setter::new(Class1::foo_property(), value.to_string())
}

fn foo_style(selector: Selector, value: &str) -> Ref<Style> {
    Style::with_setters(selector, [foo_setter(value)])
}

/// A binding-like source that counts its subscriptions.
struct TestSource {
    value: String,
    subscriptions: Rc<Cell<i32>>,
}

impl IObservable<BoxedValue> for TestSource {
    fn subscribe(&self, observer: Rc<dyn IObserver<BoxedValue>>) -> Rc<dyn IDisposable> {
        self.subscriptions.set(self.subscriptions.get() + 1);
        observer.on_next(Rc::new(self.value.clone()));
        let subscriptions = self.subscriptions.clone();
        Disposable::create(move || subscriptions.set(subscriptions.get() - 1))
    }
}

fn source(value: &str) -> (Rc<dyn IObservable<BoxedValue>>, Rc<Cell<i32>>) {
    let subscriptions = Rc::new(Cell::new(0));
    (Rc::new(TestSource { value: value.to_string(), subscriptions: subscriptions.clone() }), subscriptions)
}

/// A resource host that records the notifications it receives.
#[derive(Default)]
struct TestHost {
    notifications: Cell<u32>,
}

impl IResourceNode for TestHost {
    fn has_resources(&self) -> bool {
        false
    }

    fn try_get_resource(&self, _key: &ResourceKey, _theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        None
    }
}

impl IResourceHost for TestHost {
    fn resources_changed(&self, _handler: Rc<dyn Fn(&ResourcesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    fn notify_hosted_resources_changed(&self, _e: ResourcesChangedEventArgs) {
        self.notifications.set(self.notifications.get() + 1);
    }
}

fn test_host() -> (Rc<TestHost>, ResourceHostRef) {
    let host = Rc::new(TestHost::default());
    (host.clone(), ResourceHostRef::from(host))
}

// --- Style -------------------------------------------------------------------

#[test]
fn style_with_only_type_selector_should_update_value() {
    let style = foo_style(Selectors::of_type::<Class1>(), "Foo");
    let target = Class1::new();

    try_attach(&style, &target, None);

    assert_eq!(target.foo(), "Foo");
}

#[test]
fn style_with_class_selector_should_update_and_restore_value() {
    let style = foo_style(Selectors::of_type::<Class1>().class("foo"), "Foo");
    let target = Class1::new();

    try_attach(&style, &target, None);
    assert_eq!(target.foo(), "foodefault");
    target.classes().add("foo");
    assert_eq!(target.foo(), "Foo");
    target.classes().remove("foo");
    assert_eq!(target.foo(), "foodefault");
}

#[test]
fn style_with_class_selector_should_update_and_restore_value_with_template_priority_value() {
    let style = foo_style(Selectors::of_type::<Class1>().class("foo"), "Foo");
    let target = Class1::new();
    target.set_value_with_priority(Class1::foo_property(), "unset-foo".to_string(), BindingPriority::Template);

    try_attach(&style, &target, None);
    assert_eq!(target.foo(), "unset-foo");
    target.classes().add("foo");
    assert_eq!(target.foo(), "Foo");
    target.classes().remove("foo");
    assert_eq!(target.foo(), "unset-foo");
}

#[test]
fn style_with_no_selector_should_apply_to_containing_control() {
    let style = Style::new();
    style.add_setter(foo_setter("Foo"));
    let target = Class1::new();

    try_attach(&style, &target, None);

    assert_eq!(target.foo(), "Foo");
}

#[test]
#[should_panic(expected = "Invalid selector: Template selector must be followed by control selector.")]
fn should_throw_for_selector_with_trailing_template_selector() {
    Style::with_selector(Selectors::of_type::<Class1>().template());
}

#[test]
fn style_with_no_selector_should_not_apply_to_other_control() {
    let style = Style::new();
    style.add_setter(foo_setter("Foo"));
    let target = Class1::new();
    let other = Class1::new();

    try_attach(&style, &target, Some(StyleHostRef::from(&other)));

    assert_eq!(target.foo(), "foodefault");
}

#[test]
fn local_value_should_override_style() {
    let style = foo_style(Selectors::of_type::<Class1>(), "Foo");
    let target = Class1::new();
    target.set_foo("Original");

    try_attach(&style, &target, None);

    assert_eq!(target.foo(), "Original");
}

#[test]
fn later_styles_should_override_earlier() {
    let styles = Styles::new();
    styles.add(foo_style(Selectors::of_type::<Class1>().class("foo"), "Foo"));
    styles.add(foo_style(Selectors::of_type::<Class1>().class("foo"), "Bar"));

    let target = Class1::new();
    let values = record_values(&target, Class1::foo_property());

    styles.try_attach(&target, None);
    target.classes().add("foo");
    target.classes().remove("foo");

    assert_eq!(*values.borrow(), vec!["foodefault", "Bar", "foodefault"]);
}

#[test]
fn later_styles_should_override_earlier_2() {
    let styles = Styles::new();
    styles.add(foo_style(Selectors::of_type::<Class1>().class("foo"), "Foo"));
    styles.add(foo_style(Selectors::of_type::<Class1>().class("bar"), "Bar"));

    let target = Class1::new();
    let values = record_values(&target, Class1::foo_property());

    styles.try_attach(&target, None);
    target.classes().add("bar");
    target.classes().add("foo");
    target.classes().remove("foo");

    assert_eq!(*values.borrow(), vec!["foodefault", "Bar"]);
}

#[test]
fn later_styles_should_override_earlier_3() {
    let (foo, _) = source("Foo");
    let (bar, _) = source("Bar");
    let styles = Styles::new();
    let style1 = Style::with_selector(Selectors::of_type::<Class1>().class("foo"));
    style1.add_setter(Setter::new_binding(Class1::foo_property(), foo));
    let style2 = Style::with_selector(Selectors::of_type::<Class1>().class("bar"));
    style2.add_setter(Setter::new_binding(Class1::foo_property(), bar));
    styles.add(style1);
    styles.add(style2);

    let target = Class1::new();
    let values = record_values(&target, Class1::foo_property());

    styles.try_attach(&target, None);
    target.classes().add("bar");
    target.classes().add("foo");
    target.classes().remove("foo");

    assert_eq!(*values.borrow(), vec!["foodefault", "Bar"]);
}

#[test]
fn later_styles_should_override_earlier_4() {
    let styles = Styles::new();
    styles.add(foo_style(Selectors::of_type::<Class1>().class("foo"), "foo1"));
    let second = foo_style(Selectors::of_type::<Class1>().class("foo"), "foo2");
    second.add_setter(Setter::new(Class1::double_property(), 123.4));
    styles.add(second);

    let target = Class1::new();
    styles.try_attach(&target, None);
    target.classes().add("foo");

    assert_eq!(target.foo(), "foo2");
    assert_eq!(target.double(), 123.4);
}

#[test]
fn later_styles_should_override_earlier_with_begin_end_styling() {
    let styles = Styles::new();
    let first = foo_style(Selectors::of_type::<Class1>().class("foo"), "foo1");
    first.add_setter(Setter::new(Class1::double_property(), 123.4));
    styles.add(first);
    styles.add(foo_style(Selectors::of_type::<Class1>().class("foo").class("bar"), "foo2"));

    let target = Class1::new();
    target.values().begin_styling();
    styles.try_attach(&target, None);
    target.values().end_styling(&target);

    target.classes().add("bar");
    target.classes().add("foo");

    assert_eq!(target.foo(), "foo2");
    assert_eq!(target.double(), 123.4);

    target.classes().remove("foo");

    assert_eq!(target.double(), 0.0);
}

fn root_with_two_styles(first: Ref<Style>, second: Ref<Style>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.styles().add(first);
    root.styles().add(second);
    root
}

#[test]
fn inactive_values_should_not_be_made_active_during_style_attach() {
    let root = root_with_two_styles(
        foo_style(Selectors::of_type::<Class1>(), "Foo"),
        foo_style(Selectors::of_type::<Class1>(), "Bar"),
    );

    let target = Class1::new();
    let values = record_values(&target, Class1::foo_property());

    set_child(&root, &target);

    assert_eq!(*values.borrow(), vec!["foodefault", "Bar"]);
}

#[test]
fn inactive_bindings_should_not_be_made_active_during_style_attach() {
    let (foo, foo_subscriptions) = source("Foo");
    let (bar, bar_subscriptions) = source("Bar");
    let first = Style::with_selector(Selectors::of_type::<Class1>());
    first.add_setter(Setter::new_binding(Class1::foo_property(), foo));
    let second = Style::with_selector(Selectors::of_type::<Class1>());
    second.add_setter(Setter::new_binding(Class1::foo_property(), bar));
    let root = root_with_two_styles(first, second);

    let target = Class1::new();
    let values = record_values(&target, Class1::foo_property());

    set_child(&root, &target);

    assert_eq!(*values.borrow(), vec!["foodefault", "Bar"]);
    assert_eq!(foo_subscriptions.get(), 0);
    assert_eq!(bar_subscriptions.get(), 1);
}

#[test]
fn inactive_values_should_not_be_made_active_during_style_detach() {
    let root = root_with_two_styles(
        foo_style(Selectors::of_type::<Class1>(), "Foo"),
        foo_style(Selectors::of_type::<Class1>(), "Bar"),
    );

    let target = Class1::new();
    set_child(&root, &target);

    let values = record_values(&target, Class1::foo_property());
    remove_child(&root, &target);

    assert_eq!(*values.borrow(), vec!["Bar", "foodefault"]);
}

#[test]
fn inactive_values_should_not_be_made_active_during_style_detach_2() {
    let root = root_with_two_styles(
        foo_style(Selectors::of_type::<Class1>().class("foo"), "Foo"),
        foo_style(Selectors::of_type::<Class1>(), "Bar"),
    );

    let target = Class1::new();
    target.classes().add("foo");
    set_child(&root, &target);

    let values = record_values(&target, Class1::foo_property());
    remove_child(&root, &target);

    assert_eq!(*values.borrow(), vec!["Foo", "foodefault"]);
}

#[test]
fn inactive_bindings_should_not_be_made_active_during_style_detach() {
    let (foo, foo_subscriptions) = source("Foo");
    let (bar, bar_subscriptions) = source("Bar");
    let first = Style::with_selector(Selectors::of_type::<Class1>());
    first.add_setter(Setter::new_binding(Class1::foo_property(), foo));
    let second = Style::with_selector(Selectors::of_type::<Class1>());
    second.add_setter(Setter::new_binding(Class1::foo_property(), bar));
    let root = root_with_two_styles(first, second);

    let target = Class1::new();
    set_child(&root, &target);

    let values = record_values(&target, Class1::foo_property());
    remove_child(&root, &target);

    assert_eq!(*values.borrow(), vec!["Bar", "foodefault"]);
    assert_eq!(foo_subscriptions.get(), 0);
    assert_eq!(bar_subscriptions.get(), 0);
}

struct ChildTemplate {
    instantiation_count: Rc<Cell<u32>>,
}

impl ITemplate for ChildTemplate {
    fn build(&self) -> BoxedValue {
        self.instantiation_count.set(self.instantiation_count.get() + 1);
        Rc::new(Some(Class1::new()))
    }
}

fn template_style(selector: Selector, count: &Rc<Cell<u32>>) -> Ref<Style> {
    let style = Style::with_selector(selector);
    style.add_setter(Setter::new_template(
        Class1::child_property(),
        Rc::new(ChildTemplate { instantiation_count: count.clone() }),
    ));
    style
}

#[test]
fn template_in_non_matching_style_is_not_built() {
    let count = Rc::new(Cell::new(0));
    let styles = Styles::new();
    styles.add(template_style(Selectors::of_type::<Class1>().class("foo"), &count));
    styles.add(template_style(Selectors::of_type::<Class1>(), &count));

    let target = Class1::new();
    target.values().begin_styling();
    styles.try_attach(&target, None);
    target.values().end_styling(&target);

    assert!(target.get_value(Class1::child_property()).is_some());
    assert_eq!(count.get(), 1);
}

#[test]
fn template_in_inactive_style_is_not_built() {
    let count = Rc::new(Cell::new(0));
    let styles = Styles::new();
    styles.add(template_style(Selectors::of_type::<Class1>(), &count));
    styles.add(template_style(Selectors::of_type::<Class1>(), &count));

    let target = Class1::new();
    target.values().begin_styling();
    styles.try_attach(&target, None);
    target.values().end_styling(&target);

    assert!(target.get_value(Class1::child_property()).is_some());
    assert_eq!(count.get(), 1);
}

#[test]
fn setter_should_materialize_template_once_per_control() {
    let count = Rc::new(Cell::new(0));
    let style = template_style(Selectors::of_type::<Class1>(), &count);
    let first = Class1::new();
    let second = Class1::new();

    try_attach(&style, &first, None);
    try_attach(&style, &second, None);

    let first_child = first.get_value(Class1::child_property()).unwrap();
    let second_child = second.get_value(Class1::child_property()).unwrap();
    assert!(first_child != second_child);
    assert_eq!(first.get_value(Class1::child_property()).unwrap(), first_child);
    assert_eq!(count.get(), 2);
}

#[test]
fn style_should_detach_when_control_removed_from_logical_tree() {
    let style = foo_style(Selectors::of_type::<Class1>(), "Foo");
    let target = Class1::new();
    let root = TestRoot::with_child(&target);

    try_attach(&style, &target, None);
    assert_eq!(target.foo(), "Foo");

    remove_child(&root, &target);
    assert_eq!(target.foo(), "foodefault");
}

#[test]
fn removing_style_should_detach_from_control() {
    let target = Class1::new();
    let root = TestRoot::new();
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "Foo"));
    set_child(&root, &target);

    root.measure(Size::INFINITY);
    assert_eq!(target.foo(), "Foo");

    root.styles().remove_at(0);
    assert_eq!(target.foo(), "foodefault");
}

#[test]
fn adding_style_should_attach_to_control() {
    let target = Class1::new();
    let root = TestRoot::new();
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "Foo"));
    set_child(&root, &target);

    root.measure(Size::INFINITY);
    assert_eq!(target.foo(), "Foo");

    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "Bar"));
    target.apply_styling();
    assert_eq!(target.foo(), "Bar");
}

#[test]
fn removing_style_with_nested_style_should_detach_from_control() {
    let target = Class1::new();
    let root = TestRoot::new();
    let nested = Styles::new();
    nested.add(foo_style(Selectors::of_type::<Class1>(), "Foo"));
    root.styles().add(&nested);
    set_child(&root, &target);

    assert_eq!(target.foo(), "Foo");

    root.styles().remove_at(0);
    assert_eq!(target.foo(), "foodefault");
}

#[test]
fn adding_nested_style_should_attach_to_control() {
    let target = Class1::new();
    let root = TestRoot::new();
    let nested = Styles::new();
    nested.add(foo_style(Selectors::of_type::<Class1>(), "Foo"));
    root.styles().add(&nested);
    set_child(&root, &target);

    assert_eq!(target.foo(), "Foo");

    nested.add(foo_style(Selectors::of_type::<Class1>(), "Bar"));
    target.apply_styling();
    assert_eq!(target.foo(), "Bar");
}

#[test]
fn removing_nested_style_should_detach_from_control() {
    let target = Class1::new();
    let root = TestRoot::new();
    let nested = Styles::new();
    nested.add(foo_style(Selectors::of_type::<Class1>(), "Foo"));
    nested.add(foo_style(Selectors::of_type::<Class1>(), "Bar"));
    root.styles().add(&nested);
    set_child(&root, &target);

    assert_eq!(target.foo(), "Bar");

    nested.remove_at(1);
    target.apply_styling();
    assert_eq!(target.foo(), "Foo");
}

#[test]
fn adding_style_with_no_setters_or_animations_should_not_invalidate_styles() {
    let target = Class1::new();
    let root = TestRoot::new();
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "Foo"));
    set_child(&root, &target);

    assert_eq!(target.foo(), "Foo");

    root.styles().add(Style::with_selector(Selectors::of_type::<Class1>()));

    assert_eq!(target.foo(), "Foo");
}

#[test]
fn invalidating_styles_should_detach_activator() {
    let style = foo_style(Selectors::of_type::<Class1>().class("foo"), "Foo");
    let target = Class1::new();

    try_attach(&style, &target, None);
    assert_eq!(target.classes().listener_count(), 1);

    target.invalidate_styles(false);
    assert_eq!(target.classes().listener_count(), 0);
}

#[test]
fn style_should_set_owner_on_assigned_resources() {
    let (_host, host_ref) = test_host();
    let target = Style::new();
    target.add_owner(&host_ref);

    let resources = ResourceDictionary::new();
    target.set_resources(resources.clone());

    assert!(resources.owner() == Some(host_ref));
}

#[test]
fn style_should_set_owner_on_assigned_resources_2() {
    let (_host, host_ref) = test_host();
    let target = Style::new();

    let resources = ResourceDictionary::new();
    target.set_resources(resources.clone());
    assert!(resources.owner().is_none());

    target.add_owner(&host_ref);

    assert!(resources.owner() == Some(host_ref));
}

#[test]
fn nested_style_can_be_added() {
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let nested = Style::with_selector(Selectors::nesting(None).class("foo"));

    parent.children().add(&nested);

    assert_eq!(nested.parent().unwrap(), parent);
}

#[test]
fn nested_or_style_can_be_added() {
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let nested = Style::with_selector(Selectors::or([
        Selectors::nesting(None).class("foo"),
        Selectors::nesting(None).class("bar"),
    ]));

    parent.children().add(&nested);

    assert_eq!(nested.parent().unwrap(), parent);
}

#[test]
#[should_panic(expected = "Child styles must have a selector.")]
fn nested_style_without_selector_throws() {
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let nested = Style::new();

    parent.children().add(&nested);
}

#[test]
fn nested_style_applies_to_control() {
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    parent.children().add(foo_style(Selectors::nesting(None).class("foo"), "Nested"));
    let root = TestRoot::new();
    root.styles().add(&parent);

    let target = Class1::new();
    set_child(&root, &target);
    assert_eq!(target.foo(), "foodefault");

    target.classes().add("foo");
    assert_eq!(target.foo(), "Nested");
}

#[test]
fn should_not_share_instance_when_or_selector_is_present() {
    let style = foo_style(
        Selectors::or([Selectors::of_type::<Class1>(), Selectors::of_type::<Class2>().class("bar")]),
        "Foo",
    );

    let target1 = Class1::new();
    target1.classes().add("foo");
    let target2 = Class2::new();

    try_attach(&style, &target1, None);
    try_attach(&style, &target2, None);

    assert_eq!(target1.foo(), "Foo");
    assert_eq!(target2.foo(), "foodefault");
}

#[test]
fn style_without_activator_shares_one_instance_between_controls() {
    let style = foo_style(Selectors::of_type::<Class1>(), "Foo");
    let first = Class1::new();
    let second = Class1::new();

    try_attach(&style, &first, None);
    try_attach(&style, &second, None);

    let first_frames = first.values().frames();
    let second_frames = second.values().frames();
    assert_eq!(first_frames.len(), 1);
    assert!(Rc::ptr_eq(&first_frames[0], &second_frames[0]));
    assert_eq!(first.foo(), "Foo");
    assert_eq!(second.foo(), "Foo");
}

#[test]
fn style_with_activator_does_not_share_instances() {
    let style = foo_style(Selectors::of_type::<Class1>().class("foo"), "Foo");
    let first = Class1::new();
    let second = Class1::new();

    try_attach(&style, &first, None);
    try_attach(&style, &second, None);

    assert!(!Rc::ptr_eq(&first.values().frames()[0], &second.values().frames()[0]));
    first.classes().add("foo");
    assert_eq!(first.foo(), "Foo");
    assert_eq!(second.foo(), "foodefault");
}

#[test]
#[should_panic(expected = "Duplicate setter encountered for property 'Foo' in 'Class1'.")]
fn duplicate_setters_are_rejected() {
    let style = Style::with_setters(Selectors::of_type::<Class1>(), [foo_setter("a"), foo_setter("b")]);
    try_attach(&style, &Class1::new(), None);
}

fn duplicate_foo_style() -> Ref<Style> {
    Style::with_setters(Selectors::of_type::<Class1>(), [foo_setter("a"), foo_setter("b")])
}

#[test]
fn a_duplicate_setter_is_an_error_of_the_fallible_styling_path() {
    let root = TestRoot::new();
    root.styles().add(duplicate_foo_style());
    let target = Class1::new();

    // Attached during initialization: styling is deferred to the end of it.
    target.begin_init();
    set_child(&root, &target);
    let error = target.try_end_init().expect_err("the style has two setters for a property");
    let expected = DuplicateSetterError { property: "Foo".to_string(), style: "Class1".to_string() };
    assert_eq!(error, InitializationError::DuplicateSetter(expected.clone()));
    assert_eq!(error.to_string(), "Duplicate setter encountered for property 'Foo' in 'Class1'.");

    // As after the exception of the managed original: the initialization was ended, the
    // element is neither styled nor initialized and no frame of the style was added.
    assert!(!target.is_initialized());
    assert!(target.values().frames().is_empty());
    assert_eq!(target.foo(), "foodefault");

    // The styling pass was ended: styling can be attempted again and fails the same way.
    assert_eq!(target.try_apply_styling(), Err(expected));

    // Without the offending style the element is styled.
    root.styles().clear();
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "Root"));
    assert_eq!(target.try_apply_styling(), Ok(true));
    assert_eq!(target.foo(), "Root");
}

#[test]
fn a_duplicate_setter_of_a_control_theme_is_an_error_of_the_fallible_styling_path() {
    let root = TestRoot::new();
    let target = Class1::new();
    let theme = ControlTheme::with_setters(Class1::TYPE, [foo_setter("a"), foo_setter("b")]);
    target.begin_init();
    target.set_theme(Some(theme));
    set_child(&root, &target);
    let error = target.try_end_init().expect_err("the theme has two setters for a property");
    assert!(error.to_string().starts_with("Duplicate setter encountered for property 'Foo' in '"), "{error}");
    assert!(target.values().frames().is_empty());
}

#[test]
fn the_initialization_contract_reports_a_duplicate_setter() {
    let root = TestRoot::new();
    root.styles().add(duplicate_foo_style());
    let target = Class1::new();
    let contract: Rc<dyn ISupportInitialize> = (&target).into();
    contract.begin_init();
    set_child(&root, &target);
    let error = contract.try_end_init().expect_err("the style has two setters for a property");
    assert_eq!(error.to_string(), "Duplicate setter encountered for property 'Foo' in 'Class1'.");

    // An element without a failing style ends its initialization.
    let other = Class1::new();
    let contract: Rc<dyn ISupportInitialize> = (&other).into();
    contract.begin_init();
    assert_eq!(contract.try_end_init(), Ok(()));
}

#[test]
#[should_panic(expected = "Duplicate setter encountered for property 'Foo' in 'Class1'.")]
fn a_duplicate_setter_panics_in_end_init() {
    let root = TestRoot::new();
    root.styles().add(duplicate_foo_style());
    let target = Class1::new();
    target.begin_init();
    set_child(&root, &target);
    target.end_init();
}

#[test]
#[should_panic(expected = "Duplicate setter encountered for property 'Foo' in 'Class1'.")]
fn a_duplicate_setter_panics_in_apply_styling() {
    let root = TestRoot::new();
    root.styles().add(duplicate_foo_style());
    // Attaching to the logical tree applies styling.
    set_child(&root, &Class1::new());
}

#[test]
fn style_display_string_is_its_selector() {
    assert_eq!(Style::with_selector(Selectors::of_type::<Class1>().class("foo")).to_display_string(), "Class1.foo");
    assert_eq!(Style::new().to_display_string(), "Style");
}

#[test]
fn styles_of_ancestors_apply_with_nearer_styles_taking_precedence() {
    let root = TestRoot::new();
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "Root"));
    let parent = Class3::new();
    set_child(&root, &parent);
    let target = Class1::new();
    set_child(&parent, &target);
    assert_eq!(target.foo(), "Root");

    let other = Class1::new();
    let nearer = Class3::new();
    nearer.styles().add(foo_style(Selectors::of_type::<Class1>(), "Parent"));
    set_child(&nearer, &other);
    set_child(&parent, &nearer);
    assert_eq!(other.foo(), "Parent");
}

// --- global styles -----------------------------------------------------------

/// The global style host: what the application object is to a tree.
struct GlobalStyles {
    styles: Ref<Styles>,
    resources: Ref<ResourceDictionary>,
    added: Cell<u32>,
    removed: Cell<u32>,
}

impl IResourceNode for GlobalStyles {
    fn has_resources(&self) -> bool {
        self.resources.has_resources() || self.styles.has_resources()
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.resources.try_get_resource(key, theme).or_else(|| self.styles.try_get_resource(key, theme))
    }
}

impl IResourceHost for GlobalStyles {
    fn resources_changed(&self, _handler: Rc<dyn Fn(&ResourcesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    fn notify_hosted_resources_changed(&self, _e: ResourcesChangedEventArgs) {}

    fn as_style_host(&self) -> Option<&dyn IStyleHost> {
        Some(self)
    }
}

impl IStyleHost for GlobalStyles {
    fn is_styles_initialized(&self) -> bool {
        true
    }

    fn styles(&self) -> Ref<Styles> {
        self.styles.clone()
    }

    fn styling_parent(&self) -> Option<StyleHostRef> {
        None
    }

    fn styles_added(&self, _styles: &[Rc<dyn IStyle>]) {
        self.added.set(self.added.get() + 1);
    }

    fn styles_removed(&self, _styles: &[Rc<dyn IStyle>]) {
        self.removed.set(self.removed.get() + 1);
    }
}

impl IGlobalStyles for GlobalStyles {
    fn global_styles_added(&self, _handler: Rc<GlobalStylesHandler>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    fn global_styles_removed(&self, _handler: Rc<GlobalStylesHandler>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

fn global_styles() -> Rc<GlobalStyles> {
    let host = Rc::new(GlobalStyles {
        styles: Styles::new(),
        resources: ResourceDictionary::new(),
        added: Cell::new(0),
        removed: Cell::new(0),
    });
    let host_ref = ResourceHostRef::from(host.clone());
    host.styles.add_owner(&host_ref);
    host.resources.add_owner(&host_ref);
    host
}

/// A logical root whose styling parent is a global style host.
#[repr(C)]
struct AppRoot {
    base: layout::Layoutable,
    global: RefCell<Option<Rc<GlobalStyles>>>,
}

ferro_class!(AppRoot: Layoutable);
ferro_impl_classes!(AppRoot: FerroObjectImpl, VisualImpl, LayoutableImpl);
use crate::layout::{Layoutable, LayoutableImpl};

impl StyledElementImpl for AppRoot {
    fn is_logical_root(_this: &Self) -> bool {
        true
    }

    fn styling_parent(this: &Self) -> Option<StyleHostRef> {
        let global: Rc<dyn IStyleHost> = this.global.borrow().clone()?;
        Some(StyleHostRef::Other(global))
    }
}

impl AppRoot {
    fn new(global: &Rc<GlobalStyles>) -> Ref<Self> {
        instantiate(Self { base: Layoutable::construct(), global: RefCell::new(Some(global.clone())) })
    }
}

#[test]
fn global_styles_apply_through_the_styling_parent_of_the_root() {
    let global = global_styles();
    global.styles.add(foo_style(Selectors::of_type::<Class1>(), "Global"));
    assert_eq!(global.added.get(), 1);

    let root = AppRoot::new(&global);
    let target = Class1::new();
    set_child(&root, &target);
    assert_eq!(target.foo(), "Global");

    // Styles of the tree take precedence over the global styles.
    let other = Class1::new();
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "Root"));
    set_child(&root, &other);
    assert_eq!(other.foo(), "Root");
}

#[test]
fn global_resources_are_found_through_the_styling_parent_of_the_root() {
    let global = global_styles();
    global.resources.add_value("foo", "global".to_string());

    let root = AppRoot::new(&global);
    let target = Class1::new();
    set_child(&root, &target);

    let found = target.try_find_resource(&"foo".into(), None).flatten().unwrap();
    assert_eq!((*found).downcast_ref::<String>().unwrap(), "global");
}

// --- Styles ------------------------------------------------------------------

#[test]
fn adding_style_should_set_owner() {
    let (_host, host_ref) = test_host();
    let target = Styles::new();
    target.add_owner(&host_ref);
    let style = Style::new();

    target.add(&style);

    assert!(style.owner() == Some(host_ref));
}

#[test]
fn removing_style_should_clear_owner() {
    let (_host, host_ref) = test_host();
    let target = Styles::new();
    target.add_owner(&host_ref);
    let style = Style::new();

    target.add(&style);
    assert!(target.remove(&style));

    assert!(style.owner().is_none());
    assert_eq!(target.count(), 0);
}

#[test]
fn styles_should_set_owner_on_assigned_resources() {
    let (_host, host_ref) = test_host();
    let target = Styles::new();
    target.add_owner(&host_ref);

    let resources = ResourceDictionary::new();
    target.set_resources(resources.clone());

    assert!(resources.owner() == Some(host_ref));
}

#[test]
fn styles_should_set_owner_on_assigned_resources_2() {
    let (_host, host_ref) = test_host();
    let target = Styles::new();

    let resources = ResourceDictionary::new();
    target.set_resources(resources.clone());

    target.add_owner(&host_ref);

    assert!(resources.owner() == Some(host_ref));
}

#[test]
fn styles_should_set_owner_on_child_style() {
    let (_host, host_ref) = test_host();
    let target = Styles::new();
    target.add_owner(&host_ref);

    let style = Style::new();
    target.add(&style);

    assert!(style.owner() == Some(host_ref));
}

#[test]
fn styles_should_set_owner_on_child_style_2() {
    let (_host, host_ref) = test_host();
    let target = Styles::new();

    let style = Style::new();
    target.add(&style);
    assert!(style.owner().is_none());

    target.add_owner(&host_ref);

    assert!(style.owner() == Some(host_ref));
}

#[test]
fn styles_finds_resource_in_merged_dictionary() {
    let target = Styles::new();
    let merged = ResourceDictionary::new();
    merged.add_value("foo", "bar".to_string());
    target.resources().add_merged_dictionary(&merged);

    let found = target.try_get_resource(&"foo".into(), None).flatten().unwrap();
    assert_eq!((*found).downcast_ref::<String>().unwrap(), "bar");
    assert!(target.has_resources() || merged.has_resources());
}

#[test]
fn styles_collection_operations() {
    let target = Styles::new();
    let a = Style::new();
    let b = Style::new();
    let changes = Rc::new(Cell::new(0));
    let c = changes.clone();
    target.collection_changed(move |_| c.set(c.get() + 1));

    target.add(&a);
    target.insert(0, &b);
    assert_eq!(target.count(), 2);
    assert_eq!(target.index_of(&a), Some(1));
    assert!(target.contains(&b));

    target.clear();
    assert_eq!(target.count(), 0);
    assert!(!target.contains(&a));
    assert_eq!(changes.get(), 3);
}

// --- ControlTheme ------------------------------------------------------------

#[test]
#[should_panic(expected = "ControlTheme (for Class1) cannot be added to a Styles collection.")]
fn control_theme_cannot_be_added_to_styles() {
    let target = ControlTheme::for_type::<Class1>();
    let styles = Styles::new();
    styles.add(target);
}

#[test]
#[should_panic(expected = "ControlThemes cannot be added as a nested style.")]
fn control_theme_cannot_be_added_to_style_children() {
    let target = ControlTheme::for_type::<Class1>();
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    style.children().add(target);
}

#[test]
#[should_panic(expected = "ControlThemes cannot be added as a nested style.")]
fn control_theme_cannot_be_added_to_control_theme_children() {
    let target = ControlTheme::for_type::<Class1>();
    let other = ControlTheme::for_type::<Class2>();
    other.children().add(target);
}

#[test]
#[should_panic(expected = "Child styles must have a selector.")]
fn style_without_selector_cannot_be_added_to_children() {
    let target = ControlTheme::for_type::<Class1>();
    target.children().add(Style::new());
}

#[test]
#[should_panic(expected = "Child styles must have a nesting selector.")]
fn style_without_nesting_selector_cannot_be_added_to_children() {
    let target = ControlTheme::for_type::<Class1>();
    target.children().add(Style::with_selector(Selectors::class(None, "foo")));
}

#[test]
#[should_panic(expected = "ControlTheme style may not directly contain a child or descendent selector.")]
fn style_with_non_template_child_selector_cannot_be_added_to_children() {
    let target = ControlTheme::for_type::<Class1>();
    target.children().add(Style::with_selector(Selectors::nesting(None).child().of_type::<Class1>()));
}

#[test]
#[should_panic(expected = "ControlTheme style may not directly contain a child or descendent selector.")]
fn style_with_non_template_descendent_selector_cannot_be_added_to_children() {
    let target = ControlTheme::for_type::<Class1>();
    target.children().add(Style::with_selector(Selectors::nesting(None).descendant().of_type::<Class1>()));
}

#[test]
#[should_panic(expected = "ControlTheme style may not directly contain a child or descendent selector.")]
fn style_with_non_template_child_template_selector_cannot_be_added_to_children() {
    let target = ControlTheme::for_type::<Class1>();
    target
        .children()
        .add(Style::with_selector(Selectors::nesting(None).child().template().of_type::<Class1>()));
}

#[test]
#[should_panic(expected = "ControlTemplate styles cannot contain multiple template selectors.")]
fn style_with_double_template_selector_cannot_be_added_to_children() {
    let target = ControlTheme::for_type::<Class1>();
    target.children().add(Style::with_selector(
        Selectors::nesting(None).template().of_type::<Class3>().template().of_type::<Class1>(),
    ));
}

#[test]
fn style_with_template_selector_can_be_added_to_control_theme_children() {
    let target = ControlTheme::for_type::<Class1>();
    target.children().add(Style::with_selector(Selectors::nesting(None).template().of_type::<Class3>()));
    assert_eq!(target.children().count(), 1);
    assert_eq!(target.to_display_string(), "Class1");
}

// --- Setter ------------------------------------------------------------------

#[test]
fn setter_should_apply_binding_to_property() {
    let subject = LightweightSubject::<BoxedValue>::new();
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    style.add_setter(Setter::new_binding(Class1::foo_property(), Rc::new(subject.clone())));
    let target = Class1::new();

    try_attach(&style, &target, None);
    assert_eq!(target.foo(), "foodefault");

    subject.on_next(boxed("foo".to_string()));
    assert_eq!(target.foo(), "foo");
}

#[test]
fn setter_should_handle_binding_producing_unset_value() {
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    style.add_setter(Setter::new_binding(
        Class1::foo_property(),
        Observable::single_value(FerroProperty::unset_value()),
    ));
    let target = Class1::new();

    try_attach(&style, &target, None);

    assert_eq!(target.foo(), "foodefault");
}

#[test]
fn can_set_direct_property_in_style_without_activator() {
    let target = Class1::new();
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    style.add_setter(Setter::new_untyped(Class1::direct_property(), boxed("foo".to_string())));

    try_attach(&style, &target, None);

    assert_eq!(target.direct(), "foo");
}

#[test]
fn can_set_direct_property_binding_in_style_without_activator() {
    let target = Class1::new();
    let (source, _) = source("foo");
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    style.add_setter(Setter::new_binding(Class1::direct_property(), source));

    try_attach(&style, &target, None);

    assert_eq!(target.direct(), "foo");
}

#[test]
#[should_panic(expected = "Cannot set direct property 'Direct' in 'Class1.foo' because the style has an activator.")]
fn cannot_set_direct_property_binding_in_style_with_activator() {
    let target = Class1::new();
    let (source, _) = source("foo");
    let style = Style::with_selector(Selectors::of_type::<Class1>().class("foo"));
    style.add_setter(Setter::new_binding(Class1::direct_property(), source));

    try_attach(&style, &target, None);
}

#[test]
#[should_panic(expected = "Cannot set direct property 'Direct' in 'Class1.foo' because the style has an activator.")]
fn cannot_set_direct_property_in_style_with_activator() {
    let target = Class1::new();
    let style = Style::with_selector(Selectors::of_type::<Class1>().class("foo"));
    style.add_setter(Setter::new_untyped(Class1::direct_property(), boxed("foo".to_string())));

    try_attach(&style, &target, None);
}

#[test]
fn setter_should_apply_value_without_activator_with_style_priority() {
    let style = foo_style(Selectors::of_type::<Class1>(), "Foo");
    let target = Class1::new();
    let priority = Rc::new(Cell::new(BindingPriority::Unset));
    let p = priority.clone();
    target.property_changed(move |e| p.set(e.priority()));

    try_attach(&style, &target, None);

    assert_eq!(target.foo(), "Foo");
    assert_eq!(priority.get(), BindingPriority::Style);
}

#[test]
fn setter_should_apply_value_with_activator_with_style_trigger_priority() {
    let style = foo_style(Selectors::of_type::<Class1>().class("foo"), "Foo");
    let target = Class1::new();
    target.classes().add("foo");
    let priority = Rc::new(Cell::new(BindingPriority::Unset));
    let p = priority.clone();
    target.property_changed(move |e| p.set(e.priority()));

    try_attach(&style, &target, None);

    assert_eq!(target.foo(), "Foo");
    assert_eq!(priority.get(), BindingPriority::StyleTrigger);
}

#[test]
fn setter_should_apply_binding_without_activator_with_style_priority() {
    let (source, _) = source("Foo");
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    style.add_setter(Setter::new_binding(Class1::foo_property(), source));
    let target = Class1::new();
    let priority = Rc::new(Cell::new(BindingPriority::Unset));
    let p = priority.clone();
    target.property_changed(move |e| p.set(e.priority()));

    try_attach(&style, &target, None);

    assert_eq!(target.foo(), "Foo");
    assert_eq!(priority.get(), BindingPriority::Style);
}

#[test]
fn setter_should_apply_binding_with_activator_with_style_trigger_priority() {
    let (source, _) = source("Foo");
    let style = Style::with_selector(Selectors::of_type::<Class1>().class("foo"));
    style.add_setter(Setter::new_binding(Class1::foo_property(), source));
    let target = Class1::new();
    target.classes().add("foo");
    let priority = Rc::new(Cell::new(BindingPriority::Unset));
    let p = priority.clone();
    target.property_changed(move |e| p.set(e.priority()));

    try_attach(&style, &target, None);

    assert_eq!(target.foo(), "Foo");
    assert_eq!(priority.get(), BindingPriority::StyleTrigger);
}

#[test]
fn non_active_styled_property_binding_should_be_unsubscribed() {
    let (source, subscriptions) = source("Foo");
    let style = Style::with_selector(Selectors::of_type::<Class1>().class("foo"));
    style.add_setter(Setter::new_binding(Class1::foo_property(), source));
    let target = Class1::new();

    try_attach(&style, &target, None);
    assert_eq!(subscriptions.get(), 0);

    target.classes().add("foo");
    assert_eq!(subscriptions.get(), 1);
    assert_eq!(target.foo(), "Foo");

    target.classes().remove("foo");
    assert_eq!(subscriptions.get(), 0);
    assert_eq!(target.foo(), "foodefault");
}

#[test]
#[should_panic(expected = "is not a valid value for property 'Foo'")]
fn setter_value_of_wrong_type_is_rejected() {
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    style.add_setter(Setter::new_untyped(Class1::foo_property(), boxed(1)));
    try_attach(&style, &Class1::new(), None);
}

/// A setter without a value sets null on a property whose type admits it
/// (a reference or nullable type in the managed original).
#[test]
fn setter_with_null_value_sets_null_on_a_nullable_property() {
    let target = Class1::new();
    target.set_value(Class1::child_property(), Some(Class1::new()));
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    let setter = Setter::empty();
    setter.set_property(Some(Class1::child_property().as_property()));
    style.add_setter(setter);

    try_attach(&style, &target, None);
    // The local value still wins; clearing it shows the style's null.
    target.clear_value(Class1::child_property());
    assert!(target.get_value(Class1::child_property()).is_none());
}

/// Null is not a value of a non-nullable value type.
#[test]
#[should_panic(expected = "Setter value '(null)' is not a valid value for property 'Double'")]
fn setter_with_null_value_is_rejected_for_a_value_type() {
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    let setter = Setter::empty();
    setter.set_property(Some(Class1::double_property().as_property()));
    style.add_setter(setter);
    try_attach(&style, &Class1::new(), None);
}

#[test]
#[should_panic(expected = "Setter.Property must be set.")]
fn setter_without_property_is_rejected() {
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    style.add_setter(Setter::empty());
    try_attach(&style, &Class1::new(), None);
}

#[test]
fn setter_display_string() {
    assert!(foo_setter("x").to_string().starts_with("Setter: Foo = "));
}

// --- ControlTheme application --------------------------------------------------

fn theme(value: &str) -> Ref<ControlTheme> {
    ControlTheme::with_setters(Class1::TYPE, [foo_setter(value)])
}

#[test]
fn theme_is_applied_when_attached_to_logical_tree() {
    let target = Class1::new();
    target.set_theme(theme("theme"));
    assert_eq!(target.foo(), "foodefault");

    let _root = TestRoot::with_child(&target);

    assert_eq!(target.foo(), "theme");
}

#[test]
fn theme_is_applied_to_derived_class_when_attached_to_logical_tree() {
    let target = Class2::new();
    target.set_theme(theme("theme"));

    let _root = TestRoot::with_child(&target);

    assert_eq!(target.foo(), "theme");
}

#[test]
fn theme_is_detached_when_theme_property_cleared() {
    let target = Class1::new();
    target.set_theme(theme("theme"));
    let _root = TestRoot::with_child(&target);
    assert_eq!(target.foo(), "theme");

    target.set_theme(None);

    assert_eq!(target.foo(), "foodefault");
}

#[test]
fn setting_explicit_theme_detaches_default_theme() {
    let target = Class1::new();
    let root = TestRoot::new();
    root.resources().add(Class1::TYPE, Some(boxed(theme("implicit"))));
    set_child(&root, &target);
    assert_eq!(target.foo(), "implicit");

    target.set_theme(theme("explicit"));
    target.apply_styling();

    assert_eq!(target.foo(), "explicit");
}

#[test]
fn unrelated_styles_are_not_detached_when_theme_property_cleared() {
    let target = Class1::new();
    target.set_theme(theme("theme"));
    let root = TestRoot::new();
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Class1>(),
        [Setter::new(Class1::double_property(), 4.0)],
    ));
    set_child(&root, &target);
    assert_eq!(target.double(), 4.0);

    target.set_theme(None);

    assert_eq!(target.double(), 4.0);
    assert_eq!(target.foo(), "foodefault");
}

#[test]
fn theme_is_applied_on_layout_after_theme_property_changes() {
    let target = Class1::new();
    let root = TestRoot::with_child(&target);
    assert_eq!(target.foo(), "foodefault");

    target.set_theme(theme("theme"));
    assert_eq!(target.foo(), "foodefault");

    root.measure(Size::INFINITY);
    assert_eq!(target.foo(), "theme");
}

#[test]
fn based_on_theme_is_applied_when_attached_to_logical_tree() {
    let base = ControlTheme::with_setters(Class1::TYPE, [Setter::new(Class1::double_property(), 7.0)]);
    let derived = theme("derived");
    derived.set_based_on(Some(base));
    let target = Class1::new();
    target.set_theme(derived);

    let _root = TestRoot::with_child(&target);

    assert_eq!(target.foo(), "derived");
    assert_eq!(target.double(), 7.0);
}

#[test]
fn theme_has_lower_priority_than_style() {
    let target = Class1::new();
    target.set_theme(theme("theme"));
    let root = TestRoot::new();
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "style"));

    set_child(&root, &target);

    assert_eq!(target.foo(), "style");
}

#[test]
fn theme_has_lower_priority_than_style_after_change() {
    let target = Class1::new();
    target.set_theme(theme("theme"));
    let root = TestRoot::new();
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "style"));
    set_child(&root, &target);

    target.set_theme(theme("other"));
    target.apply_styling();

    assert_eq!(target.foo(), "style");
}

#[test]
fn nested_style_of_theme_is_applied() {
    let control_theme = theme("theme");
    control_theme.children().add(Style::with_setters(
        Selectors::nesting(None).class("foo"),
        [foo_setter("nested")],
    ));
    let target = Class1::new();
    target.set_theme(control_theme);
    let _root = TestRoot::with_child(&target);
    assert_eq!(target.foo(), "theme");

    target.classes().add("foo");
    assert_eq!(target.foo(), "nested");
}

#[test]
fn implicit_theme_is_applied_when_attached_to_logical_tree() {
    let target = Class1::new();
    let root = TestRoot::new();
    root.resources().add(Class1::TYPE, Some(boxed(theme("implicit"))));

    set_child(&root, &target);

    assert_eq!(target.foo(), "implicit");
    assert!(target.get_effective_theme().is_some());
}

#[test]
fn implicit_theme_is_not_detached_when_removed_from_logical_tree() {
    let target = Class1::new();
    let root = TestRoot::new();
    root.resources().add(Class1::TYPE, Some(boxed(theme("implicit"))));
    set_child(&root, &target);

    remove_child(&root, &target);

    assert_eq!(target.foo(), "implicit");
}

#[test]
fn can_attach_then_reattach_to_same_logical_tree() {
    let target = Class1::new();
    let root = TestRoot::new();
    root.resources().add(Class1::TYPE, Some(boxed(theme("implicit"))));
    set_child(&root, &target);

    remove_child(&root, &target);
    set_child(&root, &target);

    assert_eq!(target.foo(), "implicit");
    assert_eq!(target.values().frames().len(), 1);
}

#[test]
fn implicit_theme_is_reevaluated_when_removed_and_added_to_different_logical_tree() {
    let target = Class1::new();
    let root1 = TestRoot::new();
    root1.resources().add(Class1::TYPE, Some(boxed(theme("first"))));
    let root2 = TestRoot::new();
    root2.resources().add(Class1::TYPE, Some(boxed(theme("second"))));

    set_child(&root1, &target);
    assert_eq!(target.foo(), "first");

    remove_child(&root1, &target);
    set_child(&root2, &target);

    assert_eq!(target.foo(), "second");
}

/// The implicit theme an element found is kept while the element stays in
/// its logical tree and is looked up again when the element enters a tree:
/// a theme that replaced it in the resources meanwhile is the one applied
/// then, and the first one is detached.
#[test]
fn implicit_theme_replaced_in_the_resources_is_applied_when_reattached_to_the_same_logical_tree() {
    let target = Class1::new();
    let root = TestRoot::new();
    root.resources().add(Class1::TYPE, Some(boxed(theme("first"))));
    set_child(&root, &target);
    assert_eq!(target.foo(), "first");

    root.resources().set(Class1::TYPE, Some(boxed(theme("second"))));
    assert_eq!(target.foo(), "first");

    remove_child(&root, &target);
    set_child(&root, &target);

    assert_eq!(target.foo(), "second");
    assert_eq!(target.values().frames().len(), 1);
}

/// An element that found no implicit theme looks for one again when it
/// enters a logical tree: a theme added to the resources meanwhile is found.
#[test]
fn implicit_theme_added_to_the_resources_is_applied_when_reattached() {
    let target = Class1::new();
    let root = TestRoot::new();
    set_child(&root, &target);
    assert_eq!(target.foo(), "foodefault");
    assert!(target.get_effective_theme().is_none());

    root.resources().add(Class1::TYPE, Some(boxed(theme("implicit"))));
    remove_child(&root, &target);
    set_child(&root, &target);

    assert_eq!(target.foo(), "implicit");
    assert!(target.get_effective_theme().is_some());
}

/// An implicit theme that was removed from the resources is detached when
/// the element enters a logical tree again.
#[test]
fn implicit_theme_removed_from_the_resources_is_detached_when_reattached() {
    let target = Class1::new();
    let root = TestRoot::new();
    root.resources().add(Class1::TYPE, Some(boxed(theme("implicit"))));
    set_child(&root, &target);
    assert_eq!(target.foo(), "implicit");

    assert!(root.resources().remove(&ResourceKey::Type(Class1::TYPE)));
    remove_child(&root, &target);
    assert_eq!(target.foo(), "implicit");
    set_child(&root, &target);

    assert_eq!(target.foo(), "foodefault");
    assert!(target.get_effective_theme().is_none());
    assert_eq!(target.values().frames().len(), 0);
}

/// The implicit theme is the one of the nearest host on the way up from
/// where the element is: an element moved to another parent of the same
/// logical tree gets the theme visible from there, and a theme added
/// afterwards to a host that had no resources is found from then on.
#[test]
fn implicit_theme_is_reevaluated_when_moved_between_hosts_of_the_same_logical_tree() {
    let root = TestRoot::new();
    root.resources().add(Class1::TYPE, Some(boxed(theme("root"))));
    let first = TestPanel::new();
    first.resources().add(Class1::TYPE, Some(boxed(theme("first"))));
    let second = TestPanel::new();
    set_child(&root, &first);
    set_child(&root, &second);
    let target = Class1::new();

    set_child(&first, &target);
    assert_eq!(target.foo(), "first");

    remove_child(&first, &target);
    set_child(&second, &target);
    assert_eq!(target.foo(), "root");

    second.resources().add(Class1::TYPE, Some(boxed(theme("second"))));
    remove_child(&second, &target);
    set_child(&second, &target);
    assert_eq!(target.foo(), "second");
    assert_eq!(target.values().frames().len(), 1);
}

#[test]
fn templated_parent_theme_is_applied_to_template_children() {
    let control_theme = ControlTheme::for_type::<Class1>();
    control_theme.add_setter(foo_setter("parent"));
    control_theme.children().add(Style::with_setters(
        Selectors::nesting(None).template().of_type::<Class3>(),
        [Setter::new(Visual::opacity_property(), 0.5)],
    ));

    let templated_parent = Class1::new();
    templated_parent.set_theme(control_theme);
    let part = Class3::new();
    part.set_templated_parent(templated_parent.clone().upcast::<FerroObject>());
    set_child(&templated_parent, &part);

    let _root = TestRoot::with_child(&templated_parent);

    assert_eq!(templated_parent.foo(), "parent");
    assert_eq!(part.opacity(), 0.5);

    // Clearing the theme of the templated parent detaches the theme from the
    // parts of its template.
    templated_parent.set_theme(None);
    part.on_templated_parent_control_theme_changed();
    assert_eq!(templated_parent.foo(), "foodefault");
    assert_eq!(part.opacity(), 1.0);
}

#[test]
fn primary_theme_is_not_detached_from_template_controls_when_templated_parent_theme_cleared() {
    let parent_theme = ControlTheme::for_type::<Class1>();
    parent_theme.children().add(Style::with_setters(
        Selectors::nesting(None).template().of_type::<Class3>(),
        [Setter::new(Visual::opacity_property(), 0.5)],
    ));
    let part_theme = ControlTheme::with_setters(Class3::TYPE, [Setter::new(Visual::z_index_property(), 3)]);

    let templated_parent = Class1::new();
    templated_parent.set_theme(parent_theme);
    let part = Class3::new();
    part.set_theme(part_theme);
    part.set_templated_parent(templated_parent.clone().upcast::<FerroObject>());
    set_child(&templated_parent, &part);
    let _root = TestRoot::with_child(&templated_parent);

    assert_eq!(part.opacity(), 0.5);
    assert_eq!(part.z_index(), 3);

    templated_parent.set_theme(None);
    part.on_templated_parent_control_theme_changed();

    assert_eq!(part.opacity(), 1.0);
    assert_eq!(part.z_index(), 3);

    // And the templated parent theme is kept when the primary theme is
    // cleared.
    templated_parent.set_theme(ControlTheme::for_type::<Class1>());
    part.set_theme(None);
    assert_eq!(part.z_index(), 0);
}

#[test]
fn theme_can_be_changed_by_style_class() {
    let target = Class1::new();
    let root = TestRoot::new();
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Class1>().class("bar"),
        [Setter::new(StyledElement::theme_property(), Some(theme("bar-theme")))],
    ));
    target.set_value_with_priority(StyledElement::theme_property(), Some(theme("base")), BindingPriority::Style);
    set_child(&root, &target);
    assert_eq!(target.foo(), "base");

    target.classes().add("bar");
    target.apply_styling();
    assert_eq!(target.foo(), "bar-theme");

    target.classes().remove("bar");
    target.apply_styling();
    assert_eq!(target.foo(), "base");
}

#[test]
#[should_panic(expected = "ControlTheme has no TargetType.")]
fn theme_without_target_type_is_rejected() {
    let target = Class1::new();
    let control_theme = ControlTheme::new();
    control_theme.add_setter(foo_setter("x"));
    target.set_theme(control_theme);
    let _root = TestRoot::with_child(&target);
}

#[test]
fn begin_init_defers_styling_until_end_init() {
    let root = TestRoot::new();
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "Foo"));
    let target = Class1::new();
    target.begin_init();
    set_child(&root, &target);
    assert_eq!(target.foo(), "foodefault");

    target.end_init();
    assert_eq!(target.foo(), "Foo");
}

#[test]
fn style_instances_use_the_frame_type_of_their_source() {
    let target = Class1::new();
    target.set_theme(theme("theme"));
    let root = TestRoot::new();
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "style"));
    set_child(&root, &target);

    let frames = target.values().frames();
    assert_eq!(frames.len(), 2);
    assert!(frames[0].base().frame_priority().is_type(FrameType::Theme));
    assert!(frames[1].base().frame_priority().is_type(FrameType::Style));
}

// --- lifetime ------------------------------------------------------------------

#[test]
fn styled_tree_is_released_when_dropped() {
    let root = TestRoot::new();
    let style = foo_style(Selectors::of_type::<Class1>().class("foo"), "Foo");
    style.resources().add_value("res", 1);
    root.styles().add(&style);
    root.styles().add(foo_style(Selectors::of_type::<Class1>(), "Plain"));
    root.styles().add(foo_style(Selectors::of_type::<TestPanel>().child().nth_child(2, 0), "Even"));
    root.styles().add(foo_style(
        Selectors::of_type::<Class1>().property_equals(Class1::double_property(), 1.0),
        "One",
    ));
    root.resources().add_value("foo", "bar".to_string());

    let panel = TestPanel::new();
    set_child(&root, &panel);
    let target = Class1::new();
    target.set_theme(theme("theme"));
    panel.add_child(&target);
    panel.add_child(&Class1::new());
    target.classes().add("foo");
    assert_eq!(target.foo(), "Foo");

    let weak_root = root.downgrade();
    let weak_panel = panel.downgrade();
    let weak_target = target.downgrade();
    let weak_style = style.downgrade();
    drop((root, panel, target, style));

    assert!(weak_root.upgrade().is_none());
    assert!(weak_panel.upgrade().is_none());
    assert!(weak_target.upgrade().is_none());
    assert!(weak_style.upgrade().is_none());
}

#[test]
fn chaining_class_extends_the_preceding_type_selector() {
    let selector = Selectors::of_type::<Class1>();
    let with_class = selector.clone().class("foo").name("bar");
    // A type selector followed by classes and a name is a single selector.
    assert_eq!(selector.to_string(), "Class1#bar.foo");
    assert_eq!(with_class.to_string(), "Class1#bar.foo");
}

/// The styling parent of an element is its inheritance parent when that is a
/// styled element (the same object), and nothing when the element has no
/// inheritance parent or one that is not a styled element.
#[test]
fn styling_parent_is_the_inheritance_parent_when_it_is_a_styled_element() {
    let target = Class1::new();
    assert!(target.styling_parent().is_none());

    let parent = Class3::new();
    target.set_inheritance_parent(&parent);
    let host = target.styling_parent().expect("a styling parent");
    assert!(host.as_element().is_some_and(|element| element.ptr_eq(&parent)));
    drop(host);

    let not_an_element = Style::new();
    target.set_inheritance_parent(&not_an_element);
    assert!(target.styling_parent().is_none());
    assert!(target.inheritance_parent().is_some_and(|p| p.ptr_eq(&not_an_element)));

    target.set_inheritance_parent(None);
    assert!(target.styling_parent().is_none());
}
