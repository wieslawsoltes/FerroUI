// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::view_models::documents::DocumentViewModel;
use ferrodock_model_reactiveui_ferro::reactive_metadata::DocumentReference;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class DocumentViewModel{namespace:"FerroDockReactiveUISample.ViewModels.Documents",this:Rc<DocumentViewModel>,handles:[DocumentViewModel,Rc<DocumentViewModel>,Option<Rc<DocumentViewModel>>],base:DocumentReference,constructors:[()=>DocumentViewModel::new],notify_property_changed:DocumentViewModel});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <DocumentViewModel as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<DocumentViewModel>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentViewModel>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<DocumentViewModel>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<DocumentViewModel>>()
            .cloned()
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_conversion::<Rc<DocumentViewModel>, String>(
        |_| {
            ferroui_base::metadata::MarkupType::find_by_handle(std::any::TypeId::of::<
                Rc<DocumentViewModel>,
            >())
            .map(|meta| meta.full_name())
        },
    );
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DocumentReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DocumentReference::new(model.clone())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentViewModel>,
        Rc<dyn ferrodock_model::controls::IDocument>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentViewModel>,
        Rc<dyn ferrodock_model::controls::IMdiDocument>,
    >(|model| model.clone());
}
