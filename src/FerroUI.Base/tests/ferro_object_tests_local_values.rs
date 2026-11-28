//! Local ownership is independent of effective animation and style values.
use super::*;
use crate::data::BindingPriority;
use crate::*;

#[repr(C)]
struct Capped {
    base: FerroObject,
    maximum: Cell<i32>,
}
ferro_class!(Capped: FerroObject);
ferro_impl_classes!(Capped: FerroObjectImpl);
impl Capped {
    ferro_property!(
        fn value_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<Self, _>(
                "Value",
                StyledPropertyOptions::new(0).coerce(|owner, value| {
                    value.min(owner.downcast_ref::<Capped>().unwrap().maximum.get())
                }),
            )
        }
    );
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            maximum: Cell::new(100),
        })
    }
}

#[test]
fn suppressed_local_writes_report_the_committed_input_and_can_be_unsubscribed() {
    let target = Capped::new();
    let property = Capped::value_property();
    target.set_value(property, 150);
    let seen = Rc::new(std::cell::RefCell::new(Vec::new()));
    let changes = seen.clone();
    let subscription = target.suppressed_local_value_changed(property.as_property(), move |owner| {
        changes.borrow_mut().push((owner.get_local_value(property), owner.get_value(property)));
    });

    target.set_value(property, 150);
    target.set_value(property, 200);
    target.set_value(property, 50);
    assert_eq!(*seen.borrow(), [(Some(200), 100)]);

    let source = Subject::behavior(BindingValue::from(200));
    let binding = target.bind_value(property, source.observable(), BindingPriority::LocalValue);
    source.on_next(BindingValue::from(300));
    assert_eq!(*seen.borrow(), [(Some(200), 100), (Some(300), 100)]);

    subscription.dispose();
    source.on_next(BindingValue::from(400));
    assert_eq!(*seen.borrow(), [(Some(200), 100), (Some(300), 100)]);
    binding.dispose();
}

#[test]
fn coercion_while_animated_preserves_separate_effective_and_base_values() {
    let target = Capped::new();
    let p = Capped::value_property();
    target.set_value(p, 250);
    let animation = target
        .set_value_with_priority(p, 20, BindingPriority::Animation)
        .unwrap();
    assert_eq!(target.get_value(p), 20);
    assert_eq!(target.get_base_value(p), Some(100));
    target.maximum.set(50);
    target.coerce_value(p.as_property());
    assert_eq!(target.get_value(p), 20);
    assert_eq!(target.get_base_value(p), Some(50));
    target.maximum.set(10);
    target.coerce_value(p.as_property());
    assert_eq!(target.get_value(p), 10);
    assert_eq!(target.get_base_value(p), Some(10));
    target.maximum.set(80);
    target.coerce_value(p.as_property());
    assert_eq!(target.get_value(p), 20);
    assert_eq!(target.get_base_value(p), Some(80));
    animation.dispose();
    assert_eq!(target.get_value(p), 80);
    target.maximum.set(300);
    target.coerce_value(p.as_property());
    assert_eq!(target.get_value(p), 250);
}

test_class!(LocalOwner: FerroObject);
impl LocalOwner {
    ferro_property!(
        fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register_with::<Self, _>(
                "Text",
                StyledPropertyOptions::new(Some("default".into())).inherits(true),
            )
        }
    );
    ferro_property!(
        fn attached_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<Self, FerroObject, _>("Attached", None)
        }
    );
}

#[test]
fn local_presence_distinguishes_default_equal_write_null_and_clear() {
    let target = LocalOwner::new();
    let p = LocalOwner::text_property();
    assert_eq!(target.get_value(p), Some("default".into()));
    assert_eq!(target.get_local_value(p), None);
    target.set_value(p, Some("default".into()));
    assert_eq!(target.get_local_value(p), Some(Some("default".into())));
    target.set_value(p, None);
    assert_eq!(target.get_local_value(p), Some(None));
    target.clear_value(p);
    assert_eq!(target.get_local_value(p), None);
    assert_eq!(target.get_value(p), Some("default".into()));
}

#[test]
fn style_inherited_and_current_style_values_are_not_local_inputs() {
    let parent = LocalOwner::new();
    let target = LocalOwner::new();
    let p = LocalOwner::text_property();
    parent.set_value(p, Some("parent".into()));
    target.set_inheritance_parent(&parent);
    assert_eq!(target.get_value(p), Some("parent".into()));
    assert_eq!(target.get_local_value(p), None);
    let style = target
        .set_value_with_priority(p, Some("style".into()), BindingPriority::Style)
        .unwrap();
    assert_eq!(target.get_base_value(p), Some(Some("style".into())));
    assert_eq!(target.get_local_value(p), None);
    target.set_current_value(p, Some("current style".into()));
    assert_eq!(target.get_local_value(p), None);
    style.dispose();
}

#[test]
fn local_input_survives_animation_writes_and_clear_without_using_effective_diagnostics() {
    use crate::diagnostics::FerroObjectDiagnosticExtensions;
    let target = LocalOwner::new();
    let p = LocalOwner::text_property();
    target.set_value(p, Some("local".into()));
    let animation = target
        .set_value_with_priority(p, Some("animated".into()), BindingPriority::Animation)
        .unwrap();
    assert_eq!(
        target.get_diagnostic(p.as_property()).priority(),
        BindingPriority::Animation
    );
    assert_eq!(target.get_local_value(p), Some(Some("local".into())));
    target.set_value(p, Some("updated".into()));
    assert_eq!(target.get_value(p), Some("animated".into()));
    assert_eq!(target.get_local_value(p), Some(Some("updated".into())));
    target.clear_value(p);
    assert_eq!(target.get_local_value(p), Some(Some("updated".into())));
    target.clear_local_value(p);
    assert_eq!(target.get_value(p), Some("animated".into()));
    assert_eq!(target.get_local_value(p), None);
    target.set_value(p, None);
    assert_eq!(target.get_local_value(p), Some(None));
    animation.dispose();
    assert_eq!(target.get_value(p), None);
    assert_eq!(target.get_local_value(p), Some(None));
}

#[test]
fn reads_are_silent_and_do_not_coerce_or_install_entries() {
    let target = Capped::new();
    let p = Capped::value_property();
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    target.add_property_changed_handler(Rc::new(move |_| observed.set(observed.get() + 1)));
    for _ in 0..10 {
        assert_eq!(target.get_local_value(p), None);
    }
    assert!(!target.is_set(p.as_property()));
    target.set_value(p, 250);
    let before = changes.get();
    target.maximum.set(10);
    for _ in 0..10 {
        assert_eq!(target.get_local_value(p), Some(250));
    }
    assert_eq!(target.get_value(p), 100);
    assert_eq!(changes.get(), before);
}

#[test]
fn native_local_binding_pending_null_animation_retained_write_replacement_and_disposal() {
    let target = LocalOwner::new();
    let p = LocalOwner::text_property();
    let source = Subject::<Option<String>>::new();
    let binding = target.bind(p, source.observable(), BindingPriority::LocalValue);
    assert!(target.has_local_value_binding(p.as_property()));
    assert_eq!(target.get_local_value(p), None);
    source.on_next(Some("bound".into()));
    assert_eq!(target.get_local_value(p), Some(Some("bound".into())));
    let animation = target
        .set_value_with_priority(p, Some("animation".into()), BindingPriority::Animation)
        .unwrap();
    source.on_next(None);
    assert_eq!(target.get_value(p), Some("animation".into()));
    assert_eq!(target.get_local_value(p), Some(None));
    // Native SetValue intentionally retains a binding; WinUI binding removal is
    // a separate mapping responsibility, not changed by the inspection API.
    target.set_value(p, Some("replacement".into()));
    assert!(target.has_local_value_binding(p.as_property()));
    source.on_next(Some("later".into()));
    assert_eq!(target.get_local_value(p), Some(Some("later".into())));
    let other = Subject::behavior(Some("new binding".into()));
    let replacement = target.bind(p, other.observable(), BindingPriority::LocalValue);
    assert_eq!(target.get_local_value(p), Some(Some("new binding".into())));
    binding.dispose();
    source.on_next(Some("stale source".into()));
    assert_eq!(target.get_local_value(p), Some(Some("new binding".into())));
    replacement.dispose();
    assert!(!target.has_local_value_binding(p.as_property()));
    // Native binding disposal uses ClearValue; an animation hides the old input.
    assert_eq!(target.get_local_value(p), Some(Some("new binding".into())));
    target.clear_local_value(p);
    assert_eq!(target.get_local_value(p), None);
    animation.dispose();
    target.clear_value(p);
    assert_eq!(target.get_local_value(p), None);
}

#[test]
fn attached_property_reads_preserve_boxed_reference_identity_and_null() {
    let target = FerroObject::new();
    let p = LocalOwner::attached_property();
    assert_eq!(target.get_local_value(p), None);
    target.set_value(p, None);
    assert_eq!(target.get_local_value(p), Some(None));
    target.set_value(p, Some("attached".into()));
    assert_eq!(target.get_local_value(p), Some(Some("attached".into())));
    let object = Rc::new(Cell::new(1));
    let boxed: BoxedValue = object.clone();
    let object_property = FerroProperty::register::<LocalOwner, Option<BoxedValue>>("Object", None);
    target.set_value(object_property, Some(boxed.clone()));
    let result = target.get_local_value(object_property).unwrap().unwrap();
    assert!(Rc::ptr_eq(&result, &boxed));
    object.set(2);
    assert_eq!(result.downcast_ref::<Cell<i32>>().unwrap().get(), 2);
}

#[test]
fn local_input_is_uncoerced_before_during_and_after_recoercion_and_animation() {
    let target = Capped::new();
    let p = Capped::value_property();
    target.set_value(p, 250);
    assert_eq!(target.get_value(p), 100);
    assert_eq!(target.get_local_value(p), Some(250));
    let animation = target
        .set_value_with_priority(p, 20, BindingPriority::Animation)
        .unwrap();
    target.maximum.set(50);
    target.coerce_value(p.as_property());
    assert_eq!(target.get_local_value(p), Some(250));
    assert_eq!(target.get_base_value(p), Some(50));
    animation.dispose();
    assert_eq!(target.get_value(p), 50);
    assert_eq!(target.get_local_value(p), Some(250));
    target.maximum.set(300);
    target.coerce_value(p.as_property());
    assert_eq!(target.get_value(p), 250);
    assert_eq!(target.get_local_value(p), Some(250));
}

#[test]
fn coercion_without_animation_keeps_coerced_base_and_raw_local_input_distinct() {
    let target = Capped::new();
    let p = Capped::value_property();
    target.set_value(p, 250);
    for (maximum, expected) in [(50, 50), (300, 250), (10, 10), (100, 100)] {
        target.maximum.set(maximum);
        target.coerce_value(p.as_property());
        assert_eq!(target.get_value(p), expected);
        assert_eq!(target.get_base_value(p), Some(expected));
        assert_eq!(target.get_local_value(p), Some(250));
    }
}

#[test]
fn clearing_local_input_retains_style_fallback_and_is_silent_without_a_local_input() {
    let target = LocalOwner::new();
    let p = LocalOwner::text_property();
    let style = target
        .set_value_with_priority(p, Some("style".into()), BindingPriority::Style)
        .unwrap();
    target.clear_local_value(p);
    assert_eq!(target.get_value(p), Some("style".into()));
    target.set_value(p, Some("local".into()));
    let animation = target
        .set_value_with_priority(p, Some("animation".into()), BindingPriority::Animation)
        .unwrap();
    target.clear_local_value(p);
    assert_eq!(target.get_local_value(p), None);
    assert_eq!(target.get_base_value(p), Some(Some("style".into())));
    assert_eq!(target.get_value(p), Some("animation".into()));
    animation.dispose();
    assert_eq!(target.get_value(p), Some("style".into()));
    style.dispose();
    assert_eq!(target.get_value(p), Some("default".into()));
}

#[repr(C)]
struct ObservedOwner {
    base: LocalOwner,
    trace: RefCell<Vec<(bool, Option<Option<String>>)>>,
}
ferro_class!(ObservedOwner: LocalOwner);
impl FerroObjectImpl for ObservedOwner {
    fn on_property_changed_core(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed_core(this, e);
        let p = LocalOwner::text_property();
        if e.property() != p.as_property() {
            return;
        }
        let local = this.get_local_value(p);
        this.trace
            .borrow_mut()
            .push((e.is_effective_value_change(), local.clone()));
        if local == Some(Some("outer".into())) {
            this.set_value(p, Some("nested".into()));
        }
    }
}
#[test]
fn base_notification_can_read_local_input_and_reenter_without_losing_the_nested_write() {
    let target = instantiate(ObservedOwner {
        base: LocalOwner::construct(),
        trace: RefCell::new(Vec::new()),
    });
    let p = LocalOwner::text_property();
    target.set_value(p, Some("initial".into()));
    let animation = target
        .set_value_with_priority(p, Some("animation".into()), BindingPriority::Animation)
        .unwrap();
    target.trace.borrow_mut().clear();
    target.set_value(p, Some("outer".into()));
    assert_eq!(target.get_value(p), Some("animation".into()));
    assert_eq!(target.get_local_value(p), Some(Some("nested".into())));
    assert_eq!(
        &*target.trace.borrow(),
        &[
            (false, Some(Some("outer".into()))),
            (false, Some(Some("nested".into())))
        ]
    );
    animation.dispose();
}
