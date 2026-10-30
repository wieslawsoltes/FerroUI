use crate::models::tools::Tool4;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class Tool4{namespace:"FerroDockReactiveUISample.Models.Tools",this:Rc<Tool4>,handles:[Tool4,Rc<Tool4>,Option<Rc<Tool4>>],constructors:[()=>Tool4::new]});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <Tool4 as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<Tool4>();
    ferrodock_controls_recycling::register_value_type::<Tool4>();
}
