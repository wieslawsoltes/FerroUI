//! Explicit local binding ownership, independent of animation/style precedence.
use super::*;
use crate::data::{BindingOperations, BindingPriority, ReflectionBinding};
use crate::reactive::{Observable, ObservableExt};
use crate::*;

test_class!(Target: FerroObject);
impl Target {
    ferro_property!(
        fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<Self, _>("Text", None)
        }
    );
}

#[test]
fn removing_local_observable_binding_ends_subscription_before_clear_notification() {
    let target = Target::new();
    let p = Target::text_property();
    let count = Rc::new(Cell::new(0));
    let source = Observable::create({
        let count = count.clone();
        move |observer| {
            count.set(count.get() + 1);
            observer.on_next(Some("helper".to_owned()));
            let count = count.clone();
            Disposable::create(move || count.set(count.get() - 1))
        }
    });
    let handle = target.bind(p, source, BindingPriority::LocalValue);
    let events = Rc::new(RefCell::new(Vec::new()));
    let weak = target.downgrade();
    target.property_changed({
        let events = events.clone();
        let count = count.clone();
        move |_| {
            let target = weak.upgrade().unwrap();
            events.borrow_mut().push((
                target.get_value(p),
                count.get(),
                target.has_local_value_binding(p.as_property()),
            ));
        }
    });
    target.clear_local_value_and_binding(p);
    assert_eq!(target.get_local_value(p), None);
    assert_eq!(*events.borrow(), vec![(None, 0, false)]);
    assert_eq!(count.get(), 0);
    handle.dispose();
    target.clear_local_value_and_binding(p);
    assert_eq!(events.borrow().len(), 1);
}

#[test]
fn clear_retains_a_new_value_written_by_property_notification() {
    let target = Target::new();
    let p = Target::text_property();
    target.bind(
        p,
        Observable::single_value(Some("helper".to_owned())),
        BindingPriority::LocalValue,
    );
    let weak = target.downgrade();
    target.property_changed(move |_| {
        let target = weak.upgrade().unwrap();
        if target.get_value(p).is_none() {
            target.set_value(p, Some("nested".into()));
        }
    });
    target.clear_local_value_and_binding(p);
    assert_eq!(target.get_value(p).as_deref(), Some("nested"));
    assert_eq!(target.get_local_value(p), Some(Some("nested".into())));
    assert!(!target.has_local_value_binding(p.as_property()));
}

#[test]
fn clear_removes_local_under_animation_and_keeps_style_and_animation_sources() {
    let target = Target::new();
    let p = Target::text_property();
    let local = Subject::behavior(Some("local".to_owned()));
    let style = Subject::behavior(Some("style".to_owned()));
    let animation = Subject::behavior(Some("animation".to_owned()));
    target.bind(p, style.observable(), BindingPriority::Style);
    target.bind(p, local.observable(), BindingPriority::LocalValue);
    let animation_handle = target.bind(p, animation.observable(), BindingPriority::Animation);
    target.clear_local_value_and_binding(p);
    assert_eq!(local.subscriber_count(), 0);
    assert_eq!(style.subscriber_count(), 1);
    assert_eq!(animation.subscriber_count(), 1);
    assert_eq!(target.get_value(p).as_deref(), Some("animation"));
    assert_eq!(target.get_local_value(p), None);
    animation_handle.dispose();
    assert_eq!(target.get_value(p).as_deref(), Some("style"));
    local.on_next(Some("after".into()));
    assert_eq!(target.get_value(p).as_deref(), Some("style"));
}

#[test]
fn local_expression_query_does_not_return_style_or_animation_expression() {
    let target = Target::new();
    let p = Target::text_property();
    let binding = ReflectionBinding::new("").with_source(Some(Rc::new("bound".to_owned())));
    binding.set_priority(BindingPriority::Style);
    target.bind_binding(p, &binding);
    assert!(BindingOperations::get_local_binding_expression_base(&target, p).is_none());
    binding.set_priority(BindingPriority::LocalValue);
    let local = target.bind_binding(p, &binding);
    binding.set_priority(BindingPriority::Animation);
    let animation = target.bind_binding(p, &binding);
    assert!(Rc::ptr_eq(
        &BindingOperations::get_binding_expression_base(&target, p).unwrap(),
        &animation
    ));
    assert!(Rc::ptr_eq(
        &BindingOperations::get_local_binding_expression_base(&target, p).unwrap(),
        &local
    ));
    target.clear_local_value_and_binding(p);
    assert!(BindingOperations::get_local_binding_expression_base(&target, p).is_none());
    assert!(Rc::ptr_eq(
        &BindingOperations::get_binding_expression_base(&target, p).unwrap(),
        &animation
    ));
}

#[test]
fn disposed_previous_handle_does_not_remove_reentrant_replacement_binding() {
    let target = Target::new();
    let p = Target::text_property();
    let old = target.bind(
        p,
        Observable::single_value(Some("helper".to_owned())),
        BindingPriority::LocalValue,
    );
    let replacement = Subject::behavior(Some("nested".to_owned()));
    let weak = target.downgrade();
    target.property_changed({
        let replacement = replacement.clone();
        move |_| {
            let target = weak.upgrade().unwrap();
            if target.get_value(p).is_none() {
                target.bind(p, replacement.observable(), BindingPriority::LocalValue);
            }
        }
    });
    target.clear_local_value_and_binding(p);
    old.dispose();
    replacement.on_next(Some("after".into()));
    assert_eq!(target.get_value(p).as_deref(), Some("after"));
    assert_eq!(replacement.subscriber_count(), 1);
}

#[test]
fn source_notification_snapshot_cannot_publish_through_removed_binding() {
    let target = Target::new();
    let p = Target::text_property();
    let source = Subject::new();
    let clear = Rc::new(Cell::new(false));
    let weak = target.downgrade();
    let driver = source.observable().subscribe_fn({
        let clear = clear.clone();
        move |_| {
            if clear.replace(false) {
                weak.upgrade().unwrap().clear_local_value_and_binding(p);
            }
        }
    });
    let binding = target.bind(p, source.observable(), BindingPriority::LocalValue);
    source.on_next(Some("helper".to_owned()));
    clear.set(true);
    source.on_next(Some("after".to_owned()));
    assert_eq!(target.get_value(p), None);
    assert_eq!(target.get_local_value(p), None);
    assert_eq!(source.subscriber_count(), 1);
    binding.dispose();
    driver.dispose();
}

#[test]
fn clearing_during_initial_publication_disposes_subscription_returned_afterwards() {
    let target = Target::new();
    let p = Target::text_property();
    let weak = target.downgrade();
    target.property_changed(move |_| {
        let target = weak.upgrade().unwrap();
        if target.get_value(p).is_some() {
            target.clear_local_value_and_binding(p);
        }
    });
    let subscriptions = Rc::new(Cell::new(0));
    let source = Observable::create({
        let subscriptions = subscriptions.clone();
        move |observer| {
            subscriptions.set(subscriptions.get() + 1);
            observer.on_next(Some("helper".to_owned()));
            let subscriptions = subscriptions.clone();
            Disposable::create(move || subscriptions.set(subscriptions.get() - 1))
        }
    });
    let handle = target.bind(p, source, BindingPriority::LocalValue);
    assert_eq!(target.get_value(p), None);
    assert_eq!(subscriptions.get(), 0);
    assert!(!target.has_local_value_binding(p.as_property()));
    handle.dispose();
    assert_eq!(subscriptions.get(), 0);
}
