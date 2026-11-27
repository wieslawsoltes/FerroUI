//! Not ports: a layout pass sets the desired size and the bounds of every child, and an unchanged
//! value returns before the property is read. A value that changes is still written and raised,
//! and a value with a NaN is never equal to itself, so it counts as a change every time.

use super::Layoutable;
use crate::reactive::IDisposable;
use crate::visual::geometry_version;
use crate::{FerroObject, FerroProperty, FerroPropertyRegistry, Rect, Size, Visual};
use std::cell::Cell;
use std::rc::Rc;

/// Counts the change notifications of `property` on `object`.
fn changes_of(object: &FerroObject, property: &'static FerroProperty) -> (Rc<Cell<usize>>, Rc<dyn IDisposable>) {
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let id = property.id();
    let subscription = object.property_changed(move |e| {
        if e.property().id() == id {
            seen.set(seen.get() + 1);
        }
    });
    (count, subscription)
}

#[test]
fn bounds_raise_a_change_and_a_nan_but_not_an_unchanged_value() {
    let layoutable = Layoutable::new();
    let (changes, _subscription) = changes_of(&layoutable, Visual::bounds_property().as_property());
    let bounds = Rect::new(1.0, 2.0, 3.0, 4.0);

    layoutable.set_bounds(bounds);
    assert_eq!(changes.get(), 1);
    assert_eq!(layoutable.bounds(), bounds);

    // Unchanged: nothing is raised and the geometry keeps its version.
    let version = geometry_version();
    layoutable.set_bounds(bounds);
    assert_eq!(changes.get(), 1);
    assert_eq!(geometry_version(), version);

    // A real change.
    let moved = Rect::new(1.0, 2.0, 3.0, 5.0);
    layoutable.set_bounds(moved);
    assert_eq!(changes.get(), 2);
    assert_eq!(layoutable.bounds(), moved);
    assert_ne!(geometry_version(), version);

    // A NaN is not equal to itself: the same value is a change each time.
    let nan = Rect::new(f64::NAN, 2.0, 3.0, 4.0);
    layoutable.set_bounds(nan);
    assert_eq!(changes.get(), 3);
    let version = geometry_version();
    layoutable.set_bounds(nan);
    assert_eq!(changes.get(), 4);
    assert_ne!(geometry_version(), version);
    assert!(layoutable.bounds().x.is_nan());
}

#[test]
fn a_desired_size_raises_a_change_and_a_nan_but_not_an_unchanged_value() {
    let layoutable = Layoutable::new();
    let (changes, _subscription) = changes_of(&layoutable, Layoutable::desired_size_property().as_property());
    let size = Size::new(10.0, 20.0);

    layoutable.set_desired_size(size);
    assert_eq!(changes.get(), 1);
    assert_eq!(layoutable.desired_size(), size);

    layoutable.set_desired_size(size);
    assert_eq!(changes.get(), 1);

    let larger = Size::new(10.0, 30.0);
    layoutable.set_desired_size(larger);
    assert_eq!(changes.get(), 2);
    assert_eq!(layoutable.desired_size(), larger);

    let nan = Size::new(f64::NAN, 30.0);
    layoutable.set_desired_size(nan);
    assert_eq!(changes.get(), 3);
    layoutable.set_desired_size(nan);
    assert_eq!(changes.get(), 4);
    assert!(layoutable.desired_size().width.is_nan());
}

/// An unchanged write no longer reads the accessor of the property, which would register it on
/// first use. The object it writes to has registered it: `instantiate` runs the registration of
/// the class and its bases on the thread, whose `ferro_properties!` block calls every accessor.
#[test]
fn an_unchanged_write_on_a_fresh_thread_finds_its_property_registered() {
    std::thread::spawn(|| {
        let registry = FerroPropertyRegistry::instance();
        assert!(!registry.has_registered_name(Visual::TYPE, "Bounds"));
        assert!(!registry.has_registered_name(Layoutable::TYPE, "DesiredSize"));

        let layoutable = Layoutable::new();
        // By the construction alone: no accessor of the two properties has been called here.
        assert!(registry.has_registered_name(Visual::TYPE, "Bounds"));
        assert!(registry.has_registered_name(Layoutable::TYPE, "DesiredSize"));

        layoutable.set_bounds(layoutable.bounds());
        layoutable.set_desired_size(layoutable.desired_size());
        assert!(registry.has_registered_name(Visual::TYPE, "Bounds"));
        assert!(registry.has_registered_name(Layoutable::TYPE, "DesiredSize"));
    })
    .join()
    .expect("the test thread");
}
