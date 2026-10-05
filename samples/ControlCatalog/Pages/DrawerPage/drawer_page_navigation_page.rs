//! Port of `Pages/DrawerPage/DrawerPageNavigationPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/DrawerPageNavigationPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::NavigationDemoHelper;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{DrawerPage, ListBox, NavigationPage, SelectionChangedEventArgs, UserControl};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

/// The sections of the drawer menu: name, icon, title and content.
const SECTIONS: [(&str, &str, &str, &str); 4] = [
    (
        "Home",
        "M10,20V14H14V20H19V12H22L12,3L2,12H5V20H10Z",
        "Home",
        "Welcome back! Here is your dashboard with recent activity, quick actions, and personalized content just for you.",
    ),
    (
        "Inbox",
        "M20,8L12,13L4,8V6L12,11L20,6M20,4H4C2.89,4 2,4.89 2,6V18A2,2 0 0,0 4,20H20A2,2 0 0,0 22,18V6C22,4.89 21.1,4 20,4Z",
        "Inbox",
        "Your recent messages are displayed here. Tap on a conversation to view details and reply.",
    ),
    (
        "Sent",
        "M3,4H21V8H3V4M3,10H21V14H3V10M3,16H21V20H3V16Z",
        "Sent Messages",
        "View all messages you have sent. They are organized by date with the most recent at the top.",
    ),
    (
        "Settings",
        "M12,15.5A3.5,3.5 0 0,1 8.5,12A3.5,3.5 0 0,1 12,8.5A3.5,3.5 0 0,1 15.5,12A3.5,3.5 0 0,1 12,15.5M19.43,12.97C19.47,12.65 19.5,12.33 19.5,12C19.5,11.67 19.47,11.34 19.43,11L21.54,9.37C21.73,9.22 21.78,8.95 21.66,8.73L19.66,5.27C19.54,5.05 19.27,4.96 19.05,5.05L16.56,6.05C16.04,5.66 15.5,5.32 14.87,5.07L14.5,2.42C14.46,2.18 14.25,2 14,2H10C9.75,2 9.54,2.18 9.5,2.42L9.13,5.07C8.5,5.32 7.96,5.66 7.44,6.05L4.95,5.05C4.73,4.96 4.46,5.05 4.34,5.27L2.34,8.73C2.21,8.95 2.27,9.22 2.46,9.37L4.57,11C4.53,11.34 4.5,11.67 4.5,12C4.5,12.33 4.53,12.65 4.57,12.97L2.46,14.63C2.27,14.78 2.21,15.05 2.34,15.27L4.34,18.73C4.46,18.95 4.73,19.04 4.95,18.95L7.44,17.94C7.96,18.34 8.5,18.68 9.13,18.93L9.5,21.58C9.54,21.82 9.75,22 10,22H14C14.25,22 14.46,21.82 14.5,21.58L14.87,18.93C15.5,18.68 16.04,18.34 16.56,17.94L19.05,18.95C19.27,19.04 19.54,18.95 19.66,18.73L21.66,15.27C21.78,15.05 21.73,14.78 21.54,14.63L19.43,12.97Z",
        "Settings",
        "Configure application preferences and options. Changes are saved automatically.",
    ),
];

const TIP: &str = "Tip: drag from the left edge to open the menu, or use the hamburger button.";

#[repr(C)]
pub struct DrawerPageNavigationPage {
    base: UserControl,
    page_count: Cell<i32>,
}

user_control_class!(DrawerPageNavigationPage);
ferro_class_info!(DrawerPageNavigationPage {
    new: DrawerPageNavigationPage::new,
    markup: {
        methods: [
            fn OnMenuSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageNavigationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_menu_selection_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(DrawerPageNavigationPage, "/Pages/DrawerPage/DrawerPageNavigationPage.xaml");

impl DrawerPageNavigationPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), page_count: Cell::new(0) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn demo_drawer(&self) -> Ref<DrawerPage> {
        self.get_control::<DrawerPage>("DemoDrawer")
    }

    fn drawer_menu(&self) -> Ref<ListBox> {
        self.get_control::<ListBox>("DrawerMenu")
    }

    fn detail_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DetailNav")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let (name, icon, title, content) = SECTIONS[0];
        let page = NavigationDemoHelper::make_section_page(name, icon, title, content, 0, Some(TIP));
        drop(self.detail_nav().push_async_with_transition(page, None));
    }

    /// `async void`: the drawer closes once the detail page is replaced.
    fn on_menu_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if self.drawer_menu().selected_index() < 0 {
            return;
        }

        let index = self.drawer_menu().selected_index();
        let (name, icon, title, content) = SECTIONS[index as usize];
        self.page_count.set(self.page_count.get() + 1);

        let page = NavigationDemoHelper::make_section_page(name, icon, title, content, self.page_count.get(), Some(TIP));
        let replaced = self.detail_nav().replace_async_with_transition(page, None);
        let this = self.to_ref();
        drop(start_async(async move {
            if replaced.await.is_err() {
                return;
            }

            this.demo_drawer().set_is_open(false);
        }));
    }
}
