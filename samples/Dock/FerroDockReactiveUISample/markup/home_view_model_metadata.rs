// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::view_models::views::HomeViewModel;
use ferrodock_model_reactiveui_ferro::reactive_metadata::RootDockReference;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class HomeViewModel{namespace:"FerroDockReactiveUISample.ViewModels.Views",this:Rc<HomeViewModel>,handles:[HomeViewModel,Rc<HomeViewModel>,Option<Rc<HomeViewModel>>],base:RootDockReference,constructors:[()=>HomeViewModel::new],notify_property_changed:HomeViewModel});
pub(crate) fn register() {
    ferroui_base::metadata::MarkupType::register(
        <HomeViewModel as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<HomeViewModel>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<HomeViewModel>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<HomeViewModel>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<HomeViewModel>>()
            .cloned()
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<HomeViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<HomeViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<HomeViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockableBaseReference::new(
            model.clone(),
        )
    });
    ferroui_base::data::core::ValueTypes::register_conversion::<Rc<HomeViewModel>, String>(|_| {
        ferroui_base::metadata::MarkupType::find_by_handle(
            std::any::TypeId::of::<Rc<HomeViewModel>>(),
        )
        .map(|meta| meta.full_name())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<HomeViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockBaseReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::DockBaseReference::new(model.clone())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<HomeViewModel>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<HomeViewModel>,
        ferrodock_model_reactiveui_ferro::reactive_metadata::RootDockReference,
    >(|model| {
        ferrodock_model_reactiveui_ferro::reactive_metadata::RootDockReference::new(model.clone())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<HomeViewModel>,
        Rc<dyn ferrodock_model::controls::IRootDock>,
    >(|model| model.clone());
}
