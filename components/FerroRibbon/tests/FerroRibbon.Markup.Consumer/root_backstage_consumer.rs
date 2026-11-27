//! External markup/event integration fixture; no internal FerroRibbon paths.
use ferroribbon::controls::RibbonHandledEventArgs;
use ferroui_base::{BoxedValue, FerroObjectImpl, Ref, ferro_class, ferro_class_info, instantiate};
use ferroui_controls::{ControlImpl, PanelImpl, StackPanel};
use std::{cell::Cell, rc::Rc};
#[repr(C)]
pub struct RootBackstageConsumer {
    base: StackPanel,
    file: Cell<i32>,
    opened: Cell<i32>,
    closed: Cell<i32>,
    cancel: Cell<bool>,
}
ferro_class!(RootBackstageConsumer:StackPanel);
ferro_class_info!(RootBackstageConsumer{new:RootBackstageConsumer::new,markup:{namespace:"FerroRibbon.Consumer",methods:[fn OnFile(Option<BoxedValue>,Rc<RibbonHandledEventArgs>)=>|s:&RootBackstageConsumer,_,a:Rc<RibbonHandledEventArgs>|{s.file.set(s.file.get()+1);a.set_handled(s.cancel.get());},fn OnOpened(Option<BoxedValue>,ferroui_base::utilities::EventArgs)=>|s:&RootBackstageConsumer,_,_|s.opened.set(s.opened.get()+1),fn OnClosed(Option<BoxedValue>,ferroui_base::utilities::EventArgs)=>|s:&RootBackstageConsumer,_,_|s.closed.set(s.closed.get()+1)]}});
ferroui_base::ferro_impl_classes!(RootBackstageConsumer:FerroObjectImpl,ferroui_base::StyledElementImpl,ferroui_base::VisualImpl,ferroui_base::interactivity::InteractiveImpl,ferroui_base::input::InputElementImpl,ferroui_base::layout::LayoutableImpl,ControlImpl,PanelImpl,ferroui_controls::StackPanelImpl);
impl RootBackstageConsumer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            file: Cell::new(0),
            opened: Cell::new(0),
            closed: Cell::new(0),
            cancel: Cell::new(true),
        })
    }
    pub fn counts(&self) -> (i32, i32, i32) {
        (self.file.get(), self.opened.get(), self.closed.get())
    }
    pub fn set_cancel(&self, v: bool) {
        self.cancel.set(v);
    }
}
