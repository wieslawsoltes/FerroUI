//! Closed ObservableCollection<RibbonBackstageItem> using native System storage.
use crate::controls::RibbonBackstageItem;
use ferroui_base::Ref;
use ferroui_system::collections::object_model::ObservableCollection;
pub type RibbonBackstageItems = ObservableCollection<Option<Ref<RibbonBackstageItem>>>;
pub struct RibbonBackstageItemsMetadata;
ferroui_base::ferro_markup_type!(class RibbonBackstageItemsMetadata as "ObservableCollection`1"{
 namespace:"System.Collections.ObjectModel",this:RibbonBackstageItems,handles:[RibbonBackstageItems],generic:"ObservableCollection`1" [Option<Ref<RibbonBackstageItem>>],
 interfaces:[std::rc::Rc<dyn ferroui_system::collections::specialized::INotifyCollectionChanged>],notify_property_changed:RibbonBackstageItems,
 properties:[Count:i32{get:|items:&RibbonBackstageItems|i32::try_from(items.count()).expect("Int32 Count")}],
 indexers:[(i32)->Option<Ref<RibbonBackstageItem>>{get:|items:&RibbonBackstageItems,i|items.get(usize::try_from(i).expect("index")),set:|items:&RibbonBackstageItems,i,item|items.set(usize::try_from(i).expect("index"),item)}],
 methods:[fn Add(Option<Ref<RibbonBackstageItem>>)=>|items:&RibbonBackstageItems,item|items.add(item),fn Clear()=>|items:&RibbonBackstageItems|items.clear(),fn Remove(Option<Ref<RibbonBackstageItem>>)->bool=>|items:&RibbonBackstageItems,item|items.remove(&item),fn RemoveAt(i32)=>|items:&RibbonBackstageItems,i|items.remove_at(usize::try_from(i).expect("index")),fn Insert(i32,Option<Ref<RibbonBackstageItem>>)=>|items:&RibbonBackstageItems,i,item|items.insert(usize::try_from(i).expect("index"),item),fn Move(i32,i32)=>|items:&RibbonBackstageItems,from,to|items.move_item(usize::try_from(from).expect("index"),usize::try_from(to).expect("index")),fn Contains(Option<Ref<RibbonBackstageItem>>)->bool=>|items:&RibbonBackstageItems,item|items.contains(&item)]
});
pub(crate) fn register() {
    use ferroui_base::metadata::{MarkupType, MarkupTyped};
    ferroui_base::data::model::ModelTypes::register_notifying_collection::<RibbonBackstageItems>();
    MarkupType::register_all(&[RibbonBackstageItemsMetadata::MARKUP]);
}
