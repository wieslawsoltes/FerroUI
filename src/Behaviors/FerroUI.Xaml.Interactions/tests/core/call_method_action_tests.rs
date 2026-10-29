//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/CallMethodActionTests.cs` of
//! Xaml.Behaviors (with its windows `CallMethodAction001` and `CallMethodAction002`: the classes
//! of the windows are here, their content is `CallMethodAction001.xaml` and
//! `CallMethodAction002.xaml`, loaded with the run-time loader).
//!
//! Adapted: the method is found in the markup metadata of the class of the window (the methods
//! the class declares for markup); the second parameter of the method with event handler
//! parameters is the routed event args (`Rc<dyn IRoutedEventArgs>`, the `EventArgs` of the
//! reference). The rendered frames the reference saves are not taken.

use crate::tests::support::{click, find, load, start_app};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref,
    StyledElement, StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ContentControlImpl, Control, ControlImpl, StackPanel, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use std::cell::RefCell;
use std::rc::Rc;

/// `CallMethodAction001`: a window with a method without parameters.
#[repr(C)]
pub(crate) struct CallMethodAction001 {
    base: Window,
    test_property: RefCell<Option<String>>,
}

ferro_class!(CallMethodAction001: Window);
ferro_impl_classes!(
    CallMethodAction001: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(CallMethodAction001 {
    new: CallMethodAction001::new,
    markup: {
        methods: [
            fn TestMethod() => |this: &Ref<CallMethodAction001>| this.test_method(),
        ],
    },
});

impl CallMethodAction001 {
    pub(crate) fn new() -> Ref<Self> {
        let this = instantiate(Self {
            base: Window::construct(PlatformManager::create_window()),
            test_property: RefCell::new(None),
        });
        let content = load::<StackPanel>("CallMethodAction001.xaml", include_str!("CallMethodAction001.xaml"));
        this.set_content(Some(Control::boxed(content)));
        this
    }

    pub(crate) fn test_property(&self) -> Option<String> {
        self.test_property.borrow().clone()
    }

    pub(crate) fn test_method(&self) {
        *self.test_property.borrow_mut() = Some("Test String".to_string());
    }

    pub(crate) fn target_button(&self) -> Ref<Button> {
        find::<Button>(&content_of(self), "TargetButton")
    }
}

/// The content of a window of the tests: the root of the document of its content, which holds
/// the name scope.
fn content_of(window: &Window) -> Ref<StyledElement> {
    window
        .content()
        .as_ref()
        .and_then(object_of)
        .and_then(|content| content.cast::<StyledElement>())
        .expect("the content of the window")
}

/// `CallMethodAction002`: a window with a method with event handler parameters.
#[repr(C)]
pub(crate) struct CallMethodAction002 {
    base: Window,
    test_property: RefCell<Option<String>>,
    sender: RefCell<Option<BoxedValue>>,
    args: RefCell<Option<Rc<dyn IRoutedEventArgs>>>,
}

ferro_class!(CallMethodAction002: Window);
ferro_impl_classes!(
    CallMethodAction002: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(CallMethodAction002 {
    new: CallMethodAction002::new,
    markup: {
        methods: [
            fn TestMethod(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CallMethodAction002>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.test_method(sender, args)
                },
        ],
    },
});

impl CallMethodAction002 {
    pub(crate) fn new() -> Ref<Self> {
        let this = instantiate(Self {
            base: Window::construct(PlatformManager::create_window()),
            test_property: RefCell::new(None),
            sender: RefCell::new(None),
            args: RefCell::new(None),
        });
        let content = load::<StackPanel>("CallMethodAction002.xaml", include_str!("CallMethodAction002.xaml"));
        this.set_content(Some(Control::boxed(content)));
        this
    }

    pub(crate) fn test_property(&self) -> Option<String> {
        self.test_property.borrow().clone()
    }

    pub(crate) fn sender(&self) -> Option<BoxedValue> {
        self.sender.borrow().clone()
    }

    pub(crate) fn args(&self) -> Option<Rc<dyn IRoutedEventArgs>> {
        self.args.borrow().clone()
    }

    pub(crate) fn test_method(&self, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>) {
        *self.test_property.borrow_mut() = Some("Test String".to_string());
        *self.sender.borrow_mut() = sender;
        *self.args.borrow_mut() = Some(args);
    }

    pub(crate) fn target_button(&self) -> Ref<Button> {
        find::<Button>(&content_of(self), "TargetButton")
    }
}

/// Without parameters.
#[test]
fn call_method_action_001() {
    let _app = start_app();
    let window = CallMethodAction001::new();

    window.show();

    assert_eq!(None, window.test_property());

    // Click
    click(&window.target_button());

    assert_eq!(Some("Test String".to_string()), window.test_property());
}

/// With event handler parameters.
#[test]
fn call_method_action_002() {
    let _app = start_app();
    let window = CallMethodAction002::new();

    window.show();

    assert_eq!(None, window.test_property());
    assert!(window.sender().is_none());
    assert!(window.args().is_none());

    // Click
    click(&window.target_button());

    assert_eq!(Some("Test String".to_string()), window.test_property());
    let sender = window.sender().as_ref().and_then(object_of);
    let window_object: Ref<FerroObject> = window.clone().upcast();
    assert_eq!(Some(window_object), sender);
    assert!(window.args().is_some_and(|args| args.as_routed_event_args().routed_event().is_some()));
}
