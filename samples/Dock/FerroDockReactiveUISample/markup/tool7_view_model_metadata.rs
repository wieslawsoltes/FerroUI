// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::view_models::tools::Tool7ViewModel;
use ferrodock_model_reactiveui_ferro::reactive_metadata::ToolReference;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class Tool7ViewModel{namespace:"FerroDockReactiveUISample.ViewModels.Tools",this:Rc<Tool7ViewModel>,handles:[Tool7ViewModel,Rc<Tool7ViewModel>,Option<Rc<Tool7ViewModel>>],base:ToolReference,constructors:[()=>Tool7ViewModel::new],notify_property_changed:Tool7ViewModel});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <Tool7ViewModel as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<Tool7ViewModel>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool7ViewModel>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<Tool7ViewModel>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<Tool7ViewModel>>()
            .cloned()
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool7ViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool7ViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool7ViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_conversion::<Rc<Tool7ViewModel>, String>(|_| {
        ferroui_base::metadata::MarkupType::find_by_handle(
            std::any::TypeId::of::<Rc<Tool7ViewModel>>(),
        )
        .map(|meta| meta.full_name())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool7ViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ToolReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ToolReference::new(model.clone())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool7ViewModel>,
        Rc<dyn ferrodock_model::controls::ITool>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool7ViewModel>,
        Rc<dyn ferrodock_model::controls::IDocument>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool7ViewModel>,
        Rc<dyn ferrodock_model::controls::IMdiDocument>,
    >(|model| model.clone());
}
