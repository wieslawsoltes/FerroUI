//! Port of `Pages/NavigationPage/PulseAppPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/PulseAppPage.xaml`.

use super::{PulseHomeView, PulseLoginView, PulseProfileView, PulseWorkoutDetailView, PulseWorkoutsView};
use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, parse_geometry};
use ferroui_base::animation::{PageSlide, SlideAxis, TimeSpan};
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Color, IBrush, SolidColorBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, ContentPage, Control, ControlImpl, ControlImplExt, NavigationPage, Page, PathIcon,
    ScrollViewer, TabPlacement, TabbedPage, UserControl,
};
use mini_mvvm::start_async;
use std::cell::RefCell;
use std::rc::Rc;

/// The colors of the page (the static fields `Primary`, `BgDark`, `BgDashboard` and
/// `TextDimmed`).
fn primary() -> Color {
    parse_color("#256af4")
}

fn bg_dark() -> Color {
    parse_color("#101622")
}

fn bg_dashboard() -> Color {
    parse_color("#0a0a0a")
}

fn text_dimmed() -> Color {
    parse_color("#64748b")
}

/// `new SolidColorBrush(color)` as the value of a brush property.
fn solid(color: Color) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(color).into())
}

/// `new SolidColorBrush(color)` as the value of a resource.
fn solid_resource(color: Color) -> Option<BoxedValue> {
    let brush: Rc<dyn IBrush> = SolidColorBrush::with_color(color).into();
    Some(Rc::new(brush))
}

/// `new PathIcon { Data = Geometry.Parse(data) }` as the icon of a page.
fn page_icon(data: &str) -> Option<BoxedValue> {
    let icon = PathIcon::new();
    icon.set_data(parse_geometry(data));
    Some(Control::boxed(icon))
}

#[repr(C)]
pub struct PulseAppPage {
    base: UserControl,
    nav_page: RefCell<Option<Ref<NavigationPage>>>,
    login_page: RefCell<Option<Ref<ContentPage>>>,
    info_panel: RefCell<Option<Ref<ScrollViewer>>>,
}

ferro_class!(PulseAppPage: UserControl);
ferro_impl_classes!(
    PulseAppPage: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(PulseAppPage { new: PulseAppPage::new });
xaml_class!(PulseAppPage, "/Pages/NavigationPage/PulseAppPage.xaml");

impl ControlImpl for PulseAppPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        *this.info_panel.borrow_mut() = this.find_control::<ScrollViewer>("InfoPanel");
        this.update_info_panel_visibility();
    }
}

impl FerroObjectImpl for PulseAppPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Visual::bounds_property().as_property() {
            this.update_info_panel_visibility();
        }
    }
}

impl PulseAppPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            nav_page: RefCell::new(None),
            login_page: RefCell::new(None),
            info_panel: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        *this.nav_page.borrow_mut() = this.find_control::<NavigationPage>("NavPage");
        let nav_page = this.nav_page.borrow().clone();
        if let Some(nav_page) = nav_page {
            let login_page = this.build_login_page();
            *this.login_page.borrow_mut() = Some(login_page.clone());
            drop(nav_page.push_async(login_page));
        }
        this
    }

    fn update_info_panel_visibility(&self) {
        if let Some(info_panel) = self.info_panel.borrow().as_ref() {
            info_panel.set_is_visible(self.bounds().width >= 650.0);
        }
    }

    fn build_login_page(&self) -> Ref<ContentPage> {
        let login_view = PulseLoginView::new();
        // The view ends up in a page of the navigation page of this control: its action holds
        // this control weakly.
        let weak = self.to_ref().downgrade();
        login_view.set_login_requested(Some(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.on_login_requested();
            }
        })));

        let page = ContentPage::new();
        page.set_content(Some(Control::boxed(login_view)));
        page.set_background(solid(bg_dark()));
        NavigationPage::set_has_navigation_bar(&page, false);
        page
    }

    /// `async void`: the login page is removed once the dashboard is pushed.
    fn on_login_requested(&self) {
        let nav_page = self.nav_page.borrow().clone();
        let Some(nav_page) = nav_page else {
            return;
        };
        if self.login_page.borrow().is_none() {
            return;
        }

        let pushed = nav_page.push_async(self.build_dashboard_page());
        let this = self.to_ref();
        drop(start_async(async move {
            if pushed.await.is_err() {
                return;
            }
            // `_navPage.RemovePage(_loginPage)`: the fields are read again after the wait.
            let nav_page = this.nav_page.borrow().clone().expect("the navigation page of the document");
            let login_page = this.login_page.borrow().clone().expect("the login page");
            nav_page.remove_page(login_page);
            *this.login_page.borrow_mut() = None;
        }));
    }

    fn build_dashboard_page(&self) -> Ref<TabbedPage> {
        let tp = TabbedPage::new();
        tp.set_background(solid(bg_dashboard()));
        tp.set_tab_placement(TabPlacement::Bottom);
        tp.set_page_transition(Some(Rc::new(PageSlide::with_duration(
            TimeSpan::from_milliseconds(200.0),
            SlideAxis::Horizontal,
        ))));
        tp.resources().set("TabItemHeaderFontSize", Some(Rc::new(12.0_f64)));
        tp.resources().set("TabbedPageTabStripBackground", solid_resource(bg_dashboard()));
        tp.resources().set("TabbedPageTabItemHeaderForegroundSelected", solid_resource(primary()));
        tp.resources().set("TabbedPageTabItemHeaderForegroundUnselected", solid_resource(text_dimmed()));
        NavigationPage::set_has_navigation_bar(&tp, false);

        let home_view = PulseHomeView::new();
        // The view ends up in a page of the navigation page of this control: its action holds
        // this control weakly.
        let weak = self.to_ref().downgrade();
        home_view.set_workout_detail_requested(Some(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.push_workout_detail();
            }
        })));

        let home_page = ContentPage::new();
        home_page.set_content(Some(Control::boxed(home_view)));
        home_page.set_background(solid(bg_dashboard()));
        home_page.set_header(Some(boxed_text("Home")));
        home_page.set_icon(page_icon("M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"));

        let workouts_page = ContentPage::new();
        workouts_page.set_content(Some(Control::boxed(PulseWorkoutsView::new())));
        workouts_page.set_background(solid(bg_dashboard()));
        workouts_page.set_header(Some(boxed_text("Workouts")));
        workouts_page.set_icon(page_icon(
            "M20.57 14.86L22 13.43 20.57 12 17 15.57 8.43 7 12 3.43 10.57 2 9.14 3.43 7.71 2 5.57 4.14 4.14 2.71 2.71 4.14l1.43 1.43L2 7.71l1.43 1.43L2 10.57 3.43 12 7 8.43 15.57 17 12 20.57 13.43 22l1.43-1.43L16.29 22l2.14-2.14 1.43 1.43 1.43-1.43-1.43-1.43L22 16.29z",
        ));

        let profile_page = ContentPage::new();
        profile_page.set_content(Some(Control::boxed(PulseProfileView::new())));
        profile_page.set_background(solid(bg_dashboard()));
        profile_page.set_header(Some(boxed_text("Profile")));
        profile_page.set_icon(page_icon(
            "M12 2C9.243 2 7 4.243 7 7s2.243 5 5 5 5-2.243 5-5-2.243-5-5-5zM12 14c-5.523 0-10 3.582-10 8a1 1 0 001 1h18a1 1 0 001-1c0-4.418-4.477-8-10-8z",
        ));

        let pages: [Ref<Page>; 3] = [home_page.upcast(), workouts_page.upcast(), profile_page.upcast()];
        tp.set_pages(Some(FerroList::from_items(pages)));
        tp
    }

    /// `async void`.
    fn push_workout_detail(&self) {
        let nav_page = self.nav_page.borrow().clone();
        let Some(nav_page) = nav_page else {
            return;
        };

        let detail_view = PulseWorkoutDetailView::new();
        // The view ends up in a page of the navigation page of this control: its action holds
        // this control weakly.
        let weak = self.to_ref().downgrade();
        detail_view.set_back_requested(Some(Rc::new(move || {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let nav_page = this.nav_page.borrow().clone();
            if let Some(nav_page) = nav_page {
                drop(nav_page.pop_async());
            }
        })));

        let page = ContentPage::new();
        page.set_content(Some(Control::boxed(detail_view)));
        page.set_background(solid(bg_dark()));
        NavigationPage::set_has_navigation_bar(&page, false);
        drop(nav_page.push_async(page));
    }
}
