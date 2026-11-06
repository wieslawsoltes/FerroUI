//! Native concrete provider and scalar dictionary contracts over the original Core state.
//! Tuple/dictionary-valued APIs and Changed remain explicitly unexported.
use ferroboard_core::text::Utf16Text;
use ferroboard_core::text::{
    DictionarySuggestionProvider, SuggestionRequest, UserDictionary, WordDictionary,
};
use ferroui_base::{
    BoxedValue,
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::{rc::Rc, sync::Arc};

fn null_reference() -> MarkupInvokeError {
    MarkupInvokeError::Failed("NullReference".into())
}
fn dictionary_error(error: ferroboard_core::text::DictionaryError) -> MarkupInvokeError {
    MarkupInvokeError::Failed(format!("{error:?}"))
}

#[doc(hidden)]
pub struct WordDictionaryMetadata;
ferroui_base::ferro_markup_type!(class WordDictionaryMetadata as "WordDictionary" {
 namespace:"FerroBoard.Text",this:Arc<WordDictionary>,handles:[Arc<WordDictionary>,Option<Arc<WordDictionary>>],attributes:[Sealed],
 constructors:[(languageTag:Option<Utf16Text>[Optional,DefaultParameterValue("en")])=>(|tag|Arc::new(WordDictionary::new_nullable(tag)))],
 properties:[LanguageTag:Option<String>{try_get:|d:&Arc<WordDictionary>|crate::utf16_metadata::to_native_optional(d.language_tag())},Count:i32{get:|d:&Arc<WordDictionary>|i32::try_from(d.count()).expect("source dictionary Count exceeds Int32")},Words:crate::StringReadOnlyList{get:|d:&Arc<WordDictionary>|crate::native_string_list(Rc::new(ferroboard_core::text::PredictionListBuffer::new(d.words())))},MaxFrequency:i32{get:|d:&Arc<WordDictionary>|d.max_frequency()}],
 methods:[
 try fn Add(Option<Utf16Text>,i32)=>(|d:&Arc<WordDictionary>,word:Option<Utf16Text>,frequency|d.add(word.as_ref().ok_or_else(null_reference)?,frequency).map_err(dictionary_error)),
 try fn Contains(Option<Utf16Text>)->bool=>(|d:&Arc<WordDictionary>,word:Option<Utf16Text>|d.contains_nullable(word.as_ref()).map_err(dictionary_error)),
 try fn GetFrequency(Option<Utf16Text>)->i32=>(|d:&Arc<WordDictionary>,word:Option<Utf16Text>|d.get_frequency_nullable(word.as_ref()).map_err(dictionary_error)),
 static try fn FromText(Option<Utf16Text>)->Arc<WordDictionary> => (|text:Option<Utf16Text>|{let tag=Some(Utf16Text::from("en"));WordDictionary::from_text_nullable(text.as_ref().ok_or_else(null_reference)?,tag).map(Arc::new).map_err(dictionary_error)}),
 static try fn FromText(Option<Utf16Text>,Option<Utf16Text>)->Arc<WordDictionary> => (|text:Option<Utf16Text>,tag|WordDictionary::from_text_nullable(text.as_ref().ok_or_else(null_reference)?,tag).map(Arc::new).map_err(dictionary_error)),
 static try fn FromJson(Option<Utf16Text>)->Arc<WordDictionary> => (|json:Option<Utf16Text>|{let tag=Some(Utf16Text::from("en"));{
 let json=json.ok_or_else(||MarkupInvokeError::Failed("ArgumentNull(\"json\")".into()))?;
 let text=json.to_utf8().map_err(|_|MarkupInvokeError::Failed("unpaired UTF16 JSON input bridge is not implemented".into()))?;
 WordDictionary::from_json_nullable(&text,tag).map(Arc::new).map_err(dictionary_error)
 }}),
 static try fn FromJson(Option<Utf16Text>,Option<Utf16Text>)->Arc<WordDictionary> => (|json:Option<Utf16Text>,tag|{
 let json=json.ok_or_else(||MarkupInvokeError::Failed("ArgumentNull(\"json\")".into()))?;
 let text=json.to_utf8().map_err(|_|MarkupInvokeError::Failed("unpaired UTF16 JSON input bridge is not implemented".into()))?;
 WordDictionary::from_json_nullable(&text,tag).map(Arc::new).map_err(dictionary_error)
 }),
 static try fn LoadBundledEnglish()->Arc<WordDictionary> => (||WordDictionary::load_bundled_english().map(Arc::new).map_err(dictionary_error)),
 static try fn Normalize(Option<Utf16Text>)->Utf16Text=>(|word:Option<Utf16Text>|WordDictionary::normalize(word.as_ref().ok_or_else(null_reference)?).map_err(|e|MarkupInvokeError::Failed(format!("{e:?}"))))
 ]
});
#[doc(hidden)]
pub struct UserDictionaryMetadata;
ferroui_base::ferro_markup_type!(class UserDictionaryMetadata as "UserDictionary" {
 namespace:"FerroBoard.Text",this:UserDictionary,handles:[UserDictionary,Option<UserDictionary>],attributes:[Sealed],constructors:[()=>UserDictionary::default],
 properties:[LearnThreshold:i32{get:UserDictionary::learn_threshold,set:UserDictionary::set_learn_threshold},MaxWords:i32{get:UserDictionary::max_words,set:UserDictionary::set_max_words}],
 methods:[
 try fn Learn(Option<Utf16Text>,Option<Utf16Text>)=>(|d:&UserDictionary,word:Option<Utf16Text>,previous:Option<Utf16Text>|{d.learn(word.as_ref().ok_or_else(null_reference)?,previous.as_ref());Ok::<_,MarkupInvokeError>(())}),
 try fn GetCount(Option<Utf16Text>)->i32=>(|d:&UserDictionary,word:Option<Utf16Text>|d.get_count_nullable(word.as_ref()).map_err(|e|MarkupInvokeError::Failed(format!("{e:?}")))),
 try fn IsBlocked(Option<Utf16Text>)->bool=>(|d:&UserDictionary,word:Option<Utf16Text>|Ok::<_,MarkupInvokeError>(d.is_blocked_nullable(word.as_ref()))),
 try fn Block(Option<Utf16Text>)=>(|d:&UserDictionary,word:Option<Utf16Text>|d.block_nullable(word.as_ref()).map_err(|e|MarkupInvokeError::Failed(format!("{e:?}")))),
 try fn Unblock(Option<Utf16Text>)=>(|d:&UserDictionary,word:Option<Utf16Text>|{d.unblock_nullable(word.as_ref());Ok::<_,MarkupInvokeError>(())}),
 fn Clear()=>UserDictionary::clear,
 fn Export()->Utf16Text=>(|d:&UserDictionary|Utf16Text::from(d.export().as_str())),
 try fn Import(Option<Utf16Text>)=>(|d:&UserDictionary,json:Option<Utf16Text>|{
     let json=json.ok_or_else(||MarkupInvokeError::Failed("ArgumentNull(\"json\")".into()))?;
     let text=json.to_utf8().map_err(|_|MarkupInvokeError::Failed("unpaired UTF16 JSON input bridge is not implemented".into()))?;
     d.import(&text).map_err(|e|MarkupInvokeError::Failed(format!("{e:?}")))
 })
 ]
});
#[doc(hidden)]
pub struct DictionarySuggestionProviderMetadata;
ferroui_base::ferro_markup_type!(class DictionarySuggestionProviderMetadata as "DictionarySuggestionProvider" {
 namespace:"FerroBoard.Text",this:Rc<DictionarySuggestionProvider>,handles:[Rc<DictionarySuggestionProvider>,Option<Rc<DictionarySuggestionProvider>>,DictionarySuggestionProvider],interfaces:[crate::SuggestionProviderRef],attributes:[Sealed],
 constructors:[(dictionary:Option<Arc<WordDictionary>>,userDictionary:Option<UserDictionary>[Optional,DefaultParameterValue(null)])=>(|dictionary,user|Rc::new(DictionarySuggestionProvider::new_nullable(dictionary,user)))],
 properties:[Dictionary:Option<Arc<WordDictionary>>{get:|p:&Rc<DictionarySuggestionProvider>|p.dictionary()},UserDictionary:UserDictionary{get:|p:&Rc<DictionarySuggestionProvider>|p.user_dictionary()},MaxCorrectionDistance:f64{get:|p:&Rc<DictionarySuggestionProvider>|p.max_correction_distance(),set:|p:&Rc<DictionarySuggestionProvider>,v|p.set_max_correction_distance(v)},AutoCorrectThreshold:f64{get:|p:&Rc<DictionarySuggestionProvider>|p.auto_correct_threshold(),set:|p:&Rc<DictionarySuggestionProvider>,v|p.set_auto_correct_threshold(v)},RareWordFrequencyGap:i32{get:|p:&Rc<DictionarySuggestionProvider>|p.rare_word_frequency_gap(),set:|p:&Rc<DictionarySuggestionProvider>,v|p.set_rare_word_frequency_gap(v)},DefaultPredictions:Option<crate::StringReadOnlyList>{get:|p:&Rc<DictionarySuggestionProvider>|p.default_predictions().map(crate::native_string_list),set:|p:&Rc<DictionarySuggestionProvider>,v:Option<crate::StringReadOnlyList>|p.set_default_predictions(v.map(|v|v.interface()))}],
 methods:[
 try fn Suggest(Option<SuggestionRequest>)->crate::SuggestionReadOnlyList=>(|p:&Rc<DictionarySuggestionProvider>,r:Option<SuggestionRequest>|p.suggest_nullable(r.as_ref()).map(crate::SuggestionReadOnlyList::from_original).map_err(crate::suggestion_list_metadata::failure))[Virtual,Final],
 try fn Learn(Option<Utf16Text>,Option<Utf16Text>)=>(|p:&Rc<DictionarySuggestionProvider>,w:Option<Utf16Text>,previous:Option<Utf16Text>|p.learn_nullable(w.as_ref(),previous.as_ref()).map_err(crate::suggestion_list_metadata::failure))[Virtual,Final],
 try fn Forget(Option<Utf16Text>)=>(|p:&Rc<DictionarySuggestionProvider>,w:Option<Utf16Text>|p.forget_nullable(w.as_ref()).map_err(crate::suggestion_list_metadata::failure))[Virtual,Final]
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::DictionarySuggestionProviderMetadata,crate::WordDictionaryMetadata,crate::UserDictionaryMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[
        WordDictionaryMetadata::MARKUP,
        UserDictionaryMetadata::MARKUP,
        DictionarySuggestionProviderMetadata::MARKUP,
    ]);
    ValueTypes::register_global(register_values);
}
fn provider(object: &BoxedValue) -> Option<crate::SuggestionProviderRef> {
    let any: Rc<dyn std::any::Any> = object.clone();
    any.downcast::<DictionarySuggestionProvider>()
        .ok()
        .map(|p| crate::native_suggestion_provider(p))
}
fn register_values() {
    ValueTypes::register_reference::<DictionarySuggestionProvider>();
    ValueTypes::register_nullable::<Rc<DictionarySuggestionProvider>>();
    ValueTypes::register_boxed_cast::<DictionarySuggestionProvider, crate::SuggestionProviderRef>(
        provider,
    );
    ValueTypes::register_boxed_cast::<
        DictionarySuggestionProvider,
        Option<crate::SuggestionProviderRef>,
    >(|v| provider(v).map(Some));
    ValueTypes::register_nullable::<Arc<WordDictionary>>();
    ValueTypes::register_nullable::<UserDictionary>();
    ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<Rc<DictionarySuggestionProvider>>(),"::std::rc::Rc<::ferroboard_core::text::DictionarySuggestionProvider>"),(||std::any::TypeId::of::<Arc<WordDictionary>>(),"::std::sync::Arc<::ferroboard_core::text::WordDictionary>"),(||std::any::TypeId::of::<UserDictionary>(),"::ferroboard_core::text::UserDictionary")]);}
}
