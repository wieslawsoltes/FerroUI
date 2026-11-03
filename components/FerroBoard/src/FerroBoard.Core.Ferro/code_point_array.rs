//! Native metadata over the actual fixed-length code-point buffer.
use ferroboard_core::layouts::CodePointArray;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
#[doc(hidden)]
pub struct CodePointArrayMetadata;
ferroui_base::ferro_markup_type!(class CodePointArrayMetadata as "CodePointArray" {
 namespace:"FerroBoard.Layouts",this:CodePointArray,handles:[CodePointArray,Option<CodePointArray>],
 properties:[Length:usize{get:CodePointArray::len},IsEmpty:bool{get:CodePointArray::is_empty}],
 indexers:[(i32)->i32{try_get:CodePointArray::get,try_set:CodePointArray::set}]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::CodePointArrayMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(CodePointArrayMetadata::MARKUP);
    ValueTypes::register_global(ValueTypes::register_nullable::<CodePointArray>);
}
