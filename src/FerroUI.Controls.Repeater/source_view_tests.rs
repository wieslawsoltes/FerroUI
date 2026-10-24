// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.
// Behavioral ports of Repeater/APITests/InspectingDataSourceTests.cs.

use super::*;
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::{Border, Control, ItemsChangedEventArgs, ItemsSource};
use ferroui_base::collections::FerroList;
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[repr(C)]
struct SourceStackLayout {
    base: NonVirtualizingLayout,
}
ferroui_base::ferro_class!(SourceStackLayout: NonVirtualizingLayout);
ferroui_base::ferro_impl_classes!(SourceStackLayout: ferroui_base::FerroObjectImpl, LayoutImpl);
impl NonVirtualizingLayoutImpl for SourceStackLayout {
    fn measure_override(
        _this: &Self,
        context: &ferroui_base::Ref<NonVirtualizingLayoutContext>,
        available: ferroui_base::Size,
    ) -> ferroui_base::Size {
        let children = context.children();
        for child in &children {
            child.measure(available);
        }
        ferroui_base::Size::new(100.0, 20.0 * children.len() as f64)
    }
    fn arrange_override(
        _this: &Self,
        context: &ferroui_base::Ref<NonVirtualizingLayoutContext>,
        size: ferroui_base::Size,
    ) -> ferroui_base::Size {
        for (index, child) in context.children().iter().enumerate() {
            child.arrange(ferroui_base::Rect::new(
                0.0,
                20.0 * index as f64,
                size.width,
                20.0,
            ));
        }
        size
    }
}
impl SourceStackLayout {
    fn new() -> ferroui_base::Ref<Self> {
        ferroui_base::instantiate(Self {
            base: NonVirtualizingLayout::construct(),
        })
    }
}

// InspectingDataSourceTests: native collection traits replace the WinRT vector,
// vector-view and enumerable projections; the notification payloads are retained.
struct InspectingSource {
    items: RefCell<Vec<Option<BoxedValue>>>,
    changes: ferroui_base::utilities::HandlerList<ferroui_controls::ItemsChangedHandler>,
    keyed: bool,
}
impl InspectingSource {
    fn new(items: ItemsSource, keyed: bool) -> Rc<Self> {
        Rc::new(Self {
            items: RefCell::new(items.to_vec()),
            changes: ferroui_base::utilities::HandlerList::new(),
            keyed,
        })
    }
    fn add(&self, value: Option<BoxedValue>) {
        let index = self.items.borrow().len();
        self.items.borrow_mut().push(value.clone());
        let items = [value];
        self.notify(&ItemsChangedEventArgs::add(&items, index as i32));
    }
    fn reset(&self) {
        self.notify(&ItemsChangedEventArgs::RESET);
    }
    fn replace_source(&self, items: ItemsSource) {
        *self.items.borrow_mut() = items.to_vec();
        self.reset();
    }
    fn notify(&self, args: &ItemsChangedEventArgs<'_>) {
        for (_, handler) in self.changes.snapshot().iter() {
            handler(args);
        }
    }
}
impl ferroui_controls::IItemsList for InspectingSource {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self.items.borrow()[index].clone()
    }
    fn key_index_mapping(&self) -> Option<&dyn ferroui_controls::IKeyIndexMapping> {
        self.keyed.then_some(self)
    }
    fn is_notifying(&self) -> bool {
        true
    }
    fn add_collection_changed(
        &self,
        handler: Rc<ferroui_controls::ItemsChangedHandler>,
    ) -> Option<u64> {
        Some(self.changes.add(handler))
    }
    fn remove_collection_changed(&self, token: u64) {
        self.changes.remove(token);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl ferroui_controls::IKeyIndexMapping for InspectingSource {
    fn key_from_index(&self, index: i32) -> String {
        self.items.borrow()[index as usize]
            .as_ref()
            .unwrap()
            .downcast_ref::<i32>()
            .unwrap()
            .to_string()
    }
    fn index_from_key(&self, key: &str) -> i32 {
        self.items
            .borrow()
            .iter()
            .position(|item| {
                item.as_ref()
                    .and_then(|item| item.downcast_ref::<i32>())
                    .map(i32::to_string)
                    .as_deref()
                    == Some(key)
            })
            .map_or(-1, |index| index as i32)
    }
}

#[test]
fn verify_items_source_view_rejects_invalid_sources() {
    use ferroui_base::data::core::{ValueType, ValueTypes};
    let _scope = test_scope();
    // ItemsSourceView::new takes a nonnullable ItemsSource. Untyped binding
    // conversion rejects the same invalid values before construction.
    let target = ValueType::of::<ItemsSource>();
    assert!(ValueTypes::try_convert(None, target).is_none());
    for invalid in [Rc::new(42_i32) as BoxedValue, Control::boxed(Border::new())] {
        assert!(ValueTypes::try_convert(Some(&invalid), target).is_none());
    }
    let source = ItemsSource::from_values([7_i32, 8]);
    let boxed: BoxedValue = Rc::new(source.clone());
    assert!(ValueTypes::try_convert(Some(&boxed), target).is_some());
    let view = ferroui_controls::ItemsSourceView::new(source);
    assert_eq!(view.count(), 2);
    assert_eq!(
        ferroui_controls::unbox_item::<i32>(&view.get_at(1)),
        Some(8)
    );
}

#[test]
fn verify_key_index_mapping_not_implemented_without_mapping_source() {
    let _scope = test_scope();
    let view = ferroui_controls::ItemsSourceView::new(Rc::new(FerroList::from_items(0..5)).into());
    assert!(!view.has_key_index_mapping());
    assert!(catch_unwind(AssertUnwindSafe(|| view.key_from_index(0))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| view.index_from_key("0"))).is_err());
}

#[test]
fn can_create_from_i_bindable_iterable() {
    let _scope = test_scope();
    let view = ferroui_controls::ItemsSourceView::new(ItemsSource::from_values(0..100));
    assert_eq!(view.count(), 100);
    assert_eq!(
        ferroui_controls::unbox_item::<i32>(&view.get_at(4)),
        Some(4)
    );
}

fn verify_observable_vector_changes() {
    use ferroui_base::collections::NotifyCollectionChangedAction as Action;
    let data = Rc::new(FerroList::from_items(
        (0..100).map(|index| format!("Item #{index}")),
    ));
    let view = ferroui_controls::ItemsSourceView::new(data.clone().into());
    assert_eq!(view.count(), 100);
    assert_eq!(
        ferroui_controls::unbox_item::<String>(&view.get_at(4)).as_deref(),
        Some("Item #4")
    );
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    view.add_collection_changed(Rc::new(move |args| {
        observed.borrow_mut().push((
            args.action,
            args.old_starting_index,
            args.old_items.len(),
            args.new_starting_index,
            args.new_items.len(),
        ));
    }));
    data.insert(4, "Inserted Item".to_owned());
    data.remove_at(7);
    data.set(15, "Replaced Item".to_owned());
    data.clear();
    assert_eq!(view.count(), 0);
    assert_eq!(
        &*changes.borrow(),
        &[
            (Action::Add, -1, 0, 4, 1),
            (Action::Remove, 7, 1, -1, 0),
            (Action::Replace, 15, 1, 15, 1),
            (Action::Reset, -1, 0, -1, 0),
        ]
    );
}

// The upstream attribute is commented out (issue 24022837). The native
// observable-list adapter runs the complete source notification assertions.
#[test]
fn can_create_from_incc_i_bindable_vector() {
    let _scope = test_scope();
    verify_observable_vector_changes();
}

// The pinned Windows projection test is ignored; native observable vectors
// use the existing FerroList collection-change adapter.
#[test]
fn can_create_from_i_observable_vector() {
    let _scope = test_scope();
    verify_observable_vector_changes();
}

#[test]
fn verify_unique_id_mapping_interface() {
    let _scope = test_scope();
    let source = InspectingSource::new(ItemsSource::from_values(0..10), true);
    let view = ferroui_controls::ItemsSourceView::new(ItemsSource::new(source));
    assert_eq!(view.count(), 10);
    assert!(view.has_key_index_mapping());
    assert_eq!(view.index_from_key("5"), 5);
    assert_eq!(view.key_from_index(5), "5");
}

#[test]
fn verify_index_of_behavior() {
    let _scope = test_scope();
    let keyed = InspectingSource::new(ItemsSource::from_values(0..10), true);
    let sources = [
        ItemsSource::new(keyed),
        Rc::new(FerroList::from_items(0..10)).into(),
    ];
    for source in sources {
        let view = ferroui_controls::ItemsSourceView::new(source);
        for index in 0..10 {
            assert_eq!(view.index_of(&Some(Rc::new(index))), index);
        }
        assert_eq!(view.index_of(&Some(Rc::new(11_i32))), -1);
    }
    // The native from_values adapter explicitly materializes an iterator into
    // an indexed source, so its IndexOf is available for every materialized item.
    // This retains the existing FerroUI contract instead of a WinRT limitation.
    let view = ferroui_controls::ItemsSourceView::new(ItemsSource::from_values(0..10));
    assert_eq!(view.index_of(&Some(Rc::new(0_i32))), 0);
    let view = ferroui_controls::ItemsSourceView::new(ItemsSource::from_items([
        Some(Rc::new("text".to_owned()) as BoxedValue),
        None,
        Some(Rc::new("foobar".to_owned()) as BoxedValue),
        Some(Rc::new("FerroUI is awesome".to_owned()) as BoxedValue),
    ]));
    assert_eq!(view.index_of(&None), 1);
}

#[test]
fn verify_calling_reset_multiple_times_on_unique_id_items_source() {
    let _scope = test_scope();
    let source = InspectingSource::new(ItemsSource::from_values(0..5), true);
    let repeater = ItemsRepeater::new();
    repeater.set_items_source(Some(ItemsSource::new(source.clone())));
    let viewer = ferroui_controls::ScrollViewer::new();
    viewer.set_template(Some(ferroui_controls::templates::FuncControlTemplate::for_type::<
        ferroui_controls::ScrollViewer,
    >(crate::test_support::create_template)));
    viewer.set_width(400.0);
    viewer.set_height(400.0);
    viewer.set_content(Some(Control::boxed(repeater.clone())));
    let root = TestRoot::with_child(&viewer);
    root.execute_initial_layout_pass();
    let before: Vec<_> = (0..5)
        .map(|index| repeater.try_get_element(index).unwrap())
        .collect();
    source.reset();
    source.reset();
    root.layout_manager().execute_layout_pass();
    assert_eq!(repeater.items_source_view().unwrap().count(), 5);
    for (index, element) in before.into_iter().enumerate() {
        assert_eq!(repeater.try_get_element(index as i32), Some(element));
    }
}

fn switching_source_refreshes_elements(virtualizing: bool) {
    let repeater = ItemsRepeater::new();
    if !virtualizing {
        repeater.set_layout(Some(SourceStackLayout::new().upcast()));
    }
    repeater.set_items_source(Some(ItemsSource::from_values(0..10)));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    for index in 0..10 {
        assert_eq!(
            repeater
                .try_get_element(index)
                .unwrap()
                .cast::<ferroui_controls::TextBlock>()
                .unwrap()
                .text(),
            Some(index.to_string())
        );
    }
    repeater.set_items_source(Some(ItemsSource::from_values(20..30)));
    root.layout_manager().execute_layout_pass();
    assert_eq!(repeater.items_source_view().unwrap().count(), 10);
    for index in 0..10 {
        assert_eq!(
            repeater
                .try_get_element(index)
                .unwrap()
                .cast::<ferroui_controls::TextBlock>()
                .unwrap()
                .text(),
            Some((index + 20).to_string())
        );
    }
}

#[test]
fn validate_switching_items_source_refreshes_elements_non_virtual_layout() {
    let _scope = test_scope();
    switching_source_refreshes_elements(false);
}

#[test]
fn validate_switching_items_source_refreshes_elements_virtual_layout() {
    let _scope = test_scope();
    switching_source_refreshes_elements(true);
}

// Native read-only collection contracts expose only count/get/index and change
// subscriptions. The owning model retains its own mutation methods.
#[test]
fn verify_read_only_list_compatibility() {
    let _scope = test_scope();
    let source = InspectingSource::new(
        ItemsSource::from_strs(["something1", "something2", "something3"]),
        false,
    );
    let view = ferroui_controls::ItemsSourceView::new(ItemsSource::new(source.clone()));
    assert_eq!(view.count(), 3);
    source.add(Some(Rc::new("something4".to_owned())));
    assert_eq!(view.count(), 4);
    assert_eq!(
        ferroui_controls::unbox_item::<String>(&view.get_at(0)).as_deref(),
        Some("something1")
    );
    assert_eq!(view.index_of(&Some(Rc::new("something1".to_owned()))), 0);
}

#[test]
fn verify_notify_collection_change_with_readonly_list_behavior() {
    let _scope = test_scope();
    let source = InspectingSource::new(ItemsSource::from_items([]), false);
    let view = ferroui_controls::ItemsSourceView::new(ItemsSource::new(source.clone()));
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    view.add_collection_changed(Rc::new(move |args| {
        assert_eq!(
            args.action,
            ferroui_base::collections::NotifyCollectionChangedAction::Reset
        );
        observed.set(observed.get() + 1);
    }));
    // The native model adapter emits Reset when its underlying data changes;
    // the original test used an INPC-backed Windows collection projection.
    source.replace_source(ItemsSource::from_values(0..3));
    assert_eq!(changes.get(), 1);
    assert_eq!(view.count(), 3);
}

#[test]
fn verify_read_only_observable_list_source() {
    let _scope = test_scope();
    let source = InspectingSource::new(ItemsSource::from_strs(["A", "B", "C"]), false);
    let view = ferroui_controls::ItemsSourceView::new(ItemsSource::new(source.clone()));
    assert_eq!(view.count(), 3);
    assert_eq!(
        ferroui_controls::unbox_item::<String>(&view.get_at(1)).as_deref(),
        Some("B")
    );
    assert_eq!(view.index_of(&Some(Rc::new("C".to_owned()))), 2);
    assert_eq!(view.index_of(&Some(Rc::new("Z".to_owned()))), -1);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    view.add_collection_changed(Rc::new(move |args| observed.borrow_mut().push(args.action)));
    let repeater = ItemsRepeater::new();
    repeater.set_items_source(Some(ItemsSource::new(source.clone())));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    assert_eq!(repeater.items_source_view().unwrap().count(), 3);
    source.add(Some(Rc::new("D".to_owned())));
    root.layout_manager().execute_layout_pass();
    assert_eq!(view.count(), 4);
    assert_eq!(
        ferroui_controls::unbox_item::<String>(&view.get_at(3)).as_deref(),
        Some("D")
    );
    assert_eq!(
        &*changes.borrow(),
        &[ferroui_base::collections::NotifyCollectionChangedAction::Add]
    );
    assert_eq!(
        repeater
            .try_get_element(3)
            .unwrap()
            .cast::<ferroui_controls::TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("D")
    );
}
