// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! UI-specific model projections augment the original Core-owned contract.
use ferrodock_model::{
    controls::{IProportionalDockSplitter, ISplitter},
    core::IDockable,
};
use ferroui_base::data::core::ValueTypes;
use std::rc::Rc;
pub(crate) fn register() {
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IProportionalDockSplitter>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_proportional_dock_splitter()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn ISplitter>, Rc<dyn IProportionalDockSplitter>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_proportional_dock_splitter()
    });
}
