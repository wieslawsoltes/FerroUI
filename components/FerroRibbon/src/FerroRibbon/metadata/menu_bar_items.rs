//! Original get-only ObservableCollection<RibbonMenuBarItem>, preserving its System handle.
use crate::controls::{RibbonMenuBarItem, RibbonMenuBarItems};
use ferroui_base::{
    Ref,
    data::{core::ValueTypes, model::ModelTypes},
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_system::collections::specialized::INotifyCollectionChanged;
use std::{any::TypeId, rc::Rc};
pub struct RibbonMenuBarItemsMetadata;
fn index(value: i32) -> usize {
    usize::try_from(value).expect("ArgumentOutOfRange: index")
}
ferroui_base::ferro_markup_type!(class RibbonMenuBarItemsMetadata as "ObservableCollection`1" {
 namespace:"System.Collections.ObjectModel",this:RibbonMenuBarItems,handles:[RibbonMenuBarItems],generic:"ObservableCollection`1" [Option<Ref<RibbonMenuBarItem>>],notify_property_changed:RibbonMenuBarItems,
 interfaces:[Rc<dyn INotifyCollectionChanged>],constructors:[()=>RibbonMenuBarItems::new_managed],
 properties:[Count:i32{get:|v:&RibbonMenuBarItems|i32::try_from(v.count()).expect("Int32 Count")}],
 indexers:[(i32)->Option<Ref<RibbonMenuBarItem>>{get:|v:&RibbonMenuBarItems,i|v.get(index(i)),set:|v:&RibbonMenuBarItems,i,x|v.set(index(i),x)}],
 methods:[fn Add(Option<Ref<RibbonMenuBarItem>>)=>|v:&RibbonMenuBarItems,x|v.add(x),fn Clear()=>|v:&RibbonMenuBarItems|v.clear(),fn Insert(i32,Option<Ref<RibbonMenuBarItem>>)=>|v:&RibbonMenuBarItems,i,x|v.insert(index(i),x),fn Remove(Option<Ref<RibbonMenuBarItem>>)->bool=>|v:&RibbonMenuBarItems,x|v.remove(&x),fn RemoveAt(i32)=>|v:&RibbonMenuBarItems,i|v.remove_at(index(i)),fn Move(i32,i32)=>|v:&RibbonMenuBarItems,a,b|v.move_item(index(a),index(b))]
});
pub(crate) fn register() {
    MarkupType::register_all(&[RibbonMenuBarItemsMetadata::MARKUP]);
    if !ModelTypes::is_registered(TypeId::of::<RibbonMenuBarItems>()) {
        ModelTypes::register::<RibbonMenuBarItems>(|b| {
            b.notify_property_changed().notify_collection_changed()
        });
    }
    ValueTypes::register_erased_value::<ferroribbon_core::RibbonString>();
}
