//! Port of upstream's `Controls/ContentPageTests.cs`.

use crate::test_base::{test_font_family, CompareOptions, TestBase};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, IBrush, SolidColorBrush, Stretch, StreamGeometry};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::shapes::Path;
use ferroui_controls::{
    Border, CommandBar, CommandBarButton, CommandBarDefaultLabelPosition, CommandBarOverflowButtonVisibility,
    CommandBarSeparator, CommandBarToggleButton, ContentPage, Control, Decorator, TextBlock,
};
use ferroui_themes_simple::SimpleTheme;
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Controls\ContentPage")
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

fn solid(color: &str) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(Color::parse(color).expect("the color is valid")).into())
}

fn icon(data: &str) -> Option<BoxedValue> {
    let path = Path::new();
    path.set_data(StreamGeometry::parse(data).expect("the path data is valid"));
    path.set_fill(Some(Brushes::black()));
    path.set_width(20.0);
    path.set_height(20.0);
    path.set_stretch(Stretch::Uniform);
    Some(Control::boxed(path))
}

fn page_content() -> Option<BoxedValue> {
    let border = Border::new();
    border.set_width(120.0);
    border.set_height(40.0);
    border.set_background(solid("#F5F5F5"));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    Some(Control::boxed(border))
}

#[test]
fn content_page_default_content() {
    let t = base();
    let target = Decorator::new();
    target.set_width(400.0);
    target.set_height(200.0);
    let page = ContentPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_header(text("My Page"));
    page.set_content(page_content());
    page.set_horizontal_content_alignment(HorizontalAlignment::Center);
    page.set_vertical_content_alignment(VerticalAlignment::Center);
    target.set_child(page);

    target.styles().add(SimpleTheme::new().as_style());
    target.styles().add(font_style());
    t.render_to_file(&target, "ContentPage_Default_Content");
    t.compare_images_with("ContentPage_Default_Content", CompareOptions { skip_immediate: true, ..Default::default() });
}

#[test]
fn content_page_with_top_and_bottom_command_bars() {
    let t = base();
    let target = Decorator::new();
    target.set_width(400.0);
    target.set_height(260.0);
    let page = ContentPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_header(text("Editor"));

    let top = CommandBar::new();
    top.set_background(Some(Brushes::light_gray()));
    let save = CommandBarButton::new();
    save.set_label(Some("Save"));
    save.set_icon(icon(
        "M15,9H5V5H15M12,19A3,3 0 0,1 9,16A3,3 0 0,1 12,13A3,3 0 0,1 15,16A3,3 0 0,1 12,19M17,3H5C3.89,3 3,3.9 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V7L17,3Z",
    ));
    top.primary_commands().add(save.as_command_bar_element());
    top.primary_commands().add(CommandBarSeparator::new().as_command_bar_element());
    let bold = CommandBarToggleButton::new();
    bold.set_label(Some("Bold"));
    bold.set_icon(icon(
        "M15.6,10.79C17.04,10.07 18,8.64 18,7C18,4.79 16.21,3 14,3H7V21H14.73C16.78,21 18.5,19.37 18.5,17.32C18.5,15.82 17.72,14.53 16.5,13.77C16.2,13.59 15.9,13.44 15.6,13.32V10.79M10,6.5H13C13.83,6.5 14.5,7.17 14.5,8C14.5,8.83 13.83,9.5 13,9.5H10V6.5M13.5,17.5H10V14H13.5C14.33,14 15,14.67 15,15.5C15,16.33 14.33,17.5 13.5,17.5Z",
    ));
    top.primary_commands().add(bold.as_command_bar_element());
    page.set_top_command_bar(Some(Control::boxed(top)));

    let bottom = CommandBar::new();
    bottom.set_background(Some(Brushes::light_gray()));
    bottom.set_default_label_position(CommandBarDefaultLabelPosition::Collapsed);
    bottom.set_overflow_button_visibility(CommandBarOverflowButtonVisibility::Collapsed);
    let add = CommandBarButton::new();
    add.set_icon(icon("M19,13H13V19H11V13H5V11H11V5H13V11H19V13Z"));
    bottom.primary_commands().add(add.as_command_bar_element());
    page.set_bottom_command_bar(Some(Control::boxed(bottom)));

    page.set_content(page_content());
    page.set_horizontal_content_alignment(HorizontalAlignment::Center);
    page.set_vertical_content_alignment(VerticalAlignment::Center);
    target.set_child(page);

    target.styles().add(SimpleTheme::new().as_style());
    target.styles().add(font_style());
    t.render_to_file(&target, "ContentPage_WithTopAndBottomCommandBars");
    t.compare_images_with(
        "ContentPage_WithTopAndBottomCommandBars",
        CompareOptions { skip_immediate: true, ..Default::default() },
    );
}
