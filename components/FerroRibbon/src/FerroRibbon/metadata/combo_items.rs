//! Source object collection/array metadata, retaining native collection notifications/identity.
use crate::controls::{RibbonObjectArray, RibbonObjectItems};
use ferroui_base::{
    BoxedValue,
    data::model::ModelTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
fn index(v: i32) -> usize {
    usize::try_from(v).expect("ArgumentOutOfRange: index")
}
/// Shared source ObservableCollection<object> storage for controls and Core models.
pub type ModelObjects = RibbonObjectItems;
pub type RibbonObjectEnumerable = ferroribbon_core::model::RibbonItemsSource<Option<BoxedValue>>;
fn item(v: Option<ferroribbon_core::model::RibbonObject>) -> Result<Option<BoxedValue>, String> {
    super::model_objects::to_native(v).map_err(|error| error.to_string())
}
fn get(v: &RibbonObjectItems, i: i32) -> Result<Option<BoxedValue>, String> {
    item(v.get(index(i)))
}
fn set(v: &RibbonObjectItems, i: i32, x: Option<BoxedValue>) {
    v.set(
        index(i),
        super::model_objects::from_native_replacing(x, || v.get(index(i))),
    )
}
pub struct RibbonObjectItemsMetadata;
ferroui_base::ferro_markup_type!(class RibbonObjectItemsMetadata as "ObservableCollection`1" {
 namespace:"System.Collections.ObjectModel",this:RibbonObjectItems,handles:[RibbonObjectItems],generic:"ObservableCollection`1" [Option<BoxedValue>],
 interfaces:[RibbonObjectEnumerable,Rc<dyn ferroui_system::collections::specialized::INotifyCollectionChanged>],notify_property_changed:RibbonObjectItems,
 constructors:[()=>RibbonObjectItems::new_managed],properties:[Count:i32{get:|v:&RibbonObjectItems|i32::try_from(v.count()).expect("Int32 Count overflow")}],
 indexers:[(i32)->Option<BoxedValue>{try_get:get,set:set}],
 methods:[fn Add(Option<BoxedValue>)=>|v:&RibbonObjectItems,x|v.add(super::model_objects::from_native(x)),fn Clear()=>|v:&RibbonObjectItems|v.clear(),fn RemoveAt(i32)=>|v:&RibbonObjectItems,i|v.remove_at(index(i)),fn Insert(i32,Option<BoxedValue>)=>|v:&RibbonObjectItems,i,x|v.insert(index(i),super::model_objects::from_native(x)),fn Move(i32,i32)=>|v:&RibbonObjectItems,a,b|v.move_item(index(a),index(b)),fn Remove(Option<BoxedValue>)->bool=>|v:&RibbonObjectItems,x|v.remove(&super::model_objects::from_native(x)),fn Contains(Option<BoxedValue>)->bool=>|v:&RibbonObjectItems,x|v.contains(&super::model_objects::from_native(x))]
});
pub struct RibbonObjectEnumerableMetadata;
ferroui_base::ferro_markup_type!(interface RibbonObjectEnumerableMetadata as "IEnumerable`1" {
 namespace:"System.Collections.Generic",this:RibbonObjectEnumerable,handles:[RibbonObjectEnumerable,Option<RibbonObjectEnumerable>],generic:"IEnumerable`1" [Option<BoxedValue>]
});
/// Exact model constructor enumeration, once, retaining original payloads.
pub fn new_combo_model(
    id: Option<String>,
    label: Option<String>,
    items: Option<RibbonObjectEnumerable>,
) -> Rc<dyn ferroribbon_core::model::RibbonComboBoxModelImpl> {
    ferroribbon_core::model::RibbonComboBoxModel::with_items(
        id.map(Into::into),
        label.map(Into::into),
        items
            .into_iter()
            .flat_map(|q| q.iter())
            .map(super::model_objects::from_native),
    )
}
pub struct RibbonObjectArrayMetadata;
ferroui_base::ferro_markup_type!(class RibbonObjectArrayMetadata as "IReadOnlyList`1"{namespace:"System.Collections.Generic",this:RibbonObjectArray,handles:[RibbonObjectArray],generic:"IReadOnlyList`1" [Option<BoxedValue>],properties:[Count:i32{get:|v:&RibbonObjectArray|i32::try_from(v.len()).expect("Int32 Count overflow")}],indexers:[(i32)->Option<BoxedValue>{get:|v:&RibbonObjectArray,i|v.get(index(i))}]});
pub struct RibbonFontNamesMetadata;
ferroui_base::ferro_markup_type!(class RibbonFontNamesMetadata as "IReadOnlyList`1"{namespace:"System.Collections.Generic",this:ferroribbon_core::collections::RibbonReadOnlyList<String>,handles:[ferroribbon_core::collections::RibbonReadOnlyList<String>],generic:"IReadOnlyList`1" [String],properties:[Count:i32{get:|v:&ferroribbon_core::collections::RibbonReadOnlyList<String>|v.count()}],indexers:[(i32)->String{get:|v:&ferroribbon_core::collections::RibbonReadOnlyList<String>,i|{index(i);v.get(i)}}]});
pub struct RibbonFontSizesMetadata;
ferroui_base::ferro_markup_type!(class RibbonFontSizesMetadata as "IReadOnlyList`1"{namespace:"System.Collections.Generic",this:ferroribbon_core::collections::RibbonReadOnlyList<f64>,handles:[ferroribbon_core::collections::RibbonReadOnlyList<f64>],generic:"IReadOnlyList`1" [f64],properties:[Count:i32{get:|v:&ferroribbon_core::collections::RibbonReadOnlyList<f64>|v.count()}],indexers:[(i32)->f64{get:|v:&ferroribbon_core::collections::RibbonReadOnlyList<f64>,i|{index(i);v.get(i)}}]});
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    ValueTypes::register_nullable::<RibbonObjectEnumerable>();
    ValueTypes::register_cast::<RibbonObjectItems, RibbonObjectEnumerable>(|v| {
        let v = v.clone();
        RibbonObjectEnumerable::new(move || {
            Box::new(v.iter().map(|v| item(v).expect("Core collection payload")))
        })
    });
    ValueTypes::register_cast::<RibbonObjectArray, RibbonObjectEnumerable>(|v| {
        let v = v.clone();
        RibbonObjectEnumerable::new(move || Box::new(v.iter()))
    });
    ModelTypes::register_notifying_collection::<RibbonObjectItems>();
    MarkupType::register_all(&[
        RibbonObjectItemsMetadata::MARKUP,
        RibbonObjectEnumerableMetadata::MARKUP,
        RibbonObjectArrayMetadata::MARKUP,
        RibbonFontNamesMetadata::MARKUP,
        RibbonFontSizesMetadata::MARKUP,
        crate::controls::RibbonComboBoxCommittedEventArgs::MARKUP,
    ]);
}
