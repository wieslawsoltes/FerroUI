use crate::{
    controls::DebugOverlayAdorner, helpers::debug_overlay_helper::DebugOverlayHelper,
    DockDebugOverlayExtensions,
};
use ferrodock_ferro::controls::DockControl;
use ferrodock_settings::DockProperties;
use ferroui_base::{
    input::{InputElement, Key, KeyEventArgs, KeyGesture, KeyModifiers},
    media::{Brushes, Color, Colors, DrawingGroup, GeometryDrawing},
    threading::{CancellationToken, Dispatcher},
    Point, Rect, Ref,
};
use ferroui_controls::{
    primitives::{AdornerLayer, VisualLayerManager},
    AppBuilder, Application, Border, Canvas, Control, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-debug-overlay",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn drain() {
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
    }
}
struct Scene {
    window: Ref<Window>,
    root: Ref<Canvas>,
    target: Ref<Border>,
}
impl Scene {
    fn new() -> Self {
        let root = Canvas::new();
        let target = Border::new();
        target.set_width(80.0);
        target.set_height(35.0);
        target.set_background(Some(Brushes::transparent()));
        Canvas::set_left(&target, 17.0);
        Canvas::set_top(&target, 23.0);
        root.children().add(target.clone());
        let layers = VisualLayerManager::new();
        layers.set_child(Some(root.clone().upcast::<Control>()));
        let window = Window::new();
        window.set_width(400.0);
        window.set_height(300.0);
        window.set_content(Some(Rc::new(layers)));
        window.show();
        drain();
        Self {
            window,
            root,
            target,
        }
    }
    fn layer(&self) -> Ref<AdornerLayer> {
        AdornerLayer::get_adorner_layer(&self.root).unwrap()
    }
    fn adorner(&self) -> Ref<DebugOverlayAdorner> {
        self.layer()
            .children()
            .snapshot()
            .iter()
            .find_map(|child| child.cast())
            .unwrap()
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
    }
}
fn draw(adorner: &DebugOverlayAdorner) -> Ref<DrawingGroup> {
    let group = DrawingGroup::new();
    let mut context = group.open();
    adorner.render(&mut context);
    context.dispose();
    group
}
fn rectangles(group: &DrawingGroup) -> Vec<Ref<GeometryDrawing>> {
    group
        .children()
        .iter()
        .filter_map(|drawing| drawing.cast())
        .collect()
}
fn flags(control: &Control, mask: u8) {
    DockProperties::set_is_dock_target(control, mask & 1 != 0);
    DockProperties::set_is_drag_area(control, mask & 2 != 0);
    DockProperties::set_is_drop_area(control, mask & 4 != 0);
}

#[test]
fn actual_source_all_area_flags_and_hover_truth_table_keep_rectangles_order_pens_and_fills() {
    run(|| {
        let scene = Scene::new();
        let helper = DebugOverlayHelper::new();
        helper.add_overlay(scene.root.clone().upcast());
        let adorner = scene.adorner();
        assert!(!adorner.is_hit_test_visible());
        for mask in 0..8 {
            flags(&scene.target, mask);
            for hover in [false, true] {
                adorner.set_pointer_over(hover.then(|| scene.target.clone().upcast()));
                let actual = rectangles(&draw(&adorner));
                let mut index = 0;
                for (bit, pen_color, fill_color) in [
                    (1, Colors::RED, Color::from_argb(80, 255, 0, 0)),
                    (2, Colors::GREEN, Color::from_argb(80, 0, 255, 0)),
                    (4, Colors::BLUE, Color::from_argb(80, 0, 0, 255)),
                ] {
                    if mask & bit == 0 {
                        continue;
                    }
                    let drawing = &actual[index];
                    let pen = drawing.pen().unwrap();
                    assert_eq!(pen.thickness(), 1.0);
                    assert_eq!(
                        pen.brush().unwrap().as_solid_color_brush().unwrap().color(),
                        pen_color
                    );
                    assert_eq!(
                        drawing
                            .brush()
                            .map(|b| b.as_solid_color_brush().unwrap().color()),
                        hover.then_some(fill_color)
                    );
                    assert_eq!(
                        drawing.geometry().unwrap().bounds(),
                        Rect::new(17.0, 23.0, 80.0, 35.0)
                    );
                    index += 1;
                }
                assert_eq!(actual.len(), index);
            }
        }
        helper.remove_overlay(&scene.root);
    });
}

#[test]
fn actual_unchanged_dotnet_render_corpus_matches_all_source_rectangles() {
    run(|| {
        let scene = Scene::new();
        let helper = DebugOverlayHelper::new();
        helper.add_overlay(scene.root.clone().upcast());
        let adorner = scene.adorner();
        let corpus = include_str!("../../../tests/Dock/data/debug-overlay-render.tsv");
        let mut rows = 0;
        for line in corpus
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
        {
            let fields: Vec<_> = line.split('\t').collect();
            flags(&scene.target, fields[0].parse().unwrap());
            adorner.set_pointer_over((fields[1] == "1").then(|| scene.target.clone().upcast()));
            let actual = rectangles(&draw(&adorner));
            let expected: Vec<_> = fields[2].split(';').filter(|field| *field != "-").collect();
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected) {
                let fields: Vec<_> = expected.split(',').collect();
                let pen = actual.pen().unwrap();
                assert_eq!(
                    pen.brush().unwrap().as_solid_color_brush().unwrap().color(),
                    Color::from_uint32(u32::from_str_radix(fields[0], 16).unwrap())
                );
                assert_eq!(pen.thickness(), fields[1].parse::<f64>().unwrap());
                let fill = if fields[2] == "-" {
                    None
                } else {
                    Some(Color::from_uint32(
                        u32::from_str_radix(fields[2], 16).unwrap(),
                    ))
                };
                assert_eq!(
                    actual
                        .brush()
                        .map(|brush| brush.as_solid_color_brush().unwrap().color()),
                    fill
                );
                let bounds: Vec<f64> = fields[3..]
                    .iter()
                    .map(|field| field.parse().unwrap())
                    .collect();
                assert_eq!(
                    actual.geometry().unwrap().bounds(),
                    Rect::new(bounds[0], bounds[1], bounds[2], bounds[3])
                );
            }
            rows += 1;
        }
        assert_eq!(rows, 16);
        helper.remove_overlay(&scene.root);
    });
}

#[test]
fn source_pointer_hit_order_skips_unmarked_controls_and_uses_the_topmost_marked_hit() {
    run(|| {
        let scene = Scene::new();
        flags(&scene.target, 1);
        let upper = Border::new();
        upper.set_width(80.0);
        upper.set_height(35.0);
        upper.set_background(Some(Brushes::transparent()));
        Canvas::set_left(&upper, 17.0);
        Canvas::set_top(&upper, 23.0);
        scene.root.children().add(upper.clone());
        let helper = DebugOverlayHelper::new();
        helper.add_overlay(scene.root.clone().upcast());
        drain();
        scene
            .window
            .mouse_move(Point::new(30.0, 30.0), Default::default());
        drain();
        assert!(rectangles(&draw(&scene.adorner()))[0].brush().is_some());
        flags(&upper, 2);
        scene
            .window
            .mouse_move(Point::new(31.0, 30.0), Default::default());
        drain();
        let drawn = rectangles(&draw(&scene.adorner()));
        assert!(drawn[0].brush().is_none());
        assert!(drawn[1].brush().is_some());
        helper.remove_overlay(&scene.root);
    });
}

#[test]
fn source_footer_uses_native_object_display_and_white_twelve_point_text() {
    run(|| {
        use ferroui_base::media::{Drawing, GlyphRunDrawing};
        fn glyphs(drawing: Ref<Drawing>, result: &mut Vec<Ref<GlyphRunDrawing>>) {
            if let Some(glyph) = drawing.cast::<GlyphRunDrawing>() {
                result.push(glyph);
            } else if let Some(group) = drawing.cast::<DrawingGroup>() {
                for child in group.children().iter() {
                    glyphs(child, result);
                }
            }
        }
        let scene = Scene::new();
        let helper = DebugOverlayHelper::new();
        helper.add_overlay(scene.root.clone().upcast());
        drain();
        let adorner = scene.adorner();
        adorner.set_pointer_over(Some(scene.target.clone().upcast()));
        for empty in [
            None,
            Some(Rc::new(String::new()) as ferroui_base::BoxedValue),
        ] {
            scene.target.set_data_context(empty);
            assert!(draw(&adorner).children().is_empty());
        }
        scene
            .target
            .set_data_context(Some(Rc::new("Dock3".to_string())));
        let mut actual = Vec::new();
        glyphs(draw(&adorner).upcast(), &mut actual);
        assert!(!actual.is_empty());
        for drawing in actual {
            assert_eq!(
                drawing
                    .foreground()
                    .unwrap()
                    .as_solid_color_brush()
                    .unwrap()
                    .color(),
                Colors::WHITE
            );
            assert_eq!(drawing.glyph_run().unwrap().font_rendering_em_size(), 12.0);
        }
        helper.remove_overlay(&scene.root);
    });
}

#[test]
fn helper_same_root_repeated_add_preserves_source_old_adorners_and_explicit_subscription_ownership()
{
    run(|| {
        let scene = Scene::new();
        let helper = DebugOverlayHelper::new();
        let weak = Rc::downgrade(&helper);
        helper.add_overlay(scene.root.clone().upcast());
        let first = scene.adorner();
        helper.add_overlay(scene.root.clone().upcast());
        assert_eq!(scene.layer().children().count(), 2);
        helper.remove_overlay(&scene.root);
        assert_eq!(scene.layer().children().count(), 1);
        assert!(first.parent().is_some());
        drop(helper);
        assert!(
            weak.upgrade().is_some(),
            "the source CLR LayoutUpdated -= removes only one repeated subscription"
        );
    });
}

#[test]
fn helper_normal_removal_releases_strong_callbacks_and_keeps_the_adorned_value() {
    run(|| {
        let scene = Scene::new();
        let helper = DebugOverlayHelper::new();
        let weak = Rc::downgrade(&helper);
        helper.add_overlay(scene.root.clone().upcast());
        let adorner = scene.adorner();
        drop(helper);
        let owner = weak.upgrade().unwrap();
        owner.remove_overlay(&scene.root);
        drop(owner);
        assert!(weak.upgrade().is_none());
        assert!(adorner.parent().is_none());
        assert!(AdornerLayer::get_adorned_element(&adorner)
            .unwrap()
            .ptr_eq(&scene.root));
    });
}

#[test]
fn extension_metadata_retains_both_source_overloads_and_runtime_namespace() {
    run(|| {
        use ferroui_base::metadata::MarkupTyped;
        let metadata = DockDebugOverlayExtensions::MARKUP;
        let overloads: Vec<_> = metadata
            .methods
            .iter()
            .filter(|method| method.name == "AttachDockDebugOverlay")
            .collect();
        assert_eq!(overloads.len(), 2);
        assert!(overloads.iter().all(|method| method.parameters.len() == 2));
        assert_eq!(
            DockDebugOverlayExtensions::TYPE.full_name(),
            "FerroDock.Ferro.Diagnostics.DockDebugOverlayExtensions"
        );
    });
}

#[test]
fn application_overload_attaches_only_current_desktop_windows_and_ignores_future_windows() {
    run(|| {
        use ferroui_controls::{
            application_lifetimes::{
                ClassicDesktopStyleApplicationLifetime, ISetupApplicationLifetime,
            },
            ShutdownMode,
        };
        let desktop = ClassicDesktopStyleApplicationLifetime::new();
        desktop.set_shutdown_mode(ShutdownMode::OnExplicitShutdown);
        desktop.before_app_init();
        let first = Scene::new();
        first.root.children().add(DockControl::new());
        drain();
        let app = Application::new();
        app.set_application_lifetime(Some(desktop.clone()));
        DockDebugOverlayExtensions::attach_dock_debug_overlay_application(&app, None);
        press(&first.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(first.layer().children().count(), 1);
        let future = Scene::new();
        future.root.children().add(DockControl::new());
        drain();
        press(&future.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(future.layer().children().count(), 0);
        press(&first.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(first.layer().children().count(), 0);
        desktop.dispose();
    });
}

#[derive(Default)]
struct SingleLifetime(std::cell::RefCell<Option<Ref<Control>>>);
impl ferroui_controls::application_lifetimes::IApplicationLifetime for SingleLifetime {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_single_view_application_lifetime(
        &self,
    ) -> Option<&dyn ferroui_controls::application_lifetimes::ISingleViewApplicationLifetime> {
        Some(self)
    }
}
impl ferroui_controls::application_lifetimes::ISingleViewApplicationLifetime for SingleLifetime {
    fn main_view(&self) -> Option<Ref<Control>> {
        self.0.borrow().clone()
    }
    fn set_main_view(&self, value: Option<Ref<Control>>) {
        self.0.replace(value);
    }
}
#[test]
fn application_overload_requires_a_top_level_single_view_and_accepts_absent_lifetimes() {
    run(|| {
        use ferroui_controls::application_lifetimes::ISingleViewApplicationLifetime;
        let scene = Scene::new();
        scene.root.children().add(DockControl::new());
        drain();
        let app = Application::new();
        DockDebugOverlayExtensions::attach_dock_debug_overlay_application(&app, None);
        let single = Rc::new(SingleLifetime::default());
        app.set_application_lifetime(Some(single.clone()));
        DockDebugOverlayExtensions::attach_dock_debug_overlay_application(&app, None);
        single.set_main_view(Some(scene.root.clone().upcast()));
        DockDebugOverlayExtensions::attach_dock_debug_overlay_application(&app, None);
        press(&scene.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(scene.layer().children().count(), 0);
        single.set_main_view(Some(scene.window.clone().upcast()));
        DockDebugOverlayExtensions::attach_dock_debug_overlay_application(&app, None);
        press(&scene.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(scene.layer().children().count(), 1);
        press(&scene.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(scene.layer().children().count(), 0);
    });
}
#[test]
fn source_render_excludes_the_adorned_root_and_walks_descendants_even_when_hidden() {
    run(|| {
        let scene = Scene::new();
        flags(&scene.root, 7);
        flags(&scene.target, 1);
        scene.target.set_is_visible(false);
        let helper = DebugOverlayHelper::new();
        helper.add_overlay(scene.root.clone().upcast());
        assert_eq!(rectangles(&draw(&scene.adorner())).len(), 1);
        helper.remove_overlay(&scene.root);
        let adorner = DebugOverlayAdorner::new();
        assert_eq!(draw(&adorner).children().len(), 0);
    });
}
#[test]
fn native_pointer_move_selects_the_first_hit_debug_target_and_exit_clears_fill() {
    run(|| {
        let scene = Scene::new();
        flags(&scene.target, 7);
        let helper = DebugOverlayHelper::new();
        helper.add_overlay(scene.root.clone().upcast());
        drain();
        scene
            .window
            .mouse_move(Point::new(30.0, 30.0), Default::default());
        drain();
        assert!(rectangles(&draw(&scene.adorner()))
            .iter()
            .all(|drawing| drawing.brush().is_some()));
        scene
            .window
            .mouse_move(Point::new(200.0, 200.0), Default::default());
        drain();
        assert!(rectangles(&draw(&scene.adorner()))
            .iter()
            .all(|drawing| drawing.brush().is_none()));
        helper.remove_overlay(&scene.root);
    });
}
fn press(window: &Window, key: Key, modifiers: KeyModifiers, handled: bool) {
    let mut event = KeyEventArgs::new();
    event.set_routed_event(Some(InputElement::key_down_event()));
    event.key = key;
    event.key_modifiers = modifiers;
    event.set_handled(handled);
    window.raise_event(&event);
}
#[test]
fn original_hotkey_toggles_all_current_docks_tracks_dynamic_docks_and_disposes_explicitly() {
    run(|| {
        let scene = Scene::new();
        let dock = DockControl::new();
        dock.set_width(120.0);
        dock.set_height(120.0);
        scene.root.children().add(dock.clone());
        drain();
        let cleanup = DockDebugOverlayExtensions::attach_dock_debug_overlay(
            scene.window.clone().upcast(),
            None,
        );
        press(&scene.window, Key::F8, KeyModifiers::empty(), false);
        assert_eq!(scene.layer().children().count(), 0);
        press(&scene.window, Key::F9, KeyModifiers::empty(), true);
        assert_eq!(scene.layer().children().count(), 0);
        press(&scene.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(scene.layer().children().count(), 1);
        let other = DockControl::new();
        scene.root.children().add(other.clone());
        drain();
        assert_eq!(scene.layer().children().count(), 2);
        scene.root.children().remove(other);
        drain();
        // The source helper resolves the layer AFTER detach, leaving that adorner
        // in its old layer while the manager removes its tracked helper.
        assert_eq!(scene.layer().children().count(), 2);
        press(&scene.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(scene.layer().children().count(), 1);
        press(&scene.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(scene.layer().children().count(), 2);
        cleanup.dispose();
        cleanup.dispose();
        assert_eq!(scene.layer().children().count(), 1);
        press(&scene.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(scene.layer().children().count(), 1);
    });
}
#[test]
fn original_custom_gesture_and_ignored_cleanup_handle_keep_the_hotkey_subscription() {
    run(|| {
        let scene = Scene::new();
        scene.root.children().add(DockControl::new());
        drain();
        DockDebugOverlayExtensions::attach_dock_debug_overlay(
            scene.window.clone().upcast(),
            Some(KeyGesture::new(Key::F8, KeyModifiers::CONTROL)),
        );
        press(&scene.window, Key::F9, KeyModifiers::empty(), false);
        assert_eq!(scene.layer().children().count(), 0);
        press(&scene.window, Key::F8, KeyModifiers::empty(), false);
        assert_eq!(scene.layer().children().count(), 0);
        press(&scene.window, Key::F8, KeyModifiers::CONTROL, false);
        assert_eq!(scene.layer().children().count(), 1);
        press(&scene.window, Key::F8, KeyModifiers::CONTROL, false);
        assert_eq!(scene.layer().children().count(), 0);
    });
}
