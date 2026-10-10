//! Port of `TopmostWindowTest.xaml.cs`: the class of the document `TopmostWindowTest.xaml`.

use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, PixelPoint, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, ControlImpl, TextBox, TopLevelImpl, TopLevelImplExt, Window, WindowBaseImpl, WindowImpl,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct TopmostWindowTest {
    base: Window,
    timer: RefCell<Option<Rc<DispatcherTimer>>>,
}

ferro_class!(TopmostWindowTest: Window);
ferro_impl_classes!(
    TopmostWindowTest: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    WindowBaseImpl,
    WindowImpl
);
// The class has no constructor without arguments: its constructor takes the name of the
// window.
ferro_class_info!(TopmostWindowTest {
    markup: {
        methods: [
            fn Button_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TopmostWindowTest>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.button_on_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TopmostWindowTest, "/TopmostWindowTest.xaml", create: TopmostWindowTest::new("OwnerWindow"));

impl TopLevelImpl for TopmostWindowTest {
    fn on_closed(this: &Self) {
        Self::parent_on_closed(this);
        if let Some(timer) = this.timer.borrow().as_ref() {
            timer.stop();
        }
    }
}

impl TopmostWindowTest {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()), timer: RefCell::new(None) }
    }

    pub fn new(name: &str) -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_name(Some(name.to_string()));
        this.initialize_component();
        // The window holds the handlers of its events: the handler holds the window weakly.
        {
            let weak = this.downgrade();
            this.position_changed(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.current_position().set_text(Some(&this.position().to_string()));
                }
            });
        }

        if cfg!(target_os = "macos") {
            let timer = DispatcherTimer::new();
            timer.set_interval(Duration::from_millis(250));
            let weak = this.downgrade();
            timer.tick(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.timer_on_tick();
                }
            });
            timer.start();
            *this.timer.borrow_mut() = Some(timer);
        }
        this
    }

    fn current_position(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("CurrentPosition")
    }

    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    fn current_order(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("CurrentOrder")
    }

    fn timer_on_tick(&self) {
        #[cfg(target_os = "macos")]
        self.current_order().set_text(Some(&crate::MacOSIntegration::get_ordered_index(self).to_string()));
    }

    fn button_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.set_position(self.position() + PixelPoint::new(100, 100));
    }
}
