//! Original wheel algorithm and event hook through real native scroll viewers.
use super::{
    scroll_viewer_mouse_wheel_hook_helper::ScrollViewerMouseWheelHookHelper as Hook,
    tab_strip_mouse_wheel_scroll_helper::TabStripMouseWheelScrollHelper as Scroll,
};
use ferroui_base::{
    input::{
        KeyModifiers, Pointer, PointerPointProperties, PointerType, PointerUpdateKind,
        PointerWheelEventArgs, RawInputModifiers,
    },
    layout::Orientation,
    threading::{CancellationToken, Dispatcher},
    Point, Ref, Vector,
};
use ferroui_controls::{
    primitives::ScrollBarVisibility, AppBuilder, Application, Border, Control, ScrollViewer, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "tab-strip-wheel",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                f()
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
struct Scene {
    scroll: Ref<ScrollViewer>,
    window: Ref<Window>,
}
impl Scene {
    fn new(width: f64, height: f64) -> Self {
        let content = Border::new();
        content.set_width(width);
        content.set_height(height);
        let scroll = ScrollViewer::new();
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll.set_content(Some(Control::boxed(content.clone())));
        let window = Window::new();
        window.set_width(240.0);
        window.set_height(180.0);
        window.set_content(Some(Control::boxed(scroll.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        Self { scroll, window }
    }
    fn wheel(&self, delta: Vector, handled: bool) -> PointerWheelEventArgs {
        let args = PointerWheelEventArgs::new(
            self.scroll.clone(),
            Pointer::new(1, PointerType::Mouse, true),
            &self.window,
            Point::new(20.0, 20.0),
            0,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
            KeyModifiers::NONE,
            delta,
        );
        args.set_handled(handled);
        self.scroll.raise_event(&args);
        args
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
    }
}
#[test]
fn original_null_zero_epsilon_no_overflow_and_unmoved_edge_returns_false() {
    run(|| {
        assert!(!Scroll::try_handle(
            None,
            Orientation::Horizontal,
            Vector::new(0.0, -1.0)
        ));
        assert!(Hook::attach(None, Orientation::Horizontal).is_none());
        let scene = Scene::new(100.0, 80.0);
        assert!(scene.scroll.extent().width <= scene.scroll.viewport().width);
        assert!(!Scroll::try_handle(
            Some(&scene.scroll),
            Orientation::Horizontal,
            Vector::new(0.0, -1.0)
        ));
        assert!(!Scroll::try_handle(
            Some(&scene.scroll),
            Orientation::Vertical,
            Vector::new(0.0, -1.0)
        ));
        let scene = Scene::new(2000.0, 1000.0);
        for delta in [0.0, -0.0, f64::from_bits(1), -f64::from_bits(1)] {
            assert!(!Scroll::try_handle(
                Some(&scene.scroll),
                Orientation::Horizontal,
                Vector::new(100.0, delta)
            ));
        }
        assert_eq!(scene.scroll.offset(), Vector::default());
        assert!(!Scroll::try_handle(
            Some(&scene.scroll),
            Orientation::Horizontal,
            Vector::new(0.0, 1.0)
        ));
        assert!(!Scroll::try_handle(
            Some(&scene.scroll),
            Orientation::Vertical,
            Vector::new(0.0, 1.0)
        ));
    });
}
#[test]
fn source_ceiling_steps_and_smallest_subnormal_threshold_use_original_axis_and_line_methods() {
    run(|| {
        let scene = Scene::new(2000.0, 1000.0);
        let scroll = &scene.scroll;
        assert!(scroll.extent().width > scroll.viewport().width);
        assert!(scroll.extent().height > scroll.viewport().height);
        let size = scroll.small_change();
        assert!(Scroll::try_handle(
            Some(scroll),
            Orientation::Horizontal,
            Vector::new(999.0, -1.25)
        ));
        assert_eq!(scroll.offset(), Vector::new(size.width * 2.0, 0.0));
        assert!(Scroll::try_handle(
            Some(scroll),
            Orientation::Horizontal,
            Vector::new(0.0, 0.125)
        ));
        assert_eq!(scroll.offset(), Vector::new(size.width, 0.0));
        assert!(Scroll::try_handle(
            Some(scroll),
            Orientation::Vertical,
            Vector::new(-999.0, -2.01)
        ));
        assert_eq!(scroll.offset(), Vector::new(size.width, size.height * 3.0));
        assert!(Scroll::try_handle(
            Some(scroll),
            Orientation::Vertical,
            Vector::new(0.0, 1.0)
        ));
        assert_eq!(scroll.offset(), Vector::new(size.width, size.height * 2.0));
        scroll.set_offset(Vector::default());
        assert!(Scroll::try_handle(
            Some(scroll),
            Orientation::Horizontal,
            Vector::new(0.0, -f64::from_bits(2))
        ));
        assert_eq!(scroll.offset().x, size.width);
        scroll.set_offset(Vector::new(f64::MAX, f64::MAX));
        let end = scroll.offset();
        assert!(!Scroll::try_handle(
            Some(scroll),
            Orientation::Horizontal,
            Vector::new(0.0, -2.5)
        ));
        assert!(!Scroll::try_handle(
            Some(scroll),
            Orientation::Vertical,
            Vector::new(0.0, -1.0)
        ));
        assert_eq!(scroll.offset(), end);
    });
}
#[test]
fn routed_hook_handles_only_movement_and_explicit_disposal_allows_orientation_replacement() {
    run(|| {
        let scene = Scene::new(2000.0, 1000.0);
        let hook = Hook::attach(Some(&scene.scroll), Orientation::Horizontal).unwrap();
        let first = scene.wheel(Vector::new(0.0, -1.0), false);
        assert!(first.handled());
        assert_eq!(
            scene.scroll.offset(),
            Vector::new(scene.scroll.small_change().width, 0.0)
        );
        let before = scene.scroll.offset();
        scene.wheel(Vector::new(0.0, -1.0), true);
        assert_eq!(scene.scroll.offset(), before);
        hook.dispose();
        hook.dispose();
        let hook = Hook::attach(Some(&scene.scroll), Orientation::Vertical).unwrap();
        let next = scene.wheel(Vector::new(0.0, -1.0), false);
        assert!(next.handled());
        assert_eq!(
            scene.scroll.offset(),
            Vector::new(before.x, scene.scroll.small_change().height)
        );
        hook.dispose();
        scene.scroll.set_offset(Vector::default());
        let hook = Hook::attach(Some(&scene.scroll), Orientation::Horizontal).unwrap();
        assert!(!scene.wheel(Vector::new(0.0, 1.0), false).handled());
        hook.dispose();
    });
}
#[test]
fn source_hook_survives_detach_reattach_and_disposal_releases_strong_captured_scroll_viewer() {
    run(|| {
        let scene = Scene::new(2000.0, 1000.0);
        let hook = Hook::attach(Some(&scene.scroll), Orientation::Horizontal).unwrap();
        let weak = scene.scroll.downgrade();
        scene.window.set_content(None);
        scene.window.update_layout();
        drop(scene);
        assert!(weak.upgrade().is_some());
        hook.dispose();
        assert!(weak.upgrade().is_none());
        let scene = Scene::new(2000.0, 1000.0);
        let hook = Hook::attach(Some(&scene.scroll), Orientation::Horizontal).unwrap();
        scene.window.set_content(None);
        scene.window.update_layout();
        scene
            .window
            .set_content(Some(Control::boxed(scene.scroll.clone())));
        scene.window.update_layout();
        assert!(scene.wheel(Vector::new(0.0, -1.0), false).handled());
        hook.dispose();
    });
}
#[test]
fn child_routing_uses_horizontal_hook_when_presenter_cannot_scroll_and_respects_native_handling() {
    run(|| {
        for (height, expected) in [
            (80.0, Vector::new(16.0, 0.0)),
            (1000.0, Vector::new(0.0, 50.0)),
        ] {
            let scene = Scene::new(2000.0, height);
            let hook = Hook::attach(Some(&scene.scroll), Orientation::Horizontal).unwrap();
            let content = Control::from_boxed(&scene.scroll.content().unwrap()).unwrap();
            let args = PointerWheelEventArgs::new(
                content.clone(),
                Pointer::new(2, PointerType::Mouse, true),
                &scene.window,
                Point::new(20.0, 20.0),
                0,
                PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
                KeyModifiers::NONE,
                Vector::new(0.0, -1.0),
            );
            content.raise_event(&args);
            assert!(args.handled());
            assert_eq!(scene.scroll.offset(), expected);
            hook.dispose();
        }
    });
}
