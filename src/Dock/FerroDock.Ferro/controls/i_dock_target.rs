// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
/// Original contract implemented by reusable local and global dock targets.
pub trait IDockTarget {
    /// Resets the target's state for reuse.
    fn reset(&self);
}
impl PartialEq for dyn IDockTarget {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(interface dyn IDockTarget as "IDockTarget" {
    this:std::rc::Rc<dyn IDockTarget>,
    handles:[std::rc::Rc<dyn IDockTarget>,Option<std::rc::Rc<dyn IDockTarget>>],
    properties:[]
});
