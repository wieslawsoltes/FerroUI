//! Port of `Pages/DrawerPage/DrawerPageCustomFlyoutPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/DrawerPageCustomFlyoutPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{value_text, NavigationDemoHelper};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::media::TranslateTransform;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, FerroPropertyChangedEventArgs, Ref};
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::{Button, ContentPage, Control, DrawerPage, NavigationPage, UserControl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct DrawerPageCustomFlyoutPage {
    base: UserControl,
    bubble1: RefCell<Option<Ref<Ellipse>>>,
    bubble2: RefCell<Option<Ref<Ellipse>>>,
    /// The timer of the bubbles with the subscription of its tick.
    bubble_timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    bubble_phase: Cell<f64>,
}

user_control_class!(DrawerPageCustomFlyoutPage);
ferro_class_info!(DrawerPageCustomFlyoutPage {
    new: DrawerPageCustomFlyoutPage::new,
    markup: {
        methods: [
            fn OnMenuItemClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCustomFlyoutPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_menu_item_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(DrawerPageCustomFlyoutPage, "/Pages/DrawerPage/DrawerPageCustomFlyoutPage.xaml");

/// `control.RenderTransform as TranslateTransform`.
fn translate_transform(control: &Control) -> Option<Ref<TranslateTransform>> {
    control
        .render_transform()
        .and_then(|transform| transform.as_object().and_then(|object| object.to_ref().cast::<TranslateTransform>()))
}

impl DrawerPageCustomFlyoutPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            bubble1: RefCell::new(None),
            bubble2: RefCell::new(None),
            bubble_timer: RefCell::new(None),
            bubble_phase: Cell::new(0.0),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        *this.bubble1.borrow_mut() = this.find_control::<Ellipse>("Bubble1");
        *this.bubble2.borrow_mut() = this.find_control::<Ellipse>("Bubble2");

        // The handler belongs to a child of the page: it holds the page weakly.
        let weak = this.downgrade();
        this.drawer_page_control().property_changed(move |args: &FerroPropertyChangedEventArgs<'_>| {
            if args.property() == DrawerPage::is_open_property().as_property() {
                if let Some(this) = weak.upgrade() {
                    this.on_drawer_open_changed(args.get_new_value::<bool>());
                }
            }
        });

        drop(this.detail_nav().push_async_with_transition(Self::build_detail_page("Home"), None));
        this
    }

    fn drawer_page_control(&self) -> Ref<DrawerPage> {
        self.get_control::<DrawerPage>("DrawerPageControl")
    }

    fn detail_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DetailNav")
    }

    fn menu_items(&self) -> [Ref<Control>; 6] {
        ["MenuItem1", "MenuItem2", "MenuItem3", "MenuItem4", "MenuItem5", "FooterRow"]
            .map(|name| self.get_control::<Control>(name))
    }

    fn on_drawer_open_changed(&self, is_open: bool) {
        if is_open {
            self.start_bubbles();
            for item in self.menu_items() {
                item.set_opacity(1.0);
                if let Some(tt) = translate_transform(&item) {
                    tt.set_y(0.0);
                }
            }
        } else {
            self.stop_bubbles();
            for item in self.menu_items() {
                let saved_item_t = item.transitions();
                item.set_transitions(None);
                item.set_opacity(0.0);
                item.set_transitions(saved_item_t);

                if let Some(tt) = translate_transform(&item) {
                    let saved_tt = tt.transitions();
                    tt.set_transitions(None);
                    tt.set_y(25.0);
                    tt.set_transitions(saved_tt);
                }
            }
        }
    }

    fn start_bubbles(&self) {
        if self.bubble_timer.borrow().is_some() {
            return;
        }
        self.bubble_phase.set(0.0);
        let timer = DispatcherTimer::with_priority(DispatcherPriority::RENDER);
        timer.set_interval(Duration::from_millis(16));
        // The timer is a field of the page: its handler holds the page weakly.
        let weak = self.to_ref().downgrade();
        let tick = timer.tick(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_bubble_tick();
            }
        });
        timer.start();
        *self.bubble_timer.borrow_mut() = Some((timer, tick));
    }

    fn stop_bubbles(&self) {
        let Some((timer, tick)) = self.bubble_timer.borrow_mut().take() else {
            return;
        };
        timer.stop();
        tick.dispose();
        if let Some(bubble1) = self.bubble1.borrow().as_ref() {
            bubble1.set_render_transform(None);
        }
        if let Some(bubble2) = self.bubble2.borrow().as_ref() {
            bubble2.set_render_transform(None);
        }
    }

    fn on_bubble_tick(&self) {
        self.bubble_phase.set(self.bubble_phase.get() + 0.012);
        let phase = self.bubble_phase.get();

        if let Some(bubble1) = self.bubble1.borrow().as_ref() {
            bubble1.set_render_transform(Some(
                TranslateTransform::with_offset((phase * 0.65).sin() * 10.0, phase.sin() * 14.0).into(),
            ));
        }
        if let Some(bubble2) = self.bubble2.borrow().as_ref() {
            bubble2.set_render_transform(Some(
                TranslateTransform::with_offset((phase * 0.45 + 1.8).sin() * 7.0, (phase * 0.85 + 0.6).cos() * 10.0)
                    .into(),
            ));
        }
    }

    /// `async void`: nothing follows the replacement.
    fn on_menu_item_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let button = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Button>());
        let Some(button) = button else {
            return;
        };
        let tag = value_text(&button.tag()).unwrap_or_else(|| String::from("Home"));
        self.drawer_page_control().set_is_open(false);
        drop(self.detail_nav().replace_async_with_transition(Self::build_detail_page(&tag), None));
    }

    fn build_detail_page(section: &str) -> Ref<ContentPage> {
        let (icon_path, body) = match section {
            "Home" => (
                "M10,20V14H14V20H19V12H22L12,3L2,12H5V20H10Z",
                "Welcome back! Here is your dashboard with recent activity, quick actions, and personalized content.",
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

        let page = NavigationDemoHelper::make_section_page(section, icon_path, section, body, 0, None);
        NavigationPage::set_has_navigation_bar(&page, false);
        page
    }
}
