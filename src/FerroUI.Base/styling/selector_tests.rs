//! Tests for the selectors and their activators.
//!
//! Ported from the selector tests of the reference implementation, with the
//! classes of `test_support` in place of controls.

use super::activators::{IStyleActivator, IStyleActivatorSink};
use super::test_support::*;
use super::*;
use crate::*;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// Subscribes to an activator and tracks its state.
struct ActivatorSink {
    active: Cell<bool>,
}

impl IStyleActivatorSink for ActivatorSink {
    fn on_next(&self, value: bool) {
        self.active.set(value);
    }
}

impl ActivatorSink {
    fn new(match_: &SelectorMatch) -> (Rc<Self>, Rc<dyn IStyleActivator>) {
        let activator = match_.activator().expect("the match has an activator").clone();
        let sink = Rc::new(ActivatorSink { active: Cell::new(false) });
        let weak: Weak<dyn IStyleActivatorSink> = Rc::downgrade(&sink) as Weak<dyn IStyleActivatorSink>;
        activator.subscribe(weak);
        sink.active.set(activator.get_is_active());
        (sink, activator)
    }

    fn active(&self) -> bool {
        self.active.get()
    }
}

fn unsubscribe(sink: &Rc<ActivatorSink>, activator: &Rc<dyn IStyleActivator>) {
    let weak: Weak<dyn IStyleActivatorSink> = Rc::downgrade(sink) as Weak<dyn IStyleActivatorSink>;
    activator.unsubscribe(&weak);
}

fn element<T: ObjectType + Upcast<StyledElement>>(control: &Ref<T>) -> Ref<StyledElement> {
    control.clone().upcast()
}

// --- of type / is ------------------------------------------------------------

#[test]
fn of_type_matches_control_of_correct_type() {
    let control = Class1::new();
    let target = Selectors::of_type::<Class1>();
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::AlwaysThisType);
}

#[test]
fn of_type_doesnt_match_control_of_wrong_type() {
    let control = Class3::new();
    let target = Selectors::of_type::<Class1>();
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn of_type_class_doesnt_match_derived_class() {
    let control = Class2::new();
    let target = Selectors::of_type::<Class1>();
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn of_type_matches_control_with_style_key() {
    let control = StyledAsClass1::new();
    let target = Selectors::of_type::<Class1>();
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::AlwaysThisType);
}

#[test]
fn is_matches_derived_class() {
    let control = Class2::new();
    let target = Selectors::is::<Class1>();
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::AlwaysThisType);
}

#[test]
fn is_doesnt_match_unrelated_class() {
    let control = Class3::new();
    let target = Selectors::is::<Class1>();
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn of_type_and_is_selector_strings() {
    assert_eq!(Selectors::of_type::<Class1>().to_string(), "Class1");
    assert_eq!(Selectors::is::<Class1>().to_string(), ":is(Class1)");
    assert_eq!(Selectors::of_type::<Class1>().target_type(), Some(Class1::TYPE));
}

// --- name --------------------------------------------------------------------

#[test]
fn name_matches_control_with_correct_name() {
    let control = Class1::new();
    control.set_name(Some("foo".to_string()));
    let target = Selectors::of_type::<Class1>().name("foo");
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::AlwaysThisInstance);
}

#[test]
fn name_doesnt_match_control_of_wrong_name() {
    let control = Class1::new();
    control.set_name(Some("foo".to_string()));
    let target = Selectors::of_type::<Class1>().name("bar");
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisInstance);
}

#[test]
fn name_doesnt_match_control_without_name() {
    let control = Class1::new();
    let target = Selectors::name(None, "foo");
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisInstance);
}

#[test]
fn name_has_correct_immutable_string() {
    let target = Selectors::of_type::<Class1>().name("foo");
    assert_eq!(target.to_string(), "Class1#foo");
}

#[test]
#[should_panic(expected = "Name may not be empty")]
fn empty_name_is_rejected() {
    Selectors::name(None, " ");
}

// --- class -------------------------------------------------------------------

#[test]
fn class_selector_strings() {
    assert_eq!(Selectors::class(None, "foo").to_string(), ".foo");
    assert_eq!(Selectors::class(None, ":foo").to_string(), ":foo");
    assert_eq!(Selectors::of_type::<Class1>().class("foo").class(":bar").to_string(), "Class1.foo:bar");
}

#[test]
fn class_matches_control_with_class() {
    let control = Class1::new();
    control.classes().add("foo");
    let target = Selectors::class(None, "foo");
    let match_ = target.match_(&element(&control), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);
    assert!(match_.activator().unwrap().get_is_active());
}

#[test]
fn class_doesnt_match_control_without_class() {
    let control = Class1::new();
    control.classes().add("bar");
    let target = Selectors::class(None, "foo");
    let match_ = target.match_(&element(&control), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);
    assert!(!match_.activator().unwrap().get_is_active());
}

#[test]
fn class_matches_control_with_templated_parent() {
    let control = Class1::new();
    control.set_templated_parent(Class1::new().upcast::<FerroObject>());
    control.classes().add("foo");
    let target = Selectors::class(None, "foo");
    let match_ = target.match_(&element(&control), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);
    assert!(match_.activator().unwrap().get_is_active());
}

#[test]
fn class_tracks_additions() {
    let control = Class1::new();
    let target = Selectors::class(None, "foo");
    let (sink, _activator) = ActivatorSink::new(&target.match_(&element(&control), None, true));
    assert!(!sink.active());
    control.classes().add("foo");
    assert!(sink.active());
}

#[test]
fn class_tracks_removals() {
    let control = Class1::new();
    control.classes().add("foo");
    let target = Selectors::class(None, "foo");
    let (sink, _activator) = ActivatorSink::new(&target.match_(&element(&control), None, true));
    assert!(sink.active());
    control.classes().remove("foo");
    assert!(!sink.active());
}

#[test]
fn multiple_classes() {
    let control = Class1::new();
    let target = Selectors::class(None, "foo").class("bar");
    let (sink, _activator) = ActivatorSink::new(&target.match_(&element(&control), None, true));
    assert!(!sink.active());
    control.classes().add("foo");
    assert!(!sink.active());
    control.classes().add("bar");
    assert!(sink.active());
    control.classes().remove("bar");
    assert!(!sink.active());
}

#[test]
fn class_match_without_subscription_is_evaluated_immediately() {
    let control = Class1::new();
    let target = Selectors::class(None, "foo");
    assert_eq!(target.match_(&element(&control), None, false).result(), SelectorMatchResult::NeverThisInstance);
    control.classes().add("foo");
    assert_eq!(target.match_(&element(&control), None, false).result(), SelectorMatchResult::AlwaysThisType);
}

#[test]
fn only_notifies_when_result_changes() {
    let control = Class1::new();
    let target = Selectors::class(None, "foo");
    let match_ = target.match_(&element(&control), None, true);
    let activator = match_.activator().unwrap().clone();

    struct Counter(Cell<u32>);
    impl IStyleActivatorSink for Counter {
        fn on_next(&self, _value: bool) {
            self.0.set(self.0.get() + 1);
        }
    }
    let sink = Rc::new(Counter(Cell::new(0)));
    activator.subscribe(Rc::downgrade(&sink) as Weak<dyn IStyleActivatorSink>);
    assert!(!activator.get_is_active());

    control.classes().add("foo");
    control.classes().add("bar");
    control.classes().remove("foo");

    assert_eq!(sink.0.get(), 2);
}

#[test]
fn unsubscribing_removes_class_listener() {
    let control = Class1::new();
    let target = Selectors::class(None, "foo");
    let (sink, activator) = ActivatorSink::new(&target.match_(&element(&control), None, true));
    assert_eq!(control.classes().listener_count(), 1);
    unsubscribe(&sink, &activator);
    assert_eq!(control.classes().listener_count(), 0);
}

// --- child -------------------------------------------------------------------

#[test]
fn child_matches_control_when_it_is_child_of_type() {
    let parent = Class1::new();
    let child = Class3::new();
    set_child(&parent, &child);
    let selector = Selectors::of_type::<Class1>().child().of_type::<Class3>();
    assert_eq!(selector.match_(&element(&child), None, true).result(), SelectorMatchResult::AlwaysThisInstance);
}

#[test]
fn child_doesnt_match_control_when_it_is_grandchild_of_type() {
    let grandparent = Class1::new();
    let parent = Class3::new();
    let child = Class3::new();
    set_child(&grandparent, &parent);
    set_child(&parent, &child);
    let selector = Selectors::of_type::<Class1>().child().of_type::<Class3>();
    assert_eq!(selector.match_(&element(&child), None, true).result(), SelectorMatchResult::NeverThisInstance);
}

#[test]
fn child_matches_control_when_it_is_child_of_type_and_class() {
    let parent = Class1::new();
    let child = Class3::new();
    set_child(&parent, &child);
    let selector = Selectors::of_type::<Class1>().class("foo").child().of_type::<Class3>();
    let match_ = selector.match_(&element(&child), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);
    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(!sink.active());
    parent.classes().add("foo");
    assert!(sink.active());
    parent.classes().remove("foo");
    assert!(!sink.active());
}

#[test]
fn child_doesnt_match_control_without_parent() {
    let control = Class3::new();
    let selector = Selectors::of_type::<Class1>().child().of_type::<Class3>();
    assert_eq!(selector.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisInstance);
}

#[test]
fn child_selector_should_have_correct_string_representation() {
    let selector = Selectors::of_type::<Class1>().child().of_type::<Class3>();
    assert_eq!(selector.to_string(), "Class1 > Class3");
}

#[test]
#[should_panic(expected = "Child selector must be preceeded by a selector.")]
fn child_selector_requires_previous_selector() {
    Selectors::child(None);
}

// --- descendant --------------------------------------------------------------

#[test]
fn descendant_matches_control_when_it_is_child_of_type() {
    let parent = Class1::new();
    let child = Class3::new();
    set_child(&parent, &child);
    let selector = Selectors::of_type::<Class1>().descendant().of_type::<Class3>();
    assert_eq!(selector.match_(&element(&child), None, true).result(), SelectorMatchResult::AlwaysThisInstance);
}

#[test]
fn descendant_matches_control_when_it_is_descendant_of_type() {
    let grandparent = Class1::new();
    let parent = Class3::new();
    let child = Class3::new();
    set_child(&grandparent, &parent);
    set_child(&parent, &child);
    let selector = Selectors::of_type::<Class1>().descendant().of_type::<Class3>();
    assert_eq!(selector.match_(&element(&child), None, true).result(), SelectorMatchResult::AlwaysThisInstance);
}

#[test]
fn descendant_matches_control_when_it_is_descendant_of_type_and_class() {
    let grandparent = Class1::new();
    let parent = Class2::new();
    let child = Class3::new();
    set_child(&grandparent, &parent);
    set_child(&parent, &child);
    let selector = Selectors::of_type::<Class1>().class("foo").descendant().of_type::<Class3>();
    let match_ = selector.match_(&element(&child), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);
    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(!sink.active());
    grandparent.classes().add("foo");
    assert!(sink.active());
    grandparent.classes().remove("foo");
    assert!(!sink.active());
}

#[test]
fn descendant_matches_any_ancestor() {
    let grandparent = Class1::new();
    let parent = Class1::new();
    let child = Class3::new();
    set_child(&grandparent, &parent);
    set_child(&parent, &child);
    let selector = Selectors::of_type::<Class1>().class("foo").descendant().of_type::<Class3>();
    let (sink, _activator) = ActivatorSink::new(&selector.match_(&element(&child), None, true));
    assert!(!sink.active());
    parent.classes().add("foo");
    assert!(sink.active());
    grandparent.classes().add("foo");
    assert!(sink.active());
    parent.classes().remove("foo");
    assert!(sink.active());
    grandparent.classes().remove("foo");
    assert!(!sink.active());
}

#[test]
fn descendant_selector_should_have_correct_string_representation() {
    let selector = Selectors::of_type::<Class1>().descendant().of_type::<Class3>();
    assert_eq!(selector.to_string(), "Class1 Class3");
}

// --- template ----------------------------------------------------------------

fn templated_child(templated_parent: &Ref<Class1>) -> Ref<Class3> {
    let child = Class3::new();
    child.set_templated_parent(templated_parent.clone().upcast::<FerroObject>());
    set_child(templated_parent, &child);
    child
}

#[test]
fn control_in_template_is_matched_with_template_selector() {
    let templated_parent = Class1::new();
    let child = templated_child(&templated_parent);
    let selector = Selectors::of_type::<Class1>().template().of_type::<Class3>();
    assert_eq!(selector.match_(&element(&child), None, true).result(), SelectorMatchResult::AlwaysThisInstance);
}

#[test]
fn nested_control_in_template_is_matched_with_template_selector() {
    let templated_parent = Class1::new();
    let child = templated_child(&templated_parent);
    let grandchild = Class2::new();
    grandchild.set_templated_parent(templated_parent.clone().upcast::<FerroObject>());
    set_child(&child, &grandchild);
    let selector = Selectors::of_type::<Class1>().template().of_type::<Class2>();
    assert_eq!(
        selector.match_(&element(&grandchild), None, true).result(),
        SelectorMatchResult::AlwaysThisInstance
    );
}

#[test]
fn control_in_template_is_matched_with_typeof_templated_control_class() {
    let templated_parent = Class1::new();
    let child = templated_child(&templated_parent);
    let selector = Selectors::of_type::<Class1>().class("foo").template().of_type::<Class3>();
    let match_ = selector.match_(&element(&child), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);
    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(!sink.active());
    templated_parent.classes().add("foo");
    assert!(sink.active());
}

#[test]
fn control_without_templated_parent_is_not_matched_by_template_selector() {
    let control = Class3::new();
    let selector = Selectors::of_type::<Class1>().template().of_type::<Class3>();
    assert_eq!(selector.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisInstance);
}

#[test]
fn template_selector_should_have_correct_string_representation() {
    let selector = Selectors::of_type::<Class1>().template().of_type::<Class3>();
    assert_eq!(selector.to_string(), "Class1 /template/ Class3");
}

// --- not ---------------------------------------------------------------------

#[test]
fn not_selector_should_have_correct_string_representation() {
    let target = Selectors::not_with(None, |x| Selectors::class(x, "foo"));
    assert_eq!(target.to_string(), ":not(.foo)");
}

#[test]
fn not_of_type_matches_control_of_incorrect_type() {
    let control = Class1::new();
    let target = Selectors::not_with(None, |_| Selectors::of_type::<Class3>());
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::AlwaysThisType);
}

#[test]
fn not_of_type_doesnt_match_control_of_correct_type() {
    let control = Class1::new();
    let target = Selectors::not_with(None, |_| Selectors::of_type::<Class1>());
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn not_class_doesnt_match_control_with_class() {
    let control = Class1::new();
    control.classes().add("foo");
    let target = Selectors::not_with(None, |x| Selectors::class(x, "foo"));
    let match_ = target.match_(&element(&control), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);
    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(!sink.active());
}

#[test]
fn not_class_matches_control_without_class() {
    let control = Class1::new();
    control.classes().add("bar");
    let target = Selectors::not_with(None, |x| Selectors::class(x, "foo"));
    let match_ = target.match_(&element(&control), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);
    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(sink.active());
    control.classes().add("foo");
    assert!(!sink.active());
}

#[test]
fn of_type_not_class_matches_control_without_class() {
    let control = Class1::new();
    control.classes().add("bar");
    let target = Selectors::of_type::<Class1>().not_with(|x| Selectors::class(x, "foo"));
    let match_ = target.match_(&element(&control), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);
    assert!(match_.activator().unwrap().get_is_active());
    assert_eq!(target.to_string(), "Class1:not(.foo)");
}

#[test]
fn of_type_not_class_doesnt_match_control_of_wrong_type() {
    let control = Class3::new();
    control.classes().add("bar");
    let target = Selectors::of_type::<Class1>().not_with(|x| Selectors::class(x, "foo"));
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisType);
}

// --- or ----------------------------------------------------------------------

#[test]
fn or_selector_should_have_correct_string_representation() {
    let target = Selectors::or([Selectors::of_type::<Class1>().class("foo"), Selectors::of_type::<Class3>().class("bar")]);
    assert_eq!(target.to_string(), "Class1.foo, Class3.bar");
}

#[test]
fn or_selector_matches_control_of_correct_type() {
    let target = Selectors::or([Selectors::of_type::<Class1>(), Selectors::of_type::<Class3>().class("bar")]);
    let control = Class1::new();
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::AlwaysThisType);
}

#[test]
fn or_selector_matches_control_of_correct_type_with_class() {
    let target = Selectors::or([Selectors::of_type::<Class1>(), Selectors::of_type::<Class3>().class("bar")]);
    let control = Class3::new();
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::Sometimes);
}

#[test]
fn or_selector_doesnt_match_control_of_incorrect_type() {
    let target = Selectors::or([Selectors::of_type::<Class1>(), Selectors::of_type::<Class3>().class("bar")]);
    let control = TestRoot::new();
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn or_selector_matches_control_with_correct_name() {
    let target =
        Selectors::or([Selectors::of_type::<Class1>().name("foo"), Selectors::of_type::<Class3>().name("foo")]);
    let control = Class1::new();
    control.set_name(Some("foo".to_string()));
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::AlwaysThisInstance);
}

#[test]
fn or_selector_doesnt_match_control_with_incorrect_name() {
    let target =
        Selectors::or([Selectors::of_type::<Class1>().name("foo"), Selectors::of_type::<Class3>().name("foo")]);
    let control = Class1::new();
    control.set_name(Some("bar".to_string()));
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisInstance);
}

#[test]
fn or_selector_returns_correct_target_type_when_types_same() {
    let target = Selectors::or([Selectors::of_type::<Class1>().class("foo"), Selectors::of_type::<Class1>().class("bar")]);
    assert_eq!(target.target_type(), Some(Class1::TYPE));
}

#[test]
fn or_selector_returns_common_target_type() {
    let target = Selectors::or([Selectors::of_type::<Class1>().class("foo"), Selectors::of_type::<Class2>().class("bar")]);
    assert_eq!(target.target_type(), Some(Class1::TYPE));
    let target = Selectors::or([Selectors::of_type::<Class1>(), Selectors::of_type::<Class3>()]);
    assert_eq!(target.target_type(), Some(layout::Layoutable::TYPE));
}

#[test]
fn or_selector_returns_null_target_type_when_a_type_is_missing() {
    let target = Selectors::or([Selectors::of_type::<Class1>(), Selectors::class(None, "bar")]);
    assert_eq!(target.target_type(), None);
}

#[test]
#[should_panic(expected = "Need more than one selector to OR.")]
fn or_selector_requires_two_selectors() {
    Selectors::or([Selectors::of_type::<Class1>()]);
}

// --- property equals ---------------------------------------------------------

#[test]
fn property_equals_matches_when_property_has_matching_value() {
    let control = Class1::new();
    let target = Selectors::property_equals(None, Class1::foo_property(), "foo".to_string());
    let (sink, _activator) = ActivatorSink::new(&target.match_(&element(&control), None, true));
    assert!(!sink.active());
    control.set_foo("foo");
    assert!(sink.active());
    control.set_foo("bar");
    assert!(!sink.active());
}

#[test]
fn of_type_property_equals_doesnt_match_control_of_wrong_type() {
    let control = Class3::new();
    let target = Selectors::of_type::<Class1>().property_equals(Class1::foo_property(), "foo".to_string());
    assert_eq!(target.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn property_equals_without_subscription_compares_the_current_value() {
    let control = Class1::new();
    let target = Selectors::property_equals(None, Class1::double_property(), 5.0);
    assert_eq!(target.match_(&element(&control), None, false).result(), SelectorMatchResult::NeverThisInstance);
    control.set_value(Class1::double_property(), 5.0);
    assert_eq!(target.match_(&element(&control), None, false).result(), SelectorMatchResult::AlwaysThisInstance);
}

#[test]
fn property_equals_selector_should_have_correct_string_representation() {
    let target = Selectors::of_type::<Class1>().property_equals(Class1::foo_property(), "foo".to_string());
    assert_eq!(target.to_string(), "Class1[Foo=foo]");
    let target = Selectors::of_type::<Class1>().property_equals(Visual::flow_direction_property(), media::FlowDirection::RightToLeft);
    assert!(target.to_string().starts_with("Class1[(Visual.FlowDirection)="));
}

#[test]
fn property_equals_compares_untyped_property_values_by_content() {
    let value: BoxedValue = Rc::new("foo".to_string());
    let property_value: BoxedValue = Rc::new(Some(value.clone()));
    assert!(PropertyEqualsSelector::compare(&property_value, &value));
    assert!(PropertyEqualsSelector::compare(&value, &value));
    assert!(!PropertyEqualsSelector::compare(&boxed(1), &value));
    assert!(!PropertyEqualsSelector::compare(&boxed(None::<BoxedValue>), &value));
}

// --- multiple ----------------------------------------------------------------

#[test]
fn named_template_child_of_control_with_two_classes() {
    let templated_parent = Class1::new();
    let child = templated_child(&templated_parent);
    child.set_name(Some("baz".to_string()));

    let selector =
        Selectors::of_type::<Class1>().class("foo").class("bar").template().of_type::<Class3>().name("baz");
    let match_ = selector.match_(&element(&child), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);

    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(!sink.active());
    templated_parent.classes().add("foo");
    assert!(!sink.active());
    templated_parent.classes().add("bar");
    assert!(sink.active());
    templated_parent.classes().remove("foo");
    assert!(!sink.active());
}

#[test]
fn named_class_template_child_of_control() {
    let templated_parent = Class1::new();
    let child = templated_child(&templated_parent);
    child.set_name(Some("baz".to_string()));

    let selector = Selectors::of_type::<Class1>().template().of_type::<Class3>().name("baz").class("foo");
    let match_ = selector.match_(&element(&child), None, true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);

    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(!sink.active());
    child.classes().add("foo");
    assert!(sink.active());
    child.classes().remove("foo");
    assert!(!sink.active());
}

#[test]
fn type_child_without_matching_parent_never_matches_type() {
    let control = Class1::new();
    let selector = Selectors::of_type::<Class3>().child().of_type::<Class2>();
    // The control type does not match: the combinator is not evaluated.
    assert_eq!(selector.match_(&element(&control), None, true).result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn target_type_of_selectors() {
    assert_eq!(Selectors::of_type::<Class1>().class("foo").target_type(), Some(Class1::TYPE));
    assert_eq!(Selectors::of_type::<Class1>().child().target_type(), None);
    assert_eq!(Selectors::of_type::<Class1>().child().of_type::<Class3>().class("x").target_type(), Some(Class3::TYPE));
    assert_eq!(Selectors::of_type::<Class1>().template().of_type::<Class3>().target_type(), Some(Class3::TYPE));
    assert_eq!(Selectors::of_type::<Class1>().nth_child(2, 0).target_type(), Some(Class1::TYPE));
}

// --- nth-child ---------------------------------------------------------------

fn panel_with_children(count: usize) -> (Ref<TestPanel>, Vec<Ref<Class1>>) {
    let panel = TestPanel::new();
    let children: Vec<_> = (0..count).map(|_| Class1::new()).collect();
    for child in &children {
        panel.add_child(child);
    }
    (panel, children)
}

fn nth_matches(selector: &Selector, children: &[Ref<Class1>]) -> Vec<bool> {
    children
        .iter()
        .map(|c| {
            let match_ = selector.match_(&element(c), None, true);
            assert_eq!(match_.result(), SelectorMatchResult::Sometimes);
            match_.activator().unwrap().get_is_active()
        })
        .collect()
}

#[test]
fn nth_child_match_control_in_panel() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_child(None, 2, 0);
    assert_eq!(nth_matches(&target, &children), vec![false, true, false, true]);
}

#[test]
fn nth_child_match_control_in_panel_with_offset() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_child(None, 2, 1);
    assert_eq!(nth_matches(&target, &children), vec![true, false, true, false]);
}

#[test]
fn nth_child_match_control_in_panel_with_negative_offset() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_child(None, 4, -1);
    assert_eq!(nth_matches(&target, &children), vec![false, false, true, false]);
}

#[test]
fn nth_child_match_control_in_panel_with_singular_step() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_child(None, 1, 2);
    assert_eq!(nth_matches(&target, &children), vec![false, true, true, true]);
}

#[test]
fn nth_child_match_control_in_panel_with_singular_step_with_negative_offset() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_child(None, 1, -1);
    assert_eq!(nth_matches(&target, &children), vec![true, true, true, true]);
}

#[test]
fn nth_child_match_control_in_panel_with_zero_step_with_offset() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_child(None, 0, 2);
    assert_eq!(nth_matches(&target, &children), vec![false, true, false, false]);
}

#[test]
fn nth_child_doesnt_match_control_in_panel_with_zero_step_with_negative_offset() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_child(None, 0, -2);
    assert_eq!(nth_matches(&target, &children), vec![false, false, false, false]);
}

#[test]
fn nth_child_match_control_in_panel_with_negative_step_with_offset() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_child(None, -1, 2);
    assert_eq!(nth_matches(&target, &children), vec![true, true, false, false]);
}

#[test]
fn nth_child_match_control_with_previous_selector() {
    let (_panel, children) = panel_with_children(4);
    let other = Class3::new();
    _panel.insert_child(0, &other);
    let target = Selectors::of_type::<Class1>().nth_child(2, 0);
    // children are now at 1-based positions 2..=5.
    assert_eq!(nth_matches(&target, &children), vec![true, false, true, false]);
    assert_eq!(target.match_(&element(&other), None, true).result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn nth_child_doesnt_match_control_out_of_panel_parent() {
    let parent = Class3::new();
    let child = Class1::new();
    set_child(&parent, &child);
    let target = Selectors::nth_child(None, 1, 0);
    assert_eq!(target.match_(&element(&child), None, true).result(), SelectorMatchResult::NeverThisInstance);
}

#[test]
fn nth_child_without_subscription_is_evaluated_immediately() {
    let (_panel, children) = panel_with_children(2);
    let target = Selectors::nth_child(None, 2, 0);
    assert_eq!(target.match_(&element(&children[0]), None, false).result(), SelectorMatchResult::NeverThisInstance);
    assert_eq!(target.match_(&element(&children[1]), None, false).result(), SelectorMatchResult::AlwaysThisInstance);
}

#[test]
fn nth_child_tracks_index_changes() {
    let (panel, children) = panel_with_children(2);
    let target = Selectors::nth_child(None, 2, 0);
    let (sink, activator) = ActivatorSink::new(&target.match_(&element(&children[1]), None, true));
    assert!(sink.active());
    assert_eq!(panel.listener_count(), 1);

    panel.insert_child(0, &Class3::new());
    assert!(!sink.active());

    panel.insert_child(0, &Class3::new());
    assert!(sink.active());

    unsubscribe(&sink, &activator);
    assert_eq!(panel.listener_count(), 0);
}

#[test]
fn nth_child_selector_should_have_correct_string_representation() {
    let cases = [
        (1, 0, ":nth-child(1n)"),
        (1, 1, ":nth-child(1n+1)"),
        (2, 0, ":nth-child(2n)"),
        (2, 1, ":nth-child(2n+1)"),
        (2, -1, ":nth-child(2n-1)"),
        (0, 1, ":nth-child(1)"),
        (4, -3, ":nth-child(4n-3)"),
        (-1, 2, ":nth-child(-1n+2)"),
    ];
    for (step, offset, expected) in cases {
        assert_eq!(Selectors::nth_child(None, step, offset).to_string(), expected);
    }
    assert_eq!(Selectors::of_type::<Class1>().nth_child(2, 1).to_string(), "Class1:nth-child(2n+1)");
}

// --- nth-last-child ----------------------------------------------------------

#[test]
fn nth_last_child_match_control_in_panel() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_last_child(None, 2, 0);
    assert_eq!(nth_matches(&target, &children), vec![true, false, true, false]);
}

#[test]
fn nth_last_child_match_control_in_panel_with_offset() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_last_child(None, 2, 1);
    assert_eq!(nth_matches(&target, &children), vec![false, true, false, true]);
}

#[test]
fn nth_last_child_match_control_in_panel_with_negative_offset() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_last_child(None, 4, -1);
    assert_eq!(nth_matches(&target, &children), vec![false, true, false, false]);
}

#[test]
fn nth_last_child_match_last_control() {
    let (_panel, children) = panel_with_children(4);
    let target = Selectors::nth_last_child(None, 0, 1);
    assert_eq!(nth_matches(&target, &children), vec![false, false, false, true]);
}

#[test]
fn nth_last_child_tracks_total_count() {
    let (panel, children) = panel_with_children(2);
    let target = Selectors::nth_last_child(None, 0, 1);
    let (sink, _activator) = ActivatorSink::new(&target.match_(&element(&children[1]), None, true));
    assert!(sink.active());
    panel.add_child(&Class3::new());
    assert!(!sink.active());
}

#[test]
fn nth_last_child_selector_should_have_correct_string_representation() {
    assert_eq!(Selectors::nth_last_child(None, 2, 1).to_string(), ":nth-last-child(2n+1)");
    assert_eq!(Selectors::of_type::<Class1>().nth_last_child(0, 1).to_string(), "Class1:nth-last-child(1)");
}

// --- nesting -----------------------------------------------------------------

fn nested_style(parent: &Ref<Style>, selector: Selector) -> Ref<Style> {
    let nested = Style::with_selector(selector);
    parent.children().add(&nested);
    nested
}

#[test]
fn nesting_class_doesnt_match_parent_of_type_selector() {
    let control = Class3::new();
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let nested = nested_style(&parent, Selectors::nesting(None).class("foo"));
    let match_ = nested.selector().unwrap().match_(&element(&control), Some(&parent), true);
    assert_eq!(match_.result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn or_nesting_class_doesnt_match_parent_of_type_selector() {
    let control = Class3::new();
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let nested = nested_style(
        &parent,
        Selectors::or([Selectors::nesting(None).class("foo"), Selectors::nesting(None).class("bar")]),
    );
    let match_ = nested.selector().unwrap().match_(&element(&control), Some(&parent), true);
    assert_eq!(match_.result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn or_nesting_child_of_type_doesnt_match_parent_of_type_selector() {
    let control = Class1::new();
    let panel = Class3::new();
    set_child(&panel, &control);
    let parent = Style::with_selector(Selectors::of_type::<TestPanel>());
    let nested = nested_style(
        &parent,
        Selectors::or([
            Selectors::nesting(None).child().of_type::<Class1>(),
            Selectors::nesting(None).child().of_type::<Class1>(),
        ]),
    );
    let match_ = nested.selector().unwrap().match_(&element(&control), Some(&parent), true);
    assert_eq!(match_.result(), SelectorMatchResult::NeverThisInstance);
}

#[test]
fn double_nesting_class_doesnt_match_grandparent_of_type_selector() {
    let control = Class3::new();
    control.classes().add_range(["foo", "bar"]);
    let grandparent = Style::with_selector(Selectors::of_type::<Class1>());
    let parent = nested_style(&grandparent, Selectors::nesting(None).class("foo"));
    let nested = nested_style(&parent, Selectors::nesting(None).class("bar"));
    let match_ = nested.selector().unwrap().match_(&element(&control), Some(&parent), true);
    assert_eq!(match_.result(), SelectorMatchResult::NeverThisType);
}

#[test]
fn nesting_class_matches() {
    let control = Class1::new();
    control.classes().add("foo");
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let nested = nested_style(&parent, Selectors::nesting(None).class("foo"));
    let match_ = nested.selector().unwrap().match_(&element(&control), Some(&parent), true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);

    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(sink.active());
    control.classes().clear();
    assert!(!sink.active());
}

#[test]
fn double_nesting_class_matches() {
    let control = Class1::new();
    control.classes().add_range(["foo", "bar"]);
    let grandparent = Style::with_selector(Selectors::of_type::<Class1>());
    let parent = nested_style(&grandparent, Selectors::nesting(None).class("foo"));
    let nested = nested_style(&parent, Selectors::nesting(None).class("bar"));
    let match_ = nested.selector().unwrap().match_(&element(&control), Some(&parent), true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);

    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(sink.active());
    control.classes().remove("foo");
    assert!(!sink.active());
}

#[test]
fn or_nesting_class_matches() {
    let control = Class1::new();
    control.classes().add("foo");
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let nested = nested_style(
        &parent,
        Selectors::or([Selectors::nesting(None).class("foo"), Selectors::nesting(None).class("bar")]),
    );
    let match_ = nested.selector().unwrap().match_(&element(&control), Some(&parent), true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);

    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(sink.active());
    control.classes().clear();
    assert!(!sink.active());
}

#[test]
fn or_nesting_child_of_type_matches() {
    let control = Class1::new();
    control.classes().add("foo");
    let panel = TestPanel::new();
    panel.add_child(&control);
    let parent = Style::with_selector(Selectors::of_type::<TestPanel>());
    let nested = nested_style(
        &parent,
        Selectors::or([
            Selectors::nesting(None).child().of_type::<Class1>(),
            Selectors::nesting(None).child().of_type::<Class1>(),
        ]),
    );
    let match_ = nested.selector().unwrap().match_(&element(&control), Some(&parent), true);
    assert_eq!(match_.result(), SelectorMatchResult::AlwaysThisInstance);
}

#[test]
#[should_panic(expected = "Nesting selector was specified but cannot determine parent selector.")]
fn nesting_with_no_parent_style_fails() {
    let control = Class1::new();
    let style = Style::with_selector(Selectors::nesting(None).of_type::<Class1>());
    style.selector().unwrap().match_(&element(&control), None, true);
}

#[test]
#[should_panic(expected = "Nesting selector was specified but cannot determine parent selector.")]
fn nesting_with_no_parent_selector_fails() {
    let control = Class1::new();
    let parent = Style::new();
    let nested = nested_style(&parent, Selectors::nesting(None).class("foo"));
    nested.selector().unwrap().match_(&element(&control), Some(&parent), true);
}

#[test]
#[should_panic(expected = "Child styles must have a nesting selector.")]
fn adding_child_with_no_nesting_selector_fails() {
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let child = Style::with_selector(Selectors::class(None, "foo"));
    parent.children().add(&child);
}

#[test]
#[should_panic(expected = "Child styles must have a nesting selector.")]
fn adding_combinator_selector_child_with_no_nesting_selector_fails() {
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let child = Style::with_selector(Selectors::class(None, "foo").descendant().class("bar"));
    parent.children().add(&child);
}

#[test]
#[should_panic(expected = "Child styles must have a nesting selector.")]
fn adding_or_selector_child_with_no_nesting_selector_fails() {
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let child =
        Style::with_selector(Selectors::or([Selectors::nesting(None).class("foo"), Selectors::class(None, "bar")]));
    parent.children().add(&child);
}

#[test]
fn can_add_child_without_nesting_selector_to_style_without_selector() {
    let parent = Style::new();
    let child = Style::with_selector(Selectors::class(None, "foo"));
    parent.children().add(&child);
    assert_eq!(parent.children().count(), 1);
}

#[test]
fn nesting_not_class_matches() {
    let control = Class1::new();
    control.classes().add("foo");
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let nested = nested_style(&parent, Selectors::nesting(None).not_with(|y| Selectors::class(y, "foo")));
    let match_ = nested.selector().unwrap().match_(&element(&control), Some(&parent), true);
    assert_eq!(match_.result(), SelectorMatchResult::Sometimes);

    let (sink, _activator) = ActivatorSink::new(&match_);
    assert!(!sink.active());
    control.classes().clear();
    assert!(sink.active());
}

#[test]
fn nesting_selector_string_uses_parent_selector() {
    let parent = Style::with_selector(Selectors::of_type::<Class1>());
    let nested = nested_style(&parent, Selectors::nesting(None).class("foo"));
    assert_eq!(nested.to_display_string(), "Class1.foo");
    assert_eq!(Selectors::nesting(None).class("foo").to_string(), "^.foo");
}
