// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::{DockOperation, DragAction};
use ferroui_base::{Point, Visual};
/// Evaluates an operation at a pointer position relative to the supplied visual.
pub type DockOperationHandler<'a> = dyn Fn(Point, DockOperation, DragAction, &Visual) -> bool + 'a;
