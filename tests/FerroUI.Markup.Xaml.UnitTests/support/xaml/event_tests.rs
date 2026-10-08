//! The test types declared by the upstream test file `Xaml/EventTests.cs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::utilities::{CancelEventArgs, EventArgs};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    OwnedFerroPropertyChangedEventArgs, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::popup_positioning::{CustomPopupPlacement, PopupAnchor};
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

// --- MyHost ------------------------------------------------------------------

/// Not a type of the upstream test file: a panel with the methods the tests
/// this port adds name in a document (a method for a property of a delegate
/// type, handlers of events that are not routed events).
#[repr(C)]
pub struct MyHost {
    base: Panel,
    placements: Cell<u32>,
    opening: RefCell<Vec<MarkupDelegate>>,
    openings: Cell<u32>,
    changed_properties: RefCell<Vec<String>>,
}

ferro_class!(MyHost: Panel);
ferro_impl_classes!(
    MyHost: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);
ferro_class_info!(MyHost {
    new: MyHost::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
        methods: [
            fn OnCustomPlacement(Rc<RefCell<CustomPopupPlacement>>) =>
                |this: &Ref<MyHost>, placement: Rc<RefCell<CustomPopupPlacement>>| {
                    this.on_custom_placement(&mut placement.borrow_mut())
                },
            // `void OnOpening(object sender, EventArgs e)` of the managed form: the arguments
            // are the ones the event raises, whatever their class.
            fn OnOpening(Option<BoxedValue>, BoxedValue) =>
                |this: &Ref<MyHost>, sender: Object, e: BoxedValue| this.on_opening(&sender, &e),
            fn OnPropertyChanged(Option<BoxedValue>, OwnedFerroPropertyChangedEventArgs) =>
                |this: &Ref<MyHost>, sender: Object, e: OwnedFerroPropertyChangedEventArgs| {
                    this.on_property_changed(&sender, &e)
                },
        ],
        events: [
            // `event EventHandler Opening`, raised with cancellable arguments.
            Opening(Option<BoxedValue>, EventArgs) => |this: &Ref<MyHost>, handler: MarkupDelegate| {
                this.opening.borrow_mut().push(handler)
            },
        ],
    },
});

impl MyHost {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            placements: Cell::new(0),
            opening: RefCell::new(Vec::new()),
            openings: Cell::new(0),
            changed_properties: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// How many times the placement method was called.
    pub fn placements(&self) -> u32 {
        self.placements.get()
    }

    pub fn on_custom_placement(&self, placement: &mut CustomPopupPlacement) {
        self.placements.set(self.placements.get() + 1);
        placement.set_anchor(PopupAnchor::TOP);
    }

    /// Raises `Opening` with cancellable arguments, as `PopupFlyoutBase` raises its
    /// `Opening`. Returns whether a handler cancelled it.
    pub fn raise_opening(&self) -> bool {
        let args = CancelEventArgs::new();
        let sender = into_markup_value(self.to_ref());
        let handlers = self.opening.borrow().clone();
        for handler in handlers {
            handler.invoke(&[sender.clone(), into_markup_value(args.clone())]);
        }
        args.cancel()
    }

    /// How many times the handler of `Opening` was called.
    pub fn openings(&self) -> u32 {
        self.openings.get()
    }

    /// The names of the properties whose changes the handler of `PropertyChanged` received.
    pub fn changed_properties(&self) -> Vec<String> {
        self.changed_properties.borrow().clone()
    }

    pub fn on_property_changed(&self, sender: &Object, e: &OwnedFerroPropertyChangedEventArgs) {
        if sender.is_some() {
            self.changed_properties.borrow_mut().push(e.property().name().to_string());
        }
    }

    pub fn on_opening(&self, _sender: &Object, e: &BoxedValue) {
        self.openings.set(self.openings.get() + 1);
        if let Some(args) = e.downcast_ref::<CancelEventArgs>() {
            args.set_cancel(true);
        }
    }
}

pub(crate) const MODULE: TypeModule =
    TypeModule { types: &[MyButton::TYPE, MyPanel::TYPE, MyHost::TYPE], markup_types: &[], value_types: || {} };
