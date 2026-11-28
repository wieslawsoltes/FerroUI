use super::{CadPage, cad_action_tests::mount};
use ferroribbon::controls::RibbonScreenTipService;
use ferroui_base::{
    PixelPoint, Point, Visual,
    animation::SignedDuration,
    media::TranslateTransform,
    threading::{CancellationToken, Dispatcher},
};
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    HeadlessWindowExtensions,
};

fn native_popups() -> AppBuilder {
    native_options(false, true)
}
fn logical_popups() -> AppBuilder {
    native_options(true, true)
}
fn relative_popups() -> AppBuilder {
    native_options(true, false)
}
fn native_options(logical: bool, positionable: bool) -> AppBuilder {
    use ferroui_desktop::AppBuilderDesktopExtensions;
    AppBuilder::configure::<Application>()
        .use_platform_detect()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            overlay_popups: false,
            use_logical_desktop_coordinates: logical,
            supports_window_positioning: positionable,
            ..Default::default()
        })
}

fn run(builder: fn() -> AppBuilder, action: impl FnOnce() + Send + 'static) {
    let session = HeadlessUnitTestSession::start_new(builder);
    let result = session
        .dispatch(
            move || {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                action();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

fn scale_matrix(builder: fn() -> AppBuilder, positionable: bool) {
    run(builder, move || {
        let page = CadPage::new();
        let window = mount(&page);
        window.set_position(PixelPoint::new(125, -71));
        if !positionable {
            assert_eq!(window.position(), PixelPoint::default());
        }
        let mut cases = 0;
        let mut saw_quantization = false;
        for target_scale in [1., 1.25, 1.5, 2.] {
            window.set_render_scaling(target_scale);
            for source_scale in [1., 1.25, 1.5, 2.] {
                page.draw_panel().open_slide_out();
                page.draw_panel().set_is_slide_out_pinned(true);
                Dispatcher::ui_thread().run_jobs(None);
                let item = page
                    .cad_ribbon()
                    .find_item(Some("multiplePoints".into()))
                    .unwrap();
                let popup_root = ferroui_controls::TopLevel::get_top_level(Some(&item)).unwrap();
                popup_root.set_render_scaling(source_scale);
                let transform = TranslateTransform::new();
                transform.set_x(0.25);
                transform.set_y(0.75);
                item.set_render_transform(Some(transform.into()));
                Dispatcher::ui_thread().run_jobs(None);
                assert!(item.is_attached_to_visual_tree());
                let point = Point::new(0., item.bounds().height + 4.);
                assert!(item.translate_point(point, &window).is_none());
                let source = item.presentation_source().unwrap();
                let source_root = source.root_visual().unwrap();
                let expected_desktop = item
                    .translate_point(point, &source_root)
                    .unwrap()
                    .transform(source.client_to_screen_transform().unwrap());
                assert!(page.show_tool_tip(Some("multiplePoints")).unwrap());
                let tip = page.captured_tip_popup().unwrap();
                assert!(tip.is_open());
                assert!(!tip.is_using_overlay_layer());
                let requested = Point::new(tip.horizontal_offset(), tip.vertical_offset());
                let target = Visual::presentation_source(&window).unwrap();
                let target_root = target.root_visual().unwrap();
                let actual_desktop = window
                    .translate_point(requested, &target_root)
                    .unwrap()
                    .transform(target.client_to_screen_transform().unwrap());
                assert!(
                    (actual_desktop.x - expected_desktop.x).abs() < 1e-10
                        && (actual_desktop.y - expected_desktop.y).abs() < 1e-10,
                    "{source_scale}->{target_scale}: {actual_desktop:?} != {expected_desktop:?}"
                );
                let quantized = window.point_to_client(item.point_to_screen(point));
                saw_quantization |= (requested.x - quantized.x).abs() > 1e-6
                    || (requested.y - quantized.y).abs() > 1e-6;
                cases += 1;
                tip.set_is_open(false);
                page.draw_panel().close_slide_out();
            }
        }
        assert_eq!(cases, 16);
        assert!(
            saw_quantization,
            "Fractional cases must distinguish the continuous bridge from PixelPoint conversion"
        );
        window.close();
    });
}

#[test]
fn native_popup_physical_desktop_preserves_fractional_offsets_across_scales() {
    scale_matrix(native_popups, true);
}
#[test]
fn native_popup_logical_desktop_uses_desktop_matrices_independently_of_render_scale() {
    scale_matrix(logical_popups, true);
}
#[test]
fn native_popup_parent_relative_desktop_works_without_global_window_positioning() {
    scale_matrix(relative_popups, false);
}

#[test]
fn expanded_line_tip_keeps_its_root_offset_near_the_screen_edge() {
    run(native_popups, || {
        let page = CadPage::new();
        let window = mount(&page);
        window.set_position(PixelPoint::new(20, 1_020));
        let previous_delay = RibbonScreenTipService::extended_delay();
        RibbonScreenTipService::set_extended_delay(SignedDuration::ZERO);
        let shown = page.show_tool_tip(Some("line"));
        Dispatcher::ui_thread().run_jobs(None);
        RibbonScreenTipService::set_extended_delay(previous_delay);
        assert!(shown.unwrap());
        let popup = page.captured_tip_popup().unwrap();
        assert!(!popup.is_using_overlay_layer());
        let tip = popup.child().unwrap();
        let requested = window.point_to_screen(Point::new(
            popup.horizontal_offset(),
            popup.vertical_offset(),
        ));
        let screen_bottom = window.screens().primary().unwrap().working_area().bottom();
        assert!(f64::from(requested.y) + tip.bounds().height > f64::from(screen_bottom));
        assert_eq!(tip.point_to_screen(Point::default()), requested);
        window.close();
    });
}

#[test]
fn actual_floating_and_collapsed_panels_keep_command_identity_when_capturing_help() {
    run(native_popups, || {
        let page = CadPage::new();
        let window = mount(&page);
        let layer = page
            .cad_ribbon()
            .find_item(Some("layerProperties".into()))
            .unwrap();
        page.float_layers_panel();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(page.layers_panel().is_floating());
        assert!(
            layer.ptr_eq(
                &page
                    .cad_ribbon()
                    .find_item(Some("layerProperties".into()))
                    .unwrap()
            )
        );
        assert!(layer.is_attached_to_visual_tree());
        assert!(layer.translate_point(Point::default(), &window).is_none());
        assert!(page.show_tool_tip(Some("layerProperties")).unwrap());
        page.captured_tip_popup().unwrap().set_is_open(false);
        page.layers_panel().return_to_ribbon();
        Dispatcher::ui_thread().run_jobs(None);
        let line = page.cad_ribbon().find_item(Some("line".into())).unwrap();
        window.set_width(260.);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            page.draw_panel().state(),
            ferroribbon_core::RibbonGroupState::Collapsed,
            "The real ribbon layout must collapse Draw before its popup is tested"
        );
        page.draw_panel().open_popup();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(page.draw_panel().is_popup_open());
        assert!(line.is_attached_to_visual_tree());
        assert!(line.translate_point(Point::default(), &window).is_none());
        assert!(page.show_tool_tip(Some("line")).unwrap());
        assert!(line.ptr_eq(&page.cad_ribbon().find_item(Some("line".into())).unwrap()));
        page.captured_tip_popup().unwrap().set_is_open(false);
        page.draw_panel().close_popup();
        window.close();
    });
}

#[test]
fn real_native_slide_out_root_can_position_the_original_screen_tip() {
    let session = HeadlessUnitTestSession::start_new(native_popups);
    let result = session
        .dispatch(
            || {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                let page = CadPage::new();
                let window = mount(&page);
                window.set_position(PixelPoint::new(123, 57));
                page.draw_panel().open_slide_out();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(page.draw_panel().is_slide_out_open());
                let item = page
                    .cad_ribbon()
                    .find_item(Some("multiplePoints".into()))
                    .unwrap();
                assert!(item.is_attached_to_visual_tree());
                let point = Point::new(0., item.bounds().height + 4.);
                assert!(
                    item.translate_point(point, &window).is_none(),
                    "The reproducer must use distinct visual roots"
                );
                assert_eq!(page.show_tool_tip(Some("multiplePoints")), Ok(true));
                assert!(page.captured_tip_popup().unwrap().is_open());
                page.draw_panel().close_slide_out();
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
fn retained_native_popup_and_command_do_not_root_the_removed_page() {
    run(native_popups, || {
        let page = CadPage::new();
        let weak_page = page.downgrade();
        let weak_model = std::rc::Rc::downgrade(&page.view_model());
        let window = mount(&page);
        page.draw_panel().open_slide_out();
        Dispatcher::ui_thread().run_jobs(None);
        let command = page
            .cad_ribbon()
            .find_item(Some("multiplePoints".into()))
            .unwrap();
        assert!(command.is_attached_to_visual_tree());
        assert!(page.show_tool_tip(Some("multiplePoints")).unwrap());
        let popup = page.captured_tip_popup().unwrap();
        assert!(popup.is_open() && !popup.is_using_overlay_layer());
        window.set_content(None);
        drop(page);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak_page.upgrade().is_none());
        assert!(weak_model.upgrade().is_none());
        assert!(!popup.is_open());
        assert!(!command.is_attached_to_visual_tree());
        window.close();
    });
}
