//! Native Skia decoding, opacity-mask tint, identity and incremental property state.
use super::*;
use ferroribbon::compat::BitmapIcon;
use ferroui_base::{
    PixelRect, PixelSize, Rect, Size,
    media::{
        Color, LinearGradientBrush, SolidColorBrush,
        imaging::{Bitmap, RenderTargetBitmap},
    },
    platform::{AlphaFormat, PixelFormat, register_assets},
    utilities::{Uri, UriKind},
};
use ferroui_controls::{Grid, Image};
fn child(icon: &BitmapIcon) -> ferroui_base::Ref<Image> {
    icon.visual_children_snapshot().unwrap()[0]
        .cast::<Grid>()
        .unwrap()
        .children()
        .get(0)
        .cast::<Image>()
        .unwrap()
}
fn bytes(bitmap: &Bitmap) -> Vec<u8> {
    assert_eq!(bitmap.alpha_format(), Some(AlphaFormat::Premul));
    let size = bitmap.pixel_size();
    let mut b = vec![0; (size.width * size.height * 4) as usize];
    bitmap.copy_pixels(
        PixelRect::new(0, 0, size.width, size.height),
        &mut b,
        size.width * 4,
    );
    match bitmap.format().unwrap() {
        PixelFormat::RGBA8888 => {}
        PixelFormat::BGRA8888 => {
            for pixel in b.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
        }
        other => panic!("unexpected {other:?}"),
    };
    b
}

#[test]
#[ignore = "Uses the real Skia compositor to check incremental icon repaint"]
fn shared_drawing_icon_repaints_and_tracks_its_current_viewbox() {
    assert_shared_drawing_layout(false);
}

#[test]
#[ignore = "Uses the real Skia compositor to check a provider-backed icon"]
fn provider_drawing_icon_repaints_and_tracks_its_current_viewbox() {
    assert_shared_drawing_layout(true);
}

fn assert_shared_drawing_layout(load_from_provider: bool) {
    use ferroribbon::primitives::{
        RibbonIconPresenter, RibbonImageFuture, RibbonImageKind, RibbonImageProvider,
    };
    use ferroui_base::{
        FerroLocator,
        media::{DrawingImage, GeometryDrawing, IImage, RectangleGeometry},
        utilities::UriRef,
    };
    use ferroui_controls::Control;

    struct ReadyImage(Rc<dyn IImage>);
    impl RibbonImageProvider for ReadyImage {
        fn open(&self, _: &UriRef, _: RibbonImageKind) -> Option<RibbonImageFuture> {
            let image = self.0.clone();
            Some(Box::pin(async move { Ok(image) }))
        }
    }

    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                let _scope = FerroLocator::enter_scope();
                let drawing = GeometryDrawing::new();
                drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0., 0., 16., 16.)));
                let set_fill = |color| {
                    drawing.set_brush(Some(SolidColorBrush::from_uint32(color).into()));
                };
                set_fill(0xffff0000);
                let source = DrawingImage::with_drawing(&drawing);
                let native: Rc<dyn IImage> = source.clone().into();
                let presenter = RibbonIconPresenter::new();
                if load_from_provider {
                    FerroLocator::current_mutable()
                        .bind::<dyn RibbonImageProvider>()
                        .to_constant(Rc::new(ReadyImage(native.clone())));
                    presenter.set_icon(Some(Rc::new(
                        "ferres://ribbon_provider_repaint/drawing.png".to_owned(),
                    )));
                } else {
                    presenter.set_icon(Some(Rc::new(native.clone())));
                }
                let image = presenter.children().get(0).cast::<Image>().unwrap();
                let displayed = image.source().unwrap();
                assert_eq!(Rc::ptr_eq(&displayed, &native), !load_from_provider);
                let window = Window::new();
                window.set_width(64.);
                window.set_height(64.);
                window.set_background(Some(SolidColorBrush::from_uint32(0xffffffff).into()));
                window.set_content(Some(Control::boxed(&presenter)));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window
                    .renderer()
                    .paint(Rect::from_size(window.client_size()));

                let pixel = |dx, dy| {
                    let frame = window.capture_rendered_frame().unwrap();
                    let size = frame.pixel_size();
                    let mut rgba = [0; 4];
                    frame.copy_pixels(
                        PixelRect::new(size.width / 2 + dx, size.height / 2 + dy, 1, 1),
                        &mut rgba,
                        4,
                    );
                    if frame.format() == Some(PixelFormat::BGRA8888) {
                        rgba.swap(0, 2);
                    }
                    rgba
                };
                assert_eq!(pixel(0, 0), [255, 0, 0, 255]);
                set_fill(0xff00ff00);
                assert_eq!(pixel(0, 0), [0, 255, 0, 255]);
                set_fill(0xff0000ff);
                assert_eq!(pixel(0, 0), [0, 0, 255, 255]);

                source.set_viewbox(Some(Rect::new(0., 0., 16., 8.)));
                assert_eq!(pixel(0, -6), [255, 255, 255, 255]);
                assert_eq!(pixel(-6, 0), [0, 0, 255, 255]);
                assert_eq!(image.bounds().size(), Size::new(16., 8.));
                source.set_viewbox(Some(Rect::new(0., 0., 8., 16.)));
                assert_eq!(pixel(0, -6), [0, 0, 255, 255]);
                assert_eq!(pixel(-6, 0), [255, 255, 255, 255]);
                assert_eq!(image.bounds().size(), Size::new(8., 16.));
                assert!(Rc::ptr_eq(&image.source().unwrap(), &displayed));

                source.set_viewbox(None);
                source.set_drawing(None);
                assert_eq!(pixel(0, 0), [255, 255, 255, 255]);
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit real Skia BitmapIcon pixels; source async/OS/platform parity separate"]
fn bitmap_uri_tint_modes_update_without_decode_reload_and_keep_alpha() {
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                register_assets(
                    "bitmap_icon_pixels",
                    &[
                        (
                            "mask.png",
                            include_bytes!("../../tests/fixtures/images/bitmap-mask-3x2.png"),
                        ),
                        (
                            "second.png",
                            include_bytes!("../../tests/fixtures/images/bitmap-mask-3x2.png"),
                        ),
                    ],
                );
                let icon = BitmapIcon::new();
                icon.set_use_layout_rounding(false);
                icon.set_width(3.);
                icon.set_height(2.);
                icon.set_foreground(Some(
                    SolidColorBrush::with_color_and_opacity(Color::from_uint32(0xffff0000), 0.25)
                        .into(),
                ));
                icon.set_uri_source(Some(
                    Uri::new("ferres://bitmap_icon_pixels/mask.png", UriKind::Absolute).unwrap(),
                ));
                assert!(icon.load_error().is_none());
                let image = child(&icon);
                let original = image.source().unwrap();
                let decoded = original.as_bitmap().unwrap()
                    as *const dyn ferroui_base::media::imaging::IBitmap;
                let paint = |name: &str| {
                    icon.measure(Size::new(3., 2.));
                    icon.arrange(Rect::new(0., 0., 3., 2.));
                    let result = RenderTargetBitmap::new(PixelSize::new(3, 2));
                    result.render(&icon.clone().upcast());
                    result
                        .save_to_file(
                            directory.join(name).to_str().unwrap(),
                            &PngBitmapEncoderOptions::default().into(),
                        )
                        .unwrap();
                    bytes(&result)
                };
                let red = paint("bitmap-icon-red.png");
                let expected_alpha = [255, 128, 64, 0, 32, 200];
                for (p, a) in red.as_chunks::<4>().0.iter().zip(expected_alpha) {
                    assert_eq!(p[3], a);
                    assert_eq!(p[1], 0);
                    assert_eq!(p[2], 0);
                    assert_eq!(p[0], a);
                }
                icon.set_foreground(Some(SolidColorBrush::from_uint32(0xff0000ff).into()));
                let blue = paint("bitmap-icon-blue.png");
                for (p, a) in blue.as_chunks::<4>().0.iter().zip(expected_alpha) {
                    assert_eq!(p[3], a);
                    assert_eq!(p[0], 0);
                    assert_eq!(p[1], 0);
                    assert_eq!(p[2], a);
                }
                assert!(Rc::ptr_eq(&original, &image.source().unwrap()));
                assert!(std::ptr::addr_eq(
                    decoded,
                    image.source().unwrap().as_bitmap().unwrap()
                ));
                icon.set_foreground(Some(SolidColorBrush::from_uint32(0x80ff0000).into()));
                let translucent = paint("bitmap-icon-translucent.png");
                for (p, a) in translucent.as_chunks::<4>().0.iter().zip(expected_alpha) {
                    let expected = ((u32::from(a) * 128 + 127) / 255) as u8;
                    assert_eq!(*p, [expected, 0, 0, expected]);
                }
                icon.set_show_as_monochrome(false);
                let plain = paint("bitmap-icon-color.png");
                assert_ne!(plain, blue);
                let reference = Image::new();
                reference.set_use_layout_rounding(false);
                let mut stream = std::io::Cursor::new(include_bytes!(
                    "../../tests/fixtures/images/bitmap-mask-3x2.png"
                ));
                reference.set_source(Some(Rc::new(Bitmap::from_stream(&mut stream).unwrap())));
                reference.measure(Size::new(3., 2.));
                reference.arrange(Rect::new(0., 0., 3., 2.));
                let reference_frame = RenderTargetBitmap::new(PixelSize::new(3, 2));
                reference_frame.render(&reference.upcast());
                assert_eq!(
                    plain,
                    bytes(&reference_frame),
                    "untinted icon matches independent native Image decoding"
                );
                icon.set_foreground(Some(SolidColorBrush::from_uint32(0xff00ff00).into()));
                assert_eq!(plain, paint("bitmap-icon-color-foreground.png"));
                icon.set_show_as_monochrome(true);
                icon.set_foreground(Some(LinearGradientBrush::new().into()));
                assert_eq!(plain, paint("bitmap-icon-gradient.png"));
                assert!(Rc::ptr_eq(&original, &image.source().unwrap()));
                assert!(std::ptr::addr_eq(
                    decoded,
                    image.source().unwrap().as_bitmap().unwrap()
                ));
                let old = Rc::downgrade(&original);
                icon.set_uri_source(Some(
                    Uri::new("ferres://bitmap_icon_pixels/second.png", UriKind::Absolute).unwrap(),
                ));
                assert!(!Rc::ptr_eq(&original, &image.source().unwrap()));
                drop(original);
                assert!(old.upgrade().is_none());
                icon.set_uri_source(Some(
                    Uri::new("ferres://bitmap_icon_pixels/missing.png", UriKind::Absolute).unwrap(),
                ));
                assert!(icon.load_error().is_some());
                assert!(image.source().unwrap().as_bitmap().is_none());
                assert!(paint("bitmap-icon-failed.png").iter().all(|v| *v == 0));
                icon.set_uri_source(None);
                assert!(image.source().is_none() && icon.load_error().is_none());
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit replacement image tint/geometry pixels; source OS parity separate"]
fn bitmap_replacement_sources_preserve_identity_alpha_and_native_image_geometry() {
    replacement_source_pixels(true, false);
}

#[test]
#[ignore = "Explicit recorded image tint/geometry pixels and layer isolation"]
fn bitmap_replacement_sources_preserve_recorded_identity_alpha_and_native_image_geometry() {
    replacement_source_pixels(true, true);
}

#[test]
#[ignore = "Explicit aligned replacement pixels; full direct/recorded matrices run separately"]
fn bitmap_replacement_sources_tint_aligned_native_images_without_replacing_them() {
    replacement_source_pixels(false, false);
}

fn replacement_source_pixels(full_matrix: bool, recorded: bool) {
    use ferroui_base::media::{
        DrawingImage, GeometryDrawing, IImage, ImageDrawing, RectangleGeometry, Stretch,
        StretchDirection, imaging::CroppedBitmap,
    };
    let directory = PathBuf::from(std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").unwrap());
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                let bitmap: Rc<dyn IImage> = Rc::new(
                    Bitmap::from_stream(&mut std::io::Cursor::new(include_bytes!(
                        "../../tests/fixtures/images/bitmap-mask-3x2.png"
                    )))
                    .unwrap(),
                );
                let drawing = ImageDrawing::new();
                drawing.set_image_source(Some(bitmap.clone()));
                drawing.set_rect(Rect::new(0., 0., 3., 2.));
                let drawn: Rc<dyn IImage> = DrawingImage::with_drawing(drawing).into();
                let cropped: Rc<dyn IImage> =
                    CroppedBitmap::with_source(bitmap.clone(), PixelRect::new(1, 0, 2, 2)).into();
                let shape = GeometryDrawing::new();
                shape.set_geometry(RectangleGeometry::with_rect(Rect::new(0., 0., 3.5, 2.5)));
                shape.set_brush(Some(SolidColorBrush::from_uint32(0x804080c0).into()));
                let vector: Rc<dyn IImage> = DrawingImage::with_drawing(shape.clone()).into();
                let sources = [
                    ("bitmap", bitmap),
                    ("drawing", drawn),
                    ("cropped", cropped),
                    ("vector", vector),
                ];
                let icon = BitmapIcon::new();
                icon.set_use_layout_rounding(false);
                let image = child(&icon);
                image.clear_local_value_and_binding(Image::source_property());
                let reference = Image::new();
                let reference_root = Grid::new();
                reference_root.set_use_layout_rounding(false);
                reference_root.children().add(reference.clone());
                let render = |root: ferroui_base::Ref<ferroui_controls::Control>, size: Size| {
                    root.measure(size);
                    root.arrange(Rect::from_size(size));
                    let frame = RenderTargetBitmap::new(PixelSize::new(
                        size.width as i32,
                        size.height as i32,
                    ));
                    if recorded {
                        use ferroui_base::{media::{DrawingContext, ISceneBrushContent, immutable::ImmutableImageBrush}, rendering::{ImmediateRenderer, composition::drawing::RenderDataDrawingContext}};
                        let mut recorder = RenderDataDrawingContext::new(None);
                        {
                            let mut context = DrawingContext::new(&mut recorder);
                            ImmediateRenderer::render(&mut context,&root.upcast());
                        }
                        let content=recorder.get_immediate_scene_brush_content(Rc::new(ImmutableImageBrush::from_bitmap(None)),None,true).unwrap();
                        let mut context=frame.create_platform_drawing_context(true);
                        content.render(&mut *context,None);context.dispose();
                    } else {
                        frame.render(&root.upcast());
                    }
                    frame
                };
                let mut cases = 0;
                let mut mismatches = Vec::new();
                for (name, source) in &sources {
                    image.set_source(Some(source.clone()));
                    reference.set_source(Some(source.clone()));
                    for stretch in [
                        Stretch::None,
                        Stretch::Fill,
                        Stretch::Uniform,
                        Stretch::UniformToFill,
                    ] {
                        if !full_matrix && stretch != Stretch::Fill { continue; }
                        image.set_stretch(stretch);
                        reference.set_stretch(stretch);
                        for direction in [
                            StretchDirection::Both,
                            StretchDirection::UpOnly,
                            StretchDirection::DownOnly,
                        ] {
                            if !full_matrix && direction != StretchDirection::Both { continue; }
                            image.set_stretch_direction(direction);
                            reference.set_stretch_direction(direction);
                            for size in [
                                Size::new(3., 2.),
                                Size::new(6., 4.),
                                Size::new(5., 5.),
                                Size::new(1., 1.),
                            ] {
                                let plain = bytes(&render(reference_root.clone().upcast(), size));
                                icon.set_show_as_monochrome(true);
                                icon.set_foreground(Some(
                                    SolidColorBrush::from_uint32(0xffff0000).into(),
                                ));
                                let tinted = render(icon.clone().upcast(), size);
                                let rgba = bytes(&tinted);
                                let mut differences = Vec::new();
                                for (index, (actual, original)) in rgba
                                    .as_chunks::<4>()
                                    .0
                                    .iter()
                                    .zip(plain.as_chunks::<4>().0)
                                    .enumerate()
                                {
                                    let expected = [original[3],0,0,original[3]];
                                    if *actual != expected { differences.push(serde_json::json!({"pixel":index,"actual":actual,"expected":expected})); }
                                }
                                if !differences.is_empty() {
                                    mismatches.push(serde_json::json!({"source":name,"stretch":format!("{stretch:?}"),"direction":format!("{direction:?}"),"width":size.width,"height":size.height,"differences":differences}));
                                }
                                icon.set_show_as_monochrome(false);
                                assert_eq!(
                                    bytes(&render(icon.clone().upcast(), size)),
                                    plain,
                                    "untinted {name} {stretch:?} {direction:?} {size:?}"
                                );
                                assert!(Rc::ptr_eq(source, &image.source().unwrap()));
                                assert!(child(&icon).ptr_eq(&image));
                                if stretch == Stretch::Fill
                                    && direction == StretchDirection::Both
                                    && size == Size::new(6., 4.)
                                {
                                    tinted
                                        .save_to_file(
                                            directory
                                                .join(format!("bitmap-replacement-{}{name}.png",if recorded {"recorded-"} else {""}))
                                                .to_str()
                                                .unwrap(),
                                            &PngBitmapEncoderOptions::default().into(),
                                        )
                                        .unwrap();
                                }
                                cases += 1;
                            }
                        }
                    }
                }
                assert_eq!(cases, if full_matrix {192} else {16});
                std::fs::write(directory.join(if recorded {"bitmap-replacement-recorded.json"} else if full_matrix {"bitmap-replacement-sampling.json"} else {"bitmap-replacement-aligned.json"}),serde_json::to_vec_pretty(&serde_json::json!({"cases":cases,"mismatches":mismatches})).unwrap()).unwrap();
                // Mutating the same DrawingImage keeps native source identity while
                // native drawing/brush invalidation makes the new alpha observable.
                let vector = &sources[3].1;
                image.set_source(Some(vector.clone()));
                image.set_stretch(Stretch::Fill);
                image.set_stretch_direction(StretchDirection::Both);
                icon.set_show_as_monochrome(true);
                shape.set_brush(Some(SolidColorBrush::from_uint32(0x400000ff).into()));
                let frame = render(icon.clone().upcast(), Size::new(7., 5.));
                for pixel in bytes(&frame).as_chunks::<4>().0 {
                    assert_eq!(*pixel, [64, 0, 0, 64]);
                }
                assert!(Rc::ptr_eq(vector, &image.source().unwrap()));
                frame
                    .save_to_file(
                        directory
                            .join(if recorded {"bitmap-replacement-recorded-vector-mutated.png"} else {"bitmap-replacement-vector-mutated.png"})
                            .to_str()
                            .unwrap(),
                        &PngBitmapEncoderOptions::default().into(),
                    )
                    .unwrap();
                image.set_source(None);
                assert!(
                    bytes(&render(icon.upcast(), Size::new(3., 2.)))
                        .iter()
                        .all(|v| *v == 0)
                );
                assert!(mismatches.is_empty(), "{} of {cases} native image tint cases differ; see the explicit capture directory's sampling report", mismatches.len());
                // The isolated tint must preserve pixels outside the Image.
                let background=Grid::new();background.set_use_layout_rounding(false);
                background.set_background(Some(SolidColorBrush::from_uint32(0xff00ff00).into()));
                let icon=BitmapIcon::new();icon.set_width(3.);icon.set_height(2.);
                icon.set_foreground(Some(SolidColorBrush::from_uint32(0xffff0000).into()));
                child(&icon).set_source(Some(sources[0].1.clone()));
                background.children().add(icon);
                let background_pixels=bytes(&render(background.upcast(),Size::new(6.,4.)));
                assert_eq!(&background_pixels[..4],&[0,255,0,255]);
                assert!(background_pixels.as_chunks::<4>().0.iter().any(|p|p[0]>0));
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Negative control for the rejected brush-mask mapping; not a product parity gate"]
fn native_drawing_brush_fractional_alpha_matches_native_image() {
    use ferroui_base::{
        layout::{HorizontalAlignment, VerticalAlignment},
        media::{DrawingBrush, ImageDrawing, Stretch},
    };
    use ferroui_controls::shapes::Rectangle;
    let directory = PathBuf::from(std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").unwrap());
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session.dispatch(move || {
        // All controls, properties, images, brushes and rendering below are SDK
        // services. No FerroRibbon control or icon helper is constructed.
        let source = Rc::new(Bitmap::from_stream(&mut std::io::Cursor::new(include_bytes!("../../tests/fixtures/images/bitmap-mask-3x2.png"))).unwrap());
        let reference = Image::new(); reference.set_source(Some(source.clone())); reference.set_stretch(Stretch::None);
        reference.set_horizontal_alignment(HorizontalAlignment::Center); reference.set_vertical_alignment(VerticalAlignment::Center);
        let drawing = ImageDrawing::new(); drawing.set_image_source(Some(source)); drawing.set_rect(Rect::new(0.,0.,3.,2.));
        let mask = DrawingBrush::with_drawing(drawing); mask.set_stretch(Stretch::Fill);
        let rectangle = Rectangle::new(); rectangle.set_width(3.); rectangle.set_height(2.);
        rectangle.set_horizontal_alignment(HorizontalAlignment::Center); rectangle.set_vertical_alignment(VerticalAlignment::Center);
        rectangle.set_fill(Some(SolidColorBrush::from_uint32(0xffff0000).into())); rectangle.set_opacity_mask(Some(mask.into()));
        let render = |control: ferroui_base::Ref<ferroui_controls::Control>, name: &str| {
            let root = Grid::new(); root.set_use_layout_rounding(false); root.children().add(control);
            root.measure(Size::new(6.,4.)); root.arrange(Rect::new(0.,0.,6.,4.));
            let frame = RenderTargetBitmap::new(PixelSize::new(6,4)); frame.render(&root.upcast());
            frame.save_to_file(directory.join(name).to_str().unwrap(), &PngBitmapEncoderOptions::default().into()).unwrap(); bytes(&frame)
        };
        let plain = render(reference.upcast(),"native-image-fractional.png");
        let tinted = render(rectangle.upcast(),"native-drawing-brush-fractional.png");
        let differences: Vec<_> = plain.as_chunks::<4>().0.iter().zip(tinted.as_chunks::<4>().0).enumerate().filter(|(_, (a,b))| a[3] != b[3]).map(|(pixel,(a,b))| serde_json::json!({"pixel":pixel,"image_alpha":a[3],"brush_alpha":b[3]})).collect();
        std::fs::write(directory.join("native-alpha-difference.json"),serde_json::to_vec_pretty(&differences).unwrap()).unwrap();
        assert!(differences.is_empty(),"SDK Image vs DrawingBrush alpha differs at {} pixels",differences.len());
    },CancellationToken::none()).wait();
    session.dispose();
    result.unwrap();
}
