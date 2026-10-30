use crate::models::tools::Tool5;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class Tool5{namespace:"FerroDockReactiveUISample.Models.Tools",this:Rc<Tool5>,handles:[Tool5,Rc<Tool5>,Option<Rc<Tool5>>],constructors:[()=>Tool5::new]});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <Tool5 as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<Tool5>();
    ferrodock_controls_recycling::register_value_type::<Tool5>();
}
