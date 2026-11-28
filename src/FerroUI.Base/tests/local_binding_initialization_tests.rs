//! Synchronous local-expression initialization is a native lifecycle query.
use super::*;
use crate::data::converters::FuncValueConverter;
use crate::data::{BindingPriority, ReflectionBinding};
use crate::reactive::Observable;
use crate::*;

test_class!(Target: FerroObject);
impl Target {
    ferro_property!(
        fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<Self, _>("Text", None)
        }
    );
    ferro_property!(
        fn other_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<Self, _>("Other", None)
        }
    );
}
fn binding(value: &str) -> Rc<ReflectionBinding> {
    ReflectionBinding::new("").with_source(Some(Rc::new(value.to_owned())))
}

#[test]
fn query_is_true_only_during_local_expression_start() {
    let target = Target::new();
    let p = Target::text_property();
    assert!(!target.is_local_binding_expression_initializing(p.as_property()));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let weak = target.downgrade();
    target.property_changed({
        let seen = seen.clone();
        move |_| {
            seen.borrow_mut().push(
                weak.upgrade()
                    .unwrap()
                    .is_local_binding_expression_initializing(p.as_property()),
            );
        }
    });
    target.bind_binding(p, &binding("first"));
    assert!(!target.is_local_binding_expression_initializing(p.as_property()));
    target.set_value(p, Some("later".into()));
    assert_eq!(*seen.borrow(), vec![true, false]);
}

#[test]
fn nested_properties_restore_the_outer_start_scope() {
    let target = Target::new();
    let a = Target::text_property();
    let b = Target::other_property();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let weak = target.downgrade();
    target.property_changed({
        let seen = seen.clone();
        move |change| {
            let target = weak.upgrade().unwrap();
            if change.property().id() == a.id() {
                target.bind_binding(b, &binding("nested"));
                seen.borrow_mut().push((
                    target.is_local_binding_expression_initializing(a.as_property()),
                    target.is_local_binding_expression_initializing(b.as_property()),
                ));
            } else if change.property().id() == b.id() {
                seen.borrow_mut().push((
                    target.is_local_binding_expression_initializing(a.as_property()),
                    target.is_local_binding_expression_initializing(b.as_property()),
                ));
            }
        }
    });
    target.bind_binding(a, &binding("outer"));
    assert_eq!(*seen.borrow(), vec![(true, true), (true, false)]);
    assert!(!target.is_local_binding_expression_initializing(a.as_property()));
    assert!(!target.is_local_binding_expression_initializing(b.as_property()));
}

#[test]
fn completed_replacement_does_not_inherit_the_outer_expression_scope() {
    let target = Target::new();
    let p = Target::text_property();
    let weak = target.downgrade();
    let nested = Rc::new(Cell::new(false));
    let seen = Rc::new(RefCell::new(Vec::new()));
    target.property_changed({
        let seen = seen.clone();
        move |_| {
            let target = weak.upgrade().unwrap();
            if !nested.replace(true) {
                target.bind_binding(p, &binding("replacement"));
                seen.borrow_mut().push((
                    target.get_value(p),
                    target.is_local_binding_expression_initializing(p.as_property()),
                ));
            } else {
                seen.borrow_mut().push((
                    target.get_value(p),
                    target.is_local_binding_expression_initializing(p.as_property()),
                ));
            }
        }
    });
    target.bind_binding(p, &binding("outer"));
    assert_eq!(
        *seen.borrow(),
        vec![
            (None, false),
            (Some("replacement".into()), true),
            (Some("replacement".into()), false)
        ]
    );
    assert_eq!(target.get_value(p).as_deref(), Some("replacement"));
}

#[test]
fn initialization_scope_resets_after_callback_panic() {
    let target = Target::new();
    let p = Target::text_property();
    let weak = target.downgrade();
    let once = Cell::new(true);
    target.property_changed(move |_| {
        if once.replace(false) {
            assert!(
                weak.upgrade()
                    .unwrap()
                    .is_local_binding_expression_initializing(p.as_property())
            );
            panic!("initial callback");
        }
    });
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || target.bind_binding(p, &binding("value"))
        ))
        .is_err()
    );
    assert!(!target.is_local_binding_expression_initializing(p.as_property()));
    target.clear_local_value_and_binding(p);
}

#[test]
fn local_initialization_is_visible_under_animation_and_query_is_silent() {
    let target = Target::new();
    let p = Target::text_property();
    let calls = Rc::new(Cell::new(0));
    let animation = binding("animation");
    animation.set_priority(BindingPriority::Animation);
    target.bind_binding(p, &animation);
    let local = binding("local");
    let weak = target.downgrade();
    local.set_converter(Some(Rc::new(FuncValueConverter::<String, String>::new({
        let calls = calls.clone();
        move |value| {
            let target = weak.upgrade().unwrap();
            assert!(target.is_local_binding_expression_initializing(p.as_property()));
            calls.set(calls.get() + 1);
            value
        }
    }))));
    target.bind_binding(p, &local);
    assert_eq!(calls.get(), 1);
    for _ in 0..10 {
        assert!(!target.is_local_binding_expression_initializing(p.as_property()));
    }
    assert_eq!(calls.get(), 1);
    assert_eq!(target.get_value(p).as_deref(), Some("animation"));
}

#[test]
fn observable_and_style_bindings_do_not_report_a_local_expression_start() {
    let target = Target::new();
    let p = Target::text_property();
    let weak = target.downgrade();
    target.property_changed(move |_| {
        assert!(
            !weak
                .upgrade()
                .unwrap()
                .is_local_binding_expression_initializing(p.as_property())
        )
    });
    let style = binding("style");
    style.set_priority(BindingPriority::Style);
    target.bind_binding(p, &style);
    target.bind(
        p,
        Observable::single_value(Some("observable".into())),
        BindingPriority::LocalValue,
    );
}
