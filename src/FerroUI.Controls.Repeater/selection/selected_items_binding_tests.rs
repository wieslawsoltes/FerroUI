// Additional regressions for native binding and collection adapters of the
// Repeater selection views. The upstream IVectorView interface supplies these
// index/count/iteration entry points through its Windows projection.

use super::*;
use ferroui_controls::templates::FuncDataTemplate;
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::{Border, Control, IItemsList, ItemsSource, TextBlock};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::data::ReflectionBinding;
use ferroui_base::metadata::{MarkupTyped, MarkupValue};
use ferroui_base::{BoxedValue, FerroObjectExtensions};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct CountingSource {
    reads: Cell<usize>,
}
impl IItemsList for CountingSource {
    fn count(&self) -> usize {
        10
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self.reads.set(self.reads.get() + 1);
        Some(Rc::new(index as i32))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    Some(Rc::new(value))
}

#[test]
fn selected_views_bind_count_and_index_without_materializing_unread_items() {
    let _scope = test_scope();
    register_markup_types();
    let source = Rc::new(CountingSource {
        reads: Cell::new(0),
    });
    let model = SelectionModel::new();
    model.set_source(Some(ItemsSource::new(source.clone())));
    model.select_all_flat();
    assert_eq!(source.reads.get(), 0);
    let selected = model.selected_items();
    let metadata = <SelectedItemsView as MarkupTyped>::MARKUP;
    let instance = boxed(selected.clone());
    let count = (metadata.find_property("Count").unwrap().get.unwrap())(&[instance.clone()])
        .unwrap()
        .unwrap();
    assert_eq!(count.downcast_ref::<i32>(), Some(&10));
    assert_eq!(source.reads.get(), 0);
    let item = (metadata.find_indexer(1).unwrap().get.unwrap())(&[instance, boxed(3_i32)])
        .unwrap()
        .unwrap();
    assert_eq!(item.downcast_ref::<i32>(), Some(&3));
    assert_eq!(source.reads.get(), 1);
    let indices = model.selected_indices();
    let indexer = <SelectedIndicesView as MarkupTyped>::MARKUP
        .find_indexer(1)
        .unwrap();
    let index = (indexer.get.unwrap())(&[boxed(indices), boxed(4_i32)])
        .unwrap()
        .unwrap();
    assert_eq!(
        index.downcast_ref::<IndexPath>(),
        Some(&IndexPath::create_from(4))
    );
    assert_eq!(source.reads.get(), 1);
    let value: BoxedValue = Rc::new(selected);
    let converted = ValueTypes::try_convert(Some(&value), ValueType::of::<ItemsSource>())
        .unwrap()
        .unwrap();
    let adapted = converted.downcast_ref::<ItemsSource>().unwrap();
    assert_eq!(adapted.count(), 10);
    assert_eq!(source.reads.get(), 1);
    assert_eq!(adapted.get_at(5).unwrap().downcast_ref::<i32>(), Some(&5));
    assert_eq!(source.reads.get(), 2);
}

#[test]
fn selection_count_binding_updates_through_native_property_notifications() {
    let _scope = test_scope();
    register_markup_types();
    let model = SelectionModel::new();
    model.set_source(Some(ItemsSource::from_values(0..3)));
    let text = TextBlock::new();
    text.set_data_context(Some(Rc::new(model.clone())));
    text.bind_binding(
        TextBlock::text_property().as_property(),
        &*ReflectionBinding::new("SelectedItems.Count"),
    );
    assert_eq!(text.text().as_deref(), Some("0"));
    model.select(1).unwrap();
    assert_eq!(text.text().as_deref(), Some("1"));
    model.select(2).unwrap();
    assert_eq!(text.text().as_deref(), Some("2"));
    model.clear_selection();
    assert_eq!(text.text().as_deref(), Some("0"));
}

#[test]
fn removing_source_refreshes_selected_item_and_count_bindings_once() {
    let _scope = test_scope();
    register_markup_types();
    let model = SelectionModel::new();
    model.set_source(Some(ItemsSource::from_values([10_i32, 20, 30])));
    let text = TextBlock::new();
    text.set_data_context(Some(Rc::new(model.clone())));
    text.bind_binding(
        TextBlock::text_property().as_property(),
        &*ReflectionBinding::new("SelectedItems.Count"),
    );
    let selected = Border::new();
    selected.set_data_context(Some(Rc::new(model.clone())));
    selected.bind_binding(
        Control::tag_property().as_property(),
        &*ReflectionBinding::new("SelectedItem"),
    );
    model.select(1).unwrap();
    model.select(2).unwrap();
    assert_eq!(text.text().as_deref(), Some("2"));
    assert_eq!(selected.tag().unwrap().downcast_ref::<i32>(), Some(&20));

    let names = Rc::new(RefCell::new(Vec::new()));
    let changed = names.clone();
    model.property_changed().add(Rc::new(move |name| {
        changed.borrow_mut().push(name.to_string());
    }));
    let events = Rc::new(Cell::new(0));
    let called = events.clone();
    model.selection_changed(move |_| called.set(called.get() + 1));
    model.set_source(None);

    assert_eq!(text.text().as_deref(), Some("0"));
    assert!(selected.tag().is_none());
    assert_eq!(events.get(), 1);
    assert_eq!(*names.borrow(), [
        "AnchorIndex", "SelectedIndex", "SelectedIndices", "SelectedItem", "SelectedItems", "Source",
    ]);
    names.borrow_mut().clear();
    model.select(5).unwrap();
    assert_eq!(*names.borrow(), ["AnchorIndex", "SelectedIndex", "SelectedIndices"]);
    assert_eq!(text.text().as_deref(), Some("0"));
    assert!(selected.tag().is_none());
}

#[test]
fn selected_items_binding_drives_a_native_repeater_and_refreshes_on_selection() {
    let _scope = test_scope();
    register_markup_types();
    let model = SelectionModel::new();
    model.set_source(Some(ItemsSource::from_values([10_i32, 20, 30])));
    let repeater = crate::ItemsRepeater::new();
    repeater.set_data_context(Some(Rc::new(model.clone())));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let child = Border::new();
            child.set_width(50.0);
            child.set_height(20.0);
            Some(child.upcast())
        },
        true,
    )));
    repeater.bind_binding(
        crate::ItemsRepeater::items_source_property().as_property(),
        &*ReflectionBinding::new("SelectedItems"),
    );
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    assert_eq!(repeater.items_source_view().unwrap().count(), 0);
    model.select(2).unwrap();
    root.layout_manager().execute_layout_pass();
    assert_eq!(repeater.items_source_view().unwrap().count(), 1);
    let item: ferroui_base::Ref<Control> = repeater.try_get_element(0).unwrap();
    assert_eq!(
        item.data_context().unwrap().downcast_ref::<i32>(),
        Some(&30)
    );
    model.select(0).unwrap();
    root.layout_manager().execute_layout_pass();
    assert_eq!(repeater.items_source_view().unwrap().count(), 2);
    assert_eq!(
        repeater
            .try_get_element(0)
            .unwrap()
            .data_context()
            .unwrap()
            .downcast_ref::<i32>(),
        Some(&10)
    );
    assert_eq!(
        repeater
            .try_get_element(1)
            .unwrap()
            .data_context()
            .unwrap()
            .downcast_ref::<i32>(),
        Some(&30)
    );

    model.set_source(None);
    root.layout_manager().execute_layout_pass();
    assert_eq!(repeater.items_source_view().unwrap().count(), 0);
    assert!(repeater.realized_elements().is_empty());
    model.set_source(Some(ItemsSource::from_values([40_i32])));
    model.select(0).unwrap();
    root.layout_manager().execute_layout_pass();
    assert_eq!(repeater.items_source_view().unwrap().count(), 1);
    assert_eq!(
        repeater.try_get_element(0).unwrap().data_context().unwrap().downcast_ref::<i32>(),
        Some(&40)
    );
}
