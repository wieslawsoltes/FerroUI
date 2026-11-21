//! Framework-only consumers: no FerroRibbon control or private adapter dependency.
use ferroui_base::{
    PixelPoint, Point,
    input::{
        IPointer, InputElement, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties,
        PointerType,
    },
    threading::CancellationToken,
};
use ferroui_controls::{AppBuilder, Application, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions {
        overlay_popups: false,
        use_logical_desktop_coordinates: true,
        ..Default::default()
    })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "Framework.PointerFidelity.Consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferroui_controls::register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
#[test]
fn independent_element_release_covers_direct_external_pointer_capture() {
    run(|| {
        let handle = InputElement::new();
        let first = Pointer::new(71, PointerType::Mouse, true);
        let external = Pointer::new(72, PointerType::Touch, false);
        let first_interface: std::rc::Rc<dyn IPointer> = first.clone();
        assert!(handle.capture_pointer(&first_interface));
        external.capture(Some(&handle));
        assert_eq!(handle.pointer_captures().len(), 2);
        handle.release_pointer_captures();
        assert!(first.captured().is_none());
        assert!(external.captured().is_none());
        assert!(handle.pointer_captures().is_empty());
    });
}
#[test]
fn independent_precise_coordinates_preserve_fractions_and_legacy_pixel_api() {
    run(|| {
        let source = Window::new();
        source.set_width(400.);
        source.set_height(300.);
        source.set_position(PixelPoint::new(80, 90));
        source.show();
        source.set_render_scaling(2.);
        let target = Window::new();
        target.set_width(400.);
        target.set_height(300.);
        target.set_position(PixelPoint::ORIGIN);
        target.show();
        target.set_render_scaling(2.);
        let event = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            source.clone(),
            Pointer::new(73, PointerType::Mouse, true),
            Some(&source),
            Point::new(10.25, 15.75),
            0,
            PointerPointProperties::NONE,
            KeyModifiers::NONE,
        );
        assert_eq!(
            event.try_get_position_precise(Some(&target)),
            Some(Point::new(90.25, 105.75))
        );
        assert_eq!(event.get_position(Some(&target)), Point::new(90., 105.));
        source.close();
        target.close();
    });
}
