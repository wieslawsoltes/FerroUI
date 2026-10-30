// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native IList metadata retaining the original typed window collection and events.
use ferrodock_model::{collections::ListHandle, core::IDockWindow};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Descriptor carrier for the original IList<IDockWindow> handle.
#[doc(hidden)]
pub trait DockWindowListMetadata {}
ferroui_base::ferro_markup_type!(interface dyn DockWindowListMetadata as "IList(IDockWindow)"{
    namespace:"System.Collections.Generic",this:ListHandle<Option<Rc<dyn IDockWindow>>>,handles:[ListHandle<Option<Rc<dyn IDockWindow>>>,Option<ListHandle<Option<Rc<dyn IDockWindow>>>>],
    properties:[
        Count:i32{try_get:|list:&ListHandle<Option<Rc<dyn IDockWindow>>>|Ok::<i32,ferroui_base::metadata::MarkupInvokeError>(i32::try_from(list.count()).expect("source IList Count fits Int32"))},
        IsReadOnly:bool{get:|list:&ListHandle<Option<Rc<dyn IDockWindow>>>|list.is_read_only()},
    ],
    indexers:[(i32)->Option<Rc<dyn IDockWindow>>{get:|list:&ListHandle<Option<Rc<dyn IDockWindow>>>,i:i32|list.get(usize::try_from(i).expect("source nonnegative list index")),set:|list:&ListHandle<Option<Rc<dyn IDockWindow>>>,i:i32,value:Option<Rc<dyn IDockWindow>>|list.set(usize::try_from(i).expect("source nonnegative list index"),value)}],
    methods:[
        fn Add(Option<Rc<dyn IDockWindow>>) => (|list:&ListHandle<Option<Rc<dyn IDockWindow>>>,value|list.add(value)),
        fn Insert(i32,Option<Rc<dyn IDockWindow>>) => (|list:&ListHandle<Option<Rc<dyn IDockWindow>>>,index:i32,value|list.insert(usize::try_from(index).expect("source nonnegative list index"),value)),
        fn Remove(Option<Rc<dyn IDockWindow>>)->bool => (|list:&ListHandle<Option<Rc<dyn IDockWindow>>>,value|list.remove(&value)),
        fn RemoveAt(i32) => (|list:&ListHandle<Option<Rc<dyn IDockWindow>>>,index:i32|list.remove_at(usize::try_from(index).expect("source nonnegative list index"))),
        fn Clear() => (|list:&ListHandle<Option<Rc<dyn IDockWindow>>>|list.clear()),
        fn Contains(Option<Rc<dyn IDockWindow>>)->bool => (|list:&ListHandle<Option<Rc<dyn IDockWindow>>>,value|list.contains(&value)),
        fn IndexOf(Option<Rc<dyn IDockWindow>>)->i32 => (|list:&ListHandle<Option<Rc<dyn IDockWindow>>>,value|list.index_of(&value).map_or(-1,|i|i as i32)),
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn DockWindowListMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(list) = value.downcast_ref::<ListHandle<Option<Rc<dyn IDockWindow>>>>() {
        return list.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<ListHandle<Option<Rc<dyn IDockWindow>>>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    // Cloned handles denote the original list, even when boxed independently.
    ferroui_base::data::core::ValueTypes::register_reference_identity::<
        ListHandle<Option<Rc<dyn IDockWindow>>>,
    >(|list| list.identity() as *const ());
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::DockWindowListMetadata", true)]);
    }
}
