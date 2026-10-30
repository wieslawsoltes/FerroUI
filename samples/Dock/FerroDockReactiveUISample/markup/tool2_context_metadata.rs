use crate::models::tools::Tool2;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class Tool2{namespace:"FerroDockReactiveUISample.Models.Tools",this:Rc<Tool2>,handles:[Tool2,Rc<Tool2>,Option<Rc<Tool2>>],constructors:[()=>Tool2::new]});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <Tool2 as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<Tool2>();
    ferrodock_controls_recycling::register_value_type::<Tool2>();
}
