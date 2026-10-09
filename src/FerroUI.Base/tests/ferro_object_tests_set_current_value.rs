//! Port of the upstream `SetCurrentValue` object tests.
//!
//! `Class1` is a layoutable where upstream's is a control (the control
//! library is another crate), and the initial layout pass of the tests that
//! toggle styles is the styling of the target.

use super::*;
use crate::data::BindingPriority;
use crate::layout::{Layoutable, LayoutableImpl};
use crate::reactive::Observable;
use crate::styling::test_support::TestRoot;
use crate::styling::{Selectors, Setter, Style};
use crate::*;
use std::cell::Cell;

#[repr(C)]
pub struct Class1 {
    base: Layoutable,
    coerce_max: Cell<f64>,
}

ferro_class!(Class1: Layoutable);
ferro_impl_classes!(Class1: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });

    ferro_property!(pub fn bar_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Bar", s("bardefault"))
    });

    ferro_property!(pub fn inherited_property() -> StyledProperty<String> {
        FerroProperty::register_with::<Class1, _>(
            "Inherited",
            StyledPropertyOptions::new(s("inheriteddefault")).inherits(true),
        )
    });

    ferro_property!(pub fn coerced_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<Class1, _>("Coerced", StyledPropertyOptions::new(0.0).coerce(Class1::coerce))
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Layoutable::construct(), coerce_max: Cell::new(100.0) })
    }

    pub fn with_parent(parent: &Ref<Class1>) -> Ref<Self> {
        let result = Self::new();
        result.set_inheritance_parent(parent);
        result
    }

    pub fn foo(&self) -> String {
        self.get_value(Self::foo_property())
    }

    pub fn bar(&self) -> String {
        self.get_value(Self::bar_property())
    }

    pub fn inherited(&self) -> String {
        self.get_value(Self::inherited_property())
    }

    fn coerce(sender: &FerroObject, value: f64) -> f64 {
        value.min(sender.downcast_ref::<Class1>().expect("a Class1").coerce_max.get())
    }
}

fn get_priority(target: &FerroObject, property: &'static FerroProperty) -> BindingPriority {
    target.values().get_priority(property)
}

fn is_overridden(target: &FerroObject, property: &'static FerroProperty) -> bool {
    target.values().get_effective_value(property).is_some_and(|v| v.is_overriden_current_value())
}

const PRIORITIES: [BindingPriority; 3] =
    [BindingPriority::LocalValue, BindingPriority::Style, BindingPriority::Animation];

#[test]
fn set_current_value_sets_unset_value() {
    let target = Class1::new();

    target.set_current_value(Class1::foo_property(), s("newvalue"));

    assert_eq!("newvalue", target.get_value(Class1::foo_property()));
    assert!(target.is_set(Class1::foo_property()));
    assert_eq!(BindingPriority::Unset, get_priority(&target, Class1::foo_property()));
    assert!(is_overridden(&target, Class1::foo_property()));
}

#[test]
fn set_current_value_sets_unset_value_untyped() {
    let target = Class1::new();

    target.set_current_value_untyped(Class1::foo_property(), &s("newvalue"));

    assert_eq!("newvalue", target.get_value(Class1::foo_property()));
    assert!(target.is_set(Class1::foo_property()));
    assert_eq!(BindingPriority::Unset, get_priority(&target, Class1::foo_property()));
    assert!(is_overridden(&target, Class1::foo_property()));
}

#[test]
fn set_current_value_overrides_existing_value() {
    for priority in PRIORITIES {
        let target = Class1::new();

        target.set_value_with_priority(Class1::foo_property(), s("oldvalue"), priority);
        target.set_current_value(Class1::foo_property(), s("newvalue"));

        assert_eq!("newvalue", target.get_value(Class1::foo_property()), "{priority:?}");
        assert!(target.is_set(Class1::foo_property()));
        assert_eq!(priority, get_priority(&target, Class1::foo_property()));
        assert!(is_overridden(&target, Class1::foo_property()));
    }
}

#[test]
fn set_current_value_overrides_inherited_value() {
    let parent = Class1::new();
    let target = Class1::with_parent(&parent);

    parent.set_value(Class1::inherited_property(), s("inheritedvalue"));
    target.set_current_value(Class1::inherited_property(), s("newvalue"));

    assert_eq!("newvalue", target.get_value(Class1::inherited_property()));
    assert!(target.is_set(Class1::inherited_property()));
    assert_eq!(BindingPriority::Unset, get_priority(&target, Class1::inherited_property()));
    assert!(is_overridden(&target, Class1::inherited_property()));
}

#[test]
fn set_current_value_is_inherited() {
    let parent = Class1::new();
    let target = Class1::with_parent(&parent);

    parent.set_current_value(Class1::inherited_property(), s("newvalue"));

    assert_eq!("newvalue", target.get_value(Class1::inherited_property()));
    assert!(!target.is_set(Class1::foo_property()));
    assert_eq!(BindingPriority::Inherited, get_priority(&target, Class1::inherited_property()));
    assert!(!is_overridden(&target, Class1::inherited_property()));
}

#[test]
fn clear_value_clears_current_value_with_unset_priority() {
    let target = Class1::new();

    target.set_current_value(Class1::foo_property(), s("newvalue"));
    target.clear_value(Class1::foo_property());

    assert_eq!("foodefault", target.foo());
    assert!(!target.is_set(Class1::foo_property()));
    assert!(!is_overridden(&target, Class1::foo_property()));
}

#[test]
fn clear_value_clears_current_value_with_inherited_priority() {
    let parent = Class1::new();
    let target = Class1::with_parent(&parent);

    parent.set_value(Class1::inherited_property(), s("inheritedvalue"));
    target.set_current_value(Class1::inherited_property(), s("newvalue"));
    target.clear_value(Class1::inherited_property());

    assert_eq!("inheritedvalue", target.inherited());
    assert!(!target.is_set(Class1::foo_property()));
    assert!(!is_overridden(&target, Class1::foo_property()));
}

#[test]
fn clear_value_clears_current_value_with_local_value_priority() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("localvalue"));
    target.set_current_value(Class1::foo_property(), s("newvalue"));
    target.clear_value(Class1::foo_property());

    assert_eq!("foodefault", target.foo());
    assert!(!target.is_set(Class1::foo_property()));
    assert!(!is_overridden(&target, Class1::foo_property()));
}

#[test]
fn clear_value_clears_current_value_with_style_priority() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("stylevalue"), BindingPriority::Style);
    target.set_current_value(Class1::foo_property(), s("newvalue"));
    target.clear_value(Class1::foo_property());

    assert_eq!("stylevalue", target.foo());
    assert!(target.is_set(Class1::foo_property()));
    assert!(!is_overridden(&target, Class1::foo_property()));
}

#[test]
fn set_current_value_can_be_coerced() {
    let target = Class1::new();

    target.set_current_value(Class1::coerced_property(), 60.0);
    assert_eq!(60.0, target.get_value(Class1::coerced_property()));

    target.coerce_max.set(50.0);
    target.coerce_value(Class1::coerced_property());
    assert_eq!(50.0, target.get_value(Class1::coerced_property()));

    target.coerce_max.set(100.0);
    target.coerce_value(Class1::coerced_property());
    assert_eq!(60.0, target.get_value(Class1::coerced_property()));
}

#[test]
fn set_current_value_unset_clears_current_value() {
    let target = Class1::new();

    target.set_current_value(Class1::foo_property(), s("newvalue"));
    target.set_current_value_untyped(Class1::foo_property(), &UnsetValueType);

    assert_eq!("foodefault", target.foo());
    assert!(!target.is_set(Class1::foo_property()));
    assert!(!is_overridden(&target, Class1::foo_property()));
}

#[test]
fn set_value_overrides_current_value_with_unset_priority() {
    for priority in PRIORITIES {
        let target = Class1::new();

        target.set_current_value(Class1::foo_property(), s("current"));
        target.set_value_with_priority(Class1::foo_property(), s("setvalue"), priority);

        assert_eq!("setvalue", target.foo(), "{priority:?}");
        assert!(target.is_set(Class1::foo_property()));
        assert_eq!(priority, get_priority(&target, Class1::foo_property()));
        assert!(!is_overridden(&target, Class1::foo_property()));
    }
}

#[test]
fn animation_value_overrides_current_value_with_local_value_priority() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("localvalue"));
    target.set_current_value(Class1::foo_property(), s("current"));
    target.set_value_with_priority(Class1::foo_property(), s("setvalue"), BindingPriority::Animation);

    assert_eq!("setvalue", target.foo());
    assert!(target.is_set(Class1::foo_property()));
    assert_eq!(BindingPriority::Animation, get_priority(&target, Class1::foo_property()));
    assert!(!is_overridden(&target, Class1::foo_property()));
}

#[test]
fn style_trigger_value_overrides_current_value_with_style_priority() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("style"), BindingPriority::Style);
    target.set_current_value(Class1::foo_property(), s("current"));
    target.set_value_with_priority(Class1::foo_property(), s("setvalue"), BindingPriority::StyleTrigger);

    assert_eq!("setvalue", target.foo());
    assert!(target.is_set(Class1::foo_property()));
    assert_eq!(BindingPriority::StyleTrigger, get_priority(&target, Class1::foo_property()));
    assert!(!is_overridden(&target, Class1::foo_property()));
}

#[test]
fn binding_overrides_current_value_with_unset_priority() {
    for priority in PRIORITIES {
        let target = Class1::new();

        target.set_current_value(Class1::foo_property(), s("current"));

        let sub = target.bind(Class1::foo_property(), Observable::single_value(s("binding")), priority);

        assert_eq!("binding", target.foo(), "{priority:?}");
        assert!(target.is_set(Class1::foo_property()));
        assert_eq!(priority, get_priority(&target, Class1::foo_property()));
        assert!(!is_overridden(&target, Class1::foo_property()));

        sub.dispose();

        assert_eq!("foodefault", target.foo(), "{priority:?}");
    }
}

#[test]
fn animation_binding_overrides_current_value_with_local_value_priority() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("localvalue"));
    target.set_current_value(Class1::foo_property(), s("current"));

    let sub = target.bind(Class1::foo_property(), Observable::single_value(s("binding")), BindingPriority::Animation);

    assert_eq!("binding", target.foo());
    assert!(target.is_set(Class1::foo_property()));
    assert_eq!(BindingPriority::Animation, get_priority(&target, Class1::foo_property()));
    assert!(!is_overridden(&target, Class1::foo_property()));

    sub.dispose();

    assert_eq!("current", target.foo());
}

#[test]
fn style_trigger_binding_overrides_current_value_with_style_priority() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("style"), BindingPriority::Style);
    target.set_current_value(Class1::foo_property(), s("current"));

    let sub =
        target.bind(Class1::foo_property(), Observable::single_value(s("binding")), BindingPriority::StyleTrigger);

    assert_eq!("binding", target.foo());
    assert!(target.is_set(Class1::foo_property()));
    assert_eq!(BindingPriority::StyleTrigger, get_priority(&target, Class1::foo_property()));
    assert!(!is_overridden(&target, Class1::foo_property()));

    sub.dispose();

    assert_eq!("style", target.foo());
}

#[test]
fn current_value_is_replaced_by_binding_value() {
    for priority in PRIORITIES {
        let target = Class1::new();
        let source = Subject::behavior(s("initial"));

        target.bind(Class1::foo_property(), source.observable(), priority);
        target.set_current_value(Class1::foo_property(), s("current"));
        source.on_next(s("new"));

        assert_eq!("new", target.foo(), "{priority:?}");
    }
}

#[test]
fn set_current_value_persists_when_toggling_style_1() {
    let target = Class1::new();
    let root = TestRoot::with_child(&target);
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Class1>().class("foo"),
        [Setter::new(Class1::bar_property(), s("bar"))],
    ));

    target.apply_styling();

    target.set_current_value(Class1::foo_property(), s("current"));

    assert_eq!("current", target.foo());
    assert_eq!("bardefault", target.bar());

    target.classes().add("foo");

    assert_eq!("current", target.foo());
    assert_eq!("bar", target.bar());

    target.classes().remove("foo");

    assert_eq!("current", target.foo());
    assert_eq!("bardefault", target.bar());
}

#[test]
fn set_current_value_persists_when_toggling_style_2() {
    let target = Class1::new();
    let root = TestRoot::with_child(&target);
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Class1>().class("foo"),
        [Setter::new(Class1::bar_property(), s("bar")), Setter::new(Class1::inherited_property(), s("inherited"))],
    ));

    target.apply_styling();

    target.set_current_value(Class1::foo_property(), s("current"));

    assert_eq!("current", target.foo());
    assert_eq!("bardefault", target.bar());
    assert_eq!("inheriteddefault", target.inherited());

    target.classes().add("foo");

    assert_eq!("current", target.foo());
    assert_eq!("bar", target.bar());
    assert_eq!("inherited", target.inherited());

    target.classes().remove("foo");

    assert_eq!("current", target.foo());
    assert_eq!("bardefault", target.bar());
    assert_eq!("inheriteddefault", target.inherited());
}

#[test]
fn set_current_value_persists_when_toggling_style_3() {
    let target = Class1::new();
    let root = TestRoot::with_child(&target);
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Class1>().class("foo"),
        [Setter::new(Class1::bar_property(), s("bar")), Setter::new(Class1::inherited_property(), s("inherited"))],
    ));

    target.apply_styling();

    target.set_value_with_priority(Class1::foo_property(), s("not current"), BindingPriority::Template);
    target.set_current_value(Class1::foo_property(), s("current"));

    assert_eq!("current", target.foo());
    assert_eq!("bardefault", target.bar());
    assert_eq!("inheriteddefault", target.inherited());

    target.classes().add("foo");

    assert_eq!("current", target.foo());
    assert_eq!("bar", target.bar());
    assert_eq!("inherited", target.inherited());

    target.classes().remove("foo");

    assert_eq!("current", target.foo());
    assert_eq!("bardefault", target.bar());
    assert_eq!("inheriteddefault", target.inherited());
}

#[test]
fn current_value_is_replaced_by_new_style_activation_1() {
    let target = Class1::new();
    let root = TestRoot::with_child(&target);
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Class1>().class("foo"),
        [Setter::new(Class1::foo_property(), s("initial")), Setter::new(Class1::bar_property(), s("bar"))],
    ));
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Class1>().class("bar"),
        [Setter::new(Class1::foo_property(), s("new")), Setter::new(Class1::bar_property(), s("baz"))],
    ));

    target.apply_styling();

    target.classes().add("foo");
    assert_eq!("initial", target.foo());

    target.set_current_value(Class1::foo_property(), s("current"));
    target.classes().add("bar");

    assert_eq!("new", target.foo());
}

#[test]
fn current_value_is_replaced_by_new_style_activation_2() {
    let target = Class1::new();
    let root = TestRoot::with_child(&target);
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Class1>().class("foo"),
        [Setter::new(Class1::foo_property(), s("foo"))],
    ));
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Class1>().class("foo"),
        [Setter::new(Class1::bar_property(), s("bar"))],
    ));

    target.apply_styling();

    target.set_value_with_priority(Class1::foo_property(), s("template"), BindingPriority::Template);
    target.set_current_value(Class1::foo_property(), s("current"));

    target.classes().add("foo");
    assert_eq!("foo", target.foo());
}
