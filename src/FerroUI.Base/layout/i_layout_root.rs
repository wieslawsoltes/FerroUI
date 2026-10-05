use super::{ILayoutManager, Layoutable};
use crate::Ref;
use std::rc::Rc;

/// Defines the root of a tree that can be laid out.
pub trait ILayoutRoot {
    /// The scaling factor to use in layout.
    fn layout_scaling(&self) -> f64;

    /// The layout manager that lays out the tree.
    fn layout_manager(&self) -> Rc<dyn ILayoutManager>;

    /// The root layoutable.
    fn root_visual(&self) -> Ref<Layoutable>;
}
