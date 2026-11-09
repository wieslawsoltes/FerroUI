//! Original static field/method metadata, sharing the actual attached property registrations.
use super::ScreenKeyboardInput;
use ferroui_base::{
    BoxedValue, FerroObject, FerroProperty, Ref,
    metadata::{MarkupType, MarkupTyped},
};
ferroui_base::ferro_markup_type!(static ScreenKeyboardInput {
 namespace:"FerroBoard.Controls",type_info:ScreenKeyboardInput,
 fields:[
 InputKindProperty:&'static FerroProperty => ScreenKeyboardInput::input_kind_property,
 EnterActionProperty:&'static FerroProperty => ScreenKeyboardInput::enter_action_property,
 CapitalizationProperty:&'static FerroProperty => ScreenKeyboardInput::capitalization_property,
 SuggestionsEnabledProperty:&'static FerroProperty => ScreenKeyboardInput::suggestions_enabled_property,
 IsPrivateProperty:&'static FerroProperty => ScreenKeyboardInput::is_private_property,
 IsEnabledProperty:&'static FerroProperty => ScreenKeyboardInput::is_enabled_property
 ],methods:[
 static try fn GetInputKind(Option<Ref<FerroObject>>)->Option<BoxedValue> =>(|element:Option<Ref<FerroObject>>|ScreenKeyboardInput::get_input_kind_nullable(element.as_deref())),
 static try fn SetInputKind(Option<Ref<FerroObject>>,Option<BoxedValue>) =>(|element:Option<Ref<FerroObject>>,value|ScreenKeyboardInput::set_input_kind_nullable(element.as_deref(),value)),
 static try fn GetEnterAction(Option<Ref<FerroObject>>)->Option<BoxedValue> =>(|element:Option<Ref<FerroObject>>|ScreenKeyboardInput::get_enter_action_nullable(element.as_deref())),
 static try fn SetEnterAction(Option<Ref<FerroObject>>,Option<BoxedValue>) =>(|element:Option<Ref<FerroObject>>,value|ScreenKeyboardInput::set_enter_action_nullable(element.as_deref(),value)),
 static try fn GetCapitalization(Option<Ref<FerroObject>>)->Option<BoxedValue> =>(|element:Option<Ref<FerroObject>>|ScreenKeyboardInput::get_capitalization_nullable(element.as_deref())),
 static try fn SetCapitalization(Option<Ref<FerroObject>>,Option<BoxedValue>) =>(|element:Option<Ref<FerroObject>>,value|ScreenKeyboardInput::set_capitalization_nullable(element.as_deref(),value)),
 static try fn GetSuggestionsEnabled(Option<Ref<FerroObject>>)->bool =>(|element:Option<Ref<FerroObject>>|ScreenKeyboardInput::get_suggestions_enabled_nullable(element.as_deref())),
 static try fn SetSuggestionsEnabled(Option<Ref<FerroObject>>,bool) =>(|element:Option<Ref<FerroObject>>,value|ScreenKeyboardInput::set_suggestions_enabled_nullable(element.as_deref(),value)),
 static try fn GetIsPrivate(Option<Ref<FerroObject>>)->bool =>(|element:Option<Ref<FerroObject>>|ScreenKeyboardInput::get_is_private_nullable(element.as_deref())),
 static try fn SetIsPrivate(Option<Ref<FerroObject>>,bool) =>(|element:Option<Ref<FerroObject>>,value|ScreenKeyboardInput::set_is_private_nullable(element.as_deref(),value)),
 static try fn GetIsEnabled(Option<Ref<FerroObject>>)->bool =>(|element:Option<Ref<FerroObject>>|ScreenKeyboardInput::get_is_enabled_nullable(element.as_deref())),
 static try fn SetIsEnabled(Option<Ref<FerroObject>>,bool) =>(|element:Option<Ref<FerroObject>>,value|ScreenKeyboardInput::set_is_enabled_nullable(element.as_deref(),value))
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::input::ScreenKeyboardInput],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[ScreenKeyboardInput::MARKUP]);
}
