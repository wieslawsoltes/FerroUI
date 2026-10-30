// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::view_models::tools::Tool4ViewModel;
use ferrodock_model_reactiveui_ferro::reactive_metadata::ToolReference;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class Tool4ViewModel{namespace:"FerroDockReactiveUISample.ViewModels.Tools",this:Rc<Tool4ViewModel>,handles:[Tool4ViewModel,Rc<Tool4ViewModel>,Option<Rc<Tool4ViewModel>>],base:ToolReference,constructors:[()=>Tool4ViewModel::new],notify_property_changed:Tool4ViewModel});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <Tool4ViewModel as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<Tool4ViewModel>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool4ViewModel>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<Tool4ViewModel>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<Tool4ViewModel>>()
            .cloned()
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool4ViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool4ViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool4ViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_conversion::<Rc<Tool4ViewModel>, String>(|_| {
        ferroui_base::metadata::MarkupType::find_by_handle(
            std::any::TypeId::of::<Rc<Tool4ViewModel>>(),
        )
        .map(|meta| meta.full_name())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool4ViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ToolReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ToolReference::new(model.clone())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool4ViewModel>,
        Rc<dyn ferrodock_model::controls::ITool>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool4ViewModel>,
        Rc<dyn ferrodock_model::controls::IDocument>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool4ViewModel>,
        Rc<dyn ferrodock_model::controls::IMdiDocument>,
    >(|model| model.clone());
}
