//! Source CadFeatureTests adaptations and explicit native ownership/input cases.
use super::*;
use crate::controls::{Ribbon, RibbonButton, RibbonTab};
use ferroui_base::{
    input::{
        IPointer, KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs,
        PointerType,
    },
    threading::Dispatcher,
};
use ferroui_controls::Window;
use serde_json::{Value, json};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/floating.json")).unwrap()
}
fn bits(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn pair(v: &Value) -> Point {
    Point::new(bits(&v[0]), bits(&v[1]))
}
fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_FLOAT_ISOLATED").as_deref() == Ok(name) {
        return false;
    }
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_FLOAT_ISOLATED", name)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    true
}
fn setup() -> (Ref<Ribbon>, Ref<RibbonGroup>, Ref<Window>) {
    let r = Ribbon::new();
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    r.set_vertical_alignment(VerticalAlignment::Top);
    let t = RibbonTab::new();
    t.set_id(Some("home".into()));
    t.set_header(Some("Home".into()));
    let g = RibbonGroup::new();
    g.set_id(Some("tools".into()));
    g.set_header(Some("Tools".into()));
    let b = RibbonButton::new();
    b.set_label(Some("Cut".into()));
    g.items().add(Some(b.upcast()));
    t.groups().add(Some(g.clone()));
    r.tabs().add(Some(t));
    let t = RibbonTab::new();
    t.set_id(Some("insert".into()));
    t.set_header(Some("Insert".into()));
    r.tabs().add(Some(t));
    let w = crate::ribbon_tests::mount(&r);
    w.set_width(800.);
    w.set_height(600.);
    w.measure(Size::new(800., 600.));
    w.arrange(Rect::new(0., 0., 800., 600.));
    Dispatcher::ui_thread().run_jobs(None);
    (r, g, w)
}
fn canonical(v: &Value) -> Value {
    match v {
        Value::Number(n) => json!(n.as_f64().unwrap()),
        Value::Array(a) => Value::Array(a.iter().map(canonical).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, v)| (k.clone(), canonical(v))).collect())
        }
        _ => v.clone(),
    }
}
fn snapshot(g: &RibbonGroup) -> Value {
    let p = g.floating.panel.borrow().clone();
    let pos = g.floating_position();
    let presenter = g.parts.borrow().presenter.clone();
    json!({"floating":g.is_floating(),"open":g.is_floating_panel_open(),"position":[pos.x,pos.y],"offset":p.as_ref().map(|p|vec![p.popup.horizontal_offset(),p.popup.vertical_offset()]).unwrap_or(vec![0.,0.]),"visible":g.is_visible(),"original":presenter.is_some_and(|p|p.child().is_some_and(|c|c.ptr_eq(&g.items_panel))),"host":g.panel_in_float(),"slide":p.as_ref().is_some_and(|p|p.slide_out.child().is_some_and(|c|c.ptr_eq(&g.slide_out_panel))),"header":p.as_ref().and_then(|p|p.header.text()).map(|s|s.to_string())})
}
fn handler(f: impl Fn() + 'static) -> MarkupDelegate {
    MarkupDelegate::new(move |_| {
        f();
        None
    })
}
#[test]
fn unchanged_floating_clamp_numeric_cases() {
    for (i, c) in corpus()["corpus"]["clamp"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let size = pair(&c["size"]);
        let w = bits(&c["width"]);
        let p = clamp_position(
            pair(&c["position"]),
            Size::new(size.x, size.y),
            if w > 0. { w } else { 120. },
        );
        let expected = pair(&c["result"]);
        assert_eq!(
            [p.x.to_bits(), p.y.to_bits()],
            [expected.x.to_bits(), expected.y.to_bits()],
            "source case {i}"
        );
    }
}
#[test]
fn source_float_suspend_repeat_return_same_panels_and_event_order() {
    const NAME: &str = "controls::ribbon_group::floating::tests::source_float_suspend_repeat_return_same_panels_and_event_order";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, g, w) = setup();
        assert!(!r.can_float_groups());
        assert!(corpus()["corpus"]["emptyIdentity"].as_bool().unwrap());
        assert!(Rc::ptr_eq(
            &r.floating_groups(),
            &Ribbon::new().floating_groups()
        ));
        assert_eq!(g.floating_position(), Point::new(120., 160.));
        let log = Rc::new(RefCell::new(Vec::<&str>::new()));
        let l = log.clone();
        g.add_floating_changed(Some(handler(move || l.borrow_mut().push("floating"))));
        let l = log.clone();
        r.add_group_floating_changed(Some(handler(move || l.borrow_mut().push("root-floating"))));
        let l = log.clone();
        r.add_state_changed(Some(handler(move || l.borrow_mut().push("state"))));
        let panel = g.items_panel();
        let original = panel.get_logical_parent().unwrap();
        let item = g.items().get(0).unwrap();
        let c = corpus();
        let expected = c["corpus"]["snapshots"].as_array().unwrap();
        g.float(Some(Point::new(30., 40.)));
        assert_eq!(snapshot(&g), canonical(&expected[0]));
        assert!(panel.ptr_eq(&g.items_panel()));
        assert!(g.items_panel().children().get(0).ptr_eq(&item));
        g.float(Some(Point::new(90., 100.)));
        assert_eq!(snapshot(&g), canonical(&expected[1]));
        g.suspend_float();
        assert_eq!(snapshot(&g), canonical(&expected[2]));
        g.show_float();
        assert_eq!(snapshot(&g), canonical(&expected[3]));
        g.return_to_ribbon();
        assert_eq!(snapshot(&g), canonical(&expected[4]));
        assert!(
            g.items_panel()
                .get_logical_parent()
                .unwrap()
                .ptr_eq(&original)
        );
        let source = c["corpus"]["log"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .filter(|s| matches!(*s, "floating" | "root-floating"))
            .collect::<Vec<_>>();
        assert_eq!(
            log.borrow()
                .iter()
                .copied()
                .filter(|s| *s != "state")
                .collect::<Vec<_>>(),
            source
        );
        assert_eq!(
            &*log.borrow(),
            &[
                "floating",
                "root-floating",
                "state",
                "floating",
                "root-floating",
                "state"
            ]
        );
        w.close();
    });
}
#[test]
fn source_cad_float_tab_switch_visibility_suspend_reload_and_return_all_snapshot() {
    const NAME: &str = "controls::ribbon_group::floating::tests::source_cad_float_tab_switch_visibility_suspend_reload_and_return_all_snapshot";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, g, w) = setup();
        r.set_can_float_groups(true);
        let slide = RibbonButton::new();
        slide.set_label(Some("Expanded".into()));
        g.slide_out_items().add(Some(slide.clone().upcast()));
        let slide_panel = g.slide_out_panel.clone();
        let items = g.items_panel();
        g.float(Some(Point::new(80., 320.)));
        assert!(g.is_floating_panel_open());
        assert!(!g.is_visible());
        let list = r.floating_groups();
        assert!(list.get(0).unwrap().ptr_eq(&g));
        assert!(
            g.floating
                .panel
                .borrow()
                .as_ref()
                .unwrap()
                .slide_out
                .child()
                .unwrap()
                .ptr_eq(&slide_panel)
        );
        assert!(slide_panel.children().get(0).ptr_eq(&slide));
        assert!(items.get_visual_root().is_some());
        assert!(r.select_tab_id("insert"));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(g.is_floating_panel_open());
        r.set_is_visible(false);
        assert!(g.is_floating());
        assert!(!g.is_floating_panel_open());
        r.set_is_visible(true);
        assert!(g.is_floating_panel_open());
        r.suspend_popups();
        assert!(!g.is_floating_panel_open());
        assert!(g.panel_in_float());
        r.resume_popups();
        assert!(g.is_floating_panel_open());
        w.set_content(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!g.is_floating_panel_open());
        assert!(g.is_floating());
        w.set_content(Some(Control::boxed(r.clone())));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(g.is_floating_panel_open());
        r.return_all_panels_to_ribbon();
        assert!(!g.is_floating());
        assert!(g.is_visible());
        assert_eq!(r.floating_groups().count(), 0);
        assert!(list.get(0).unwrap().ptr_eq(&g));
        assert!(g.items_panel().ptr_eq(&items));
        assert!(g.slide_out_panel.ptr_eq(&slide_panel));
        w.close();
    });
}
fn press(
    handle: &Control,
    w: &Window,
    pointer: Rc<dyn IPointer>,
    point: Point,
    left: bool,
    source: Option<Ref<Control>>,
) -> PointerPressedEventArgs {
    let e = PointerPressedEventArgs::new(
        source.clone().unwrap_or_else(|| handle.to_ref()),
        pointer,
        w,
        point,
        0,
        PointerPointProperties {
            is_left_button_pressed: left,
            ..PointerPointProperties::NONE
        },
        KeyModifiers::NONE,
        1,
    );
    if let Some(source) = source {
        // Explicit native route preserves a distinct source for the WinUI
        // OriginalSource guard; Interactive::raise_event resets Source itself.
        handle
            .build_event_route(InputElement::pointer_pressed_event())
            .raise_event(&source.upcast(), &e);
    } else {
        handle.raise_event(&e);
    }
    e
}
fn move_pointer(
    handle: &Control,
    w: &Window,
    pointer: Rc<dyn IPointer>,
    point: Point,
) -> PointerEventArgs {
    let e = PointerEventArgs::new(
        Some(InputElement::pointer_moved_event()),
        handle.to_ref(),
        pointer,
        Some(w),
        point,
        0,
        PointerPointProperties::NONE,
        KeyModifiers::NONE,
    );
    handle.raise_event(&e);
    e
}
#[test]
fn source_caption_drag_strict_threshold_capture_and_original_button_guard() {
    const NAME: &str = "controls::ribbon_group::floating::tests::source_caption_drag_strict_threshold_capture_and_original_button_guard";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, g, w) = setup();
        r.set_can_float_groups(true);
        let caption = g.parts.borrow().caption.clone().unwrap();
        let pointer = Pointer::new(7, PointerType::Mouse, true);
        let p = press(
            &caption,
            &w,
            pointer.clone(),
            Point::new(100., 100.),
            true,
            None,
        );
        assert!(!p.handled());
        assert!(pointer.captured().unwrap().ptr_eq(&caption));
        move_pointer(&caption, &w, pointer.clone(), Point::new(124., 116.));
        assert!(!g.is_floating());
        let width = g.bounds().width;
        move_pointer(&caption, &w, pointer.clone(), Point::new(124.001, 100.));
        assert!(g.is_floating());
        assert!(pointer.captured().is_none());
        assert_eq!(
            g.floating_position(),
            Point::new(max(0., 124.001 - width / 2.), 90.)
        );
        g.return_to_ribbon();
        let button = Button::new();
        press(
            &caption,
            &w,
            pointer.clone(),
            Point::new(100., 100.),
            true,
            Some(button.upcast()),
        );
        move_pointer(&caption, &w, pointer.clone(), Point::new(200., 200.));
        assert!(!g.is_floating());
        press(
            &caption,
            &w,
            pointer.clone(),
            Point::new(100., 100.),
            false,
            None,
        );
        move_pointer(&caption, &w, pointer.clone(), Point::new(200., 200.));
        assert!(!g.is_floating());
        press(
            &caption,
            &w,
            pointer.clone(),
            Point::new(100., 100.),
            true,
            None,
        );
        pointer.platform_capture_lost();
        assert!(g.floating.caption_start.get().is_none());
        assert!(!g.is_floating());
        w.close();
    });
}
#[test]
fn source_floating_drag_clears_capture_before_single_state_notification() {
    const NAME: &str = "controls::ribbon_group::floating::tests::source_floating_drag_clears_capture_before_single_state_notification";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, g, w) = setup();
        g.float(Some(Point::new(30., 40.)));
        let popup = g.floating.panel.borrow().clone().unwrap();
        let chrome = popup.popup.child().unwrap().cast::<Border>().unwrap();
        let layout = chrome.child().unwrap().cast::<Grid>().unwrap();
        let handle = layout.children().get(0);
        let pointer = Pointer::new(9, PointerType::Mouse, true);
        let states = Rc::new(Cell::new(0));
        let s = states.clone();
        let p = pointer.clone();
        r.add_state_changed(Some(handler(move || {
            assert!(p.captured().is_none());
            s.set(s.get() + 1);
        }))); // cleared before notification, even capture-lost reentrancy
        let pressed = press(
            &handle,
            &w,
            pointer.clone(),
            Point::new(100., 100.),
            true,
            None,
        );
        assert!(pressed.handled());
        let moved = move_pointer(&handle, &w, pointer.clone(), Point::new(112.5, 97.25));
        assert!(moved.handled());
        assert_eq!(g.floating_position(), Point::new(42.5, 37.25));
        pointer.platform_capture_lost();
        assert!(!g.floating.dragging.get());
        assert_eq!(states.get(), 1);
        assert!(pointer.captured().is_none());
        let before = g.floating_position();
        move_pointer(&handle, &w, pointer.clone(), Point::new(200., 200.));
        assert_eq!(g.floating_position(), before);
        g.return_to_ribbon();
        w.close();
    });
}
#[test]
fn source_floating_retemplate_removes_old_caption_handlers_and_keeps_popup_identity() {
    const NAME: &str = "controls::ribbon_group::floating::tests::source_floating_retemplate_removes_old_caption_handlers_and_keeps_popup_identity";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, g, w) = setup();
        r.set_can_float_groups(true);
        let old = g.parts.borrow().caption.clone().unwrap();
        let original_template = g.template();
        let stale_route = old.build_event_route(InputElement::pointer_pressed_event());
        let pointer = Pointer::new(10, PointerType::Mouse, true);
        g.float(Some(Point::new(50., 60.)));
        let popup = g.floating.panel.borrow().clone().unwrap();
        let panel = g.items_panel();
        g.set_template(None);
        g.apply_template();
        assert!(corpus()["corpus"]["staleCaptionIgnored"].as_bool().unwrap());
        let e = PointerPressedEventArgs::new(
            old.clone(),
            pointer.clone(),
            &w,
            Point::new(100., 100.),
            0,
            PointerPointProperties {
                is_left_button_pressed: true,
                ..PointerPointProperties::NONE
            },
            KeyModifiers::NONE,
            1,
        );
        stale_route.raise_event(&old.clone().upcast(), &e);
        assert!(
            g.floating.caption_start.get().is_none(),
            "stale source callback must read current missing caption part"
        );
        assert!(pointer.captured().is_none());
        assert!(g.is_floating_panel_open());
        assert!(g.panel_in_float());
        assert!(panel.get_logical_parent().unwrap().ptr_eq(&popup.items));
        press(
            &old,
            &w,
            pointer.clone(),
            Point::new(100., 100.),
            true,
            None,
        );
        move_pointer(&old, &w, pointer.clone(), Point::new(200., 200.));
        assert!(g.floating.caption_start.get().is_none());
        assert!(pointer.captured().is_none());
        g.set_template(original_template);
        g.apply_template();
        assert!(
            g.floating
                .panel
                .borrow()
                .as_ref()
                .unwrap()
                .popup
                .ptr_eq(&popup.popup)
        );
        assert!(g.panel_in_float());
        g.return_to_ribbon();
        assert!(
            g.parts
                .borrow()
                .presenter
                .as_ref()
                .unwrap()
                .child()
                .unwrap()
                .ptr_eq(&panel)
        );
        w.close();
    });
}
#[test]
fn floating_changed_reentrancy_rechecks_state_and_unretained_native_graph_releases() {
    const NAME: &str = "controls::ribbon_group::floating::tests::floating_changed_reentrancy_rechecks_state_and_unretained_native_graph_releases";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let weak = {
            let (r, g, w) = setup();
            let changes = Rc::new(Cell::new(0));
            let c = changes.clone();
            let weak = g.downgrade();
            let handler = handler(move || {
                c.set(c.get() + 1);
                if let Some(g) = weak.upgrade()
                    && g.is_floating()
                {
                    g.return_to_ribbon();
                }
            });
            g.add_floating_changed(Some(handler.clone()));
            g.float(Some(Point::new(80., 90.)));
            assert!(!g.is_floating());
            assert!(!g.is_floating_panel_open());
            assert_eq!(changes.get(), 2);
            g.remove_floating_changed(Some(handler));
            g.float(Some(Point::new(80., 90.)));
            assert!(g.is_floating_panel_open());
            let weak = (
                r.downgrade(),
                g.downgrade(),
                g.items_panel().downgrade(),
                g.floating
                    .panel
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .popup
                    .downgrade(),
            );
            r.return_all_panels_to_ribbon();
            w.close();
            w.set_content(None);
            weak
        };
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.0.upgrade().is_none(), "unretained floating ribbon");
        assert!(weak.1.upgrade().is_none(), "unretained floating group");
        assert!(weak.2.upgrade().is_none(), "unretained actual items panel");
        assert!(weak.3.upgrade().is_none(), "unretained floating popup");
    });
}
/// Source regression: release every handle capture, including an external one.
#[test]
fn source_release_pointer_captures_includes_external_capture() {
    const NAME: &str = "controls::ribbon_group::floating::tests::source_release_pointer_captures_includes_external_capture";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        assert!(
            corpus()["corpus"]["externalCaptureReleased"]
                .as_bool()
                .unwrap()
        );
        let (r, g, w) = setup();
        r.set_can_float_groups(true);
        let h = g.parts.borrow().caption.clone().unwrap();
        let p = Pointer::new(11, PointerType::Mouse, true);
        let external = Pointer::new(12, PointerType::Touch, false);
        press(&h, &w, p.clone(), Point::new(100., 100.), true, None);
        external.capture(Some(&h.clone().upcast()));
        move_pointer(&h, &w, p, Point::new(125., 100.));
        assert!(g.is_floating());
        assert!(
            external.captured().is_none(),
            "source ReleasePointerCaptures must release externally installed capture"
        );
        w.close();
    });
}
#[test]
fn source_float_before_load_defers_hosting_and_plain_position_changes_defer_offsets() {
    const NAME: &str = "controls::ribbon_group::floating::tests::source_float_before_load_defers_hosting_and_plain_position_changes_defer_offsets";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let group = RibbonGroup::new();
        group.float(None);
        assert!(group.is_floating());
        assert!(!group.is_floating_panel_open());
        assert_eq!(group.floating_position(), Point::new(120., 160.));
        assert!(!group.is_visible());
        let r = Ribbon::new();
        r.set_is_application_button_visible(false);
        r.set_is_display_options_button_visible(false);
        let tab = RibbonTab::new();
        tab.set_header(Some("Home".into()));
        tab.groups().add(Some(group.clone()));
        r.tabs().add(Some(tab));
        let w = crate::ribbon_tests::mount(&r);
        assert!(group.is_floating_panel_open());
        assert!(
            group
                .parts
                .borrow()
                .presenter
                .as_ref()
                .unwrap()
                .child()
                .is_none()
        );
        let parts = group.floating.panel.borrow().clone().unwrap();
        let old = Point::new(
            parts.popup.horizontal_offset(),
            parts.popup.vertical_offset(),
        );
        group.set_floating_position(Point::new(33., 44.));
        assert_eq!(
            Point::new(
                parts.popup.horizontal_offset(),
                parts.popup.vertical_offset()
            ),
            old
        );
        r.suspend_popups();
        r.resume_popups();
        assert_eq!(
            Point::new(
                parts.popup.horizontal_offset(),
                parts.popup.vertical_offset()
            ),
            Point::new(33., 44.)
        );
        let snapshot = r.floating_groups();
        assert!(!Rc::ptr_eq(&snapshot, &r.floating_groups()));
        r.return_all_panels_to_ribbon();
        assert!(snapshot.get(0).unwrap().ptr_eq(&group));
        w.close();
    });
}
#[test]
fn native_floating_pointer_coordinates_bridge_popup_presentation_to_window() {
    const NAME: &str = "controls::ribbon_group::floating::tests::native_floating_pointer_coordinates_bridge_popup_presentation_to_window";
    if isolated(NAME) {
        return;
    }
    // Use a separate native-popup headless session. The ordinary library
    // assembly defaults to overlays, whose presentation source is the window.
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
        FerroTestApplicationAttribute, FerroTestAssembly, HeadlessUnitTestSession,
    };
    fn build() -> ferroui_controls::AppBuilder {
        ferroui_controls::AppBuilder::configure::<Application>().use_headless(
            FerroHeadlessPlatformOptions {
                overlay_popups: false,
                use_shared_mouse_device: Some(true),
                ..Default::default()
            },
        )
    }
    const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
        name: "FerroRibbon.FloatingPlatformPopup.Tests",
        test_application: Some(FerroTestApplicationAttribute::new(build)),
        test_isolation: None,
    };
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                crate::register_types();
                let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
                Application::current().unwrap().styles().add(theme.clone());
                let (_r, g, w) = setup();
                g.float(Some(Point::new(80., 90.)));
                Dispatcher::ui_thread().run_jobs(None);
                let parts = g.floating.panel.borrow().clone().unwrap();
                let chrome = parts.popup.child().unwrap().cast::<Border>().unwrap();
                let bar = chrome
                    .child()
                    .unwrap()
                    .cast::<Grid>()
                    .unwrap()
                    .children()
                    .get(0);
                let popup_root = bar.get_visual_root().unwrap();
                bar.get_layout_manager().unwrap().execute_layout_pass();
                assert!(!parts.popup.is_using_overlay_layer());
                assert!(!popup_root.ptr_eq(&w.get_visual_root().unwrap()));
                let pointer = Pointer::new(13, PointerType::Mouse, true);
                let e = PointerEventArgs::new(
                    Some(InputElement::pointer_moved_event()),
                    bar.clone(),
                    pointer,
                    Some(&popup_root),
                    Point::new(10., 15.),
                    0,
                    PointerPointProperties::NONE,
                    KeyModifiers::NONE,
                );
                let position = g.pointer_in_window(&e);
                assert_eq!(e.get_position(None), Point::new(10., 15.));
                let popup_origin = popup_root
                    .presentation_source()
                    .unwrap()
                    .point_to_screen(Point::default())
                    .unwrap();
                let root_origin = w
                    .to_ref()
                    .upcast::<ferroui_base::Visual>()
                    .presentation_source()
                    .unwrap()
                    .point_to_screen(Point::default())
                    .unwrap();
                assert_eq!(
                    position,
                    Point::new(
                        10. + f64::from(popup_origin.x - root_origin.x),
                        15. + f64::from(popup_origin.y - root_origin.y)
                    )
                );
                assert_ne!(
                    position,
                    e.get_position(None),
                    "popup-local origin must not replace source window coordinates"
                );
                {
                    let e=PointerEventArgs::new(Some(InputElement::pointer_moved_event()),bar.clone(),Pointer::new(14,PointerType::Mouse,true),Some(&popup_root),Point::new(10.25,15.75),0,PointerPointProperties::NONE,KeyModifiers::NONE);
                    let expected=Point::new(10.25+f64::from(popup_origin.x-root_origin.x),15.75+f64::from(popup_origin.y-root_origin.y));
                    assert_eq!(g.pointer_in_window(&e),expected,"source XamlRoot pointer coordinates must retain fractional DIP positions across native popup roots");
                }
                g.return_to_ribbon();
                w.close();
                Application::current().unwrap().styles().remove(theme);
            },
            ferroui_base::threading::CancellationToken::none(),
        )
        .wait()
        .expect("native-popup headless case");
}
