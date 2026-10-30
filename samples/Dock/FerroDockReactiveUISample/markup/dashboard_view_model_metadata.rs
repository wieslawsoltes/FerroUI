// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::view_models::views::DashboardViewModel;
use ferrodock_model_reactiveui_ferro::reactive_metadata::DockBaseReference;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class DashboardViewModel{namespace:"FerroDockReactiveUISample.ViewModels.Views",this:Rc<DashboardViewModel>,handles:[DashboardViewModel,Rc<DashboardViewModel>,Option<Rc<DashboardViewModel>>],base:DockBaseReference,constructors:[()=>DashboardViewModel::new],notify_property_changed:DashboardViewModel});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <DashboardViewModel as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<DashboardViewModel>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DashboardViewModel>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<DashboardViewModel>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<DashboardViewModel>>()
            .cloned()
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DashboardViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DashboardViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DashboardViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_conversion::<Rc<DashboardViewModel>, String>(
        |_| {
            ferroui_base::metadata::MarkupType::find_by_handle(std::any::TypeId::of::<
                Rc<DashboardViewModel>,
            >())
            .map(|meta| meta.full_name())
        },
    );
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DashboardViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockBaseReference::new(model.clone())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DashboardViewModel>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
}
