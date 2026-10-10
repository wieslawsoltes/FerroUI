//! Port of upstream's `Controls/CarouselPageTests.cs`.

use crate::test_base::{test_font_family, CompareOptions, TestBase};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, FontWeight, IBrush, SolidColorBrush};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::Ref;
use ferroui_controls::{CarouselPage, ContentPage, Control, Decorator, Page, TextBlock};
use ferroui_themes_simple::SimpleTheme;
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Controls\CarouselPage")
}

fn font_style() -> Ref<Style> {
    Style::with_setters(
        Selectors::of_type::<TextBlock>(),
        [Setter::new(TextBlock::font_family_property(), test_font_family())],
    )
}

fn solid(color: &str) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(Color::parse(color).expect("the color is valid")).into())
}

fn make_page(label: &str, bg_hex: &str, fg_hex: &str) -> Ref<Page> {
    let page = ContentPage::new();
    page.set_header(Some(Rc::new(label.to_string())));
    page.set_background(solid(bg_hex));
    let content = TextBlock::new();
    content.set_text(Some(label));
    content.set_foreground(solid(fg_hex));
    content.set_font_size(28.0);
    content.set_font_weight(FontWeight::Bold);
    content.set_horizontal_alignment(HorizontalAlignment::Center);
    content.set_vertical_alignment(VerticalAlignment::Center);
    page.set_content(Some(Control::boxed(content)));
    page.set_horizontal_content_alignment(HorizontalAlignment::Center);
    page.set_vertical_content_alignment(VerticalAlignment::Center);
    page.upcast()
}

fn add_page(cp: &CarouselPage, page: Ref<Page>) {
    cp.pages().expect("the carousel page has its pages").add(page);
}

fn render_and_compare(t: &TestBase, cp: &Ref<CarouselPage>, test_name: &str) {
    let target = Decorator::new();
    target.set_width(400.0);
    target.set_height(300.0);
    target.set_child(cp);
    target.styles().add(SimpleTheme::new().as_style());
    target.styles().add(font_style());
    t.render_to_file(&target, test_name);
    t.compare_images_with(test_name, CompareOptions { skip_immediate: true, ..Default::default() });
}

#[test]
fn carousel_page_blue_page() {
    let t = base();
    let cp = CarouselPage::new();
    cp.set_background(Some(Brushes::white()));
    cp.set_page_transition(None);
    add_page(&cp, make_page("Page 1", "#1565C0", "#FFFFFF"));

    render_and_compare(&t, &cp, "CarouselPage_Blue_Page");
}

#[test]
fn carousel_page_green_page() {
    let t = base();
    let cp = CarouselPage::new();
    cp.set_background(Some(Brushes::white()));
    cp.set_page_transition(None);
    add_page(&cp, make_page("Page 2", "#2E7D32", "#FFFFFF"));

    render_and_compare(&t, &cp, "CarouselPage_Green_Page");
}

#[test]
fn carousel_page_red_page() {
    let t = base();
    let cp = CarouselPage::new();
    cp.set_background(Some(Brushes::white()));
    cp.set_page_transition(None);
    add_page(&cp, make_page("Page 3", "#C62828", "#FFFFFF"));

    render_and_compare(&t, &cp, "CarouselPage_Red_Page");
}

#[test]
fn carousel_page_three_pages_first_selected() {
    let t = base();
    let cp = CarouselPage::new();
    cp.set_background(Some(Brushes::white()));
    cp.set_page_transition(None);
    add_page(&cp, make_page("Page 1", "#1565C0", "#FFFFFF"));
    add_page(&cp, make_page("Page 2", "#2E7D32", "#FFFFFF"));
    add_page(&cp, make_page("Page 3", "#C62828", "#FFFFFF"));

    render_and_compare(&t, &cp, "CarouselPage_ThreePages_FirstSelected");
}

#[test]
fn carousel_page_three_pages_second_selected() {
    let t = base();
    let cp = CarouselPage::new();
    cp.set_background(Some(Brushes::white()));
    cp.set_page_transition(None);
    add_page(&cp, make_page("Page 1", "#1565C0", "#FFFFFF"));
    add_page(&cp, make_page("Page 2", "#2E7D32", "#FFFFFF"));
    add_page(&cp, make_page("Page 3", "#C62828", "#FFFFFF"));
    cp.set_selected_index(1);

    render_and_compare(&t, &cp, "CarouselPage_ThreePages_SecondSelected");
}

#[test]
fn carousel_page_custom_background() {
    let t = base();
    let cp = CarouselPage::new();
    cp.set_background(solid("#212121"));
    cp.set_page_transition(None);
    add_page(&cp, make_page("Dark Theme", "#F57F17", "#212121"));

    render_and_compare(&t, &cp, "CarouselPage_CustomBackground");
}
