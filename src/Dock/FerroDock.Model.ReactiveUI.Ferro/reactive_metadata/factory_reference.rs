// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{
    collections::ListHandle,
    controls::{
        IDockDock, IDocument, IDocumentDock, IGridDock, IGridDockSplitter, IProportionalDock,
        IProportionalDockSplitter, IRootDock, ISplitViewDock, IStackDock, ITool, IToolDock,
        IUniformGridDock, IWrapDock,
    },
    core::{IDockControl, IDockWindow, IFactory, IHostWindow},
};
use ferrodock_model_ferro::{
    DockableControlMapReference, DockableObjectMapReference, FactoryBaseReference,
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
/// Native source Factory class view retaining the original factory engine.
#[derive(Clone)]
pub struct FactoryReference {
    model: Rc<dyn IFactory>,
    base: FactoryBaseReference,
}
impl FactoryReference {
    /// Creates the genuine source ReactiveUI factory and its fresh source registries.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::Factory::new())
    }
    /// Views a source factory or composed subclass without copying engine state.
    pub fn new<T: ferrodock_model_reactiveui::FactoryImpl + 'static>(model: Rc<T>) -> Self {
        Self {
            base: FactoryBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same original enclosing factory.
    pub fn model(&self) -> Rc<dyn IFactory> {
        self.model.clone()
    }
    /// Gets the source abstract base view of the same enclosing factory.
    pub fn base(&self) -> FactoryBaseReference {
        self.base.clone()
    }
}
impl PartialEq for FactoryReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
ferroui_base::ferro_markup_type!(class FactoryReference as "Factory" {
 namespace:"FerroDock.Model.ReactiveUI",this:FactoryReference,handles:[FactoryReference,Option<FactoryReference>],base:FactoryBaseReference,interfaces:[Rc<dyn IFactory>],constructors:[()=>FactoryReference::create],
 properties:[
 VisibleDockableControls:DockableControlMapReference{get:|this:&FactoryReference|DockableControlMapReference::new(this.model.visible_dockable_controls())},
 VisibleRootControls:DockableObjectMapReference{get:|this:&FactoryReference|DockableObjectMapReference::new(this.model.visible_root_controls())},
 PinnedDockableControls:DockableControlMapReference{get:|this:&FactoryReference|DockableControlMapReference::new(this.model.pinned_dockable_controls())},
 PinnedRootControls:DockableObjectMapReference{get:|this:&FactoryReference|DockableObjectMapReference::new(this.model.pinned_root_controls())},
 TabDockableControls:DockableControlMapReference{get:|this:&FactoryReference|DockableControlMapReference::new(this.model.tab_dockable_controls())},
 TabRootControls:DockableObjectMapReference{get:|this:&FactoryReference|DockableObjectMapReference::new(this.model.tab_root_controls())},
 ToolControls:DockableObjectMapReference{get:|this:&FactoryReference|DockableObjectMapReference::new(this.model.tool_controls())},
 DocumentControls:DockableObjectMapReference{get:|this:&FactoryReference|DockableObjectMapReference::new(this.model.document_controls())},
 DockControls:ListHandle<Rc<dyn IDockControl>>{get:|this:&FactoryReference|this.model.dock_controls()},
 HostWindows:ListHandle<Rc<dyn IHostWindow>>{get:|this:&FactoryReference|this.model.host_windows()},
 ],methods:[
 fn CreateRootDock()->Rc<dyn IRootDock> =>(|this:&FactoryReference|this.model.create_root_dock()),
 fn CreateProportionalDock()->Rc<dyn IProportionalDock> =>(|this:&FactoryReference|this.model.create_proportional_dock()),
 fn CreateDockDock()->Rc<dyn IDockDock> =>(|this:&FactoryReference|this.model.create_dock_dock()),
 fn CreateStackDock()->Rc<dyn IStackDock> =>(|this:&FactoryReference|this.model.create_stack_dock()),
 fn CreateGridDock()->Rc<dyn IGridDock> =>(|this:&FactoryReference|this.model.create_grid_dock()),
 fn CreateWrapDock()->Rc<dyn IWrapDock> =>(|this:&FactoryReference|this.model.create_wrap_dock()),
 fn CreateUniformGridDock()->Rc<dyn IUniformGridDock> =>(|this:&FactoryReference|this.model.create_uniform_grid_dock()),
 fn CreateProportionalDockSplitter()->Rc<dyn IProportionalDockSplitter> =>(|this:&FactoryReference|this.model.create_proportional_dock_splitter()),
 fn CreateGridDockSplitter()->Rc<dyn IGridDockSplitter> =>(|this:&FactoryReference|this.model.create_grid_dock_splitter()),
 fn CreateToolDock()->Rc<dyn IToolDock> =>(|this:&FactoryReference|this.model.create_tool_dock()),
 fn CreateDocumentDock()->Rc<dyn IDocumentDock> =>(|this:&FactoryReference|this.model.create_document_dock()),
 fn CreateSplitViewDock()->Rc<dyn ISplitViewDock> =>(|this:&FactoryReference|this.model.create_split_view_dock()),
 fn CreateDockWindow()->Rc<dyn IDockWindow> =>(|this:&FactoryReference|this.model.create_dock_window()),
 fn CreateDocument()->Rc<dyn IDocument> =>(|this:&FactoryReference|this.model.create_document()),
 fn CreateTool()->Rc<dyn ITool> =>(|this:&FactoryReference|this.model.create_tool()),
 fn CreateLayout()->Option<Rc<dyn IRootDock>> =>(|this:&FactoryReference|this.model.create_layout()),
 ],
});
pub(crate) fn register() {
    MarkupType::register(FactoryReference::MARKUP);
    ValueTypes::register_nullable::<FactoryReference>();
    ValueTypes::register_reference_identity::<FactoryReference>(|v| {
        Rc::as_ptr(&v.model) as *const ()
    });
    ValueTypes::register_cast::<FactoryReference, FactoryBaseReference>(|v| v.base());
    ValueTypes::register_cast::<FactoryReference, Rc<dyn IFactory>>(|v| v.model());
    ValueTypes::register_reference::<ferrodock_model_reactiveui::Factory>();
    ValueTypes::register_cast::<Rc<ferrodock_model_reactiveui::Factory>, FactoryReference>(|v| {
        FactoryReference::new(v.clone())
    });
    ValueTypes::register_cast::<Rc<ferrodock_model_reactiveui::Factory>, FactoryBaseReference>(
        |v| FactoryBaseReference::new(v.clone()),
    );
    ValueTypes::register_cast::<Rc<ferrodock_model_reactiveui::Factory>, Rc<dyn IFactory>>(|v| {
        v.clone()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IFactory>, FactoryReference>(|v| {
        let original = v
            .downcast_ref::<Rc<dyn IFactory>>()?
            .clone()
            .boxed_reference();
        if original.as_ref().type_id() == std::any::TypeId::of::<Rc<dyn IFactory>>() {
            return None;
        }
        let probe: ferroui_base::BoxedValue = Rc::new(super::model_probe::ModelProbe(original));
        ValueTypes::try_cast(
            &probe,
            ferroui_base::data::core::ValueType::of::<FactoryReference>(),
        )?
        .downcast_ref::<FactoryReference>()
        .cloned()
    });
    MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::Factory>>(
        FactoryReference::MARKUP,
    );
}
