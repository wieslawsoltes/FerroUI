use super::SettingsPage;
use crate::{demo_settings::DemoSettings, settings_view_model::SettingsSelection};
use ferroribbon::{localization::RibbonFrameworkCulture, theming::RibbonTheme};
use ferroribbon_core::{
    localization::RibbonStrings,
    theming::{RibbonChromeStyle, RibbonThemePalette, RibbonThemeStyle},
};
use ferroui_base::{
    Ref,
    styling::ThemeVariant,
    threading::{CancellationToken, Dispatcher},
    utilities::CultureInfo,
};
use ferroui_controls::{
    AppBuilder, Application, Button, ComboBox, Control, TextBlock, TextBox, Window,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
// Settings changes the real process-wide Current; independent scenarios must
// not change another concurrently running test's language or formatting data.
fn isolated(name: &str) -> bool {
    let name = name
        .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
        .unwrap_or(name);
    if std::env::var("FERRORIBBON_SETTINGS_CASE").as_deref() == Ok(name) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_SETTINGS_CASE", name)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("running 1 test\n") && output.status.success(),
        "{name}\n{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
fn run(f: impl FnOnce() + Send + 'static) {
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>()
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
    });
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
    serde_json::from_str(include_str!("../../../tests/fixtures/settings-page.json")).unwrap()
}
fn settings() -> Rc<DemoSettings> {
    DemoSettings::new(Rc::new(|| Ok("03:04:05".into())))
}
fn reset() {
    RibbonTheme::apply_with_style(
        Some(RibbonThemePalette::cad()),
        RibbonChromeStyle::Neutral,
        RibbonThemeStyle::Office,
    )
    .unwrap();
    RibbonStrings::set_current(Some(
        RibbonStrings::for_culture(Some(RibbonFrameworkCulture::new(
            CultureInfo::get_culture_info("en"),
        )))
        .unwrap(),
    ))
    .unwrap();
}
fn mount(page: &Ref<SettingsPage>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1200.);
    window.set_height(900.);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window
}
fn text(v: Option<ferroribbon_core::RibbonString>) -> Option<String> {
    v.map(|s| s.to_utf8().unwrap())
}
fn snapshot(page: &SettingsPage, s: &DemoSettings, root: bool, calls: &[String]) -> Value {
    let combo = page.named::<ComboBox>("Palette");
    let palettes = (0..combo.items().count())
        .map(|i| {
            combo
                .items()
                .get_at(i)
                .map(|v| v.downcast_ref::<String>().unwrap().clone())
        })
        .collect::<Vec<_>>();
    let requested = page.get_value(ThemeVariant::requested_theme_variant_property());
    let theme = if requested == Some(ThemeVariant::light()) {
        "Light"
    } else if requested == Some(ThemeVariant::dark()) {
        "Dark"
    } else {
        "Default"
    };
    json!({"state":page.view_model().state_json(),"log":page.view_model().log_text(),"palettes":palettes,"density":s.density().raw(),"layout":s.display_mode().map(|m|m.raw()),"style":s.surface_style().raw(),"overrideName":s.palette_override().and_then(|p|text(p.name.clone())),"paletteName":text(RibbonTheme::palette().name.clone()),"chrome":RibbonTheme::chrome_style().raw(),"theme":if root{Some(theme)}else{None},"culture":text(RibbonStrings::current().culture().name()),"calls":calls,"wordCalls":null})
}
#[test]
fn whole_source_settings_actions_match_native_services_and_view_bindings() {
    if isolated(concat!(
        module_path!(),
        "::whole_source_settings_actions_match_native_services_and_view_bindings"
    )) {
        return;
    }
    run(|| {
        let mut compared = 0;
        let mut explicit_unsupported = 0;
        for case in fixture()["cases"].as_array().unwrap().iter().filter(|c| {
            matches!(
                c["kind"].as_str(),
                Some("initial" | "select" | "layout" | "language")
            )
        }) {
            reset();
            let s = settings();
            let page = SettingsPage::new_with_settings(s.clone());
            let root = case["root"].as_bool().unwrap_or(true);
            let window = root.then(|| mount(&page));
            let calls = Rc::new(RefCell::new(Vec::new()));
            let c = calls.clone();
            let name = case["name"].as_str().unwrap_or("").to_owned();
            let theme = RibbonTheme::changed(move |_, _| {
                let line = match name.as_str() {
                    "Palette" => format!(
                        "palette:{}",
                        text(RibbonTheme::palette().name.clone()).unwrap()
                    ),
                    "Chrome" => format!("chrome:{}", RibbonTheme::chrome_style()),
                    "Surface" => format!("style:{}", RibbonTheme::style()),
                    _ => panic!("Unexpected eager theme update"),
                };
                c.borrow_mut().push(line);
            });
            let result = match case["kind"].as_str().unwrap() {
                "initial" => Ok(()),
                "select" => {
                    let index = case["index"].as_i64().unwrap() as i32;
                    let selection = match case["name"].as_str().unwrap() {
                        "Theme" => SettingsSelection::Theme(index),
                        "Palette" => SettingsSelection::Palette(index),
                        "Chrome" => SettingsSelection::Chrome(index),
                        "Surface" => SettingsSelection::Surface(index),
                        _ => unreachable!(),
                    };
                    page.view_model().apply_selection(selection)
                }
                "layout" => {
                    let count = Rc::new(Cell::new(0));
                    let n = count.clone();
                    let _sub = s.changed(move || n.set(n.get() + 1));
                    for _ in 0..2 {
                        page.view_model()
                            .apply_selection(SettingsSelection::Layout {
                                density_index: case["density"].as_i64().unwrap() as i32,
                                layout_index: case["layout"].as_i64().unwrap() as i32,
                            })
                            .unwrap();
                    }
                    assert_eq!(count.get(), case["notifications"].as_i64().unwrap() as i32);
                    Ok(())
                }
                "language" => page
                    .view_model()
                    .apply_selection(SettingsSelection::Language(
                        case["text"].as_str().map(str::to_owned),
                    )),
                _ => unreachable!(),
            };
            theme.dispose();
            if case["kind"] == "language" && case["text"] == "zz" {
                assert!(
                    result.is_err(),
                    "Unverified source culture construction must remain explicit"
                );
                explicit_unsupported += 1;
            } else {
                assert_eq!(result.is_err(), !case["error"].is_null(), "{case}");
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(
                    snapshot(&page, &s, root, &calls.borrow()),
                    case["state"],
                    "{case}"
                );
                assert_eq!(
                    page.named::<TextBox>("StateJson").text().as_deref(),
                    Some(page.view_model().state_json().as_str())
                );
                assert_eq!(
                    page.named::<TextBlock>("LogText").text(),
                    page.view_model().log_text()
                );
                compared += 1;
            }
            if let Some(window) = window {
                window.close();
            }
            drop(page);
            Dispatcher::ui_thread().run_jobs(None);
        }
        assert_eq!(compared, 79);
        assert_eq!(explicit_unsupported, 1);
    });
}

#[test]
fn settings_constructor_has_no_eager_global_effects_and_actual_combos_forward_selection() {
    if isolated(concat!(
        module_path!(),
        "::settings_constructor_has_no_eager_global_effects_and_actual_combos_forward_selection"
    )) {
        return;
    }
    run(|| {
        reset();
        let s = settings();
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        let sub = RibbonTheme::changed(move |_, _| c.set(c.get() + 1));
        let page = SettingsPage::new_with_settings(s.clone());
        assert_eq!(calls.get(), 0);
        assert_eq!(
            text(RibbonTheme::palette().name.clone()).as_deref(),
            Some("CAD")
        );
        let window = mount(&page);
        assert_eq!(calls.get(), 0);
        sub.dispose();
        for (name, index, expected) in [("Chrome", 1, "Colorful"), ("Surface", 1, "Cad")] {
            page.named::<ComboBox>(name).set_selected_index(index);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                if name == "Chrome" {
                    RibbonTheme::chrome_style().to_string()
                } else {
                    s.surface_style().to_string()
                },
                expected
            );
        }
        page.named::<ComboBox>("Density").set_selected_index(2);
        page.named::<ComboBox>("Layout").set_selected_index(2);
        assert_eq!(s.density().raw(), 2);
        assert_eq!(s.display_mode().unwrap().raw(), 1);
        for (index, name) in [(1, "de"), (2, "fr"), (3, "es"), (4, "pl"), (0, "en")] {
            page.named::<ComboBox>("LanguageBox")
                .set_selected_index(index);
            assert_eq!(
                text(RibbonStrings::current().culture().name()).as_deref(),
                Some(name)
            );
        }
        for name in [
            "Theme",
            "Palette",
            "Chrome",
            "Surface",
            "Density",
            "Layout",
            "LanguageBox",
        ] {
            let combo = page.named::<ComboBox>(name);
            let header = page.named::<TextBlock>(&format!("{name}Header"));
            let label =
                ferroui_controls::automation::AutomationProperties::get_labeled_by(&combo).unwrap();
            assert!(label.ptr_eq(&header));
        }
        page.named::<ComboBox>("Theme").set_selected_index(2);
        assert_eq!(
            page.get_value(ThemeVariant::requested_theme_variant_property()),
            Some(ThemeVariant::dark())
        );
        page.named::<ComboBox>("Theme").set_selected_index(0);
        assert_eq!(
            page.get_value(ThemeVariant::requested_theme_variant_property()),
            None
        );
        window.close();
    });
}

#[test]
fn actual_language_picker_keeps_original_word_table_popup_and_command_usable() {
    if isolated(concat!(
        module_path!(),
        "::actual_language_picker_keeps_original_word_table_popup_and_command_usable"
    )) {
        return;
    }
    run(|| {
        use ferroribbon::controls::{RibbonDropDownButton, RibbonGridPicker};
        use ferroribbon_core::model::RibbonGridSize;
        use ferroui_base::input::{InputElement, Key, KeyEventArgs};
        use ferroui_controls::Flyout;
        reset();
        let s = settings();
        let settings = SettingsPage::new_with_settings(s.clone());
        let settings_window = mount(&settings);
        let word = crate::pages::word_page::WordPage::new_with_settings(s.clone());
        let window = Window::new();
        window.set_width(1200.);
        window.set_height(850.);
        window.set_content(Some(Control::boxed(word.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let button = word.named::<RibbonDropDownButton>("InsertTable");
        let picker = word.named::<RibbonGridPicker>("TablePicker");
        let flyout = button.flyout().unwrap().cast::<Flyout>().unwrap();
        let source = serde_json::from_str::<Value>(include_str!(
            "../../../tests/fixtures/native-numbers.json"
        ))
        .unwrap();
        for (i, (index, language)) in [(1, "de"), (2, "fr"), (3, "es"), (4, "pl"), (0, "en")]
            .into_iter()
            .enumerate()
        {
            settings
                .named::<ComboBox>("LanguageBox")
                .set_selected_index(index);
            word.ribbon()
                .set_selected_tab(Some(word.ribbon().tabs().get(1).unwrap()));
            window.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            flyout.show_at(&button);
            assert!(flyout.is_open());
            picker.highlight(RibbonGridSize::new(1, 2));
            let expected = &source["cultures"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["name"] == language)
                .unwrap()["table"];
            let expected = String::from_utf16(
                &expected
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u16)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            assert!(
                picker
                    .get_visual_descendants()
                    .filter_map(|v| v.cast::<TextBlock>())
                    .any(|v| v.text().as_deref() == Some(&expected)),
                "{language}: {expected}"
            );
            let mut key = KeyEventArgs::new();
            key.key = Key::Enter;
            key.set_routed_event(Some(InputElement::key_down_event()));
            picker.raise_event(&key);
            assert!(key.handled());
            assert!(!flyout.is_open());
            assert_eq!(
                s.log()
                    .borrow()
                    .iter()
                    .filter(|s| s.contains("Inserted 2×1 table"))
                    .count(),
                i + 1
            );
            assert!(
                Control::from_boxed(&flyout.content().unwrap())
                    .unwrap()
                    .ptr_eq(&picker)
            );
        }
        window.close();
        window.set_content(None);
        settings_window.close();
        settings_window.set_content(None);
        s.set_word_ribbon(None);
        Dispatcher::ui_thread().run_jobs(None);
    });
}

#[test]
fn settings_live_log_keeps_source_newest40_and_disposes_page_and_model() {
    if isolated(concat!(
        module_path!(),
        "::settings_live_log_keeps_source_newest40_and_disposes_page_and_model"
    )) {
        return;
    }
    run(|| {
        for case in fixture()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["kind"] == "log")
        {
            reset();
            let s = settings();
            let page = SettingsPage::new_with_settings(s.clone());
            let weak = page.downgrade();
            let model = Rc::downgrade(&page.view_model());
            let window = mount(&page);
            s.log()
                .borrow_mut()
                .extend((0..case["count"].as_u64().unwrap()).map(|i| format!("seed:{i}")));
            page.view_model().refresh_action().execute(None);
            Dispatcher::ui_thread().run_jobs(None);
            let expected = |key: &str| {
                let mut result = case[key].clone();
                if cfg!(windows) {
                    result["log"] = json!(result["log"].as_str().unwrap().replace('\n', "\r\n"));
                }
                result
            };
            assert_eq!(snapshot(&page, &s, true, &[]), expected("loaded"));
            s.write(Some("new")).unwrap();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(snapshot(&page, &s, true, &[]), expected("state"));
            assert_eq!(
                page.named::<TextBlock>("LogText").text(),
                page.view_model().log_text()
            );
            window.set_content(None);
            window.close();
            drop(page);
            Dispatcher::ui_thread().run_jobs(None);
            assert!(weak.upgrade().is_none());
            assert!(model.upgrade().is_none());
            s.write(Some("after disposal")).unwrap();
        }
    });
}

#[test]
fn actual_word_commands_use_public_state_customization_and_palette_apis() {
    if isolated(concat!(
        module_path!(),
        "::actual_word_commands_use_public_state_customization_and_palette_apis"
    )) {
        return;
    }
    run(|| {
        use crate::settings_view_model::SettingsWordAction;
        use ferroribbon::controls::{Ribbon, RibbonCustomizePage};
        reset();
        let s = settings();
        let page = SettingsPage::new_with_settings(s.clone());
        let window = mount(&page);
        for action in [
            SettingsWordAction::ShowState,
            SettingsWordAction::ResetCustomization,
            SettingsWordAction::Customize,
            SettingsWordAction::CommandPalette,
        ] {
            page.view_model().word_action(action).execute(None);
        }
        assert_eq!(page.view_model().state_json(), "Open the Word page first.");
        let ribbon = Ribbon::new();
        s.set_word_ribbon(Some(ribbon.clone()));
        let expected = ribbon.save_state_to_json().unwrap().to_utf8().unwrap();
        page.named::<Button>("ShowState")
            .command()
            .unwrap()
            .execute(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            page.named::<TextBox>("StateJson").text().as_deref(),
            Some(expected.as_str())
        );
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        let subscription = ribbon.customize_requested(move |args| {
            assert_eq!(args.page, RibbonCustomizePage::Ribbon);
            args.handled.set(true);
            c.set(c.get() + 1);
        });
        page.named::<Button>("Customize")
            .command()
            .unwrap()
            .execute(None);
        assert_eq!(calls.get(), 1);
        subscription.dispose();
        page.named::<Button>("ResetCustomization")
            .command()
            .unwrap()
            .execute(None);
        page.named::<TextBox>("StateJson").set_text(Some("edited"));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(page.view_model().state_json(), "edited");

        // The source opens the palette in WordRibbon's root. Exercise the
        // attached branch: a missing WordRibbon only tests the source no-op.
        let word_window = Window::new();
        word_window.set_width(800.);
        word_window.set_height(600.);
        word_window.set_content(Some(Control::boxed(ribbon.clone())));
        word_window.show();
        Dispatcher::ui_thread().run_jobs(None);
        page.named::<Button>("Palette2")
            .command()
            .unwrap()
            .execute(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(window.opened_popups().is_empty());
        assert_eq!(word_window.opened_popups().len(), 1);
        let popup = word_window.opened_popups()[0].clone();
        let weak_popup = popup.downgrade();
        assert!(popup.is_open());
        {
            use ferroui_base::input::{PhysicalKey, RawInputModifiers};
            use ferroui_headless::HeadlessWindowExtensions;
            word_window.key_press_qwerty(PhysicalKey::Escape, RawInputModifiers::NONE);
            word_window.key_release_qwerty(PhysicalKey::Escape, RawInputModifiers::NONE);
        }
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!popup.is_open());
        assert!(word_window.opened_popups().is_empty());
        drop(popup);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak_popup.upgrade().is_none());
        word_window.close();
        s.set_word_ribbon(None);
        window.close();
    });
}

#[test]
fn settings_class_compiled_and_runtime_markup_use_the_same_native_view() {
    run(|| {
        use ferroui_base::{data::core::ValueTypes, utilities::UriRef};
        use ferroui_markup_xaml::FerroXamlLoader;
        use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
        let compiled = FerroXamlLoader::load(
            &UriRef::parse_absolute("ferres://ferroribbon_sample/SettingsPage.xaml")
                .unwrap()
                .into(),
            None,
        )
        .unwrap();
        FerroRuntimeXamlLoader::register_class_document(
            SettingsPage::TYPE,
            "ferres://ferroribbon_sample/SettingsPage.xaml",
        );
        let runtime = FerroRuntimeXamlLoader::parse(
            include_str!("../SettingsPage.xaml"),
            Some(&crate::ASSEMBLY),
        )
        .unwrap();
        let map: Value =
            serde_json::from_str(include_str!("../../../docs/settings-page-markup.json")).unwrap();
        for value in [compiled, runtime] {
            let page = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<SettingsPage>()
                .unwrap();
            for name in map["source_names"].as_array().unwrap() {
                page.named::<Control>(name.as_str().unwrap());
            }
            let window = mount(&page);
            page.named::<ComboBox>("Chrome").set_selected_index(1);
            assert_eq!(RibbonTheme::chrome_style(), RibbonChromeStyle::Colorful);
            page.named::<Button>("ShowState")
                .command()
                .unwrap()
                .execute(None);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                page.named::<TextBox>("StateJson").text().as_deref(),
                Some("Open the Word page first.")
            );
            window.close();
        }
    });
}

#[test]
fn actual_settings_button_keyboard_input_executes_one_reactive_action() {
    run(|| {
        use ferroui_base::input::{PhysicalKey, RawInputModifiers};
        use ferroui_headless::HeadlessWindowExtensions;
        let s = settings();
        let page = SettingsPage::new_with_settings(s.clone());
        let window = mount(&page);
        window.set_height(1400.);
        Dispatcher::ui_thread().run_jobs(None);
        let ribbon = ferroribbon::controls::Ribbon::new();
        s.set_word_ribbon(Some(ribbon.clone()));
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        let sub = ribbon.customize_requested(move |args| {
            args.handled.set(true);
            c.set(c.get() + 1);
        });
        let button = page.named::<Button>("Customize");
        assert!(button.focus());
        window.key_press_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        window.key_release_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(calls.get(), 1);
        sub.dispose();
        s.set_word_ribbon(None);
        window.close();
    });
}

#[test]
fn settings_inside_separate_native_popup_themes_the_original_owner_root() {
    use ferroui_controls::{Border, primitives::Popup};
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>()
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                overlay_popups: false,
                ..Default::default()
            })
    });
    let result = session
        .dispatch(
            || {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                let owner = Window::new();
                let content = Border::new();
                content.set_width(1200.);
                content.set_height(900.);
                owner.set_content(Some(Control::boxed(content.clone())));
                owner.show();
                Dispatcher::ui_thread().run_jobs(None);
                let page = SettingsPage::new_with_settings(settings());
                let popup = Popup::new();
                popup.set_popup_parent(Some(&owner.clone().upcast::<Control>()));
                popup.set_placement_target(content.clone());
                popup.set_child(page.clone());
                popup.set_is_open(true);
                Dispatcher::ui_thread().run_jobs(None);
                assert!(!popup.is_using_overlay_layer());
                let top = ferroui_controls::TopLevel::get_top_level(Some(&page)).unwrap();
                assert!(
                    top.cast::<ferroui_controls::primitives::PopupRoot>()
                        .is_some()
                );
                let before = page.get_value(ThemeVariant::requested_theme_variant_property());
                page.view_model()
                    .apply_selection(SettingsSelection::Theme(2))
                    .unwrap();
                assert_eq!(
                    content.get_value(ThemeVariant::requested_theme_variant_property()),
                    Some(ThemeVariant::dark())
                );
                assert_eq!(
                    page.get_value(ThemeVariant::requested_theme_variant_property()),
                    before
                );
                popup.set_is_open(false);
                owner.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
