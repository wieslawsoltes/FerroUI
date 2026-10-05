//! Port of `Pages/DrawerPage/ControlsGalleryAppPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/ControlsGalleryAppPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, parse_geometry};
use ferroui_base::animation::{CrossFade, IPageTransition, TimeSpan};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{Color, FontWeight, GradientStop, IBrush, LinearGradientBrush, SolidColorBrush, TextWrapping};
use ferroui_base::{
    ferro_class_info, instantiate, BoxedValue, CornerRadius, Ref, RelativePoint, RelativeUnit, Thickness,
};
use ferroui_controls::{
    Border, Button, ColumnDefinitions, ContentPage, Control, DrawerPage, Grid, NavigationPage, PathIcon, ScrollViewer,
    StackPanel, TextBlock, TextBox, TextChangedEventArgs, UserControl, WrapPanel,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The colors of the page (the static fields of the original).
fn accent() -> Color {
    parse_color("#60CDFF")
}

fn content_bg() -> Color {
    parse_color("#141414")
}

fn card_bg() -> Color {
    parse_color("#1F1F1F")
}

fn border_col() -> Color {
    parse_color("#2EFFFFFF")
}

fn text_col() -> Color {
    parse_color("#FFFFFF")
}

fn text_sec() -> Color {
    parse_color("#C8FFFFFF")
}

fn text_muted() -> Color {
    parse_color("#80FFFFFF")
}

/// `new SolidColorBrush(color)` as the value of a brush property.
fn solid(color: Color) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(color).into())
}

/// `new ColumnDefinitions(text)`.
fn column_definitions(text: &str) -> ColumnDefinitions {
    match ColumnDefinitions::parse(text) {
        Ok(definitions) => definitions,
        Err(error) => panic!("{error}"),
    }
}

/// `new CrossFade(TimeSpan.FromMilliseconds(180))`.
fn cross_fade() -> Option<Rc<dyn IPageTransition>> {
    Some(Rc::new(CrossFade::with_duration(TimeSpan::from_milliseconds(180.0))))
}

/// The controls of the gallery by category.
const ALL_SECTIONS: [(&str, &[&str]); 6] = [
    ("Basic Input", &["Button", "CheckBox", "ComboBox", "RadioButton", "Slider", "ToggleButton", "ToggleSwitch"]),
    ("Collections", &["DataGrid", "ItemsControl", "ListBox", "ListView", "TreeView"]),
    ("Date & Time", &["CalendarDatePicker", "DatePicker", "TimePicker"]),
    ("Layout", &["Border", "Grid", "Panel", "StackPanel", "WrapPanel"]),
    ("Navigation", &["DrawerPage", "NavigationPage", "TabControl", "TabbedPage"]),
    ("Text", &["AutoCompleteBox", "RichTextBox", "TextBlock", "TextBox"]),
];

#[repr(C)]
pub struct ControlsGalleryAppPage {
    base: UserControl,
    drawer: RefCell<Option<Ref<DrawerPage>>>,
    detail_nav: RefCell<Option<Ref<NavigationPage>>>,
    selected_btn: RefCell<Option<Ref<Button>>>,
    search_box: RefCell<Option<Ref<TextBox>>>,
    pre_search_page: RefCell<Option<Ref<ContentPage>>>,
    is_searching: Cell<bool>,
}

user_control_class!(ControlsGalleryAppPage);
ferro_class_info!(ControlsGalleryAppPage {
    new: ControlsGalleryAppPage::new,
    markup: {
        methods: [
            fn OnHamburgerClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ControlsGalleryAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_hamburger_click(&sender, e.as_routed_event_args())
                },
            fn OnNavItemClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ControlsGalleryAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_nav_item_click(&sender, e.as_routed_event_args())
                },
            fn OnSearchTextChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ControlsGalleryAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<TextChangedEventArgs>() {
                        this.on_search_text_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(ControlsGalleryAppPage, "/Pages/DrawerPage/ControlsGalleryAppPage.xaml");

impl ControlsGalleryAppPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            drawer: RefCell::new(None),
            detail_nav: RefCell::new(None),
            selected_btn: RefCell::new(None),
            search_box: RefCell::new(None),
            pre_search_page: RefCell::new(None),
            is_searching: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        *this.drawer.borrow_mut() = this.find_control::<DrawerPage>("NavDrawer");
        *this.detail_nav.borrow_mut() = this.find_control::<NavigationPage>("DetailNav");
        *this.selected_btn.borrow_mut() = this.find_control::<Button>("BtnWhatsNew");
        *this.search_box.borrow_mut() = this.find_control::<TextBox>("SearchBox");

        let detail_nav = this.detail_nav.borrow().clone();
        if let Some(detail_nav) = detail_nav {
            drop(detail_nav.push_async(Self::build_whats_new_page()));
        }
        this
    }

    fn on_hamburger_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(drawer) = self.drawer.borrow().as_ref() {
            drawer.set_is_open(!drawer.is_open());
        }
    }

    /// `async void`: nothing follows the replacement.
    fn on_nav_item_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
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
        if selected_btn.as_ref() == Some(&btn) {
            return;
        }

        if let Some(selected_btn) = selected_btn {
            selected_btn.classes().remove("navItemSelected");
        }

        *self.selected_btn.borrow_mut() = Some(btn.clone());
        btn.classes().add("navItemSelected");

        let detail_nav = self.detail_nav.borrow().clone();
        let Some(detail_nav) = detail_nav else {
            return;
        };

        let page = match tag.as_str() {
            "WhatsNew" => Self::build_whats_new_page(),
            "AllControls" => Self::build_all_controls_page(),
            "BasicInput" => Self::build_category_page(
                "Basic Input",
                "Buttons, checkboxes, radio buttons, sliders, and toggle switches.",
            ),
            "Collections" => {
                Self::build_category_page("Collections", "List view, tree view, data grid, flip view, and more.")
            }
            "Media" => Self::build_category_page("Media", "Image, web view, map control, and media player."),
            "Menus" => {
                Self::build_category_page("Menus and Toolbars", "Menus, context menus, command bar, and toolbar.")
            }
            "Navigation" => {
                Self::build_category_page("Navigation", "Navigation view, pivot, tab control, and breadcrumb bar.")
            }
            "Text" => Self::build_category_page("Text", "Text block, text box, auto-suggest box, and rich text."),
            "Settings" => Self::build_settings_page(),
            _ => Self::build_whats_new_page(),
        };

        NavigationPage::set_has_back_button(&page, false);
        drop(detail_nav.replace_async_with_transition(page, cross_fade()));
    }

    fn on_search_text_changed(&self, _sender: &Option<BoxedValue>, _e: &TextChangedEventArgs) {
        let detail_nav = self.detail_nav.borrow().clone();
        let Some(detail_nav) = detail_nav else {
            return;
        };

        let query = self
            .search_box
            .borrow()
            .as_ref()
            .and_then(|search_box| search_box.text())
            .map(|text| text.trim().to_string())
            .unwrap_or_default();

        if query.is_empty() {
            if self.is_searching.get() {
                self.is_searching.set(false);
                let restore = self.pre_search_page.borrow_mut().take().unwrap_or_else(Self::build_whats_new_page);
                NavigationPage::set_has_back_button(&restore, false);
                drop(detail_nav.replace_async_with_transition(restore, cross_fade()));
            }
            return;
        }

        if !self.is_searching.get() {
            *self.pre_search_page.borrow_mut() =
                detail_nav.current_page().and_then(|page| page.cast::<ContentPage>());
            self.is_searching.set(true);
        }

        let results_page = Self::build_search_results_page(&query);
        NavigationPage::set_has_back_button(&results_page, false);
        drop(detail_nav.replace_async_with_transition(results_page, None));
    }

    /// A chip with the name of a control.
    fn chip(text: &str) -> Ref<Border> {
        let chip = Border::new();
        chip.set_margin(Thickness::new(0.0, 0.0, 8.0, 8.0));
        chip.set_padding(Thickness::symmetric(12.0, 6.0));
        chip.set_corner_radius(CornerRadius::uniform(4.0));
        chip.set_background(solid(card_bg()));
        chip.set_border_brush(solid(border_col()));
        chip.set_border_thickness(Thickness::uniform(1.0));
        chip.set_child(Self::txt(text, 12.0, FontWeight::Normal, text_col()));
        chip
    }

    /// A page of the gallery with its header: `new ContentPage { Background = ... }`.
    fn new_page(header: &str) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(content_bg()));
        page.set_header(Some(boxed_text(header)));
        page
    }

    fn build_search_results_page(query: &str) -> Ref<ContentPage> {
        let page = Self::new_page("Search");

        let scroll = ScrollViewer::new();
        let root = StackPanel::new();
        root.set_spacing(0.0);

        let q = query.to_lowercase();
        let mut any_match = false;

        for (category, controls) in ALL_SECTIONS {
            let matches: Vec<&str> = controls.iter().copied().filter(|c| c.to_lowercase().contains(&q)).collect();
            if matches.is_empty() {
                continue;
            }

            any_match = true;
            root.children().add(Self::section_header(category));

            let chips = WrapPanel::new();
            chips.set_margin(Thickness::new(24.0, 4.0, 24.0, 0.0));
            chips.set_orientation(Orientation::Horizontal);
            for ctrl in matches {
                chips.children().add(Self::chip(ctrl));
            }
            root.children().add(chips);
        }

        if !any_match {
            let empty = StackPanel::new();
            empty.set_horizontal_alignment(HorizontalAlignment::Center);
            empty.set_margin(Thickness::new(0.0, 40.0, 0.0, 0.0));
            empty.set_spacing(8.0);
            empty.children().add(Self::txt("No results", 16.0, FontWeight::SemiBold, text_col()));
            empty.children().add(Self::txt(
                &format!("No controls match \"{query}\""),
                13.0,
                FontWeight::Normal,
                text_sec(),
            ));
            root.children().add(empty);
        }

        scroll.set_content(Some(Control::boxed(root)));
        page.set_content(Some(Control::boxed(scroll)));
        page
    }

    fn build_whats_new_page() -> Ref<ContentPage> {
        let page = Self::new_page("What's New");

        let scroll = ScrollViewer::new();
        let root = StackPanel::new();
        root.set_spacing(0.0);

        let hero_brush = LinearGradientBrush::new();
        hero_brush.set_start_point(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
        hero_brush.set_end_point(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative));
        hero_brush.gradient_stops().add(GradientStop::with_color_and_offset(parse_color("#003B6F"), 0.0));
        hero_brush.gradient_stops().add(GradientStop::with_color_and_offset(parse_color("#0078D4"), 0.5));
        hero_brush.gradient_stops().add(GradientStop::with_color_and_offset(parse_color("#60CDFF"), 1.0));

        let hero_content = StackPanel::new();
        hero_content.set_margin(Thickness::uniform(24.0));
        hero_content.set_spacing(8.0);
        hero_content.set_vertical_alignment(VerticalAlignment::Bottom);
        hero_content.children().add(Self::txt("NEW IN FERROUI", 11.0, FontWeight::SemiBold, accent()));
        hero_content.children().add(Self::txt("Controls Gallery", 28.0, FontWeight::Bold, text_col()));
        let hero_text = TextBlock::new();
        hero_text.set_text(Some("Explore all controls, styles, and animations available in FerroUI."));
        hero_text.set_font_size(13.0);
        hero_text.set_foreground(solid(Color::from_argb(200, 255, 255, 255)));
        hero_text.set_text_wrapping(TextWrapping::Wrap);
        hero_content.children().add(hero_text);

        let hero = Border::new();
        hero.set_height(200.0);
        hero.set_margin(Thickness::new(24.0, 24.0, 24.0, 0.0));
        hero.set_corner_radius(CornerRadius::uniform(8.0));
        hero.set_background(Some(hero_brush.into()));
        hero.set_child(hero_content);
        root.children().add(hero);

        root.children().add(Self::section_header("New Controls"));
        let wrap = WrapPanel::new();
        wrap.set_margin(Thickness::new(24.0, 8.0, 24.0, 0.0));
        wrap.set_orientation(Orientation::Horizontal);

        let new_controls = [
            (
                "M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5",
                "DrawerPage",
                "Master-detail navigation with compact icon rail",
            ),
            (
                "M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z",
                "NavigationPage",
                "Push/pop stack with animated transitions",
            ),
            (
                "M3 3h8v8H3zm10 0h8v8h-8zM3 13h8v8H3zm10 0h8v8h-8z",
                "TabbedPage",
                "Multi-tab layout with swipe navigation",
            ),
            (
                "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z",
                "ContentPage",
                "Single-content page with lifecycle events",
            ),
        ];

        for (icon, title, desc) in new_controls {
            wrap.children().add(Self::control_card(icon, title, desc));
        }

        root.children().add(wrap);

        root.children().add(Self::section_header("Recently Updated"));
        let upd_list = StackPanel::new();
        upd_list.set_margin(Thickness::new(24.0, 8.0, 24.0, 24.0));
        upd_list.set_spacing(4.0);

        let updates = [
            ("CommandBar", "Overflow menu and compact label mode"),
            ("ContentPage", "Lifecycle events and navigation bar customization"),
        ];

        for (name, change) in updates {
            let info_stack = StackPanel::new();
            info_stack.set_spacing(2.0);
            info_stack.children().add(Self::txt(name, 13.0, FontWeight::SemiBold, text_col()));
            info_stack.children().add(Self::txt(change, 11.0, FontWeight::Normal, text_sec()));

            let row = Border::new();
            row.set_padding(Thickness::symmetric(12.0, 10.0));
            row.set_corner_radius(CornerRadius::uniform(4.0));
            row.set_background(solid(card_bg()));
            row.set_border_brush(solid(border_col()));
            row.set_border_thickness(Thickness::new(0.0, 0.0, 0.0, 1.0));
            row.set_child(info_stack);
            upd_list.children().add(row);
        }

        root.children().add(upd_list);
        scroll.set_content(Some(Control::boxed(root)));
        page.set_content(Some(Control::boxed(scroll)));
        page
    }

    fn control_card(icon_data: &str, title: &str, desc: &str) -> Ref<Border> {
        let stack = StackPanel::new();
        stack.set_spacing(8.0);

        let icon = PathIcon::new();
        icon.set_width(20.0);
        icon.set_height(20.0);
        icon.set_data(parse_geometry(icon_data));
        icon.set_foreground(solid(accent()));
        icon.set_horizontal_alignment(HorizontalAlignment::Left);
        stack.children().add(icon);

        stack.children().add(Self::txt(title, 14.0, FontWeight::SemiBold, text_col()));

        let desc_text = TextBlock::new();
        desc_text.set_text(Some(desc));
        desc_text.set_font_size(11.0);
        desc_text.set_foreground(solid(text_sec()));
        desc_text.set_text_wrapping(TextWrapping::Wrap);
        stack.children().add(desc_text);

        let card = Border::new();
        card.set_width(182.0);
        card.set_height(130.0);
        card.set_margin(Thickness::new(0.0, 0.0, 12.0, 12.0));
        card.set_corner_radius(CornerRadius::uniform(6.0));
        card.set_background(solid(card_bg()));
        card.set_border_brush(solid(border_col()));
        card.set_border_thickness(Thickness::uniform(1.0));
        card.set_padding(Thickness::uniform(16.0));
        card.set_child(stack);
        card
    }

    fn build_all_controls_page() -> Ref<ContentPage> {
        let page = Self::new_page("All Controls");

        let scroll = ScrollViewer::new();
        let root = StackPanel::new();
        root.set_spacing(0.0);

        for (category, controls) in ALL_SECTIONS {
            root.children().add(Self::section_header(category));
            let chips = WrapPanel::new();
            chips.set_margin(Thickness::new(24.0, 4.0, 24.0, 0.0));
            chips.set_orientation(Orientation::Horizontal);
            for ctrl in controls {
                chips.children().add(Self::chip(ctrl));
            }

            root.children().add(chips);
        }

        scroll.set_content(Some(Control::boxed(root)));
        page.set_content(Some(Control::boxed(scroll)));
        page
    }

    fn build_category_page(title: &str, description: &str) -> Ref<ContentPage> {
        let page = Self::new_page(title);

        let scroll = ScrollViewer::new();
        let root = StackPanel::new();
        root.set_spacing(0.0);

        let header_stack = StackPanel::new();
        header_stack.set_spacing(4.0);
        header_stack.children().add(Self::txt(title, 20.0, FontWeight::SemiBold, text_col()));
        header_stack.children().add(Self::txt(description, 13.0, FontWeight::Normal, text_sec()));

        let header = Border::new();
        header.set_margin(Thickness::new(24.0, 24.0, 24.0, 0.0));
        header.set_padding(Thickness::uniform(16.0));
        header.set_corner_radius(CornerRadius::uniform(6.0));
        header.set_background(solid(card_bg()));
        header.set_child(header_stack);
        root.children().add(header);

        root.children().add(Self::section_header("Controls"));
        let list = StackPanel::new();
        list.set_margin(Thickness::new(24.0, 4.0, 24.0, 24.0));
        list.set_spacing(4.0);

        let sample_names = ["Primary Control", "Secondary Control", "Advanced Control", "Variant A", "Variant B"];
        for (i, sample_name) in sample_names.into_iter().enumerate() {
            let row_grid = Grid::new();
            row_grid.set_column_definitions(column_definitions("*,Auto"));
            let label = StackPanel::new();
            label.set_spacing(2.0);
            label.children().add(Self::txt(sample_name, 13.0, FontWeight::SemiBold, text_col()));
            label.children().add(Self::txt(
                &format!("Example usage in {title}"),
                11.0,
                FontWeight::Normal,
                text_sec(),
            ));
            row_grid.children().add(label);

            if i < 2 {
                let badge = Border::new();
                badge.set_padding(Thickness::symmetric(6.0, 2.0));
                badge.set_corner_radius(CornerRadius::uniform(10.0));
                badge.set_background(solid(Color::from_argb(30, 96, 205, 255)));
                badge.set_child(Self::txt("NEW", 10.0, FontWeight::Bold, accent()));
                badge.set_horizontal_alignment(HorizontalAlignment::Right);
                badge.set_vertical_alignment(VerticalAlignment::Center);
                Grid::set_column(&badge, 1);
                row_grid.children().add(badge);
            }

            let row = Border::new();
            row.set_padding(Thickness::symmetric(16.0, 12.0));
            row.set_corner_radius(CornerRadius::uniform(4.0));
            row.set_background(solid(card_bg()));
            row.set_border_brush(solid(border_col()));
            row.set_border_thickness(Thickness::new(0.0, 0.0, 0.0, 1.0));
            row.set_child(row_grid);
            list.children().add(row);
        }

        root.children().add(list);
        scroll.set_content(Some(Control::boxed(root)));
        page.set_content(Some(Control::boxed(scroll)));
        page
    }

    fn build_settings_page() -> Ref<ContentPage> {
        let page = Self::new_page("Settings");

        let scroll = ScrollViewer::new();
        let root = StackPanel::new();
        root.set_spacing(0.0);

        root.children().add(Self::section_header("Appearance"));
        let appear_list = StackPanel::new();
        appear_list.set_margin(Thickness::new(24.0, 4.0, 24.0, 0.0));
        appear_list.set_spacing(2.0);
        appear_list.children().add(Self::settings_row("App theme", "Dark"));
        appear_list.children().add(Self::settings_row("Accent color", "#60CDFF"));
        appear_list.children().add(Self::settings_row("Font size", "Medium"));
        root.children().add(appear_list);

        root.children().add(Self::section_header("About"));
        let about_list = StackPanel::new();
        about_list.set_margin(Thickness::new(24.0, 4.0, 24.0, 24.0));
        about_list.set_spacing(2.0);
        about_list.children().add(Self::settings_row("Version", "1.0.0"));
        about_list.children().add(Self::settings_row("Framework", "FerroUI"));
        about_list.children().add(Self::settings_row("Theme", "Fluent"));
        root.children().add(about_list);

        scroll.set_content(Some(Control::boxed(root)));
        page.set_content(Some(Control::boxed(scroll)));
        page
    }

    fn section_header(title: &str) -> Ref<Border> {
        let header = Border::new();
        header.set_margin(Thickness::new(24.0, 20.0, 24.0, 0.0));
        header.set_padding(Thickness::new(0.0, 0.0, 0.0, 8.0));
        header.set_border_brush(solid(border_col()));
        header.set_border_thickness(Thickness::new(0.0, 0.0, 0.0, 1.0));
        header.set_child(Self::txt(title, 13.0, FontWeight::SemiBold, accent()));
        header
    }

    fn settings_row(label: &str, value: &str) -> Ref<Border> {
        let grid = Grid::new();
        grid.set_column_definitions(column_definitions("*,Auto"));
        grid.children().add(Self::txt(label, 13.0, FontWeight::Normal, text_col()));
        let val = Self::txt(value, 12.0, FontWeight::Normal, text_muted());
        val.set_vertical_alignment(VerticalAlignment::Center);
        Grid::set_column(&val, 1);
        grid.children().add(val);

        let row = Border::new();
        row.set_padding(Thickness::symmetric(16.0, 14.0));
        row.set_corner_radius(CornerRadius::uniform(4.0));
        row.set_background(solid(card_bg()));
        row.set_border_brush(solid(border_col()));
        row.set_border_thickness(Thickness::new(0.0, 0.0, 0.0, 1.0));
        row.set_child(grid);
        row
    }

    fn txt(text: &str, size: f64, weight: FontWeight, color: Color) -> Ref<TextBlock> {
        let text_block = TextBlock::new();
        text_block.set_text(Some(text));
        text_block.set_font_size(size);
        text_block.set_font_weight(weight);
        text_block.set_foreground(solid(color));
        text_block
    }
}
