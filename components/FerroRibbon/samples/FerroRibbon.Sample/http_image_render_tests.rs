use super::build;
use ferroribbon::{
    compat::{BitmapIcon, ImageIcon},
    controls::RibbonItemHelper,
    primitives::{RibbonDeferredImage, RibbonIconPresenter, RibbonImageLoadError},
};
use ferroui_base::{
    PixelRect, PixelSize, Rect, Size, Vector,
    media::{
        IImage, ImageDrawing,
        imaging::{Bitmap, RenderTargetBitmap},
    },
    platform::{PixelFormat, register_assets},
    threading::{CancellationToken, Dispatcher},
    utilities::Uri,
};
use ferroui_controls::{Control, Grid, Image, StackPanel, Window};
use ferroui_headless::{HeadlessUnitTestSession, HeadlessWindowExtensions};
use std::{
    cell::RefCell,
    io::{self, BufRead, BufReader, Cursor, Read, Write},
    net::{TcpListener, TcpStream},
    rc::Rc,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const PNG: &[u8] = include_bytes!("../../tests/fixtures/images/bitmap-3x2.png");
const SVG: &[u8] =
    br#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="2" viewBox="0 0 4 2">
  <path fill="red" d="M0 0h2v2H0z"/><path fill="blue" d="M2 0h2v2H2z"/>
  <path fill="white" d="M.2 0h.1v2H.2z"/>
</svg>"#;
const TIMEOUT: Duration = Duration::from_secs(10);

#[test]
#[ignore = "Requires a loopback HTTP listener and the real Skia image decoder"]
fn built_in_http_icons_decode_redirect_report_errors_and_cancel() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (pending, accepted) = mpsc::channel();
    let server = thread::spawn(move || {
        for expected in [
            "/redirect.png?x=1",
            "/pixels.png",
            "/menu.svg",
            "/remote.svg",
            "/missing.png",
            "/large.png",
            "/cancel.png",
        ] {
            let mut socket = accept(&listener);
            let mut input = BufReader::new(&mut socket);
            let mut line = String::new();
            input.read_line(&mut line).unwrap();
            assert_eq!(line.split_whitespace().nth(1), Some(expected));
            loop {
                line.clear();
                assert!(input.read_line(&mut line).unwrap() > 0);
                if line == "\r\n" {
                    break;
                }
            }
            match expected {
                "/redirect.png?x=1" => socket.write_all(b"HTTP/1.1 302 Found\r\nLocation: /pixels.png\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap(),
                "/missing.png" => socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap(),
                "/large.png" => socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20000000\r\nConnection: close\r\n\r\n").unwrap(),
                "/cancel.png" => {
                    pending.send(()).unwrap();
                    assert_eq!(socket.read(&mut [0]).unwrap(), 0, "dropping the icon closes the pending request");
                }
                _ => {
                    let (kind, data) = if expected == "/remote.svg" { ("image/svg+xml", SVG) } else { ("image/png", PNG) };
                    write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", data.len()).unwrap();
                    socket.write_all(data).unwrap();
                }
            }
        }
    });

    let session = HeadlessUnitTestSession::start_new(build);
    let result = session.dispatch(move || {
        ferroribbon::register_types();
        let presenter = RibbonIconPresenter::new();
        presenter.set_icon(Some(Rc::new(format!("{base}/redirect.png?x=1#ignored"))));
        let image = presenter.children().get(0).cast::<Image>().unwrap();
        let source = image.source().unwrap();
        assert!(deferred(&source).load_result().is_none());
        assert_png(&source);
        assert!(Rc::ptr_eq(&image.source().unwrap(), &source));

        let menu = RibbonItemHelper::create_menu_icon(Some(Rc::new(format!("{base}/menu.svg"))))
            .unwrap().unwrap().cast::<ImageIcon>().unwrap();
        assert_png(&menu.source().unwrap());

        presenter.set_icon(Some(Rc::new(format!("{base}/remote.svg"))));
        let svg = presenter.children().get(0).cast::<Image>().unwrap().source().unwrap();
        wait_until(|| deferred(&svg).load_result().is_some());
        deferred(&svg).load_result().unwrap().unwrap();
        assert_eq!(svg.size(), Size::new(4., 2.));

        let bitmap = BitmapIcon::new();
        bitmap.set_uri_source(Some(Uri::absolute(&format!("{base}/missing.png")).unwrap()));
        wait_until(|| bitmap.load_error().is_some());
        assert!(matches!(bitmap.load_error().as_deref(), Some(RibbonImageLoadError::Load(error)) if error.to_string().contains("404")));
        bitmap.set_uri_source(Some(Uri::absolute(&format!("{base}/large.png")).unwrap()));
        wait_until(|| bitmap.load_error().is_some());
        assert!(matches!(bitmap.load_error().as_deref(), Some(RibbonImageLoadError::Load(error)) if error.kind() == io::ErrorKind::InvalidData));

        // Retire the source after the server starts waiting for the response.
        presenter.set_icon(Some(Rc::new(format!("{base}/cancel.png"))));
        wait_until(|| accepted.try_recv().is_ok());
        presenter.set_icon(None);
        Dispatcher::ui_thread().run_jobs(None);
    }, CancellationToken::none()).wait();
    session.dispose();
    result.unwrap();
    server.join().unwrap();
}

#[test]
#[ignore = "Requires the real Skia renderer for SVG pixels at multiple scales"]
fn built_in_svg_icons_render_asset_pixels_at_layout_scale() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session.dispatch(|| {
        ferroribbon::register_types();
        register_assets("ferroribbon_svg_test", &[("icon.svg", SVG), ("bad.svg", b"invalid SVG")]);
        let presenter = RibbonIconPresenter::new();
        presenter.set_icon_size(20.);
        presenter.set_icon(Some(Rc::new("ms-appx://ferroribbon_svg_test/icon.%73vg?not=.png".to_owned())));
        let image = presenter.children().get(0).cast::<Image>().unwrap();
        let source = image.source().unwrap();
        assert!(deferred(&source).load_result().is_none());

        let window = Window::new();
        window.set_content(Some(Control::boxed(presenter.clone())));
        window.show();
        wait_until(|| deferred(&source).load_result().is_some());
        deferred(&source).load_result().unwrap().unwrap();
        assert_eq!(source.size(), Size::new(4., 2.));

        for scale in [1, 2] {
            window.set_render_scaling(f64::from(scale));
            presenter.measure(Size::new(20., 20.));
            presenter.arrange(Rect::new(0., 0., 20., 20.));
            let dpi = 96. * f64::from(scale);
            let frame = RenderTargetBitmap::with_dpi(PixelSize::new(20 * scale, 20 * scale), Vector::new(dpi, dpi));
            frame.render(&presenter.clone().upcast());
            let pixel = |x, y| {
                let mut bytes = [0; 4];
                frame.copy_pixels(PixelRect::new(x * scale, y * scale, 1, 1), &mut bytes, 4);
                if frame.format() == Some(PixelFormat::BGRA8888) {
                    bytes.swap(0, 2);
                }
                bytes
            };
            assert_eq!(window.render_scaling(), f64::from(scale));
            assert_eq!(image.bounds(), Rect::new(0., 5., 20., 10.));
            assert_eq!(pixel(2, 1), [0, 0, 0, 0]);
            assert_eq!(pixel(2, 8), [255, 0, 0, 255]);
            assert_eq!(pixel(15, 8), [0, 0, 255, 255]);
            let stripe = pixel(1, 8);
            if scale == 1 {
                assert!(stripe[1] > 0 && stripe[1] < 255);
            } else {
                assert_eq!(stripe, [255, 255, 255, 255]);
            }
            assert!(Rc::ptr_eq(&image.source().unwrap(), &source));
        }

        presenter.set_icon(Some(Rc::new("ms-appx://ferroribbon_svg_test/bad.svg".to_owned())));
        let failed = presenter.children().get(0).cast::<Image>().unwrap().source().unwrap();
        wait_until(|| deferred(&failed).load_result().is_some());
        let error = deferred(&failed).load_result().unwrap().unwrap_err();
        assert!(matches!(&*error, RibbonImageLoadError::Load(error) if error.kind() == io::ErrorKind::InvalidData));
        window.set_content(None);
        window.close();
    }, CancellationToken::none()).wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Requires the Skia decoder and rendering at more than one display scale"]
fn packaged_uri_images_reload_the_selected_scale_variant() {
    const QUALIFIED_PNG: &[u8] = include_bytes!("../../tests/fixtures/images/bitmap-scale-6x4.png");
    const URI: &str = "ms-appx://ferroribbon_scale_test/icons/deep%20folder/tone%23mark.png";
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                register_assets(
                    "ferroribbon_scale_test",
                    &[
                        ("icons/deep folder/tone#mark.scale-100.png", PNG),
                        ("icons/scale-200/deep folder/tone#mark.png", QUALIFIED_PNG),
                        ("icons/deep folder/tone#mark.scale-400.png", b"invalid PNG"),
                        ("neutral.png", PNG),
                        ("neutral.scale-200.png", QUALIFIED_PNG),
                    ],
                );

                let native = RibbonIconPresenter::try_load_bitmap_uri(
                    &"ms-appx://ferroribbon_scale_test/neutral.png".into(),
                )
                .unwrap();
                assert_eq!(native.pixel_size(), PixelSize::new(6, 4));

                let panel = StackPanel::new();
                let presenter = RibbonIconPresenter::new();
                presenter.set_icon_size(20.);
                presenter.set_icon(Some(Rc::new(URI.to_owned())));
                let presented = presenter.children().get(0).cast::<Image>().unwrap();
                panel.children().add(presenter.clone());
                let menu = RibbonItemHelper::create_menu_icon(Some(Rc::new(URI.to_owned())))
                    .unwrap()
                    .unwrap()
                    .cast::<ImageIcon>()
                    .unwrap();
                panel.children().add(menu.clone());
                let bitmap = BitmapIcon::new();
                bitmap.set_uri_source(Some(Uri::absolute(URI).unwrap()));
                panel.children().add(bitmap.clone());

                let window = Window::new();
                window.set_content(Some(Control::boxed(panel.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                let images = [
                    presented.clone(),
                    templated_image(&menu),
                    templated_image(&bitmap),
                ];
                let sources = images.each_ref().map(|image| image.source().unwrap());
                let events = Rc::new(RefCell::new([Vec::new(), Vec::new(), Vec::new()]));
                for (index, image) in images.iter().enumerate() {
                    let opened = events.clone();
                    image.image_opened(move |_, _| opened.borrow_mut()[index].push(true));
                    let failed = events.clone();
                    image.image_failed(move |_, args| {
                        assert!(
                            args.error()
                                .downcast_ref::<RibbonImageLoadError>()
                                .is_some()
                        );
                        failed.borrow_mut()[index].push(false);
                    });
                }

                let check_dims = |pixels: PixelSize| {
                    for (image, source) in images.iter().zip(&sources) {
                        assert!(Rc::ptr_eq(&image.source().unwrap(), source));
                        let selected = image.displayed_source().unwrap();
                        assert_eq!(selected.size(), Size::new(3., 2.));
                        assert_eq!(selected.as_bitmap().unwrap().pixel_size(), pixels);
                    }
                };
                check_dims(PixelSize::new(3, 2));

                // A second window can render the same logical source at its own scale.
                let secondary = Image::new();
                secondary.set_source(Some(sources[0].clone()));
                let other_window = Window::new();
                other_window.set_content(Some(Control::boxed(secondary.clone())));
                other_window.show();

                window.set_render_scaling(2.);
                Dispatcher::ui_thread().run_jobs(None);
                check_dims(PixelSize::new(6, 4));
                assert_eq!(
                    secondary
                        .displayed_source()
                        .unwrap()
                        .as_bitmap()
                        .unwrap()
                        .pixel_size(),
                    PixelSize::new(3, 2)
                );
                presenter.measure(Size::new(20., 20.));
                presenter.arrange(Rect::new(0., 0., 20., 20.));
                let frame =
                    RenderTargetBitmap::with_dpi(PixelSize::new(40, 40), Vector::new(192., 192.));
                frame.render(&presenter.upcast());
                let mut pixel = [0; 4];
                frame.copy_pixels(PixelRect::new(33, 18, 1, 1), &mut pixel, 4);
                if frame.format() == Some(PixelFormat::BGRA8888) {
                    pixel.swap(0, 2);
                }
                assert_eq!(pixel, [0, 192, 255, 255]);

                window.set_render_scaling(1.5);
                Dispatcher::ui_thread().run_jobs(None);
                check_dims(PixelSize::new(6, 4));
                assert!(events.borrow().iter().all(|history| history == &[true]));

                window.set_render_scaling(3.);
                Dispatcher::ui_thread().run_jobs(None);
                assert!(bitmap.load_error().is_some());
                assert!(
                    events
                        .borrow()
                        .iter()
                        .all(|history| history == &[true, false])
                );
                window.set_render_scaling(1.);
                Dispatcher::ui_thread().run_jobs(None);
                check_dims(PixelSize::new(3, 2));
                assert!(bitmap.load_error().is_none());
                assert!(
                    events
                        .borrow()
                        .iter()
                        .all(|history| history == &[true, false, true])
                );
                other_window.set_content(None);
                other_window.close();
                window.set_content(None);
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Fetches the pinned RibbonSpace icon from GitHub over HTTPS"]
fn built_in_https_loads_the_pinned_ribbonspace_svg() {
    const URI: &str = concat!(
        "https://raw.githubusercontent.com/wieslawsoltes/RibbonSpace/",
        "9755e281547923f10397dffa366e3355b12b55ea/samples/",
        "RibbonSpace.Demo/Assets/Icons/icon.svg",
    );
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                let presenter = RibbonIconPresenter::new();
                presenter.set_icon(Some(Rc::new(URI.to_owned())));
                let image = presenter.children().get(0).cast::<Image>().unwrap();
                let source = image.source().unwrap();
                wait_until(|| deferred(&source).load_result().is_some());
                deferred(&source).load_result().unwrap().unwrap();
                assert_eq!(source.size(), Size::new(456., 456.));
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match listener.accept() {
            Ok((socket, _)) => {
                socket.set_nonblocking(false).unwrap();
                socket.set_read_timeout(Some(TIMEOUT)).unwrap();
                socket.set_write_timeout(Some(TIMEOUT)).unwrap();
                return socket;
            }
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => panic!("waiting for test HTTP request: {error}"),
        }
    }
}

fn deferred(source: &Rc<dyn IImage>) -> &RibbonDeferredImage {
    source
        .as_object()
        .unwrap()
        .downcast_ref::<RibbonDeferredImage>()
        .unwrap()
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + TIMEOUT;
    while !ready() {
        assert!(
            Instant::now() < deadline,
            "waiting for URI image completion"
        );
        Dispatcher::ui_thread().run_jobs(None);
        thread::sleep(Duration::from_millis(2));
    }
}

fn assert_png(source: &Rc<dyn IImage>) {
    let deferred = deferred(source);
    wait_until(|| deferred.load_result().is_some());
    deferred.load_result().unwrap().unwrap();
    let drawing = deferred.drawing().unwrap().cast::<ImageDrawing>().unwrap();
    let decoded = drawing.image_source().unwrap();
    let decoded = decoded.as_any().downcast_ref::<Bitmap>().unwrap();
    assert_eq!(decoded.pixel_size(), PixelSize::new(3, 2));
    let expected = Bitmap::from_stream(&mut Cursor::new(PNG)).unwrap();
    let mut actual_pixels = [0; 24];
    let mut expected_pixels = [0; 24];
    let bounds = PixelRect::new(0, 0, 3, 2);
    decoded.copy_pixels(bounds, &mut actual_pixels, 12);
    expected.copy_pixels(bounds, &mut expected_pixels, 12);
    assert_eq!(actual_pixels, expected_pixels);
}

fn templated_image(icon: &Control) -> ferroui_base::Ref<Image> {
    icon.visual_children_snapshot().unwrap()[0]
        .cast::<Grid>()
        .unwrap()
        .children()
        .get(0)
        .cast::<Image>()
        .unwrap()
}

#[test]
#[ignore = "Requires the Skia decoder and live palette and theme changes"]
fn packaged_images_follow_theme_and_contrast_in_each_window() {
    use ferroui_base::{
        FerroLocator,
        platform::{
            ColorContrastPreference, DefaultPlatformSettings, IPlatformSettings,
            PlatformColorValues, SystemColors,
        },
        styling::ThemeVariant,
    };

    const LARGE: &[u8] = include_bytes!("../../tests/fixtures/images/bitmap-scale-6x4.png");
    const URI: &str = "ms-appx://ferroribbon_appearance_test/icons/tone%23mark.png";
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session.dispatch(
        || {
            let _locator = FerroLocator::enter_scope();
            let settings = Rc::new(DefaultPlatformSettings::new());
            let normal = PlatformColorValues::new();
            settings.set_color_values(normal);
            FerroLocator::current_mutable()
                .bind::<dyn IPlatformSettings>()
                .to_constant(settings.clone());
            ferroribbon::register_types();
            register_assets(
                "ferroribbon_appearance_test",
                &[
                    ("icons/tone#mark.theme-light.png", PNG),
                    ("icons/tone#mark.theme-dark_scale-200.png", LARGE),
                    ("contrast-black/icons/tone#mark.scale-100.png", PNG),
                    (
                        "icons/contrast-white/scale-100/tone#mark.png",
                        b"invalid PNG",
                    ),
                ],
            );

            let presenter = RibbonIconPresenter::new();
            presenter.set_icon(Some(Rc::new(URI.to_owned())));
            let image = presenter.children().get(0).cast::<Image>().unwrap();
            let menu_icon = RibbonItemHelper::create_menu_icon(Some(Rc::new(URI.to_owned())))
                .unwrap()
                .unwrap()
                .cast::<ImageIcon>()
                .unwrap();
            let bitmap_icon = BitmapIcon::new();
            bitmap_icon.set_uri_source(Some(Uri::absolute(URI).unwrap()));
            let panel = StackPanel::new();
            panel.children().add(presenter.clone());
            panel.children().add(menu_icon.clone());
            panel.children().add(bitmap_icon.clone());
            panel.set_value(
                ThemeVariant::requested_theme_variant_property(),
                Some(ThemeVariant::light()),
            );
            let root = Window::new();
            root.set_content(Some(Control::boxed(panel.clone())));
            root.show();
            root.set_render_scaling(2.);
            Dispatcher::ui_thread().run_jobs(None);
            let images = [
                image,
                templated_image(&menu_icon),
                templated_image(&bitmap_icon),
            ];
            let sources = images.each_ref().map(|image| image.source().unwrap());
            let source = sources[0].clone();
            let other = Image::new();
            other.set_source(Some(source.clone()));
            other.set_value(
                ThemeVariant::requested_theme_variant_property(),
                Some(ThemeVariant::light()),
            );
            let other_root = Window::new();
            other_root.set_content(Some(Control::boxed(other.clone())));
            other_root.show();
            let pixels = |image: &Image| {
                image
                    .displayed_source()
                    .unwrap()
                    .as_bitmap()
                    .unwrap()
                    .pixel_size()
            };
            let check = |expected| {
                for image in &images {
                    assert_eq!(pixels(image), expected);
                }
            };
            check(PixelSize::new(3, 2));

            let events = Rc::new(RefCell::new([Vec::new(), Vec::new(), Vec::new()]));
            for (index, image) in images.iter().enumerate() {
                let opened = events.clone();
                image.image_opened(move |_, _| opened.borrow_mut()[index].push(true));
                let failed = events.clone();
                image.image_failed(move |_, _| failed.borrow_mut()[index].push(false));
            }

            panel.set_value(
                ThemeVariant::requested_theme_variant_property(),
                Some(ThemeVariant::dark()),
            );
            Dispatcher::ui_thread().run_jobs(None);
            check(PixelSize::new(6, 4));
            assert!(
                images
                    .iter()
                    .all(|image| image.displayed_source().unwrap().size() == Size::new(3., 2.))
            );
            assert_eq!(pixels(&other), PixelSize::new(3, 2));
            let high = normal.with_contrast_preference(ColorContrastPreference::High);
            settings.set_color_values(
                high.with_system_colors(Some(SystemColors::high_contrast_black())),
            );
            Dispatcher::ui_thread().run_jobs(None);
            check(PixelSize::new(3, 2));

            settings.set_color_values(
                high.with_system_colors(Some(SystemColors::high_contrast_white())),
            );
            Dispatcher::ui_thread().run_jobs(None);
            assert!(
                images
                    .iter()
                    .all(|image| image.displayed_source().unwrap().as_bitmap().is_none())
            );
            assert!(bitmap_icon.load_error().is_some());
            settings.set_color_values(normal);
            Dispatcher::ui_thread().run_jobs(None);
            check(PixelSize::new(6, 4));
            assert!(bitmap_icon.load_error().is_none());
            assert!(
                events
                    .borrow()
                    .iter()
                    .all(|history| history == &[true, true, false, true])
            );
            for (image, source) in images.iter().zip(&sources) {
                assert!(Rc::ptr_eq(&image.source().unwrap(), source));
            }
            assert!(Rc::ptr_eq(&other.source().unwrap(), &source));

            root.close();
            other_root.close();
        },
        CancellationToken::none(),
    );
    let result = result.wait();
    session.dispose();
    result.unwrap();
}
