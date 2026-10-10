//! Port of upstream's `Controls/DrawerPageTests.cs`.
//!
//! The subtrees upstream writes out in every test (the text of the drawer, the text and the border of the
//! detail, the squares of the rail) are built by the helpers below with upstream's values; the properties of
//! the drawer page are set in every test in upstream's order.

use crate::test_base::{test_font_family, CompareOptions, TestBase};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, IBrush, SolidColorBrush};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::{
    Border, Control, Decorator, DrawerBehavior, DrawerLayoutBehavior, DrawerPage, DrawerPlacement, StackPanel,
    TextBlock,
};
use ferroui_themes_simple::SimpleTheme;
use std::rc::Rc;

const SKIP_IMMEDIATE: CompareOptions = CompareOptions { skip_immediate: true, skip_compositor: false };

fn base() -> TestBase {
    TestBase::new(r"Controls\DrawerPage")
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

/// The text block upstream's tests use as the drawer.
fn drawer_text(value: &str) -> Option<BoxedValue> {
    let block = TextBlock::new();
    block.set_text(Some(value));
    block.set_foreground(Some(Brushes::black()));
    block.set_font_family(test_font_family());
    block.set_margin(Thickness::uniform(16.0));
    block.set_vertical_alignment(VerticalAlignment::Top);
    Some(Control::boxed(block))
}

/// The text block upstream's tests use as the content.
fn detail_text() -> Option<BoxedValue> {
    let block = TextBlock::new();
    block.set_text(Some("Detail content"));
    block.set_foreground(Some(Brushes::black()));
    block.set_font_family(test_font_family());
    block.set_horizontal_alignment(HorizontalAlignment::Center);
    block.set_vertical_alignment(VerticalAlignment::Center);
    Some(Control::boxed(block))
}

/// The border upstream's tests of the rails without a header use as the content.
fn detail_border() -> Option<BoxedValue> {
    let border = Border::new();
    border.set_width(120.0);
    border.set_height(80.0);
    border.set_background(solid("#DCEEFB"));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    Some(Control::boxed(border))
}

/// The rail of a drawer at the left or at the right: three centered squares, one below the other.
fn vertical_rail(colors: [&str; 3]) -> Option<BoxedValue> {
    let panel = StackPanel::new();
    panel.set_margin(Thickness::symmetric(0.0, 4.0));
    for (color, margin) in colors.into_iter().zip([8.0, 4.0, 4.0]) {
        let square = Border::new();
        square.set_width(24.0);
        square.set_height(24.0);
        square.set_background(solid(color));
        square.set_horizontal_alignment(HorizontalAlignment::Center);
        square.set_margin(Thickness::symmetric(0.0, margin));
        panel.children().add(square);
    }
    Some(Control::boxed(panel))
}

/// The rail of a drawer at the top or at the bottom: three squares, one beside the other.
fn horizontal_rail(colors: [&str; 3]) -> Option<BoxedValue> {
    let panel = StackPanel::new();
    panel.set_orientation(Orientation::Horizontal);
    panel.set_margin(Thickness::symmetric(4.0, 0.0));
    for (color, margin) in colors.into_iter().zip([8.0, 4.0, 4.0]) {
        let square = Border::new();
        square.set_width(24.0);
        square.set_height(24.0);
        square.set_background(solid(color));
        square.set_margin(Thickness::symmetric(margin, 0.0));
        panel.children().add(square);
    }
    Some(Control::boxed(panel))
}

const INDIGO_RAIL: [&str; 3] = ["#3949AB", "#E53935", "#43A047"];
const GREEN_RAIL: [&str; 3] = ["#2E7D32", "#E53935", "#FB8C00"];

fn target_with(page: &Ref<DrawerPage>) -> Ref<Decorator> {
    let target = Decorator::new();
    target.set_width(500.0);
    target.set_height(350.0);
    target.set_child(page);
    target
}

/// The end of the tests that add the theme and the font style.
fn render_with_font_style(t: &TestBase, page: &Ref<DrawerPage>, test_name: &str) {
    let target = target_with(page);
    target.styles().add(SimpleTheme::new().as_style());
    target.styles().add(font_style());
    t.render_to_file(&target, test_name);
    t.compare_images_with(test_name, SKIP_IMMEDIATE);
}

/// The end of the tests that add the theme alone.
fn render_with_theme(t: &TestBase, page: &Ref<DrawerPage>, test_name: &str) {
    let target = target_with(page);
    target.styles().add(SimpleTheme::new().as_style());
    t.render_to_file(&target, test_name);
    t.compare_images_with(test_name, SKIP_IMMEDIATE);
}

#[test]
fn drawer_page_closed_shows_top_bar() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_header(text("My App"));
    page.set_drawer_length(200.0);
    page.set_drawer(drawer_text("Drawer content"));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_Closed_ShowsTopBar");
}

#[test]
fn drawer_page_open_shows_drawer_pane() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_header(text("My App"));
    page.set_drawer_length(200.0);
    page.set_is_open(true);
    page.set_drawer_background(solid("#F5F5F5"));
    page.set_drawer(drawer_text("Drawer content"));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_Open_ShowsDrawerPane");
}

#[test]
fn drawer_page_locked_no_top_bar() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_drawer_length(180.0);
    page.set_drawer_behavior(DrawerBehavior::Locked);
    page.set_drawer_background(solid("#E3F2FD"));
    page.set_drawer(drawer_text("Locked drawer"));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_Locked_NoTopBar");
}

#[test]
fn drawer_page_left_placement_compact_overlay_closed_shows_rail() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_header(text("My App"));
    page.set_drawer_length(200.0);
    page.set_drawer_layout_behavior(DrawerLayoutBehavior::CompactOverlay);
    page.set_compact_drawer_length(48.0);
    page.set_drawer_background(solid("#E8EAF6"));
    page.set_drawer(vertical_rail(INDIGO_RAIL));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_LeftPlacement_CompactOverlay_Closed_ShowsRail");
}

#[test]
fn drawer_page_left_placement_compact_overlay_open_pane_overlays_content() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_header(text("My App"));
    page.set_drawer_length(200.0);
    page.set_is_open(true);
    page.set_drawer_layout_behavior(DrawerLayoutBehavior::CompactOverlay);
    page.set_compact_drawer_length(48.0);
    page.set_drawer_background(solid("#E8EAF6"));
    page.set_drawer(drawer_text("Drawer content"));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_LeftPlacement_CompactOverlay_Open_PaneOverlaysContent");
}

#[test]
fn drawer_page_left_placement_compact_inline_closed_shows_rail() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_header(text("My App"));
    page.set_drawer_length(200.0);
    page.set_drawer_layout_behavior(DrawerLayoutBehavior::CompactInline);
    page.set_compact_drawer_length(48.0);
    page.set_drawer_background(solid("#E8F5E9"));
    page.set_drawer(vertical_rail(GREEN_RAIL));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_LeftPlacement_CompactInline_Closed_ShowsRail");
}

#[test]
fn drawer_page_left_placement_compact_inline_open_pane_pushes_content() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_header(text("My App"));
    page.set_drawer_length(200.0);
    page.set_is_open(true);
    page.set_drawer_layout_behavior(DrawerLayoutBehavior::CompactInline);
    page.set_compact_drawer_length(48.0);
    page.set_drawer_background(solid("#E8F5E9"));
    page.set_drawer(drawer_text("Drawer content"));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_LeftPlacement_CompactInline_Open_PanePushesContent");
}

#[test]
fn drawer_page_split_open_shows_both_panes() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_drawer_length(180.0);
    page.set_drawer_layout_behavior(DrawerLayoutBehavior::Split);
    page.set_is_open(true);
    page.set_drawer_background(solid("#FFF3E0"));
    page.set_drawer(drawer_text("Split drawer"));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_Split_Open_ShowsBothPanes");
}

#[test]
fn drawer_page_right_placement_open() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_header(text("My App"));
    page.set_drawer_length(200.0);
    page.set_is_open(true);
    page.set_drawer_placement(DrawerPlacement::Right);
    page.set_drawer_background(solid("#FCE4EC"));
    page.set_drawer(drawer_text("Right drawer"));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_RightPlacement_Open");
}

#[test]
fn drawer_page_top_placement_open() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_drawer_length(160.0);
    page.set_is_open(true);
    page.set_drawer_placement(DrawerPlacement::Top);
    page.set_drawer_background(solid("#E8EAF6"));
    page.set_drawer(drawer_text("Top drawer"));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_TopPlacement_Open");
}

#[test]
fn drawer_page_bottom_placement_open() {
    let t = base();
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_drawer_length(160.0);
    page.set_is_open(true);
    page.set_drawer_placement(DrawerPlacement::Bottom);
    page.set_drawer_background(solid("#FFF3E0"));
    page.set_drawer(drawer_text("Bottom drawer"));
    page.set_content(detail_text());

    render_with_font_style(&t, &page, "DrawerPage_BottomPlacement_Open");
}

/// The page of the tests of a closed compact drawer without a header.
fn compact_rail_page(
    layout_behavior: DrawerLayoutBehavior,
    placement: DrawerPlacement,
    drawer_background: &str,
    drawer: Option<BoxedValue>,
) -> Ref<DrawerPage> {
    let page = DrawerPage::new();
    page.set_background(Some(Brushes::white()));
    page.set_drawer_length(200.0);
    page.set_drawer_layout_behavior(layout_behavior);
    page.set_compact_drawer_length(48.0);
    page.set_drawer_placement(placement);
    page.set_drawer_background(solid(drawer_background));
    page.set_drawer(drawer);
    page.set_content(detail_border());
    page
}

#[test]
fn drawer_page_right_placement_compact_overlay_closed_shows_rail() {
    let t = base();
    let page = compact_rail_page(
        DrawerLayoutBehavior::CompactOverlay,
        DrawerPlacement::Right,
        "#E8EAF6",
        vertical_rail(INDIGO_RAIL),
    );

    render_with_theme(&t, &page, "DrawerPage_RightPlacement_CompactOverlay_Closed_ShowsRail");
}

#[test]
fn drawer_page_right_placement_compact_inline_closed_shows_rail() {
    let t = base();
    let page = compact_rail_page(
        DrawerLayoutBehavior::CompactInline,
        DrawerPlacement::Right,
        "#E8F5E9",
        vertical_rail(GREEN_RAIL),
    );

    render_with_theme(&t, &page, "DrawerPage_RightPlacement_CompactInline_Closed_ShowsRail");
}

#[test]
fn drawer_page_top_placement_compact_overlay_closed_shows_rail() {
    let t = base();
    let page = compact_rail_page(
        DrawerLayoutBehavior::CompactOverlay,
        DrawerPlacement::Top,
        "#E8EAF6",
        horizontal_rail(INDIGO_RAIL),
    );

    render_with_theme(&t, &page, "DrawerPage_TopPlacement_CompactOverlay_Closed_ShowsRail");
}

#[test]
fn drawer_page_top_placement_compact_inline_closed_shows_rail() {
    let t = base();
    let page = compact_rail_page(
        DrawerLayoutBehavior::CompactInline,
        DrawerPlacement::Top,
        "#E8F5E9",
        horizontal_rail(GREEN_RAIL),
    );

    render_with_theme(&t, &page, "DrawerPage_TopPlacement_CompactInline_Closed_ShowsRail");
}

#[test]
fn drawer_page_bottom_placement_compact_overlay_closed_shows_rail() {
    let t = base();
    let page = compact_rail_page(
        DrawerLayoutBehavior::CompactOverlay,
        DrawerPlacement::Bottom,
        "#E8EAF6",
        horizontal_rail(INDIGO_RAIL),
    );

    render_with_theme(&t, &page, "DrawerPage_BottomPlacement_CompactOverlay_Closed_ShowsRail");
}

#[test]
fn drawer_page_bottom_placement_compact_inline_closed_shows_rail() {
    let t = base();
    let page = compact_rail_page(
        DrawerLayoutBehavior::CompactInline,
        DrawerPlacement::Bottom,
        "#E8F5E9",
        horizontal_rail(GREEN_RAIL),
    );

    render_with_theme(&t, &page, "DrawerPage_BottomPlacement_CompactInline_Closed_ShowsRail");
}
