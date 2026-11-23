//! A change of an inherited value that reaches an object from an ancestor is raised on the
//! object where something there can hear it: a listener of the object, a handler of the property
//! for its class, or a change handler of its class that reacts to the property
//! (desktop-performance.md, section 9.101).

use super::*;
use crate::*;
use std::cell::Cell;

ferro_thread_local! {
    /// The calls of the change handlers of the test classes.
    static HANDLED: Cell<u32> = const { Cell::new(0) };
    /// What `Stating` answers when it is asked whether an object follows every change.
    static FOLLOWS: Cell<bool> = const { Cell::new(false) };
}

fn handled() -> u32 {
    HANDLED.with(Cell::get)
}

test_class!(Holder: FerroObject);

impl Holder {
    ferro_property!(pub fn stated_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Holder, _>("Stated", StyledPropertyOptions::new(0).inherits(true))
    });

    ferro_property!(pub fn other_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Holder, _>("Other", StyledPropertyOptions::new(0).inherits(true))
    });
}

/// A class whose change handler states that it reacts to `Stated`, and to every property while
/// `FOLLOWS` is set.
#[repr(C)]
pub struct Stating {
    base: Holder,
}

ferro_class!(Stating: Holder);

ferro_overrides! { impl FerroObjectImpl for Stating {
    inherited_reactions: InheritedReactions::to(|property| {
        if property.name() == "Stated" {
            InheritedReaction::Always
        } else {
            InheritedReaction::While(|_| FOLLOWS.with(Cell::get))
        }
    });

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        HANDLED.with(|handled| handled.set(handled.get() + 1));
    }
} }

impl Stating {
    fn below<T: ObjectType + Upcast<FerroObject>>(parent: &Ref<T>) -> Ref<Self> {
        let result = instantiate(Self { base: Holder::construct() });
        result.set_inheritance_parent(parent);
        result
    }
}

/// A class whose change handler states nothing.
#[repr(C)]
pub struct Silent {
    base: Holder,
}

ferro_class!(Silent: Holder);

impl FerroObjectImpl for Silent {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        HANDLED.with(|handled| handled.set(handled.get() + 1));
    }
}

impl Silent {
    fn below<T: ObjectType + Upcast<FerroObject>>(parent: &Ref<T>) -> Ref<Self> {
        let result = instantiate(Self { base: Holder::construct() });
        result.set_inheritance_parent(parent);
        result
    }
}

#[test]
fn a_class_hears_the_inherited_properties_it_states_and_no_other() {
    let parent = Holder::new();
    let child = Stating::below(&parent);

    parent.set_value(Holder::stated_property(), 1);
    assert_eq!(1, handled());

    parent.set_value(Holder::other_property(), 2);
    assert_eq!(1, handled());
    // The value is the one of the parent all the same.
    assert_eq!(2, child.get_value(Holder::other_property()));

    parent.clear_value(Holder::other_property());
    assert_eq!(1, handled());
    assert_eq!(0, child.get_value(Holder::other_property()));
}

#[test]
fn an_object_given_an_inheritance_parent_is_told_the_values_it_hears() {
    let parent = Holder::new();
    parent.set_value(Holder::stated_property(), 1);
    parent.set_value(Holder::other_property(), 2);

    let child = Stating::below(&parent);
    assert_eq!(1, handled());
    assert_eq!(1, child.get_value(Holder::stated_property()));
    assert_eq!(2, child.get_value(Holder::other_property()));

    child.set_inheritance_parent(None::<Ref<FerroObject>>);
    assert_eq!(2, handled());
    assert_eq!(0, child.get_value(Holder::other_property()));
}

#[test]
fn a_class_that_states_nothing_hears_every_inherited_change() {
    let parent = Holder::new();
    let _child = Silent::below(&parent);

    parent.set_value(Holder::stated_property(), 1);
    parent.set_value(Holder::other_property(), 2);
    assert_eq!(2, handled());
}

#[test]
fn the_change_goes_on_below_an_object_that_does_not_hear_it() {
    let parent = Holder::new();
    let child = Stating::below(&parent);
    let quiet = Holder::new();
    quiet.set_inheritance_parent(&child);
    let _grandchild = Silent::below(&quiet);

    parent.set_value(Holder::other_property(), 2);
    assert_eq!(1, handled());
    assert_eq!(2, quiet.get_value(Holder::other_property()));
}

#[test]
fn a_listener_of_the_object_hears_every_inherited_change_while_it_listens() {
    let parent = Holder::new();
    let child = Stating::below(&parent);
    let raised = Counter::new();

    let token = {
        let raised = raised.clone();
        child.add_property_changed_handler(Rc::new(move |_| raised.increment()))
    };
    parent.set_value(Holder::other_property(), 2);
    assert_eq!(1, raised.get());
    // The change is raised as it always was: the handler of the class is called too.
    assert_eq!(1, handled());

    child.remove_property_changed_handler(token);
    parent.set_value(Holder::other_property(), 3);
    assert_eq!(1, raised.get());
    assert_eq!(1, handled());
}

#[test]
fn a_handler_of_the_property_for_the_class_is_heard_while_it_is_there() {
    let parent = Holder::new();
    let _child = Stating::below(&parent);
    let raised = Counter::new();

    // A handler for another class makes no object of this class hear the property.
    let for_another = Holder::other_property().changed().add_class_handler::<Silent>(|_, _| {});
    parent.set_value(Holder::other_property(), 1);
    assert_eq!(0, handled());

    let subscription = {
        let raised = raised.clone();
        Holder::other_property().changed().add_class_handler::<Stating>(move |_, _| raised.increment())
    };
    parent.set_value(Holder::other_property(), 2);
    assert_eq!(1, raised.get());
    assert_eq!(1, handled());

    drop(subscription);
    parent.set_value(Holder::other_property(), 3);
    assert_eq!(1, raised.get());
    assert_eq!(1, handled());

    // A handler for every class is heard by every object.
    let for_all = {
        let raised = raised.clone();
        Holder::other_property().changed().subscribe(move |_| raised.increment())
    };
    parent.set_value(Holder::other_property(), 4);
    // The parent and the child.
    assert_eq!(3, raised.get());
    assert_eq!(2, handled());

    drop(for_all);
    drop(for_another);
}

#[test]
fn an_object_that_follows_every_change_hears_it() {
    let parent = Holder::new();
    let _child = Stating::below(&parent);

    parent.set_value(Holder::other_property(), 1);
    assert_eq!(0, handled());

    FOLLOWS.with(|follows| follows.set(true));
    parent.set_value(Holder::other_property(), 2);
    assert_eq!(1, handled());

    FOLLOWS.with(|follows| follows.set(false));
    parent.set_value(Holder::other_property(), 3);
    assert_eq!(1, handled());
}
