// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! UI projection augments the Core-owned original contract registration.
use ferrodock_model::core::{IDockable, IGlobalTarget};
use ferroui_base::data::core::ValueTypes;
use std::rc::Rc;
pub(crate) fn register() {
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IGlobalTarget>>(|value| {
        crate::model_projection::dockable(Some(value))?.as_global_target()
    });
}
