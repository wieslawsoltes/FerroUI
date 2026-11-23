//! Source QAT string collections retain UTF-16 in Rust. Native markup string
//! conversions reject unpaired surrogates instead of replacing them.
use ferroribbon_core::{RibbonString, collections::RibbonReadOnlyList};
use ferroribbon_core::collections::ObservableCollection;
pub type RibbonQuickAccessCandidates = ObservableCollection<Option<RibbonString>>;
pub type RibbonQuickAccessItemIds = RibbonReadOnlyList<Option<RibbonString>>;
pub struct RibbonQuickAccessCandidatesMetadata;
pub struct RibbonQuickAccessItemIdsMetadata;
ferroui_base::ferro_markup_type!(class RibbonQuickAccessCandidatesMetadata as "ObservableCollection`1"{
 namespace:"System.Collections.ObjectModel",this:RibbonQuickAccessCandidates,handles:[RibbonQuickAccessCandidates],generic:"ObservableCollection`1" [Option<String>],
 interfaces:[std::rc::Rc<dyn ferroui_system::collections::specialized::INotifyCollectionChanged>],notify_property_changed:RibbonQuickAccessCandidates,
 properties:[Count:i32{get:|v:&RibbonQuickAccessCandidates|i32::try_from(v.count()).expect("Int32 Count")}],
 indexers:[(i32)->Option<String>{try_get:|v:&RibbonQuickAccessCandidates,i|v.get(usize::try_from(i).expect("index")).map(|s|s.to_utf8()).transpose(),set:|v:&RibbonQuickAccessCandidates,i,s:Option<String>|v.set(usize::try_from(i).expect("index"),s.map(Into::into))}],
 methods:[fn Add(Option<String>)=>|v:&RibbonQuickAccessCandidates,s:Option<String>|v.add(s.map(Into::into)),fn Clear()=>|v:&RibbonQuickAccessCandidates|v.clear(),fn Remove(Option<String>)->bool=>|v:&RibbonQuickAccessCandidates,s:Option<String>|v.remove(&s.map(Into::into)),fn RemoveAt(i32)=>|v:&RibbonQuickAccessCandidates,i|v.remove_at(usize::try_from(i).expect("index")),fn Insert(i32,Option<String>)=>|v:&RibbonQuickAccessCandidates,i,s:Option<String>|v.insert(usize::try_from(i).expect("index"),s.map(Into::into)),fn Move(i32,i32)=>|v:&RibbonQuickAccessCandidates,a,b|v.move_item(usize::try_from(a).expect("index"),usize::try_from(b).expect("index"))]
});
ferroui_base::ferro_markup_type!(interface RibbonQuickAccessItemIdsMetadata as "IReadOnlyList`1"{
 namespace:"System.Collections.Generic",this:RibbonQuickAccessItemIds,handles:[RibbonQuickAccessItemIds,Option<RibbonQuickAccessItemIds>],generic:"IReadOnlyList`1" [Option<String>],properties:[Count:i32{get:|v:&RibbonQuickAccessItemIds|v.count()}],indexers:[(i32)->Option<String>{try_get:|v:&RibbonQuickAccessItemIds,i|v.get(i).map(|s|s.to_utf8()).transpose()}]
});
pub(crate) fn register() {
    use ferroui_base::metadata::{MarkupType, MarkupTyped};
    ferroui_base::data::model::ModelTypes::register_notifying_collection::<
        RibbonQuickAccessCandidates,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<RibbonQuickAccessItemIds>();
    MarkupType::register_all(&[
        RibbonQuickAccessCandidatesMetadata::MARKUP,
        RibbonQuickAccessItemIdsMetadata::MARKUP,
    ]);
}
