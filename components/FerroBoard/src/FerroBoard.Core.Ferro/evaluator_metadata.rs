//! Native interface dispatch over the original evaluator object and original culture references.
use ferroboard_core::{
    keys::{KeyVariation, KeyboardMode, LayoutDirection, ShiftState},
    layouts::{IKeyEvaluator, Key, TextKey},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::rc::Rc;
// Metadata needs owned dispatch and downcasting; the Core trait stays usable with borrowed evaluators.
trait ErasedKeyEvaluator: IKeyEvaluator + std::any::Any {}
impl<T: IKeyEvaluator + std::any::Any> ErasedKeyEvaluator for T {}
#[derive(Clone)]
pub struct KeyEvaluatorRef(Rc<dyn ErasedKeyEvaluator>);
impl KeyEvaluatorRef {
    pub fn interface(&self) -> Rc<dyn IKeyEvaluator> {
        self.0.clone()
    }
    pub fn downcast<T: IKeyEvaluator + 'static>(&self) -> Option<Rc<T>> {
        let value: Rc<dyn std::any::Any> = self.0.clone();
        value.downcast().ok()
    }
}
impl std::ops::Deref for KeyEvaluatorRef {
    type Target = dyn IKeyEvaluator;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl PartialEq for KeyEvaluatorRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for KeyEvaluatorRef {}
/// Native metadata handle over the caller's actual allocation and synchronous trait dispatch.
pub fn native_key_evaluator<T: IKeyEvaluator + 'static>(object: Rc<T>) -> KeyEvaluatorRef {
    KeyEvaluatorRef(object)
}
fn culture(value: &KeyEvaluatorRef) -> Option<String> {
    crate::locale_metadata::name(value.culture())
}
pub(crate) fn compute(
    key: &Key,
    evaluator: Option<KeyEvaluatorRef>,
) -> Result<Option<TextKey>, MarkupInvokeError> {
    key.clone()
        .compute_nullable(evaluator.as_deref())
        .map_err(|error| MarkupInvokeError::Failed(format!("{error:?}")))
}
#[doc(hidden)]
pub struct IKeyEvaluatorMetadata;
ferroui_base::ferro_markup_type!(interface IKeyEvaluatorMetadata as "IKeyEvaluator"{
 namespace:"FerroBoard.Layouts",this:KeyEvaluatorRef,handles:[KeyEvaluatorRef,Option<KeyEvaluatorRef>],properties:[
 Mode:KeyboardMode{get:|e:&KeyEvaluatorRef|e.mode()}[Abstract,Virtual],
 ShiftState:ShiftState{get:|e:&KeyEvaluatorRef|e.shift_state()}[Abstract,Virtual],
 Variation:KeyVariation{get:|e:&KeyEvaluatorRef|e.variation()}[Abstract,Virtual],
 Direction:LayoutDirection{get:|e:&KeyEvaluatorRef|e.direction()}[Abstract,Virtual],
 IsCharHalfWidth:bool{get:|e:&KeyEvaluatorRef|e.is_char_half_width()}[Abstract,Virtual],
 IsKanaKata:bool{get:|e:&KeyEvaluatorRef|e.is_kana_kata()}[Abstract,Virtual],
 Culture:Option<String>{get:culture}[Abstract,Virtual]
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::IKeyEvaluatorMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(IKeyEvaluatorMetadata::MARKUP);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<KeyEvaluatorRef>();
        ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<KeyEvaluatorRef>(),"::ferroboard_core_ferro::KeyEvaluatorRef")]);}
    });
}
