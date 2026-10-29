// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::core::*;
use crate::BoxedValue;
use std::any::TypeId;
use std::rc::Rc;

pub(crate) fn cast_dockable<T: ?Sized + 'static>(dockable: Rc<dyn IDockable>) -> Option<Rc<T>> {
    let target = TypeId::of::<T>();
    if target == TypeId::of::<dyn IDockable>() {
        return (Rc::new(dockable) as BoxedValue)
            .downcast_ref::<Rc<T>>()
            .cloned();
    }
    macro_rules! interface {
        ($contract:ty, $query:ident) => {
            if target == TypeId::of::<$contract>() {
                let reference = dockable.$query()?;
                return (Rc::new(reference) as BoxedValue)
                    .downcast_ref::<Rc<T>>()
                    .cloned();
            }
        };
    }
    interface!(dyn IDock, as_dock);
    interface!(dyn crate::controls::IDocument, as_document);
    interface!(dyn crate::controls::IToolContent, as_tool_content);
    interface!(dyn crate::controls::IDocumentContent, as_document_content);
    interface!(dyn crate::controls::ITool, as_tool);
    interface!(dyn crate::controls::IMdiDocument, as_mdi_document);
    interface!(dyn crate::controls::ISplitter, as_splitter);
    interface!(
        dyn crate::controls::IGridDockSplitter,
        as_grid_dock_splitter
    );
    interface!(
        dyn crate::controls::IProportionalDockSplitter,
        as_proportional_dock_splitter
    );
    interface!(dyn crate::controls::IRootDock, as_root_dock);
    interface!(dyn crate::controls::IDocumentDock, as_document_dock);
    interface!(dyn crate::controls::IToolDock, as_tool_dock);
    interface!(dyn crate::controls::IProportionalDock, as_proportional_dock);
    interface!(dyn crate::controls::IDockDock, as_dock_dock);
    interface!(dyn crate::controls::IStackDock, as_stack_dock);
    interface!(dyn crate::controls::IWrapDock, as_wrap_dock);
    interface!(dyn crate::controls::IGridDock, as_grid_dock);
    interface!(dyn crate::controls::IUniformGridDock, as_uniform_grid_dock);
    interface!(dyn crate::controls::ISplitViewDock, as_split_view_dock);
    interface!(
        dyn crate::controls::IDocumentDockContent,
        as_document_dock_content
    );
    interface!(dyn crate::controls::IToolDockContent, as_tool_dock_content);
    let reference = dockable
        .clone()
        .boxed_interface(target)
        .unwrap_or_else(|| dockable.boxed_reference());
    reference.downcast_ref::<Rc<T>>().cloned()
}
