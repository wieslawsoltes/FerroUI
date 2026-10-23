//! Exercises the original page handlers against native Frame and animation services.

use crate::connected_animation_samples::{
    content, item_control, ConnectedCardPage, ConnectedDetailPage, ConnectedFirstPage,
    ConnectedListPage, ConnectedRepeaterPage, ConnectedSecondPage,
};
use crate::{examples, tests::start};
use ferroui_base::input::{
    InputElement, Key, KeyEventArgs, KeyboardNavigation, KeyboardNavigationMode,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Ref, Vector};
use ferroui_controls::animation::ConnectedAnimationService;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::navigation::NavigationCacheMode;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::testing::{
    create_split_view_list_box_theme, CompositorTestServices, MockWindowingPlatform, TestServices,
    UnitTestApplication,
};
use ferroui_controls::{
    Border, Button, Control, Frame, Grid, Image, Panel, RadioButtons, ScrollViewer, TextBlock,
    ToolTip, Window,
};
use std::time::{Duration, Instant};

fn page() -> Ref<Control> {
    examples::create(examples::find("ConnectedAnimation").unwrap()).unwrap()
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

fn item_count(root: &Control) -> usize {
    root.get_logical_descendants()
        .filter_map(|child| child.cast::<Button>())
        .filter(|button| {
            button
                .name()
                .is_some_and(|name| name.starts_with("ConnectedItem"))
        })
        .count()
}

#[test]
fn connected_animation_initializes_all_four_source_compositions_and_original_item_counts() {
    let _application = start();
    let root = page();
    for (name, expected_type, count) in [
        ("ConnectedCollectionFrame", ConnectedListPage::TYPE, 8),
        ("ConnectedCardFrame", ConnectedCardPage::TYPE, 13),
        ("ConnectedRepeaterFrame", ConnectedRepeaterPage::TYPE, 13),
    ] {
        let frame = root.get_control::<Frame>(name);
        assert_eq!(frame.current_source_page_type(), Some(expected_type));
        assert_eq!(frame.back_stack_depth(), 0);
        let form = content(&frame).unwrap();
        assert_eq!(item_count(&form), count);
        // The ordinary test renderer rejects stream decoding. The composition
        // must disclose that failure while its native controls remain usable.
        let errors = form.get_control::<TextBlock>(if name == "ConnectedCardFrame" {
            "CardImageErrors"
        } else {
            "CollectionImageErrors"
        });
        assert!(errors.is_visible());
        let message = errors.text().unwrap();
        assert!(message.contains("the mock render interface does not decode bitmaps"));
        assert_eq!(message.lines().count(), count);
        for index in 1..=count {
            let image = item_control::<Image>(&form, &format!("ConnectedImage{index}")).unwrap();
            assert!(image.source().is_none());
            assert!(message.contains(&format!("LandscapeImage{index}.jpg")));
            assert_eq!(
                AutomationProperties::get_name(&image).as_deref(),
                Some(format!("Item {index}").as_str())
            );
        }
    }
    assert_eq!(
        root.get_control::<Frame>("ConnectedContentFrame")
            .current_source_page_type(),
        Some(ConnectedFirstPage::TYPE)
    );
    assert_eq!(
        root.get_control::<RadioButtons>("ConfigurationPanel")
            .selected_index(),
        0
    );
}

#[test]
fn connected_collection_passes_selected_data_and_returns_to_the_cached_source_page() {
    let _application = start();
    let root = page();
    let frame = root.get_control::<Frame>("ConnectedCollectionFrame");
    let original_page = frame.current_page().unwrap();
    assert_eq!(
        original_page.navigation_cache_mode(),
        NavigationCacheMode::Enabled
    );
    let list = content(&frame).unwrap();
    let third = item_control::<Button>(&list, "ConnectedItem3").unwrap();
    click(&third);
    assert_eq!(
        frame.current_source_page_type(),
        Some(ConnectedDetailPage::TYPE)
    );
    assert_eq!(frame.back_stack_depth(), 1);
    let detail = content(&frame).unwrap();
    assert_eq!(
        detail
            .get_control::<TextBlock>("DetailTitle")
            .text()
            .as_deref(),
        Some("Item 3")
    );
    assert_eq!(
        AutomationProperties::get_name(&detail.get_control::<Image>("DetailedImage")).as_deref(),
        Some("Item 3")
    );
    assert!(detail
        .get_control::<Image>("DetailedImage")
        .source()
        .is_none());
    let image_error = detail.get_control::<TextBlock>("DetailImageError");
    assert!(image_error.is_visible());
    assert!(image_error.text().unwrap().contains("LandscapeImage3.jpg"));
    assert!(detail
        .get_control::<TextBlock>("DetailDescription")
        .text()
        .unwrap()
        .starts_with("Quisque accumsan"));
    let views = detail
        .get_control::<TextBlock>("DetailViews")
        .text()
        .unwrap();
    let views: u32 = views.strip_prefix("Views: ").unwrap().parse().unwrap();
    assert!((100..999).contains(&views));
    let likes = detail
        .get_control::<TextBlock>("DetailLikes")
        .text()
        .unwrap();
    let likes: u32 = likes.strip_prefix("Likes: ").unwrap().parse().unwrap();
    assert!((10..99).contains(&likes));
    click(&detail.get_control::<Button>("GoBackButton"));
    assert_eq!(frame.current_page(), Some(original_page));
    assert_eq!(frame.back_stack_depth(), 0);
    assert_eq!(
        root.get_control::<TextBlock>("ConnectedCollectionStatus")
            .text()
            .as_deref(),
        Some("Returned to Item 3")
    );
}

#[test]
fn connected_collection_description_tooltip_tracks_real_trimming_and_releases_its_control() {
    let _application = start();
    let weak_description = {
        let root = page();
        let collection = content(&root.get_control::<Frame>("ConnectedCollectionFrame")).unwrap();
        let item = item_control::<Button>(&collection, "ConnectedItem5").unwrap();
        let description = item
            .get_logical_descendants()
            .filter_map(|child| child.cast::<TextBlock>())
            .find(|text| {
                text.text()
                    .is_some_and(|value| value.starts_with("Ut consequat magna"))
            })
            .expect("the original fifth item's description");
        let full_text = description.text().unwrap();
        assert!(ToolTip::get_tip(&description).is_none());

        // Resize the actual wired sample description in a bounded native host;
        // its real text layout determines each collapsed/fitting transition.
        let parent = description
            .get_logical_parent()
            .unwrap()
            .cast::<Panel>()
            .unwrap();
        parent.children().remove(&description);
        let viewport = Border::new();
        viewport.set_width(100.0);
        viewport.set_height(80.0);
        viewport.set_child(Some(description.clone().upcast()));
        let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(
            640.0, 160.0,
        ));
        window.set_content(Some(Control::boxed(viewport.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);

        for (width, expected_collapsed) in
            [(100.0, true), (500.0, false), (100.0, true), (500.0, false)]
        {
            viewport.set_width(width);
            window.update_layout();
            let layout = description.text_layout();
            assert!(!layout.text_lines().is_empty());
            assert!(layout.text_lines().len() <= 2);
            let collapsed = layout.text_lines().iter().any(|line| line.has_collapsed());
            assert_eq!(
                collapsed, expected_collapsed,
                "actual text layout at width {width}"
            );
            let tooltip = ToolTip::get_tip(&description)
                .map(|value| value.downcast_ref::<String>().unwrap().clone());
            assert_eq!(
                tooltip.as_deref(),
                expected_collapsed.then_some(full_text.as_str()),
                "the full description is available only while its rendered lines are collapsed",
            );
        }
        let weak = description.downgrade();
        window.close();
        weak
    };
    Dispatcher::ui_thread().run_jobs(None);
    assert!(
        weak_description.upgrade().is_none(),
        "the layout callback must not retain the description"
    );
}

#[test]
fn connected_manual_items_accept_keyboard_activation_and_restore_the_saved_scroll_offset() {
    ferroui_markup_xaml::register_types();
    let focus = TestServices::real_focus();
    // The base window test theme supplies no ScrollViewer template; it would
    // otherwise inherit an ordinary ContentPresenter with no scroll extent.
    // Use the existing native test theme that supplies ScrollContentPresenter
    // and scroll bars, so this exercises actual measured scrolling.
    let mut services = TestServices::styled_window().with_theme(create_split_view_list_box_theme);
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    let _application = UnitTestApplication::start(services);
    let root = page();
    let frame = root.get_control::<Frame>("ConnectedRepeaterFrame");
    let parent = frame.get_logical_parent().unwrap().cast::<Panel>().unwrap();
    parent.children().remove(&frame);
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(
        640.0, 400.0,
    ));
    window.set_content(Some(Control::boxed(frame.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
    let cached = frame.current_page().unwrap();
    let form = content(&frame).unwrap();
    let scroll = form.get_control::<ScrollViewer>("CollectionScroll");
    assert!(scroll.presenter().unwrap().is::<ScrollContentPresenter>());
    assert!(scroll.viewport().height > 0.0);
    assert!(
        scroll.extent().height > scroll.viewport().height,
        "the measured item extent {:?} must exceed its viewport {:?}",
        scroll.extent(),
        scroll.viewport(),
    );
    assert_eq!(
        KeyboardNavigation::get_tab_navigation(&scroll),
        KeyboardNavigationMode::Once
    );
    scroll.set_offset(Vector::new(0.0, 120.0));
    window.update_layout();
    let saved = scroll.offset();
    assert!(
        saved.y > 0.0,
        "the real scroll presenter has a nonzero offset"
    );
    let item = item_control::<Button>(&form, "ConnectedItem7").unwrap();
    assert!(item.focus());
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    item.raise_event(&enter);
    assert!(enter.handled());
    let detail = content(&frame).unwrap();
    assert_eq!(
        detail
            .get_control::<TextBlock>("DetailTitle")
            .text()
            .as_deref(),
        Some("Item 7")
    );
    assert!(detail
        .get_control::<TextBlock>("DetailDescription")
        .text()
        .unwrap()
        .starts_with("Aenean vulputate"));
    click(&detail.get_control::<Button>("GoBackButton"));
    assert_eq!(frame.current_page(), Some(cached));
    assert_eq!(scroll.offset(), saved);
    assert!(
        item.is_focused(),
        "back navigation returns focus to the activated native button"
    );
    assert_eq!(
        root.get_control::<TextBlock>("ConnectedRepeaterStatus")
            .text()
            .as_deref(),
        Some("Returned to Item 7")
    );
    let mut space_down = KeyEventArgs::new();
    space_down.key = Key::Space;
    space_down.set_routed_event(Some(InputElement::key_down_event()));
    item.raise_event(&space_down);
    assert!(space_down.handled());
    let mut space_up = KeyEventArgs::new();
    space_up.key = Key::Space;
    space_up.set_routed_event(Some(InputElement::key_up_event()));
    item.raise_event(&space_up);
    assert!(space_up.handled());
    assert_eq!(
        frame.current_source_page_type(),
        Some(ConnectedDetailPage::TYPE)
    );
    window.close();
}

#[test]
fn connected_card_uses_selected_item_details_and_restores_its_overlay_after_return() {
    let _application = start();
    let root = page();
    let frame = root.get_control::<Frame>("ConnectedCardFrame");
    let form = content(&frame).unwrap();
    let smoke = form.get_control::<Grid>("SmokeGrid");
    let destination = form.get_control::<Border>("DestinationElement");
    assert!(!smoke.is_visible());
    click(&item_control::<Button>(&form, "ConnectedItem10").unwrap());
    assert!(smoke.is_visible());
    assert_eq!(
        form.get_control::<TextBlock>("CardTitle").text().as_deref(),
        Some("Item 10")
    );
    assert!(form
        .get_control::<TextBlock>("CardDescription")
        .text()
        .unwrap()
        .starts_with("Nullam eget"));
    assert!(form.get_control::<Image>("DetailImage").source().is_none());
    let error = form.get_control::<TextBlock>("DetailImageError");
    assert!(error.is_visible());
    assert!(error.text().unwrap().contains("LandscapeImage10.jpg"));
    click(&form.get_control::<Button>("CloseCard"));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!smoke.is_visible());
    assert!(smoke.children().contains(&destination.upcast()));
    assert_eq!(
        frame.current_source_page_type(),
        Some(ConnectedCardPage::TYPE)
    );
    assert_eq!(frame.back_stack_depth(), 0);
}

#[test]
fn connected_configurations_alternate_original_destinations_and_preserve_frame_history() {
    let _application = start();
    let root = page();
    let frame = root.get_control::<Frame>("ConnectedContentFrame");
    let options = root.get_control::<RadioButtons>("ConfigurationPanel");
    for index in 0..4 {
        options.set_selected_index(index);
        click(&root.get_control::<Button>("ConnectedNavigate"));
        let forward = index % 2 == 0;
        assert_eq!(
            frame.current_source_page_type(),
            Some(if forward {
                ConnectedSecondPage::TYPE
            } else {
                ConnectedFirstPage::TYPE
            })
        );
        let current = content(&frame).unwrap();
        assert!(current
            .find_control::<Grid>(if forward {
                "DestinationElement"
            } else {
                "SourceElement"
            })
            .is_some());
        assert_eq!(frame.back_stack_depth(), index + 1);
    }
}

#[test]
fn connected_configuration_handler_starts_a_real_compositor_flight_and_releases_it() {
    ferroui_markup_xaml::register_types();
    let services = CompositorTestServices::start(TestServices::styled_window());
    let root = page();
    let frame = root.get_control::<Frame>("ConnectedContentFrame");
    // Host this original composition by itself, as the capture host can do, so
    // its source is visible rather than below the other three examples.
    let parent = frame.get_logical_parent().unwrap().cast::<Panel>().unwrap();
    parent.children().remove(&frame);
    let platform = MockWindowingPlatform::create_window_mock_with_size(640.0, 500.0);
    services.setup(&platform);
    let window = Window::with_impl(platform);
    window.set_content(Some(Control::boxed(frame.clone())));
    window.show();
    services.run_jobs();
    services.run_jobs();
    let source = content(&frame)
        .unwrap()
        .get_control::<Grid>("SourceElement");
    assert!(source.bounds().width > 0.0 && source.bounds().height > 0.0);
    root.get_control::<RadioButtons>("ConfigurationPanel")
        .set_selected_index(1);
    click(&root.get_control::<Button>("ConnectedNavigate"));
    let destination = content(&frame)
        .unwrap()
        .get_control::<Grid>("DestinationElement");
    assert!(destination.is_hidden_by_composition());
    services.run_jobs();
    let layer = window
        .composition_layer_visual()
        .expect("the native service created an animation layer");
    assert_eq!(layer.children().count(), 1);
    assert_eq!(
        layer.children().get(0).children().count(),
        3,
        "Gravity uses a shadow plus source and destination images"
    );
    let started = Instant::now();
    while destination.is_hidden_by_composition() && started.elapsed() < Duration::from_secs(2) {
        services.run_jobs();
    }
    assert!(!destination.is_hidden_by_composition());
    assert_eq!(layer.children().count(), 0);
    assert!(ConnectedAnimationService::get_for_top_level(&window)
        .get_animation("ForwardConnectedAnimation")
        .is_none());
    window.close();
}

#[test]
fn connected_gallery_can_be_released_after_navigation_without_retaining_its_root() {
    let _application = start();
    let weak = {
        let root = page();
        click(&root.get_control::<Button>("ConnectedNavigate"));
        Dispatcher::ui_thread().run_jobs(None);
        root.downgrade()
    };
    Dispatcher::ui_thread().run_jobs(None);
    assert!(weak.upgrade().is_none());
}

/// Run with `--features capture`: the existing Skia renderer decodes the actual
/// embedded JPEG streams. Mock-renderer logical tests above intentionally do
/// not stand in for successful image loading or pixel rendering.
#[cfg(feature = "capture")]
#[test]
fn connected_capture_decodes_original_landscapes_and_propagates_the_selected_bitmap() {
    use ferroui_base::media::imaging::Bitmap;
    use ferroui_base::{PixelRect, PixelSize};
    use std::rc::Rc;

    fn pixels(image: &Image, expected: PixelSize) -> Vec<u8> {
        let source = image.source().expect("the pinned JPEG must decode");
        let bitmap = source.as_any().downcast_ref::<Bitmap>().unwrap();
        assert_eq!(bitmap.pixel_size(), expected);
        let bits = bitmap
            .format()
            .expect("Skia decoded readable pixels")
            .bits_per_pixel();
        assert_eq!(bits % 8, 0);
        let bytes_per_pixel = usize::try_from(bits / 8).unwrap();
        let stride = expected.width * i32::try_from(bytes_per_pixel).unwrap();
        let mut pixels = vec![0; (stride * expected.height) as usize];
        bitmap.copy_pixels(PixelRect::default(), &mut pixels, stride);
        let first = &pixels[..bytes_per_pixel];
        assert!(
            pixels
                .chunks_exact(bytes_per_pixel)
                .any(|pixel| pixel != first),
            "the original photograph contains varying decoded pixels"
        );
        pixels
    }

    ferroui_markup_xaml::register_types();
    let _application =
        UnitTestApplication::start(TestServices::styled_window().with_render_interface(Rc::new(
            ferroui_skia::PlatformRenderInterface::new(None, None),
        )));
    let root = page();
    // Natural dimensions independently read from the pinned source JPEGs.
    let dimensions = [
        (400, 268),
        (400, 268),
        (400, 268),
        (400, 268),
        (400, 268),
        (400, 268),
        (400, 268),
        (400, 268),
        (266, 266),
        (1024, 400),
        (133, 266),
        (265, 265),
        (400, 130),
    ];
    for (name, count, error_name) in [
        ("ConnectedCollectionFrame", 8, "CollectionImageErrors"),
        ("ConnectedCardFrame", 13, "CardImageErrors"),
        ("ConnectedRepeaterFrame", 13, "CollectionImageErrors"),
    ] {
        let form = content(&root.get_control::<Frame>(name)).unwrap();
        assert!(!form.get_control::<TextBlock>(error_name).is_visible());
        for (index, &(width, height)) in dimensions.iter().take(count).enumerate() {
            let image =
                item_control::<Image>(&form, &format!("ConnectedImage{}", index + 1)).unwrap();
            pixels(&image, PixelSize::new(width, height));
        }
    }

    let frame = root.get_control::<Frame>("ConnectedCollectionFrame");
    let collection = content(&frame).unwrap();
    let source = item_control::<Image>(&collection, "ConnectedImage3").unwrap();
    let source_pixels = pixels(&source, PixelSize::new(400, 268));
    click(&item_control::<Button>(&collection, "ConnectedItem3").unwrap());
    let detail = content(&frame).unwrap();
    assert_eq!(
        pixels(
            &detail.get_control::<Image>("DetailedImage"),
            PixelSize::new(400, 268)
        ),
        source_pixels,
    );
    assert!(!detail
        .get_control::<TextBlock>("DetailImageError")
        .is_visible());

    let card = content(&root.get_control::<Frame>("ConnectedCardFrame")).unwrap();
    let original = item_control::<Image>(&card, "ConnectedImage10")
        .unwrap()
        .source()
        .unwrap();
    click(&item_control::<Button>(&card, "ConnectedItem10").unwrap());
    let selected = card.get_control::<Image>("DetailImage").source().unwrap();
    assert!(
        Rc::ptr_eq(&original, &selected),
        "the card shares its already-decoded original image"
    );
    assert!(!card
        .get_control::<TextBlock>("DetailImageError")
        .is_visible());
}
