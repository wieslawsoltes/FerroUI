//! Native shared popup maps and explicit snapshots; no CLR dictionary/iterator runtime.
use crate::resource_error_metadata::failure;
use ferroboard_core::{
    keys::KeyVariation,
    layouts::{
        PopupLabelMappings, PopupSet, PopupVariationMappings, ResourceDictionaryEntry,
        ResourceEnumerable, ResourceItems,
    },
    text::Utf16Text,
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::rc::Rc;
type Result<T> = std::result::Result<T, MarkupInvokeError>;
// XAML 0.4 index literals use its supported i32 boundary; native snapshots use usize.
fn snapshot_index(index: i32) -> Result<usize> {
    usize::try_from(index)
        .map_err(|_| MarkupInvokeError::Failed("popup snapshot index out of range".into()))
}

#[derive(Clone, PartialEq)]
pub struct PopupLabelPair {
    key: Option<Utf16Text>,
    value: Option<Rc<PopupSet>>,
}
impl PopupLabelPair {
    pub fn key(&self) -> Option<&Utf16Text> {
        self.key.as_ref()
    }
    pub fn value(&self) -> Option<Rc<PopupSet>> {
        self.value.clone()
    }
}
#[derive(Clone, PartialEq)]
pub struct PopupVariationPair {
    key: KeyVariation,
    value: Option<PopupLabelsDictionary>,
}
impl PopupVariationPair {
    pub fn key(&self) -> KeyVariation {
        self.key
    }
    pub fn value(&self) -> Option<PopupLabelsDictionary> {
        self.value.clone()
    }
}
ferroui_base::ferro_markup_type!(struct PopupLabelPair as "PopupLabelPair"{namespace:"FerroBoard.Layouts",this:PopupLabelPair,handles:[PopupLabelPair],constructors:[(key:Option<Utf16Text>,value:Option<Rc<PopupSet>>)=>(|key,value|PopupLabelPair{key,value})],properties:[Key:Option<String>{try_get:|v:&PopupLabelPair|crate::utf16_metadata::to_native_optional(v.key.clone())},Value:Option<Rc<PopupSet>>{get:|v:&PopupLabelPair|v.value.clone()}]});
ferroui_base::ferro_markup_type!(struct PopupVariationPair as "PopupVariationPair"{namespace:"FerroBoard.Layouts",this:PopupVariationPair,handles:[PopupVariationPair],constructors:[(key:KeyVariation,value:Option<PopupLabelsDictionary>)=>(|key,value|PopupVariationPair{key,value})],properties:[Key:KeyVariation{get:|v:&PopupVariationPair|v.key},Value:Option<PopupLabelsDictionary>{get:|v:&PopupVariationPair|v.value.clone()}]});
#[derive(Clone, PartialEq)]
pub struct PopupLabelLookup {
    found: bool,
    value: Option<Rc<PopupSet>>,
}
#[derive(Clone, PartialEq)]
pub struct PopupVariationLookup {
    found: bool,
    value: Option<PopupLabelsDictionary>,
}
impl PopupLabelLookup {
    pub fn found(&self) -> bool {
        self.found
    }
    pub fn value(&self) -> Option<Rc<PopupSet>> {
        self.value.clone()
    }
}
impl PopupVariationLookup {
    pub fn found(&self) -> bool {
        self.found
    }
    pub fn value(&self) -> Option<PopupLabelsDictionary> {
        self.value.clone()
    }
}
ferroui_base::ferro_markup_type!(struct PopupLabelLookup {
 namespace:"FerroBoard.Layouts",this:PopupLabelLookup,handles:[PopupLabelLookup],
 properties:[Found:bool{get:PopupLabelLookup::found},Value:Option<Rc<PopupSet>>{get:PopupLabelLookup::value}]
});
ferroui_base::ferro_markup_type!(struct PopupVariationLookup {
 namespace:"FerroBoard.Layouts",this:PopupVariationLookup,handles:[PopupVariationLookup],
 properties:[Found:bool{get:PopupVariationLookup::found},Value:Option<PopupLabelsDictionary>{get:PopupVariationLookup::value}]
});
fn label_pair(v: ResourceDictionaryEntry<Utf16Text, Rc<PopupSet>>) -> Result<PopupLabelPair> {
    Ok(PopupLabelPair {
        key: v.key,
        value: v.value,
    })
}
fn variation_pair(
    v: ResourceDictionaryEntry<KeyVariation, PopupLabelMappings>,
) -> Result<PopupVariationPair> {
    Ok(PopupVariationPair {
        key: v.key.ok_or_else(|| {
            MarkupInvokeError::Failed("Core dictionary entry returned a null enum key".into())
        })?,
        value: v.value.map(PopupLabelsDictionary::from_original),
    })
}

// Native iterators release their resources through Drop on success, error or unwind.
fn collect<T: Clone + 'static, R>(
    cursor: ResourceItems<T>,
    mut map: impl FnMut(T) -> Result<R>,
) -> Result<Vec<Option<R>>> {
    cursor
        .map(|v| v.map_err(failure)?.map(&mut map).transpose())
        .collect()
}

#[derive(Clone)]
pub struct PopupStringList(Rc<[Option<Utf16Text>]>);
impl PartialEq for PopupStringList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for PopupStringList {}
impl PopupStringList {
    fn new(values: Vec<Option<Utf16Text>>) -> Self {
        Self(Rc::from(values))
    }
    pub fn items(&self) -> &[Option<Utf16Text>] {
        &self.0
    }
    pub fn count(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, index: usize) -> Result<Option<String>> {
        let value =
            self.0.get(index).cloned().ok_or_else(|| {
                MarkupInvokeError::Failed("popup snapshot index out of range".into())
            })?;
        crate::utf16_metadata::to_native_optional(value).map_err(MarkupInvokeError::Failed)
    }
}
#[doc(hidden)]
pub struct PopupStringListMetadata;
ferroui_base::ferro_markup_type!(class PopupStringListMetadata as "PopupStringList"{namespace:"FerroBoard.Layouts",this:PopupStringList,handles:[PopupStringList,Option<PopupStringList>],properties:[Count:usize{get:PopupStringList::count}],indexers:[(i32)->Option<String>{try_get:|value:&PopupStringList,index:i32|value.get(snapshot_index(index)?)}]});
#[derive(Clone)]
pub struct PopupSetList(Rc<[Option<Rc<PopupSet>>]>);
impl PartialEq for PopupSetList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for PopupSetList {}
impl PopupSetList {
    fn new(values: Vec<Option<Rc<PopupSet>>>) -> Self {
        Self(Rc::from(values))
    }
    pub fn items(&self) -> &[Option<Rc<PopupSet>>] {
        &self.0
    }
    pub fn count(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, index: usize) -> Result<Option<Rc<PopupSet>>> {
        let value =
            self.0.get(index).cloned().ok_or_else(|| {
                MarkupInvokeError::Failed("popup snapshot index out of range".into())
            })?;
        Ok(value)
    }
}
#[doc(hidden)]
pub struct PopupSetListMetadata;
ferroui_base::ferro_markup_type!(class PopupSetListMetadata as "PopupSetList"{namespace:"FerroBoard.Layouts",this:PopupSetList,handles:[PopupSetList,Option<PopupSetList>],properties:[Count:usize{get:PopupSetList::count}],indexers:[(i32)->Option<Rc<PopupSet>>{try_get:|value:&PopupSetList,index:i32|value.get(snapshot_index(index)?)}]});
#[derive(Clone)]
pub struct PopupVariationList(Rc<[Option<KeyVariation>]>);
impl PartialEq for PopupVariationList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for PopupVariationList {}
impl PopupVariationList {
    fn new(values: Vec<Option<KeyVariation>>) -> Self {
        Self(Rc::from(values))
    }
    pub fn items(&self) -> &[Option<KeyVariation>] {
        &self.0
    }
    pub fn count(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, index: usize) -> Result<Option<KeyVariation>> {
        let value =
            self.0.get(index).cloned().ok_or_else(|| {
                MarkupInvokeError::Failed("popup snapshot index out of range".into())
            })?;
        Ok(value)
    }
}
#[doc(hidden)]
pub struct PopupVariationListMetadata;
ferroui_base::ferro_markup_type!(class PopupVariationListMetadata as "PopupVariationList"{namespace:"FerroBoard.Layouts",this:PopupVariationList,handles:[PopupVariationList,Option<PopupVariationList>],properties:[Count:usize{get:PopupVariationList::count}],indexers:[(i32)->Option<KeyVariation>{try_get:|value:&PopupVariationList,index:i32|value.get(snapshot_index(index)?)}]});
#[derive(Clone)]
pub struct PopupLabelsList(Rc<[Option<PopupLabelsDictionary>]>);
impl PartialEq for PopupLabelsList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for PopupLabelsList {}
impl PopupLabelsList {
    fn new(values: Vec<Option<PopupLabelsDictionary>>) -> Self {
        Self(Rc::from(values))
    }
    pub fn items(&self) -> &[Option<PopupLabelsDictionary>] {
        &self.0
    }
    pub fn count(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, index: usize) -> Result<Option<PopupLabelsDictionary>> {
        let value =
            self.0.get(index).cloned().ok_or_else(|| {
                MarkupInvokeError::Failed("popup snapshot index out of range".into())
            })?;
        Ok(value)
    }
}
#[doc(hidden)]
pub struct PopupLabelsListMetadata;
ferroui_base::ferro_markup_type!(class PopupLabelsListMetadata as "PopupLabelsList"{namespace:"FerroBoard.Layouts",this:PopupLabelsList,handles:[PopupLabelsList,Option<PopupLabelsList>],properties:[Count:usize{get:PopupLabelsList::count}],indexers:[(i32)->Option<PopupLabelsDictionary>{try_get:|value:&PopupLabelsList,index:i32|value.get(snapshot_index(index)?)}]});
#[derive(Clone)]
pub struct PopupLabelEntries(Rc<[Option<PopupLabelPair>]>);
impl PartialEq for PopupLabelEntries {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for PopupLabelEntries {}
impl PopupLabelEntries {
    fn new(values: Vec<Option<PopupLabelPair>>) -> Self {
        Self(Rc::from(values))
    }
    pub fn items(&self) -> &[Option<PopupLabelPair>] {
        &self.0
    }
    pub fn count(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, index: usize) -> Result<PopupLabelPair> {
        let value =
            self.0.get(index).cloned().ok_or_else(|| {
                MarkupInvokeError::Failed("popup snapshot index out of range".into())
            })?;
        value.ok_or_else(|| {
            MarkupInvokeError::Failed("popup entry snapshot contains a null value".into())
        })
    }
}
#[doc(hidden)]
pub struct PopupLabelEntriesMetadata;
ferroui_base::ferro_markup_type!(class PopupLabelEntriesMetadata as "PopupLabelEntries"{namespace:"FerroBoard.Layouts",this:PopupLabelEntries,handles:[PopupLabelEntries,Option<PopupLabelEntries>],properties:[Count:usize{get:PopupLabelEntries::count}],indexers:[(i32)->PopupLabelPair{try_get:|value:&PopupLabelEntries,index:i32|value.get(snapshot_index(index)?)}]});
#[derive(Clone)]
pub struct PopupVariationEntries(Rc<[Option<PopupVariationPair>]>);
impl PartialEq for PopupVariationEntries {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for PopupVariationEntries {}
impl PopupVariationEntries {
    fn new(values: Vec<Option<PopupVariationPair>>) -> Self {
        Self(Rc::from(values))
    }
    pub fn items(&self) -> &[Option<PopupVariationPair>] {
        &self.0
    }
    pub fn count(&self) -> usize {
        self.0.len()
    }
    pub fn get(&self, index: usize) -> Result<PopupVariationPair> {
        let value =
            self.0.get(index).cloned().ok_or_else(|| {
                MarkupInvokeError::Failed("popup snapshot index out of range".into())
            })?;
        value.ok_or_else(|| {
            MarkupInvokeError::Failed("popup entry snapshot contains a null value".into())
        })
    }
}
#[doc(hidden)]
pub struct PopupVariationEntriesMetadata;
ferroui_base::ferro_markup_type!(class PopupVariationEntriesMetadata as "PopupVariationEntries"{namespace:"FerroBoard.Layouts",this:PopupVariationEntries,handles:[PopupVariationEntries,Option<PopupVariationEntries>],properties:[Count:usize{get:PopupVariationEntries::count}],indexers:[(i32)->PopupVariationPair{try_get:|value:&PopupVariationEntries,index:i32|value.get(snapshot_index(index)?)}]});
#[derive(Clone)]
pub struct PopupStringSequence(ResourceEnumerable<Utf16Text>);
impl PartialEq for PopupStringSequence {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for PopupStringSequence {}
impl PopupStringSequence {
    fn new(value: ResourceEnumerable<Utf16Text>) -> Self {
        Self(value)
    }
    pub fn original(&self) -> ResourceEnumerable<Utf16Text> {
        self.0.clone()
    }
    pub fn snapshot(&self) -> Result<PopupStringList> {
        collect(self.0.iter().map_err(failure)?, Ok).map(PopupStringList::new)
    }
}
#[doc(hidden)]
pub struct PopupStringSequenceMetadata;
ferroui_base::ferro_markup_type!(class PopupStringSequenceMetadata as "PopupStringSequence"{namespace:"FerroBoard.Layouts",this:PopupStringSequence,handles:[PopupStringSequence,Option<PopupStringSequence>],methods:[try fn Snapshot()->PopupStringList=>PopupStringSequence::snapshot]});
#[derive(Clone)]
pub struct PopupSetSequence(ResourceEnumerable<Rc<PopupSet>>);
impl PartialEq for PopupSetSequence {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for PopupSetSequence {}
impl PopupSetSequence {
    fn new(value: ResourceEnumerable<Rc<PopupSet>>) -> Self {
        Self(value)
    }
    pub fn original(&self) -> ResourceEnumerable<Rc<PopupSet>> {
        self.0.clone()
    }
    pub fn snapshot(&self) -> Result<PopupSetList> {
        collect(self.0.iter().map_err(failure)?, Ok).map(PopupSetList::new)
    }
}
#[doc(hidden)]
pub struct PopupSetSequenceMetadata;
ferroui_base::ferro_markup_type!(class PopupSetSequenceMetadata as "PopupSetSequence"{namespace:"FerroBoard.Layouts",this:PopupSetSequence,handles:[PopupSetSequence,Option<PopupSetSequence>],methods:[try fn Snapshot()->PopupSetList=>PopupSetSequence::snapshot]});
#[derive(Clone)]
pub struct PopupVariationSequence(ResourceEnumerable<KeyVariation>);
impl PartialEq for PopupVariationSequence {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for PopupVariationSequence {}
impl PopupVariationSequence {
    fn new(value: ResourceEnumerable<KeyVariation>) -> Self {
        Self(value)
    }
    pub fn original(&self) -> ResourceEnumerable<KeyVariation> {
        self.0.clone()
    }
    pub fn snapshot(&self) -> Result<PopupVariationList> {
        collect(self.0.iter().map_err(failure)?, Ok).map(PopupVariationList::new)
    }
}
#[doc(hidden)]
pub struct PopupVariationSequenceMetadata;
ferroui_base::ferro_markup_type!(class PopupVariationSequenceMetadata as "PopupVariationSequence"{namespace:"FerroBoard.Layouts",this:PopupVariationSequence,handles:[PopupVariationSequence,Option<PopupVariationSequence>],methods:[try fn Snapshot()->PopupVariationList=>PopupVariationSequence::snapshot]});
#[derive(Clone)]
pub struct PopupLabelsSequence(ResourceEnumerable<PopupLabelMappings>);
impl PartialEq for PopupLabelsSequence {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for PopupLabelsSequence {}
impl PopupLabelsSequence {
    fn new(value: ResourceEnumerable<PopupLabelMappings>) -> Self {
        Self(value)
    }
    pub fn original(&self) -> ResourceEnumerable<PopupLabelMappings> {
        self.0.clone()
    }
    pub fn snapshot(&self) -> Result<PopupLabelsList> {
        collect(self.0.iter().map_err(failure)?, |v| {
            Ok(PopupLabelsDictionary::from_original(v))
        })
        .map(PopupLabelsList::new)
    }
}
#[doc(hidden)]
pub struct PopupLabelsSequenceMetadata;
ferroui_base::ferro_markup_type!(class PopupLabelsSequenceMetadata as "PopupLabelsSequence"{namespace:"FerroBoard.Layouts",this:PopupLabelsSequence,handles:[PopupLabelsSequence,Option<PopupLabelsSequence>],methods:[try fn Snapshot()->PopupLabelsList=>PopupLabelsSequence::snapshot]});
#[derive(Clone)]
pub struct PopupLabelsDictionary(PopupLabelMappings);
impl PartialEq for PopupLabelsDictionary {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl Eq for PopupLabelsDictionary {}
impl PopupLabelsDictionary {
    pub fn from_original(value: PopupLabelMappings) -> Self {
        Self(value)
    }
    pub fn original(&self) -> PopupLabelMappings {
        self.0.clone()
    }
    pub fn count(&self) -> Result<i32> {
        self.0.count().map_err(failure)
    }
    pub fn get(&self, key: Option<Utf16Text>) -> Result<Option<Rc<PopupSet>>> {
        self.0.get(key.as_ref()).map_err(failure)
    }
    pub fn contains_key(&self, key: Option<Utf16Text>) -> Result<bool> {
        self.0.contains_key(key.as_ref()).map_err(failure)
    }
    pub fn lookup(&self, key: Option<Utf16Text>) -> Result<PopupLabelLookup> {
        let (found, value) = self.0.try_get_value(key.as_ref()).map_err(failure)?;
        Ok(PopupLabelLookup { found, value })
    }
    pub fn keys(&self) -> Result<Option<PopupStringSequence>> {
        self.0
            .keys()
            .map(|v| v.map(PopupStringSequence::new))
            .map_err(failure)
    }
    pub fn values(&self) -> Result<Option<PopupSetSequence>> {
        self.0
            .values()
            .map(|v| v.map(PopupSetSequence::new))
            .map_err(failure)
    }
    pub fn snapshot(&self) -> Result<PopupLabelEntries> {
        collect(self.0.iter().map_err(failure)?, label_pair).map(PopupLabelEntries::new)
    }
}
#[doc(hidden)]
pub struct PopupLabelsDictionaryMetadata;
ferroui_base::ferro_markup_type!(class PopupLabelsDictionaryMetadata as "PopupLabelsDictionary"{namespace:"FerroBoard.Layouts",this:PopupLabelsDictionary,handles:[PopupLabelsDictionary,Option<PopupLabelsDictionary>],properties:[Count:i32{try_get:PopupLabelsDictionary::count},Keys:Option<PopupStringSequence>{try_get:PopupLabelsDictionary::keys},Values:Option<PopupSetSequence>{try_get:PopupLabelsDictionary::values}],indexers:[(Option<Utf16Text>)->Option<Rc<PopupSet>>{try_get:PopupLabelsDictionary::get}],methods:[try fn ContainsKey(Option<Utf16Text>)->bool=>PopupLabelsDictionary::contains_key,try fn TryGetValue(Option<Utf16Text>)->PopupLabelLookup=>PopupLabelsDictionary::lookup,try fn Snapshot()->PopupLabelEntries=>PopupLabelsDictionary::snapshot]});
#[derive(Clone)]
pub struct PopupVariationsDictionary(PopupVariationMappings);
impl PartialEq for PopupVariationsDictionary {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl Eq for PopupVariationsDictionary {}
impl PopupVariationsDictionary {
    pub fn from_original(value: PopupVariationMappings) -> Self {
        Self(value)
    }
    pub fn original(&self) -> PopupVariationMappings {
        self.0.clone()
    }
    pub fn count(&self) -> Result<i32> {
        self.0.count().map_err(failure)
    }
    pub fn get(&self, key: KeyVariation) -> Result<Option<PopupLabelsDictionary>> {
        self.0
            .get(Some(&key))
            .map(|v| v.map(PopupLabelsDictionary::from_original))
            .map_err(failure)
    }
    pub fn contains_key(&self, key: KeyVariation) -> Result<bool> {
        self.0.contains_key(Some(&key)).map_err(failure)
    }
    pub fn lookup(&self, key: KeyVariation) -> Result<PopupVariationLookup> {
        let (found, value) = self.0.try_get_value(Some(&key)).map_err(failure)?;
        let value = value.map(PopupLabelsDictionary::from_original);
        Ok(PopupVariationLookup { found, value })
    }
    pub fn keys(&self) -> Result<Option<PopupVariationSequence>> {
        self.0
            .keys()
            .map(|v| v.map(PopupVariationSequence::new))
            .map_err(failure)
    }
    pub fn values(&self) -> Result<Option<PopupLabelsSequence>> {
        self.0
            .values()
            .map(|v| v.map(PopupLabelsSequence::new))
            .map_err(failure)
    }
    pub fn snapshot(&self) -> Result<PopupVariationEntries> {
        collect(self.0.iter().map_err(failure)?, variation_pair).map(PopupVariationEntries::new)
    }
}
#[doc(hidden)]
pub struct PopupVariationsDictionaryMetadata;
ferroui_base::ferro_markup_type!(class PopupVariationsDictionaryMetadata as "PopupVariationsDictionary"{namespace:"FerroBoard.Layouts",this:PopupVariationsDictionary,handles:[PopupVariationsDictionary,Option<PopupVariationsDictionary>],properties:[Count:i32{try_get:PopupVariationsDictionary::count},Keys:Option<PopupVariationSequence>{try_get:PopupVariationsDictionary::keys},Values:Option<PopupLabelsSequence>{try_get:PopupVariationsDictionary::values}],indexers:[(KeyVariation)->Option<PopupLabelsDictionary>{try_get:PopupVariationsDictionary::get}],methods:[try fn ContainsKey(KeyVariation)->bool=>PopupVariationsDictionary::contains_key,try fn TryGetValue(KeyVariation)->PopupVariationLookup=>PopupVariationsDictionary::lookup,try fn Snapshot()->PopupVariationEntries=>PopupVariationsDictionary::snapshot]});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::popup_dictionary_metadata::PopupLabelLookup,crate::popup_dictionary_metadata::PopupVariationLookup,crate::popup_dictionary_metadata::PopupLabelPair,crate::popup_dictionary_metadata::PopupVariationPair,crate::popup_dictionary_metadata::PopupStringListMetadata,crate::popup_dictionary_metadata::PopupSetListMetadata,crate::popup_dictionary_metadata::PopupVariationListMetadata,crate::popup_dictionary_metadata::PopupLabelsListMetadata,crate::popup_dictionary_metadata::PopupLabelEntriesMetadata,crate::popup_dictionary_metadata::PopupVariationEntriesMetadata,crate::popup_dictionary_metadata::PopupStringSequenceMetadata,crate::popup_dictionary_metadata::PopupSetSequenceMetadata,crate::popup_dictionary_metadata::PopupVariationSequenceMetadata,crate::popup_dictionary_metadata::PopupLabelsSequenceMetadata,crate::popup_dictionary_metadata::PopupLabelsDictionaryMetadata,crate::popup_dictionary_metadata::PopupVariationsDictionaryMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[
        PopupLabelLookup::MARKUP,
        PopupVariationLookup::MARKUP,
        PopupLabelPair::MARKUP,
        PopupVariationPair::MARKUP,
        PopupStringListMetadata::MARKUP,
        PopupSetListMetadata::MARKUP,
        PopupVariationListMetadata::MARKUP,
        PopupLabelsListMetadata::MARKUP,
        PopupLabelEntriesMetadata::MARKUP,
        PopupVariationEntriesMetadata::MARKUP,
        PopupStringSequenceMetadata::MARKUP,
        PopupSetSequenceMetadata::MARKUP,
        PopupVariationSequenceMetadata::MARKUP,
        PopupLabelsSequenceMetadata::MARKUP,
        PopupLabelsDictionaryMetadata::MARKUP,
        PopupVariationsDictionaryMetadata::MARKUP,
    ]);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<PopupStringList>();
        ValueTypes::register_nullable::<PopupSetList>();
        ValueTypes::register_nullable::<PopupVariationList>();
        ValueTypes::register_nullable::<PopupLabelsList>();
        ValueTypes::register_nullable::<PopupLabelEntries>();
        ValueTypes::register_nullable::<PopupVariationEntries>();
        ValueTypes::register_nullable::<PopupStringSequence>();
        ValueTypes::register_nullable::<PopupSetSequence>();
        ValueTypes::register_nullable::<PopupVariationSequence>();
        ValueTypes::register_nullable::<PopupLabelsSequence>();
        ValueTypes::register_nullable::<PopupLabelsDictionary>();
        ValueTypes::register_nullable::<PopupVariationsDictionary>();
    });
}
