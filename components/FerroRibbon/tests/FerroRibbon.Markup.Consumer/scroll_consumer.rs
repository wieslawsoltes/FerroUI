//! Independent subclass and contract implementations; no library-private construction.
use ferroribbon::primitives::{IRibbonScrollContent, RibbonScrollPanel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::{
    FerroObject, FerroObjectImpl, Ref, Size, StyledElementImpl, VisualImpl, ferro_class,
    ferro_class_info, ferro_impl_classes, instantiate,
};
use ferroui_controls::{Control, ControlImpl, PanelImpl};
use std::{cell::Cell, rc::Rc};
#[repr(C)]
pub struct ConsumerScroll {
    base: RibbonScrollPanel,
    arrangements: Cell<usize>,
}
ferro_class!(ConsumerScroll:RibbonScrollPanel);
ferro_impl_classes!(ConsumerScroll:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(ConsumerScroll {new:ConsumerScroll::new,markup:{namespace:"FerroRibbon.Consumer",content:Children,}});
impl ConsumerScroll {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonScrollPanel::construct(),
            arrangements: Cell::new(0),
        })
    }
    pub fn arrangements(&self) -> usize {
        self.arrangements.get()
    }
}
impl LayoutableImpl for ConsumerScroll {
    fn arrange_override(this: &Self, size: Size) -> Size {
        this.arrangements.set(this.arrangements.get() + 1);
        Self::parent_arrange_override(this, size)
    }
}
#[repr(C)]
pub struct ConsumerAdaptive {
    base: Control,
    extent: Cell<f64>,
}
ferro_class!(ConsumerAdaptive:Control);
ferro_impl_classes!(ConsumerAdaptive:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(ConsumerAdaptive {
 new:ConsumerAdaptive::new,
 interfaces:[Rc<dyn IRibbonScrollContent> => |value| Rc::new(AdaptiveView(value)) as Rc<dyn IRibbonScrollContent>],
 markup:{namespace:"FerroRibbon.Consumer",properties:[ExtentWidth:f64 {get:ConsumerAdaptive::extent_width,set:ConsumerAdaptive::set_extent_width}],},
});
impl ConsumerAdaptive {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            extent: Cell::new(0.),
        })
    }
    pub fn extent_width(&self) -> f64 {
        self.extent.get()
    }
    pub fn set_extent_width(&self, value: f64) {
        self.extent.set(value);
        self.invalidate_measure();
        // DesiredSize deliberately stays at 50 units. The separate extent can
        // change without that notification, so invalidate its layout owner.
        if let Some(parent) = self.visual_parent()
            && let Some(parent) = parent.downcast_ref::<ferroui_base::layout::Layoutable>()
        {
            parent.invalidate_measure();
        }
    }
}
impl LayoutableImpl for ConsumerAdaptive {
    fn measure_override(_this: &Self, size: Size) -> Size {
        Size::new(size.width.min(50.), 20.)
    }
}
struct AdaptiveView(Ref<ConsumerAdaptive>);
impl IRibbonScrollContent for AdaptiveView {
    fn extent_width(&self) -> f64 {
        self.0.extent_width()
    }
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
}
