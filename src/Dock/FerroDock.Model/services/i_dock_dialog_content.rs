// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::BoxedValue;
use std::rc::Rc;
/// Callback by which dialog content closes itself with a nullable result.
pub type DialogContentCloseAction = Rc<dyn Fn(Option<BoxedValue>)>;
/// Source hook through which dialog content receives its close callback.
pub trait IDockDialogContent {
    /// Supplies the close action used to dismiss this content.
    fn set_close_action(&self, close_action: DialogContentCloseAction);
}
