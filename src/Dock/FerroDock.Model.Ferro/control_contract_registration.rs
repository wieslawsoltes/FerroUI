// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original model-control ownership and source-only registration.
use ferrodock_model::{
    controls::{
        IDockDock, IDocument, IDocumentContent, IDocumentDock, IDocumentDockContent,
        IDocumentTemplate, IGridDock, IGridDockSplitter, IMdiDocument, IProportionalDock,
        IProportionalDockSplitter, IRootDock, ISplitViewDock, ISplitter, IStackDock, ITool,
        IToolContent, IToolDock, IToolDockContent, IToolTemplate, IUniformGridDock, IWrapDock,
    },
    core::{IDock, IDockWindow, IDockable, ILocalTarget},
};
use ferroui_base::{data::core::ValueTypes, metadata::from_markup_value, BoxedValue};
use std::rc::Rc;
pub(crate) fn dockable(value: &BoxedValue) -> Option<Rc<dyn IDockable>> {
    from_markup_value(&Some(value.clone()))
}
pub(crate) fn register() {
    crate::document_dock_factory_metadata::register();
    crate::tool_template_metadata::register();
    crate::tool_dock_content_metadata::register();
    crate::dock_window_list_metadata::register();
    ValueTypes::register_reference_identity::<Rc<dyn IToolTemplate>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_reference_identity::<Rc<dyn IToolDockContent>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    crate::dock_dock_metadata::register();
    crate::document_content_metadata::register();
    crate::document_dock_content_metadata::register();
    crate::document_dock_metadata::register();
    crate::document_metadata::register();
    crate::document_template_metadata::register();
    crate::grid_dock_metadata::register();
    crate::grid_dock_splitter_metadata::register();
    crate::proportional_dock_metadata::register();
    crate::proportional_dock_splitter_metadata::register();
    crate::root_dock_metadata::register();
    crate::split_view_dock_metadata::register();
    crate::splitter_metadata::register();
    crate::stack_dock_metadata::register();
    crate::tool_content_metadata::register();
    crate::tool_dock_metadata::register();
    crate::tool_metadata::register();
    crate::uniform_grid_dock_metadata::register();
    crate::wrap_dock_metadata::register();
    crate::mdi_document_metadata::register();
    ValueTypes::register_nullable::<Rc<dyn IDockDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDockDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IDocumentContent>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDocumentContent>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IDocumentDockContent>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDocumentDockContent>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IDocumentDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDocumentDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IDocument>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDocument>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IDocumentTemplate>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDocumentTemplate>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IGridDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IGridDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IGridDockSplitter>>();
    ValueTypes::register_reference_identity::<Rc<dyn IGridDockSplitter>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IProportionalDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IProportionalDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IProportionalDockSplitter>>();
    ValueTypes::register_reference_identity::<Rc<dyn IProportionalDockSplitter>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IRootDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IRootDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn ISplitViewDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn ISplitViewDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn ISplitter>>();
    ValueTypes::register_reference_identity::<Rc<dyn ISplitter>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IStackDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IStackDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IToolContent>>();
    ValueTypes::register_reference_identity::<Rc<dyn IToolContent>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IToolDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IToolDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn ITool>>();
    ValueTypes::register_reference_identity::<Rc<dyn ITool>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IUniformGridDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IUniformGridDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IWrapDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IWrapDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<Rc<dyn IMdiDocument>>();
    ValueTypes::register_reference_identity::<Rc<dyn IMdiDocument>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_nullable::<
        ferrodock_model::collections::ListHandle<Option<Rc<dyn IDockWindow>>>,
    >();
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IRootDock>>(|v| {
        dockable(v)?.as_root_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IRootDock>>(|v| {
        dockable(v)?.as_root_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IMdiDocument>>(|v| {
        dockable(v)?.as_mdi_document()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDocument>, Rc<dyn IMdiDocument>>(|v| {
        dockable(v)?.as_mdi_document()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDocumentDock>>(|v| {
        dockable(v)?.as_document_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IDocumentDock>>(|v| {
        dockable(v)?.as_document_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IToolDock>>(|v| {
        dockable(v)?.as_tool_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IToolDock>>(|v| {
        dockable(v)?.as_tool_dock()
    });
    ValueTypes::register_cast::<Rc<dyn IMdiDocument>, Rc<dyn ILocalTarget>>(|v| v.clone());
    ferrodock_runtime::register_value_type::<Rc<dyn IRootDock>>();
}
