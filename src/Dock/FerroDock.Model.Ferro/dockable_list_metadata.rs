// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native IList metadata retaining the original typed model collection and events.
use ferrodock_model::{collections::ListHandle, core::IDockable};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Descriptor carrier for the original IList<IDockable> handle.
#[doc(hidden)]
pub trait DockableListMetadata {}
ferroui_base::ferro_markup_type!(interface dyn DockableListMetadata as "IList(IDockable)"{
    namespace:"System.Collections.Generic",this:ListHandle<Option<Rc<dyn IDockable>>>,handles:[ListHandle<Option<Rc<dyn IDockable>>>,Option<ListHandle<Option<Rc<dyn IDockable>>>>],
    properties:[
        Count:i32{try_get:|list:&ListHandle<Option<Rc<dyn IDockable>>>|Ok::<i32,ferroui_base::metadata::MarkupInvokeError>(i32::try_from(list.count()).expect("source IList Count fits Int32"))},
        IsReadOnly:bool{get:|list:&ListHandle<Option<Rc<dyn IDockable>>>|list.is_read_only()},
    ],
    indexers:[(i32)->Option<Rc<dyn IDockable>>{get:|list:&ListHandle<Option<Rc<dyn IDockable>>>,i:i32|list.get(usize::try_from(i).expect("source nonnegative list index")),set:|list:&ListHandle<Option<Rc<dyn IDockable>>>,i:i32,value:Option<Rc<dyn IDockable>>|list.set(usize::try_from(i).expect("source nonnegative list index"),value)}],
    methods:[
        fn Add(Option<Rc<dyn IDockable>>) => (|list:&ListHandle<Option<Rc<dyn IDockable>>>,value|list.add(value)),
        fn Insert(i32,Option<Rc<dyn IDockable>>) => (|list:&ListHandle<Option<Rc<dyn IDockable>>>,index:i32,value|list.insert(usize::try_from(index).expect("source nonnegative list index"),value)),
        fn Remove(Option<Rc<dyn IDockable>>)->bool => (|list:&ListHandle<Option<Rc<dyn IDockable>>>,value|list.remove(&value)),
        fn RemoveAt(i32) => (|list:&ListHandle<Option<Rc<dyn IDockable>>>,index:i32|list.remove_at(usize::try_from(index).expect("source nonnegative list index"))),
        fn Clear() => (|list:&ListHandle<Option<Rc<dyn IDockable>>>|list.clear()),
        fn Contains(Option<Rc<dyn IDockable>>)->bool => (|list:&ListHandle<Option<Rc<dyn IDockable>>>,value|list.contains(&value)),
        fn IndexOf(Option<Rc<dyn IDockable>>)->i32 => (|list:&ListHandle<Option<Rc<dyn IDockable>>>,value|list.index_of(&value).map_or(-1,|i|i as i32)),
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn DockableListMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(list) = value.downcast_ref::<ListHandle<Option<Rc<dyn IDockable>>>>() {
        return list.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<ListHandle<Option<Rc<dyn IDockable>>>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    // Cloned handles denote the original list, even when boxed independently.
    ferroui_base::data::core::ValueTypes::register_reference_identity::<
        ListHandle<Option<Rc<dyn IDockable>>>,
    >(|list| list.identity() as *const ());
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::DockableListMetadata", true)]);
    }
}
