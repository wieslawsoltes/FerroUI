//! Concrete metadata for the source ObservableCollection<UIElement> item lists.
//! Actual storage/notifications/iteration are FerroUI.System's existing collection.
use ferroui_base::Ref;
use ferroui_controls::Control;
use ferroui_system::collections::object_model::ObservableCollection;
pub type RibbonControlItems = ObservableCollection<Option<Ref<Control>>>;
#[doc(hidden)]
pub struct RibbonControlItemsMetadata;
fn index(value: i32) -> usize {
    usize::try_from(value).expect("ArgumentOutOfRange: index")
}
ferroui_base::ferro_markup_type!(class RibbonControlItemsMetadata as "ObservableCollection`1" {
 namespace:"System.Collections.ObjectModel",handles:[RibbonControlItems],this:RibbonControlItems,
 generic:"ObservableCollection`1" [Option<Ref<Control>>],
 interfaces:[std::rc::Rc<dyn ferroui_system::collections::specialized::INotifyCollectionChanged>],
 notify_property_changed:RibbonControlItems,
 properties:[Count:i32{get:|items:&RibbonControlItems|i32::try_from(items.count()).expect("collection exceeds Int32 Count")}],
 indexers:[(i32)->Option<Ref<Control>>{get:|items:&RibbonControlItems,i|items.get(index(i)),set:|items:&RibbonControlItems,i,value|items.set(index(i),value)}],
 methods:[
 fn Add(Option<Ref<Control>>)=>|items:&RibbonControlItems,item|items.add(item),
 fn Clear()=>|items:&RibbonControlItems|items.clear(),
 fn Insert(i32,Option<Ref<Control>>)=>|items:&RibbonControlItems,i,item|items.insert(index(i),item),
 fn Remove(Option<Ref<Control>>)->bool=>|items:&RibbonControlItems,item|items.remove(&item),
 fn RemoveAt(i32)=>|items:&RibbonControlItems,i|items.remove_at(index(i)),
 fn Move(i32,i32)=>|items:&RibbonControlItems,from,to|items.move_item(index(from),index(to)),
 fn Contains(Option<Ref<Control>>)->bool=>|items:&RibbonControlItems,item|items.contains(&item),
 ],
});
pub(crate) fn register() {
    use ferroui_base::metadata::{MarkupType, MarkupTyped};
    ferroui_base::data::model::ModelTypes::register_notifying_collection::<RibbonControlItems>();
    MarkupType::register_all(&[<RibbonControlItemsMetadata as MarkupTyped>::MARKUP]);
}
