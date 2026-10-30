use crate::models::tools::Tool1;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class Tool1{namespace:"FerroDockReactiveUISample.Models.Tools",this:Rc<Tool1>,handles:[Tool1,Rc<Tool1>,Option<Rc<Tool1>>],constructors:[()=>Tool1::new]});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <Tool1 as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<Tool1>();
    ferrodock_controls_recycling::register_value_type::<Tool1>();
}
