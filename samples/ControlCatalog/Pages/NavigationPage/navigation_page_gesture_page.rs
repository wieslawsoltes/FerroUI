//! Port of `Pages/NavigationPage/NavigationPageGesturePage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageGesturePage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::NavigationDemoHelper;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{CheckBox, Control, NavigationPage, TextBlock, UserControl};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

const DRAG_HINT: &str = "\u{2190} Drag from the left edge to go back";

#[repr(C)]
pub struct NavigationPageGesturePage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    initialized: Cell<bool>,
}

user_control_class!(NavigationPageGesturePage);
ferro_class_info!(NavigationPageGesturePage {
    new: NavigationPageGesturePage::new,
    markup: {
        methods: [
            fn OnGestureEnabledChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageGesturePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_gesture_enabled_changed(&sender, e.as_routed_event_args())
                },
            fn OnPushPages(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageGesturePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_pages(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageGesturePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageGesturePage, "/Pages/NavigationPage/NavigationPageGesturePage.xaml");

impl NavigationPageGesturePage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            component_initialized: Cell::new(false),
            initialized: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        Self::enable_mouse_swipe_gesture(&this.nav());

        // The handler of an event of the control itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    /// The field `DemoNav`: null until `InitializeComponent()` has returned.
    fn demo_nav(&self) -> Option<Ref<NavigationPage>> {
        self.component_initialized.get().then(|| self.get_control::<NavigationPage>("DemoNav"))
    }

    fn gesture_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("GestureCheck")
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    /// The field `DemoNav` once the document is loaded.
    fn nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    /// `async void`.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);
        let this = self.to_ref();
        drop(start_async(async move {
            for (index, header) in ["Page 1", "Page 2", "Page 3"].into_iter().enumerate() {
                let page = NavigationDemoHelper::make_page(header, DRAG_HINT, index as i32);
                if this.nav().push_async_with_transition(page, None).await.is_err() {
                    return;
                }
            }
            this.update_status();
        }));
    }

    fn on_gesture_enabled_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };
        demo_nav.set_is_gesture_enabled(self.gesture_check().is_checked() == Some(true));
    }

    /// `async void`.
    fn on_push_pages(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            let depth = this.nav().stack_depth();
            let page = NavigationDemoHelper::make_page(&format!("Page {}", depth + 1), DRAG_HINT, depth);
            if this.nav().push_async_with_transition(page, None).await.is_err() {
                return;
            }
            this.update_status();
        }));
    }

    /// `async void`.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            if this.nav().pop_async().await.is_err() {
                return;
            }
            this.update_status();
        }));
    }

    fn update_status(&self) {
        self.status_text().set_text(Some(&format!("Depth: {}", self.nav().stack_depth())));
    }

    fn enable_mouse_swipe_gesture(control: &Control) {
        let recognizer = control
            .gesture_recognizers()
            .to_vec()
            .into_iter()
            .find_map(|recognizer| recognizer.cast::<SwipeGestureRecognizer>());
        if let Some(recognizer) = recognizer {
            recognizer.set_is_mouse_enabled(true);
        }
    }
}
