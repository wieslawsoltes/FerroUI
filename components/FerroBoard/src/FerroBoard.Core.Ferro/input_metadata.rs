//! Static native metadata over original input objects; all virtual calls reach the Core contract.
use ferroboard_core::text::Utf16Text;
use ferroboard_core::{
    editor::{
        CapitalizationMode, EnterAction, ITextInputTarget, InputAttributes, InputKind,
        TextBufferTarget,
    },
    keys::{KeyVariation, KeyboardMode},
};
use ferroui_base::{
    BoxedValue,
    data::core::ValueTypes,
    metadata::{MarkupDelegate, MarkupType, MarkupTyped, into_markup_value},
    reactive::Disposable,
    utilities::EventArgs,
};
use ferroui_system::IDisposable;
use std::{cell::RefCell, rc::Rc, sync::Arc};
#[doc(hidden)]
pub struct ITextInputTargetMetadata;
#[doc(hidden)]
pub struct TextBufferTargetMetadata;
#[doc(hidden)]
pub struct InputAttributesMetadata;
#[derive(Clone)]
pub struct PerformedActionList(Rc<RefCell<Vec<EnterAction>>>);
impl PartialEq for PerformedActionList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl PerformedActionList {
    pub fn from_original(value: Rc<RefCell<Vec<EnterAction>>>) -> Self {
        Self(value)
    }
    pub fn original(&self) -> Rc<RefCell<Vec<EnterAction>>> {
        self.0.clone()
    }
    pub fn count(&self) -> i32 {
        i32::try_from(self.0.borrow().len()).expect("source List.Count exceeds i32")
    }
    pub fn add(&self, value: EnterAction) {
        self.0.borrow_mut().push(value);
    }
    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }
    pub fn get(&self, index: i32) -> Result<EnterAction, &'static str> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.0.borrow().get(i).copied())
            .ok_or("index was out of range")
    }
    pub fn set(&self, index: i32, value: EnterAction) -> Result<(), &'static str> {
        let i = usize::try_from(index).map_err(|_| "index was out of range")?;
        *self
            .0
            .borrow_mut()
            .get_mut(i)
            .ok_or("index was out of range")? = value;
        Ok(())
    }
}
ferroui_base::ferro_markup_type!(interface ITextInputTargetMetadata as "ITextInputTarget" {
 namespace:"FerroBoard.Editor",this:Rc<dyn ITextInputTarget>,handles:[Rc<dyn ITextInputTarget>,Option<Rc<dyn ITextInputTarget>>],
 properties:[
 Attributes:Arc<InputAttributes>{get:|t:&Rc<dyn ITextInputTarget>|t.attributes()},
 Text:String{try_get:|t:&Rc<dyn ITextInputTarget>|crate::utf16_metadata::to_native(t.text())},
 SelectionStart:i32{get:|t:&Rc<dyn ITextInputTarget>|t.selection_start()},SelectionLength:i32{get:|t:&Rc<dyn ITextInputTarget>|t.selection_length()}
 ],methods:[try fn Replace(i32,i32,Option<Utf16Text>)=>(|t:&Rc<dyn ITextInputTarget>,start,length,text:Option<Utf16Text>|t.replace(start,length,text.as_ref())),fn Select(i32,i32)=>(|t:&Rc<dyn ITextInputTarget>,start,length|t.select(start,length)),fn PerformEnterAction(EnterAction)->bool=>(|t:&Rc<dyn ITextInputTarget>,action|t.perform_enter_action(action))],
 events:[Changed(Option<BoxedValue>,EventArgs)=>target_changed],
});
ferroui_base::ferro_markup_type!(class TextBufferTargetMetadata as "TextBufferTarget" {
 namespace:"FerroBoard.Editor",this:Rc<TextBufferTarget>,handles:[Rc<TextBufferTarget>,Option<Rc<TextBufferTarget>>,TextBufferTarget],interfaces:[Rc<dyn ITextInputTarget>],
 constructors:[()=> (||Rc::new(TextBufferTarget::default())),(Utf16Text)=>(|text:Utf16Text|Rc::new(TextBufferTarget::new(text,None))),(Utf16Text,Option<Arc<InputAttributes>>)=>(|text:Utf16Text,attributes|Rc::new(TextBufferTarget::new(text,attributes)))],
 properties:[
 Attributes:Arc<InputAttributes>{get:|t:&Rc<TextBufferTarget>|t.attributes(),set:|t:&Rc<TextBufferTarget>,v|t.set_attributes(v)},
 Text:Option<String>{try_get:|t:&Rc<TextBufferTarget>|crate::utf16_metadata::to_native_optional(Some(t.text())),set:|t:&Rc<TextBufferTarget>,v:Option<String>|t.set_text(crate::utf16_metadata::from_native_optional(v))},
 SelectionStart:i32{get:|t:&Rc<TextBufferTarget>|t.selection_start()},SelectionLength:i32{get:|t:&Rc<TextBufferTarget>|t.selection_length()},
 PerformedActions:PerformedActionList{get:|t:&Rc<TextBufferTarget>|PerformedActionList::from_original(t.performed_actions())},
 EnterActionHandler:Option<MarkupDelegate>{get:|t:&Rc<TextBufferTarget>|crate::input_delegates::get(t),set:|t:&Rc<TextBufferTarget>,v:Option<MarkupDelegate>|crate::input_delegates::set(t,v)},
 ],methods:[try fn Replace(i32,i32,Option<Utf16Text>)=>(|t:&Rc<TextBufferTarget>,start,length,text:Option<Utf16Text>|t.replace(start,length,text.as_ref())),fn Select(i32,i32)=>(|t:&Rc<TextBufferTarget>,start,length|t.select(start,length)),fn PerformEnterAction(EnterAction)->bool=>(|t:&Rc<TextBufferTarget>,action|t.perform_enter_action(action)),fn MoveCaretExternally(i32)=>(|t:&Rc<TextBufferTarget>,position|t.move_caret_externally(position)),fn ToString()->Utf16Text=>(|t:&Rc<TextBufferTarget>|t.to_text())],
 events:[Changed(Option<BoxedValue>,EventArgs)=>(|t:&Rc<TextBufferTarget>,h|target_changed(&(t.clone() as Rc<dyn ITextInputTarget>),h)),Edited(Option<BoxedValue>,EventArgs)=>buffer_edited],
});
// Native mutable configuration properties over the actual shared attribute value.
ferroui_base::ferro_markup_type!(class InputAttributesMetadata as "InputAttributes" {
 namespace:"FerroBoard.Editor",this:Arc<InputAttributes>,handles:[Arc<InputAttributes>,Option<Arc<InputAttributes>>],constructors:[()=> (||Arc::new(InputAttributes::default()))],
 static_properties:[Default:Arc<InputAttributes>{get:InputAttributes::shared_default}],properties:[
 Kind:InputKind{get:|a:&Arc<InputAttributes>|a.kind(),set:|a:&Arc<InputAttributes>,v|a.set_kind(v)},
 EnterAction:EnterAction{get:|a:&Arc<InputAttributes>|a.enter_action(),set:|a:&Arc<InputAttributes>,v|a.set_enter_action(v)},
 Capitalization:CapitalizationMode{get:|a:&Arc<InputAttributes>|a.capitalization(),set:|a:&Arc<InputAttributes>,v|a.set_capitalization(v)},
 IsMultiline:bool{get:|a:&Arc<InputAttributes>|a.is_multiline(),set:|a:&Arc<InputAttributes>,v|a.set_is_multiline(v)},
 AllowSuggestions:bool{get:|a:&Arc<InputAttributes>|a.allow_suggestions(),set:|a:&Arc<InputAttributes>,v|a.set_allow_suggestions(v)},
 AllowAutoCorrect:bool{get:|a:&Arc<InputAttributes>|a.allow_auto_correct(),set:|a:&Arc<InputAttributes>,v|a.set_allow_auto_correct(v)},
 IsPrivate:bool{get:|a:&Arc<InputAttributes>|a.is_private(),set:|a:&Arc<InputAttributes>,v|a.set_is_private(v)},
 IsPassword:bool{get:|a:&Arc<InputAttributes>|a.is_password()},Variation:KeyVariation{get:|a:&Arc<InputAttributes>|a.variation()},InitialMode:KeyboardMode{get:|a:&Arc<InputAttributes>|a.initial_mode()},EffectiveEnterAction:EnterAction{get:|a:&Arc<InputAttributes>|a.effective_enter_action()},SupportsNlp:bool{get:|a:&Arc<InputAttributes>|a.supports_nlp()}
 ]
});
ferroui_base::ferro_markup_type!(class PerformedActionList as "PerformedActionList" {
 namespace:"FerroBoard.Editor",this:PerformedActionList,handles:[PerformedActionList,Option<PerformedActionList>],properties:[Count:i32{get:PerformedActionList::count}],indexers:[(i32)->EnterAction{try_get:PerformedActionList::get,try_set:PerformedActionList::set}],methods:[fn Add(EnterAction)=>PerformedActionList::add,fn Clear()=>PerformedActionList::clear]
});
fn target_changed(t: &Rc<dyn ITextInputTarget>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(t);
    let token = t.changed().add(Rc::new(move |_| {
        crate::subscriptions::invoke(
            &handler,
            &[
                into_markup_value(sender.upgrade()),
                into_markup_value(EventArgs::EMPTY),
            ],
        )
    }));
    let weak = Rc::downgrade(t);
    let cleanup = Disposable::create(move || {
        if let Some(t) = weak.upgrade() {
            t.changed().remove(token);
        }
    });
    crate::subscriptions::retain_target(t, cleanup.clone());
    cleanup
}
fn buffer_edited(t: &Rc<TextBufferTarget>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(t);
    let token = t.edited().add(Rc::new(move |_| {
        crate::subscriptions::invoke(
            &handler,
            &[
                into_markup_value(sender.upgrade()),
                into_markup_value(EventArgs::EMPTY),
            ],
        )
    }));
    let weak = Rc::downgrade(t);
    let cleanup = Disposable::create(move || {
        if let Some(t) = weak.upgrade() {
            t.edited().remove(token);
        }
    });
    crate::subscriptions::retain_target(&(t.clone() as Rc<dyn ITextInputTarget>), cleanup.clone());
    cleanup
}
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::ITextInputTargetMetadata,crate::TextBufferTargetMetadata,crate::InputAttributesMetadata,crate::PerformedActionList],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[
        ITextInputTargetMetadata::MARKUP,
        TextBufferTargetMetadata::MARKUP,
        InputAttributesMetadata::MARKUP,
        PerformedActionList::MARKUP,
    ]);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_reference::<TextBufferTarget>();
    ValueTypes::register_nullable::<Rc<dyn ITextInputTarget>>();
    ValueTypes::register_nullable::<Arc<InputAttributes>>();
    ValueTypes::register_cast::<Rc<TextBufferTarget>, Rc<dyn ITextInputTarget>>(|t| t.clone());
    ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<Rc<dyn ITextInputTarget>>(),"::std::rc::Rc<dyn ::ferroboard_core::editor::ITextInputTarget>"),(||std::any::TypeId::of::<Rc<TextBufferTarget>>(),"::std::rc::Rc<::ferroboard_core::editor::TextBufferTarget>"),(||std::any::TypeId::of::<Arc<InputAttributes>>(),"::std::sync::Arc<::ferroboard_core::editor::InputAttributes>")]);}
}
