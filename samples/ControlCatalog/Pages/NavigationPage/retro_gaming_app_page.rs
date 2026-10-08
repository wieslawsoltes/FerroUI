//! Port of `Pages/NavigationPage/RetroGamingAppPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/RetroGamingAppPage.xaml`.

use super::{
    RetroGamingDetailView, RetroGamingFavoritesView, RetroGamingGamesView, RetroGamingHomeView,
    RetroGamingProfileView, RetroGamingSearchView,
};
use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, parse_geometry};
use ferroui_base::animation::{PageSlide, SlideAxis, TimeSpan};
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::{BoxShadow, BoxShadows, Brushes, Color, FontFamily, FontWeight, IBrush, SolidColorBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, CornerRadius, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, Thickness, Visual, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    BarLayoutBehavior, Border, Button, ContentControlImpl, ContentPage, Control, ControlImpl, ControlImplExt, Grid,
    NavigationPage, Page, Panel, PathIcon, ScrollViewer, StackPanel, TabPlacement, TabbedPage, TextBlock,
    UserControl,
};
use mini_mvvm::start_async;
use std::cell::RefCell;
use std::rc::Rc;

const SEARCH_ICON: &str = "M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z";

const HEART_ICON: &str = "M12,21.35L10.55,20.03C5.4,15.36 2,12.27 2,8.5C2,5.41 4.42,3 7.5,3C9.24,3 10.91,3.81 12,5.08C13.09,3.81 14.76,3 16.5,3C19.58,3 22,5.41 22,8.5C22,12.27 18.6,15.36 13.45,20.03L12,21.35Z";

const SHARE_ICON: &str = "M18,16.08C17.24,16.08 16.56,16.38 16.04,16.85L8.91,12.7C8.96,12.47 9,12.24 9,12C9,11.76 8.96,11.53 8.91,11.3L15.96,7.19C16.5,7.69 17.21,8 18,8A3,3 0 0,0 21,5A3,3 0 0,0 18,2A3,3 0 0,0 15,5C15,5.24 15.04,5.47 15.09,5.7L8.04,9.81C7.5,9.31 6.79,9 6,9A3,3 0 0,0 3,12A3,3 0 0,0 6,15C6.79,15 7.5,14.69 8.04,14.19L15.16,18.35C15.11,18.56 15.08,18.78 15.08,19C15.08,20.61 16.39,21.92 18,21.92C19.61,21.92 20.92,20.61 20.92,19C20.92,17.39 19.61,16.08 18,16.08Z";

/// The colors of the page (the static fields `BgColor`, `SurfaceColor`, `CyanColor`,
/// `YellowColor`, `MutedColor` and `TextColor`).
fn bg_color() -> Color {
    parse_color("#120a1f")
}

fn surface_color() -> Color {
    parse_color("#2d1b4e")
}

fn cyan_color() -> Color {
    parse_color("#00ffff")
}

fn yellow_color() -> Color {
    parse_color("#ffff00")
}

fn muted_color() -> Color {
    parse_color("#7856a8")
}

fn text_color() -> Color {
    parse_color("#e0d0ff")
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

/// `new PathIcon { Width = size, Height = size, Foreground = new SolidColorBrush(color), Data = ... }`.
fn sized_icon(data: &str, size: f64, color: Color) -> Ref<PathIcon> {
    let icon = PathIcon::new();
    icon.set_width(size);
    icon.set_height(size);
    icon.set_foreground(solid(color));
    icon.set_data(parse_geometry(data));
    icon
}

#[repr(C)]
pub struct RetroGamingAppPage {
    base: UserControl,
    nav: RefCell<Option<Ref<NavigationPage>>>,
    info_panel: RefCell<Option<Ref<ScrollViewer>>>,
}

ferro_class!(RetroGamingAppPage: UserControl);
ferro_impl_classes!(
    RetroGamingAppPage: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(RetroGamingAppPage { new: RetroGamingAppPage::new });
xaml_class!(RetroGamingAppPage, "/Pages/NavigationPage/RetroGamingAppPage.xaml");

impl ControlImpl for RetroGamingAppPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        *this.info_panel.borrow_mut() = this.find_control::<ScrollViewer>("InfoPanel");
        this.update_info_panel_visibility();
    }
}

impl FerroObjectImpl for RetroGamingAppPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Visual::bounds_property().as_property() {
            this.update_info_panel_visibility();
        }
    }
}

impl RetroGamingAppPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), nav: RefCell::new(None), info_panel: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        *this.nav.borrow_mut() = this.find_control::<NavigationPage>("RetroNav");
        let nav = this.nav.borrow().clone();
        if let Some(nav) = nav {
            drop(nav.push_async(this.build_home_page()));
        }
        this
    }

    fn update_info_panel_visibility(&self) {
        if let Some(info_panel) = self.info_panel.borrow().as_ref() {
            info_panel.set_is_visible(self.bounds().width >= 650.0);
        }
    }

    fn build_home_page(&self) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(bg_color()));
        page.set_header(Some(Control::boxed(Self::build_pixel_arcade_logo())));
        NavigationPage::set_top_command_bar(&page, Self::build_nav_bar_right());

        let panel = Panel::new();
        panel.children().add(self.build_home_tabbed_page());
        panel.children().add(self.build_search_fab());

        page.set_content(Some(Control::boxed(panel)));
        page
    }

    /// A line of the logo: `PIXEL` or `ARCADE`.
    fn logo_line(text: &str) -> Ref<TextBlock> {
        let line = TextBlock::new();
        line.set_text(Some(text));
        line.set_font_family(FontFamily::new("Courier New, monospace"));
        line.set_font_size(14.0);
        line.set_font_weight(FontWeight::Bold);
        line.set_foreground(solid(yellow_color()));
        line.set_line_height(16.0);
        line
    }

    fn build_pixel_arcade_logo() -> Ref<StackPanel> {
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(10.0);
        row.set_vertical_alignment(VerticalAlignment::Center);

        let icon_panel = Grid::new();
        icon_panel.set_width(36.0);
        icon_panel.set_height(30.0);

        let body = Border::new();
        body.set_width(36.0);
        body.set_height(20.0);
        body.set_corner_radius(CornerRadius::uniform(3.0));
        body.set_background(solid(parse_color("#cc44dd")));
        body.set_vertical_alignment(VerticalAlignment::Bottom);
        icon_panel.children().add(body);

        let left_eye = Border::new();
        left_eye.set_width(9.0);
        left_eye.set_height(9.0);
        left_eye.set_background(solid(surface_color()));
        left_eye.set_horizontal_alignment(HorizontalAlignment::Left);
        left_eye.set_vertical_alignment(VerticalAlignment::Bottom);
        left_eye.set_margin(Thickness::new(4.0, 0.0, 0.0, 6.0));
        icon_panel.children().add(left_eye);

        let right_eye = Border::new();
        right_eye.set_width(9.0);
        right_eye.set_height(9.0);
        right_eye.set_background(solid(surface_color()));
        right_eye.set_horizontal_alignment(HorizontalAlignment::Right);
        right_eye.set_vertical_alignment(VerticalAlignment::Bottom);
        right_eye.set_margin(Thickness::new(0.0, 0.0, 4.0, 6.0));
        icon_panel.children().add(right_eye);
        row.children().add(icon_panel);

        let text_stack = StackPanel::new();
        text_stack.set_spacing(1.0);
        text_stack.children().add(Self::logo_line("PIXEL"));
        text_stack.children().add(Self::logo_line("ARCADE"));
        row.children().add(text_stack);
        row
    }

    fn build_nav_bar_right() -> Ref<StackPanel> {
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(10.0);
        row.set_vertical_alignment(VerticalAlignment::Center);
        row.set_margin(Thickness::new(0.0, 0.0, 8.0, 0.0));
        row.children().add(sized_icon(
            "M12 22c1.1 0 2-.9 2-2h-4c0 1.1.9 2 2 2zm6-6v-5c0-3.07-1.64-5.64-4.5-6.32V4c0-.83-.67-1.5-1.5-1.5s-1.5.67-1.5 1.5v.68C7.63 5.36 6 7.92 6 11v5l-2 2v1h16v-1l-2-2z",
            16.0,
            text_color(),
        ));

        let avatar = Border::new();
        avatar.set_width(26.0);
        avatar.set_height(26.0);
        avatar.set_corner_radius(CornerRadius::uniform(0.0));
        avatar.set_clip_to_bounds(true);
        avatar.set_background(solid(surface_color()));
        avatar.set_border_brush(solid(muted_color()));
        avatar.set_border_thickness(Thickness::uniform(1.0));

        let player = TextBlock::new();
        player.set_text(Some("P1"));
        player.set_font_family(FontFamily::new("Courier New, monospace"));
        player.set_font_size(7.0);
        player.set_font_weight(FontWeight::Bold);
        player.set_foreground(solid(cyan_color()));
        player.set_horizontal_alignment(HorizontalAlignment::Center);
        player.set_vertical_alignment(VerticalAlignment::Center);
        avatar.set_child(player);
        row.children().add(avatar);
        row
    }

    /// A tab of the home page: `new ContentPage { Header = ..., Icon = ..., Background = ..., Content = ... }`.
    fn tab(header: &str, icon_data: &str, content: BoxedValue) -> Ref<Page> {
        let tab = ContentPage::new();
        tab.set_header(Some(boxed_text(header)));
        tab.set_icon(page_icon(icon_data));
        tab.set_background(solid(bg_color()));
        tab.set_content(Some(content));
        tab.upcast()
    }

    fn build_home_tabbed_page(&self) -> Ref<TabbedPage> {
        let tp = TabbedPage::new();
        tp.set_background(solid(bg_color()));
        tp.set_tab_placement(TabPlacement::Bottom);
        tp.set_page_transition(Some(Rc::new(PageSlide::with_duration(
            TimeSpan::from_milliseconds(250.0),
            SlideAxis::Horizontal,
        ))));
        tp.resources().set("TabItemHeaderFontSize", Some(Rc::new(12.0_f64)));
        tp.resources().set("TabbedPageTabStripBackground", solid_resource(surface_color()));
        tp.resources().set("TabbedPageTabItemHeaderForegroundSelected", solid_resource(parse_color("#ad2bee")));
        tp.resources().set("TabbedPageTabItemHeaderForegroundUnselected", solid_resource(muted_color()));

        // The views end up in pages of the navigation page of this control: their actions
        // hold this control weakly.
        let home_view = RetroGamingHomeView::new();
        let weak = self.to_ref().downgrade();
        home_view.set_game_selected(Some(Rc::new(move |game_title: &str| {
            if let Some(this) = weak.upgrade() {
                this.push_detail_page(game_title);
            }
        })));

        let home_tab = Self::tab("Home", "M10,20V14H14V20H19V12H22L12,3L2,12H5V20H10Z", Control::boxed(home_view));

        let games_view = RetroGamingGamesView::new();
        let weak = self.to_ref().downgrade();
        games_view.set_game_selected(Some(Rc::new(move |game_title: &str| {
            if let Some(this) = weak.upgrade() {
                this.push_detail_page(game_title);
            }
        })));

        let games_tab = Self::tab(
            "Games",
            "M7.97,16L5,19C4.67,19.3 4.23,19.5 3.75,19.5A1.75,1.75 0 0,1 2,17.75V17.5L3,10.12C3.21,7.81 5.14,6 7.5,6H16.5C18.86,6 20.79,7.81 21,10.12L22,17.5V17.75A1.75,1.75 0 0,1 20.25,19.5C19.77,19.5 19.33,19.3 19,19L16.03,16H7.97M7,9V11H5V13H7V15H9V13H11V11H9V9H7M14.5,12A1.5,1.5 0 0,0 13,13.5A1.5,1.5 0 0,0 14.5,15A1.5,1.5 0 0,0 16,13.5A1.5,1.5 0 0,0 14.5,12M17.5,9A1.5,1.5 0 0,0 16,10.5A1.5,1.5 0 0,0 17.5,12A1.5,1.5 0 0,0 19,10.5A1.5,1.5 0 0,0 17.5,9Z",
            Control::boxed(games_view),
        );

        let fav_tab = Self::tab("Favorites", HEART_ICON, Control::boxed(RetroGamingFavoritesView::new()));

        let profile_tab = Self::tab(
            "Profile",
            "M12,4A4,4 0 0,1 16,8A4,4 0 0,1 12,12A4,4 0 0,1 8,8A4,4 0 0,1 12,4M12,14C16.42,14 20,15.79 20,18V20H4V18C4,15.79 7.58,14 12,14Z",
            Control::boxed(RetroGamingProfileView::new()),
        );

        tp.set_pages(Some(FerroList::from_items([home_tab, games_tab, fav_tab, profile_tab])));
        tp
    }

    fn build_search_fab(&self) -> Ref<Border> {
        let fab = Button::new();
        fab.set_width(50.0);
        fab.set_height(50.0);
        fab.set_corner_radius(CornerRadius::uniform(0.0));
        fab.set_background(solid(yellow_color()));
        fab.set_padding(Thickness::uniform(0.0));
        fab.classes().add("retro-fab");
        fab.set_content(Some(Control::boxed(sized_icon(SEARCH_ICON, 22.0, bg_color()))));
        // The button ends up in a page of the navigation page of this control: its handler
        // holds this control weakly.
        let weak = self.to_ref().downgrade();
        fab.click(move |_, _| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let nav = this.nav.borrow().clone();
            if let Some(nav) = nav {
                drop(nav.push_modal_async(this.build_search_modal()));
            }
        });

        let wrap = Border::new();
        wrap.set_horizontal_alignment(HorizontalAlignment::Center);
        wrap.set_vertical_alignment(VerticalAlignment::Bottom);
        wrap.set_margin(Thickness::new(0.0, 0.0, 0.0, 35.0));
        wrap.set_box_shadow(BoxShadows::new(BoxShadow {
            blur: 10.0,
            spread: 1.0,
            color: Color::from_argb(140, 255, 255, 0),
            ..BoxShadow::default()
        }));
        wrap.set_child(fab);
        wrap
    }

    fn build_search_modal(&self) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(bg_color()));

        let search_view = RetroGamingSearchView::new();
        // The view ends up in a modal page of the navigation page of this control: its actions
        // hold this control weakly.
        let weak = self.to_ref().downgrade();
        search_view.set_close_requested(Some(Rc::new(move || {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let nav = this.nav.borrow().clone();
            if let Some(nav) = nav {
                drop(nav.pop_modal_async());
            }
        })));
        let weak = self.to_ref().downgrade();
        search_view.set_game_selected(Some(Rc::new(move |game_title: &str| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            // `await (_nav?.PopModalAsync() ?? Task.CompletedTask)`, then the detail page.
            let nav = this.nav.borrow().clone();
            match nav {
                Some(nav) => {
                    let popped = nav.pop_modal_async();
                    let game_title = game_title.to_string();
                    drop(start_async(async move {
                        if popped.await.is_err() {
                            return;
                        }
                        this.push_detail_page(&game_title);
                    }));
                }
                None => this.push_detail_page(game_title),
            }
        })));

        page.set_content(Some(Control::boxed(search_view)));
        page
    }

    /// `async void`: nothing follows the push.
    fn push_detail_page(&self, game_title: &str) {
        let nav = self.nav.borrow().clone();
        let Some(nav) = nav else {
            return;
        };

        let detail_view = RetroGamingDetailView::with_game_title(game_title);

        let page = ContentPage::new();
        page.set_background(solid(bg_color()));
        page.set_content(Some(Control::boxed(detail_view)));

        NavigationPage::set_bar_layout_behavior(&page, Some(BarLayoutBehavior::Overlay));
        // The page ends up in the navigation page of this control: its handlers hold this
        // control weakly.
        let weak = self.to_ref().downgrade();
        page.navigated_to(move |_| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let nav = this.nav.borrow().clone();
            if let Some(nav) = nav {
                let transparent: Rc<dyn IBrush> = Brushes::transparent();
                nav.resources().set("NavigationBarBackground", Some(Rc::new(transparent)));
            }
        });
        let weak = self.to_ref().downgrade();
        page.navigated_from(move |_| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let nav = this.nav.borrow().clone();
            if let Some(nav) = nav {
                nav.resources().set("NavigationBarBackground", solid_resource(surface_color()));
            }
        });

        let cmd_bar = StackPanel::new();
        cmd_bar.set_orientation(Orientation::Horizontal);
        cmd_bar.set_spacing(4.0);
        cmd_bar.set_vertical_alignment(VerticalAlignment::Center);
        cmd_bar.set_margin(Thickness::new(0.0, 0.0, 8.0, 0.0));

        let heart_btn = Button::new();
        heart_btn.classes().add("retro-icon-btn");
        heart_btn.set_content(Some(Control::boxed(sized_icon(HEART_ICON, 16.0, parse_color("#ad2bee")))));

        let share_btn = Button::new();
        share_btn.classes().add("retro-icon-btn");
        share_btn.set_content(Some(Control::boxed(sized_icon(SHARE_ICON, 16.0, text_color()))));

        cmd_bar.children().add(heart_btn);
        cmd_bar.children().add(share_btn);
        NavigationPage::set_top_command_bar(&page, cmd_bar);

        drop(nav.push_async(page));
    }
}
