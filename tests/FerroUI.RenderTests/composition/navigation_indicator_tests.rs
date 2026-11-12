//! Pixel regressions for the actual WinUI sidebar template and nested item hierarchy.
use crate::manual_render_timer::ManualRenderTimer;
use crate::test_base::TestBase;
use crate::test_render_helper::{get_output_directory, Rgba32Image};
use crate::test_render_root::TestRenderRoot;
use ferroui_base::media::imaging::PngBitmapEncoderOptions;
use ferroui_base::media::{Brushes, MediaContext};
use ferroui_base::platform::surfaces::{
    FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::platform::{IPlatformRenderInterface, IWriteableBitmapImpl};
use ferroui_base::rendering::composition::{
    CompositingRenderer, Compositor, ElementComposition, ICompositionObjectAnimations, RenderSurfaces,
};
use ferroui_base::rendering::{IRenderer, RenderLoop};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize, Point, Rect, Ref, Size, Vector, Visual};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Border, Control, NavigationView, NavigationViewItem, NavigationViewPaneDisplayMode};
use ferroui_themes_winui::WinUITheme;
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

struct Surface(Arc<dyn IWriteableBitmapImpl>);
impl IPlatformRenderSurface for Surface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl IFramebufferPlatformSurface for Surface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let bitmap = self.0.clone();
        Rc::new(FuncFramebufferRenderTarget::new(move || bitmap.lock()))
    }
}

struct Sidebar {
    root: Ref<TestRenderRoot>,
    renderer: Rc<CompositingRenderer>,
    bitmap: Arc<dyn IWriteableBitmapImpl>,
    nav: Ref<NavigationView>,
    home: Ref<NavigationViewItem>,
    basic: Ref<NavigationViewItem>,
    slider: Ref<NavigationViewItem>,
    table: Ref<NavigationViewItem>,
    scaling: f64,
    // The last field: the scope of the test ends after everything made in it is dropped. The
    // compositor goes with the last of the items, and is released on the thread it was made on.
    _test: TestBase,
}

fn item(text: &str) -> Ref<NavigationViewItem> {
    let item = NavigationViewItem::new();
    item.set_content(Some(Rc::new(text.to_owned())));
    item
}

fn indicator(item: &Ref<NavigationViewItem>) -> Ref<Control> {
    // Exclude child items' templates when finding a parent's own indicator.
    item.clone()
        .upcast::<Visual>()
        .get_visual_descendants()
        .filter_map(|v| v.cast::<Control>())
        .find(|c| c.name().as_deref() == Some("SelectionIndicator"))
        .expect("the real item template has an indicator")
}

impl Sidebar {
    fn new(scaling: f64) -> Self {
        let test = TestBase::new("Composition/NavigationIndicator");
        FerroLocator::current_mutable()
            .bind::<dyn ferroui_base::animation::IGlobalClock>()
            .to_constant(MediaContext::instance().clock());
        let host = Border::new();
        host.set_width(560.0);
        host.set_height(640.0);
        host.set_background(Some(Brushes::black()));
        host.styles().add(WinUITheme::new().as_style());
        let nav = NavigationView::new();
        nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
        nav.set_is_pane_open(true);
        let home = item("Home");
        let basic = item("Basic Input");
        let slider = item("Slider");
        for child in [item("Buttons"), item("CheckBox"), slider.clone(), item("ToggleSwitch")] {
            basic.menu_items().add(Some(Control::boxed(child)));
        }
        basic.set_is_expanded(true);
        let collections = item("Collections & Data");
        let table = item("TableView");
        for child in [item("ItemsRepeater"), table.clone(), item("TreeDataGrid")] {
            collections.menu_items().add(Some(Control::boxed(child)));
        }
        collections.set_is_expanded(true);
        for root_item in [&home, &basic, &collections] {
            nav.menu_items().add(Some(Control::boxed(root_item)));
        }
        host.set_child(Some(nav.clone().upcast()));
        let factory = FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();
        let bitmap = factory.create_writeable_bitmap(
            PixelSize::new((560.0 * scaling) as i32, (640.0 * scaling) as i32),
            Vector::new(96.0 * scaling, 96.0 * scaling),
            factory.default_pixel_format(),
            factory.default_alpha_format(),
        );
        let surface: Arc<dyn IPlatformRenderSurface> = Arc::new(Surface(bitmap.clone()));
        let surfaces: RenderSurfaces = Arc::new(move || vec![surface.clone()]);
        let compositor = Compositor::with_scheduler(
            RenderLoop::from_timer(Arc::new(ManualRenderTimer::new())),
            None,
            true,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );
        let root = TestRenderRoot::new(scaling);
        let renderer = CompositingRenderer::new(&root.source(), &compositor, surfaces);
        root.initialize(&renderer, &host.upcast());
        renderer.start();
        let scene = Self {
            root,
            renderer,
            bitmap,
            nav,
            home,
            basic,
            slider,
            table,
            scaling,
            _test: test,
        };
        scene.flush();
        for visual in scene.nav.clone().upcast::<Visual>().get_visual_descendants() {
            if let Some(rect) = visual
                .cast::<Rectangle>()
                .filter(|r| r.name().as_deref() == Some("SelectionIndicator"))
            {
                rect.set_fill(Some(Brushes::magenta()));
            }
        }
        scene.flush();
        scene
    }
    fn flush(&self) {
        self.root.source().layout_root().layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(None);
        self.renderer.paint(Rect::from_size(Size::new(560.0, 640.0)));
    }
    fn transition(&self, from: &Ref<NavigationViewItem>, to: &Ref<NavigationViewItem>) {
        self.nav.set_selected_item(Some(Control::boxed(from)));
        self.flush();
        self.nav.set_selected_item(Some(Control::boxed(to)));
        self.flush();
    }
    fn seek(&self, progress: f32) {
        for node in self.nav.clone().upcast::<Visual>().get_visual_descendants() {
            if let Some(control) = node
                .cast::<Control>()
                .filter(|r| r.name().as_deref() == Some("SelectionIndicator"))
            {
                let visual = ElementComposition::get_element_visual(&control).unwrap();
                for property in [
                    "Offset.X",
                    "Scale.X",
                    "CenterPoint.X",
                    "Offset.Y",
                    "Scale.Y",
                    "CenterPoint.Y",
                    "Opacity",
                ] {
                    if let Some(controller) = visual.try_get_animation_controller(property) {
                        controller.pause();
                        controller.set_progress(progress);
                    }
                }
            }
        }
        self.flush();
        self.flush();
    }
    fn image(&self, name: &str) -> Rgba32Image {
        let dir = get_output_directory().join("Composition/NavigationIndicator");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}-{}x.png", self.scaling));
        let mut file = std::fs::File::create(&path).unwrap();
        self.bitmap
            .save(&mut file, &PngBitmapEncoderOptions::default().into())
            .unwrap();
        Rgba32Image::load(&path)
    }
    fn position(&self, item: &Ref<NavigationViewItem>) -> Point {
        indicator(item)
            .transform_to_visual(&self.root)
            .unwrap()
            .transform(Point::new(0.0, 0.0))
    }
}
impl Drop for Sidebar {
    fn drop(&mut self) {
        self.renderer.dispose();
        self.root.set_child(None);
        self.root.release();
        self.bitmap.dispose();
    }
}
fn magenta(p: [u8; 4]) -> bool {
    p[0] > 150 && p[2] > 150 && p[1] < 100
}
fn at(image: &Rgba32Image, s: f64, x: f64, y: f64) -> [u8; 4] {
    image.pixels[(y * s) as usize * image.width + (x * s) as usize]
}

#[test]
fn sidebar_leaf_and_expanded_parent_indicators_are_continuous_with_round_caps() {
    for scale in [1.0, 2.0] {
        for (case, reverse) in [("leaf", false), ("leaf", true), ("parent", false), ("parent", true)] {
            let scene = Sidebar::new(scale);
            let (from, to) = if case == "leaf" {
                (&scene.home, &scene.table)
            } else {
                (&scene.basic, &scene.slider)
            };
            let (from, to) = if reverse { (to, from) } else { (from, to) };
            let a = scene.position(from);
            let b = scene.position(to);
            let y0 = a.y.min(b.y);
            let y1 = a.y.max(b.y) + indicator(to).bounds().height;
            let x = a.x + indicator(from).bounds().width / 2.0;
            scene.transition(from, to);
            scene.seek(1.0 / 3.0 + 0.000001);
            let image = scene.image(&format!("{case}-{reverse}"));
            for py in ((y0 + 3.0) * scale).ceil() as usize..((y1 - 3.0) * scale).floor() as usize {
                assert!(
                    magenta(image.pixels[py * image.width + (x * scale) as usize]),
                    "clipped {case} reverse={reverse} at scale={scale}, y={}",
                    py as f64 / scale
                );
            }
            // A scale-stretched round cap becomes a long needle. Two logical pixels
            // inside either end must already have the full middle column coverage.
            assert!(
                magenta(at(&image, scale, x, y0 + 2.0)),
                "top cap elongated: {case} reverse={reverse}, scale={scale}"
            );
            assert!(
                magenta(at(&image, scale, x, y1 - 2.0)),
                "bottom cap elongated: {case} reverse={reverse}, scale={scale}"
            );
            // The viewport's clip must still hold even though items permit overflow.
            let viewport = scene
                .nav
                .clone()
                .upcast::<Visual>()
                .get_visual_descendants()
                .filter_map(|v| v.cast::<ferroui_controls::presenters::ScrollContentPresenter>())
                .next()
                .unwrap();
            assert!(viewport.clip_to_bounds());
        }
    }
}

#[test]
fn sidebar_caps_keep_their_radius_when_the_indicator_stretches() {
    for scale in [1.0, 2.0] {
        let scene = Sidebar::new(scale);
        // Isolate cap rendering from the separate item-clipping regression.
        for node in scene.nav.clone().upcast::<Visual>().get_visual_descendants() {
            if let Some(item) = node.cast::<NavigationViewItem>() {
                item.set_clip_to_bounds(false);
            }
        }
        let a = scene.position(&scene.home);
        let b = scene.position(&scene.table);
        scene.transition(&scene.home, &scene.table);
        scene.seek(1.0 / 3.0 + 0.000001);
        let image = scene.image("caps");
        assert!(
            magenta(at(&image, scale, a.x + 0.5, a.y + 3.0)),
            "top cap stretched at {scale}x"
        );
        assert!(
            magenta(at(
                &image,
                scale,
                a.x + 0.5,
                b.y + indicator(&scene.table).bounds().height - 3.0
            )),
            "bottom cap stretched at {scale}x"
        );
        for (x, y) in [
            (a.x + 0.1, a.y + 0.1),
            (a.x + 2.9, a.y + 0.1),
            (a.x + 0.1, b.y + indicator(&scene.table).bounds().height - 0.1),
        ] {
            assert!(
                at(&image, scale, x, y)[0] < 230,
                "rounded cap became square at {scale}x"
            );
        }
    }
}

#[test]
fn sidebar_viewport_still_clips_an_overflowing_indicator() {
    for scale in [1.0, 2.0] {
        let scene = Sidebar::new(scale);
        scene.nav.set_selected_item(Some(Control::boxed(&scene.home)));
        scene.flush();
        let marker = indicator(&scene.home);
        let viewport = marker
            .clone()
            .upcast::<Visual>()
            .get_visual_ancestors()
            .find_map(|v| v.cast::<ferroui_controls::presenters::ScrollContentPresenter>())
            .unwrap();
        let bounds = Rect::from_size(viewport.bounds().size())
            .transform_to_aabb(viewport.transform_to_visual(&scene.root).unwrap());
        let visual = ElementComposition::get_element_visual(&marker).unwrap();
        visual.set_scale(ferroui_base::Vector3D::new(1.0, 100.0, 1.0));
        visual.set_offset(ferroui_base::Vector3D::new(
            marker.bounds().x,
            marker.bounds().y - 500.0,
            0.0,
        ));
        scene.flush();
        scene.flush();
        let image = scene.image("viewport");
        let x = scene.position(&scene.home).x + 1.5;
        assert!(magenta(at(&image, scale, x, bounds.y + 3.0)));
        assert!(
            !magenta(at(&image, scale, x, bounds.y - 2.0)),
            "overflow above viewport at {scale}x"
        );
        assert!(
            !magenta(at(&image, scale, x, bounds.bottom() + 2.0)),
            "overflow below viewport at {scale}x"
        );
    }
}

#[test]
fn sidebar_indicator_keeps_live_brushes_and_radius_changes_during_animation() {
    let scene = Sidebar::new(2.0);
    let a = scene.position(&scene.home);
    let b = scene.position(&scene.table);
    scene.transition(&scene.home, &scene.table);
    scene.seek(1.0 / 3.0 + 0.000001);
    let rect = indicator(&scene.table).cast::<Rectangle>().unwrap();
    let brush = ferroui_base::media::SolidColorBrush::with_color(ferroui_base::media::Colors::MAGENTA);
    let fill: Rc<dyn ferroui_base::media::IBrush> = brush.clone().into();
    rect.set_fill(Some(fill.clone()));
    scene.flush();
    brush.set_color(ferroui_base::media::Colors::RED);
    scene.flush();
    scene.flush();
    assert!(Rc::ptr_eq(&rect.fill().unwrap(), &fill));
    let image = scene.image("live-brush");
    let middle = at(&image, 2.0, a.x + 1.5, (a.y + b.y) / 2.0);
    assert!(
        middle[0] > 200 && middle[1] < 30 && middle[2] < 30,
        "live brush did not update: {middle:?}"
    );
    assert!(
        at(&image, 2.0, a.x + 0.1, a.y + 0.1)[0] < 230,
        "live brush lost the rounded cap"
    );
    rect.set_radius_y(0.0);
    scene.flush();
    scene.flush();
    let image = scene.image("live-radius");
    let corner = at(&image, 2.0, a.x + 0.1, a.y + 0.1);
    assert!(
        corner[0] > 200 && corner[1] < 30 && corner[2] < 30,
        "radius change did not update: {corner:?}"
    );
    assert!(ElementComposition::get_element_visual(&rect)
        .unwrap()
        .try_get_animation_controller("Scale.Y")
        .is_some());
}

#[test]
fn travelling_sidebar_indicator_does_not_capture_clicks_on_other_rows() {
    let scene = Sidebar::new(2.0);
    scene.transition(&scene.home, &scene.table);
    scene.seek(1.0 / 3.0 + 0.000001);
    let marker = scene.position(&scene.slider);
    // The renderer also exposes unfiltered visual hit testing. Pointer routing
    // applies InputElement visibility, as in InputElement::input_hit_test.
    let filter = |visual: &Visual| {
        visual.is_visible()
            && visual
                .downcast_ref::<ferroui_base::input::InputElement>()
                .is_some_and(|input| input.is_hit_test_visible() && input.is_effectively_enabled())
    };
    let hit = scene
        .renderer
        .hit_tester()
        .unwrap()
        .hit_test_first(Point::new(marker.x + 1.5, marker.y + 8.0), &scene.root, Some(&filter))
        .expect("the row under the travelling indicator is hit");
    assert!(
        hit == scene.slider.clone().upcast::<Visual>()
            || hit
                .get_visual_ancestors()
                .any(|v| v == scene.slider.clone().upcast::<Visual>()),
        "travelling indicator intercepted the underlying row"
    );
}

#[test]
#[ignore = "local before/after pixel benchmark"]
fn benchmark_sidebar_pixels() {
    for scale in [1.0, 2.0] {
        for mode in ["hierarchy", "caps"] {
            for reverse in [false, true] {
                let scene = Sidebar::new(scale);
                if mode == "caps" {
                    for node in scene.nav.clone().upcast::<Visual>().get_visual_descendants() {
                        if let Some(item) = node.cast::<NavigationViewItem>() {
                            item.set_clip_to_bounds(false);
                        }
                    }
                }
                let (from, to) = if reverse {
                    (&scene.table, &scene.home)
                } else {
                    (&scene.home, &scene.table)
                };
                let a = scene.position(from);
                let b = scene.position(to);
                scene.transition(from, to);
                scene.seek(1.0 / 3.0 + 0.000001);
                let image = scene.image(&format!("benchmark-{mode}-{reverse}"));
                let top = a.y.min(b.y);
                let bottom = a.y.max(b.y) + indicator(to).bounds().height;
                let x = a.x + indicator(from).bounds().width / 2.0;
                let mut gap = 0usize;
                let mut longest = 0usize;
                let mut blue = 0usize;
                let mut full_width = None;
                let first = (top * scale).ceil() as usize;
                let last = (bottom * scale).floor() as usize;
                for y in first..last {
                    if magenta(image.pixels[y * image.width + (x * scale) as usize]) {
                        blue += 1;
                        gap = 0;
                    } else {
                        gap += 1;
                        longest = longest.max(gap);
                    }
                    if full_width.is_none()
                        && magenta(image.pixels[y * image.width + ((a.x + 0.5) * scale) as usize])
                        && magenta(image.pixels[y * image.width + ((a.x + 2.5) * scale) as usize])
                    {
                        full_width = Some((y as f64 / scale) - top);
                    }
                }
                println!("SIDEBAR_PIXELS {{\"scale\":{scale},\"mode\":\"{mode}\",\"reverse\":{reverse},\"span_rows\":{},\"blue_rows\":{blue},\"longest_gap_dip\":{},\"cap_full_width_inset_dip\":{}}}",last-first,longest as f64/scale,full_width.map_or("null".to_owned(),|x|x.to_string()));
            }
        }
    }
}

#[test]
fn sidebar_menu_to_footer_uses_local_motion_in_separate_viewports() {
    for scale in [1.0, 2.0] {
        let scene = Sidebar::new(scale);
        let settings = scene.nav.settings_item();
        scene.transition(&scene.home, &settings);
        scene.seek(0.25);
        let image = scene.image("menu-to-footer");
        let x = ((scene.position(&scene.home).x + 1.5) * scale) as usize;
        let rows = (0..image.height)
            .filter(|y| magenta(image.pixels[y * image.width + x]))
            .count();
        assert!(
            rows <= (32.0 * scale) as usize,
            "indicator stretched across separate viewports: {rows} rows at {scale}x"
        );
    }
}

#[test]
fn top_navigation_keeps_rectangle_styles_and_horizontal_round_caps() {
    for scale in [1.0, 2.0] {
        let scene = Sidebar::new(scale);
        let items: Vec<_> = scene
            .nav
            .clone()
            .upcast::<Visual>()
            .get_visual_descendants()
            .filter_map(|v| v.cast::<NavigationViewItem>())
            .collect();
        for item in items {
            item.set_is_expanded(false);
        }
        scene.flush();
        scene.nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
        scene.flush();
        let marker = indicator(&scene.home);
        assert_eq!(marker.bounds().height, 3.0, "Rectangle template styles did not apply");
        let a = scene.position(&scene.home);
        let b = scene.position(&scene.basic);
        scene.transition(&scene.home, &scene.basic);
        scene.seek(1.0 / 3.0 + 0.000001);
        let image = scene.image("top-navigation");
        let y = a.y + 1.5;
        for x in ((a.x + 3.0) * scale).ceil() as usize
            ..((b.x + indicator(&scene.basic).bounds().width - 3.0) * scale).floor() as usize
        {
            assert!(
                magenta(image.pixels[(y * scale) as usize * image.width + x]),
                "horizontal bar clipped at {scale}x"
            );
        }
        assert!(
            magenta(at(&image, scale, a.x + 3.0, a.y + 0.5)),
            "horizontal cap stretched at {scale}x"
        );
        assert!(
            at(&image, scale, a.x + 0.1, a.y + 0.1)[0] < 230,
            "horizontal cap became square at {scale}x"
        );
    }
}

#[test]
fn sidebar_caps_do_not_pop_when_animation_returns_to_cached_drawing() {
    let scene = Sidebar::new(2.0);
    scene.transition(&scene.home, &scene.table);
    scene.seek(0.995);
    let late = scene.image("cap-handoff-late");
    scene.seek(1.0);
    let idle = scene.image("cap-handoff-idle");
    let p = scene.position(&scene.table);
    let size = indicator(&scene.table).bounds().size();
    let mut largest = 0u8;
    for y in (p.y * 2.0) as usize..((p.y + size.height) * 2.0) as usize {
        for x in (p.x * 2.0) as usize..((p.x + size.width) * 2.0) as usize {
            for c in 0..3 {
                largest = largest.max(late.pixels[y * late.width + x][c].abs_diff(idle.pixels[y * idle.width + x][c]));
            }
        }
    }
    assert!(
        largest <= 16,
        "cap changed at the cached-drawing handoff: channel delta {largest}"
    );
}
