//! Projections of original model handles at the native DataContext boundary.
use ferrodock_model::controls::{IMdiDocument, IRootDock};
use ferrodock_model::core::IDockable;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::BoxedValue;
use std::rc::Rc;
pub(crate) fn root(value: Option<&BoxedValue>) -> Option<Rc<dyn IRootDock>> {
    let value = value?;
    if let Some(reference) = value.downcast_ref::<crate::controls::ManagedDockableBaseReference>() {
        return reference.model().as_root_dock();
    }
    if let Some(root) = value.downcast_ref::<Rc<dyn IRootDock>>() {
        return Some(root.clone());
    }
    if let Some(dockable) = value.downcast_ref::<Rc<dyn IDockable>>() {
        return dockable.clone().as_root_dock();
    }
    ValueTypes::try_cast(value, ValueType::of::<Rc<dyn IRootDock>>())?
        .downcast_ref::<Rc<dyn IRootDock>>()
        .cloned()
}
pub(crate) fn dockable(value: Option<&BoxedValue>) -> Option<Rc<dyn IDockable>> {
    let value = value?;
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IToolContent>>() {
        return Some(model.clone());
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IDocumentContent>>()
    {
        return Some(model.clone());
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::ISplitViewDock>>() {
        return Some(model.clone());
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IUniformGridDock>>()
    {
        return Some(model.clone());
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IWrapDock>>() {
        return Some(model.clone());
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IStackDock>>() {
        return Some(model.clone());
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IGridDock>>() {
        return Some(model.clone());
    }
    if let Some(model) =
        value.downcast_ref::<Rc<dyn ferrodock_model::controls::IProportionalDock>>()
    {
        return Some(model.clone());
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IDockDock>>() {
        return Some(model.clone());
    }
    if let Some(reference) = value.downcast_ref::<crate::controls::ManagedDockableBaseReference>() {
        return Some(reference.model());
    }
    if let Some(metadata) =
        value.downcast_ref::<Rc<dyn ferrodock_model::core::IDockItemContainerMetadata>>()
    {
        return metadata.clone().as_dockable();
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IDocumentContent>>()
    {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IToolContent>>() {
        return Some(value.clone());
    }
    if let Some(value) =
        value.downcast_ref::<Rc<dyn ferrodock_model::controls::IProportionalDockSplitter>>()
    {
        return Some(value.clone());
    }
    if let Some(value) =
        value.downcast_ref::<Rc<dyn ferrodock_model::controls::IGridDockSplitter>>()
    {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IDocumentDock>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IToolDock>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::ISplitViewDock>>() {
        return Some(value.clone());
    }
    if let Some(value) =
        value.downcast_ref::<Rc<dyn ferrodock_model::controls::IProportionalDock>>()
    {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IStackDock>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IGridDock>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IWrapDock>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IUniformGridDock>>()
    {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IDockDock>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::core::IDock>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::ISplitter>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IDocument>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::ITool>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn IMdiDocument>>() {
        return Some(value.clone());
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn IDockable>>() {
        return Some(value.clone());
    }
    if let Some(root) = root(Some(value)) {
        return Some(root);
    }
    ValueTypes::try_cast(value, ValueType::of::<Rc<dyn IDockable>>())?
        .downcast_ref::<Rc<dyn IDockable>>()
        .cloned()
}

/// Boxes an original root interface handle for a native DataContext.
/// The same model is retained. Concrete raw models can instead register their
/// casts with shared `ValueTypes` in the application's UI composition root.
pub fn to_root_value(root: Rc<dyn IRootDock>) -> BoxedValue {
    Rc::new(root)
}
/// Boxes an original dockable interface handle for a native DataContext.
/// The same model is retained without a replacement ViewModel or UI model type.
pub fn to_dockable_value(dockable: Rc<dyn IDockable>) -> BoxedValue {
    Rc::new(dockable)
}

pub(crate) fn mdi_document(value: Option<&BoxedValue>) -> Option<Rc<dyn IMdiDocument>> {
    let value = value?;
    if let Some(document) = value.downcast_ref::<Rc<dyn IMdiDocument>>() {
        return Some(document.clone());
    }
    if let Some(document) = dockable(Some(value)).and_then(|dockable| dockable.as_mdi_document()) {
        return Some(document);
    }
    ValueTypes::try_cast(value, ValueType::of::<Rc<dyn IMdiDocument>>())?
        .downcast_ref::<Rc<dyn IMdiDocument>>()
        .cloned()
}
/// Boxes the original MDI document interface for native DataContext use.
/// No replacement document or framework model is constructed.
pub fn to_mdi_document_value(document: Rc<dyn IMdiDocument>) -> BoxedValue {
    Rc::new(document)
}

/// Carries an original managed model into native compiled bindings and controls.
/// The reference adapter forwards the original PropertyChanged event and retains
/// the same model identity; it stores no substitute model state.
pub fn to_managed_dockable_value(
    model: Rc<dyn crate::controls::ManagedDockableBaseImpl>,
) -> BoxedValue {
    Rc::new(crate::controls::ManagedDockableBaseReference::new(model))
}

/// Boxes original optional generated-container metadata for a native DataContext.
/// The handle can implement metadata without being a dockable, as in the source.
pub fn to_item_container_metadata_value(
    metadata: Rc<dyn ferrodock_model::core::IDockItemContainerMetadata>,
) -> BoxedValue {
    Rc::new(metadata)
}

/// Borrows original optional notifications from canonical model interface handles.
/// Registered raw models use the shared framework metadata lookup.
pub(crate) fn notifier(
    value: &dyn ferroui_base::AnyValue,
) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::core::IDockable>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::core::IDock>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IDocumentContent>>()
    {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IToolContent>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) =
        value.downcast_ref::<Rc<dyn ferrodock_model::controls::IProportionalDockSplitter>>()
    {
        return model.as_notify_property_changed();
    }
    if let Some(model) =
        value.downcast_ref::<Rc<dyn ferrodock_model::controls::IGridDockSplitter>>()
    {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IDocumentDock>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IToolDock>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::ISplitViewDock>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) =
        value.downcast_ref::<Rc<dyn ferrodock_model::controls::IProportionalDock>>()
    {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IStackDock>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IGridDock>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IWrapDock>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IUniformGridDock>>()
    {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IDockDock>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IRootDock>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::ISplitter>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IDocument>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::ITool>>() {
        return model.as_notify_property_changed();
    }
    if let Some(model) = value.downcast_ref::<Rc<dyn ferrodock_model::controls::IMdiDocument>>() {
        return model.as_notify_property_changed();
    }
    ferroui_base::data::core::plugins::InpcPropertyAccessor::find_notifier(value)
}
