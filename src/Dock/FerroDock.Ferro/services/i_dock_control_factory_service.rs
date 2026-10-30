// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::controls::DockControl;
use ferrodock_model::core::IDock;
use std::rc::Rc;
pub(crate) trait IDockControlFactoryService {
    fn initialize_control_recycling(&self, control: &DockControl);
    fn cleanup_factory(&self, control: &DockControl, layout: Rc<dyn IDock>);
}
