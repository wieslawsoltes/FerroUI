// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::view_models::{LayoutFileViewModel, MainWindowViewModel};
use ferrodock_model::controls::IRootDock;
use ferrodock_model_reactiveui_ferro::reactive_metadata::ReactiveObjectReference;
use ferroui_reactiveui::ICommand;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class MainWindowViewModel{namespace:"FerroDockReactiveUISample.ViewModels",this:Rc<MainWindowViewModel>,handles:[MainWindowViewModel,Rc<MainWindowViewModel>,Option<Rc<MainWindowViewModel>>],base:ReactiveObjectReference,constructors:[()=>MainWindowViewModel::new],notify_property_changed:MainWindowViewModel,properties:[
Layout:Option<Rc<dyn IRootDock>>{get:MainWindowViewModel::layout,set:MainWindowViewModel::set_layout},
GlobalStatus:String{get:MainWindowViewModel::global_status,set:MainWindowViewModel::set_global_status},
LayoutFiles:Option<Rc<LayoutFileViewModel>>{get:MainWindowViewModel::layout_files},
NewLayout:Rc<dyn ICommand>{get:MainWindowViewModel::new_layout}],methods:[fn CloseLayout() => MainWindowViewModel::close_layout,fn ResetLayout() => MainWindowViewModel::reset_layout]});
pub(crate) fn register() {
    use ferroui_base::{
        data::core::ValueTypes,
        metadata::{MarkupType, MarkupTyped},
    };
    MarkupType::register(<MainWindowViewModel as MarkupTyped>::MARKUP);
    ValueTypes::register_reference::<MainWindowViewModel>();
    ValueTypes::register_cast::<Rc<MainWindowViewModel>, ReactiveObjectReference>(|model| {
        ReactiveObjectReference::new(model.clone())
    });
}
