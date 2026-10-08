//! Port of the code-behind of the streaming showcase of the upstream sample: the class of the
//! document `Pages/NavigationPage/FerroFlixAppPage.xaml`.

use super::{FerroFlixDetailView, FerroFlixHomeView, FerroFlixSearchView};
use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{parse_color, parse_geometry};
use ferroui_base::animation::{PageSlide, SlideAxis, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::{Brushes, FontWeight, IBrush, SolidColorBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, CornerRadius, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, Thickness, Visual, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Border, Button, ContentControlImpl, ContentPage, Control, ControlImpl, ControlImplExt, DrawerPage,
    NavigationPage, PathIcon, ScrollViewer, StackPanel, TextBlock, UserControl,
};
use mini_mvvm::start_async;
use std::cell::RefCell;
use std::rc::Rc;

const SEARCH_ICON: &str = "M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z";

const SHARE_ICON: &str = "M18,16.08C17.24,16.08 16.56,16.38 16.04,16.85L8.91,12.7C8.96,12.47 9,12.24 9,12C9,11.76 8.96,11.53 8.91,11.3L15.96,7.19C16.5,7.69 17.21,8 18,8A3,3 0 0,0 21,5A3,3 0 0,0 18,2A3,3 0 0,0 15,5C15,5.24 15.04,5.47 15.09,5.7L8.04,9.81C7.5,9.31 6.79,9 6,9A3,3 0 0,0 3,12A3,3 0 0,0 6,15C6.79,15 7.5,14.69 8.04,14.19L15.16,18.35C15.11,18.56 15.08,18.78 15.08,19C15.08,20.61 16.39,21.92 18,21.92C19.61,21.92 20.92,20.61 20.92,19C20.92,17.39 19.61,16.08 18,16.08Z";

const BOOKMARK_ICON: &str = "M17,3H7A2,2 0 0,0 5,5V21L12,18L19,21V5C19,3.89 18.1,3 17,3Z";

/// `new SolidColorBrush(Color.Parse(text))` as the value of a brush property.
fn solid(text: &str) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(parse_color(text)).into())
}

/// `Brushes.White` as the value of a brush property.
fn white() -> Option<Rc<dyn IBrush>> {
    let white: Rc<dyn IBrush> = Brushes::white();
    Some(white)
}

/// `Brushes.Transparent` as the value of a brush property.
fn transparent() -> Option<Rc<dyn IBrush>> {
    let transparent: Rc<dyn IBrush> = Brushes::transparent();
    Some(transparent)
}

/// `new PathIcon { Width = 20, Height = 20, Foreground = Brushes.White, Data = Geometry.Parse(data) }`.
fn white_icon(data: &str) -> Ref<PathIcon> {
    let icon = PathIcon::new();
    icon.set_width(20.0);
    icon.set_height(20.0);
    icon.set_foreground(white());
    icon.set_data(parse_geometry(data));
    icon
}

#[repr(C)]
pub struct FerroFlixAppPage {
    base: UserControl,
    detail_nav: RefCell<Option<Ref<NavigationPage>>>,
    info_panel: RefCell<Option<Ref<ScrollViewer>>>,
}

ferro_class!(FerroFlixAppPage: UserControl);
ferro_impl_classes!(
    FerroFlixAppPage: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(FerroFlixAppPage {
    new: FerroFlixAppPage::new,
    markup: {
        methods: [
            fn OnMenuItemClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<FerroFlixAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_menu_item_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(FerroFlixAppPage, "/Pages/NavigationPage/FerroFlixAppPage.xaml");

impl ControlImpl for FerroFlixAppPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        *this.info_panel.borrow_mut() = this.find_control::<ScrollViewer>("InfoPanel");
        this.update_info_panel_visibility();
    }
}

impl FerroObjectImpl for FerroFlixAppPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Visual::bounds_property().as_property() {
            this.update_info_panel_visibility();
        }
    }
}

impl FerroFlixAppPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), detail_nav: RefCell::new(None), info_panel: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        *this.detail_nav.borrow_mut() = this.find_control::<NavigationPage>("DetailNav");
        let detail_nav = this.detail_nav.borrow().clone();
        if let Some(detail_nav) = detail_nav {
            detail_nav.set_modal_transition(Some(Rc::new(PageSlide::with_duration(
                TimeSpan::from_milliseconds(300.0),
                SlideAxis::Vertical,
            ))));

            let home_view = FerroFlixHomeView::new();
            // The view ends up in a page of the navigation page of this control: its actions
            // hold this control weakly.
            let weak = this.downgrade();
            home_view.set_movie_selected(Some(Rc::new(move |title: &str| {
                if let Some(this) = weak.upgrade() {
                    this.push_detail_page(title);
                }
            })));
            let weak = this.downgrade();
            home_view.set_search_requested(Some(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.push_search_page();
                }
            })));

            let home_page = ContentPage::new();
            home_page.set_content(Some(Control::boxed(home_view)));
            home_page.set_background(transparent());
            home_page.set_header(Some(Control::boxed(Self::build_home_header())));
            NavigationPage::set_top_command_bar(&home_page, this.build_home_command_bar());
            drop(detail_nav.push_async(home_page));
        }
        this
    }

    fn update_info_panel_visibility(&self) {
        if let Some(info_panel) = self.info_panel.borrow().as_ref() {
            info_panel.set_is_visible(self.bounds().width >= 650.0);
        }
    }

    fn build_home_header() -> Ref<TextBlock> {
        let header = TextBlock::new();
        header.set_text(Some("FERROFLIX"));
        header.set_foreground(solid("#E50914"));
        header.set_font_size(18.0);
        header.set_font_weight(FontWeight::Black);
        header.set_vertical_alignment(VerticalAlignment::Center);
        header
    }

    fn build_home_command_bar(&self) -> Ref<StackPanel> {
        let cmd_bar = StackPanel::new();
        cmd_bar.set_orientation(Orientation::Horizontal);
        cmd_bar.set_spacing(12.0);
        cmd_bar.set_vertical_alignment(VerticalAlignment::Center);
        cmd_bar.set_margin(Thickness::new(0.0, 0.0, 8.0, 0.0));

        let search_btn = Button::new();
        search_btn.set_padding(Thickness::uniform(4.0));
        search_btn.classes().add("flixTransparent");
        // The button ends up in a page of the navigation page of this control: its handler
        // holds this control weakly.
        let weak = self.to_ref().downgrade();
        search_btn.click(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.on_search_click(e);
            }
        });
        search_btn.set_content(Some(Control::boxed(white_icon(SEARCH_ICON))));
        cmd_bar.children().add(search_btn);

        let initials = TextBlock::new();
        initials.set_text(Some("JD"));
        initials.set_font_size(10.0);
        initials.set_font_weight(FontWeight::Bold);
        initials.set_foreground(white());
        initials.set_horizontal_alignment(HorizontalAlignment::Center);
        initials.set_vertical_alignment(VerticalAlignment::Center);

        let avatar = Border::new();
        avatar.set_width(30.0);
        avatar.set_height(30.0);
        avatar.set_corner_radius(CornerRadius::uniform(4.0));
        avatar.set_background(solid("#333333"));
        avatar.set_child(initials);
        cmd_bar.children().add(avatar);
        cmd_bar
    }

    /// `async void`: the drawer is closed once the detail page is pushed.
    fn push_detail_page(&self, title: &str) {
        let detail_nav = self.detail_nav.borrow().clone();
        let Some(detail_nav) = detail_nav else {
            return;
        };

        let detail_view = FerroFlixDetailView::with_movie_title(title);

        let header_title = TextBlock::new();
        header_title.set_text(Some(title));
        header_title.set_font_size(17.0);
        header_title.set_font_weight(FontWeight::Bold);
        header_title.set_foreground(white());
        header_title.set_vertical_alignment(VerticalAlignment::Center);

        let share_btn = Button::new();
        share_btn.set_padding(Thickness::uniform(8.0));
        share_btn.set_content(Some(Control::boxed(white_icon(SHARE_ICON))));
        share_btn.classes().add("flixTransparent");

        let bookmark_btn = Button::new();
        bookmark_btn.set_padding(Thickness::uniform(8.0));
        bookmark_btn.set_content(Some(Control::boxed(white_icon(BOOKMARK_ICON))));
        bookmark_btn.classes().add("flixTransparent");

        let detail_cmd_bar = StackPanel::new();
        detail_cmd_bar.set_orientation(Orientation::Horizontal);
        detail_cmd_bar.set_spacing(8.0);
        detail_cmd_bar.set_vertical_alignment(VerticalAlignment::Center);
        detail_cmd_bar.children().add(share_btn);
        detail_cmd_bar.children().add(bookmark_btn);

        let detail_page = ContentPage::new();
        detail_page.set_content(Some(Control::boxed(detail_view)));
        detail_page.set_background(transparent());
        detail_page.set_header(Some(Control::boxed(header_title)));
        NavigationPage::set_top_command_bar(&detail_page, detail_cmd_bar);

        let pushed = detail_nav.push_async(detail_page);
        let this = self.to_ref();
        drop(start_async(async move {
            if pushed.await.is_err() {
                return;
            }
            if let Some(drawer) = this.find_control::<DrawerPage>("DrawerPageControl") {
                if drawer.is_open() {
                    drawer.set_is_open(false);
                }
            }
        }));
    }

    /// `async void`: nothing follows the push of the search page.
    fn on_search_click(&self, _e: &RoutedEventArgs) {
        self.push_search_page();
    }

    /// `PushSearchPageAsync`: nothing follows the push of the modal page, so its task is not
    /// awaited here.
    fn push_search_page(&self) {
        let detail_nav = self.detail_nav.borrow().clone();
        let Some(detail_nav) = detail_nav else {
            return;
        };

        let search_view = FerroFlixSearchView::new();
        // The view ends up in a modal page of the navigation page of this control: its actions
        // hold this control weakly.
        let weak = self.to_ref().downgrade();
        search_view.set_close_requested(Some(Rc::new(move || {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let detail_nav = this.detail_nav.borrow().clone();
            if let Some(detail_nav) = detail_nav {
                drop(detail_nav.pop_modal_async());
            }
        })));
        let weak = self.to_ref().downgrade();
        search_view.set_movie_selected(Some(Rc::new(move |title: &str| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let detail_nav = this.detail_nav.borrow().clone();
            match detail_nav.filter(|detail_nav| !detail_nav.modal_stack().is_empty()) {
                Some(detail_nav) => {
                    let popped = detail_nav.pop_modal_async();
                    let title = title.to_string();
                    drop(start_async(async move {
                        if popped.await.is_err() {
                            return;
                        }
                        this.push_detail_page(&title);
                    }));
                }
                None => this.push_detail_page(title),
            }
        })));

        let search_page = ContentPage::new();
        search_page.set_content(Some(Control::boxed(search_view)));
        search_page.set_background(solid("#0A0A0A"));
        NavigationPage::set_has_navigation_bar(&search_page, false);

        drop(detail_nav.push_modal_async(search_page));
    }

    fn on_menu_item_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(drawer) = self.find_control::<DrawerPage>("DrawerPageControl") {
            drawer.set_is_open(false);
        }

        let detail_nav = self.detail_nav.borrow().clone();
        if let Some(detail_nav) = detail_nav {
            drop(detail_nav.pop_to_root_async());
        }
    }
}
