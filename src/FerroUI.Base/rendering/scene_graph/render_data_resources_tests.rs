//! Port of `Rendering/SceneGraph/RenderDataResourcesTests.cs`.
//!
//! The table holds the typed resources of render data rather than any
//! object; the plain objects of upstream are custom draw operations, a
//! resource kind that has no value equality.

use super::scene_graph_test_support::TestCustomOperation;
use crate::rendering::composition::drawing::{RenderDataResource, RenderDataResources, NULL_HANDLE};
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{Point, Rect};
use std::rc::Rc;

fn new_object() -> Rc<dyn ICustomDrawOperation> {
    TestCustomOperation::new()
}

fn resource(object: &Rc<dyn ICustomDrawOperation>) -> Option<RenderDataResource> {
    Some(RenderDataResource::CustomDrawOperation(object.clone()))
}

fn is_same(resource: Option<&RenderDataResource>, object: &Rc<dyn ICustomDrawOperation>) -> bool {
    matches!(resource, Some(RenderDataResource::CustomDrawOperation(stored)) if Rc::ptr_eq(stored, object))
}

#[test]
fn intern_null_returns_null_handle() {
    let mut resources = RenderDataResources::default();
    assert_eq!(NULL_HANDLE, resources.intern(None));
    assert_eq!(0, resources.count());
}

#[test]
fn intern_same_reference_returns_same_handle() {
    let mut resources = RenderDataResources::default();
    let obj = new_object();

    let first = resources.intern(resource(&obj));
    let second = resources.intern(resource(&obj));

    assert_eq!(first, second);
    assert_eq!(1, resources.count());
}

#[test]
fn intern_distinct_references_return_distinct_handles() {
    let mut resources = RenderDataResources::default();
    let a = resources.intern(resource(&new_object()));
    let b = resources.intern(resource(&new_object()));

    assert_ne!(a, b);
    assert_eq!(2, resources.count());
}

#[test]
fn intern_equal_but_distinct_references_return_distinct_handles() {
    let mut resources = RenderDataResources::default();
    let a = resources.intern(resource(&equal_by_value()));
    let b = resources.intern(resource(&equal_by_value()));

    assert_ne!(a, b);
    assert_eq!(2, resources.count());
}

#[test]
fn indexer_returns_interned_resource() {
    let mut resources = RenderDataResources::default();
    let obj = new_object();

    let handle = resources.intern(resource(&obj));

    assert!(is_same(resources.get(handle), &obj));
}

#[test]
fn indexer_null_handle_returns_null() {
    let mut resources = RenderDataResources::default();
    resources.intern(resource(&new_object()));

    assert!(resources.get(NULL_HANDLE).is_none());
}

#[test]
fn append_deserialized_appends_without_deduplication() {
    let mut resources = RenderDataResources::default();
    let obj = new_object();

    let first = resources.append_deserialized(RenderDataResource::CustomDrawOperation(obj.clone()));
    let second = resources.append_deserialized(RenderDataResource::CustomDrawOperation(obj.clone()));

    assert_ne!(first, second);
    assert_eq!(2, resources.count());
    assert!(is_same(resources.get(first), &obj));
    assert!(is_same(resources.get(second), &obj));
}

#[test]
fn dispose_resets_the_table() {
    let mut resources = RenderDataResources::default();
    resources.intern(resource(&new_object()));

    resources.dispose();

    assert_eq!(0, resources.count());
}

/// `new EqualByValue()`: an object equal to every other object of its
/// kind. Custom draw operations compare with `equals`.
fn equal_by_value() -> Rc<dyn ICustomDrawOperation> {
    struct EqualByValue;

    impl ICustomDrawOperation for EqualByValue {
        fn bounds(&self) -> Rect {
            Rect::default()
        }
        fn hit_test(&self, _p: Point) -> bool {
            false
        }
        fn render(&self, _context: &mut crate::media::ImmediateDrawingContext<'_>) {}
        // Equal to every operation: only `EqualByValue` instances are
        // compared in the test.
        fn equals(&self, _other: &dyn ICustomDrawOperation) -> bool {
            true
        }
        fn dispose(&self) {}
    }

    Rc::new(EqualByValue)
}
