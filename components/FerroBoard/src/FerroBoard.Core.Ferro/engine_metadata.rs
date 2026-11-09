//! Engine metadata invokes the original synchronous Core implementation.
use ferroboard_core::text::Utf16Text;
use ferroboard_core::{
    editor::{EditorController, ITextInputTarget, InputAttributes},
    engine::{KeyboardEngine, LanguageModels},
    keys::{KeyboardMode, KeyboardUiMode, ShiftState},
    layouts::KeyboardResources,
    settings::{FeedbackKind, KeyboardSettings, SwipeAction},
};
use ferroui_base::{
    BoxedValue,
    data::core::ValueTypes,
    metadata::{MarkupDelegate, MarkupType, MarkupTyped, into_markup_value},
    reactive::Disposable,
    utilities::EventArgs,
};
use ferroui_system::{IDisposable, events::Event};
use std::{rc::Rc, sync::Arc, time::Duration};
#[doc(hidden)]
pub struct KeyboardEngineMetadata;
#[doc(hidden)]
pub struct KeyboardResourcesMetadata;
#[doc(hidden)]
pub struct LanguageModelsMetadata;
fn new_engine(
    settings: Option<Rc<KeyboardSettings>>,
    resources: Option<Rc<KeyboardResources>>,
    models: Option<Arc<LanguageModels>>,
) -> Result<Rc<KeyboardEngine>, String> {
    KeyboardEngine::new(settings, resources, models).map_err(|e| e.to_string())
}
fn new_default_engine() -> Result<Rc<KeyboardEngine>, String> {
    KeyboardEngine::with_defaults().map_err(|error| error.to_string())
}
ferroui_base::ferro_markup_type!(class KeyboardEngineMetadata as "KeyboardEngine" {
 namespace:"FerroBoard.Engine",this:Rc<KeyboardEngine>,handles:[Rc<KeyboardEngine>,Option<Rc<KeyboardEngine>>,KeyboardEngine],
 constructors:[try (Option<Rc<KeyboardSettings>>,Option<Rc<KeyboardResources>>,Option<Arc<LanguageModels>>)=>new_engine, try ()=>new_default_engine],
 properties:[
 Suggestions:crate::SuggestionReadOnlyList{get:|e:&Rc<KeyboardEngine>|crate::SuggestionReadOnlyList::from_original(e.suggestions())},
 SuggestionProviderFactory:Option<crate::SuggestionProviderFactoryRef>{get:|e:&Rc<KeyboardEngine>|e.suggestion_provider_factory().map(crate::SuggestionProviderFactoryRef::new),set:|e:&Rc<KeyboardEngine>,v:Option<crate::SuggestionProviderFactoryRef>|e.set_suggestion_provider_factory(v.as_ref().map(crate::SuggestionProviderFactoryRef::callback))},
 Settings:Rc<KeyboardSettings>{get:|e:&Rc<KeyboardEngine>|e.settings()},Resources:Rc<KeyboardResources>{get:|e:&Rc<KeyboardEngine>|e.resources()},LanguageModels:Arc<LanguageModels>{get:|e:&Rc<KeyboardEngine>|e.language_models()},
 Culture:String{get:|e:&Rc<KeyboardEngine>|e.culture().name().to_owned()},
 Target:Option<Rc<dyn ITextInputTarget>>{get:|e:&Rc<KeyboardEngine>|e.target()},Editor:Option<Rc<EditorController>>{get:|e:&Rc<KeyboardEngine>|e.editor()},Attributes:Arc<InputAttributes>{get:|e:&Rc<KeyboardEngine>|e.attributes()},
 Mode:KeyboardMode{get:|e:&Rc<KeyboardEngine>|e.mode()},ShiftState:ShiftState{get:|e:&Rc<KeyboardEngine>|e.shift_state()},UiMode:KeyboardUiMode{get:|e:&Rc<KeyboardEngine>|e.ui_mode()},
 IsCharHalfWidth:bool{get:|e:&Rc<KeyboardEngine>|e.is_char_half_width()},IsKanaKata:bool{get:|e:&Rc<KeyboardEngine>|e.is_kana_kata()},IsGliding:bool{get:|e:&Rc<KeyboardEngine>|e.is_gliding()},IsIncognito:bool{get:|e:&Rc<KeyboardEngine>|e.is_incognito()},
 PunctuationRule:Arc<ferroboard_core::layouts::PunctuationRule>{get:|e:&Rc<KeyboardEngine>|e.punctuation_rule()},
 Clock:Option<crate::ClockRef>{get:|e:&Rc<KeyboardEngine>|e.clock().map(crate::ClockRef::new),set:|e:&Rc<KeyboardEngine>,v:Option<crate::ClockRef>|e.set_clock_nullable(v.as_ref().map(crate::ClockRef::callback))},
 DoubleSpaceWindow:Duration{get:|e:&Rc<KeyboardEngine>|e.double_space_window(),set:|e:&Rc<KeyboardEngine>,v:Duration|e.set_double_space_window(v)},
 ],methods:[
 try fn Attach(Rc<dyn ITextInputTarget>)=>KeyboardEngine::attach,try fn Detach()=> (|e:&Rc<KeyboardEngine>|e.detach()),try fn Dispose()=> (|e:&Rc<KeyboardEngine>|e.dispose()),
 try fn InputText(Option<Utf16Text>)=>(|e:&Rc<KeyboardEngine>,text:Option<Utf16Text>|e.input_text_nullable(text.as_ref())),try fn TypeText(Option<Utf16Text>)=>(|e:&Rc<KeyboardEngine>,text:Option<Utf16Text>|e.type_text_nullable(text.as_ref())),try fn InputEmoji(Option<Utf16Text>)=>(|e:&Rc<KeyboardEngine>,text:Option<Utf16Text>|e.input_emoji_nullable(text.as_ref())),
 try fn SetMode(KeyboardMode)=>(|e:&Rc<KeyboardEngine>,v|e.set_mode(v)),try fn SetUiMode(KeyboardUiMode)=>(|e:&Rc<KeyboardEngine>,v|e.set_ui_mode(v)),try fn SetShiftState(ShiftState)=>(|e:&Rc<KeyboardEngine>,v|e.set_shift_state(v)),
 try fn UpdateLayout(f64,f64)=>(|e:&Rc<KeyboardEngine>,w,h|e.update_layout(w,h,None)),try fn RecomputeKeyboard()=>(|e:&Rc<KeyboardEngine>|e.recompute_keyboard()),try fn NextSubtype()=>(|e:&Rc<KeyboardEngine>|e.next_subtype()),try fn PreviousSubtype()=>(|e:&Rc<KeyboardEngine>|e.previous_subtype()),
 try fn MoveCursor(i32)=>(|e:&Rc<KeyboardEngine>,v|e.move_cursor(v)),try fn ExecuteSwipeAction(SwipeAction,i32)=>(|e:&Rc<KeyboardEngine>,a,c|e.execute_swipe_action(a,c)),try fn ExecuteSwipeAction(SwipeAction)=>(|e:&Rc<KeyboardEngine>,a|e.execute_swipe_action(a,1)),
 fn FeedbackLongPress()=>(|e:&Rc<KeyboardEngine>|e.feedback_long_press()),fn FeedbackGestureStep()=>(|e:&Rc<KeyboardEngine>|e.feedback_gesture_step()),
 ],events:[
 KeyboardChanged(Option<BoxedValue>,EventArgs)=>(|e:&Rc<KeyboardEngine>,h|engine_event(e,KeyboardEngine::keyboard_changed,h)),
 StateChanged(Option<BoxedValue>,EventArgs)=>(|e:&Rc<KeyboardEngine>,h|engine_event(e,KeyboardEngine::state_changed,h)),
 SuggestionsChanged(Option<BoxedValue>,EventArgs)=>(|e:&Rc<KeyboardEngine>,h|engine_event(e,KeyboardEngine::suggestions_changed,h)),
 HideRequested(Option<BoxedValue>,EventArgs)=>(|e:&Rc<KeyboardEngine>,h|engine_event(e,KeyboardEngine::hide_requested,h)),
 SubtypePickerRequested(Option<BoxedValue>,EventArgs)=>(|e:&Rc<KeyboardEngine>,h|engine_event(e,KeyboardEngine::subtype_picker_requested,h)),
 SettingsRequested(Option<BoxedValue>,EventArgs)=>(|e:&Rc<KeyboardEngine>,h|engine_event(e,KeyboardEngine::settings_requested,h)),
 ResizeModeRequested(Option<BoxedValue>,EventArgs)=>(|e:&Rc<KeyboardEngine>,h|engine_event(e,KeyboardEngine::resize_mode_requested,h)),
 VoiceInputRequested(Option<BoxedValue>,EventArgs)=>(|e:&Rc<KeyboardEngine>,h|engine_event(e,KeyboardEngine::voice_input_requested,h)),
 FeedbackRequested(Option<BoxedValue>,FeedbackKind)=>feedback_event,
 TextCommitted(Option<BoxedValue>,Utf16Text)=>text_event,
 ]
});
ferroui_base::ferro_markup_type!(class KeyboardResourcesMetadata as "KeyboardResources" {
 namespace:"FerroBoard.Layouts",this:Rc<KeyboardResources>,handles:[Rc<KeyboardResources>,Option<Rc<KeyboardResources>>,KeyboardResources],
 methods:[static fn CreateEmpty()->Rc<KeyboardResources> =>(||Rc::new(KeyboardResources::create_empty())),static try fn CreateDefault()->Rc<KeyboardResources> =>(||KeyboardResources::create_default().map(Rc::new)),
 try fn GetLayout(ferroboard_core::layouts::LayoutType,ferroboard_core::layouts::ComponentName)->Option<Rc<ferroboard_core::layouts::KeyboardLayout>> => (|r:&Rc<KeyboardResources>,kind,name:ferroboard_core::layouts::ComponentName|r.get_layout(kind,&name)),
 try fn AddLayoutJson(Option<Utf16Text>)->Rc<ferroboard_core::layouts::KeyboardLayout> => (|r:&Rc<KeyboardResources>,json:Option<Utf16Text>|r.add_layout_json_utf16(json.as_ref(),Some(KeyboardResources::CUSTOM_EXTENSION.into()))),
 try fn AddLayoutJson(Option<Utf16Text>,Option<Utf16Text>)->Rc<ferroboard_core::layouts::KeyboardLayout> => (|r:&Rc<KeyboardResources>,json:Option<Utf16Text>,extension:Option<Utf16Text>|r.add_layout_json_utf16(json.as_ref(),extension)),
 try fn GetLayoutMetadata(ferroboard_core::layouts::LayoutType)->crate::LayoutMetadataSnapshot=>(|r:&Rc<KeyboardResources>,kind|r.get_layout_metadata(kind).map(crate::LayoutMetadataSnapshot::new)),
 try fn AddPunctuationRule(Option<Arc<ferroboard_core::layouts::PunctuationRule>>)=>(|r:&Rc<KeyboardResources>,rule|r.add_punctuation_rule_nullable(rule)),
 fn GetPunctuationRule(ferroboard_core::layouts::ComponentName)->Arc<ferroboard_core::layouts::PunctuationRule> => (|r:&Rc<KeyboardResources>,name|r.get_punctuation_rule(&name))],
 events:[Changed(Option<BoxedValue>,EventArgs)=>resource_event]
});
ferroui_base::ferro_markup_type!(class LanguageModelsMetadata as "LanguageModels" {
 namespace:"FerroBoard.Engine",this:Arc<LanguageModels>,handles:[Arc<LanguageModels>,Option<Arc<LanguageModels>>],constructors:[()=>LanguageModels::new],
 properties:[Languages:crate::StringReadOnlyList{get:|m:&Arc<LanguageModels>|crate::native_string_list(Rc::new(ferroboard_core::text::PredictionListBuffer::new(m.languages())))}],
 methods:[try fn HasDictionary(Option<Utf16Text>)->bool=>(|m:&Arc<LanguageModels>,text:Option<Utf16Text>|m.has_dictionary_nullable(text.as_ref())),
 try fn GetDictionary(Option<Utf16Text>)->Option<Arc<ferroboard_core::text::WordDictionary>> => (|m:&Arc<LanguageModels>,text:Option<Utf16Text>|m.get_dictionary_nullable(text.as_ref())),
 try fn IsLoaded(Option<Utf16Text>)->bool=>(|m:&Arc<LanguageModels>,text:Option<Utf16Text>|m.is_loaded_nullable(text.as_ref()))]
});
fn resource_event(
    resources: &Rc<KeyboardResources>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(resources);
    let token = resources.changed().add(Rc::new(move |_| {
        crate::subscriptions::invoke(
            &handler,
            &[
                into_markup_value(sender.upgrade()),
                into_markup_value(EventArgs::EMPTY),
            ],
        );
    }));
    let weak = Rc::downgrade(resources);
    let cleanup = Disposable::create(move || {
        if let Some(resources) = weak.upgrade() {
            resources.changed().remove(token);
        }
    });
    crate::subscriptions::retain_resources(resources, cleanup.clone());
    cleanup
}
fn engine_event(
    e: &Rc<KeyboardEngine>,
    event: fn(&KeyboardEngine) -> &Event<KeyboardEngine>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(e);
    let token = event(e).add(Rc::new(move |_| {
        crate::subscriptions::invoke(
            &handler,
            &[
                into_markup_value(sender.upgrade()),
                into_markup_value(EventArgs::EMPTY),
            ],
        )
    }));
    let weak = Rc::downgrade(e);
    let cleanup = Disposable::create(move || {
        if let Some(e) = weak.upgrade() {
            event(&e).remove(token);
        }
    });
    crate::subscriptions::retain_engine(e, cleanup.clone());
    cleanup
}
fn feedback_event(e: &Rc<KeyboardEngine>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(e);
    let token = e.feedback_requested().add(Rc::new(move |v| {
        crate::subscriptions::invoke(
            &handler,
            &[into_markup_value(sender.upgrade()), into_markup_value(*v)],
        )
    }));
    let weak = Rc::downgrade(e);
    let cleanup = Disposable::create(move || {
        if let Some(e) = weak.upgrade() {
            e.feedback_requested().remove(token);
        }
    });
    crate::subscriptions::retain_engine(e, cleanup.clone());
    cleanup
}
fn text_event(e: &Rc<KeyboardEngine>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(e);
    let token = e.text_committed().add(Rc::new(move |v| {
        crate::subscriptions::invoke(
            &handler,
            &[
                into_markup_value(sender.upgrade()),
                into_markup_value(v.clone()),
            ],
        );
    }));
    let weak = Rc::downgrade(e);
    let cleanup = Disposable::create(move || {
        if let Some(e) = weak.upgrade() {
            e.text_committed().remove(token);
        }
    });
    crate::subscriptions::retain_engine(e, cleanup.clone());
    cleanup
}
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::KeyboardEngineMetadata,crate::KeyboardResourcesMetadata,crate::LanguageModelsMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[
        KeyboardEngineMetadata::MARKUP,
        KeyboardResourcesMetadata::MARKUP,
        LanguageModelsMetadata::MARKUP,
    ]);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_reference::<KeyboardEngine>();
    ValueTypes::register_reference::<KeyboardResources>();
    ValueTypes::register_nullable::<Arc<LanguageModels>>();
    ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<Rc<KeyboardEngine>>(),"::std::rc::Rc<::ferroboard_core::engine::KeyboardEngine>"),(||std::any::TypeId::of::<Rc<KeyboardResources>>(),"::std::rc::Rc<::ferroboard_core::layouts::KeyboardResources>"),(||std::any::TypeId::of::<Arc<LanguageModels>>(),"::std::sync::Arc<::ferroboard_core::engine::LanguageModels>")]);}
}
