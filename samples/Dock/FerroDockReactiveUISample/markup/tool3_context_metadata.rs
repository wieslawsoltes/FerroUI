use crate::models::tools::Tool3;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class Tool3{namespace:"FerroDockReactiveUISample.Models.Tools",this:Rc<Tool3>,handles:[Tool3,Rc<Tool3>,Option<Rc<Tool3>>],constructors:[()=>Tool3::new]});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <Tool3 as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<Tool3>();
    ferrodock_controls_recycling::register_value_type::<Tool3>();
}
