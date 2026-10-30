//! Closed metadata facts for the original engine model classes.
use ferrodock_model_reactiveui::controls::{Document, DocumentDock, RootDock, Tool};
use std::rc::Rc;
pub(crate) fn register() {
    ferroui_base::data::core::ValueTypes::register_reference::<Document>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Document>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<Document>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<Document>>()
            .cloned()
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Document>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Document>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Document>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_conversion::<Rc<Document>, String>(|_| {
        ferroui_base::metadata::MarkupType::find_by_handle(std::any::TypeId::of::<Rc<Document>>())
            .map(|meta| meta.full_name())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Document>,
        crate::reactive_metadata::DocumentReference,
    >(|model| crate::reactive_metadata::DocumentReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Document>,
        Rc<dyn ferrodock_model::controls::IDocument>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Document>,
        Rc<dyn ferrodock_model::controls::IMdiDocument>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_reference::<Tool>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<Tool>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<Tool>>()
            .cloned()
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_conversion::<Rc<Tool>, String>(|_| {
        ferroui_base::metadata::MarkupType::find_by_handle(std::any::TypeId::of::<Rc<Tool>>())
            .map(|meta| meta.full_name())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool>,
        crate::reactive_metadata::ToolReference,
    >(|model| crate::reactive_metadata::ToolReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool>,
        Rc<dyn ferrodock_model::controls::ITool>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool>,
        Rc<dyn ferrodock_model::controls::IDocument>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<Tool>,
        Rc<dyn ferrodock_model::controls::IMdiDocument>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_reference::<RootDock>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<RootDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<RootDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<RootDock>>()
            .cloned()
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<RootDock>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<RootDock>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<RootDock>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_conversion::<Rc<RootDock>, String>(|_| {
        ferroui_base::metadata::MarkupType::find_by_handle(std::any::TypeId::of::<Rc<RootDock>>())
            .map(|meta| meta.full_name())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<RootDock>,
        crate::reactive_metadata::DockBaseReference,
    >(|model| crate::reactive_metadata::DockBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<RootDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<RootDock>,
        crate::reactive_metadata::RootDockReference,
    >(|model| crate::reactive_metadata::RootDockReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<RootDock>,
        Rc<dyn ferrodock_model::controls::IRootDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_reference::<DocumentDock>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentDock>,
        Rc<dyn ferrodock_model::controls::IDocumentDockFactory>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<DocumentDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<DocumentDock>>()
            .cloned()
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentDock>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentDock>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentDock>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_conversion::<Rc<DocumentDock>, String>(|_| {
        ferroui_base::metadata::MarkupType::find_by_handle(
            std::any::TypeId::of::<Rc<DocumentDock>>(),
        )
        .map(|meta| meta.full_name())
    });
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentDock>,
        crate::reactive_metadata::DockBaseReference,
    >(|model| crate::reactive_metadata::DockBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentDock>,
        crate::reactive_metadata::DocumentDockReference,
    >(|model| crate::reactive_metadata::DocumentDockReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<DocumentDock>,
        Rc<dyn ferrodock_model::controls::IDocumentDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_reference::<
        ferrodock_model_reactiveui::controls::ToolDock,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ToolDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ToolDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ToolDock>,
        Rc<dyn ferrodock_model::controls::IToolDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ToolDock>,
        Rc<dyn ferrodock_model::core::ILocalTarget>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ToolDock>,
        crate::reactive_metadata::DockBaseReference,
    >(|model| crate::reactive_metadata::DockBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ToolDock>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ToolDock>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ToolDock>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ToolDock>,
        crate::reactive_metadata::ToolDockReference,
    >(|model| crate::reactive_metadata::ToolDockReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<ferrodock_model_reactiveui::controls::ToolDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::controls::ToolDock>>()
            .cloned()
    });
    ferroui_base::metadata::MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::ToolDock>>(<crate::reactive_metadata::ToolDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    crate::reactive_metadata::register_model_name::<ferrodock_model_reactiveui::controls::ToolDock>(<crate::reactive_metadata::ToolDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_reference::<
        ferrodock_model_reactiveui::controls::ProportionalDock,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDock>,
        Rc<dyn ferrodock_model::controls::IProportionalDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDock>,
        Rc<dyn ferrodock_model::core::IGlobalTarget>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDock>,
        crate::reactive_metadata::DockBaseReference,
    >(|model| crate::reactive_metadata::DockBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDock>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDock>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDock>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDock>,
        crate::reactive_metadata::ProportionalDockReference,
    >(|model| crate::reactive_metadata::ProportionalDockReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<ferrodock_model_reactiveui::controls::ProportionalDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::controls::ProportionalDock>>()
            .cloned()
    });
    ferroui_base::metadata::MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::ProportionalDock>>(<crate::reactive_metadata::ProportionalDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    crate::reactive_metadata::register_model_name::<ferrodock_model_reactiveui::controls::ProportionalDock>(<crate::reactive_metadata::ProportionalDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_reference::<
        ferrodock_model_reactiveui::controls::StackDock,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::StackDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::StackDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::StackDock>,
        Rc<dyn ferrodock_model::controls::IStackDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::StackDock>,
        crate::reactive_metadata::DockBaseReference,
    >(|model| crate::reactive_metadata::DockBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::StackDock>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::StackDock>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::StackDock>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::StackDock>,
        crate::reactive_metadata::StackDockReference,
    >(|model| crate::reactive_metadata::StackDockReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<ferrodock_model_reactiveui::controls::StackDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::controls::StackDock>>()
            .cloned()
    });
    ferroui_base::metadata::MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::StackDock>>(<crate::reactive_metadata::StackDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    crate::reactive_metadata::register_model_name::<ferrodock_model_reactiveui::controls::StackDock>(<crate::reactive_metadata::StackDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_reference::<
        ferrodock_model_reactiveui::controls::GridDock,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDock>,
        Rc<dyn ferrodock_model::controls::IGridDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDock>,
        crate::reactive_metadata::DockBaseReference,
    >(|model| crate::reactive_metadata::DockBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDock>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDock>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDock>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDock>,
        crate::reactive_metadata::GridDockReference,
    >(|model| crate::reactive_metadata::GridDockReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<ferrodock_model_reactiveui::controls::GridDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::controls::GridDock>>()
            .cloned()
    });
    ferroui_base::metadata::MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::GridDock>>(<crate::reactive_metadata::GridDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    crate::reactive_metadata::register_model_name::<ferrodock_model_reactiveui::controls::GridDock>(<crate::reactive_metadata::GridDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_reference::<
        ferrodock_model_reactiveui::controls::DockDock,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::DockDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::DockDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::DockDock>,
        Rc<dyn ferrodock_model::controls::IDockDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::DockDock>,
        crate::reactive_metadata::DockBaseReference,
    >(|model| crate::reactive_metadata::DockBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::DockDock>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::DockDock>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::DockDock>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::DockDock>,
        crate::reactive_metadata::DockDockReference,
    >(|model| crate::reactive_metadata::DockDockReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<ferrodock_model_reactiveui::controls::DockDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::controls::DockDock>>()
            .cloned()
    });
    ferroui_base::metadata::MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::DockDock>>(<crate::reactive_metadata::DockDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    crate::reactive_metadata::register_model_name::<ferrodock_model_reactiveui::controls::DockDock>(<crate::reactive_metadata::DockDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_reference::<
        ferrodock_model_reactiveui::controls::WrapDock,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::WrapDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::WrapDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::WrapDock>,
        Rc<dyn ferrodock_model::controls::IWrapDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::WrapDock>,
        crate::reactive_metadata::DockBaseReference,
    >(|model| crate::reactive_metadata::DockBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::WrapDock>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::WrapDock>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::WrapDock>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::WrapDock>,
        crate::reactive_metadata::WrapDockReference,
    >(|model| crate::reactive_metadata::WrapDockReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<ferrodock_model_reactiveui::controls::WrapDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::controls::WrapDock>>()
            .cloned()
    });
    ferroui_base::metadata::MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::WrapDock>>(<crate::reactive_metadata::WrapDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    crate::reactive_metadata::register_model_name::<ferrodock_model_reactiveui::controls::WrapDock>(<crate::reactive_metadata::WrapDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_reference::<
        ferrodock_model_reactiveui::controls::UniformGridDock,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::UniformGridDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::UniformGridDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::UniformGridDock>,
        Rc<dyn ferrodock_model::controls::IUniformGridDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::UniformGridDock>,
        crate::reactive_metadata::DockBaseReference,
    >(|model| crate::reactive_metadata::DockBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::UniformGridDock>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::UniformGridDock>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::UniformGridDock>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::UniformGridDock>,
        crate::reactive_metadata::UniformGridDockReference,
    >(|model| crate::reactive_metadata::UniformGridDockReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<ferrodock_model_reactiveui::controls::UniformGridDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::controls::UniformGridDock>>()
            .cloned()
    });
    ferroui_base::metadata::MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::UniformGridDock>>(<crate::reactive_metadata::UniformGridDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    crate::reactive_metadata::register_model_name::<ferrodock_model_reactiveui::controls::UniformGridDock>(<crate::reactive_metadata::UniformGridDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_reference::<
        ferrodock_model_reactiveui::controls::SplitViewDock,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::SplitViewDock>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::SplitViewDock>,
        Rc<dyn ferrodock_model::core::IDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::SplitViewDock>,
        Rc<dyn ferrodock_model::controls::ISplitViewDock>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::SplitViewDock>,
        crate::reactive_metadata::DockBaseReference,
    >(|model| crate::reactive_metadata::DockBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::SplitViewDock>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::SplitViewDock>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::SplitViewDock>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::SplitViewDock>,
        crate::reactive_metadata::SplitViewDockReference,
    >(|model| crate::reactive_metadata::SplitViewDockReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<ferrodock_model_reactiveui::controls::SplitViewDock>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::controls::SplitViewDock>>()
            .cloned()
    });
    ferroui_base::metadata::MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::SplitViewDock>>(<crate::reactive_metadata::SplitViewDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    crate::reactive_metadata::register_model_name::<ferrodock_model_reactiveui::controls::SplitViewDock>(<crate::reactive_metadata::SplitViewDockReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_reference::<
        ferrodock_model_reactiveui::controls::ProportionalDockSplitter,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>,
        Rc<dyn ferrodock_model::controls::IProportionalDockSplitter>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>,
        Rc<dyn ferrodock_model::controls::ISplitter>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>,
        crate::reactive_metadata::ProportionalDockSplitterReference,
    >(|model| crate::reactive_metadata::ProportionalDockSplitterReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>>()
            .cloned()
    });
    ferroui_base::metadata::MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>>(<crate::reactive_metadata::ProportionalDockSplitterReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    crate::reactive_metadata::register_model_name::<ferrodock_model_reactiveui::controls::ProportionalDockSplitter>(<crate::reactive_metadata::ProportionalDockSplitterReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_reference::<
        ferrodock_model_reactiveui::controls::GridDockSplitter,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDockSplitter>,
        Rc<dyn ferrodock_model::core::IDockable>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDockSplitter>,
        Rc<dyn ferrodock_model::controls::IGridDockSplitter>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDockSplitter>,
        Rc<dyn ferrodock_model::controls::ISplitter>,
    >(|model| model.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDockSplitter>,
        crate::reactive_metadata::DockableBaseReference,
    >(|model| crate::reactive_metadata::DockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDockSplitter>,
        crate::reactive_metadata::ReactiveBaseReference,
    >(|model| crate::reactive_metadata::ReactiveBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDockSplitter>,
        crate::reactive_metadata::ReactiveObjectReference,
    >(|model| crate::reactive_metadata::ReactiveObjectReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::controls::GridDockSplitter>,
        crate::reactive_metadata::GridDockSplitterReference,
    >(|model| crate::reactive_metadata::GridDockSplitterReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_boxed_cast::<
        Rc<dyn ferrodock_model::core::IDockable>,
        Rc<ferrodock_model_reactiveui::controls::GridDockSplitter>,
    >(|value| {
        value
            .downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::controls::GridDockSplitter>>()
            .cloned()
    });
    ferroui_base::metadata::MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::controls::GridDockSplitter>>(<crate::reactive_metadata::GridDockSplitterReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
    crate::reactive_metadata::register_model_name::<ferrodock_model_reactiveui::controls::GridDockSplitter>(<crate::reactive_metadata::GridDockSplitterReference as ferroui_base::metadata::MarkupTyped>::MARKUP);
}
