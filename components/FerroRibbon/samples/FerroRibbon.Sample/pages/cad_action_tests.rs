use super::{
    CadPage,
    cad_actions::{coordinate_text, initial_cursor},
};
use crate::cad::{
    CadDrawingCanvas,
    cad_view_model::{CadPageAction, CadSceneAction},
};
use ferroribbon::controls::{RibbonButton, RibbonToggleButton};
use ferroui_base::{
    Point, Ref,
    interactivity::RoutedEventArgs,
    threading::{CancellationToken, Dispatcher},
};
use ferroui_controls::{Application, Button, Control, TextBlock, Window};
use ferroui_headless::HeadlessUnitTestSession;
use ferroui_headless::HeadlessWindowExtensions;
use serde_json::{Value, json};
pub(super) fn run(f: impl FnOnce() + Send + 'static) {
    let session = HeadlessUnitTestSession::start_new(super::cad_page_tests::build);
    let result = session
        .dispatch(
            move || {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                f();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/cad-live.json")).unwrap()
}
fn click(control: &Control) {
    control.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}
pub(super) fn mount(page: &Ref<CadPage>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1440.);
    window.set_height(960.);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window
}

#[test]
fn original_scene_toggle_callbacks_and_clean_screen_match_source_cases() {
    run(|| {
        for case in fixture()["scene"].as_array().unwrap() {
            let kind = case["kind"].as_str().unwrap();
            if kind != "toggle" && kind != "clean" && kind != "captions" {
                continue;
            }
            let page = CadPage::new();
            let canvas = page.named::<CadDrawingCanvas>("Canvas");
            let state = &case["state"];
            if kind == "toggle" {
                let name = case["name"].as_str().unwrap();
                let toggle = page.named::<RibbonToggleButton>(name);
                toggle.set_is_checked(case["check"].as_bool());
                click(&toggle);
            } else if kind == "clean" {
                page.named::<RibbonToggleButton>("FileTabsToggle")
                    .set_is_checked(case["tabs"].as_bool());
                page.view_model()
                    .page_action()
                    .execute(Some(&CadPageAction::Scene(CadSceneAction::CleanScreen(
                        case["clean"].as_bool().unwrap(),
                    ))));
            } else {
                page.cad_ribbon()
                    .set_show_group_captions(case["check"].as_bool().unwrap());
            }
            let visible: Vec<_> = [
                "UcsIcon",
                "ViewCube",
                "NavBar",
                "CommandLine",
                "FileTabsRow",
                "LayoutTabs",
                "TitleBar",
            ]
            .into_iter()
            .map(|name| {
                if page.named::<Control>(name).is_visible() {
                    "Visible"
                } else {
                    "Collapsed"
                }
            })
            .collect();
            assert_eq!(json!(visible), state["visibility"], "{case}");
            assert_eq!(json!(canvas.is_grid_visible()), state["grid"]);
            assert_eq!(json!(canvas.show_lineweights()), state["lineweights"]);
            assert_eq!(json!(canvas.is_pan_mode()), state["pan"]);
            assert_eq!(
                json!(page.named::<RibbonToggleButton>("ModelToggle").label()),
                state["model"]
            );
            assert_eq!(
                json!(page.cad_ribbon().show_group_captions()),
                state["captions"]
            );
            assert_eq!(
                json!(
                    page.named::<RibbonToggleButton>("PanelTitlesToggle")
                        .is_checked()
                ),
                state["panelChecked"]
            );
            let mode = page.cad_ribbon().visibility_mode();
            assert_eq!(mode.to_string(), state["mode"].as_str().unwrap());
            let expected_light = state["theme"] == "Light";
            assert_eq!(
                page.actual_theme_variant() == Some(ferroui_base::styling::ThemeVariant::light()),
                expected_light
            );
        }
    });
}

#[test]
fn original_coordinate_format_and_page_messages_match_verbatim_callbacks() {
    run(|| {
        let page = CadPage::new();
        for case in fixture()["scene"].as_array().unwrap() {
            match case["kind"].as_str().unwrap() {
                "loaded" => {
                    let point = initial_cursor(
                        f64::from_bits(case["widthBits"].as_u64().unwrap()),
                        f64::from_bits(case["heightBits"].as_u64().unwrap()),
                    );
                    let expected = &case["state"]["cursorBits"];
                    for (value, bits) in [point.x, point.y]
                        .into_iter()
                        .zip(expected.as_array().unwrap())
                    {
                        let wanted = f64::from_bits(bits.as_u64().unwrap());
                        assert!(
                            value.to_bits() == wanted.to_bits()
                                || value.is_nan() && wanted.is_nan()
                        );
                    }
                }
                "coordinates" => {
                    let x = f64::from_bits(case["xBits"].as_u64().unwrap());
                    let y = f64::from_bits(case["yBits"].as_u64().unwrap());
                    assert_eq!(
                        coordinate_text(x, y),
                        case["state"]["coordinates"].as_str().unwrap()
                    );
                    page.view_model()
                        .page_action()
                        .execute(Some(&CadPageAction::Scene(CadSceneAction::CursorMoved(
                            x, y,
                        ))));
                    assert_eq!(
                        page.named::<TextBlock>("Coordinates").text().as_deref(),
                        case["state"]["coordinates"].as_str()
                    );
                }
                "message" => {
                    let text = case["text"].as_str().map(str::to_owned);
                    let action = match case["action"].as_str().unwrap() {
                        "layer" => CadPageAction::LayerCommitted(text),
                        "item" => CadPageAction::ApplicationItem(text),
                        "recent" => CadPageAction::RecentItem(text),
                        "exit" => CadPageAction::Exit,
                        "options" => CadPageAction::Options,
                        _ => panic!("source message kind"),
                    };
                    page.view_model().page_action().execute(Some(&action));
                    let expected = case["state"]["messages"][0]["text"].as_str().unwrap();
                    assert_eq!(
                        page.view_model().history().last().unwrap(),
                        &format!("Command: {expected}")
                    );
                }
                _ => {}
            }
        }
    });
}

#[test]
fn native_footer_layer_and_application_events_reach_reactive_actions_once() {
    run(|| {
        let page = CadPage::new();
        for (name, text) in [
            ("AppMenuExit", "EXIT is disabled in the showcase."),
            ("AppMenuOptions", "OPTIONS"),
        ] {
            let prior = page.view_model().history();
            let before = prior.len();
            let prompt = prior.last().is_some_and(|s| s == "Command:");
            click(&page.named::<Button>(name));
            let history = page.view_model().history();
            assert_eq!(
                history.len(),
                before + 1 - usize::from(prompt),
                "source prompt replacement on first echo"
            );
            assert_eq!(history.last().unwrap(), &format!("Command: {text}"));
        }
        let item = page.application_menu().items().get(2).unwrap();
        let label = item.label().unwrap();
        page.application_menu().invoke(item);
        assert_eq!(
            page.view_model().history().last().unwrap(),
            &format!("Command: Application menu: {label}")
        );
        let recent = page.application_menu().recent_items().get(0).unwrap();
        let title = recent.title().unwrap();
        page.application_menu().invoke_recent(recent);
        assert_eq!(
            page.view_model().history().last().unwrap(),
            &format!("Command: Opening \"{title}\"...")
        );
        let layer = crate::cad::CadLayer::samples()[2].clone();
        page.layer_drop_down()
            .commit(Some(std::rc::Rc::new(layer.clone())));
        assert!(
            page.view_model()
                .history()
                .contains(&format!("Command: Current layer: \"{}\"", layer.name()))
        );
        let theme = page
            .named::<ferroui_controls::Flyout>("AppMenuFlyout")
            .flyout_presenter_theme()
            .unwrap();
        assert_eq!(
            theme.target_type(),
            Some(ferroui_controls::FlyoutPresenter::TYPE)
        );
        assert!(theme.based_on().is_some());
    });
}

#[test]
fn native_loaded_cursor_helpers_float_return_and_application_anchor() {
    run(|| {
        let page = CadPage::new();
        let window = mount(&page);
        let canvas = page.named::<CadDrawingCanvas>("Canvas");
        assert!(page.initialized.get());
        // At first load, the source page seeds the actual canvas crosshair.
        let first = initial_cursor(page.bounds().width, page.bounds().height);
        let seeded: Vec<_> = canvas
            .get_visual_descendants()
            .filter_map(|v| v.cast::<ferroui_controls::shapes::Line>())
            .collect();
        assert!(seeded.iter().any(|line| line.start_point()
            == Point::new(0., first.y.round_ties_even() + 0.5)
            && line.end_point().x == canvas.bounds().width));
        let plot = Point::new(canvas.bounds().width * 0.45, canvas.bounds().height * 0.48);
        let window_point = canvas.translate_point(plot, &window).unwrap();
        window.mouse_move(window_point, ferroui_base::input::RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        let zero = canvas.model_to_screen(0., 0.);
        let expected = coordinate_text(
            (plot.x - zero.x) / canvas.view_scale(),
            (zero.y - plot.y) / canvas.view_scale(),
        );
        assert_eq!(
            page.named::<TextBlock>("Coordinates").text().as_deref(),
            Some(expected.as_str()),
            "Real pointer motion reaches the native CursorMoved event and the view model"
        );
        // Loaded seeds once; a later load must not overwrite a user cursor position.
        canvas.set_crosshair_cursor(Point::new(42., 84.));
        page.on_cad_loaded();
        let lines: Vec<_> = canvas
            .get_visual_descendants()
            .filter_map(|v| v.cast::<ferroui_controls::shapes::Line>())
            .filter(|line| line.stroke_thickness() == 1.)
            .collect();
        assert!(
            lines
                .iter()
                .any(|line| line.start_point() == Point::new(0., 84.5)
                    && line.end_point().x == canvas.bounds().width)
        );
        assert!(
            lines
                .iter()
                .any(|line| line.start_point() == Point::new(42.5, 0.)
                    && line.end_point().y == canvas.bounds().height)
        );
        let panel = page.layers_panel();
        let origin = canvas
            .translate_point(Point::default(), &canvas.visual_root().unwrap())
            .unwrap();
        page.float_layers_panel();
        assert!(panel.is_floating());
        assert_eq!(
            panel.floating_position(),
            Point::new(origin.x + 120., origin.y + 70.)
        );
        let position = panel.floating_position();
        page.float_layers_panel();
        assert_eq!(panel.floating_position(), position);
        click(&page.named::<RibbonButton>("ReturnPanelsButton"));
        assert!(!panel.is_floating());
        click(&page.named::<Button>("CommandClose"));
        assert!(!page.named::<Control>("CommandLine").is_visible());
        assert_eq!(
            page.named::<RibbonToggleButton>("CommandLineToggle")
                .is_checked(),
            Some(false)
        );
        let expected = page.cad_ribbon().next_minimize_state();
        click(&page.named::<RibbonButton>("CycleButton"));
        assert_eq!(page.cad_ribbon().visibility_mode(), expected);
        page.cad_ribbon()
            .set_visibility_mode(ferroribbon_core::RibbonVisibilityMode::AlwaysShow);
        page.show_application_menu();
        assert!(
            page.named::<ferroui_controls::Flyout>("AppMenuFlyout")
                .is_open()
        );
        page.named::<ferroui_controls::Flyout>("AppMenuFlyout")
            .hide();
        window.set_content(None);
        window.close();
    });
}
