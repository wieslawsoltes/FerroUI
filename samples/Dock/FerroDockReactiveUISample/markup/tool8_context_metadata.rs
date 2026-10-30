use crate::models::tools::Tool8;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class Tool8{namespace:"FerroDockReactiveUISample.Models.Tools",this:Rc<Tool8>,handles:[Tool8,Rc<Tool8>,Option<Rc<Tool8>>],constructors:[()=>Tool8::new]});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <Tool8 as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<Tool8>();
    ferrodock_controls_recycling::register_value_type::<Tool8>();
}
