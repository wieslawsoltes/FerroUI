//! The placement, the properties and the markup of the icons.

use crate::placement::{image_rect, measure};
use crate::{register_types, BitmapIcon, ImageIcon};
use ferroui_base::{Rect, Size, StaticType};
use ferroui_controls::testing::{TestServices, UnitTestApplication};

#[test]
fn an_image_is_scaled_uniformly_and_centered() {
    let rect = image_rect(Size::new(100.0, 40.0), Size::new(20.0, 20.0)).expect("a rectangle");
    assert_eq!(Rect::new(30.0, 0.0, 40.0, 40.0), rect);
}

#[test]
fn nothing_is_drawn_for_an_empty_icon_or_image() {
    assert!(image_rect(Size::new(0.0, 40.0), Size::new(20.0, 20.0)).is_none());
    assert!(image_rect(Size::new(40.0, 40.0), Size::new(0.0, 20.0)).is_none());
}

#[test]
fn an_unconstrained_icon_takes_the_size_of_its_image() {
    let size = measure(Size::new(f64::INFINITY, f64::INFINITY), Size::new(32.0, 16.0));
    assert_eq!(Size::new(32.0, 16.0), size);
}

#[test]
fn a_bitmap_icon_is_monochrome_by_default() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let icon = BitmapIcon::new();
    assert!(icon.show_as_monochrome());
    assert!(icon.source().is_none());

    icon.set_show_as_monochrome(false);
    assert!(!icon.show_as_monochrome());
}

#[test]
fn an_icon_without_an_image_takes_no_space() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let bitmap = BitmapIcon::new();
    let image = ImageIcon::new();
    bitmap.measure(Size::new(50.0, 50.0));
    image.measure(Size::new(50.0, 50.0));

    assert_eq!(Size::default(), bitmap.desired_size());
    assert_eq!(Size::default(), image.desired_size());
}

#[test]
fn the_icons_are_registered_for_markup() {
    register_types();
    assert!(<BitmapIcon as StaticType>::TYPE.name().ends_with("BitmapIcon"));
    assert!(<ImageIcon as StaticType>::TYPE.name().ends_with("ImageIcon"));
}
