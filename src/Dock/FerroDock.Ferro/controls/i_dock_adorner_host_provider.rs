// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{Ref, Visual};
use ferroui_controls::Control;
/// Selects an enclosing native control for a local docking preview.
pub trait IDockAdornerHostProvider {
    /// Resolves a preview host without changing the model or visual hierarchy.
    fn get_adorner_host(&self, surface: &Visual) -> Option<Ref<Control>>;
}
impl PartialEq for dyn IDockAdornerHostProvider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(interface dyn IDockAdornerHostProvider as "IDockAdornerHostProvider" {
    this:std::rc::Rc<dyn IDockAdornerHostProvider>,
    handles:[std::rc::Rc<dyn IDockAdornerHostProvider>,Option<std::rc::Rc<dyn IDockAdornerHostProvider>>],
    properties:[]
});
