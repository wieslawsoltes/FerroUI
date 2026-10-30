use crate::models::DemoData;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class DemoData{namespace:"FerroDockReactiveUISample.Models",this:Rc<DemoData>,handles:[DemoData,Rc<DemoData>,Option<Rc<DemoData>>],constructors:[()=>DemoData::new]});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <DemoData as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<DemoData>();
    ferrodock_controls_recycling::register_value_type::<DemoData>();
}
