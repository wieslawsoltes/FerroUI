//! Port of `Pages/NavigationPage/NavigationPageModalPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageModalPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::NavigationDemoHelper;
use ferroui_base::animation::{CrossFade, IPageTransition, PageSlide, SlideAxis, TimeSpan};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ComboBox, NavigationPage, SelectionChangedEventArgs, TextBlock, UserControl};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct NavigationPageModalPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    initialized: Cell<bool>,
    modal_count: Cell<i32>,
}

user_control_class!(NavigationPageModalPage);
ferro_class_info!(NavigationPageModalPage {
    new: NavigationPageModalPage::new,
    markup: {
        methods: [
            fn OnPushModal(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageModalPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push_modal(&sender, e.as_routed_event_args())
                },
            fn OnPopModal(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageModalPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_modal(&sender, e.as_routed_event_args())
                },
            fn OnPopAllModals(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageModalPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_all_modals(&sender, e.as_routed_event_args())
                },
            fn OnTransitionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageModalPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_transition_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(NavigationPageModalPage, "/Pages/NavigationPage/NavigationPageModalPage.xaml");

impl NavigationPageModalPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            component_initialized: Cell::new(false),
            initialized: Cell::new(false),
            modal_count: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);

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

    fn transition_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("TransitionCombo")
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    /// The field `DemoNav` once the document is loaded.
    fn nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);
        let page = NavigationDemoHelper::make_page("Home", "Use Push Modal to show a modal on top.", 0);
        drop(self.nav().push_async_with_transition(page, None));
    }

    /// `async void`.
    fn on_push_modal(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            this.modal_count.set(this.modal_count.get() + 1);
            let modal_count = this.modal_count.get();
            let modal = NavigationDemoHelper::make_page(
                &format!("Modal {modal_count}"),
                "This page was presented modally.\nTap 'Pop Modal' to dismiss.",
                modal_count,
            );
            if this.nav().push_modal_async(modal).await.is_err() {
                return;
            }
            this.update_status();
        }));
    }

    /// `async void`.
    fn on_pop_modal(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            if this.nav().pop_modal_async().await.is_err() {
                return;
            }
            this.update_status();
        }));
    }

    /// `async void`.
    fn on_pop_all_modals(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            if this.nav().pop_all_modals_async().await.is_err() {
                return;
            }
            this.modal_count.set(0);
            this.update_status();
        }));
    }

    fn on_transition_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };

        let transition: Option<Rc<dyn IPageTransition>> = match self.transition_combo().selected_index() {
            1 => Some(Rc::new(CrossFade::with_duration(TimeSpan::from_milliseconds(250.0)))),
            2 => None,
            _ => Some(Rc::new(PageSlide::with_duration(TimeSpan::from_milliseconds(300.0), SlideAxis::Vertical))),
        };
        demo_nav.set_modal_transition(transition);
    }

    fn update_status(&self) {
        self.status_text().set_text(Some(&format!("Modals: {}", self.nav().modal_stack().len())));
    }
}
