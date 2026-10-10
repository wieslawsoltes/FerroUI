//! Port of upstream's `Controls/NavigationPageTests.cs`.
//!
//! Upstream awaits the pushes; `wait` runs the jobs of the dispatcher until
//! the push has completed.

use crate::test_base::{test_font_family, CompareOptions, TestBase};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, SolidColorBrush};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::threading::{Dispatcher, DispatcherTask};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Border, ContentPage, Control, Decorator, NavigationPage, TextBlock};
use ferroui_themes_simple::SimpleTheme;
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Controls\NavigationPage")
}

fn font_style() -> Ref<Style> {
    Style::with_setters(
        Selectors::of_type::<TextBlock>(),
        [Setter::new(TextBlock::font_family_property(), test_font_family())],
    )
}

fn text(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_string()))
}

fn solid(color: &str) -> Ref<SolidColorBrush> {
    SolidColorBrush::with_color(Color::parse(color).expect("the color is valid"))
}

fn wait(task: DispatcherTask<()>) {
    if !task.is_completed() {
        Dispatcher::ui_thread().run_jobs(None);
    }
    assert!(task.is_completed(), "the navigation did not complete");
    task.result().expect("the navigation was not canceled")
}

fn page_content() -> Ref<Border> {
    let border = Border::new();
    border.set_width(80.0);
    border.set_height(40.0);
    border.set_background(Some(solid("#F5F5F5").into()));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    border
}

fn content_page(header: &str) -> Ref<ContentPage> {
    let page = ContentPage::new();
    page.set_header(text(header));
    page.set_background(Some(Brushes::white()));
    page.set_content(Some(Control::boxed(page_content())));
    page.set_horizontal_content_alignment(HorizontalAlignment::Center);
    page.set_vertical_content_alignment(VerticalAlignment::Center);
    page
}

const SKIP_IMMEDIATE: CompareOptions = CompareOptions { skip_immediate: true, skip_compositor: false };

#[test]
fn navigation_page_single_page_shows_nav_bar() {
    let t = base();
    let nav = NavigationPage::new();
    nav.set_background(Some(Brushes::white()));
    nav.resources().set("NavigationBarBackground", Some(Rc::new(solid("#1565C0"))));
    nav.resources().set("NavigationBarForeground", Some(Rc::new(Brushes::white())));
    let target = Decorator::new();
    target.set_width(400.0);
    target.set_height(300.0);
    target.set_child(&nav);
    wait(nav.push_async(content_page("Home")));

    target.styles().add(SimpleTheme::new().as_style());
    target.styles().add(font_style());
    t.render_to_file(&target, "NavigationPage_SinglePage_ShowsNavBar");
    t.compare_images_with("NavigationPage_SinglePage_ShowsNavBar", SKIP_IMMEDIATE);
}

#[test]
fn navigation_page_two_pages_shows_back_button() {
    let t = base();
    let nav = NavigationPage::new();
    nav.set_background(Some(Brushes::white()));
    nav.resources().set("NavigationBarBackground", Some(Rc::new(solid("#1565C0"))));
    nav.resources().set("NavigationBarForeground", Some(Rc::new(Brushes::white())));
    let target = Decorator::new();
    target.set_width(400.0);
    target.set_height(300.0);
    target.set_child(&nav);
    wait(nav.push_async(content_page("Home")));

    wait(nav.push_async_with_transition(content_page("Details"), None));

    target.styles().add(SimpleTheme::new().as_style());
    target.styles().add(font_style());
    t.render_to_file(&target, "NavigationPage_TwoPages_ShowsBackButton");
    t.compare_images_with("NavigationPage_TwoPages_ShowsBackButton", SKIP_IMMEDIATE);
}

#[test]
fn navigation_page_custom_bar_background() {
    let t = base();
    let nav = NavigationPage::new();
    nav.set_background(Some(Brushes::white()));
    nav.set_has_shadow(true);
    nav.resources().set("NavigationBarBackground", Some(Rc::new(solid("#2E7D32"))));
    nav.resources().set("NavigationBarForeground", Some(Rc::new(Brushes::white())));
    let target = Decorator::new();
    target.set_width(400.0);
    target.set_height(300.0);
    target.set_child(&nav);

    wait(nav.push_async(content_page("Green Theme")));

    target.styles().add(SimpleTheme::new().as_style());
    target.styles().add(font_style());
    t.render_to_file(&target, "NavigationPage_CustomBarBackground");
    t.compare_images_with("NavigationPage_CustomBarBackground", SKIP_IMMEDIATE);
}
