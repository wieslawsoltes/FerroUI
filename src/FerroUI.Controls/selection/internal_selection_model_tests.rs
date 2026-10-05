use super::{ISelectionModel, InternalSelectionModel, WritableSelectedItems};
use crate::items_source::{box_item, items_equal, IItemsList, ItemsChangedHandler, ItemsSource};
use crate::utils::CollectionUtils;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

fn item(value: &str) -> Option<BoxedValue> {
    box_item(&value.to_string())
}

fn items(values: &[&str]) -> Vec<Option<BoxedValue>> {
    values.iter().map(|v| item(v)).collect()
}

fn source(values: &[&str]) -> Option<ItemsSource> {
    Some(ItemsSource::from_strs(values.iter().copied()))
}

fn create_target() -> Rc<InternalSelectionModel> {
    create_target_with(false, None, false)
}

fn create_target_with(single_select: bool, source: Option<ItemsSource>, null_source: bool) -> Rc<InternalSelectionModel> {
    let source = source.or_else(|| if !null_source { self::source(&["foo", "bar", "baz"]) } else { None });

    let result = InternalSelectionModel::new();
    result.set_single_select(single_select);

    ISelectionModel::set_source(&*result, source);
    result
}

#[test]
fn selecting_item_adds_to_writable_selected_items() {
    let target = create_target();

    target.select(0);

    assert_eq!(target.writable_selected_items().to_vec(), items(&["foo"]));
}

#[test]
fn selecting_duplicate_on_model_adds_to_writable_selected_items() {
    let target = create_target_with(false, source(&["foo", "bar", "baz", "foo", "bar", "baz"]), false);

    target.select_range(1, 4);

    assert_eq!(target.writable_selected_items().to_vec(), items(&["bar", "baz", "foo", "bar"]));
}

#[test]
fn deselecting_on_model_removes_selected_item() {
    let target = create_target();

    target.select_range(1, 2);
    target.deselect(1);

    assert_eq!(target.writable_selected_items().to_vec(), items(&["baz"]));
}

#[test]
fn deselecting_duplicate_on_model_removes_selected_item() {
    let target = create_target_with(false, source(&["foo", "bar", "baz", "foo", "bar", "baz"]), false);

    target.select_range(1, 2);
    target.select(4);
    target.deselect(4);

    assert_eq!(target.writable_selected_items().to_vec(), items(&["baz", "bar"]));
}

#[test]
fn adding_to_writable_selected_items_selects_on_model() {
    let target = create_target();

    target.select_range(1, 2);
    target.writable_selected_items().add(item("foo"));

    assert_eq!(target.selected_indexes().to_vec(), vec![0, 1, 2]);
    assert_eq!(target.writable_selected_items().to_vec(), items(&["bar", "baz", "foo"]));
}

#[test]
fn removing_from_writable_selected_items_deselects_on_model() {
    let target = create_target();

    target.select_range(1, 2);
    target.writable_selected_items().remove(&item("baz"));

    assert_eq!(target.selected_indexes().to_vec(), vec![1]);
    assert_eq!(target.writable_selected_items().to_vec(), items(&["bar"]));
}

#[test]
fn replacing_selected_item_updates_model() {
    let target = create_target();

    target.select_range(1, 2);
    target.writable_selected_items().set(0, item("foo"));

    assert_eq!(target.selected_indexes().to_vec(), vec![0, 2]);
    assert_eq!(target.writable_selected_items().to_vec(), items(&["foo", "baz"]));
}

#[test]
fn clearing_writable_selected_items_updates_model() {
    let target = create_target();

    target.writable_selected_items().clear();

    assert!(target.selected_indexes().is_empty());
    assert!(target.writable_selected_items().is_empty());
}

#[test]
fn setting_writable_selected_items_updates_model() {
    let target = create_target();
    let old_items = target.writable_selected_items();

    let new_items: WritableSelectedItems = Rc::new(FerroList::from_items(items(&["foo", "baz"])));
    target.set_writable_selected_items(Some(new_items.clone()));

    assert_eq!(target.selected_indexes().to_vec(), vec![0, 2]);
    assert!(Rc::ptr_eq(&new_items, &target.writable_selected_items()));
    assert!(!Rc::ptr_eq(&old_items, &target.writable_selected_items()));
    assert_eq!(new_items.to_vec(), items(&["foo", "baz"]));
}

#[test]
fn setting_items_to_null_clears_selection() {
    let target = create_target();

    target.select_range(1, 2);
    target.set_writable_selected_items(None);

    assert!(target.selected_indexes().is_empty());
    assert!(target.writable_selected_items().is_empty());
}

#[test]
fn setting_items_to_null_creates_empty_items() {
    let target = create_target();
    let old_items = target.writable_selected_items();

    target.set_writable_selected_items(None);

    // The list is a notifying list of untyped items by its type.
    let new_items: WritableSelectedItems = target.writable_selected_items();
    assert!(!Rc::ptr_eq(&old_items, &new_items));
}

#[test]
fn adds_null_writable_selected_items_when_source_is_null() {
    let target = create_target_with(false, None, true);

    target.select_range(1, 2);
    assert_eq!(target.writable_selected_items().to_vec(), vec![None, None]);
}

#[test]
fn updates_writable_selected_items_when_source_changes_from_null() {
    let target = create_target_with(false, None, true);

    target.select_range(1, 2);
    assert_eq!(target.writable_selected_items().to_vec(), vec![None, None]);

    target.set_source(source(&["foo", "bar", "baz"]));
    assert_eq!(target.writable_selected_items().to_vec(), items(&["bar", "baz"]));
}

#[test]
fn updates_writable_selected_items_when_source_changes_to_null() {
    let target = create_target();

    target.select_range(1, 2);
    assert_eq!(target.writable_selected_items().to_vec(), items(&["bar", "baz"]));

    target.set_source(None);
    assert_eq!(target.writable_selected_items().to_vec(), vec![None, None]);
}

#[test]
fn writable_selected_items_can_be_set_before_source() {
    let target = create_target_with(false, None, true);
    let items_list = Rc::new(FerroList::from_items(["foo", "bar", "baz"].map(str::to_string)));
    let writable_selected_items: WritableSelectedItems = Rc::new(FerroList::from_items(items(&["bar"])));

    target.set_writable_selected_items(Some(writable_selected_items));
    target.set_source(Some(items_list.into()));

    assert_eq!(target.selected_index(), 1);
    assert_eq!(target.writable_selected_items().to_vec(), items(&["bar"]));
}

#[test]
fn restores_selection_on_items_reset() {
    let items_list = Rc::new(ResettingCollection::new(&["foo", "bar", "baz"]));
    let target = create_target_with(false, Some(ItemsSource::new(items_list.clone())), false);

    target.set_selected_index(1);
    items_list.reset(&["baz", "foo", "bar"]);

    assert_eq!(target.selected_index(), 2);
    assert_eq!(target.writable_selected_items().to_vec(), items(&["bar"]));
}

#[test]
fn raises_selection_changed_on_items_reset() {
    let items_list = Rc::new(ResettingCollection::new(&["foo", "bar", "baz"]));
    let target = create_target_with(false, Some(ItemsSource::new(items_list.clone())), false);

    target.set_selected_index(1);

    let changed = Rc::new(RefCell::new(Vec::<String>::new()));

    let c = changed.clone();
    target.property_changed().add(Rc::new(move |name: &str| c.borrow_mut().push(name.to_string())));

    let old_selected_index = target.selected_index();
    let old_selected_item = target.selected_item();

    items_list.reset(&[]);

    assert_ne!(old_selected_index, target.selected_index());
    assert_ne!(old_selected_item, target.selected_item());

    assert_eq!(target.selected_index(), -1);
    assert_eq!(target.selected_item(), None);
    assert!(target.writable_selected_items().is_empty());

    assert!(changed.borrow().iter().any(|name| name == "SelectedIndex"));
    assert!(changed.borrow().iter().any(|name| name == "SelectedItem"));
}

#[test]
fn preserves_selected_item_on_items_reset() {
    let items_list = Rc::new(ResettingCollection::new(&["foo", "bar", "baz"]));
    let target = create_target_with(false, Some(ItemsSource::new(items_list.clone())), false);

    target.set_selected_item(item("foo"));

    assert_eq!(target.selected_index(), 0);

    items_list.reset(&["baz", "foo", "bar"]);

    assert_eq!(target.selected_item(), item("foo"));
    assert_eq!(target.selected_index(), 1);
    assert_eq!(target.writable_selected_items().to_vec(), items(&["foo"]));
}

#[test]
fn preserves_selection_on_source_changed() {
    let target = create_target();

    target.set_selected_index(1);
    target.set_source(source(&["baz", "foo", "bar"]));

    assert_eq!(target.selected_index(), 2);
    assert_eq!(target.writable_selected_items().to_vec(), items(&["bar"]));
}

/// A list that only notifies of resets.
struct ResettingCollection {
    items: RefCell<Vec<Option<BoxedValue>>>,
    collection_changed: HandlerList<ItemsChangedHandler>,
}

impl ResettingCollection {
    fn new(values: &[&str]) -> Self {
        Self { items: RefCell::new(items(values)), collection_changed: HandlerList::new() }
    }

    fn reset(&self, values: &[&str]) {
        *self.items.borrow_mut() = items(values);

        for (_, handler) in self.collection_changed.snapshot().iter() {
            handler(&CollectionUtils::RESET_EVENT_ARGS);
        }
    }
}

impl IItemsList for ResettingCollection {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self.items.borrow()[index].clone()
    }

    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        self.items.borrow().iter().position(|i| items_equal(i, item)).map_or(-1, |i| i as i32)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.collection_changed.add(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.collection_changed.remove(token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
