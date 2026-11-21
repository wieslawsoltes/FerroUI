//! Native reusable metadata for Core Search/RibbonSearchEngine.cs.
//! Copyright Wiesław Šoltés, MIT. Core remains independent of native controls.
use crate::controls::{RibbonAdditionalSearchEntries, RibbonSearchEntries};
use ferroribbon_core::{
    RibbonString,
    collections::{RibbonList, RibbonReadOnlyList},
    model::RibbonItemsSource,
    search::{
        RibbonSearchEngine, RibbonSearchEntry, RibbonSearchEntryOptions, RibbonSearchResult,
        RibbonSearchResults, RibbonSearchStrings,
    },
};
use ferroui_base::{
    BoxedValue,
    data::{core::ValueTypes, model::ModelTypes},
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
pub type RibbonSearchEntryEnumerable = RibbonItemsSource<Option<Rc<RibbonSearchEntry>>>;
pub type RibbonSearchStringEnumerable = RibbonItemsSource<Option<RibbonString>>;
fn string(value: Option<RibbonString>) -> Result<Option<String>, String> {
    value
        .map(|s| s.to_utf8().map_err(|e| e.to_string()))
        .transpose()
}
fn index(value: i32) -> usize {
    usize::try_from(value).expect("ArgumentOutOfRange: index")
}
// The source primary constructor has eight fields.
#[allow(clippy::too_many_arguments)]
fn entry(
    id: Option<String>,
    label: Option<String>,
    path: Option<String>,
    description: Option<String>,
    keywords: Option<RibbonSearchStrings>,
    shortcut: Option<String>,
    target: Option<BoxedValue>,
    is_enabled: bool,
) -> Rc<RibbonSearchEntry> {
    RibbonSearchEntry::with_options(
        id.map(Into::into),
        label.map(Into::into),
        RibbonSearchEntryOptions {
            path: path.map(Into::into),
            description: description.map(Into::into),
            keywords,
            shortcut: shortcut.map(Into::into),
            target: crate::metadata::model_objects::from_native(target),
            is_enabled,
        },
    )
}
pub struct RibbonSearchEntryMetadata;
ferroui_base::ferro_markup_type!(class RibbonSearchEntryMetadata as "RibbonSearchEntry"{
 namespace:"FerroRibbon.Search",this:Rc<RibbonSearchEntry>,handles:[Rc<RibbonSearchEntry>,Option<Rc<RibbonSearchEntry>>,RibbonSearchEntry],
 constructors:[(Option<String>,Option<String>)=>|id:Option<String>,label:Option<String>|entry(id,label,None,None,None,None,None,true),(Option<String>,Option<String>,Option<String>)=>|id:Option<String>,label:Option<String>,path:Option<String>|entry(id,label,path,None,None,None,None,true),(Option<String>,Option<String>,Option<String>,Option<String>)=>|id:Option<String>,label:Option<String>,path:Option<String>,description:Option<String>|entry(id,label,path,description,None,None,None,true),(Option<String>,Option<String>,Option<String>,Option<String>,Option<RibbonSearchStrings>)=>|id:Option<String>,label:Option<String>,path:Option<String>,description:Option<String>,keywords:Option<RibbonSearchStrings>|entry(id,label,path,description,keywords,None,None,true),(Option<String>,Option<String>,Option<String>,Option<String>,Option<RibbonSearchStrings>,Option<String>)=>|id:Option<String>,label:Option<String>,path:Option<String>,description:Option<String>,keywords:Option<RibbonSearchStrings>,shortcut:Option<String>|entry(id,label,path,description,keywords,shortcut,None,true),(Option<String>,Option<String>,Option<String>,Option<String>,Option<RibbonSearchStrings>,Option<String>,Option<BoxedValue>)=>|id:Option<String>,label:Option<String>,path:Option<String>,description:Option<String>,keywords:Option<RibbonSearchStrings>,shortcut:Option<String>,target:Option<BoxedValue>|entry(id,label,path,description,keywords,shortcut,target,true),(Option<String>,Option<String>,Option<String>,Option<String>,Option<RibbonSearchStrings>,Option<String>,Option<BoxedValue>,bool)=>|id:Option<String>,label:Option<String>,path:Option<String>,description:Option<String>,keywords:Option<RibbonSearchStrings>,shortcut:Option<String>,target:Option<BoxedValue>,enabled:bool|entry(id,label,path,description,keywords,shortcut,target,enabled)],
 properties:[Id:Option<String>{try_get:|v:&Rc<RibbonSearchEntry>|string(v.id())},Label:Option<String>{try_get:|v:&Rc<RibbonSearchEntry>|string(v.label())},Path:Option<String>{try_get:|v:&Rc<RibbonSearchEntry>|string(v.path())},Description:Option<String>{try_get:|v:&Rc<RibbonSearchEntry>|string(v.description())},Keywords:Option<RibbonSearchStrings>{get:|v:&Rc<RibbonSearchEntry>|v.keywords()},Shortcut:Option<String>{try_get:|v:&Rc<RibbonSearchEntry>|string(v.shortcut())},Target:Option<BoxedValue>{try_get:|v:&Rc<RibbonSearchEntry>|crate::metadata::model_objects::to_native(v.target())},IsEnabled:bool{get:|v:&Rc<RibbonSearchEntry>|v.is_enabled()}]
});
pub struct RibbonSearchResultMetadata;
ferroui_base::ferro_markup_type!(class RibbonSearchResultMetadata as "RibbonSearchResult"{
 namespace:"FerroRibbon.Search",this:Rc<RibbonSearchResult>,handles:[Rc<RibbonSearchResult>,Option<Rc<RibbonSearchResult>>,RibbonSearchResult],
 constructors:[(Option<Rc<RibbonSearchEntry>>,f64)=>RibbonSearchResult::new],properties:[Entry:Option<Rc<RibbonSearchEntry>>{get:|v:&Rc<RibbonSearchResult>|v.entry()},Score:f64{get:|v:&Rc<RibbonSearchResult>|v.score()}]
});
pub struct RibbonSearchEntriesMetadata;
ferroui_base::ferro_markup_type!(interface RibbonSearchEntriesMetadata as "IReadOnlyList`1"{
 namespace:"System.Collections.Generic",this:RibbonSearchEntries,handles:[RibbonSearchEntries,Option<RibbonSearchEntries>],generic:"IReadOnlyList`1" [Option<Rc<RibbonSearchEntry>>],properties:[Count:i32{get:|v:&RibbonSearchEntries|v.count()}],indexers:[(i32)->Option<Rc<RibbonSearchEntry>>{get:|v:&RibbonSearchEntries,i|v.get(i)}]
});
pub struct RibbonSearchResultsMetadata;
ferroui_base::ferro_markup_type!(interface RibbonSearchResultsMetadata as "IReadOnlyList`1"{
 namespace:"System.Collections.Generic",this:RibbonSearchResults,handles:[RibbonSearchResults],generic:"IReadOnlyList`1" [Option<Rc<RibbonSearchResult>>],properties:[Count:i32{get:|v:&RibbonSearchResults|i32::try_from(v.len()).expect("Int32 Count")}],indexers:[(i32)->Option<Rc<RibbonSearchResult>>{get:|v:&RibbonSearchResults,i|v.get(index(i))}]
});
pub struct RibbonAdditionalSearchEntriesMetadata;
ferroui_base::ferro_markup_type!(class RibbonAdditionalSearchEntriesMetadata as "ObservableCollection`1"{
 namespace:"System.Collections.ObjectModel",this:RibbonAdditionalSearchEntries,handles:[RibbonAdditionalSearchEntries],generic:"ObservableCollection`1" [Option<Rc<RibbonSearchEntry>>],notify_property_changed:RibbonAdditionalSearchEntries,
 interfaces:[Rc<dyn ferroui_system::collections::specialized::INotifyCollectionChanged>],constructors:[()=>RibbonAdditionalSearchEntries::new_managed],
 properties:[Count:i32{get:|v:&RibbonAdditionalSearchEntries|i32::try_from(v.count()).expect("Int32 Count")}],
 indexers:[(i32)->Option<Rc<RibbonSearchEntry>>{get:|v:&RibbonAdditionalSearchEntries,i|v.get(index(i)),set:|v:&RibbonAdditionalSearchEntries,i,x|v.set(index(i),x)}],
 methods:[fn Add(Option<Rc<RibbonSearchEntry>>)=>|v:&RibbonAdditionalSearchEntries,x|v.add(x),fn Clear()=>|v:&RibbonAdditionalSearchEntries|v.clear(),fn Remove(Option<Rc<RibbonSearchEntry>>)->bool =>|v:&RibbonAdditionalSearchEntries,x|v.remove(&x),fn RemoveAt(i32)=>|v:&RibbonAdditionalSearchEntries,i|v.remove_at(index(i)),fn Insert(i32,Option<Rc<RibbonSearchEntry>>)=>|v:&RibbonAdditionalSearchEntries,i,x|v.insert(index(i),x),fn Move(i32,i32)=>|v:&RibbonAdditionalSearchEntries,a,b|v.move_item(index(a),index(b))]
});
pub struct RibbonSearchStringsMetadata;
ferroui_base::ferro_markup_type!(interface RibbonSearchStringsMetadata as "IReadOnlyList`1"{
 namespace:"System.Collections.Generic",this:RibbonSearchStrings,handles:[RibbonSearchStrings,Option<RibbonSearchStrings>],generic:"IReadOnlyList`1" [Option<String>],properties:[Count:i32{get:|v:&RibbonSearchStrings|v.count()}],indexers:[(i32)->Option<String>{try_get:|v:&RibbonSearchStrings,i|string(v.get(i))}]
});
pub struct RibbonSearchEntryEnumerableMetadata;
ferroui_base::ferro_markup_type!(interface RibbonSearchEntryEnumerableMetadata as "IEnumerable`1"{namespace:"System.Collections.Generic",this:RibbonSearchEntryEnumerable,handles:[RibbonSearchEntryEnumerable,Option<RibbonSearchEntryEnumerable>],generic:"IEnumerable`1" [Option<Rc<RibbonSearchEntry>>]});
pub struct RibbonSearchStringEnumerableMetadata;
ferroui_base::ferro_markup_type!(interface RibbonSearchStringEnumerableMetadata as "IEnumerable`1"{namespace:"System.Collections.Generic",this:RibbonSearchStringEnumerable,handles:[RibbonSearchStringEnumerable,Option<RibbonSearchStringEnumerable>],generic:"IEnumerable`1" [Option<String>]});
pub struct RibbonSearchEngineMetadata;
ferroui_base::ferro_markup_type!(class RibbonSearchEngineMetadata as "RibbonSearchEngine"{
 namespace:"FerroRibbon.Search",this:Rc<RibbonSearchEngine>,handles:[Rc<RibbonSearchEngine>,Option<Rc<RibbonSearchEngine>>,RibbonSearchEngine],constructors:[()=>RibbonSearchEngine::new],
 properties:[MaxRecent:i32{get:|v:&Rc<RibbonSearchEngine>|v.max_recent(),set:|v:&Rc<RibbonSearchEngine>,x|v.set_max_recent(x)},Entries:RibbonSearchEntries{get:|v:&Rc<RibbonSearchEngine>|v.entries() as RibbonReadOnlyList<_>},Recent:RibbonSearchStrings{get:|v:&Rc<RibbonSearchEngine>|v.recent() as RibbonReadOnlyList<_>}],
 methods:[fn SetEntries(Option<RibbonSearchEntryEnumerable>)=>|v:&Rc<RibbonSearchEngine>,x|v.set_entries(x),fn Add(Option<Rc<RibbonSearchEntry>>)=>|v:&Rc<RibbonSearchEngine>,x|v.add(x),fn MarkUsed(Option<String>)=>|v:&Rc<RibbonSearchEngine>,x:Option<String>|v.mark_used(x.map(Into::into)),fn SetRecent(Option<RibbonSearchStringEnumerable>)=>|v:&Rc<RibbonSearchEngine>,x|v.set_recent(x),fn GetRecentEntries()->RibbonSearchEntryEnumerable =>|v:&Rc<RibbonSearchEngine>|{let query=v.clone().get_recent_entries();RibbonItemsSource::new(move ||Box::new(query.iter().map(Some)))},
 fn Search(Option<String>)->RibbonSearchResults =>|v:&Rc<RibbonSearchEngine>,q:Option<String>|v.search_default(q.map(RibbonString::from).as_ref()),fn Search(Option<String>,i32)->RibbonSearchResults =>|v:&Rc<RibbonSearchEngine>,q:Option<String>,n|v.search(q.map(RibbonString::from).as_ref(),n),
 static fn ScoreEntry(Option<Rc<RibbonSearchEntry>>,Option<RibbonSearchStrings>)->f64 =>|e:Option<Rc<RibbonSearchEntry>>,terms:Option<RibbonSearchStrings>|RibbonSearchEngine::score_entry(e.as_deref(),terms.as_ref()),static try fn Normalize(Option<String>)->String =>|q:Option<String>|RibbonSearchEngine::normalize(q.map(RibbonString::from).as_ref()).to_utf8().map_err(|e|e.to_string())]
});
pub(crate) fn register() {
    ValueTypes::register_nullable::<RibbonSearchStrings>();
    ValueTypes::register_nullable::<RibbonSearchEntries>();
    ValueTypes::register_nullable::<RibbonSearchEntryEnumerable>();
    ValueTypes::register_nullable::<RibbonSearchStringEnumerable>();
    ValueTypes::register_reference::<RibbonSearchEntry>();
    ValueTypes::register_reference::<RibbonSearchResult>();
    ValueTypes::register_reference::<RibbonSearchEngine>();
    // The control library supplies reference views; Core does not depend on native metadata.
    ValueTypes::register_reference::<ferroribbon_core::commands::RibbonCommandDescriptor>();
    ModelTypes::register_notifying_collection::<RibbonAdditionalSearchEntries>();
    ValueTypes::register_cast::<RibbonAdditionalSearchEntries, RibbonSearchEntryEnumerable>(|v| {
        RibbonItemsSource::from_collection(v.clone())
    });
    ValueTypes::register_cast::<RibbonSearchEntries, RibbonSearchEntryEnumerable>(|v| {
        let v = v.clone();
        RibbonItemsSource::new(move || Box::new(v.iter()))
    });
    ValueTypes::register_cast::<
        RibbonList<Option<Rc<RibbonSearchEntry>>>,
        RibbonSearchEntryEnumerable,
    >(|v| {
        let v = v.clone();
        RibbonItemsSource::new(move || Box::new(v.iter()))
    });
    ValueTypes::register_cast::<RibbonSearchStrings, RibbonSearchStringEnumerable>(|v| {
        let v = v.clone();
        RibbonItemsSource::new(move || Box::new(v.iter()))
    });
    MarkupType::register_all(&[
        RibbonSearchEntryMetadata::MARKUP,
        RibbonSearchResultMetadata::MARKUP,
        RibbonSearchEngineMetadata::MARKUP,
        RibbonSearchEntriesMetadata::MARKUP,
        RibbonSearchResultsMetadata::MARKUP,
        RibbonAdditionalSearchEntriesMetadata::MARKUP,
        RibbonSearchStringsMetadata::MARKUP,
        RibbonSearchEntryEnumerableMetadata::MARKUP,
        RibbonSearchStringEnumerableMetadata::MARKUP,
    ]);
}
