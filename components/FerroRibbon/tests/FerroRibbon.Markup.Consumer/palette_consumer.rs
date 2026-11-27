use ferroribbon::controls::{
    RibbonColorPalette, RibbonColorPicker, RibbonControlBaseImpl, RibbonSplitButtonImpl,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl, media::Color, utilities::EventArgs,
};
use ferroui_controls::{
    ControlImpl, PanelImpl, StackPanel, StackPanelImpl, primitives::TemplatedControlImpl,
};
use std::cell::{Cell, RefCell};
#[repr(C)]
pub struct ConsumerPalette {
    base: RibbonColorPalette,
}
ferro_class!(ConsumerPalette:RibbonColorPalette);
ferro_class_info!(ConsumerPalette{new:ConsumerPalette::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerPalette:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl ConsumerPalette {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonColorPalette::construct(),
        })
    }
}
#[repr(C)]
pub struct PaletteView {
    base: StackPanel,
    selected: RefCell<Vec<Option<Color>>>,
    preview: RefCell<Vec<Option<Color>>>,
    more: Cell<usize>,
}
ferro_class!(PaletteView:StackPanel);
ferro_class_info!(PaletteView{new:PaletteView::new,markup:{namespace:"FerroRibbon.Consumer",methods:[fn Selected(Option<BoxedValue>,Option<Color>)=>PaletteView::selected,fn Preview(Option<BoxedValue>,Option<Color>)=>PaletteView::preview,fn More(Option<BoxedValue>,EventArgs)=>PaletteView::more]}});
ferro_impl_classes!(PaletteView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
impl PaletteView {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            selected: RefCell::new(vec![]),
            preview: RefCell::new(vec![]),
            more: Cell::new(0),
        })
    }
    pub fn selected(&self, _: Option<BoxedValue>, c: Option<Color>) {
        self.selected.borrow_mut().push(c);
    }
    pub fn preview(&self, _: Option<BoxedValue>, c: Option<Color>) {
        self.preview.borrow_mut().push(c);
    }
    pub fn more(&self, _: Option<BoxedValue>, a: EventArgs) {
        assert_eq!(a, EventArgs::EMPTY);
        self.more.set(self.more.get() + 1);
    }
    pub fn selections(&self) -> Vec<Option<Color>> {
        self.selected.borrow().clone()
    }
    pub fn previews(&self) -> Vec<Option<Color>> {
        self.preview.borrow().clone()
    }
    pub fn more_calls(&self) -> usize {
        self.more.get()
    }
}
#[repr(C)]
pub struct ConsumerColorPicker {
    base: RibbonColorPicker,
}
ferro_class!(ConsumerColorPicker:RibbonColorPicker);
ferro_class_info!(ConsumerColorPicker{new:ConsumerColorPicker::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerColorPicker:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl,RibbonSplitButtonImpl);
impl ConsumerColorPicker {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonColorPicker::construct(),
        })
    }
}
