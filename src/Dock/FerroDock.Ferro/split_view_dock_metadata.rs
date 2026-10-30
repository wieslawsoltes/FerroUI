// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! UI-specific model projections augment the original Core-owned contract.
use ferrodock_model::{
    controls::ISplitViewDock,
    core::{IDock, IDockable},
};
use ferroui_base::data::core::ValueTypes;
use std::rc::Rc;
pub(crate) fn register() {
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn ISplitViewDock>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_split_view_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn ISplitViewDock>>(|v| {
        crate::model_projection::dockable(Some(v))?.as_split_view_dock()
    });
}
