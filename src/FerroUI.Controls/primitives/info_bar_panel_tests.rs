//! Tests of `InfoBarPanel` (WinUI; docs/porting/winui-controls.md, section 10.2): the layouts of
//! `InfoBarPanel.cpp` with the paddings and margins of the InfoBar template.

use super::InfoBarPanel;
use crate::testing::{TestServices, UnitTestApplication};
use crate::Border;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::{Rect, Ref, Size, Thickness};

fn child(width: f64, height: f64, horizontal: Thickness, vertical: Thickness) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    // Left-aligned, as the title and message text blocks of InfoBar's template: a fixed-width
    // child is otherwise centred in the rest of the width the panel gives the last child.
    border.set_horizontal_alignment(HorizontalAlignment::Left);
    InfoBarPanel::set_horizontal_orientation_margin(&border, horizontal);
    InfoBarPanel::set_vertical_orientation_margin(&border, vertical);
    border
}

/// The panel of InfoBar's template: title and message with the margins of
/// InfoBar_themeresources.xaml, the paddings of InfoBarPanelHorizontalOrientationPadding (0) and
/// InfoBarPanelVerticalOrientationPadding (0,14,0,18).
fn info_bar_panel(title: &Ref<Border>, message: &Ref<Border>) -> Ref<InfoBarPanel> {
    let panel = InfoBarPanel::new();
    panel.set_vertical_orientation_padding(Thickness::new(0.0, 14.0, 0.0, 18.0));
    panel.children().add(title.clone());
    panel.children().add(message.clone());
    panel
}

fn title() -> Ref<Border> {
    child(60.0, 20.0, Thickness::new(0.0, 14.0, 0.0, 0.0), Thickness::new(0.0, 14.0, 0.0, 0.0))
}

fn message(width: f64) -> Ref<Border> {
    child(width, 20.0, Thickness::new(12.0, 14.0, 0.0, 0.0), Thickness::new(0.0, 4.0, 0.0, 0.0))
}

#[test]
fn title_and_message_that_fit_are_on_one_line() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let (title, message) = (title(), message(100.0));
    let panel = info_bar_panel(&title, &message);

    panel.measure(Size::new(300.0, f64::INFINITY));
    assert_eq!(Size::new(172.0, 20.0), panel.desired_size());
    panel.arrange(Rect::new(0.0, 0.0, 300.0, 48.0));
    assert_eq!(Rect::new(0.0, 14.0, 60.0, 20.0), title.bounds());
    assert_eq!(Rect::new(72.0, 14.0, 100.0, 20.0), message.bounds());
}

#[test]
fn title_and_message_that_do_not_fit_are_below_each_other() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let (title, message) = (title(), message(250.0));
    let panel = info_bar_panel(&title, &message);

    panel.measure(Size::new(300.0, f64::INFINITY));
    assert_eq!(Size::new(250.0, 76.0), panel.desired_size());
    panel.arrange(Rect::new(0.0, 0.0, 300.0, 76.0));
    assert_eq!(Rect::new(0.0, 14.0, 60.0, 20.0), title.bounds());
    assert_eq!(Rect::new(0.0, 38.0, 250.0, 20.0), message.bounds());
}

#[test]
fn a_collapsed_child_is_left_out() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let (title, message) = (title(), message(100.0));
    title.set_is_visible(false);
    let panel = info_bar_panel(&title, &message);

    // One item left: the layout below each other, with the vertical padding.
    panel.measure(Size::new(300.0, f64::INFINITY));
    assert_eq!(Size::new(100.0, 52.0), panel.desired_size());
    panel.arrange(Rect::new(0.0, 0.0, 300.0, 52.0));
    assert_eq!(Rect::new(0.0, 14.0, 100.0, 20.0), message.bounds());
}

#[test]
fn a_change_of_an_attached_margin_measures_the_panel_again() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let (title, message) = (title(), message(100.0));
    let panel = info_bar_panel(&title, &message);

    panel.measure(Size::new(150.0, f64::INFINITY));
    panel.arrange(Rect::new(0.0, 0.0, 150.0, panel.desired_size().height));
    assert_eq!(38.0, message.bounds().y);
    InfoBarPanel::set_vertical_orientation_margin(&message, Thickness::new(0.0, 10.0, 0.0, 0.0));
    assert!(!panel.is_measure_valid());
    panel.measure(Size::new(150.0, f64::INFINITY));
    panel.arrange(Rect::new(0.0, 0.0, 150.0, panel.desired_size().height));
    assert_eq!(44.0, message.bounds().y);
}
