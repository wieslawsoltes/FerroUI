// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{
    controls::{IDocument, IDocumentDock, ITool, IToolDock},
    core::{IDockable, ILocalTarget},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
/// Descriptor carrier for the original local-target marker.
#[doc(hidden)]
pub trait ILocalTargetMetadata {}
ferroui_base::ferro_markup_type!(interface dyn ILocalTargetMetadata as "ILocalTarget"{namespace:"FerroDock.Model.Core",handles:[Rc<dyn ILocalTarget>,Option<Rc<dyn ILocalTarget>>]});
pub(crate) fn register() {
    MarkupType::register(<dyn ILocalTargetMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn ILocalTarget>>();
    ValueTypes::register_reference_identity::<Rc<dyn ILocalTarget>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn ITool>, Rc<dyn ILocalTarget>>(|model| model.clone());
    ValueTypes::register_cast::<Rc<dyn IDocument>, Rc<dyn ILocalTarget>>(|model| model.clone());
    ValueTypes::register_cast::<Rc<dyn IDocumentDock>, Rc<dyn ILocalTarget>>(|model| model.clone());
    ValueTypes::register_cast::<Rc<dyn IToolDock>, Rc<dyn ILocalTarget>>(|model| model.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn ILocalTarget>>(|v| {
        v.downcast_ref::<Rc<dyn IDockable>>()?
            .clone()
            .as_local_target()
    });
}
