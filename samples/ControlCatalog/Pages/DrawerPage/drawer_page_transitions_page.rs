//! Port of `Pages/DrawerPage/DrawerPageTransitionsPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/DrawerPageTransitionsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{value_text, NavigationDemoHelper};
use ferroui_base::animation::{CompositePageTransition, CrossFade, IPageTransition, PageSlide, SlideAxis, TimeSpan};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    Button, ComboBox, ContentPage, DrawerPage, NavigationPage, SelectionChangedEventArgs, UserControl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct DrawerPageTransitionsPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    selected_transition: RefCell<String>,
}

user_control_class!(DrawerPageTransitionsPage);
ferro_class_info!(DrawerPageTransitionsPage {
    new: DrawerPageTransitionsPage::new,
    markup: {
        methods: [
            fn OnTransitionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_transition_changed(&sender, e)
                    }
                },
            fn OnSectionClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_section_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(DrawerPageTransitionsPage, "/Pages/DrawerPage/DrawerPageTransitionsPage.xaml");

/// `sender as Button`.
fn as_button(sender: &Option<BoxedValue>) -> Option<Ref<Button>> {
    sender.as_ref().and_then(|sender| ValueTypes::as_object(&**sender)).and_then(|sender| sender.cast::<Button>())
}

impl DrawerPageTransitionsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            component_initialized: Cell::new(false),
            selected_transition: RefCell::new(String::from("None")),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn transition_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("TransitionCombo")
    }

    fn demo_drawer(&self) -> Ref<DrawerPage> {
        self.get_control::<DrawerPage>("DemoDrawer")
    }

    /// The field `DetailNav`: null until `InitializeComponent()` has returned.
    fn detail_nav(&self) -> Option<Ref<NavigationPage>> {
        self.component_initialized.get().then(|| self.get_control::<NavigationPage>("DetailNav"))
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let detail_nav = self.get_control::<NavigationPage>("DetailNav");
        // The default transition is cleared: `on_transition_changed` runs while the document
        // loads, before the navigation page exists.
        detail_nav.set_page_transition(None);
        let page = Self::build_page("Home", &self.selected_transition.borrow());
        drop(detail_nav.push_async_with_transition(page, None));
    }

    fn on_transition_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(detail_nav) = self.detail_nav() else {
            return;
        };

        let selected_index = self.transition_combo().selected_index();
        *self.selected_transition.borrow_mut() = String::from(match selected_index {
            1 => "CrossFade",
            2 => "PageSlide (H)",
            3 => "PageSlide (V)",
            4 => "Composite (Slide + Fade)",
            _ => "None",
        });

        let duration = TimeSpan::from_milliseconds(300.0);
        let transition: Option<Rc<dyn IPageTransition>> = match selected_index {
            1 => Some(Rc::new(CrossFade::with_duration(duration))),
            2 => Some(Rc::new(PageSlide::with_duration(duration, SlideAxis::Horizontal))),
            3 => Some(Rc::new(PageSlide::with_duration(duration, SlideAxis::Vertical))),
            4 => {
                let composite = CompositePageTransition::new();
                composite.add(Rc::new(PageSlide::with_duration(duration, SlideAxis::Horizontal)));
                composite.add(Rc::new(CrossFade::with_duration(duration)));
                Some(Rc::new(composite))
            }
            _ => None,
        };
        detail_nav.set_page_transition(transition);
    }

    /// `async void`: nothing follows the replacement.
    fn on_section_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(button) = as_button(sender) else {
            return;
        };
        let section = value_text(&button.tag()).unwrap_or_else(|| String::from("Home"));
        self.demo_drawer().set_is_open(false);
        let page = Self::build_page(&section, &self.selected_transition.borrow());
        drop(self.get_control::<NavigationPage>("DetailNav").replace_async(page));
    }

    fn build_page(section: &str, transition_name: &str) -> Ref<ContentPage> {
        let (icon_path, body) = match section {
            "Home" => (
                "M10,20V14H14V20H19V12H22L12,3L2,12H5V20H10Z",
                "Your dashboard with recent activity, quick actions, and personalized content.",
            ),
            "Explore" => (
                "M12,11.5A2.5,2.5 0 0,1 9.5,9A2.5,2.5 0 0,1 12,6.5A2.5,2.5 0 0,1 14.5,9A2.5,2.5 0 0,1 12,11.5M12,2A7,7 0 0,0 5,9C5,14.25 12,22 12,22C12,22 19,14.25 19,9A7,7 0 0,0 12,2Z",
                "Discover new places, trending topics, and recommended content tailored to your interests.",
            ),
            "Messages" => (
                "M20,8L12,13L4,8V6L12,11L20,6M20,4H4C2.89,4 2,4.89 2,6V18A2,2 0 0,0 4,20H20A2,2 0 0,0 22,18V6C22,4.89 21.1,4 20,4Z",
                "Your conversations and notifications. Stay connected with the people who matter.",
            ),
            "Profile" => (
                "M12,4A4,4 0 0,1 16,8A4,4 0 0,1 12,12A4,4 0 0,1 8,8A4,4 0 0,1 12,4M12,14C16.42,14 20,15.79 20,18V20H4V18C4,15.79 7.58,14 12,14Z",
                "View and edit your profile, manage privacy settings, and control your account preferences.",
            ),
            "Settings" => (
                "M12,15.5A3.5,3.5 0 0,1 8.5,12A3.5,3.5 0 0,1 12,8.5A3.5,3.5 0 0,1 15.5,12A3.5,3.5 0 0,1 12,15.5M19.43,12.97C19.47,12.65 19.5,12.33 19.5,12C19.5,11.67 19.47,11.34 19.43,11L21.54,9.37C21.73,9.22 21.78,8.95 21.66,8.73L19.66,5.27C19.54,5.05 19.27,4.96 19.05,5.05L16.56,6.05C16.04,5.66 15.5,5.32 14.87,5.07L14.5,2.42C14.46,2.18 14.25,2 14,2H10C9.75,2 9.54,2.18 9.5,2.42L9.13,5.07C8.5,5.32 7.96,5.66 7.44,6.05L4.95,5.05C4.73,4.96 4.46,5.05 4.34,5.27L2.34,8.73C2.21,8.95 2.27,9.22 2.46,9.37L4.57,11C4.53,11.34 4.5,11.67 4.5,12C4.5,12.33 4.53,12.65 4.57,12.97L2.46,14.63C2.27,14.78 2.21,15.05 2.34,15.27L4.34,18.73C4.46,18.95 4.73,19.04 4.95,18.95L7.44,17.94C7.96,18.34 8.5,18.68 9.13,18.93L9.5,21.58C9.54,21.82 9.75,22 10,22H14C14.25,22 14.46,21.82 14.5,21.58L14.87,18.93C15.5,18.68 16.04,18.34 16.56,17.94L19.05,18.95C19.27,19.04 19.54,18.95 19.66,18.73L21.66,15.27C21.78,15.05 21.73,14.78 21.54,14.63L19.43,12.97Z",
                "Configure application preferences, notifications, and privacy options.",
            ),
            _ => ("M12,2A10,10 0 0,1 22,12A10,10 0 0,1 12,22A10,10 0 0,1 2,12A10,10 0 0,1 12,2Z", ""),
        };

        let page = NavigationDemoHelper::make_section_page(
            section,
            icon_path,
            section,
            body,
            0,
            Some(&format!("Transition: {transition_name}")),
        );
        NavigationPage::set_has_navigation_bar(&page, false);
        page
    }
}
