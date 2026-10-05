//! Port of `Pages/DrawerPage/EcoTrackerAppPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/EcoTrackerAppPage.xaml`.

use super::{EcoTrackerCommunityView, EcoTrackerHabitsView, EcoTrackerHomeView, EcoTrackerStatsView};
use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, parse_geometry};
use ferroui_base::animation::{CrossFade, IPageTransition, PageSlide, SlideAxis, TimeSpan};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, FontWeight, IBrush, SolidColorBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, CornerRadius, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, Thickness, Visual, VisualImpl,
};
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControlImpl};
use ferroui_controls::{
    Border, Button, ColumnDefinitions, ContentControlImpl, ContentPage, Control, ControlImpl, ControlImplExt,
    DrawerPage, Grid, NavigationPage, PathIcon, ScrollViewer, StackPanel, TextBlock, UserControl,
};
use mini_mvvm::start_async;
use std::cell::RefCell;
use std::rc::Rc;

const LEAF_PATH: &str = "M12 3C9 6 6 9 6 13C6 17.4 8.7 21 12 22C15.3 21 18 17.4 18 13C18 9 15 6 12 3Z";

/// The colors of the page (the static fields `Accent`, `BgLight`, `TextDark` and `TextMuted`;
/// the field `Primary` of the original is not used).
fn accent() -> Color {
    parse_color("#4CAF50")
}

fn bg_light() -> Color {
    parse_color("#F1F8E9")
}

fn text_dark() -> Color {
    parse_color("#1A2E1C")
}

fn text_muted() -> Color {
    parse_color("#90A4AE")
}

/// `new SolidColorBrush(color)` as the value of a brush property.
fn solid(color: Color) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(color).into())
}

fn white() -> Option<Rc<dyn IBrush>> {
    let white: Rc<dyn IBrush> = Brushes::white();
    Some(white)
}

/// `new ColumnDefinitions(text)`.
fn column_definitions(text: &str) -> ColumnDefinitions {
    match ColumnDefinitions::parse(text) {
        Ok(definitions) => definitions,
        Err(error) => panic!("{error}"),
    }
}

#[repr(C)]
pub struct EcoTrackerAppPage {
    base: UserControl,
    nav_page: RefCell<Option<Ref<NavigationPage>>>,
    drawer_page: RefCell<Option<Ref<DrawerPage>>>,
    info_panel: RefCell<Option<Ref<ScrollViewer>>>,
    selected_btn: RefCell<Option<Ref<Button>>>,
}

ferro_class!(EcoTrackerAppPage: UserControl);
ferro_impl_classes!(
    EcoTrackerAppPage: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(EcoTrackerAppPage {
    new: EcoTrackerAppPage::new,
    markup: {
        methods: [
            fn OnDrawerToggleClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EcoTrackerAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_drawer_toggle_click(&sender, e.as_routed_event_args())
                },
            fn OnMenuItemClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EcoTrackerAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_menu_item_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(EcoTrackerAppPage, "/Pages/DrawerPage/EcoTrackerAppPage.xaml");

impl ControlImpl for EcoTrackerAppPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.update_info_panel_visibility();
    }
}

impl FerroObjectImpl for EcoTrackerAppPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Visual::bounds_property().as_property() {
            this.update_info_panel_visibility();
        }
    }
}

impl EcoTrackerAppPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            nav_page: RefCell::new(None),
            drawer_page: RefCell::new(None),
            info_panel: RefCell::new(None),
            selected_btn: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        *this.info_panel.borrow_mut() = this.find_control::<ScrollViewer>("InfoPanel");
        *this.nav_page.borrow_mut() = this.find_control::<NavigationPage>("NavPage");
        *this.drawer_page.borrow_mut() = this.find_control::<DrawerPage>("DrawerPageControl");
        *this.selected_btn.borrow_mut() = this.find_control::<Button>("BtnHome");

        let nav_page = this.nav_page.borrow().clone();
        if let Some(nav_page) = nav_page {
            drop(nav_page.push_async(this.build_home_page()));
        }
        this
    }

    fn update_info_panel_visibility(&self) {
        if let Some(info_panel) = self.info_panel.borrow().as_ref() {
            info_panel.set_is_visible(self.bounds().width >= 650.0);
        }
    }

    fn on_drawer_toggle_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(drawer_page) = self.drawer_page.borrow().as_ref() {
            drawer_page.set_is_open(!drawer_page.is_open());
        }
    }

    /// `async void`: the page is pushed once the stack is popped to its root.
    fn on_menu_item_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let btn = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Button>());
        let Some(btn) = btn else {
            return;
        };
        let Some(tag) = btn.tag().and_then(|tag| tag.downcast_ref::<String>().cloned()) else {
            return;
        };

        let selected_btn = self.selected_btn.borrow().clone();
        if let Some(selected_btn) = selected_btn {
            selected_btn.classes().remove("ecoNavItemSelected");
            selected_btn.classes().add("ecoNavItem");
        }

        *self.selected_btn.borrow_mut() = Some(btn.clone());
        btn.classes().add("ecoNavItemSelected");

        if let Some(drawer_page) = self.drawer_page.borrow().as_ref() {
            drawer_page.set_is_open(false);
        }

        let nav_page = self.nav_page.borrow().clone();
        let Some(nav_page) = nav_page else {
            return;
        };

        let page = match tag.as_str() {
            "Home" => self.build_home_page(),
            "Stats" => Self::build_stats_page(),
            "Habits" => Self::build_habits_page(),
            "Community" => Self::build_community_page(),
            _ => self.build_home_page(),
        };

        NavigationPage::set_has_back_button(&page, false);

        let popped = nav_page.pop_to_root_async_with_transition(None);
        drop(start_async(async move {
            if popped.await.is_err() {
                return;
            }
            let transition: Rc<dyn IPageTransition> =
                Rc::new(CrossFade::with_duration(TimeSpan::from_milliseconds(200.0)));
            let _ = nav_page.push_async_with_transition(page, Some(transition)).await;
        }));
    }

    fn build_home_page(&self) -> Ref<ContentPage> {
        let home_view = EcoTrackerHomeView::new();
        // The view ends up in a page of the navigation page of this control: its action holds
        // this control weakly.
        let weak = self.to_ref().downgrade();
        home_view.set_tree_detail_requested(Some(Rc::new(move || {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let nav_page = this.nav_page.borrow().clone();
            if let Some(nav_page) = nav_page {
                let transition: Rc<dyn IPageTransition> = Rc::new(PageSlide::with_duration(
                    TimeSpan::from_milliseconds(250.0),
                    SlideAxis::Horizontal,
                ));
                drop(nav_page.push_async_with_transition(Self::build_tree_detail_page(), Some(transition)));
            }
        })));

        let page = ContentPage::new();
        page.set_background(solid(bg_light()));
        page.set_content(Some(Control::boxed(home_view)));
        page.set_header(Some(boxed_text("Home")));
        NavigationPage::set_has_back_button(&page, false);
        page
    }

    fn build_stats_page() -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(bg_light()));
        page.set_content(Some(Control::boxed(EcoTrackerStatsView::new())));
        page.set_header(Some(boxed_text("Stats")));
        page
    }

    fn build_habits_page() -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(bg_light()));
        page.set_content(Some(Control::boxed(EcoTrackerHabitsView::new())));
        page.set_header(Some(boxed_text("Habits")));
        page
    }

    fn build_community_page() -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(bg_light()));
        page.set_content(Some(Control::boxed(EcoTrackerCommunityView::new())));
        page.set_header(Some(boxed_text("Community")));
        page
    }

    fn build_tree_detail_page() -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(bg_light()));
        page.set_header(Some(boxed_text("Tree Progress")));

        let scroll = ScrollViewer::new();
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        let root = StackPanel::new();
        root.set_spacing(0.0);

        let summary_card = Border::new();
        summary_card.set_corner_radius(CornerRadius::uniform(16.0));
        summary_card.set_background(white());
        summary_card.set_padding(Thickness::uniform(20.0));
        summary_card.set_margin(Thickness::new(16.0, 20.0, 16.0, 0.0));

        let summary_stack = StackPanel::new();
        summary_stack.set_spacing(12.0);

        let progress_header = Grid::new();
        progress_header.set_column_definitions(column_definitions("*,Auto"));
        progress_header.children().add(Self::label("Your Tree Goal", 18.0, FontWeight::Bold, text_dark()));
        let pct_label = Self::label("75%", 18.0, FontWeight::Black, accent());
        Grid::set_column(&pct_label, 1);
        progress_header.children().add(pct_label);
        summary_stack.children().add(progress_header);

        let track_bg = Border::new();
        track_bg.set_height(10.0);
        track_bg.set_corner_radius(CornerRadius::uniform(5.0));
        track_bg.set_background(solid(parse_color("#E8F5E9")));

        let fill = Border::new();
        fill.set_height(10.0);
        fill.set_corner_radius(CornerRadius::uniform(5.0));
        fill.set_background(solid(accent()));
        fill.set_horizontal_alignment(HorizontalAlignment::Left);

        let progress_bar = Grid::new();
        progress_bar.children().add(track_bg);

        // The handler belongs to the grid: it holds the grid and its child weakly.
        let (weak_bar, weak_fill) = (progress_bar.downgrade(), fill.downgrade());
        progress_bar.layout_updated(move || {
            if let (Some(progress_bar), Some(fill)) = (weak_bar.upgrade(), weak_fill.upgrade()) {
                if progress_bar.bounds().width > 0.0 {
                    fill.set_width(progress_bar.bounds().width * 0.75);
                }
            }
        });

        progress_bar.children().add(fill);
        summary_stack.children().add(progress_bar);

        summary_stack.children().add(Self::label(
            "15 of 20 trees planted this month",
            13.0,
            FontWeight::Normal,
            text_muted(),
        ));

        summary_card.set_child(summary_stack);
        root.children().add(summary_card);

        let milestones_section = StackPanel::new();
        milestones_section.set_margin(Thickness::new(16.0, 20.0, 16.0, 0.0));
        milestones_section.set_spacing(10.0);
        milestones_section.children().add(Self::label("Milestones", 16.0, FontWeight::SemiBold, text_dark()));
        milestones_section.children().add(Self::build_milestone_item("First Tree", "Plant your first tree", true));
        milestones_section.children().add(Self::build_milestone_item("Green Starter", "Plant 5 trees", true));
        milestones_section.children().add(Self::build_milestone_item("Forest Builder", "Plant 10 trees", true));
        milestones_section.children().add(Self::build_milestone_item("Eco Champion", "Plant 20 trees", false));
        milestones_section.children().add(Self::build_milestone_item("Nature Guardian", "Plant 50 trees", false));
        root.children().add(milestones_section);

        let recent_section = StackPanel::new();
        recent_section.set_margin(Thickness::new(16.0, 20.0, 16.0, 20.0));
        recent_section.set_spacing(10.0);
        recent_section.children().add(Self::label("Recent Plantings", 16.0, FontWeight::SemiBold, text_dark()));
        recent_section.children().add(Self::build_planting_item("Oak Tree", "Central Park area", "2 days ago"));
        recent_section.children().add(Self::build_planting_item("Maple Tree", "Riverside zone", "5 days ago"));
        recent_section.children().add(Self::build_planting_item("Pine Tree", "Mountain trail", "1 week ago"));
        root.children().add(recent_section);

        scroll.set_content(Some(Control::boxed(root)));
        page.set_content(Some(Control::boxed(scroll)));
        page
    }

    fn build_milestone_item(title: &str, description: &str, achieved: bool) -> Ref<Border> {
        let card = Border::new();
        card.set_corner_radius(CornerRadius::uniform(10.0));
        card.set_background(white());
        card.set_padding(Thickness::uniform(12.0));

        let grid = Grid::new();
        grid.set_column_definitions(column_definitions("Auto,*"));

        let icon = Border::new();
        icon.set_width(36.0);
        icon.set_height(36.0);
        icon.set_corner_radius(CornerRadius::uniform(18.0));
        icon.set_background(solid(parse_color(if achieved { "#4CAF50" } else { "#E0E0E0" })));
        icon.set_vertical_alignment(VerticalAlignment::Center);

        let path_icon = PathIcon::new();
        path_icon.set_width(18.0);
        path_icon.set_height(18.0);
        if achieved {
            path_icon.set_data(parse_geometry("M9 16.2L4.8 12l-1.4 1.4L9 19 21 7l-1.4-1.4L9 16.2z"));
            path_icon.set_foreground(white());
        } else {
            path_icon.set_data(parse_geometry(
                "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z",
            ));
            path_icon.set_foreground(solid(parse_color("#BDBDBD")));
        }
        path_icon.set_horizontal_alignment(HorizontalAlignment::Center);
        path_icon.set_vertical_alignment(VerticalAlignment::Center);
        icon.set_child(path_icon);
        grid.children().add(icon);

        let info = StackPanel::new();
        info.set_margin(Thickness::new(12.0, 0.0, 0.0, 0.0));
        info.set_vertical_alignment(VerticalAlignment::Center);
        info.set_spacing(2.0);
        info.children().add(Self::label(
            title,
            14.0,
            FontWeight::SemiBold,
            parse_color(if achieved { "#1A2E1C" } else { "#9E9E9E" }),
        ));
        info.children().add(Self::label(description, 12.0, FontWeight::Normal, parse_color("#90A4AE")));
        Grid::set_column(&info, 1);
        grid.children().add(info);

        card.set_child(grid);
        card
    }

    fn build_planting_item(tree_name: &str, location: &str, time: &str) -> Ref<Border> {
        let card = Border::new();
        card.set_corner_radius(CornerRadius::uniform(10.0));
        card.set_background(white());
        card.set_padding(Thickness::uniform(12.0));

        let grid = Grid::new();
        grid.set_column_definitions(column_definitions("Auto,*,Auto"));

        let leaf = PathIcon::new();
        leaf.set_width(20.0);
        leaf.set_height(20.0);
        leaf.set_data(parse_geometry(LEAF_PATH));
        leaf.set_foreground(solid(parse_color("#4CAF50")));
        leaf.set_horizontal_alignment(HorizontalAlignment::Center);
        leaf.set_vertical_alignment(VerticalAlignment::Center);

        let tree_bg = Border::new();
        tree_bg.set_width(40.0);
        tree_bg.set_height(40.0);
        tree_bg.set_corner_radius(CornerRadius::uniform(10.0));
        tree_bg.set_background(solid(Color::from_argb(25, 76, 175, 80)));
        tree_bg.set_vertical_alignment(VerticalAlignment::Center);
        tree_bg.set_child(leaf);
        grid.children().add(tree_bg);

        let info = StackPanel::new();
        info.set_margin(Thickness::new(10.0, 0.0, 0.0, 0.0));
        info.set_vertical_alignment(VerticalAlignment::Center);
        info.set_spacing(2.0);
        info.children().add(Self::label(tree_name, 14.0, FontWeight::SemiBold, parse_color("#1A2E1C")));
        info.children().add(Self::label(location, 12.0, FontWeight::Normal, parse_color("#90A4AE")));
        Grid::set_column(&info, 1);
        grid.children().add(info);

        let time_label = Self::label(time, 11.0, FontWeight::Normal, parse_color("#90A4AE"));
        time_label.set_vertical_alignment(VerticalAlignment::Center);
        Grid::set_column(&time_label, 2);
        grid.children().add(time_label);

        card.set_child(grid);
        card
    }

    fn label(text: &str, size: f64, weight: FontWeight, color: Color) -> Ref<TextBlock> {
        let label = TextBlock::new();
        label.set_text(Some(text));
        label.set_font_size(size);
        label.set_font_weight(weight);
        label.set_foreground(solid(color));
        label
    }
}
