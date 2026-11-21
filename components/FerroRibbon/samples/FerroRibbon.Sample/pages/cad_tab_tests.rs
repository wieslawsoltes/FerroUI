use super::{
    CadPage,
    cad_action_tests::{mount, run},
};
use crate::cad::{CadDrawingCanvas, cad_view_model::CadDocumentTab};
use ferroribbon::{controls::RibbonToggleButton, theming::RibbonTheme};
use ferroui_base::{
    Point, Ref,
    input::{MouseButton, RawInputModifiers},
    media::{Color, IBrush},
    threading::Dispatcher,
};
use ferroui_controls::{Border, Button, Control, TextBlock, Window};
use ferroui_headless::HeadlessWindowExtensions;
use serde_json::Value;
use std::rc::Rc;

fn click(window: &Ref<Window>, control: &Control) {
    window.update_layout();
    let point = control
        .translate_point(
            Point::new(control.bounds().width * 0.5, control.bounds().height * 0.5),
            window,
        )
        .unwrap();
    window.mouse_move(point, RawInputModifiers::NONE);
    window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
    window.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
    Dispatcher::ui_thread().run_jobs(None);
}
fn same_brush(a: &Rc<dyn IBrush>, b: &Rc<dyn IBrush>) -> bool {
    match (a.as_object(), b.as_object()) {
        (Some(a), Some(b)) => a.to_ref().ptr_eq(&b.to_ref()),
        _ => Rc::ptr_eq(a, b),
    }
}
fn verify(page: &Ref<CadPage>, state: &Value, created: bool) {
    let tabs = page.document_tabs.get().unwrap();
    assert_eq!(
        page.view_model().selected_file_tab(),
        CadDocumentTab::ALL[state["selectedFile"].as_u64().unwrap() as usize]
    );
    assert_eq!(
        page.view_model().selected_layout_tab(),
        CadDocumentTab::ALL[state["selectedLayout"].as_u64().unwrap() as usize]
    );
    let toggle = page.named::<RibbonToggleButton>("ModelToggle");
    assert_eq!(toggle.is_checked(), state["model"].as_bool());
    assert_eq!(toggle.label().as_deref(), state["label"].as_str());
    for (index, ((tab, choice), expected)) in tabs
        .iter()
        .zip(state["tabs"].as_array().unwrap())
        .enumerate()
    {
        let padding = tab.padding();
        assert_eq!(
            [padding.left, padding.top, padding.right, padding.bottom],
            std::array::from_fn::<_, 4, _>(|i| expected["padding"][i].as_f64().unwrap())
        );
        assert_eq!(tab.min_width(), expected["minimum"].as_f64().unwrap());
        if expected["height"].is_null() {
            assert!(tab.height().is_nan())
        } else {
            assert_eq!(tab.height(), expected["height"].as_f64().unwrap())
        };
        let radius = tab.corner_radius();
        assert_eq!(
            [
                radius.top_left,
                radius.top_right,
                radius.bottom_right,
                radius.bottom_left
            ],
            std::array::from_fn::<_, 4, _>(|i| expected["radius"][i].as_f64().unwrap())
        );
        let background = tab.background().unwrap();
        if !created && expected["background"]["R"] == 7 {
            let theme: Rc<dyn IBrush> =
                RibbonTheme::get_brush(Some(page), Some("RibbonCommandBarBackgroundBrush"))
                    .unwrap()
                    .unwrap()
                    .into();
            assert!(same_brush(&background, &theme));
        } else {
            let expected = &expected["background"];
            assert_eq!(
                background.as_solid_color_brush().unwrap().color(),
                Color::from_argb(
                    expected["A"].as_u64().unwrap() as u8,
                    expected["R"].as_u64().unwrap() as u8,
                    expected["G"].as_u64().unwrap() as u8,
                    expected["B"].as_u64().unwrap() as u8
                )
            );
        }
        if expected["text"].is_string() {
            let label = page.named::<TextBlock>(&format!("{}Label", choice.control_name()));
            assert_eq!(label.text().as_deref(), expected["text"].as_str());
            assert_eq!(
                i64::from(label.font_weight().0),
                expected["weight"].as_i64().unwrap()
            );
            if !created {
                let brush: Rc<dyn IBrush> =
                    RibbonTheme::get_brush(Some(page), expected["foreground"].as_str())
                        .unwrap()
                        .unwrap()
                        .into();
                assert!(same_brush(&label.foreground().unwrap(), &brush));
            }
        }
        for (other, _) in tabs {
            let j = tabs.iter().position(|(t, _)| t.ptr_eq(other)).unwrap();
            assert_eq!(
                same_brush(&background, &other.background().unwrap()),
                state["sameBackground"][index][j].as_bool().unwrap()
            );
        }
    }
    for (name, expected) in ["CadFileAdd", "CadLayoutAdd"]
        .into_iter()
        .zip(state["adds"].as_array().unwrap())
    {
        let button = page.named::<Button>(name);
        assert_eq!(button.width(), expected["Width"].as_f64().unwrap());
        assert_eq!(
            button.theme().is_some(),
            expected["styled"].as_bool().unwrap()
        );
        assert_eq!(
            ferroui_controls::automation::AutomationProperties::get_name(&button).as_deref(),
            Some("New")
        );
    }
}

#[test]
fn source_tab_selection_geometry_brush_identity_and_native_taps() {
    run(|| {
        let fixture: Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/cad-live.json")).unwrap();
        let page = CadPage::new();
        let mut window = None;
        let original: Vec<_> = page
            .document_tabs
            .get()
            .unwrap()
            .iter()
            .map(|(tab, _)| tab.clone())
            .collect();
        for case in fixture["tabs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["style"] == "valid" && c["missing"] == false)
        {
            let kind = case["kind"].as_str().unwrap();
            match kind {
                "created" => (),
                "updated" => page.update_document_tab_visuals(),
                "tap" => {
                    let window = window.get_or_insert_with(|| mount(&page));
                    click(window, &original[case["index"].as_u64().unwrap() as usize]);
                }
                "external-toggle" => {
                    page.named::<RibbonToggleButton>("ModelToggle")
                        .set_is_checked(Some(false));
                    page.named::<RibbonToggleButton>("ModelToggle")
                        .set_label(Some("PAPER".into()));
                    page.update_document_tab_visuals();
                }
                "replaced-child" => {
                    original[0].set_child(Border::new());
                    click(window.as_ref().unwrap(), &original[2]);
                }
                _ => panic!("unmapped source case {case}"),
            }
            verify(&page, &case["state"], kind == "created");
            for ((tab, _), original) in page.document_tabs.get().unwrap().iter().zip(&original) {
                assert!(tab.ptr_eq(original));
            }
        }
        let window = window.unwrap();
        for name in ["CadFileAdd", "CadLayoutAdd"] {
            click(&window, &page.named::<Button>(name));
        }
        assert_eq!(
            page.named::<ferroui_controls::StackPanel>("FileTabs")
                .children()
                .count(),
            4
        );
        assert_eq!(
            page.named::<ferroui_controls::StackPanel>("LayoutTabs")
                .children()
                .count(),
            4
        );
        assert_eq!(
            page.view_model().selected_file_tab(),
            CadDocumentTab::FloorPlan
        );
        let close_glyph = original[1]
            .child()
            .unwrap()
            .cast::<ferroui_controls::StackPanel>()
            .unwrap()
            .children()
            .get(1);
        click(&window, &close_glyph);
        assert_eq!(
            page.view_model().selected_file_tab(),
            CadDocumentTab::Drawing
        );
        assert_eq!(
            page.named::<ferroui_controls::StackPanel>("FileTabs")
                .children()
                .count(),
            4,
            "The source close glyph selects its tab but does not close it"
        );
        window.close();
    });
}

#[test]
fn tab_retheme_replaces_canvas_and_transparent_brushes_without_rebuilding_controls() {
    run(|| {
        let page = CadPage::new();
        let window = mount(&page);
        let drawing = page.named::<Border>("CadFileTab1");
        let start = page.named::<Border>("CadFileTab0");
        let original_canvas = drawing.background().unwrap();
        let original_transparent = start.background().unwrap();
        page.update_document_tab_visuals();
        assert!(!same_brush(
            &original_canvas,
            &drawing.background().unwrap()
        ));
        assert!(!same_brush(
            &original_transparent,
            &start.background().unwrap()
        ));
        for light in [true, false, true] {
            page.set_light_theme(light);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                drawing
                    .background()
                    .unwrap()
                    .as_solid_color_brush()
                    .unwrap()
                    .color(),
                page.named::<CadDrawingCanvas>("Canvas").canvas_color()
            );
            assert!(drawing.ptr_eq(&page.named::<Border>("CadFileTab1")));
        }
        let retained = drawing.clone();
        let weak = page.downgrade();
        let model = Rc::downgrade(&page.view_model());
        let descendants: Vec<_> = page
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<Control>())
            .map(|control| (control.name(), control.downgrade()))
            .collect();
        window.set_content(None);
        window.close();
        drop(page);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
        // A retained native child may retain its inherited DataContext. This is
        // normal ownership, not a callback cycle: releasing those children must
        // release the presentation model, while none of them roots the page.
        drop(drawing);
        drop(start);
        drop(retained);
        drop(window);
        Dispatcher::ui_thread().run_jobs(None);
        if let Some(vm) = model.upgrade() {
            let live: Vec<_> = descendants
                .iter()
                .filter_map(|(name, control)| control.upgrade().map(|_| name.clone()))
                .collect();
            eprintln!(
                "CAD retained model refs: {}; live descendants: {}; first named: {:?}",
                Rc::strong_count(&vm),
                live.len(),
                live.iter().flatten().take(12).collect::<Vec<_>>()
            );
        }
        assert!(model.upgrade().is_none());
    });
}

#[test]
fn native_tab_add_button_style_requires_a_control_theme_resource() {
    run(|| {
        let app = ferroui_controls::Application::current().unwrap();
        let key = "RibbonChromeButtonStyle".into();
        let original = app.resources().try_get_value(&key);
        let page = CadPage::new();
        for wrong in [None, Some(Rc::new(42_i32) as ferroui_base::BoxedValue)] {
            app.resources().set(key.clone(), wrong);
            // The original XAML itself needs a valid style when it loads. These
            // cases exercise the later CreateAddButton conditional in isolation.
            for name in ["CadFileAdd", "CadLayoutAdd"] {
                page.named::<Button>(name).set_theme(None);
            }
            page.apply_document_tab_add_button_theme();
            for name in ["CadFileAdd", "CadLayoutAdd"] {
                assert!(page.named::<Button>(name).theme().is_none());
            }
        }
        match original {
            Some(value) => app.resources().set(key, value),
            None => {
                app.resources().remove(&key);
            }
        }
    });
}
