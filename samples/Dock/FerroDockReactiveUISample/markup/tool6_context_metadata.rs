use crate::models::tools::Tool6;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class Tool6{namespace:"FerroDockReactiveUISample.Models.Tools",this:Rc<Tool6>,handles:[Tool6,Rc<Tool6>,Option<Rc<Tool6>>],constructors:[()=>Tool6::new]});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <Tool6 as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<Tool6>();
    ferrodock_controls_recycling::register_value_type::<Tool6>();
}
