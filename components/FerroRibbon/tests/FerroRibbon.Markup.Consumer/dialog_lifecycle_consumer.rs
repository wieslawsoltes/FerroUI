//! Independent native subclass and actual compiled/runtime lifecycle consumer.
use ferroribbon::controls::{
    RibbonDialogClosedEventArgs, RibbonDialogClosingDeferral, RibbonDialogClosingEventArgs,
    RibbonDialogFuture, RibbonDialogHost, RibbonDialogOpenedEventArgs, RibbonDialogOutcome,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl, threading::Dispatcher,
};
use ferroui_controls::{ContentControlImpl, ControlImpl, primitives::TemplatedControlImpl};
use futures_util::{FutureExt, future::Shared};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct DialogEventsHost {
    base: RibbonDialogHost,
    mode: Cell<u8>,
    opened: Cell<usize>,
    closing: Cell<usize>,
    closed: Cell<usize>,
    pending: RefCell<Option<RibbonDialogClosingDeferral>>,
    future: RefCell<Option<Shared<RibbonDialogFuture>>>,
    closed_pending_before: Cell<bool>,
    closed_pending_after_pump: Cell<bool>,
    result: Cell<Option<RibbonDialogOutcome>>,
}
ferro_class!(DialogEventsHost:RibbonDialogHost);
ferro_impl_classes!(DialogEventsHost:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,ContentControlImpl,TemplatedControlImpl);
ferro_class_info!(DialogEventsHost {new:DialogEventsHost::new,markup:{namespace:"FerroRibbon.Consumer",methods:[fn OnOpened(Option<BoxedValue>,Rc<RibbonDialogOpenedEventArgs>)=>DialogEventsHost::on_opened,fn OnClosing(Option<BoxedValue>,Rc<RibbonDialogClosingEventArgs>)=>DialogEventsHost::on_closing,fn OnClosed(Option<BoxedValue>,Rc<RibbonDialogClosedEventArgs>)=>DialogEventsHost::on_closed]}});
impl DialogEventsHost {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonDialogHost::construct(),
            mode: Cell::new(0),
            opened: Cell::new(0),
            closing: Cell::new(0),
            closed: Cell::new(0),
            pending: RefCell::default(),
            future: RefCell::default(),
            closed_pending_before: Cell::new(false),
            closed_pending_after_pump: Cell::new(false),
            result: Cell::new(None),
        })
    }
    pub fn set_mode(&self, value: u8) {
        self.mode.set(value);
    }
    pub fn set_future(&self, future: RibbonDialogFuture) {
        self.future.replace(Some(future.shared()));
    }
    pub fn future_result(&self) -> Option<RibbonDialogOutcome> {
        self.future
            .borrow()
            .as_ref()
            .and_then(|future| future.clone().now_or_never())
            .map(Result::unwrap)
    }
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.opened.get(), self.closing.get(), self.closed.get())
    }
    pub fn complete(&self) {
        self.pending.take().unwrap().complete().unwrap();
    }
    pub fn pending_during_closed(&self) -> (bool, bool) {
        (
            self.closed_pending_before.get(),
            self.closed_pending_after_pump.get(),
        )
    }
    pub fn result(&self) -> Option<RibbonDialogOutcome> {
        self.result.get()
    }
    pub fn on_opened(&self, _: Option<BoxedValue>, _: Rc<RibbonDialogOpenedEventArgs>) {
        self.opened.set(self.opened.get() + 1);
        assert!(self.is_dialog_showing());
        assert!(self.primary_button().unwrap().is_focused());
    }
    pub fn on_closing(&self, _: Option<BoxedValue>, args: Rc<RibbonDialogClosingEventArgs>) {
        self.closing.set(self.closing.get() + 1);
        match self.mode.get() {
            0 => args.set_cancel(true),
            2 => {
                self.pending.replace(Some(args.get_deferral()));
            }
            _ => {}
        }
    }
    pub fn on_closed(&self, _: Option<BoxedValue>, args: Rc<RibbonDialogClosedEventArgs>) {
        self.closed.set(self.closed.get() + 1);
        self.result.set(Some(args.result()));
        self.closed_pending_before
            .set(self.future_result().is_none());
        Dispatcher::ui_thread().run_jobs(None);
        self.closed_pending_after_pump
            .set(self.future_result().is_none());
    }
}
