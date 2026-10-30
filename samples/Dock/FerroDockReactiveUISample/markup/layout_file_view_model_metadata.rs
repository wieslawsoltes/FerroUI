//! Explicit metadata for compiled file command/status bindings.
use crate::view_models::LayoutFileViewModel;
use ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference;
use ferroui_reactiveui::ICommand;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class LayoutFileViewModel {
    namespace:"FerroDockReactiveUISample.ViewModels", this:Rc<LayoutFileViewModel>,
    handles:[LayoutFileViewModel,Rc<LayoutFileViewModel>,Option<Rc<LayoutFileViewModel>>],
    base:ReactiveObjectReference, notify_property_changed:LayoutFileViewModel,
    properties:[
        SaveLayout:Rc<dyn ICommand>{get:LayoutFileViewModel::save_layout},
        LoadLayout:Rc<dyn ICommand>{get:LayoutFileViewModel::load_layout},
        Status:String{get:LayoutFileViewModel::status},
        IsBusy:bool{get:LayoutFileViewModel::is_busy}
    ]
});
pub(crate) fn register() {
    use ferroui_base::{
        data::core::ValueTypes,
        metadata::{MarkupType, MarkupTyped},
    };
    MarkupType::register(<LayoutFileViewModel as MarkupTyped>::MARKUP);
    ValueTypes::register_reference::<LayoutFileViewModel>();
    ValueTypes::register_cast::<Rc<LayoutFileViewModel>, ReactiveObjectReference>(|model| {
        ReactiveObjectReference::new(model.clone())
    });
}
