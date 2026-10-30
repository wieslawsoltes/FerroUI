// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::ReactiveObjectReference;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
pub(crate) fn register() {
    register_reference_identities();
    register_engine_interface_identities();
    ValueTypes::register_nullable::<Rc<dyn ferroui_reactiveui::ICommand>>();
    ValueTypes::register_conversion::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        Option<String>,
    >(|model| super::model_probe::source_name(model).map(Some));
    ValueTypes::register_conversion::<std::rc::Rc<dyn ferrodock_model::core::IDockable>, String>(
        super::model_probe::source_name,
    );
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::ReactiveObjectReference,
    >(super::model_probe::project::<super::ReactiveObjectReference>);
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::ReactiveBaseReference,
    >(super::model_probe::project::<super::ReactiveBaseReference>);
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::DockableBaseReference,
    >(super::model_probe::project::<super::DockableBaseReference>);
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::DockBaseReference,
    >(super::model_probe::project::<super::DockBaseReference>);
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::RootDockReference,
    >(super::model_probe::project::<super::RootDockReference>);
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::DocumentDockReference,
    >(super::model_probe::project::<super::DocumentDockReference>);
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::DocumentReference,
    >(super::model_probe::project::<super::DocumentReference>);
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::ToolReference,
    >(super::model_probe::project::<super::ToolReference>);
    MarkupType::register(<ReactiveObjectReference as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<ReactiveObjectReference>();
    super::reactive_base_reference::register();
    super::dockable_base_reference::register();
    super::dock_base_reference::register();
    super::root_dock_reference::register();
    super::document_dock_reference::register();
    super::document_reference::register();
    super::tool_reference::register();
    super::tool_dock_reference::register();
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::ToolDockReference,
    >(super::model_probe::project::<super::ToolDockReference>);
    super::proportional_dock_reference::register();
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::ProportionalDockReference,
    >(super::model_probe::project::<super::ProportionalDockReference>);
    super::stack_dock_reference::register();
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::StackDockReference,
    >(super::model_probe::project::<super::StackDockReference>);
    super::grid_dock_reference::register();
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::GridDockReference,
    >(super::model_probe::project::<super::GridDockReference>);
    super::dock_dock_reference::register();
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::DockDockReference,
    >(super::model_probe::project::<super::DockDockReference>);
    super::wrap_dock_reference::register();
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::WrapDockReference,
    >(super::model_probe::project::<super::WrapDockReference>);
    super::uniform_grid_dock_reference::register();
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::UniformGridDockReference,
    >(super::model_probe::project::<super::UniformGridDockReference>);
    super::split_view_dock_reference::register();
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::SplitViewDockReference,
    >(super::model_probe::project::<super::SplitViewDockReference>);
    super::proportional_dock_splitter_reference::register();
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::ProportionalDockSplitterReference,
    >(super::model_probe::project::<super::ProportionalDockSplitterReference>);
    super::grid_dock_splitter_reference::register();
    ValueTypes::register_boxed_cast::<
        std::rc::Rc<dyn ferrodock_model::core::IDockable>,
        super::GridDockSplitterReference,
    >(super::model_probe::project::<super::GridDockSplitterReference>);
    super::dock_window_reference::register();
    super::factory_reference::register();
    super::engine_models::register();
    MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::Document>>(
        <super::DocumentReference as MarkupTyped>::MARKUP,
    );
    MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::Tool>>(
        <super::ToolReference as MarkupTyped>::MARKUP,
    );
    MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::DocumentDock>>(
        <super::DocumentDockReference as MarkupTyped>::MARKUP,
    );
    MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::RootDock>>(
        <super::RootDockReference as MarkupTyped>::MARKUP,
    );
}

use ferrodock_model::core::IDockable;
use std::rc::Rc;
/// Registers a known source class name for a concrete model whose metadata uses
/// a shared class-reference handle. Custom model classes register their own metadata.
pub fn register_model_name<T: IDockable + PartialEq>(metadata: &'static MarkupType) {
    ValueTypes::register_conversion::<Rc<T>, String>(move |_| Some(metadata.full_name()));
}

// These are views of the original enclosing object, not separate objects. Keep
// their identity in the shared value metadata, so scopes and converters agree.
fn register_reference_identities() {
    use super::*;
    ValueTypes::register_reference_identity::<ReactiveObjectReference>(|value| {
        value.reference_id()
    });
    ValueTypes::register_reference_identity::<ReactiveBaseReference>(|value| {
        value.base().reference_id()
    });
    ValueTypes::register_reference_identity::<DockableBaseReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
    ValueTypes::register_reference_identity::<DockBaseReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
    ValueTypes::register_reference_identity::<RootDockReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
    ValueTypes::register_reference_identity::<DocumentDockReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
    ValueTypes::register_reference_identity::<DocumentReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
    ValueTypes::register_reference_identity::<ToolReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}

// SDK-only consumers also compare original model interface views. General
// controls register the same backend-independent identities when used alone.
fn register_engine_interface_identities() {
    use ferrodock_model::{controls::*, core::*};
    fn shared<T: ?Sized + 'static>() {
        ValueTypes::register_reference_identity::<Rc<T>>(|value| Rc::as_ptr(value) as *const ());
    }
    ValueTypes::register_nullable::<Rc<dyn IDockable>>();
    ValueTypes::register_nullable::<Rc<dyn IDock>>();
    ValueTypes::register_nullable::<Rc<dyn IFactory>>();
    ValueTypes::register_nullable::<Rc<dyn IDockWindow>>();
    ValueTypes::register_nullable::<Rc<dyn IRootDock>>();
    ValueTypes::register_nullable::<Rc<dyn IToolDock>>();
    ValueTypes::register_nullable::<
        ferrodock_model::collections::ListHandle<Option<Rc<dyn IDockable>>>,
    >();
    ValueTypes::register_nullable::<
        ferrodock_model::collections::ListHandle<Option<Rc<dyn IDockWindow>>>,
    >();
    shared::<dyn IDockable>();
    shared::<dyn IDock>();
    shared::<dyn IDocument>();
    shared::<dyn ITool>();
    shared::<dyn IMdiDocument>();
    shared::<dyn IDockingWindowState>();
    shared::<dyn IRootDock>();
    shared::<dyn IDocumentDock>();
    shared::<dyn IToolDock>();
    shared::<dyn IProportionalDock>();
    shared::<dyn IStackDock>();
    shared::<dyn IGridDock>();
    shared::<dyn IDockDock>();
    shared::<dyn IWrapDock>();
    shared::<dyn IUniformGridDock>();
    shared::<dyn ISplitViewDock>();
    shared::<dyn IProportionalDockSplitter>();
    shared::<dyn IGridDockSplitter>();
    shared::<dyn ISplitter>();
    shared::<dyn IGlobalTarget>();
    shared::<dyn ILocalTarget>();
    shared::<dyn ferroui_reactiveui::IReactiveObject>();
    shared::<dyn ferroui_reactiveui::ICommand>();
}
