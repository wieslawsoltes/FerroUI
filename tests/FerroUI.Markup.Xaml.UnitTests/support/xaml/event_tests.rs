//! The test types declared by the upstream test file `Xaml/EventTests.cs`.

use std::cell::Cell;
use std::rc::Rc;

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Button, ButtonImpl, ContentControlImpl, ControlImpl, Panel, PanelImpl};

use crate::support::TypeModule;

type Object = Option<BoxedValue>;

// --- MyButton ----------------------------------------------------------------

/// A button with the event handlers a document names (the code-behind of
/// the root instance).
#[repr(C)]
pub struct MyButton {
    base: Button,
    was_clicked: Cell<bool>,
    was_tapped: Cell<bool>,
}

ferro_class!(MyButton: Button);
ferro_impl_classes!(
    MyButton: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ButtonImpl
);
ferro_class_info!(MyButton {
    new: MyButton::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
        properties: [
            WasClicked: bool { get: |this: &Ref<MyButton>| this.was_clicked() },
            WasTapped: bool { get: |this: &Ref<MyButton>| this.was_tapped() },
        ],
        methods: [
            fn OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MyButton>, sender: Object, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_click(&sender, e.as_routed_event_args())
                },
            fn OnTapped(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MyButton>, sender: Object, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_tapped(&sender, e.as_routed_event_args())
                },
        ],
    },
});

impl MyButton {
    pub fn construct() -> Self {
        Self { base: Button::construct(), was_clicked: Cell::new(false), was_tapped: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn was_clicked(&self) -> bool {
        self.was_clicked.get()
    }

    pub fn was_tapped(&self) -> bool {
        self.was_tapped.get()
    }

    pub fn on_click(&self, _sender: &Object, _e: &RoutedEventArgs) {
        self.was_clicked.set(true);
    }

    pub fn on_tapped(&self, _sender: &Object, _e: &RoutedEventArgs) {
        self.was_tapped.set(true);
    }
}

// --- MyPanel -----------------------------------------------------------------

/// A panel with the event handlers a document names (the code-behind of the
/// root instance).
#[repr(C)]
pub struct MyPanel {
    base: Panel,
    was_clicked: Cell<bool>,
    was_tapped: Cell<bool>,
}

ferro_class!(MyPanel: Panel);
ferro_impl_classes!(
    MyPanel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);
ferro_class_info!(MyPanel {
    new: MyPanel::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
        properties: [
            WasClicked: bool { get: |this: &Ref<MyPanel>| this.was_clicked() },
            WasTapped: bool { get: |this: &Ref<MyPanel>| this.was_tapped() },
        ],
        methods: [
            fn OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MyPanel>, sender: Object, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_click(&sender, e.as_routed_event_args())
                },
            fn OnTapped(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MyPanel>, sender: Object, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_tapped(&sender, e.as_routed_event_args())
                },
        ],
    },
});

impl MyPanel {
    pub fn construct() -> Self {
        Self { base: Panel::construct(), was_clicked: Cell::new(false), was_tapped: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn was_clicked(&self) -> bool {
        self.was_clicked.get()
    }

    pub fn was_tapped(&self) -> bool {
        self.was_tapped.get()
    }

    pub fn on_click(&self, _sender: &Object, _e: &RoutedEventArgs) {
        self.was_clicked.set(true);
    }

    pub fn on_tapped(&self, _sender: &Object, _e: &RoutedEventArgs) {
        self.was_tapped.set(true);
    }
}

pub(crate) const MODULE: TypeModule =
    TypeModule { types: &[MyButton::TYPE, MyPanel::TYPE], markup_types: &[], value_types: || {} };
