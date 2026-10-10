//! Port of upstream's `Controls/TabbedPageTests.cs`.

use crate::test_base::{test_font_family, CompareOptions, TestBase};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, SolidColorBrush, StreamGeometry};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{ContentPage, Control, Decorator, Page, PageList, TabPlacement, TabbedPage, TextBlock};
use ferroui_themes_simple::SimpleTheme;
use std::rc::Rc;

const HOME_GEOMETRY: &str = "M10,20V14H14V20H19V12H22L12,3L2,12H5V20H10Z";
const SEARCH_GEOMETRY: &str = "M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z";
const SETTINGS_GEOMETRY: &str = "M12,15.5A3.5,3.5 0 0,1 8.5,12A3.5,3.5 0 0,1 12,8.5A3.5,3.5 0 0,1 15.5,12A3.5,3.5 0 0,1 12,15.5M19.43,12.97C19.47,12.65 19.5,12.33 19.5,12C19.5,11.67 19.47,11.34 19.43,11L21.54,9.37C21.73,9.22 21.78,8.95 21.66,8.73L19.66,5.27C19.54,5.05 19.27,4.96 19.05,5.05L16.56,6.05C16.04,5.66 15.5,5.32 14.87,5.07L14.5,2.42C14.46,2.18 14.25,2 14,2H10C9.75,2 9.54,2.18 9.5,2.42L9.13,5.07C8.5,5.32 7.96,5.66 7.44,6.05L4.95,5.05C4.73,4.96 4.46,5.05 4.34,5.27L2.34,8.73C2.21,8.95 2.27,9.22 2.46,9.37L4.57,11C4.53,11.34 4.5,11.67 4.5,12C4.5,12.33 4.53,12.65 4.57,12.97L2.46,14.63C2.27,14.78 2.21,15.05 2.34,15.27L4.34,18.73C4.46,18.95 4.73,19.04 4.95,18.95L7.44,17.94C7.96,18.34 8.5,18.68 9.13,18.93L9.5,21.58C9.54,21.82 9.75,22 10,22H14C14.25,22 14.46,21.82 14.5,21.58L14.87,18.93C15.5,18.67 16.04,18.34 16.56,17.94L19.05,18.95C19.27,19.04 19.54,18.95 19.66,18.73L21.66,15.27C21.78,15.05 21.73,14.78 21.54,14.63L19.43,12.97Z";

const SKIP_IMMEDIATE: CompareOptions = CompareOptions { skip_immediate: true, skip_compositor: false };

fn base() -> TestBase {
    TestBase::new(r"Controls\TabbedPage")
}

fn font_style() -> Ref<Style> {
    Style::with_setters(
        Selectors::of_type::<TextBlock>(),
        [Setter::new(TextBlock::font_family_property(), test_font_family())],
    )
}

fn solid(color: &str) -> Option<BoxedValue> {
    Some(Rc::new(SolidColorBrush::with_color(Color::parse(color).expect("the color is valid"))))
}

fn parse(data: &str) -> Ref<StreamGeometry> {
    StreamGeometry::parse(data).expect("the path data is valid")
}

/// A content page with a header, an optional icon and a centered text as its content.
fn content_page(header: &str, icon: Option<&Ref<StreamGeometry>>, text: &str) -> Ref<Page> {
    let page = ContentPage::new();
    page.set_header(Some(Rc::new(header.to_string())));
    if let Some(icon) = icon {
        page.set_icon(Some(Rc::new(icon.clone())));
    }
    let content = TextBlock::new();
    content.set_text(Some(text));
    content.set_foreground(Some(Brushes::black()));
    content.set_font_family(test_font_family());
    content.set_horizontal_alignment(HorizontalAlignment::Center);
    content.set_vertical_alignment(VerticalAlignment::Center);
    page.set_content(Some(Control::boxed(content)));
    page.upcast()
}

fn create_basic_tabbed_page() -> Ref<TabbedPage> {
    let tabbed_page = TabbedPage::new();
    tabbed_page.set_background(Some(Brushes::white()));
    tabbed_page.set_pages(Some(PageList::from_items([
        content_page("Home", None, "Home page"),
        content_page("Favorites", None, "Favorites page"),
        content_page("Settings", None, "Settings page"),
    ])));
    tabbed_page
}

fn create_tabbed_page_with_icons(placement: TabPlacement) -> Ref<TabbedPage> {
    let home_geometry = parse(HOME_GEOMETRY);
    let search_geometry = parse(SEARCH_GEOMETRY);
    let settings_geometry = parse(SETTINGS_GEOMETRY);

    let tabbed_page = TabbedPage::new();
    tabbed_page.set_background(Some(Brushes::white()));
    tabbed_page.set_tab_placement(placement);
    tabbed_page.set_pages(Some(PageList::from_items([
        content_page("Home", Some(&home_geometry), "Home page"),
        content_page("Search", Some(&search_geometry), "Search page"),
        content_page("Settings", Some(&settings_geometry), "Settings page"),
    ])));

    tabbed_page.set_selected_index(0);
    tabbed_page
}

fn render_and_compare(t: &TestBase, tabbed_page: &Ref<TabbedPage>, width: f64, test_name: &str) {
    let target = Decorator::new();
    target.set_width(width);
    target.set_height(300.0);
    target.set_child(tabbed_page);

    target.styles().add(SimpleTheme::new().as_style());
    target.styles().add(font_style());
    t.render_to_file(&target, test_name);
    t.compare_images_with(test_name, SKIP_IMMEDIATE);
}

#[test]
fn tabbed_page_default_top_placement_first_tab_selected() {
    let t = base();
    let tabbed_page = create_basic_tabbed_page();
    tabbed_page.set_tab_placement(TabPlacement::Top);
    tabbed_page.set_selected_index(0);

    render_and_compare(&t, &tabbed_page, 400.0, "TabbedPage_Default_TopPlacement_FirstTabSelected");
}

#[test]
fn tabbed_page_top_placement_second_tab_selected() {
    let t = base();
    let tabbed_page = create_basic_tabbed_page();
    tabbed_page.set_tab_placement(TabPlacement::Top);
    tabbed_page.set_selected_index(1);

    render_and_compare(&t, &tabbed_page, 400.0, "TabbedPage_TopPlacement_SecondTabSelected");
}

#[test]
fn tabbed_page_bottom_placement() {
    let t = base();
    let tabbed_page = create_basic_tabbed_page();
    tabbed_page.set_tab_placement(TabPlacement::Bottom);
    tabbed_page.set_selected_index(0);

    render_and_compare(&t, &tabbed_page, 400.0, "TabbedPage_BottomPlacement");
}

#[test]
fn tabbed_page_left_placement() {
    let t = base();
    let tabbed_page = create_basic_tabbed_page();
    tabbed_page.set_tab_placement(TabPlacement::Left);
    tabbed_page.set_selected_index(0);

    render_and_compare(&t, &tabbed_page, 500.0, "TabbedPage_LeftPlacement");
}

#[test]
fn tabbed_page_right_placement() {
    let t = base();
    let tabbed_page = create_basic_tabbed_page();
    tabbed_page.set_tab_placement(TabPlacement::Right);
    tabbed_page.set_selected_index(0);

    render_and_compare(&t, &tabbed_page, 500.0, "TabbedPage_RightPlacement");
}

#[test]
fn tabbed_page_custom_bar_background() {
    let t = base();
    let tabbed_page = create_basic_tabbed_page();
    tabbed_page.set_tab_placement(TabPlacement::Top);
    tabbed_page.set_selected_index(0);
    tabbed_page.resources().set("TabbedPageTabStripBackground", solid("#2196F3"));
    tabbed_page.resources().set("TabbedPageTabItemHeaderForegroundSelected", Some(Rc::new(Brushes::white())));
    tabbed_page.resources().set("TabbedPageTabItemHeaderForegroundUnselected", Some(Rc::new(Brushes::white())));

    render_and_compare(&t, &tabbed_page, 400.0, "TabbedPage_CustomBarBackground");
}

#[test]
fn tabbed_page_custom_tab_colors() {
    let t = base();
    let tabbed_page = create_basic_tabbed_page();
    tabbed_page.set_tab_placement(TabPlacement::Bottom);
    tabbed_page.set_selected_index(1);
    tabbed_page.resources().set("TabbedPageTabItemHeaderForegroundSelected", solid("#E91E63"));
    tabbed_page.resources().set("TabbedPageTabItemHeaderForegroundUnselected", solid("#9E9E9E"));

    render_and_compare(&t, &tabbed_page, 400.0, "TabbedPage_CustomTabColors");
}

#[test]
fn tabbed_page_two_tabs() {
    let t = base();
    let tabbed_page = TabbedPage::new();
    tabbed_page.set_background(Some(Brushes::white()));
    tabbed_page.set_tab_placement(TabPlacement::Top);
    tabbed_page.set_pages(Some(PageList::from_items([
        content_page("First", None, "First tab content"),
        content_page("Second", None, "Second tab content"),
    ])));

    render_and_compare(&t, &tabbed_page, 400.0, "TabbedPage_TwoTabs");
}

#[test]
fn tabbed_page_with_icons_top_placement() {
    let t = base();
    let tabbed_page = create_tabbed_page_with_icons(TabPlacement::Top);

    render_and_compare(&t, &tabbed_page, 400.0, "TabbedPage_WithIcons_TopPlacement");
}

#[test]
fn tabbed_page_with_icons_bottom_placement() {
    let t = base();
    let tabbed_page = create_tabbed_page_with_icons(TabPlacement::Bottom);

    render_and_compare(&t, &tabbed_page, 400.0, "TabbedPage_WithIcons_BottomPlacement");
}
