// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{
    controls::IProportionalDock,
    core::{IDockable, IGlobalTarget},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
/// Descriptor carrier for the original global-target marker.
#[doc(hidden)]
pub trait IGlobalTargetMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IGlobalTargetMetadata as "IGlobalTarget"{namespace:"FerroDock.Model.Core",handles:[Rc<dyn IGlobalTarget>,Option<Rc<dyn IGlobalTarget>>]});
pub(crate) fn register() {
    MarkupType::register(<dyn IGlobalTargetMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IGlobalTarget>>();
    ValueTypes::register_reference_identity::<Rc<dyn IGlobalTarget>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IProportionalDock>, Rc<dyn IGlobalTarget>>(|p| p.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IGlobalTarget>>(|v| {
        v.downcast_ref::<Rc<dyn IDockable>>()?
            .clone()
            .as_global_target()
    });
}
