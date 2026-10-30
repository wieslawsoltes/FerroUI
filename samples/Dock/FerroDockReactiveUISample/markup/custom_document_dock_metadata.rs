// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::view_models::docks::CustomDocumentDock;
use ferrodock_model_reactiveui_ferro::reactive_metadata::DocumentDockReference;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class CustomDocumentDock{namespace:"FerroDockReactiveUISample.ViewModels.Docks",this:Rc<CustomDocumentDock>,handles:[CustomDocumentDock,Rc<CustomDocumentDock>,Option<Rc<CustomDocumentDock>>],base:DocumentDockReference,constructors:[()=>CustomDocumentDock::new],notify_property_changed:CustomDocumentDock});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <CustomDocumentDock as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<CustomDocumentDock>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<CustomDocumentDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<CustomDocumentDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<CustomDocumentDock>>()
            .cloned()
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<CustomDocumentDock>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<CustomDocumentDock>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<CustomDocumentDock>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_conversion::<Rc<CustomDocumentDock>, String>(
        |_| {
            ferroui_base::metadata::MarkupType::find_by_handle(std::any::TypeId::of::<
                Rc<CustomDocumentDock>,
            >())
            .map(|meta| meta.full_name())
        },
    );
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<CustomDocumentDock>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockBaseReference::new(model.clone())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<CustomDocumentDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<CustomDocumentDock>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DocumentDockReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DocumentDockReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<CustomDocumentDock>,
        Rc<dyn ferrodock_model::controls::IDocumentDock>,
    >(|model| model.clone());
}
