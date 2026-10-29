use crate::models::tools::Tool7;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class Tool7{namespace:"FerroDockReactiveUISample.Models.Tools",this:Rc<Tool7>,handles:[Tool7,Rc<Tool7>,Option<Rc<Tool7>>],constructors:[()=>Tool7::new]});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <Tool7 as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<Tool7>();
    ferrodock_controls_recycling::register_value_type::<Tool7>();
}
