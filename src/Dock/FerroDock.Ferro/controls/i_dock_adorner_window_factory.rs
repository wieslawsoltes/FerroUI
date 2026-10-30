// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockAdornerWindow;
use ferroui_base::{Ref, Visual};
use ferroui_controls::Control;
/// Creates a floating host for the existing docking selectors and indicators.
pub trait IDockAdornerWindowFactory {
    /// Creates an unshown host, or returns None to use the standard host.
    fn create_window(
        &self,
        adorned_element: &Visual,
        adorner: &Control,
    ) -> Option<Ref<DockAdornerWindow>>;
}
impl PartialEq for dyn IDockAdornerWindowFactory {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(interface dyn IDockAdornerWindowFactory as "IDockAdornerWindowFactory" {
    this:std::rc::Rc<dyn IDockAdornerWindowFactory>,
    handles:[std::rc::Rc<dyn IDockAdornerWindowFactory>,Option<std::rc::Rc<dyn IDockAdornerWindowFactory>>],
    properties:[]
});
