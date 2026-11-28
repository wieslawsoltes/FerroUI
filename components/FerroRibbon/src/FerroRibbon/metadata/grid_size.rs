//! Original readonly record struct; native value/ctor/member registration.
use ferroribbon_core::model::RibbonGridSize;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
pub struct RibbonGridSizeMetadata;
ferroui_base::ferro_markup_type!(struct RibbonGridSizeMetadata as "RibbonGridSize" {
 namespace:"FerroRibbon.Model",this:RibbonGridSize,handles:[RibbonGridSize],
 constructors:[()=>RibbonGridSize::default,(i32,i32)=>RibbonGridSize::new],
 properties:[Rows:i32{get:|v:&RibbonGridSize|v.rows},Columns:i32{get:|v:&RibbonGridSize|v.columns}],
 methods:[try fn ToString()->String=>|v:&RibbonGridSize|v.to_ribbon_string().to_utf8().map_err(|e|e.to_string())]
});
pub(crate) fn register() {
    // The Core value and the UI metadata descriptor have distinct public paths.
    // Use the framework registry so runtime emission never names private modules.
    ferroui_base::__ferro_compiler_metadata! {
        ferroui_base::metadata::register_type_rust_paths(&[(
            || std::any::TypeId::of::<RibbonGridSize>(),
            "ferroribbon_core::model::RibbonGridSize",
        )]);
    }
    ValueTypes::register_erased_value::<RibbonGridSize>();
    ValueTypes::register_display::<RibbonGridSize>();
    MarkupType::register_all(&[RibbonGridSizeMetadata::MARKUP]);
}
