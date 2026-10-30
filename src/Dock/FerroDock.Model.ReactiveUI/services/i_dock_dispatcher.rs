// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_system::threading::tasks::Task;
use std::rc::Rc;
/// UI-thread dispatch without a dependency on framework controls or windowing.
pub trait IDockDispatcher {
    /// Executes the action on the dispatcher scheduler; the task completes with
    /// its outcome. The action handle can be reused in subsequent invocations.
    fn invoke_async(&self, action: Rc<dyn Fn()>) -> Task<()>;
}
