//! Native properties over actual Core model allocations, including mutable XAML configuration.
use ferroboard_core::text::Utf16Text;
use ferroboard_core::{
    keys::LayoutDirection,
    layouts::{
        ArrangementRow, ArrangementRows, ComponentName, Key, KeyboardLayout, LayoutArrangement,
        LayoutMetadata, LayoutType,
    },
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::sync::Arc;
#[derive(Clone)]
pub struct LayoutMetadataSnapshot(Arc<Vec<Arc<LayoutMetadata>>>);
impl PartialEq for LayoutMetadataSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl LayoutMetadataSnapshot {
    pub fn new(items: Vec<Arc<LayoutMetadata>>) -> Self {
        Self(Arc::new(items))
    }
    pub fn count(&self) -> i32 {
        i32::try_from(self.0.len()).expect("source List.Count exceeds i32")
    }
    pub fn get(&self, index: i32) -> Result<Arc<LayoutMetadata>, &'static str> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.0.get(i).cloned())
            .ok_or("index was out of range")
    }
}
#[derive(Clone)]
pub struct LayoutAuthorsList(ferroboard_core::layouts::ResourceTexts);
impl PartialEq for LayoutAuthorsList {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl LayoutAuthorsList {
    pub fn new(source: ferroboard_core::layouts::ResourceTexts) -> Self {
        Self(source)
    }
    pub fn source(&self) -> ferroboard_core::layouts::ResourceTexts {
        self.0.clone()
    }
    pub fn count(&self) -> i32 {
        i32::try_from(self.0.len()).expect("source List.Count exceeds i32")
    }
    pub fn get(&self, index: i32) -> Result<Option<Utf16Text>, String> {
        self.0
            .get(usize::try_from(index).map_err(|_| "index was out of range".to_owned())?)
            .map_err(|e| e.to_string())
    }
}
#[doc(hidden)]
pub struct LayoutMetadataMetadata;
#[doc(hidden)]
pub struct ComponentNameMetadata;
#[doc(hidden)]
pub struct KeyboardLayoutMetadata;
#[doc(hidden)]
pub struct LayoutArrangementMetadata;
#[doc(hidden)]
pub struct ArrangementRowsMetadata;
#[doc(hidden)]
pub struct ArrangementRowMetadata;
#[doc(hidden)]
pub struct AbstractKeyDataMetadata;
// Native model setters also serve compiled/runtime XAML construction. RequiredMember
// remains a real FerroUI compiler annotation for required layout properties.
ferroui_base::ferro_markup_type!(class KeyboardLayoutMetadata as "KeyboardLayout" {
 namespace:"FerroBoard.Layouts",this:std::rc::Rc<KeyboardLayout>,handles:[std::rc::Rc<KeyboardLayout>,Option<std::rc::Rc<KeyboardLayout>>],
 constructors:[()=>(||std::rc::Rc::new(KeyboardLayout::default()))],attributes:[RequiredMember],
 properties:[Type:LayoutType{get:|l:&std::rc::Rc<KeyboardLayout>|l.layout_type(),set:|l:&std::rc::Rc<KeyboardLayout>,v|l.set_type(v)}[RequiredMember],
 ExtensionId:Option<String>{try_get:|l:&std::rc::Rc<KeyboardLayout>|crate::utf16_metadata::to_native_optional(l.extension_id()),set:|l:&std::rc::Rc<KeyboardLayout>,v:Option<String>|l.set_extension_id(crate::utf16_metadata::from_native_optional(v))}[RequiredMember],
 Id:Option<String>{try_get:|l:&std::rc::Rc<KeyboardLayout>|crate::utf16_metadata::to_native_optional(l.id()),set:|l:&std::rc::Rc<KeyboardLayout>,v:Option<String>|l.set_id(crate::utf16_metadata::from_native_optional(v))}[RequiredMember],
 Label:Option<String>{try_get:|l:&std::rc::Rc<KeyboardLayout>|crate::utf16_metadata::to_native_optional(l.label()),set:|l:&std::rc::Rc<KeyboardLayout>,v:Option<String>|l.set_label(crate::utf16_metadata::from_native_optional(v))},
 Authors:Option<LayoutAuthorsList>{get:|l:&std::rc::Rc<KeyboardLayout>|l.authors().map(LayoutAuthorsList),set:|l:&std::rc::Rc<KeyboardLayout>,v:Option<LayoutAuthorsList>|l.set_authors(v.map(|v|v.0))},
 Direction:ferroboard_core::keys::LayoutDirection{get:|l:&std::rc::Rc<KeyboardLayout>|l.direction(),set:|l:&std::rc::Rc<KeyboardLayout>,v|l.set_direction(v)},
 Modifier:Option<ComponentName>{get:|l:&std::rc::Rc<KeyboardLayout>|l.modifier(),set:|l:&std::rc::Rc<KeyboardLayout>,v:Option<ComponentName>|l.set_modifier(v)},
 Arrangement:Option<std::rc::Rc<LayoutArrangement>>{get:|l:&std::rc::Rc<KeyboardLayout>|l.arrangement(),set:|l:&std::rc::Rc<KeyboardLayout>,v|l.set_arrangement(v)}[RequiredMember],
 Name:ComponentName{get:|l:&std::rc::Rc<KeyboardLayout>|l.name()}]
});
ferroui_base::ferro_markup_type!(class LayoutArrangementMetadata as "LayoutArrangement" {
 namespace:"FerroBoard.Layouts",this:std::rc::Rc<LayoutArrangement>,handles:[std::rc::Rc<LayoutArrangement>,Option<std::rc::Rc<LayoutArrangement>>],
 constructors:[(rows:Option<ArrangementRows>)=>(|rows|std::rc::Rc::new(LayoutArrangement::new_nullable(rows)))],
 static_properties:[Empty:std::rc::Rc<LayoutArrangement>{get:LayoutArrangement::empty}],
 properties:[Rows:Option<ArrangementRows>{get:|a:&std::rc::Rc<LayoutArrangement>|a.rows.clone()}]
});
ferroui_base::ferro_markup_type!(class ArrangementRowsMetadata as "ArrangementRows" {
 namespace:"FerroBoard.Layouts",this:ArrangementRows,handles:[ArrangementRows,Option<ArrangementRows>],
 properties:[Count:i32{try_get:ArrangementRows::count}],indexers:[(i32)->Option<ArrangementRow>{try_get:ArrangementRows::get}]
});
ferroui_base::ferro_markup_type!(class ArrangementRowMetadata as "ArrangementRow" {
 namespace:"FerroBoard.Layouts",this:ArrangementRow,handles:[ArrangementRow,Option<ArrangementRow>],
 properties:[Count:i32{try_get:ArrangementRow::count}],indexers:[(i32)->Option<Key>{try_get:ArrangementRow::get}]
});
ferroui_base::ferro_markup_type!(class AbstractKeyDataMetadata as "AbstractKeyData" {
 namespace:"FerroBoard.Layouts",this:Key,handles:[Key,Option<Key>],attributes:[Abstract],
 methods:[
 try fn AsString(bool)->Option<Utf16Text> => (|k:&Key,display|k.as_string(display).map_err(|e|format!("{e:?}")))[Abstract,Virtual],
 try fn Compute(Option<crate::KeyEvaluatorRef>)->Option<ferroboard_core::layouts::TextKey> => crate::evaluator_metadata::compute[Abstract,Virtual]
 ]
});
ferroui_base::ferro_markup_type!(class LayoutMetadataMetadata as "LayoutMetadata" {
 namespace:"FerroBoard.Layouts",this:Arc<LayoutMetadata>,handles:[Arc<LayoutMetadata>,Option<Arc<LayoutMetadata>>],
 properties:[
 Type:LayoutType{get:|m:&Arc<LayoutMetadata>|m.layout_type},
 Id:Option<String>{try_get:|m:&Arc<LayoutMetadata>|crate::utf16_metadata::to_native_optional(m.id.clone())},
 Label:Option<String>{try_get:|m:&Arc<LayoutMetadata>|crate::utf16_metadata::to_native_optional(m.label.clone())},
 Authors:Option<LayoutAuthorsList>{get:|m:&Arc<LayoutMetadata>|m.authors.clone().map(LayoutAuthorsList)},
 Direction:LayoutDirection{get:|m:&Arc<LayoutMetadata>|m.direction},
 Modifier:Option<ComponentName>{get:|m:&Arc<LayoutMetadata>|m.modifier.clone()}
 ]
});
ferroui_base::ferro_markup_type!(class LayoutMetadataSnapshot {
 namespace:"FerroBoard.Layouts",this:LayoutMetadataSnapshot,handles:[LayoutMetadataSnapshot],
 properties:[Count:i32{get:LayoutMetadataSnapshot::count}],indexers:[(i32)->Arc<LayoutMetadata>{try_get:LayoutMetadataSnapshot::get}]
});
ferroui_base::ferro_markup_type!(class LayoutAuthorsList {
 namespace:"FerroBoard.Layouts",this:LayoutAuthorsList,handles:[LayoutAuthorsList,Option<LayoutAuthorsList>],
 properties:[Count:i32{get:LayoutAuthorsList::count}],indexers:[(i32)->Option<Utf16Text>{try_get:LayoutAuthorsList::get}]
});
fn parse_component_name(value: &str) -> Result<ComponentName, std::convert::Infallible> {
    Ok(ComponentName::from(value))
}
ferroui_base::ferro_markup_type!(struct ComponentNameMetadata as "ComponentName" {
 namespace:"FerroBoard.Layouts",this:ComponentName,handles:[ComponentName],
 parse:parse_component_name,
 constructors:[()=>ComponentName::default,(Option<Utf16Text>,Option<Utf16Text>)=>ComponentName::new_nullable],
 properties:[ExtensionId:Option<String>{try_get:|n:&ComponentName|crate::utf16_metadata::to_native_optional(n.extension_id.clone())},ComponentId:Option<String>{try_get:|n:&ComponentName|crate::utf16_metadata::to_native_optional(n.component_id.clone())}],
 methods:[fn ToString()->Utf16Text=>ComponentName::to_text,
 static try fn Parse(Option<Utf16Text>)->ComponentName => (|value:Option<Utf16Text>|ComponentName::parse_nullable(value.as_ref(),Some(&Utf16Text::default()))),
 static try fn Parse(Option<Utf16Text>,Option<Utf16Text>)->ComponentName => (|value:Option<Utf16Text>,extension:Option<Utf16Text>|ComponentName::parse_nullable(value.as_ref(),extension.as_ref()))]
});
pub(crate) fn register() {
    MarkupType::register_all(&[
        LayoutMetadataMetadata::MARKUP,
        LayoutMetadataSnapshot::MARKUP,
        LayoutAuthorsList::MARKUP,
        ComponentNameMetadata::MARKUP,
        KeyboardLayoutMetadata::MARKUP,
        LayoutArrangementMetadata::MARKUP,
        ArrangementRowsMetadata::MARKUP,
        ArrangementRowMetadata::MARKUP,
        AbstractKeyDataMetadata::MARKUP,
    ]);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<Arc<LayoutMetadata>>();
        ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<Arc<LayoutMetadata>>(),"::std::sync::Arc<::ferroboard_core::layouts::LayoutMetadata>")]);}
        ValueTypes::register_nullable::<ComponentName>();
        ValueTypes::register_nullable::<LayoutAuthorsList>();
        ValueTypes::register_nullable::<std::rc::Rc<KeyboardLayout>>();
        ValueTypes::register_nullable::<std::rc::Rc<LayoutArrangement>>();
        ValueTypes::register_nullable::<ArrangementRows>();
        ValueTypes::register_nullable::<ArrangementRow>();
        ValueTypes::register_nullable::<Key>();
    });
}
