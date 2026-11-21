//! Actual library button subclass, not a substitute control family.
use ferroribbon::controls::{RibbonButton, RibbonButtonImpl};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ButtonImpl, ContentControlImpl, ControlImpl, StackPanel, primitives::TemplatedControlImpl,
};
#[repr(C)]
pub struct ConsumerRibbonButton {
    base: RibbonButton,
}
ferro_class!(ConsumerRibbonButton:RibbonButton);
ferro_class_info!(ConsumerRibbonButton{new:ConsumerRibbonButton::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerRibbonButton:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl);
impl ConsumerRibbonButton {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonButton::construct(),
        })
    }
}
impl RibbonButtonImpl for ConsumerRibbonButton {
    fn create_copy_instance(_this: &Self) -> Ref<RibbonButton> {
        ConsumerRibbonButton::new().upcast()
    }
}

/// Test-only passive view with a real compiled markup Click method.
#[repr(C)]
pub struct ClickDeliveryConsumer {
    base: StackPanel,
    clicks: std::cell::Cell<usize>,
    last_sender: std::cell::RefCell<Option<ferroui_base::WeakRef<ferroui_controls::Control>>>,
}
ferro_class!(ClickDeliveryConsumer:StackPanel);
ferro_class_info!(ClickDeliveryConsumer{new:ClickDeliveryConsumer::new,markup:{namespace:"FerroRibbon.Consumer",methods:[fn MarkHandled(Option<ferroui_base::BoxedValue>,std::rc::Rc<dyn ferroui_base::interactivity::IRoutedEventArgs>)=>ClickDeliveryConsumer::mark_handled]}});
ferro_impl_classes!(ClickDeliveryConsumer:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,ferroui_controls::PanelImpl,ferroui_controls::StackPanelImpl);
impl ClickDeliveryConsumer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            clicks: std::cell::Cell::new(0),
            last_sender: std::cell::RefCell::new(None),
        })
    }
    pub fn mark_handled(
        &self,
        sender: Option<ferroui_base::BoxedValue>,
        args: std::rc::Rc<dyn ferroui_base::interactivity::IRoutedEventArgs>,
    ) {
        let sender =
            ferroui_base::data::core::ValueTypes::as_object(&*sender.expect("Click sender"))
                .unwrap()
                .cast::<ferroui_controls::Control>()
                .unwrap();
        assert!(args.is_source(&sender));
        self.last_sender.replace(Some(sender.downgrade()));
        self.clicks.set(self.clicks.get() + 1);
        args.set_handled(true);
    }
    pub fn clicks(&self) -> usize {
        self.clicks.get()
    }
    pub fn last_sender(&self) -> Option<Ref<ferroui_controls::Control>> {
        self.last_sender
            .borrow()
            .as_ref()
            .and_then(ferroui_base::WeakRef::upgrade)
    }
}
