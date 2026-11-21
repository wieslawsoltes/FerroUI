//! The base value of an effective value: kept once where it is the value, and a value of its own under an
//! animation.

use super::*;
#[allow(unused_imports)]
use crate::{PropertyValue, StyledProperty};
use crate::reactive::IDisposable;
use crate::{StyledPropertyOptions, Thickness};
use std::mem::size_of;
use std::sync::atomic::{AtomicUsize, Ordering};

fn unique_name() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("EffectiveValueBase{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

/// A value that counts how many of it are alive.
struct Counted {
    number: i32,
    alive: Rc<Cell<usize>>,
}

crate::styled_value!(Counted);

impl Counted {
    fn new(number: i32, alive: &Rc<Cell<usize>>) -> Self {
        alive.set(alive.get() + 1);
        Self { number, alive: alive.clone() }
    }
}

impl Clone for Counted {
    fn clone(&self) -> Self {
        Self::new(self.number, &self.alive)
    }
}

impl PartialEq for Counted {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.alive.set(self.alive.get() - 1);
    }
}

/// An object, a property of counted values, and the count of those alive before a value is set.
fn counted() -> (crate::Ref<FerroObject>, &'static StyledProperty<Counted>, Rc<Cell<usize>>, usize) {
    let alive = Rc::new(Cell::new(0));
    let default = Counted::new(0, &alive);
    let property = FerroProperty::register_with::<FerroObject, _>(&unique_name(), StyledPropertyOptions::new(default));
    let before = alive.get();
    (FerroObject::new(), property, alive, before)
}

/// Whether the base value of the property is the value (`true`) or a value of its own.
fn base_is_the_value<T: crate::StyledValue>(owner: &FerroObject, property: &'static StyledProperty<T>) -> bool {
    let effective = owner.values().get_effective_value(property).unwrap();
    let typed = cast_effective_value::<T::Storage>(&*effective);
    assert!(!(typed.state.base_is_value.get() && typed.base_value.is_some()));
    typed.state.base_is_value.get()
}

fn number(value: Option<Counted>) -> Option<i32> {
    value.map(|value| value.number)
}

#[cfg(target_pointer_width = "64")]
#[test]
fn an_effective_value_has_no_field_for_a_base_value_that_is_the_value() {
    assert!(size_of::<EffectiveValueState>() <= 56, "{}", size_of::<EffectiveValueState>());
    assert!(size_of::<EffectiveValue<f64>>() <= 96, "{}", size_of::<EffectiveValue<f64>>());
    assert!(size_of::<EffectiveValue<bool>>() <= 96, "{}", size_of::<EffectiveValue<bool>>());
    assert!(size_of::<EffectiveValue<Thickness>>() <= 120, "{}", size_of::<EffectiveValue<Thickness>>());
    // The value, a pointer for a base value of its own, and what every value has.
    assert_eq!(size_of::<EffectiveValue<Thickness>>() - size_of::<EffectiveValue<f64>>(), 24);
}

/// The memory of the tagged storage, printed for the size report of the experiment
/// (`cargo test -p ferroui-base --lib small_value_memory -- --nocapture`).
#[cfg(target_pointer_width = "64")]
#[test]
fn small_value_memory() {
    use crate::SmallValue;
    println!(
        "slot: SmallValue {} bytes; EffectiveValue<SmallValue> {} bytes; typed f64 {}, bool {}, Thickness {}",
        size_of::<SmallValue>(),
        size_of::<EffectiveValue<SmallValue>>(),
        size_of::<EffectiveValue<f64>>(),
        size_of::<EffectiveValue<bool>>(),
        size_of::<EffectiveValue<Thickness>>(),
    );
    assert!(size_of::<SmallValue>() <= 40);
}

#[test]
fn a_value_without_an_animation_over_it_is_kept_once() {
    let (owner, property, alive, before) = counted();

    owner.set_value(property, Counted::new(1, &alive));
    assert_eq!(alive.get(), before + 1);
    assert!(base_is_the_value(&owner, property));
    assert_eq!(number(owner.get_base_value(property)), Some(1));
    assert_eq!(owner.get_value(property).number, 1);
    assert_eq!(alive.get(), before + 1);

    // Set again, to another value and to the same one.
    owner.set_value(property, Counted::new(2, &alive));
    owner.set_value(property, Counted::new(2, &alive));
    assert_eq!(alive.get(), before + 1);
    assert!(base_is_the_value(&owner, property));
    assert_eq!(number(owner.get_base_value(property)), Some(2));

    // A value of a style is one as well, beside the one its frame keeps.
    let (owner, property, alive, before) = counted();
    let style = owner.set_value_with_priority(property, Counted::new(3, &alive), BindingPriority::Style);
    assert_eq!(alive.get(), before + 2);
    assert!(base_is_the_value(&owner, property));
    assert_eq!(number(owner.get_base_value(property)), Some(3));
    drop(style);
}

#[test]
fn an_animation_over_a_value_leaves_the_value_as_the_base_value() {
    let (owner, property, alive, before) = counted();

    owner.set_value(property, Counted::new(1, &alive));
    let animation = owner.set_value_with_priority(property, Counted::new(5, &alive), BindingPriority::Animation);
    assert_eq!(owner.get_value(property).number, 5);
    assert_eq!(number(owner.get_base_value(property)), Some(1));
    assert!(!base_is_the_value(&owner, property));
    // The base value, the value, and the one the frame of the animation keeps.
    assert_eq!(alive.get(), before + 3);

    // A set under the animation replaces the base value alone.
    owner.set_value(property, Counted::new(2, &alive));
    assert_eq!(owner.get_value(property).number, 5);
    assert_eq!(number(owner.get_base_value(property)), Some(2));
    assert!(!base_is_the_value(&owner, property));
    assert_eq!(alive.get(), before + 3);

    // Without the animation the base value is the value, kept once again.
    animation.expect("a disposable").dispose();
    assert_eq!(owner.get_value(property).number, 2);
    assert_eq!(number(owner.get_base_value(property)), Some(2));
    assert!(base_is_the_value(&owner, property));
    assert_eq!(alive.get(), before + 1);
}

#[test]
fn a_value_set_under_an_animation_that_came_first_is_the_base_value_alone() {
    let (owner, property, alive, before) = counted();

    let animation = owner.set_value_with_priority(property, Counted::new(5, &alive), BindingPriority::Animation);
    assert_eq!(owner.get_value(property).number, 5);
    assert_eq!(number(owner.get_base_value(property)), None);
    assert!(!base_is_the_value(&owner, property));

    owner.set_value(property, Counted::new(1, &alive));
    assert_eq!(owner.get_value(property).number, 5);
    assert_eq!(number(owner.get_base_value(property)), Some(1));
    assert!(!base_is_the_value(&owner, property));
    assert_eq!(alive.get(), before + 3);

    animation.expect("a disposable").dispose();
    assert_eq!(owner.get_value(property).number, 1);
    assert_eq!(number(owner.get_base_value(property)), Some(1));
    assert!(base_is_the_value(&owner, property));
    assert_eq!(alive.get(), before + 1);
}
