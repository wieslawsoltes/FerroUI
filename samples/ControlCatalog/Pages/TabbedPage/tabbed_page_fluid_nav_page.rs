//! Port of `Pages/TabbedPage/TabbedPageFluidNavPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageFluidNavPage.xaml`.

use super::{FluidNavBar, FluidNavItem};
use crate::markup::{user_control_class, xaml_class};
use ferroui_base::collections::FerroList;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{TabbedPage, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

// The paths of the icons: the coordinate system is centred at (0,0), roughly in the range of
// 12 in each direction. All paths are open strokes so the path measure traces them naturally.

/// A house: the roof (an open triangle) and the body (an open rectangle).
const HOME_PATH: &str = "M-12,2 L0,-12 L12,2 M-7,2 L-7,10 L7,10 L7,2";

/// A magnifying glass: the circle of the head and the line of the handle.
const EXPLORE_PATH: &str = "M6,-6 A7,7 0 0 1 0,-13 A7,7 0 0 1 -7,-6 A7,7 0 0 1 0,1 A7,7 0 0 1 6,-6 M3.9,1.1 L10,8";

/// A person: the circle of the head (4 quarter arcs) and the arc of the shoulders.
const PROFILE_PATH: &str =
    "M5,-7 A5,5 0 0 1 0,-2 A5,5 0 0 1 -5,-7 A5,5 0 0 1 0,-12 A5,5 0 0 1 5,-7 M-9,12 A9,7 0 0 1 9,12";

#[repr(C)]
pub struct TabbedPageFluidNavPage {
    base: UserControl,
    syncing: Cell<bool>,
}

user_control_class!(TabbedPageFluidNavPage);
ferro_class_info!(TabbedPageFluidNavPage { new: TabbedPageFluidNavPage::new });
xaml_class!(TabbedPageFluidNavPage, "/Pages/TabbedPage/TabbedPageFluidNavPage.xaml");

impl TabbedPageFluidNavPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), syncing: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.setup_nav_bar();
        this.wire_events();
        this
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    fn tabbed_page_control(&self) -> Ref<TabbedPage> {
        self.get_control::<TabbedPage>("TabbedPageControl")
    }

    fn nav_bar(&self) -> Ref<FluidNavBar> {
        self.get_control::<FluidNavBar>("NavBar")
    }

    fn setup_nav_bar(&self) {
        let nav_bar = self.nav_bar();
        nav_bar.set_items(FerroList::from_items([
            FluidNavItem::new(HOME_PATH, "Home"),
            FluidNavItem::new(EXPLORE_PATH, "Explore"),
            FluidNavItem::new(PROFILE_PATH, "Profile"),
        ]));
        nav_bar.set_selected_index(0);
    }

    fn wire_events(&self) {
        // The handlers belong to children of the page: they hold the page weakly.

        // A tap of the bar selects the tab of the tabbed page.
        let weak = self.to_ref().downgrade();
        self.nav_bar().selection_changed().add(Rc::new(move |index: &i32| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            if this.syncing.get() {
                return;
            }
            this.syncing.set(true);
            this.tabbed_page_control().set_selected_index(*index);
            this.update_status();
            this.syncing.set(false);
        }));

        // A swipe of the tabbed page selects the item of the bar.
        let weak = self.to_ref().downgrade();
        self.tabbed_page_control().selection_changed(move |_, _| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            if this.syncing.get() {
                return;
            }
            let i = this.tabbed_page_control().selected_index();
            if this.nav_bar().selected_index() != i {
                this.syncing.set(true);
                this.nav_bar().set_selected_index(i);
                this.update_status();
                this.syncing.set(false);
            }
        });
    }

    fn update_status(&self) {
        let names = ["Home", "Explore", "Profile"];
        let idx = self.tabbed_page_control().selected_index();
        let tab = usize::try_from(idx).ok().and_then(|idx| names.get(idx).copied()).unwrap_or("?");
        self.status_text().set_text(Some(&format!("Active tab: {tab}  (swipe or tap)")));
    }
}
