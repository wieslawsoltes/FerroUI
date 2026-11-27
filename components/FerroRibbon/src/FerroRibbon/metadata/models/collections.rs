//! Core model collection metadata; closed instantiations of existing System collections.
use super::*;
use ferroribbon_core::RibbonString;
use ferroribbon_core::model::RibbonBackstageItemModelImpl;

pub type ModelColors = ObservableCollection<ferroribbon_core::theming::RibbonColor>;
pub type ModelSwatches =
    ObservableCollection<Option<Rc<ferroribbon_core::theming::RibbonColorSwatch>>>;
pub type ModelSegments =
    ObservableCollection<Option<Rc<dyn ferroribbon_core::model::RibbonSegmentModelImpl>>>;
pub type ModelItems =
    ObservableCollection<Option<Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>>>;
pub type ModelNodes =
    ObservableCollection<Option<Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>>>;
pub type ModelTabs =
    ObservableCollection<Option<Rc<dyn ferroribbon_core::model::RibbonTabModelImpl>>>;
pub type ModelGroups =
    ObservableCollection<Option<Rc<dyn ferroribbon_core::model::RibbonGroupModelImpl>>>;
pub type ModelContextualGroups =
    ObservableCollection<Option<Rc<dyn ferroribbon_core::model::RibbonContextualGroupModelImpl>>>;
pub type ModelBackstageItems = ObservableCollection<Option<Rc<dyn RibbonBackstageItemModelImpl>>>;
pub type ModelGalleryItems =
    ObservableCollection<Option<Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>>>;
macro_rules! ribbon_model_collection_metadata {
 ($marker:ident,$items:ident,$element:ty)=>{
 pub struct $marker;
 ferroui_base::ferro_markup_type!(class $marker as "ObservableCollection`1" {
 namespace:"System.Collections.ObjectModel",this:$items,handles:[$items],generic:"ObservableCollection`1" [$element],
 interfaces:[Rc<dyn ferroui_system::collections::specialized::INotifyCollectionChanged>],notify_property_changed:$items,
 constructors:[()=>$items::new_managed],
 properties:[Count:i32{get:|v:&$items|i32::try_from(v.count()).expect("Int32 Count")}],
 indexers:[(i32)->$element{get:|v:&$items,i|v.get(index(i)),set:|v:&$items,i,x|v.set(index(i),x)}],
 methods:[fn Add($element)=>|v:&$items,x|v.add(x),fn Clear()=>|v:&$items|v.clear(),fn Insert(i32,$element)=>|v:&$items,i,x|v.insert(index(i),x),fn Remove($element)->bool=>|v:&$items,x|v.remove(&x),fn RemoveAt(i32)=>|v:&$items,i|v.remove_at(index(i)),fn Move(i32,i32)=>|v:&$items,a,b|v.move_item(index(a),index(b)),fn Contains($element)->bool=>|v:&$items,x|v.contains(&x)]
 });
 };
}
ribbon_model_collection_metadata!(
    ModelColorsMetadata,
    ModelColors,
    ferroribbon_core::theming::RibbonColor
);
ribbon_model_collection_metadata!(
    ModelSwatchesMetadata,
    ModelSwatches,
    Option<Rc<ferroribbon_core::theming::RibbonColorSwatch>>
);
ribbon_model_collection_metadata!(
    ModelSegmentsMetadata,
    ModelSegments,
    Option<Rc<dyn ferroribbon_core::model::RibbonSegmentModelImpl>>
);
ribbon_model_collection_metadata!(
    ModelItemsMetadata,
    ModelItems,
    Option<Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>>
);
ribbon_model_collection_metadata!(
    ModelNodesMetadata,
    ModelNodes,
    Option<Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>>
);
ribbon_model_collection_metadata!(
    ModelTabsMetadata,
    ModelTabs,
    Option<Rc<dyn ferroribbon_core::model::RibbonTabModelImpl>>
);
ribbon_model_collection_metadata!(
    ModelGroupsMetadata,
    ModelGroups,
    Option<Rc<dyn ferroribbon_core::model::RibbonGroupModelImpl>>
);
ribbon_model_collection_metadata!(
    ModelContextualGroupsMetadata,
    ModelContextualGroups,
    Option<Rc<dyn ferroribbon_core::model::RibbonContextualGroupModelImpl>>
);
ribbon_model_collection_metadata!(
    ModelBackstageItemsMetadata,
    ModelBackstageItems,
    Option<Rc<dyn RibbonBackstageItemModelImpl>>
);
ribbon_model_collection_metadata!(
    ModelGalleryItemsMetadata,
    ModelGalleryItems,
    Option<Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>>
);
pub type ModelStrings = ObservableCollection<Option<RibbonString>>;
pub struct ModelStringsMetadata;
ferroui_base::ferro_markup_type!(class ModelStringsMetadata as "ObservableCollection`1" {
 namespace:"System.Collections.ObjectModel",this:ModelStrings,handles:[ModelStrings],generic:"ObservableCollection`1" [Option<String>],notify_property_changed:ModelStrings,
 interfaces:[Rc<dyn ferroui_system::collections::specialized::INotifyCollectionChanged>],constructors:[()=>ModelStrings::new_managed],
 properties:[Count:i32{get:|v:&ModelStrings|i32::try_from(v.count()).expect("Int32 Count")}],
 indexers:[(i32)->Option<String>{try_get:|v:&ModelStrings,i|string(v.get(index(i))),set:|v:&ModelStrings,i,x:Option<String>|v.set(index(i),x.map(Into::into))}],
 methods:[fn Add(Option<String>)=>|v:&ModelStrings,x:Option<String>|v.add(x.map(Into::into)),fn Clear()=>|v:&ModelStrings|v.clear(),fn Insert(i32,Option<String>)=>|v:&ModelStrings,i,x:Option<String>|v.insert(index(i),x.map(Into::into)),fn RemoveAt(i32)=>|v:&ModelStrings,i|v.remove_at(index(i)),fn Move(i32,i32)=>|v:&ModelStrings,a,b|v.move_item(index(a),index(b)),fn Contains(Option<String>)->bool=>|v:&ModelStrings,x:Option<String>|v.contains(&x.map(Into::into)),fn Remove(Option<String>)->bool=>|v:&ModelStrings,x:Option<String>|v.remove(&x.map(Into::into))]
});
