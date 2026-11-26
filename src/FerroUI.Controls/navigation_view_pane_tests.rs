//! Tests of the pane of `NavigationView` that #1394 ports from WinUI (docs/porting/winui-controls.md,
//! section 10.4): the shadow of the overlaying pane (`SetDropShadow`, `UnsetDropShadow`), its
//! motion (the transitions of `PaneOverlayGroup` on the compositor) and the padding of the title
//! bar (`UpdateTitleBarPadding`).

use crate::compositor_hit_testing_tests::CompositorTestServices;
use crate::navigation_test_support::{part, show, start, start_with_window_impl};
use crate::navigation_view_pane_shadow::{PANE_SHADOW_CLOSE_DURATION, PANE_SHADOW_OPEN_DURATION};
use crate::platform::IWindowImpl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::MockWindowingPlatform;
use crate::{Control, Grid, NavigationView, NavigationViewPaneDisplayMode, SplitView};
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::rendering::composition::server::ServerCompositionVisual;
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition, ICompositionObjectAnimations};
use ferroui_base::{Ref, Size, Thickness};
use std::rc::Rc;
use std::time::{Duration, Instant};

fn caster(nav: &NavigationView) -> Ref<Grid> {
    part::<Grid>(nav, "ShadowCaster")
}

fn casts_theme_shadow(caster: &Grid) -> bool {
    caster.shadow().is_some_and(|shadow| shadow.as_theme_shadow().is_some())
}

#[test]
fn the_overlaying_pane_casts_the_theme_shadow_until_it_closes() {
    let _app = start();
    for mode in [
        NavigationViewPaneDisplayMode::LeftCompact,
        NavigationViewPaneDisplayMode::LeftMinimal,
    ] {
        let nav = NavigationView::new();
        nav.set_pane_display_mode(mode);
        let scene = show(&nav, 800.0, 500.0);
        let caster = caster(&nav);
        assert!(!nav.is_pane_open());
        assert!(caster.shadow().is_none(), "{mode:?}");
        assert_eq!(caster.opacity(), 0.0, "PaneNotOverlaying");
        nav.set_is_pane_open(true);
        scene.0.update_layout();
        // SetDropShadow: the theme shadow at PaneOverlayShadowDepth; PaneOverlaying shows the caster.
        assert!(casts_theme_shadow(&caster), "{mode:?}");
        assert_eq!(caster.translation().z, 16.0);
        assert_eq!(caster.opacity(), 1.0);
        nav.set_is_pane_open(false);
        scene.0.update_layout();
        // UnsetDropShadow without a running ease out removes the shadow at once.
        assert!(caster.shadow().is_none(), "{mode:?}");
        assert_eq!(caster.opacity(), 0.0);
    }
}

#[test]
fn an_inline_pane_casts_no_shadow_and_the_depth_comes_from_the_resources() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let scene = show(&nav, 1200.0, 500.0);
    assert!(nav.is_pane_open());
    nav.set_is_pane_open(false);
    nav.set_is_pane_open(true);
    scene.0.update_layout();
    // SetDropShadow only in the compact and the minimal display mode.
    assert!(caster(&nav).shadow().is_none());
    assert_eq!(caster(&nav).opacity(), 0.0);

    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    nav.resources().set("PaneOverlayShadowDepth", Some(Rc::new(24.0_f64)));
    let scene = show(&nav, 800.0, 500.0);
    nav.set_is_pane_open(true);
    scene.0.update_layout();
    assert!(casts_theme_shadow(&caster(&nav)));
    assert_eq!(caster(&nav).translation().z, 24.0);
}

#[test]
fn the_caster_is_as_wide_as_the_pane_it_follows() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftCompact);
    let _scene = show(&nav, 800.0, 500.0);
    let caster = caster(&nav);
    assert_eq!(caster.width(), nav.template_settings().open_pane_length());
    assert_eq!(caster.horizontal_alignment(), HorizontalAlignment::Left);
}

fn server(visual: &Rc<CompositionVisual>) -> Rc<ServerCompositionVisual> {
    visual
        .compositor()
        .server()
        .get::<ServerCompositionVisual>(visual.server())
        .unwrap()
}

fn seek(visual: &Rc<CompositionVisual>, progress: f32) {
    for property in ["Offset.X", "Opacity"] {
        let controller = visual
            .try_get_animation_controller(property)
            .expect("a running animation");
        controller.pause();
        controller.set_progress(progress);
    }
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.05, "{actual} != {expected}");
}

/// A navigation view whose template has a split view and a caster 320 wide, on the compositor.
fn composited_navigation_view() -> (CompositorTestServices, Ref<NavigationView>, Ref<Grid>) {
    let services = CompositorTestServices::new(Size::new(800.0, 500.0));
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftCompact);
    nav.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let root = Grid::new();
        let split = SplitView::new();
        split.set_name(Some("RootSplitView".to_owned()));
        root.children().add(split.register_in_name_scope(&**scope));
        let caster = Grid::new();
        caster.set_name(Some("ShadowCaster".to_owned()));
        caster.set_width(320.0);
        caster.set_horizontal_alignment(HorizontalAlignment::Left);
        root.children().add(caster.register_in_name_scope(&**scope));
        root.upcast()
    })));
    services.set_content(&nav);
    services.run_jobs();
    let caster = part::<Grid>(&nav, "ShadowCaster");
    (services, nav, caster)
}

fn run_to_end(services: &CompositorTestServices, nav: &NavigationView, visual: Option<&Rc<CompositionVisual>>) {
    for controller in visual
        .into_iter()
        .flat_map(|visual| ["Offset.X", "Opacity"].map(|property| visual.try_get_animation_controller(property)))
        .flatten()
    {
        controller.resume();
    }
    let started = Instant::now();
    while nav.pane_shadow_is_running() && started.elapsed() < Duration::from_secs(2) {
        services.run_jobs();
    }
    services.run_jobs();
    assert!(!nav.pane_shadow_is_running());
}

#[test]
fn the_caster_slides_and_fades_with_the_pane_on_the_compositor() {
    assert_eq!(PANE_SHADOW_OPEN_DURATION, Duration::from_millis(350));
    assert_eq!(PANE_SHADOW_CLOSE_DURATION, Duration::from_millis(120));
    let (services, nav, caster) = composited_navigation_view();
    let split = part::<SplitView>(&nav, "RootSplitView");
    let distance = -(split.open_pane_length() - split.compact_pane_length());
    assert!(distance < 0.0);
    let visual = ElementComposition::get_element_visual(&caster).expect("a composited caster");
    let base = caster.bounds().x;

    // PaneNotOverlaying to PaneOverlaying: from NegativeOpenPaneLengthMinusCompactLength to 0
    // (key spline 0.1,0.9 0.2,1.0), the opacity from 0 to 1 (linear).
    nav.set_is_pane_open(true);
    assert!(nav.pane_shadow_is_running());
    assert!(casts_theme_shadow(&caster));
    for (progress, offset, opacity) in [(0.01, base + distance, 0.01), (0.5, f64::NAN, 0.5), (1.0, base, 1.0)] {
        seek(&visual, progress);
        services.run_jobs();
        services.run_jobs();
        let server = server(&visual);
        if offset.is_nan() {
            // The spline covers most of the distance in the first half.
            assert!(server.offset().x > base + distance * 0.2, "{}", server.offset().x);
        } else if progress < 0.5 {
            // Just after the start, near the start of the slide.
            assert!(server.offset().x < offset * 0.85, "{}", server.offset().x);
        } else {
            close(server.offset().x, offset);
        }
        close(server.opacity() as f64, opacity);
    }
    run_to_end(&services, &nav, Some(&visual));
    close(server(&visual).offset().x, base);
    close(server(&visual).opacity() as f64, 1.0);
    assert_eq!(caster.opacity(), 1.0);

    // PaneOverlaying to PaneNotOverlaying (ShadowCasterEaseOutStoryboard): back to the distance and
    // to the opacity 0; the shadow stays until the ease out completes.
    nav.set_is_pane_open(false);
    assert!(nav.pane_shadow_is_running());
    assert!(casts_theme_shadow(&caster));
    seek(&visual, 0.5);
    services.run_jobs();
    services.run_jobs();
    close(server(&visual).opacity() as f64, 0.5);
    assert!(server(&visual).offset().x < base);
    run_to_end(&services, &nav, Some(&visual));
    assert!(caster.shadow().is_none());
    assert_eq!(caster.opacity(), 0.0);
    close(server(&visual).offset().x, base);
    close(server(&visual).opacity() as f64, 0.0);
}

#[test]
fn reopening_during_the_ease_out_keeps_the_shadow() {
    let (services, nav, caster) = composited_navigation_view();
    nav.set_is_pane_open(true);
    run_to_end(&services, &nav, None);
    nav.set_is_pane_open(false);
    assert!(nav.pane_shadow_is_running());
    nav.set_is_pane_open(true);
    assert!(casts_theme_shadow(&caster));
    run_to_end(&services, &nav, None);
    assert!(casts_theme_shadow(&caster));
    assert_eq!(caster.opacity(), 1.0);
}

#[test]
fn title_bar_padding_follows_a_window_extended_into_its_decorations() {
    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.extended_margins.set(Thickness::new(0.0, 32.0, 0.0, 0.0));
    let _app = start_with_window_impl(&window_impl);
    let nav = NavigationView::new();
    let content: Ref<Control> = nav.clone().upcast();
    let scene = show(&content, 800.0, 500.0);
    let settings = nav.template_settings();
    assert_eq!(settings.top_padding(), 0.0);
    let extend = window_impl
        .extend_client_area_to_decorations_changed()
        .expect("the window listens");
    extend(true);
    scene.0.update_layout();
    assert!(scene.0.is_extended_into_window_decorations());
    // The height of the title bar above a navigation view at the top of the window.
    assert_eq!(settings.top_padding(), 32.0);
    // IsTitleBarAutoPaddingEnabled False keeps the panes at the top.
    nav.set_is_title_bar_auto_padding_enabled(false);
    assert_eq!(settings.top_padding(), 0.0);
    nav.set_is_title_bar_auto_padding_enabled(true);
    assert_eq!(settings.top_padding(), 32.0);
    // A navigation view below the top of the window gets none.
    nav.set_margin(Thickness::new(0.0, 10.0, 0.0, 0.0));
    scene.0.update_layout();
    assert_eq!(settings.top_padding(), 0.0);
    nav.set_margin(Thickness::new(0.0, 0.0, 0.0, 0.0));
    scene.0.update_layout();
    assert_eq!(settings.top_padding(), 32.0);
    extend(false);
    scene.0.update_layout();
    assert_eq!(settings.top_padding(), 0.0);
}
