//! Port of `Pages/TabbedPage/TabbedPageFabPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageFabPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::parse_geometry;
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, ContentPage, Control, PathIcon, TextBlock, UserControl};
use std::cell::Cell;

const FEED_GEOMETRY: &str = "M12.9942 2.79444C12.4118 2.30208 11.5882 2.30208 11.0058 2.79444L3.50582 9.39444C3.18607 9.66478 3 10.0634 3 10.4828V20.25C3 20.9404 3.55964 21.5 4.25 21.5H8.25C8.94036 21.5 9.5 20.9404 9.5 20.25V14.75C9.5 14.6119 9.61193 14.5 9.75 14.5H14.25C14.3881 14.5 14.5 14.6119 14.5 14.75V20.25C14.5 20.9404 15.0596 21.5 15.75 21.5H19.75C20.4404 21.5 21 20.9404 21 20.25V10.4828C21 10.0634 20.8139 9.66478 20.4942 9.39444L12.9942 2.79444Z";

const DISCOVER_GEOMETRY: &str = "M12 2C6.47 2 2 6.47 2 12s4.47 10 10 10 10-4.47 10-10S17.53 2 12 2zm4.24 5.76-3.03 6.55-6.55 3.03L9.69 10.8l6.55-3.04zM12 13.5c-.83 0-1.5-.67-1.5-1.5s.67-1.5 1.5-1.5 1.5.67 1.5 1.5-.67 1.5-1.5 1.5z";

const ALERTS_GEOMETRY: &str = "M12 22c1.1 0 2-.9 2-2h-4c0 1.1.9 2 2 2zm6-6v-5c0-3.07-1.64-5.64-4.5-6.32V4c0-.83-.67-1.5-1.5-1.5s-1.5.67-1.5 1.5v.68C7.63 5.36 6 7.92 6 11v5l-2 2v1h16v-1l-2-2z";

const PROFILE_GEOMETRY: &str = "M12 2C9.243 2 7 4.243 7 7s2.243 5 5 5 5-2.243 5-5-2.243-5-5-5zM12 14c-5.523 0-10 3.582-10 8a1 1 0 001 1h18a1 1 0 001-1c0-4.418-4.477-8-10-8z";

#[repr(C)]
pub struct TabbedPageFabPage {
    base: UserControl,
    post_count: Cell<i32>,
}

user_control_class!(TabbedPageFabPage);
ferro_class_info!(TabbedPageFabPage { new: TabbedPageFabPage::new });
xaml_class!(TabbedPageFabPage, "/Pages/TabbedPage/TabbedPageFabPage.xaml");

impl TabbedPageFabPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), post_count: Cell::new(0) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.setup_icons();

        // The handlers belong to children of the page: they hold the page weakly.
        for name in ["FabButton", "TriggerFabButton"] {
            let weak = this.downgrade();
            this.get_control::<Button>(name).click(move |sender, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_fab_clicked(sender, e);
                }
            });
        }
        this
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    /// `page.Icon = new PathIcon { Data = geometry }` for the named page.
    fn set_icon(&self, page: &str, geometry: &str) {
        let icon = PathIcon::new();
        icon.set_data(parse_geometry(geometry));
        self.get_control::<ContentPage>(page).set_icon(Some(Control::boxed(icon)));
    }

    fn setup_icons(&self) {
        self.set_icon("FeedPage", FEED_GEOMETRY);
        self.set_icon("DiscoverPage", DISCOVER_GEOMETRY);
        self.set_icon("AlertsPage", ALERTS_GEOMETRY);
        self.set_icon("ProfilePage", PROFILE_GEOMETRY);
    }

    fn on_fab_clicked(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.post_count.set(self.post_count.get() + 1);
        let post_count = self.post_count.get();
        self.status_text().set_text(Some(&if post_count == 1 {
            String::from("Post created! Check your feed.")
        } else {
            format!("{post_count} posts created!")
        }));
    }
}
