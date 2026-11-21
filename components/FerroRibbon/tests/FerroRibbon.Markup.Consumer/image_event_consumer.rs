use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::{IRoutedEventArgs, InteractiveImpl},
    layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, ImageFailedEventArgs, PanelImpl, StackPanel, StackPanelImpl};
use std::{cell::RefCell, rc::Rc};

#[repr(C)]
pub struct ImageEventsView {
    base: StackPanel,
    events: RefCell<Vec<Result<(), String>>>,
}

ferro_class!(ImageEventsView:StackPanel);
ferro_class_info!(ImageEventsView {
    new: ImageEventsView::new,
    markup: {
        namespace: "FerroRibbon.Consumer",
        methods: [
            fn OnImageOpened(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) => ImageEventsView::on_image_opened,
            fn OnImageFailed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) => ImageEventsView::on_image_failed,
        ],
    },
});
ferro_impl_classes!(ImageEventsView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);

impl ImageEventsView {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            events: RefCell::default(),
        })
    }

    pub fn on_image_opened(&self, _: Option<BoxedValue>, _: Rc<dyn IRoutedEventArgs>) {
        self.events.borrow_mut().push(Ok(()));
    }

    pub fn on_image_failed(&self, _: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>) {
        let args = args.downcast_ref::<ImageFailedEventArgs>().unwrap();
        self.events.borrow_mut().push(Err(args.error_message()));
    }

    pub fn events(&self) -> Vec<Result<(), String>> {
        self.events.borrow().clone()
    }
}
