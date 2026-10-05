//! Port of `Pages/DrawerPage/ModernAppPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/ModernAppPage.xaml`.

use super::{ModernDiscoverView, ModernMyTripsView, ModernProfileView, ModernSettingsView};
use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{parse_color, value_text};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, IBrush, SolidColorBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, IntoRef, Ref, StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ContentControlImpl, ContentPage, Control, ControlImpl, ControlImplExt, DrawerPage, NavigationPage,
    ScrollViewer, TextBlock, UserControl,
};
use mini_mvvm::start_async;
use std::cell::RefCell;
use std::rc::Rc;

/// The background of the pages (the static property `BgBrush`: a new brush of the color
/// `BgLight` per use; the field `Primary` of the original is not used).
fn bg_brush() -> Rc<dyn IBrush> {
    SolidColorBrush::with_color(parse_color("#f5f8f8")).into()
}

#[repr(C)]
pub struct ModernAppPage {
    base: UserControl,
    drawer_page: RefCell<Option<Ref<DrawerPage>>>,
    nav_page: RefCell<Option<Ref<NavigationPage>>>,
    info_panel: RefCell<Option<Ref<ScrollViewer>>>,
    page_title: RefCell<Option<Ref<TextBlock>>>,
    selected_nav_btn: RefCell<Option<Ref<Button>>>,
}

ferro_class!(ModernAppPage: UserControl);
ferro_impl_classes!(
    ModernAppPage: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(ModernAppPage {
    new: ModernAppPage::new,
    markup: {
        methods: [
            fn OnNavClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ModernAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_nav_click(&sender, e.as_routed_event_args())
                },
            fn OnCloseDrawer(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ModernAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_close_drawer(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ModernAppPage, "/Pages/DrawerPage/ModernAppPage.xaml");

impl ControlImpl for ModernAppPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.update_info_panel_visibility();
    }
}

impl FerroObjectImpl for ModernAppPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Visual::bounds_property().as_property() {
            this.update_info_panel_visibility();
        }
    }
}

impl ModernAppPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            drawer_page: RefCell::new(None),
            nav_page: RefCell::new(None),
            info_panel: RefCell::new(None),
            page_title: RefCell::new(None),
            selected_nav_btn: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        *this.info_panel.borrow_mut() = this.find_control::<ScrollViewer>("InfoPanel");
        *this.drawer_page.borrow_mut() = this.find_control::<DrawerPage>("DrawerPageControl");
        *this.nav_page.borrow_mut() = this.find_control::<NavigationPage>("NavPage");
        *this.page_title.borrow_mut() = this.find_control::<TextBlock>("PageTitle");

        if this.nav_page.borrow().is_some() {
            this.navigate_to_discover();
        }
        this
    }

    fn update_info_panel_visibility(&self) {
        if let Some(info_panel) = self.info_panel.borrow().as_ref() {
            info_panel.set_is_visible(self.bounds().width >= 640.0);
        }
    }

    /// # Panics
    /// Panics if the document has no drawer page (a null reference in the managed original).
    fn on_nav_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let btn = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Button>());
        let Some(btn) = btn else {
            return;
        };
        self.select_nav_button(&btn);
        self.drawer_page.borrow().clone().expect("the drawer page of the document").set_is_open(false);

        match value_text(&btn.tag()).as_deref() {
            Some("Discover") => self.navigate_to_discover(),
            Some("MyTrips") => self.navigate_to_my_trips(),
            Some("Profile") => self.navigate_to_profile(),
            Some("Settings") => self.navigate_to_settings(),
            _ => {}
        }
    }

    fn on_close_drawer(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(drawer_page) = self.drawer_page.borrow().as_ref() {
            drawer_page.set_is_open(false);
        }
    }

    fn select_nav_button(&self, btn: &Ref<Button>) {
        let selected_nav_btn = self.selected_nav_btn.borrow().clone();
        if let Some(selected_nav_btn) = selected_nav_btn {
            let transparent: Rc<dyn IBrush> = Brushes::transparent();
            selected_nav_btn.set_background(Some(transparent));
        }
        *self.selected_nav_btn.borrow_mut() = Some(btn.clone());
        btn.set_background(Some(SolidColorBrush::with_color(parse_color("#1A0dccf2")).into()));
    }

    /// `async Task Navigate(page)`, started: the page is pushed once the stack is popped to
    /// its root.
    fn navigate(&self, page: Ref<ContentPage>) {
        let nav_page = self.nav_page.borrow().clone();
        let Some(nav_page) = nav_page else {
            return;
        };
        NavigationPage::set_has_back_button(&page, false);
        NavigationPage::set_has_navigation_bar(&page, false);
        let popped = nav_page.pop_to_root_async();
        drop(start_async(async move {
            if popped.await.is_err() {
                return;
            }
            let _ = nav_page.push_async(page).await;
        }));
    }

    /// The page of a view: `new ContentPage { Content = view, Background = BgBrush }`.
    fn page_of(view: impl IntoRef<Control>) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_content(Some(Control::boxed(view)));
        page.set_background(Some(bg_brush()));
        page
    }

    fn set_page_title(&self, title: &str) {
        if let Some(page_title) = self.page_title.borrow().as_ref() {
            page_title.set_text(Some(title));
        }
    }

    /// `this.FindControl<Button>(name)!`.
    fn nav_button(&self, name: &str) -> Ref<Button> {
        self.find_control::<Button>(name).expect("the button of the document")
    }

    /// `async void`.
    fn navigate_to_discover(&self) {
        self.set_page_title("Discover");
        self.select_nav_button(&self.nav_button("BtnDiscover"));
        self.navigate(Self::page_of(ModernDiscoverView::new()));
    }

    /// `async void`.
    fn navigate_to_my_trips(&self) {
        self.set_page_title("My Trips");
        self.select_nav_button(&self.nav_button("BtnMyTrips"));
        self.navigate(Self::page_of(ModernMyTripsView::new()));
    }

    /// `async void`.
    fn navigate_to_profile(&self) {
        self.set_page_title("Profile");
        self.select_nav_button(&self.nav_button("BtnProfile"));
        self.navigate(Self::page_of(ModernProfileView::new()));
    }

    /// `async void`.
    fn navigate_to_settings(&self) {
        self.set_page_title("Settings");
        self.select_nav_button(&self.nav_button("BtnSettings"));
        self.navigate(Self::page_of(ModernSettingsView::new()));
    }
}
