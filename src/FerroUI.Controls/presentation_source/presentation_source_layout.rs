use super::PresentationSource;
use ferroui_base::layout::{ILayoutManager, ILayoutRoot, LayoutManager, Layoutable};
use ferroui_base::Ref;
use std::rc::{Rc, Weak};

impl PresentationSource {
    /// The scaling factor to use in layout.
    pub fn layout_scaling(&self) -> f64 {
        self.render_scaling()
    }

    /// The layout manager of the tree.
    pub fn layout_manager(&self) -> &Rc<LayoutManager> {
        &self.layout_manager
    }

    pub(super) fn create_layout_manager(owner: Weak<dyn ILayoutRoot>) -> Rc<LayoutManager> {
        LayoutManager::new(owner)
    }
}

impl ILayoutRoot for PresentationSource {
    fn layout_scaling(&self) -> f64 {
        PresentationSource::layout_scaling(self)
    }

    fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.layout_manager.clone()
    }

    fn root_visual(&self) -> Ref<Layoutable> {
        self.root_element().upcast()
    }
}
