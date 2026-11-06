//! Internal composition adapter for Uno Grid.Padding, absent from the pinned native Grid.
//! The outer class remains a Grid; its Border pads an inner Grid. Typed port code and source
//! fixtures address the inner rows/columns/children explicitly. This is not a metadata alias.
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl, Thickness, VisualImpl,
    ferro_class, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{Border, ControlImpl, Grid, PanelImpl};
#[repr(C)]
pub(crate) struct PaddedGrid {
    base: Grid,
    border: Ref<Border>,
    content: Ref<Grid>,
}
ferro_class!(PaddedGrid:Grid);
ferro_impl_classes!(PaddedGrid:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl FerroObjectImpl for PaddedGrid {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.border.set_child(this.content.clone());
        this.children().add(this.border.clone());
    }
}
impl PaddedGrid {
    pub fn new(padding: Thickness) -> Ref<Self> {
        let grid = instantiate(Self {
            base: Grid::construct(),
            border: Border::new(),
            content: Grid::new(),
        });
        grid.set_padding(padding);
        grid
    }
    pub fn set_padding(&self, padding: Thickness) {
        self.border.set_padding(padding);
    }
    pub fn padding(&self) -> Thickness {
        self.border.padding()
    }
    pub fn content_grid(&self) -> Ref<Grid> {
        self.content.clone()
    }
}
