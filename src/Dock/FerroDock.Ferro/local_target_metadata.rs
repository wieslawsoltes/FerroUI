// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! UI projection augments the Core-owned original contract registration.
use ferrodock_model::core::{IDockable, ILocalTarget};
use ferroui_base::data::core::ValueTypes;
use std::rc::Rc;
pub(crate) fn register() {
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn ILocalTarget>>(|value| {
        crate::model_projection::dockable(Some(value))?.as_local_target()
    });
}
