// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root.

use super::{
    IndexPath, SelectedItems, SelectionModel, SelectionModelChildrenRequestedEventArgs,
    SelectionModelSelectionChangedEventArgs,
};
use ferroui_controls::ItemsSource;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupType, MarkupTyped};
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::rc::Rc;

// FerroUI metadata replaces WinRT ICustomPropertyProvider. In particular SelectedItem remains
// lazy and notifies through the framework's INotifyPropertyChanged contract.
ferro_markup_type!(class SelectionModel {
    namespace: "FerroUI.Controls.Repeater",
    handles: [Rc<SelectionModel>, Option<Rc<SelectionModel>>],
    this: Rc<SelectionModel>,
    constructors: [() => SelectionModel::new],
    notify_property_changed: SelectionModel,
    properties: [
        Source: Option<ItemsSource> {
            get: |model: &Rc<SelectionModel>| model.source(),
            set: |model: &Rc<SelectionModel>, source: Option<ItemsSource>| model.set_source(source)
        },
        SingleSelect: bool {
            get: |model: &Rc<SelectionModel>| model.single_select(),
            set: |model: &Rc<SelectionModel>, single: bool| model.set_single_select(single)
        },
        AnchorIndex: Option<IndexPath> {
            get: |model: &Rc<SelectionModel>| model.anchor_index(),
            try_set: |model: &Rc<SelectionModel>, path: Option<IndexPath>| model.set_anchor_index(path)
        },
        SelectedIndex: Option<IndexPath> {
            get: |model: &Rc<SelectionModel>| model.selected_index(),
            try_set: |model: &Rc<SelectionModel>, path: Option<IndexPath>| model.set_selected_index(path)
        },
        SelectedItem: Option<BoxedValue> { get: |model: &Rc<SelectionModel>| model.selected_item() },
        SelectedItems: SelectedItems<Option<BoxedValue>> { get: |model: &Rc<SelectionModel>| model.selected_items() },
        SelectedIndices: SelectedItems<IndexPath> { get: |model: &Rc<SelectionModel>| model.selected_indices() },
    ],
    methods: [
        try fn SetAnchorIndex(i32) => |model: &Rc<SelectionModel>, index: i32| model.set_anchor_index_flat(index),
        try fn SetAnchorIndex(i32, i32) => |model: &Rc<SelectionModel>, group: i32, item: i32| model.set_anchor_index_with_group(group, item),
        try fn Select(i32) => |model: &Rc<SelectionModel>, index: i32| model.select(index),
        try fn Select(i32, i32) => |model: &Rc<SelectionModel>, group: i32, item: i32| model.select_with_group(group, item),
        try fn Deselect(i32) => |model: &Rc<SelectionModel>, index: i32| model.deselect(index),
        try fn Deselect(i32, i32) => |model: &Rc<SelectionModel>, group: i32, item: i32| model.deselect_with_group(group, item),
        try fn SelectAt(IndexPath) => |model: &Rc<SelectionModel>, path: IndexPath| model.select_at(&path),
        try fn DeselectAt(IndexPath) => |model: &Rc<SelectionModel>, path: IndexPath| model.deselect_at(&path),
        fn IsSelected(i32) -> Option<bool> => |model: &Rc<SelectionModel>, index: i32| model.is_selected(index),
        fn IsSelected(i32, i32) -> Option<bool> => |model: &Rc<SelectionModel>, group: i32, item: i32| model.is_selected_with_group(group, item),
        try fn IsSelectedAt(IndexPath) -> Option<bool> => |model: &Rc<SelectionModel>, path: IndexPath| model.is_selected_at(&path),
        try fn SelectRangeFromAnchor(i32) => |model: &Rc<SelectionModel>, index: i32| model.select_range_from_anchor(index),
        try fn SelectRangeFromAnchor(i32, i32) => |model: &Rc<SelectionModel>, group: i32, item: i32| model.select_range_from_anchor_with_group(group, item),
        try fn DeselectRangeFromAnchor(i32) => |model: &Rc<SelectionModel>, index: i32| model.deselect_range_from_anchor(index),
        try fn DeselectRangeFromAnchor(i32, i32) => |model: &Rc<SelectionModel>, group: i32, item: i32| model.deselect_range_from_anchor_with_group(group, item),
        try fn SelectRangeFromAnchorTo(IndexPath) => |model: &Rc<SelectionModel>, path: IndexPath| model.select_range_from_anchor_to(&path),
        try fn DeselectRangeFromAnchorTo(IndexPath) => |model: &Rc<SelectionModel>, path: IndexPath| model.deselect_range_from_anchor_to(&path),
        try fn SelectRange(IndexPath, IndexPath) => |model: &Rc<SelectionModel>, start: IndexPath, end: IndexPath| model.select_range(&start, &end),
        try fn DeselectRange(IndexPath, IndexPath) => |model: &Rc<SelectionModel>, start: IndexPath, end: IndexPath| model.deselect_range(&start, &end),
        try fn SelectAll() => |model: &Rc<SelectionModel>| model.select_all(),
        fn SelectAllFlat() => |model: &Rc<SelectionModel>| model.select_all_flat(),
        fn ClearSelection() => |model: &Rc<SelectionModel>| model.clear_selection(),
    ],
    events: [
        SelectionChanged(Option<Rc<SelectionModel>>, SelectionModelSelectionChangedEventArgs) => |model: &Rc<SelectionModel>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(model);
            model.selection_changed(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(*args)]); })
        },
        ChildrenRequested(Option<Rc<SelectionModel>>, Rc<SelectionModelChildrenRequestedEventArgs>) => |model: &Rc<SelectionModel>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(model);
            model.children_requested(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
    ],
});

ferro_markup_type!(struct IndexPath {
    namespace: "FerroUI.Controls.Repeater",
    handles: [IndexPath, Option<IndexPath>],
    constructors: [() => IndexPath::default, (i32) => IndexPath::create_from, (i32, i32) => IndexPath::create_from_group_and_item_index, (Vec<i32>) => IndexPath::create_from_indices],
    methods: [
        static fn CreateFrom(i32) -> IndexPath => IndexPath::create_from,
        static fn CreateFrom(i32, i32) -> IndexPath => IndexPath::create_from_group_and_item_index,
        static fn CreateFromGroupAndItemIndex(i32, i32) -> IndexPath => IndexPath::create_from_group_and_item_index,
        static fn CreateFromIndices(Vec<i32>) -> IndexPath => IndexPath::create_from_indices,
        fn GetSize() -> i32 => IndexPath::get_size,
        try fn GetAt(i32) -> i32 => IndexPath::get_at,
        fn CompareTo(IndexPath) -> i32 => |path: &IndexPath, other: IndexPath| path.compare_to(&other),
        fn ToString() -> String => IndexPath::to_string,
    ],
});

ferro_markup_type!(class SelectionModelChildrenRequestedEventArgs {
    namespace: "FerroUI.Controls.Repeater",
    handles: [Rc<SelectionModelChildrenRequestedEventArgs>],
    this: Rc<SelectionModelChildrenRequestedEventArgs>,
    properties: [
        Source: Option<BoxedValue> { try_get: |args: &Rc<SelectionModelChildrenRequestedEventArgs>| args.source() },
        SourceIndex: IndexPath { try_get: |args: &Rc<SelectionModelChildrenRequestedEventArgs>| args.source_index() },
        Children: Option<ItemsSource> {
            get: |args: &Rc<SelectionModelChildrenRequestedEventArgs>| args.children(),
            set: |args: &Rc<SelectionModelChildrenRequestedEventArgs>, value: Option<ItemsSource>| args.set_children(value)
        },
    ],
});

ferro_markup_type!(struct SelectionModelSelectionChangedEventArgs {
    namespace: "FerroUI.Controls.Repeater",
    handles: [SelectionModelSelectionChangedEventArgs],
});

const TYPES: &[&MarkupType] = &[
    <SelectionModel as MarkupTyped>::MARKUP,
    <IndexPath as MarkupTyped>::MARKUP,
    <SelectionModelChildrenRequestedEventArgs as MarkupTyped>::MARKUP,
    <SelectionModelSelectionChangedEventArgs as MarkupTyped>::MARKUP,
];

pub fn register_markup_types() {
    MarkupType::register_all(TYPES);
    ValueTypes::register_global(register_value_types);
    super::selected_items_markup::register_markup_types();
}

fn register_value_types() {
    ValueTypes::register_nullable::<Rc<SelectionModel>>();
    ValueTypes::register_nullable::<IndexPath>();
    ValueTypes::register_display::<IndexPath>();
}
