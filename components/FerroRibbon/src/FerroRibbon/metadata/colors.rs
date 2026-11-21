//! Actual palette collection and shared read-only dictionary view metadata.
use crate::controls::{RibbonColorNames, RibbonColors};
use ferroui_base::{
    data::{core::ValueTypes, model::ModelTypes},
    media::Color,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_system::collections::specialized::INotifyCollectionChanged;
use std::{any::TypeId, rc::Rc};
pub struct RibbonColorsMetadata;
fn index(i: i32) -> usize {
    usize::try_from(i).expect("ArgumentOutOfRange: index")
}
ferroui_base::ferro_markup_type!(class RibbonColorsMetadata as "ObservableCollection`1" {
 namespace:"System.Collections.ObjectModel",this:RibbonColors,handles:[RibbonColors],generic:"ObservableCollection`1" [Color],notify_property_changed:RibbonColors,interfaces:[Rc<dyn INotifyCollectionChanged>],constructors:[()=>RibbonColors::new_managed],
 properties:[Count:i32{get:|v:&RibbonColors|i32::try_from(v.count()).expect("Int32 Count")}],indexers:[(i32)->Color{get:|v:&RibbonColors,i|v.get(index(i)),set:|v:&RibbonColors,i,x|v.set(index(i),x)}],
 methods:[fn Add(Color)=>|v:&RibbonColors,x|v.add(x),fn Clear()=>|v:&RibbonColors|v.clear(),fn Insert(i32,Color)=>|v:&RibbonColors,i,x|v.insert(index(i),x),fn Remove(Color)->bool=>|v:&RibbonColors,x|v.remove(&x),fn RemoveAt(i32)=>|v:&RibbonColors,i|v.remove_at(index(i)),fn Move(i32,i32)=>|v:&RibbonColors,a,b|v.move_item(index(a),index(b))]
});
pub struct RibbonColorNamesMetadata;
// Keep the existing markup name. Rust consumers use the actual shared palette
// names object; no generic dictionary or object/view conversion is registered.
ferroui_base::ferro_markup_type!(class RibbonColorNamesMetadata as "IReadOnlyDictionary`2"{
 namespace:"System.Collections.Generic",this:Rc<RibbonColorNames>,handles:[RibbonColorNames,Rc<RibbonColorNames>],generic:"IReadOnlyDictionary`2" [Color,Option<String>],properties:[Count:i32{get:|v:&Rc<RibbonColorNames>|i32::try_from(v.len()).expect("Int32 Count")}],indexers:[(Color)->Option<String>{get:|v:&Rc<RibbonColorNames>,c|Some(v.get(&c).expect("KeyNotFound: color"))}],methods:[fn ContainsKey(Color)->bool=>|v:&Rc<RibbonColorNames>,c|v.contains_key(&c)]
});
pub struct RibbonColorSwatchMetadata;
ferroui_base::ferro_markup_type!(class RibbonColorSwatchMetadata as "RibbonColorSwatch" {
 namespace:"FerroRibbon.Theming",this:Rc<ferroribbon_core::theming::RibbonColorSwatch>,handles:[ferroribbon_core::theming::RibbonColorSwatch,Rc<ferroribbon_core::theming::RibbonColorSwatch>,Option<Rc<ferroribbon_core::theming::RibbonColorSwatch>>],
 constructors:[(ferroribbon_core::theming::RibbonColor,Option<String>)=>|color,name:Option<String>|ferroribbon_core::theming::RibbonColorSwatch::new(color,name.map(Into::into))],
 properties:[Color:ferroribbon_core::theming::RibbonColor{get:|v:&Rc<ferroribbon_core::theming::RibbonColorSwatch>|v.color},Name:Option<String>{try_get:|v:&Rc<ferroribbon_core::theming::RibbonColorSwatch>|v.name.as_ref().map(|n|n.to_utf8().map_err(|e|e.to_string())).transpose()}]
});
pub(crate) fn register() {
    ValueTypes::register_reference::<ferroribbon_core::theming::RibbonColorSwatch>();
    ValueTypes::register_nullable::<Rc<ferroribbon_core::theming::RibbonColorSwatch>>();
    ValueTypes::register_reference::<RibbonColorNames>();
    MarkupType::register_all(&[
        RibbonColorSwatchMetadata::MARKUP,
        RibbonColorsMetadata::MARKUP,
        RibbonColorNamesMetadata::MARKUP,
    ]);
    if !ModelTypes::is_registered(TypeId::of::<RibbonColors>()) {
        ModelTypes::register::<RibbonColors>(|b| {
            b.notify_property_changed().notify_collection_changed()
        });
    }
}
