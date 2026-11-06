//! Original editor operations with native Duration timing; no duplicated editing state.
use ferroboard_core::text::Utf16Text;
use ferroboard_core::{
    editor::{CapitalizationMode, EditorController, ITextInputTarget},
    layouts::PunctuationRule,
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::{rc::Rc, sync::Arc, time::Duration};
#[doc(hidden)]
pub struct EditorControllerMetadata;
#[doc(hidden)]
pub struct PunctuationRuleMetadata;
ferroui_base::ferro_markup_type!(class EditorControllerMetadata as "EditorController" {
 namespace:"FerroBoard.Editor",this:Rc<EditorController>,handles:[Rc<EditorController>,Option<Rc<EditorController>>,EditorController],constructors:[(Rc<dyn ITextInputTarget>)=>(|target|Rc::new(EditorController::new(target)))],
 properties:[
 Target:Rc<dyn ITextInputTarget>{get:|e:&Rc<EditorController>|e.target()},MaxUndoSteps:i32{get:|e:&Rc<EditorController>|e.max_undo_steps(),set:|e:&Rc<EditorController>,v|e.set_max_undo_steps(v)},UndoGroupTimeout:Duration{get:|e:&Rc<EditorController>|e.undo_group_timeout(),set:|e:&Rc<EditorController>,v:Duration|e.set_undo_group_timeout(v)},
 Clock:Option<crate::ClockRef>{get:|e:&Rc<EditorController>|e.clock().map(crate::ClockRef::new),set:|e:&Rc<EditorController>,v:Option<crate::ClockRef>|e.set_clock_nullable(v.as_ref().map(crate::ClockRef::callback))},
 SelectionAnchor:Option<i32>{get:|e:&Rc<EditorController>|e.selection_anchor()},IsSelecting:bool{get:|e:&Rc<EditorController>|e.is_selecting()},Text:String{try_get:|e:&Rc<EditorController>|crate::utf16_metadata::to_native(e.text())},SelectionStart:i32{get:|e:&Rc<EditorController>|e.selection_start()},
SelectionLength:i32{try_get:|e:&Rc<EditorController>|e.selection_length()},
SelectionEnd:i32{try_get:|e:&Rc<EditorController>|e.selection_end()},
HasSelection:bool{try_get:|e:&Rc<EditorController>|e.has_selection()},
IsComposingWord:bool{try_get:|e:&Rc<EditorController>|e.is_composing_word()},
SelectedText:String{try_get:|e:&Rc<EditorController>|(e.selected_text()).map_err(|e|e.to_string()).and_then(crate::utf16_metadata::to_native)},
CurrentWord:String{try_get:|e:&Rc<EditorController>|(e.current_word()).map_err(|e|e.to_string()).and_then(crate::utf16_metadata::to_native)},
CanUndo:bool{get:|e:&Rc<EditorController>|e.can_undo()},CanRedo:bool{get:|e:&Rc<EditorController>|e.can_redo()}],methods:[
try fn GetTextBeforeCursor()->Utf16Text=>(|e:&Rc<EditorController>|e.get_text_before_cursor(i32::MAX)),try fn GetTextBeforeCursor(i32)->Utf16Text=>(|e:&Rc<EditorController>,v|e.get_text_before_cursor(v)),
try fn GetTextAfterCursor()->Utf16Text=>(|e:&Rc<EditorController>|e.get_text_after_cursor(i32::MAX)),try fn GetTextAfterCursor(i32)->Utf16Text=>(|e:&Rc<EditorController>,v|e.get_text_after_cursor(v)),
try fn CommitText(Option<Utf16Text>)=>(|e:&Rc<EditorController>,text:Option<Utf16Text>|e.commit_text_nullable(text.as_ref())),
try fn ReplaceCurrentWord(Option<Utf16Text>)=>(|e:&Rc<EditorController>,text:Option<Utf16Text>|e.replace_current_word_nullable(text.as_ref())),
try fn ReplaceBeforeCursor(i32,Option<Utf16Text>)=>(|e:&Rc<EditorController>,n,text:Option<Utf16Text>|e.replace_before_cursor_nullable(n,text.as_ref())),
try fn DeleteBackward()->bool=>(|e:&Rc<EditorController>|e.delete_backward()),
try fn DeleteWordBackward()->bool=>(|e:&Rc<EditorController>|e.delete_word_backward()),
try fn DeleteForward()->bool=>(|e:&Rc<EditorController>|e.delete_forward()),
try fn DeleteWordForward()->bool=>(|e:&Rc<EditorController>|e.delete_word_forward()),
try fn Undo()->bool=>(|e:&Rc<EditorController>|e.undo()),
try fn Redo()->bool=>(|e:&Rc<EditorController>|e.redo()),
try fn DeleteCharactersBefore(i32)=>(|e:&Rc<EditorController>,v|e.delete_characters_before(v)),
try fn MoveHorizontal(i32)=>(|e:&Rc<EditorController>,v|e.move_horizontal(v)),
try fn MoveWord(i32)=>(|e:&Rc<EditorController>,v|e.move_word(v)),
try fn MoveVertical(i32)=>(|e:&Rc<EditorController>,v|e.move_vertical(v)),
try fn MoveLineStart()=>(|e:&Rc<EditorController>|e.move_line_start()),
try fn MoveLineEnd()=>(|e:&Rc<EditorController>|e.move_line_end()),
try fn ToggleSelectionMode()=>(|e:&Rc<EditorController>|e.toggle_selection_mode()),
fn MoveDocumentStart()=>(|e:&Rc<EditorController>|e.move_document_start()),
fn MoveDocumentEnd()=>(|e:&Rc<EditorController>|e.move_document_end()),
fn SelectAll()=>(|e:&Rc<EditorController>|e.select_all()),
fn EndSelectionMode()=>(|e:&Rc<EditorController>|e.end_selection_mode()),
fn BreakUndoGroup()=>(|e:&Rc<EditorController>|e.break_undo_group()),
fn ClearHistory()=>(|e:&Rc<EditorController>|e.clear_history()),
fn SetCaret(i32)=>(|e:&Rc<EditorController>,v|e.set_caret(v)),try fn ShouldAutoCapitalize(CapitalizationMode,Option<Arc<PunctuationRule>>)->bool=>(|e:&Rc<EditorController>,mode,rule:Option<Arc<PunctuationRule>>|e.should_auto_capitalize_nullable(mode,rule.as_deref()))
 ]
});
ferroui_base::ferro_markup_type!(class PunctuationRuleMetadata as "PunctuationRule"{
 namespace:"FerroBoard.Layouts",this:Arc<PunctuationRule>,handles:[Arc<PunctuationRule>,Option<Arc<PunctuationRule>>],constructors:[()=> (||Arc::new(PunctuationRule::default()))],static_properties:[Default:Arc<PunctuationRule>{get:PunctuationRule::shared_default}],properties:[
ExtensionId:Option<String>{try_get:|r:&Arc<PunctuationRule>|crate::utf16_metadata::to_native_optional(r.extension_id()),set:|r:&Arc<PunctuationRule>,v:Option<String>|r.set_extension_id(crate::utf16_metadata::from_native_optional(v))},
Id:Option<String>{try_get:|r:&Arc<PunctuationRule>|crate::utf16_metadata::to_native_optional(r.id()),set:|r:&Arc<PunctuationRule>,v:Option<String>|r.set_id(crate::utf16_metadata::from_native_optional(v))},
Label:Option<String>{try_get:|r:&Arc<PunctuationRule>|crate::utf16_metadata::to_native_optional(r.label()),set:|r:&Arc<PunctuationRule>,v:Option<String>|r.set_label(crate::utf16_metadata::from_native_optional(v))},
SymbolsPrecedingAutoSpace:Option<String>{try_get:|r:&Arc<PunctuationRule>|crate::utf16_metadata::to_native_optional(r.symbols_preceding_auto_space()),set:|r:&Arc<PunctuationRule>,v:Option<String>|r.set_symbols_preceding_auto_space(crate::utf16_metadata::from_native_optional(v))},
SymbolsFollowingAutoSpace:Option<String>{try_get:|r:&Arc<PunctuationRule>|crate::utf16_metadata::to_native_optional(r.symbols_following_auto_space()),set:|r:&Arc<PunctuationRule>,v:Option<String>|r.set_symbols_following_auto_space(crate::utf16_metadata::from_native_optional(v))},
SymbolsPrecedingPhantomSpace:Option<String>{try_get:|r:&Arc<PunctuationRule>|crate::utf16_metadata::to_native_optional(r.symbols_preceding_phantom_space()),set:|r:&Arc<PunctuationRule>,v:Option<String>|r.set_symbols_preceding_phantom_space(crate::utf16_metadata::from_native_optional(v))},
SymbolsFollowingPhantomSpace:Option<String>{try_get:|r:&Arc<PunctuationRule>|crate::utf16_metadata::to_native_optional(r.symbols_following_phantom_space()),set:|r:&Arc<PunctuationRule>,v:Option<String>|r.set_symbols_following_phantom_space(crate::utf16_metadata::from_native_optional(v))},
SymbolsTerminatingSentence:Option<String>{try_get:|r:&Arc<PunctuationRule>|crate::utf16_metadata::to_native_optional(r.symbols_terminating_sentence()),set:|r:&Arc<PunctuationRule>,v:Option<String>|r.set_symbols_terminating_sentence(crate::utf16_metadata::from_native_optional(v))},

 ],methods:[fn ToString()->Utf16Text=>(|r:&Arc<PunctuationRule>|Utf16Text::from(r.to_string().as_str()))]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::EditorControllerMetadata,crate::PunctuationRuleMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[
        EditorControllerMetadata::MARKUP,
        PunctuationRuleMetadata::MARKUP,
    ]);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_reference::<EditorController>();
    ValueTypes::register_nullable::<Arc<PunctuationRule>>();
    ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<Rc<EditorController>>(),"::std::rc::Rc<::ferroboard_core::editor::EditorController>"),(||std::any::TypeId::of::<Arc<PunctuationRule>>(),"::std::sync::Arc<::ferroboard_core::layouts::PunctuationRule>")]);}
}
