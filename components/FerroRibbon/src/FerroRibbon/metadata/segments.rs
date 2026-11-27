//! Original get-only ObservableCollection<RibbonSegment>, preserving its System handle.
use crate::controls::{RibbonSegment, RibbonSegments};
use ferroui_base::{
    Ref,
    data::{core::ValueTypes, model::ModelTypes},
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_system::collections::specialized::INotifyCollectionChanged;
use std::{any::TypeId, rc::Rc};
pub struct RibbonSegmentsMetadata;
fn index(value: i32) -> usize {
    usize::try_from(value).expect("ArgumentOutOfRange: index")
}
ferroui_base::ferro_markup_type!(class RibbonSegmentsMetadata as "ObservableCollection`1" {
 namespace:"System.Collections.ObjectModel",this:RibbonSegments,handles:[RibbonSegments],generic:"ObservableCollection`1" [Option<Ref<RibbonSegment>>],notify_property_changed:RibbonSegments,
 interfaces:[Rc<dyn INotifyCollectionChanged>],constructors:[()=>RibbonSegments::new_managed],
 properties:[Count:i32{get:|v:&RibbonSegments|i32::try_from(v.count()).expect("Int32 Count")}],
 indexers:[(i32)->Option<Ref<RibbonSegment>>{get:|v:&RibbonSegments,i|v.get(index(i)),set:|v:&RibbonSegments,i,x|v.set(index(i),x)}],
 methods:[fn Add(Option<Ref<RibbonSegment>>)=>|v:&RibbonSegments,x|v.add(x),fn Clear()=>|v:&RibbonSegments|v.clear(),fn Insert(i32,Option<Ref<RibbonSegment>>)=>|v:&RibbonSegments,i,x|v.insert(index(i),x),fn Remove(Option<Ref<RibbonSegment>>)->bool=>|v:&RibbonSegments,x|v.remove(&x),fn RemoveAt(i32)=>|v:&RibbonSegments,i|v.remove_at(index(i)),fn Move(i32,i32)=>|v:&RibbonSegments,a,b|v.move_item(index(a),index(b))]
});
pub(crate) fn register() {
    MarkupType::register_all(&[RibbonSegmentsMetadata::MARKUP]);
    if !ModelTypes::is_registered(TypeId::of::<RibbonSegments>()) {
        ModelTypes::register::<RibbonSegments>(|b| {
            b.notify_property_changed().notify_collection_changed()
        });
    }
    ValueTypes::register_erased_value::<ferroribbon_core::RibbonString>();
}
