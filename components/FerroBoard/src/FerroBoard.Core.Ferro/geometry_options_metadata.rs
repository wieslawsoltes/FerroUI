//! Reusable XAML construction and configuration for native geometry settings.
use ferroboard_core::layouts::KeyboardGeometryOptions;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;

#[doc(hidden)]
pub struct KeyboardGeometryOptionsMetadata;
ferroui_base::ferro_markup_type!(class KeyboardGeometryOptionsMetadata as "KeyboardGeometryOptions" {
    namespace:"FerroBoard.Layouts",
    this:Rc<KeyboardGeometryOptions>,
    handles:[KeyboardGeometryOptions,Rc<KeyboardGeometryOptions>,Option<Rc<KeyboardGeometryOptions>>],
    constructors:[()=>(||Rc::new(KeyboardGeometryOptions::default()))],
    properties:[
        KeySpacingHorizontal:f64{get:|v:&Rc<KeyboardGeometryOptions>|v.key_spacing_horizontal(),set:|v:&Rc<KeyboardGeometryOptions>,n|v.set_key_spacing_horizontal(n)},
        KeySpacingVertical:f64{get:|v:&Rc<KeyboardGeometryOptions>|v.key_spacing_vertical(),set:|v:&Rc<KeyboardGeometryOptions>,n|v.set_key_spacing_vertical(n)},
        SplitGap:f64{get:|v:&Rc<KeyboardGeometryOptions>|v.split_gap(),set:|v:&Rc<KeyboardGeometryOptions>,n|v.set_split_gap(n)},
        UnitsPerRow:f64{get:|v:&Rc<KeyboardGeometryOptions>|v.units_per_row(),set:|v:&Rc<KeyboardGeometryOptions>,n|v.set_units_per_row(n)}
    ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::KeyboardGeometryOptionsMetadata],contracts:[],generics:[],generic_contracts:[]}

pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(KeyboardGeometryOptionsMetadata::MARKUP);
    ValueTypes::register_global(ValueTypes::register_reference::<KeyboardGeometryOptions>);
}
