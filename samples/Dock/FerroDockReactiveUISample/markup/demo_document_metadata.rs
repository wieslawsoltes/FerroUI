use crate::models::documents::DemoDocument;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class DemoDocument{namespace:"FerroDockReactiveUISample.Models.Documents",this:Rc<DemoDocument>,handles:[DemoDocument,Rc<DemoDocument>,Option<Rc<DemoDocument>>],constructors:[()=>DemoDocument::new]});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <DemoDocument as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<DemoDocument>();
    ferrodock_controls_recycling::register_value_type::<DemoDocument>();
}
