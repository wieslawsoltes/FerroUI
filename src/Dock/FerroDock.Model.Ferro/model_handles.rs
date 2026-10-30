// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native registration of the original interface upcasts, independent of UI/backend.
use ferrodock_model::{
    controls::{
        IDockDock, IDocument, IDocumentContent, IDocumentDock, IDocumentDockContent, IGridDock,
        IGridDockSplitter, IMdiDocument, IProportionalDock, IProportionalDockSplitter, IRootDock,
        ISplitViewDock, ISplitter, IStackDock, ITool, IToolContent, IToolDock, IToolDockContent,
        IUniformGridDock, IWrapDock,
    },
    core::{DockWindowOptions, IDock, IDockItemContainerMetadata, IDockable},
};
use ferroui_base::data::core::ValueTypes;
use std::rc::Rc;
pub(crate) fn register() {
    ValueTypes::register_nullable::<Rc<DockWindowOptions>>();
    ValueTypes::register_cast::<Rc<dyn IDock>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDock>>(|value| {
        value.downcast_ref::<Rc<dyn IDockable>>()?.clone().as_dock()
    });
    ValueTypes::register_nullable::<Rc<dyn IDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IRootDock>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IRootDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IRootDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IDocumentDock>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IDocumentDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDocumentDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IToolDock>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IToolDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IToolDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IDockDock>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IDockDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDockDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IGridDock>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IGridDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IGridDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IProportionalDock>, Rc<dyn IDockable>>(|value| {
        value.clone()
    });
    ValueTypes::register_nullable::<Rc<dyn IProportionalDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IProportionalDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IStackDock>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IStackDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IStackDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IWrapDock>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IWrapDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IWrapDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IUniformGridDock>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IUniformGridDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn IUniformGridDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn ISplitViewDock>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn ISplitViewDock>>();
    ValueTypes::register_reference_identity::<Rc<dyn ISplitViewDock>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn ISplitter>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn ISplitter>>();
    ValueTypes::register_reference_identity::<Rc<dyn ISplitter>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IGridDockSplitter>, Rc<dyn IDockable>>(|value| {
        value.clone()
    });
    ValueTypes::register_nullable::<Rc<dyn IGridDockSplitter>>();
    ValueTypes::register_reference_identity::<Rc<dyn IGridDockSplitter>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IProportionalDockSplitter>, Rc<dyn IDockable>>(|value| {
        value.clone()
    });
    ValueTypes::register_nullable::<Rc<dyn IProportionalDockSplitter>>();
    ValueTypes::register_reference_identity::<Rc<dyn IProportionalDockSplitter>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IDocument>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IDocument>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDocument>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn ITool>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn ITool>>();
    ValueTypes::register_reference_identity::<Rc<dyn ITool>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IMdiDocument>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IMdiDocument>>();
    ValueTypes::register_reference_identity::<Rc<dyn IMdiDocument>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IDocumentContent>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IDocumentContent>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDocumentContent>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IToolContent>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IToolContent>>();
    ValueTypes::register_reference_identity::<Rc<dyn IToolContent>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IDocumentDockContent>, Rc<dyn IDockable>>(|value| {
        value.clone()
    });
    ValueTypes::register_nullable::<Rc<dyn IDocumentDockContent>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDocumentDockContent>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IToolDockContent>, Rc<dyn IDockable>>(|value| value.clone());
    ValueTypes::register_nullable::<Rc<dyn IToolDockContent>>();
    ValueTypes::register_reference_identity::<Rc<dyn IToolDockContent>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IRootDock>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn IDocumentDock>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn IToolDock>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn IDockDock>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn IGridDock>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn IProportionalDock>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn IStackDock>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn IWrapDock>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn IUniformGridDock>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn ISplitViewDock>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn IDocumentDockContent>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_cast::<Rc<dyn IToolDockContent>, Rc<dyn IDock>>(|value| value.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockItemContainerMetadata>, Rc<dyn IDockable>>(
        |value| {
            value
                .downcast_ref::<Rc<dyn IDockItemContainerMetadata>>()?
                .clone()
                .as_dockable()
        },
    );
    ValueTypes::register_reference_identity::<Rc<dyn IDockable>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
}
