// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root.
// Native collection and metadata adapters for the source IVectorView contract.

use super::{IndexPath, SelectedItems, SelectionError};
use ferroui_controls::{IItemsList, ItemsSource};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{ferro_markup_type, BoxedValue, PropertyValue};
use std::rc::Rc;

/// The lazy selected-data view exposed to native bindings and item controls.
pub type SelectedItemsView = SelectedItems<Option<BoxedValue>>;
/// The lazy selected-index view exposed to native bindings and item controls.
pub type SelectedIndicesView = SelectedItems<IndexPath>;

impl<T: PropertyValue> IItemsList for SelectedItems<T> {
    fn count(&self) -> usize {
        SelectedItems::count(self)
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        // A view read after the selection changed has no item at a stale index.
        let value = SelectedItems::get_at(self, index).ok()?;
        ferroui_controls::box_item(&value)
    }
    // Deviation (winui-repeater/DEVIATIONS.md, WR-37): the source SelectedItems vector view
    // rejects IndexOf; as an items source of a selecting control (SelectedItem, ScrollIntoView)
    // the view answers it with the default linear search instead of failing.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn item_at<T>(view: &SelectedItems<T>, index: i32) -> Result<T, SelectionError> {
    usize::try_from(index)
        .map_err(|_| SelectionError::IndexOutOfRange)
        .and_then(|index| view.get_at(index))
}

ferro_markup_type!(class SelectedItemsView {
    namespace: "FerroUI.Controls.Repeater",
    this: SelectedItemsView,
    handles: [SelectedItemsView],
    properties: [
        Count: i32 { get: |view: &SelectedItemsView| i32::try_from(view.count()).expect("Selection count exceeds the layout index range.") },
        Size: i32 { get: |view: &SelectedItemsView| i32::try_from(view.count()).expect("Selection count exceeds the layout index range.") },
    ],
    indexers: [(i32) -> Option<BoxedValue> { try_get: item_at::<Option<BoxedValue>> }],
    methods: [
        try fn GetAt(i32) -> Option<BoxedValue> => item_at::<Option<BoxedValue>>,
    ],
});
ferro_markup_type!(class SelectedIndicesView {
    namespace: "FerroUI.Controls.Repeater",
    this: SelectedIndicesView,
    handles: [SelectedIndicesView],
    properties: [
        Count: i32 { get: |view: &SelectedIndicesView| i32::try_from(view.count()).expect("Selection count exceeds the layout index range.") },
        Size: i32 { get: |view: &SelectedIndicesView| i32::try_from(view.count()).expect("Selection count exceeds the layout index range.") },
    ],
    indexers: [(i32) -> IndexPath { try_get: item_at::<IndexPath> }],
    methods: [try fn GetAt(i32) -> IndexPath => item_at::<IndexPath>],
});

const TYPES: &[&MarkupType] = &[
    <SelectedItemsView as MarkupTyped>::MARKUP,
    <SelectedIndicesView as MarkupTyped>::MARKUP,
];

pub(super) fn register_markup_types() {
    MarkupType::register_all(TYPES);
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    ItemsSource::register_binding_conversion_with::<SelectedItemsView>(|view| {
        ItemsSource::new(Rc::new(view.clone()))
    });
    ItemsSource::register_binding_conversion_with::<SelectedIndicesView>(|view| {
        ItemsSource::new(Rc::new(view.clone()))
    });
}
