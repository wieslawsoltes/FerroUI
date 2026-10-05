//! Port of `Pages/NativeEmbedPage.xaml.cs`: the class of the document
//! `Pages/NativeEmbedPage.xaml`, the native control host of its samples and
//! the contract of the platform specific demo control.

use crate::markup::xaml_class;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Rect, Ref, StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::platform::IPlatformHandle;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentPage, ContextMenu, Control, ControlImpl, DockPanel, MenuItem, NativeControlHost, NativeControlHostImpl,
    NativeControlHostImplExt, PageImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct NativeEmbedPage {
    base: ContentPage,
}

ferro_class!(NativeEmbedPage: ContentPage);
ferro_impl_classes!(
    NativeEmbedPage: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);
ferro_class_info!(NativeEmbedPage {
    new: NativeEmbedPage::new,
    markup: {
        methods: [
            fn ShowPopupDelay(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NativeEmbedPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.show_popup_delay(sender, args)
                },
            fn ShowPopup(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NativeEmbedPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.show_popup(&sender, args.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NativeEmbedPage, "/Pages/NativeEmbedPage.xaml");

impl FerroObjectImpl for NativeEmbedPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Visual::bounds_property().as_property() {
            let is_mobile = change.get_new_value::<Rect>().width < 1200.0;
            this.first_panel().classes().set("mobile", is_mobile);
            this.second_panel().classes().set("mobile", is_mobile);
        }
    }
}

impl NativeEmbedPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn first_panel(&self) -> Ref<DockPanel> {
        self.get_control::<DockPanel>("FirstPanel")
    }

    fn second_panel(&self) -> Ref<DockPanel> {
        self.get_control::<DockPanel>("SecondPanel")
    }

    /// Shows the popup three seconds later.
    pub fn show_popup_delay(&self, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>) {
        let this = self.to_ref();
        DispatcherTimer::run_once(
            move || this.show_popup(&sender, args.as_routed_event_args()),
            Duration::from_millis(3000),
            DispatcherPriority::NORMAL,
        );
    }

    /// # Panics
    /// Panics if the sender is not a control (an invalid cast in the managed
    /// original).
    pub fn show_popup(&self, sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let sender = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Control>())
            .expect("the sender of the event is a control");

        let menu = ContextMenu::new();
        for _ in 0..2 {
            let item = MenuItem::new();
            item.set_header(Some(Rc::new(String::from("Test")) as BoxedValue));
            menu.items().add(Some(Control::boxed(&item)));
        }
        menu.open_at(Some(&sender));
    }
}

thread_local! {
    static IMPLEMENTATION: RefCell<Option<Rc<dyn INativeDemoControl>>> = const { RefCell::new(None) };
}

/// A native control host that shows the demo control of the platform.
#[repr(C)]
pub struct EmbedSample {
    base: NativeControlHost,
    is_second: Cell<bool>,
}

ferro_class!(EmbedSample: NativeControlHost);
ferro_impl_classes!(
    EmbedSample: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(EmbedSample {
    new: EmbedSample::new,
    markup: {
        properties: [
            IsSecond: bool {
                get: |this: &Ref<EmbedSample>| this.is_second(),
                set: |this: &Ref<EmbedSample>, value: bool| this.set_is_second(value)
            },
        ],
    },
});

impl NativeControlHostImpl for EmbedSample {
    fn create_native_control_core(this: &Self, parent: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle> {
        match Self::implementation() {
            Some(implementation) => implementation.create_control(this.is_second(), parent.clone(), &|| {
                Self::parent_create_native_control_core(this, parent.clone())
            }),
            None => Self::parent_create_native_control_core(this, parent),
        }
    }

    fn destroy_native_control_core(this: &Self, control: Rc<dyn IPlatformHandle>) {
        Self::parent_destroy_native_control_core(this, control);
    }
}

impl EmbedSample {
    /// The demo control of the platform; set by the application of the
    /// platform.
    pub fn implementation() -> Option<Rc<dyn INativeDemoControl>> {
        IMPLEMENTATION.with(|implementation| implementation.borrow().clone())
    }

    pub fn set_implementation(value: Option<Rc<dyn INativeDemoControl>>) {
        IMPLEMENTATION.with(|implementation| *implementation.borrow_mut() = value);
    }

    pub fn construct() -> Self {
        Self { base: NativeControlHost::construct(), is_second: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn is_second(&self) -> bool {
        self.is_second.get()
    }

    pub fn set_is_second(&self, value: bool) {
        self.is_second.set(value);
    }
}

pub trait INativeDemoControl {
    /// `is_second` specifies which control should be displayed as a demo.
    fn create_control(
        &self,
        is_second: bool,
        parent: Rc<dyn IPlatformHandle>,
        create_default: &dyn Fn() -> Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn IPlatformHandle>;
}
