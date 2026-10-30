// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{ManagedDockWindowDocument, ManagedDockableBaseReference, ManagedWindowDock};
use ferrodock_controls_deferred_content_control::IDeferredContentPresentation;
use ferrodock_model::{
    controls::{IDocumentContent, IMdiDocument},
    core::{IDock, IDockable},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
    BoxedValue,
};
use ferroui_controls::templates::{IDataTemplate, IRecyclingDataTemplate};
use std::rc::Rc;
pub(crate) fn register() {
    ValueTypes::register_nullable::<Rc<dyn IDataTemplate>>();
    ValueTypes::register_nullable::<Rc<dyn IRecyclingDataTemplate>>();
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<ManagedDockWindowDocument>>(document);
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDataTemplate>>(|value| {
        interface::<dyn IDataTemplate>(value)
            .or_else(|| document(value).map(|model| model as Rc<dyn IDataTemplate>))
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IRecyclingDataTemplate>>(|value| {
        interface::<dyn IRecyclingDataTemplate>(value)
            .or_else(|| document(value).map(|model| model as Rc<dyn IRecyclingDataTemplate>))
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDeferredContentPresentation>>(
        |value| {
            interface::<dyn IDeferredContentPresentation>(value).or_else(|| {
                document(value).map(|model| model as Rc<dyn IDeferredContentPresentation>)
            })
        },
    );
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, ManagedDockableBaseReference>(
        managed_base,
    );
    ValueTypes::register_boxed_cast::<Rc<dyn IMdiDocument>, Rc<ManagedDockWindowDocument>>(
        document,
    );
    ValueTypes::register_boxed_cast::<Rc<dyn IMdiDocument>, Rc<dyn IDataTemplate>>(|value| {
        interface::<dyn IDataTemplate>(value)
            .or_else(|| document(value).map(|model| model as Rc<dyn IDataTemplate>))
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IMdiDocument>, Rc<dyn IRecyclingDataTemplate>>(
        |value| {
            interface::<dyn IRecyclingDataTemplate>(value)
                .or_else(|| document(value).map(|model| model as Rc<dyn IRecyclingDataTemplate>))
        },
    );
    ValueTypes::register_boxed_cast::<Rc<dyn IMdiDocument>, Rc<dyn IDeferredContentPresentation>>(
        |value| {
            interface::<dyn IDeferredContentPresentation>(value).or_else(|| {
                document(value).map(|model| model as Rc<dyn IDeferredContentPresentation>)
            })
        },
    );
    ValueTypes::register_boxed_cast::<Rc<dyn IMdiDocument>, ManagedDockableBaseReference>(
        managed_base,
    );
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<ManagedWindowDock>>(dock);
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<ManagedWindowDock>>(dock);
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, ManagedDockableBaseReference>(managed_base);
    MarkupType::register_all(&[ManagedDockWindowDocument::MARKUP, ManagedWindowDock::MARKUP]);
    ValueTypes::register_reference::<ManagedDockWindowDocument>();
    ValueTypes::register_conversion::<Rc<ManagedDockWindowDocument>, Option<Rc<dyn IDataTemplate>>>(
        |model| Some(Some(model.clone())),
    );
    ValueTypes::register_conversion::<
        Option<Rc<ManagedDockWindowDocument>>,
        Option<Rc<dyn IDataTemplate>>,
    >(|model| {
        Some(
            model
                .as_ref()
                .map(|model| model.clone() as Rc<dyn IDataTemplate>),
        )
    });

    ValueTypes::register_reference::<ManagedWindowDock>();
    ValueTypes::register_cast::<Rc<ManagedDockWindowDocument>, Rc<dyn IDockable>>(|m| m.clone());
    ValueTypes::register_cast::<Rc<ManagedDockWindowDocument>, Rc<dyn IMdiDocument>>(|m| m.clone());
    ValueTypes::register_cast::<Rc<ManagedDockWindowDocument>, Rc<dyn IDocumentContent>>(|m| {
        m.clone()
    });
    ValueTypes::register_cast::<Rc<ManagedDockWindowDocument>, Rc<dyn IDataTemplate>>(|m| {
        m.clone()
    });
    ValueTypes::register_cast::<Rc<ManagedDockWindowDocument>, Rc<dyn IRecyclingDataTemplate>>(
        |m| m.clone(),
    );
    ValueTypes::register_cast::<Rc<ManagedDockWindowDocument>, Rc<dyn IDeferredContentPresentation>>(
        |m| m.clone(),
    );
    ValueTypes::register_cast::<Rc<ManagedDockWindowDocument>, ManagedDockableBaseReference>(|m| {
        ManagedDockableBaseReference::new(m.clone())
    });
    ValueTypes::register_cast::<Rc<ManagedWindowDock>, Rc<dyn IDock>>(|m| m.clone());
    // Native property conversion consults direct conversions, while metadata
    // assignability can traverse reference/nullable casts. State both forms
    // of the same original reference at the styled nullable Dock boundary.
    ValueTypes::register_conversion::<Rc<ManagedWindowDock>, Option<Rc<dyn IDock>>>(|m| {
        Some(Some(m.clone()))
    });
    ValueTypes::register_boxed_conversion::<ManagedWindowDock, Option<Rc<dyn IDock>>>(|value| {
        let model = ferroui_base::metadata::from_markup_value::<Rc<ManagedWindowDock>>(&Some(
            value.clone(),
        ))?;
        Some(Some(model))
    });
    ValueTypes::register_cast::<Rc<ManagedWindowDock>, Rc<dyn IDockable>>(|m| m.clone());
    ValueTypes::register_cast::<Rc<ManagedWindowDock>, ManagedDockableBaseReference>(|m| {
        ManagedDockableBaseReference::new(m.clone())
    });
}

fn document(value: &BoxedValue) -> Option<Rc<ManagedDockWindowDocument>> {
    crate::model_projection::dockable(Some(value))?
        .boxed_reference()
        .downcast_ref::<Rc<ManagedDockWindowDocument>>()
        .cloned()
}
fn dock(value: &BoxedValue) -> Option<Rc<ManagedWindowDock>> {
    crate::model_projection::dockable(Some(value))?
        .boxed_reference()
        .downcast_ref::<Rc<ManagedWindowDock>>()
        .cloned()
}
fn managed_base(value: &BoxedValue) -> Option<ManagedDockableBaseReference> {
    if let Some(model) = interface::<dyn super::ManagedDockableBaseImpl>(value) {
        return Some(ManagedDockableBaseReference::new(model));
    }
    if let Some(document) = document(value) {
        return Some(ManagedDockableBaseReference::new(document));
    }
    dock(value).map(|dock| ManagedDockableBaseReference::new(dock))
}

fn interface<T: ?Sized + 'static>(value: &BoxedValue) -> Option<Rc<T>> {
    crate::model_projection::dockable(Some(value))?
        .boxed_interface(std::any::TypeId::of::<Rc<T>>())?
        .downcast_ref::<Rc<T>>()
        .cloned()
}
