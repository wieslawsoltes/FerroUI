// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Shared native metadata/casts over the original model handles.
use ferrodock_model::{
    collections::ListHandle,
    controls::{IDocumentDock, IMdiDocument, IRootDock, IToolDock},
    core::{IDock, IDockWindow, IDockable},
};
use ferroui_base::data::core::ValueTypes;
use ferroui_controls::ItemsSource;
use std::rc::Rc;
pub(crate) fn register() {
    register_shared_model_identities();
    crate::local_target_metadata::register();
    ValueTypes::register_conversion::<ListHandle<Option<Rc<dyn IDockWindow>>>, ItemsSource>(
        crate::model_items::items,
    );
    ValueTypes::register_conversion::<ListHandle<Option<Rc<dyn IDockWindow>>>, Option<ItemsSource>>(
        |list| crate::model_items::items(list).map(Some),
    );
    ValueTypes::register_conversion::<
        Option<ListHandle<Option<Rc<dyn IDockWindow>>>>,
        Option<ItemsSource>,
    >(|list| match list {
        None => Some(None),
        Some(list) => crate::model_items::items(list).map(Some),
    });

    crate::document_metadata::register();
    crate::tool_metadata::register();
    crate::document_dock_content_metadata::register();
    crate::grid_dock_splitter_metadata::register();
    crate::proportional_dock_splitter_metadata::register();
    crate::splitter_metadata::register();
    ferrodock_controls_recycling::register_value_type::<Rc<dyn IRootDock>>();
    crate::global_target_metadata::register();
    crate::tool_content_metadata::register();
    crate::document_content_metadata::register();
    crate::split_view_dock_metadata::register();
    crate::uniform_grid_dock_metadata::register();
    crate::wrap_dock_metadata::register();
    crate::stack_dock_metadata::register();
    crate::grid_dock_metadata::register();
    crate::proportional_dock_metadata::register();
    crate::dock_dock_metadata::register();
    ValueTypes::register_nullable::<Rc<dyn IRootDock>>();
    ValueTypes::register_nullable::<Rc<dyn IDockWindow>>();
    ValueTypes::register_nullable::<ListHandle<Option<Rc<dyn IDockWindow>>>>();
    ValueTypes::register_cast::<Rc<dyn IRootDock>, Rc<dyn IDock>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn IRootDock>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IRootDock>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_root_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IRootDock>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_root_dock()
    });
    ValueTypes::register_nullable::<Rc<dyn IDock>>();
    ValueTypes::register_nullable::<Rc<dyn IDocumentDock>>();
    ValueTypes::register_nullable::<Rc<dyn IToolDock>>();
    ValueTypes::register_nullable::<ListHandle<Option<Rc<dyn IDockable>>>>();
    ValueTypes::register_cast::<Rc<dyn IDock>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn IDocumentDock>, Rc<dyn IDock>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn IDocumentDock>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn IToolDock>, Rc<dyn IDock>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn IToolDock>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IMdiDocument>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_mdi_document()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDock>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDocumentDock>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_document_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IToolDock>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_tool_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IDocumentDock>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_document_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IToolDock>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_tool_dock()
    });
    ValueTypes::register_conversion::<ListHandle<Option<Rc<dyn IDockable>>>, ItemsSource>(
        crate::model_items::items,
    );
    ValueTypes::register_conversion::<ListHandle<Option<Rc<dyn IDockable>>>, Option<ItemsSource>>(
        |list| crate::model_items::items(list).map(Some),
    );
    ValueTypes::register_conversion::<
        Option<ListHandle<Option<Rc<dyn IDockable>>>>,
        Option<ItemsSource>,
    >(|list| match list {
        None => Some(None),
        Some(list) => crate::model_items::items(list).map(Some),
    });
    ValueTypes::register_cast::<
        ListHandle<Option<Rc<dyn IDockable>>>,
        ListHandle<Option<ferroui_base::BoxedValue>>,
    >(crate::model_items::mutable);
}

// Interface handles keep their enclosing engine identity across all model backends.
fn register_shared_model_identities() {
    use ferrodock_model::{controls::*, core::*};
    fn shared<T: ?Sized + 'static>() {
        ValueTypes::register_reference_identity::<Rc<T>>(|value| Rc::as_ptr(value) as *const ());
    }
    shared::<dyn IDockable>();
    shared::<dyn IDock>();
    shared::<dyn IFactory>();
    shared::<dyn IDockWindow>();
    shared::<dyn IHostWindow>();
    shared::<dyn IDockingWindowState>();
    shared::<dyn IDocument>();
    shared::<dyn ITool>();
    shared::<dyn IRootDock>();
    shared::<dyn IDocumentDock>();
    shared::<dyn IToolDock>();
    shared::<dyn IMdiDocument>();
    shared::<dyn ISplitter>();
    shared::<dyn IProportionalDockSplitter>();
    shared::<dyn IGridDockSplitter>();
    shared::<dyn IProportionalDock>();
    shared::<dyn IGridDock>();
    shared::<dyn IStackDock>();
    shared::<dyn IWrapDock>();
    shared::<dyn IDockDock>();
    shared::<dyn IUniformGridDock>();
    shared::<dyn ISplitViewDock>();
    shared::<dyn IDocumentContent>();
    shared::<dyn IToolContent>();
    shared::<dyn IDocumentDockContent>();
    shared::<dyn IToolDockContent>();
    shared::<dyn IItemsSourceDock>();
    shared::<dyn ferroui_reactiveui::IReactiveObject>();
    shared::<dyn ferroui_reactiveui::ICommand>();
}
