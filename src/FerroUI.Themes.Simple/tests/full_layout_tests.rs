//! Port of `Layout/FullLayoutTests.cs` (base unit tests). The tests show
//! windows with the services of a styled window, whose theme is the Simple
//! theme, so they live with the theme.

use super::support::start_themed_application;
use ferroui_base::layout::{HorizontalAlignment, ILayoutManager, Orientation, VerticalAlignment};
use ferroui_base::{Point, Size, Visual};
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::{ScrollBar, ScrollBarVisibility};
use ferroui_controls::{Border, Control, ScrollViewer, SizeToContent, TextBlock, Window};
use std::rc::Rc;

#[test]
fn grandchild_size_changed() {
    let _app = start_themed_application();

    let text_block = TextBlock::new();
    text_block.set_width(400.0);
    text_block.set_height(400.0);
    text_block.set_text(Some("Hello World!"));
    let inner = Border::new();
    inner.set_child(&text_block);
    let border = Border::new();
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    border.set_child(&inner);

    let window = Window::new();
    window.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
    window.set_content(Some(Control::boxed(&border)));

    window.show();

    assert_eq!(Size::new(400.0, 400.0), border.bounds().size());
    text_block.set_width(200.0);
    window.layout_manager().execute_layout_pass();

    assert_eq!(Size::new(200.0, 400.0), border.bounds().size());
}

#[test]
fn test_scroll_viewer_with_text_block() {
    let _app = start_themed_application();

    let text_block = TextBlock::new();
    text_block.set_width(400.0);
    text_block.set_height(400.0);
    text_block.set_text(Some("Hello World!"));
    let scroll_viewer = ScrollViewer::new();
    scroll_viewer.set_width(200.0);
    scroll_viewer.set_height(200.0);
    scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
    scroll_viewer.set_horizontal_alignment(HorizontalAlignment::Center);
    scroll_viewer.set_vertical_alignment(VerticalAlignment::Center);
    scroll_viewer.set_content(Some(Control::boxed(&text_block)));

    let window = Window::new();
    window.set_width(800.0);
    window.set_height(600.0);
    window.set_content(Some(Control::boxed(&scroll_viewer)));

    window.resources().set("ScrollBarThickness", Some(Rc::new(10.0_f64)));

    window.show();

    assert_eq!(Size::new(800.0, 600.0), window.bounds().size());
    assert_eq!(Size::new(200.0, 200.0), scroll_viewer.bounds().size());
    assert_eq!(Point::new(300.0, 200.0), position(&scroll_viewer));
    assert_eq!(Size::new(400.0, 400.0), text_block.bounds().size());

    let scroll_bars: Vec<_> =
        scroll_viewer.get_template_descendants().into_iter().filter_map(|x| x.cast::<ScrollBar>()).collect();
    let presenters: Vec<_> = scroll_viewer
        .get_template_descendants()
        .into_iter()
        .filter_map(|x| x.cast::<ScrollContentPresenter>())
        .collect();

    assert_eq!(2, scroll_bars.len());
    assert_eq!(1, presenters.len());

    let presenter = &presenters[0];
    assert_eq!(Size::new(190.0, 190.0), presenter.bounds().size());

    let single = |orientation: Orientation| {
        let matching: Vec<_> = scroll_bars.iter().filter(|x| x.orientation() == orientation).collect();
        assert_eq!(1, matching.len());
        matching[0].clone()
    };
    let horz_scroll = single(Orientation::Horizontal);
    let vert_scroll = single(Orientation::Vertical);

    assert!(horz_scroll.is_visible());
    assert!(vert_scroll.is_visible());
    assert_eq!(Size::new(190.0, 10.0), horz_scroll.bounds().size());
    assert_eq!(Size::new(10.0, 190.0), vert_scroll.bounds().size());
    assert_eq!(Point::new(0.0, 190.0), position(&horz_scroll));
    assert_eq!(Point::new(190.0, 0.0), position(&vert_scroll));
}

fn position(v: &Visual) -> Point {
    v.bounds().position()
}
