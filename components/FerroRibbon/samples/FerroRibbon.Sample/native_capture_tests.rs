use super::*;
use std::path::PathBuf;

fn test_capture_directory() -> PathBuf {
    let directory = std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY")
        .or_else(|| std::env::var_os("FERRORIBBON_TEMP"))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroribbon-captures"));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn gallery_preview_local_clip_contains_real_glyph_pixels_under_application_styles() {
    use ferroribbon::controls::{RibbonGallery, RibbonGalleryItem};
    use ferroui_base::{
        Rect,
        media::{Color, FlowDirection, SolidColorBrush, TextTrimming},
        styling::{Selectors, Setter, Style},
        threading::CancellationToken,
    };
    use ferroui_controls::{AppBuilder, Application, TextBlock};
    use ferroui_desktop::AppBuilderDesktopExtensions;
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    };
    let session = HeadlessUnitTestSession::start_new(|| {
        let symbols = ferroui_base::media::FontFamily::new(
            "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
        );
        AppBuilder::configure::<Application>()
            .with(Rc::new(ferroui_base::media::FontManagerOptions {
                font_family_mappings: Some(std::collections::HashMap::from([
                    ("Segoe Fluent Icons".into(), symbols.clone()),
                    ("Segoe MDL2 Assets".into(), symbols),
                ])),
                ..Default::default()
            }))
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
    });
    let directory = test_capture_directory();
    session.dispatch(move || {
        crate::register_types();
        let app = Application::current().unwrap();
        app.styles().add(ferroui_themes_simple::SimpleTheme::new().as_style());
        crate::install_application_resources();
        app.styles().add(Style::with_setters(Selectors::of_type::<TextBlock>(), [
            Setter::new_untyped(TextBlock::text_trimming_property().as_property(), Rc::new(<dyn TextTrimming>::word_ellipsis())),
            Setter::new(ferroui_base::Visual::clip_to_bounds_property(), false),
        ]));
        let gallery = RibbonGallery::new();
        gallery.set_min_columns(1);
        gallery.set_max_columns(1);
        gallery.set_show_item_labels(false);
        gallery.set_item_height(52.);
        let item = RibbonGalleryItem::new();
        item.set_preview_font_size(26.);
        item.set_preview_foreground(Some(SolidColorBrush::with_color(Color::from_argb(255, 240, 0, 0)).into()));
        let window = Window::new();
        window.set_width(300.);
        window.set_height(130.);
        window.set_use_layout_rounding(false);
        window.set_content(Some(Control::boxed(gallery.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let mut results = Vec::new();
        let mut unclipped_outside = 0;
        for text in ["MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM", "A\u{301}😀 A\u{301}😀 A\u{301}😀 A\u{301}😀 A\u{301}😀", "العربية אבג العربية אבג العربية אבג"] {
            for direction in [FlowDirection::LeftToRight, FlowDirection::RightToLeft] {
                for width in [37.25, 71.5] {
                    gallery.set_flow_direction(direction);
                    gallery.set_item_width(width);
                    item.set_preview_text(Some(text.into()));
                    // The source snapshots preview properties when the item
                    // collection rebuilds; changing the item alone is not a refresh.
                    gallery.items().clear();
                    gallery.items().add_native(Some(Rc::new(item.clone())));
                    window.update_layout();
                    Dispatcher::ui_thread().run_jobs(None);
                    window.update_layout();
                    let preview = gallery.get_visual_descendants().filter_map(|v| v.cast::<TextBlock>())
                        .find(|v| v.text().as_deref() == Some(text)).expect("actual Gallery preview text");
                    assert_eq!(preview.text_trimming().to_string(), "None");
                    assert!(preview.clip_to_bounds());
                    assert!(preview.text_layout().text_lines().iter().all(|line| !line.has_collapsed()));
                    assert!(preview.text_layout().width_including_trailing_whitespace() > preview.bounds().width);
                    let bounds = Rect::from_size(preview.bounds().size())
                        .transform_to_aabb(preview.transform_to_visual(&window).unwrap());
                    for scale in [1., 2.] {
                        let count = |frame: &CaptureFrame| {
                            let mut ink = 0;
                            let mut outside = 0;
                            for (i, pixel) in frame.pixels.as_chunks::<4>().0.iter().enumerate() {
                                // The capture service returns premultiplied BGRA.
                                if pixel[2] > 180 && pixel[1] < 70 && pixel[0] < 70 && pixel[3] > 0 {
                                    ink += 1;
                                    let x = (i % frame.size.width as usize) as f64;
                                    let y = (i / frame.size.width as usize) as f64;
                                    // Permit the one device-pixel antialias fringe.
                                    if x < (bounds.x * scale).floor() - 1. || x > ((bounds.x + bounds.width) * scale).ceil()
                                        || y < (bounds.y * scale).floor() - 1. || y > ((bounds.y + bounds.height) * scale).ceil() {
                                        outside += 1;
                                    }
                                }
                            }
                            (ink, outside)
                        };
                        let clipped = render_node(&window.clone().upcast(), scale).unwrap();
                        let (ink, outside) = count(&clipped);
                        assert!(ink > 0, "glyph pixels must actually be rendered");
                        assert_eq!(outside, 0, "clip: {text:?}, {direction:?}, width={width}, scale={scale}");
                        // A deliberate control proves the pixel assertion detects
                        // real overflow instead of relying on ancestor clipping.
                        preview.set_clip_to_bounds(false);
                        let open = render_node(&window.clone().upcast(), scale).unwrap();
                        let (_, leaked) = count(&open);
                        unclipped_outside += leaked;
                        preview.set_clip_to_bounds(true);
                        if results.is_empty() {
                            for (name, frame) in [("clipped", &clipped), ("unclipped-control", &open)] {
                                let bitmap = Bitmap::from_pixels(PixelFormat::BGRA8888, AlphaFormat::Premul,
                                    &frame.pixels, frame.size, Vector::new(96. * scale, 96. * scale), frame.size.width * 4);
                                let mut png = Vec::new();
                                bitmap.save(&mut png, &BitmapEncoderOptions::Png(
                                    ferroui_base::media::imaging::PngBitmapEncoderOptions::default(),
                                )).unwrap();
                                bitmap.dispose();
                                std::fs::write(directory.join(format!("gallery-preview-{name}.png")), png).unwrap();
                            }
                        }
                        results.push(serde_json::json!({"text":text,"direction":format!("{direction:?}"),"width":width,"scale":scale,"ink":ink,"outside":outside,"unclippedOutside":leaked}));
                    }
                }
            }
        }
        assert_eq!(results.len(), 24);
        assert!(unclipped_outside > 0, "the deliberately unclipped control must leak ink");
        std::fs::write(directory.join("gallery-preview-render-cases.json"), serde_json::to_vec_pretty(&results).unwrap()).unwrap();
        window.close();
        window.set_content(None);
    }, CancellationToken::none()).wait().unwrap();
    session.dispose();
}

#[test]
fn native_keytip_pixels_and_qat_positions_use_real_word_controls() {
    check_qat_keytip_pixels_with_normal_work(false);
}

#[test]
fn qat_keytip_pixels_wait_for_rendering_after_normal_priority_work() {
    check_qat_keytip_pixels_with_normal_work(true);
}

fn check_qat_keytip_pixels_with_normal_work(stress_dispatcher: bool) {
    use ferroui_base::{Rect, Size, threading::CancellationToken};
    use ferroui_controls::{AppBuilder, Application, Canvas};
    use ferroui_desktop::AppBuilderDesktopExtensions;
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    };
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>()
            .with(Rc::new(ferroui_base::media::FontManagerOptions {
                font_family_mappings: Some(std::collections::HashMap::from([
                    (
                        "Segoe Fluent Icons".into(),
                        ferroui_base::media::FontFamily::new(
                            "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
                        ),
                    ),
                    (
                        "Segoe MDL2 Assets".into(),
                        ferroui_base::media::FontFamily::new(
                            "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
                        ),
                    ),
                ])),
                ..Default::default()
            }))
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
    });
    let directory = test_capture_directory();
    let result = session
        .dispatch_task(
            move || async move {
        crate::register_types();
        Application::current().unwrap().styles().add(
            ferroui_themes_simple::SimpleTheme::new().as_style());
        crate::install_application_resources();
        Application::current().unwrap().resources().set(
            "SymbolThemeFontFamily",
            Some(Rc::new(ferroui_base::media::FontFamily::new("ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols"))),
        );
        let settings = crate::demo_settings::DemoSettings::new(Rc::new(|| Ok("03:04:05".into())));
        let shell = ShellPage::new_with_settings(settings);
        let window = Window::new();
        window.set_width(1200.);
        window.set_height(820.);
        window.set_content(Some(Control::boxed(shell.clone())));
        window.show();
        // Loaded creates the source Word page. Drain the native dispatcher,
        // as the other shell mounts do; elapsed time is not a Loaded barrier.
        Dispatcher::ui_thread().run_jobs(None);
        window.measure(Size::new(1200.,820.));
        window.arrange(Rect::new(0.,0.,1200.,820.));
        wait_for(Duration::from_millis(100)).await;
        let surface = NativeCaptureSurface::new(window.clone(),shell);
        let ribbon = surface.find_ribbon().unwrap().unwrap();
        let qat = ribbon.quick_access_tool_bar().unwrap();
        assert!(qat.items().count() > 0, "real Word QAT entries required");
        let mut diagnostics = Vec::new();
        for (theme_name,theme) in [("light",ThemeVariant::light()),("dark",ThemeVariant::dark())] {
            surface.shell.set_value(ThemeVariant::requested_theme_variant_property(),Some(theme));
            for (position_name,position) in [("above",RibbonQuickAccessPosition::AboveRibbon),("below",RibbonQuickAccessPosition::BelowRibbon)] {
                let previous_count = diagnostics.len();
                ribbon.set_quick_access_position(position);
                window.update_layout();
                wait_for(Duration::from_millis(30)).await;
                if stress_dispatcher {
                    Dispatcher::ui_thread().post_local(
                        || std::thread::sleep(Duration::from_millis(60)),
                        DispatcherPriority::NORMAL,
                    );
                }
                ribbon.show_key_tips();
                wait_for(Duration::from_millis(30)).await;
                eprintln!("[qat-priority] normal timer: {theme_name}/{position_name}; stress={stress_dispatcher}; tips={}; badges={}; qat={:?}",
                    ribbon.current_key_tips().count(), ribbon.active_key_tips().count(),
                    qat.items().iter().flatten().map(|item| item.bounds()).collect::<Vec<_>>());
                // Elapsed time at NORMAL can precede the queued LOADED badge
                // render. Await the native operation behind that render, rather
                // than assuming a timer establishes dispatcher ordering.
                Dispatcher::ui_thread().invoke_async_local_with_priority(
                    || (), DispatcherPriority::LOADED).await.unwrap();
                eprintln!("[qat-priority] loaded barrier: {theme_name}/{position_name}; stress={stress_dispatcher}; tips={}; badges={}",
                    ribbon.current_key_tips().count(), ribbon.active_key_tips().count());
                let root = surface.content().unwrap().unwrap();
                let background = render_node(&root,window.render_scaling()).unwrap();
                let frame = capture_current(&window).unwrap();
                assert_ne!(frame.pixels,background.pixels,"actual KeyTip pixels {theme_name}/{position_name}");
                for badge in ribbon.active_key_tips().iter().flatten() {
                    assert!(badge.bounds().width > 0. && badge.bounds().height > 0.);
                    if let Some(target) = badge.target().filter(|target|qat.items().contains(&Some(target.clone()))) {
                        let bounds = Rect::from_size(target.bounds().size()).transform_to_aabb(target.transform_to_visual(&window).unwrap());
                        let size = badge.desired_size();
                        let x = (bounds.x+(bounds.width-size.width)/2.).clamp(0.,(window.bounds().width-size.width).max(0.));
                        let y = (bounds.bottom()-size.height/2.-2.).clamp(0.,(window.bounds().height-size.height).max(0.));
                        assert_eq!((Canvas::get_left(&badge),Canvas::get_top(&badge)),(x,y));
                        diagnostics.push(format!("{theme_name}/{position_name}: {} at {x},{y} size {size:?}",badge.text().unwrap()));
                    }
                }
                assert_eq!(diagnostics.len() - previous_count, qat.items().iter().flatten().count(),
                    "Each surface must show all actual QAT badges: {theme_name}/{position_name}; \
                     badges={}; qat={:?}",
                    ribbon.active_key_tips().count(),
                    qat.items().iter().flatten().map(|item|
                        (item.bounds(), item.is_visible(), item.get_visual_root().is_some())
                    ).collect::<Vec<_>>());
                let prefix = if stress_dispatcher { "qat-keytips-normal-work" } else { "qat-keytips" };
                let path = directory.join(format!("{prefix}-{theme_name}-{position_name}.png"));
                surface.save(path.to_str().unwrap()).await.unwrap();
                ribbon.cancel_key_tips();
                wait_for(Duration::from_millis(30)).await;
                assert!(ribbon.active_key_tips().count()==0);
            }
        }
        assert_eq!(diagnostics.len(), 4 * qat.items().iter().flatten().count(),
            "Every theme/placement surface must show all actual QAT badges");
        let diagnostics_path = if stress_dispatcher {
            "qat-keytip-normal-work-native-pixels.txt"
        } else {
            "qat-keytip-native-pixels.txt"
        };
        std::fs::write(directory.join(diagnostics_path),diagnostics.join("\n")).unwrap();
        window.close();
        window.set_content(None);
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
fn native_capture_timer_drop_cancels_wakeup_and_io_reports_failure() {
    use ferroui_base::threading::CancellationToken;
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    };
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        task::{Context, Wake},
    };
    struct CountWake(AtomicUsize);
    impl Wake for CountWake {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    });
    let result = session
        .dispatch_task(
            || async {
                let counter = Arc::new(CountWake(AtomicUsize::new(0)));
                let waker = Waker::from(counter.clone());
                let mut pending = Box::pin(wait_for(Duration::from_millis(20)));
                assert!(
                    pending
                        .as_mut()
                        .poll(&mut Context::from_waker(&waker))
                        .is_pending()
                );
                assert_eq!(
                    Arc::strong_count(&counter),
                    3,
                    "Future must retain the pending task waker"
                );
                drop(pending);
                assert_eq!(
                    Arc::strong_count(&counter),
                    2,
                    "Cancellation must release the pending waker"
                );
                wait_for(Duration::from_millis(80)).await;
                assert_eq!(
                    counter.0.load(Ordering::SeqCst),
                    0,
                    "Cancelled timer must never wake its task"
                );
                let directory = test_capture_directory();
                let failure = write_file(directory.into_os_string().into_string().unwrap(), vec![1, 2, 3]).await;
                assert!(
                    failure.is_err(),
                    "A directory cannot be reported as a successfully saved capture"
                );
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
fn native_capture_request_uses_dispatcher_timers_real_bgra_png_and_async_io() {
    use ferroui_base::threading::CancellationToken;
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_desktop::AppBuilderDesktopExtensions;
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    };
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>()
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
    });
    let result = session
        .dispatch_task(
            || async {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                Application::current().unwrap().resources().set(
                    "SymbolThemeFontFamily",
                    Some(Rc::new(ferroui_base::media::FontFamily::new(
                        "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
                    ))),
                );
                let settings =
                    crate::demo_settings::DemoSettings::new(Rc::new(|| Ok("03:04:05".into())));
                let shell = ShellPage::new_with_settings(settings);
                let window = Window::new();
                window.set_width(1200.);
                window.set_height(820.);
                window.set_content(Some(Control::boxed(shell.clone())));
                window.show();
                let path = std::env::temp_dir().join(format!(
                    "ferroribbon-capture-numbers-{}.png",
                    std::process::id()
                ));
                let capture_path = path.to_str().expect("capture path").to_owned();
                let surface = NativeCaptureSurface::with_environment(
                    window.clone(),
                    shell.clone(),
                    Rc::new(move |key| {
                        Ok(match key {
                            "RIBBONSPACE_CAPTURE" => Some(capture_path.clone()),
                            "RIBBONSPACE_PAGE" => Some("excel".into()),
                            "RIBBONSPACE_ACTION" => Some("paste".into()),
                            "RIBBONSPACE_WIDTH" => Some("up1180".into()),
                            "RIBBONSPACE_HEIGHT" => Some("up800".into()),
                            _ => None,
                        })
                    }),
                );
                let started = std::time::Instant::now();
                let request = CaptureRequest::read(&surface).unwrap().unwrap();
                use ferroui_base::utilities::{CultureInfo, NumberFormatInfo};
                struct Restore(CultureInfo);
                impl Drop for Restore {
                    fn drop(&mut self) {
                        CultureInfo::set_current_formatting(self.0.clone());
                    }
                }
                let _restore = Restore(CultureInfo::current_formatting());
                let mut format = NumberFormatInfo::new();
                format.set_positive_sign("up");
                format.set_negative_sign("down");
                format.set_number_negative_pattern(2);
                CultureInfo::set_current_formatting(
                    CultureInfo::invariant_culture().with_number_format(format),
                );
                request.run(&surface).await.unwrap();
                assert_eq!(window.width(), 1180.);
                assert_eq!(window.height(), 800.);
                assert!(started.elapsed() >= Duration::from_millis(3800));
                let bytes = std::fs::read(&path).unwrap();
                std::fs::remove_file(path).unwrap();
                assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
                assert_eq!(shell.current_page().as_deref(), Some("excel"));
                let paste = surface
                    .find_ribbon()
                    .unwrap()
                    .unwrap()
                    .find_item(Some("paste".into()))
                    .unwrap();
                assert!(
                    paste
                        .cast::<RibbonSplitButton>()
                        .unwrap()
                        .is_drop_down_open()
                );
                let popups = opened_popups(&window);
                assert!(
                    !popups.is_empty(),
                    "Paste must have an actual registered popup"
                );
                for popup in &popups {
                    let child = popup.child().expect("popup child");
                    eprintln!(
                        "capture popup: {} attached={} bounds={:?} host={} sequence={:?}",
                        child.get_type().name(),
                        child.is_attached_to_visual_tree(),
                        child.bounds(),
                        popup.host().is_some(),
                        popup.opening_sequence()
                    );
                    assert!(
                        child.is_attached_to_visual_tree(),
                        "Open-state flags must correspond to displayed popup content"
                    );
                    assert!(child.bounds().width > 0. && child.bounds().height > 0.);
                }
                let frame = capture_current(&window).unwrap();
                let root = surface.content().unwrap().unwrap();
                let background = render_node(&root, window.render_scaling()).unwrap();
                assert_ne!(
                    frame.pixels, background.pixels,
                    "Displayed Paste popup must change the captured pixels"
                );
                assert_eq!(
                    frame.pixels.len(),
                    frame.size.width as usize * frame.size.height as usize * 4
                );
                assert!(frame.pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 255));
                let decoded = Bitmap::from_stream(&mut std::io::Cursor::new(bytes)).unwrap();
                assert_eq!(decoded.pixel_size(), frame.size);
                let buffer = WriteableBitmap::new(
                    frame.size,
                    Vector::new(96., 96.),
                    Some(PixelFormat::BGRA8888),
                    Some(AlphaFormat::Premul),
                );
                let locked = buffer.lock();
                decoded.copy_pixels_to_framebuffer(locked.as_ref());
                locked.dispose();
                let mut decoded_pixels = vec![0; frame.pixels.len()];
                buffer.copy_pixels(
                    ferroui_base::PixelRect::from_size(frame.size),
                    &mut decoded_pixels,
                    frame.size.width * 4,
                );
                decoded.dispose();
                buffer.dispose();
                assert_eq!(
                    decoded_pixels, frame.pixels,
                    "Saved PNG must contain the actual composed popup pixels"
                );
                window.close();
                window.set_content(None);
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
#[test]
fn native_capture_popup_order_interleaved_roots_and_child() {
    use ferroui_base::threading::CancellationToken;
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_desktop::AppBuilderDesktopExtensions;
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    };
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>()
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
    });
    let result = session
        .dispatch_task(
            || async {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                use ferroui_controls::{
                    Border, Button, PlacementMode, StackPanel, primitives::Popup,
                };
                let window = Window::new();
                window.set_width(600.);
                window.set_height(400.);
                let panel = StackPanel::new();
                let anchor = Button::new();
                anchor.set_content(Some(Rc::new("anchor".to_owned())));
                panel.children().add(anchor.clone());
                window.set_content(Some(Control::boxed(panel.clone())));
                window.show();
                window.measure(ferroui_base::Size::new(600., 400.));
                window.arrange(ferroui_base::Rect::new(0., 0., 600., 400.));
                window.update_layout();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(anchor.is_attached_to_visual_tree());
                let first = Popup::new();
                first.set_placement_target(Some(anchor.clone().upcast()));
                first.set_placement(PlacementMode::Bottom);
                first.set_is_light_dismiss_enabled(false);
                let child_anchor = Button::new();
                child_anchor.set_content(Some(Rc::new("child anchor".to_owned())));
                first.set_child(Some(child_anchor.clone().upcast()));
                let second = Popup::new();
                second.set_placement_target(Some(anchor.upcast()));
                second.set_placement(PlacementMode::Right);
                second.set_is_light_dismiss_enabled(false);
                let body = Border::new();
                body.set_width(120.);
                body.set_height(60.);
                second.set_child(Some(body.upcast()));
                let child = Popup::new();
                child.set_placement_target(Some(child_anchor.upcast()));
                child.set_placement(PlacementMode::Bottom);
                child.set_is_light_dismiss_enabled(false);
                let body = Border::new();
                body.set_width(80.);
                body.set_height(40.);
                child.set_child(Some(body.upcast()));
                panel.children().add(first.clone());
                panel.children().add(second.clone());
                panel.children().add(child.clone());
                first.set_is_open(true);
                wait_for(Duration::from_millis(100)).await;
                Dispatcher::ui_thread().run_jobs(None);
                let popup_host = first.host().unwrap();
                let host = popup_host
                    .hosted_visual_tree_root()
                    .and_then(|r| r.cast::<Control>())
                    .or_else(|| {
                        popup_host
                            .as_overlay_popup_host()
                            .map(|r| r.upcast::<Control>())
                    })
                    .unwrap();
                host.apply_template();
                host.measure(ferroui_base::Size::new(200., 100.));
                host.arrange(ferroui_base::Rect::new(0., 0., 200., 100.));
                host.update_layout();
                Dispatcher::ui_thread().run_jobs(None);
                window
                    .renderer()
                    .paint(ferroui_base::Rect::from_size(window.client_size()));
                let target = child.placement_target().unwrap();
                assert!(
                    target.is_attached_to_visual_tree(),
                    "Nested anchor must have a visual root"
                );
                second.set_is_open(true);
                wait_for(Duration::from_millis(50)).await;
                Dispatcher::ui_thread().run_jobs(None);
                child.set_is_open(true);
                wait_for(Duration::from_millis(50)).await;
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(window.opened_popups().len(), 2);
                assert!(child.is_open());
                assert!(
                    child.host().is_some(),
                    "The child must actually be displayed, not merely request IsOpen"
                );
                let actual = opened_popups(&window);
                let expected = vec![child.clone(), second.clone(), first.clone()];
                let names = |values: Vec<Ref<Popup>>| {
                    values
                        .into_iter()
                        .map(|p| {
                            if p == first {
                                "first"
                            } else if p == second {
                                "second"
                            } else {
                                "child"
                            }
                        })
                        .collect::<Vec<_>>()
                };
                let observed = names(actual);
                let expected = names(expected);
                child.set_is_open(false);
                second.set_is_open(false);
                first.set_is_open(false);
                window.close();
                assert_eq!(
                    observed, expected,
                    "Pinned Uno GetOpenPopups returns most recently opened first"
                );
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
