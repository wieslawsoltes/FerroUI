//! `TreeDataGridElementFactoryTests.cs`.

use super::support::start_app;
use crate::primitives::{RecycleKey, TreeDataGridElementFactory, TreeDataGridElementFactoryImpl};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Border, Control, StackPanel};
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

/// `TestElementFactory` of the tests: counts the borders it creates, all with the same recycle key.
struct CountingElementFactory {
    created_count: Rc<Cell<i32>>,
}

impl TreeDataGridElementFactoryImpl for CountingElementFactory {
    fn create_element(&self, _factory: &TreeDataGridElementFactory, _data: &dyn Any) -> Ref<Control> {
        self.created_count.set(self.created_count.get() + 1);
        Border::new().upcast()
    }

    fn get_data_recycle_key(&self, _factory: &TreeDataGridElementFactory, _data: &dyn Any) -> RecycleKey {
        "element".into()
    }

    fn get_element_recycle_key(&self, _factory: &TreeDataGridElementFactory, _element: &Control) -> RecycleKey {
        "element".into()
    }
}

fn counting_factory() -> (Rc<TreeDataGridElementFactory>, Rc<Cell<i32>>) {
    let created_count = Rc::new(Cell::new(0));
    (TreeDataGridElementFactory::with_impl(CountingElementFactory { created_count: created_count.clone() }), created_count)
}

/// `KeyedElementFactory`: the recycle key of data is the data (a string), and of an element its
/// tag.
struct KeyedElementFactory;

impl TreeDataGridElementFactoryImpl for KeyedElementFactory {
    fn get_data_recycle_key(&self, _factory: &TreeDataGridElementFactory, data: &dyn Any) -> RecycleKey {
        data.downcast_ref::<String>().cloned().map(RecycleKey::from).expect("a string")
    }

    fn get_element_recycle_key(&self, _factory: &TreeDataGridElementFactory, element: &Control) -> RecycleKey {
        element.tag().and_then(|tag| tag.downcast_ref::<String>().cloned()).map(RecycleKey::from).expect("a string tag")
    }
}

fn data() -> Rc<()> {
    Rc::new(())
}

#[test]
fn reuses_element_with_same_parent_in_preference_to_fallback() {
    let _app = start_app();
    let (target, created_count) = counting_factory();
    let first_parent = StackPanel::new();
    let requested_parent = StackPanel::new();
    let fallback = target.get_or_create_element(&data(), &first_parent);
    let expected = target.get_or_create_element(&data(), &requested_parent);

    first_parent.children().add(&fallback);
    requested_parent.children().add(&expected);
    target.recycle_element(&fallback);
    target.recycle_element(&expected);

    let actual = target.get_or_create_element(&data(), &requested_parent);

    assert_eq!(expected, actual);
    assert!(actual.parent().is_some_and(|parent| parent.ptr_eq(&requested_parent)));
    assert_eq!(2, created_count.get());
}

#[test]
fn reparents_fallback_element_from_a_panel() {
    let _app = start_app();
    let (target, created_count) = counting_factory();
    let old_parent = StackPanel::new();
    let new_parent = StackPanel::new();
    let element = target.get_or_create_element(&data(), &old_parent);

    old_parent.children().add(&element);
    target.recycle_element(&element);

    let actual = target.get_or_create_element(&data(), &new_parent);

    assert_eq!(element, actual);
    assert!(actual.parent().is_none());
    assert_eq!(0, old_parent.children().count());
    assert_eq!(1, created_count.get());
}

#[test]
fn does_not_retain_an_element_after_it_leaves_the_pool() {
    let _app = start_app();
    let (factory, _) = counting_factory();
    let weak_element = {
        let parent = StackPanel::new();
        let element = factory.get_or_create_element(&data(), &parent);

        factory.recycle_element(&element);
        let element = factory.get_or_create_element(&data(), &parent);
        element.downgrade()
    };

    assert!(weak_element.upgrade().is_none());
    drop(factory);
}

#[test]
fn can_reuse_element_uses_the_factorys_recycle_keys() {
    let _app = start_app();
    let target = TreeDataGridElementFactory::with_impl(KeyedElementFactory);
    let element = Border::new();
    element.set_tag(Some(Rc::new("group".to_string()) as BoxedValue));

    assert!(target.can_reuse_element(&element, &"group".to_string()));
    assert!(!target.can_reuse_element(&element, &"leaf".to_string()));
}
