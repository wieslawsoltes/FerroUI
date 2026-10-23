//! Source examples and real native control wiring; no WinUI visual-parity claim.

use crate::media_samples::{self, FlipItem};
use crate::{examples, tests::start};
use ferroui_base::animation::SlideAxis;
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::imaging::{Bitmap, CroppedBitmap};
use ferroui_base::media::{DrawingGroup, DrawingImage, ImageDrawing, Stretch};
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::{PixelRect, PixelSize, Rect, Ref, Size, Vector};
use ferroui_controls::automation::peers::AutomationControlType;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Border, Button, Carousel, Control, Grid, Image, PipsPager, RadioButtons, StackPanel, TextBlock,
};
use std::rc::Rc;

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap()
}

fn key(carousel: &Carousel, key: Key) -> bool {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.set_routed_event(Some(InputElement::key_down_event()));
    carousel.raise_event(&args);
    args.handled()
}

#[test]
fn image_page_preserves_all_six_compositions_and_marks_unavailable_decoders() {
    let _application = start();
    let root = page("Image");
    for index in 1..=6 {
        assert!(root
            .find_control::<StackPanel>(&format!("ImageExample{index}"))
            .is_some());
    }
    assert_eq!(root.get_control::<Image>("BasicImage").height(), 100.0);
    assert_eq!(root.get_control::<Image>("DecodedImage").height(), 100.0);
    let stretch = root.get_control::<Image>("StretchImage");
    assert_eq!((stretch.width(), stretch.height()), (100.0, 100.0));
    assert_eq!(stretch.stretch(), Stretch::None);
    assert_eq!(root.get_control::<Image>("NineGridNormal").height(), 82.0);
    for name in ["NineGridSmallInsets", "NineGridLargeInsets"] {
        let image = root.get_control::<Image>(name);
        assert_eq!((image.width(), image.height()), (164.0, 164.0));
    }
    assert!(root.get_control::<Image>("SvgImage").source().is_none());
    let svg = root.get_control::<TextBlock>("SvgImageDiagnostic");
    assert!(svg.is_visible());
    assert!(svg.text().unwrap().contains("MirrorPCConsent.svg"));
    for name in ["GifAutoPreview", "GifPausedPreview", "GifManualPreview"] {
        assert_eq!(root.get_control::<Image>(name).height(), 40.0);
    }
    assert!(root
        .get_control::<TextBlock>("GifPlaybackDiagnostic")
        .text()
        .unwrap()
        .contains("first-frame"));
    assert!(!root.get_control::<Button>("PlayGif").is_enabled());
    assert!(!root.get_control::<Button>("StopGif").is_enabled());
    // The default test renderer deliberately cannot decode streams. The page
    // reports that failure instead of substituting a generated/sample image.
    let error = root.get_control::<TextBlock>("ImageLoadError");
    assert!(error.is_visible());
    assert!(error.text().unwrap().contains("treetops.jpg"));
    assert!(error.text().unwrap().contains("Decode to 100 pixels"));
}

#[test]
fn image_stretch_options_update_the_real_native_image() {
    let _application = start();
    let root = page("Image");
    let modes = root.get_control::<RadioButtons>("ImageStretchModes");
    let image = root.get_control::<Image>("StretchImage");
    for (index, stretch) in [
        (1, Stretch::Fill),
        (2, Stretch::Uniform),
        (3, Stretch::UniformToFill),
        (0, Stretch::None),
    ] {
        modes.set_selected_index(index);
        assert_eq!(image.stretch(), stretch);
    }
}

#[test]
fn native_nine_grid_drawings_retain_source_corners_and_stretch_the_center() {
    let _application = start();
    let bitmap = Rc::new(Bitmap::from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Opaque,
        &vec![255; 82 * 82 * 4],
        PixelSize::new(82, 82),
        Vector::new(96.0, 96.0),
        82 * 4,
    ));
    for (insets, expected_corner, expected_source_center, expected_target_center) in [
        (
            [3, 3, 3, 3],
            Rect::new(0.0, 0.0, 3.0, 3.0),
            PixelRect::new(3, 3, 76, 76),
            Rect::new(3.0, 3.0, 158.0, 158.0),
        ),
        (
            [30, 20, 30, 20],
            Rect::new(0.0, 0.0, 30.0, 20.0),
            PixelRect::new(30, 20, 22, 42),
            Rect::new(30.0, 20.0, 104.0, 124.0),
        ),
    ] {
        let source = media_samples::nine_grid_image(bitmap.clone(), insets);
        assert_eq!(source.size(), Size::new(164.0, 164.0));
        let drawing = source.as_any().downcast_ref::<DrawingImage>().unwrap();
        let group = drawing.drawing().unwrap().cast::<DrawingGroup>().unwrap();
        assert_eq!(group.children().len(), 9);
        let corner = group.children().get(0).cast::<ImageDrawing>().unwrap();
        assert_eq!(corner.rect(), expected_corner);
        let center = group.children().get(4).cast::<ImageDrawing>().unwrap();
        assert_eq!(center.rect(), expected_target_center);
        let crop_source = center.image_source().unwrap();
        let crop = crop_source
            .as_any()
            .downcast_ref::<CroppedBitmap>()
            .unwrap();
        assert_eq!(crop.source_rect(), expected_source_center);
        assert_eq!(
            crop.source().unwrap().as_bitmap().unwrap().pixel_size(),
            PixelSize::new(82, 82)
        );
    }
}

#[test]
fn image_decode_reports_missing_asset_errors() {
    let _application = start();
    assert!(
        media_samples::decode_to_height("ferres://WinUIGallery/Assets/missing-image.jpg", 100)
            .is_err()
    );
}

#[test]
fn flip_view_page_preserves_declared_images_data_order_and_orientation() {
    let _application = start();
    let root = page("FlipView");
    for (name, count, axis, orientation) in [
        (
            "DeclaredFlipView",
            5,
            SlideAxis::Horizontal,
            Orientation::Horizontal,
        ),
        (
            "BoundFlipView",
            15,
            SlideAxis::Horizontal,
            Orientation::Horizontal,
        ),
        (
            "VerticalFlipView",
            5,
            SlideAxis::Vertical,
            Orientation::Vertical,
        ),
    ] {
        let carousel = root.get_control::<Carousel>(name);
        let pager = root.get_control::<PipsPager>(&format!("{name}Pager"));
        assert_eq!(carousel.item_count(), count);
        assert_eq!(carousel.selected_index(), 0);
        assert!(!carousel.wrap_selection());
        assert!(carousel.is_swipe_enabled());
        assert_eq!(carousel.max_width(), 400.0);
        assert_eq!(
            carousel
                .page_transition()
                .unwrap()
                .as_page_slide()
                .unwrap()
                .orientation(),
            axis
        );
        assert_eq!(
            AutomationProperties::get_control_type_override(&carousel),
            Some(AutomationControlType::List)
        );
        assert_eq!(pager.number_of_pages(), count);
        assert_eq!(pager.selected_page_index(), 0);
        assert_eq!(pager.orientation(), orientation);
    }
    for name in ["DeclaredFlipView", "VerticalFlipView"] {
        let carousel = root.get_control::<Carousel>(name);
        assert_eq!(carousel.height(), 270.0);
        for (index, expected) in ["Cliff", "Grapes", "Rainier", "Sunset", "Valley"]
            .into_iter()
            .enumerate()
        {
            let image = Control::from_boxed(&carousel.items().get_at(index).unwrap())
                .unwrap()
                .cast::<Image>()
                .unwrap();
            assert_eq!(
                AutomationProperties::get_name(&image).as_deref(),
                Some(expected)
            );
            assert_eq!(image.stretch(), Stretch::Uniform);
        }
    }
    assert_eq!(
        root.get_control::<Carousel>("BoundFlipView").height(),
        180.0
    );
}

#[test]
fn flip_view_pager_and_carousel_selection_synchronize_both_directions_without_wrapping() {
    let _application = start();
    let root = page("FlipView");
    for name in ["DeclaredFlipView", "BoundFlipView", "VerticalFlipView"] {
        let carousel = root.get_control::<Carousel>(name);
        let pager = root.get_control::<PipsPager>(&format!("{name}Pager"));
        let output = root.get_control::<TextBlock>(&format!("{name}Selection"));
        let last = carousel.item_count() - 1;
        pager.set_selected_page_index(last);
        assert_eq!(carousel.selected_index(), last);
        assert_eq!(output.text(), Some(format!("{} of {}", last + 1, last + 1)));
        carousel.next();
        assert_eq!(carousel.selected_index(), last);
        carousel.previous();
        assert_eq!(pager.selected_page_index(), last - 1);
        assert_eq!(output.text(), Some(format!("{} of {}", last, last + 1)));
        carousel.set_selected_index(0);
        carousel.previous();
        assert_eq!(carousel.selected_index(), 0);
        assert_eq!(pager.selected_page_index(), 0);
    }
}

#[test]
fn flip_view_keyboard_navigation_follows_each_native_carousel_axis() {
    let _application = start();
    let root = page("FlipView");
    for (name, forward, backward, other_axis) in [
        ("DeclaredFlipView", Key::Right, Key::Left, Key::Down),
        ("VerticalFlipView", Key::Down, Key::Up, Key::Right),
    ] {
        let carousel = root.get_control::<Carousel>(name);
        let pager = root.get_control::<PipsPager>(&format!("{name}Pager"));
        assert!(key(&carousel, forward));
        assert_eq!(carousel.selected_index(), 1);
        assert_eq!(pager.selected_page_index(), 1);
        assert!(!key(&carousel, other_axis));
        assert_eq!(carousel.selected_index(), 1);
        assert!(key(&carousel, Key::End));
        assert_eq!(carousel.selected_index(), 4);
        assert!(key(&carousel, backward));
        assert_eq!(carousel.selected_index(), 3);
        assert!(key(&carousel, Key::Home));
        assert_eq!(pager.selected_page_index(), 0);
    }
}

#[test]
fn flip_view_bound_template_uses_original_first_three_groups_and_actual_page_layout() {
    let _application = start();
    let root = page("FlipView");
    let carousel = root.get_control::<Carousel>("BoundFlipView");
    let source = carousel.items_source().unwrap();
    let expected = [
        "XamlResources",
        "XamlStyles",
        "Binding",
        "Templates",
        "CustomUserControls",
        "CustomXamlConditionals",
        "ScratchPad",
        "Color",
        "Geometry",
        "Iconography",
        "Spacing",
        "Typography",
        "AccessibilityColorContrast",
        "AccessibilityKeyboard",
        "AccessibilityScreenReader",
    ];
    for (index, id) in expected.into_iter().enumerate() {
        let value = source.get_at(index).unwrap();
        assert_eq!(value.downcast_ref::<FlipItem>().unwrap().id, id);
    }
    let template = carousel.item_template().unwrap();
    for (index, expected_id, expected_title) in [
        (0, "XamlResources", "Resources"),
        (6, "ScratchPad", "Scratch Pad"),
        (14, "AccessibilityScreenReader", "Screen Reader"),
    ] {
        let child = template.build(&source.get_at(index)).unwrap();
        let grid = child.cast::<Grid>().unwrap();
        assert_eq!(
            AutomationProperties::get_automation_id(&grid).as_deref(),
            Some(expected_id)
        );
        assert_eq!(grid.row_definitions().count(), 2);
        assert!(grid.row_definitions().get(0).height().is_star());
        assert!(grid.row_definitions().get(1).height().is_auto());
        let image = grid
            .children()
            .to_vec()
            .into_iter()
            .find_map(|child| child.cast::<Image>())
            .unwrap();
        assert_eq!(image.width(), 36.0);
        assert_eq!(image.stretch(), Stretch::Uniform);
        assert_eq!(image.vertical_alignment(), VerticalAlignment::Center);
        let caption = grid
            .children()
            .to_vec()
            .into_iter()
            .find_map(|child| child.cast::<Border>())
            .unwrap();
        assert_eq!(caption.height(), 60.0);
        assert_eq!(Grid::get_row(&caption), 1);
        let text = caption.child().unwrap().cast::<TextBlock>().unwrap();
        assert_eq!(text.text().as_deref(), Some(expected_title));
    }
}

#[test]
fn media_page_callbacks_do_not_keep_the_page_and_controls_alive() {
    let _application = start();
    let root = page("FlipView");
    let weak_root = root.downgrade();
    let weak_carousel = root.get_control::<Carousel>("DeclaredFlipView").downgrade();
    let weak_pager = root
        .get_control::<PipsPager>("DeclaredFlipViewPager")
        .downgrade();
    drop(root);
    assert!(weak_root.upgrade().is_none());
    assert!(weak_carousel.upgrade().is_none());
    assert!(weak_pager.upgrade().is_none());
    let root = page("Image");
    let weak_root = root.downgrade();
    let weak_image = root.get_control::<Image>("StretchImage").downgrade();
    let weak_modes = root
        .get_control::<RadioButtons>("ImageStretchModes")
        .downgrade();
    drop(root);
    assert!(weak_root.upgrade().is_none());
    assert!(weak_image.upgrade().is_none());
    assert!(weak_modes.upgrade().is_none());
}
