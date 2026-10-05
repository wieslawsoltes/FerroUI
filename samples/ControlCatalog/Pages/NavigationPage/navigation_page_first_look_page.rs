//! Port of `Pages/NavigationPage/NavigationPageFirstLookPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageFirstLookPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{value_text, NavigationDemoHelper};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{CheckBox, NavigationPage, TextBlock, UserControl};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct NavigationPageFirstLookPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    initialized: Cell<bool>,
    page_count: Cell<i32>,
}

user_control_class!(NavigationPageFirstLookPage);
ferro_class_info!(NavigationPageFirstLookPage {
    new: NavigationPageFirstLookPage::new,
    markup: {
        methods: [
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
            fn OnPopToRoot(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_to_root(&sender, e.as_routed_event_args())
                },
            fn OnHasNavBarChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_has_nav_bar_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageFirstLookPage, "/Pages/NavigationPage/NavigationPageFirstLookPage.xaml");

impl NavigationPageFirstLookPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            component_initialized: Cell::new(false),
            initialized: Cell::new(false),
            page_count: Cell::new(0),
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

    fn has_nav_bar_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("HasNavBarCheck")
    }

    fn has_back_button_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("HasBackButtonCheck")
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    fn header_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("HeaderText")
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
            let page = NavigationDemoHelper::make_page("Home", "Welcome!\nUse the buttons to push and pop pages.", 0);
            if this.nav().push_async_with_transition(page, None).await.is_err() {
                return;
            }
            this.update_status();
        }));
    }

    /// `async void`.
    fn on_push(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            this.page_count.set(this.page_count.get() + 1);
            let page_count = this.page_count.get();
            let page = NavigationDemoHelper::make_page(
                &format!("Page {page_count}"),
                &format!("This is page {page_count}."),
                page_count,
            );
            NavigationPage::set_has_navigation_bar(&page, this.has_nav_bar_check().is_checked() == Some(true));
            NavigationPage::set_has_back_button(&page, this.has_back_button_check().is_checked() == Some(true));
            if this.nav().push_async(page).await.is_err() {
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

    /// `async void`.
    fn on_pop_to_root(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            if this.nav().pop_to_root_async().await.is_err() {
                return;
            }
            this.page_count.set(0);
            this.update_status();
        }));
    }

    fn on_has_nav_bar_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };
        if let Some(current_page) = demo_nav.current_page() {
            NavigationPage::set_has_navigation_bar(&current_page, self.has_nav_bar_check().is_checked() == Some(true));
        }
    }

    fn update_status(&self) {
        let demo_nav = self.nav();
        self.status_text().set_text(Some(&format!("Depth: {}", demo_nav.stack_depth())));
        let header = demo_nav.current_page().and_then(|page| value_text(&page.header()));
        self.header_text().set_text(Some(&format!("Current: {}", header.unwrap_or_else(|| String::from("(none)")))));
    }
}
