use super::WordPage;
use crate::{demo_settings::DemoSettings, word_view_model::WordAction};
use ferroui_base::{
    Point, Ref, Vector,
    input::RawInputModifiers,
    media::{FontWeight, TextAlignment},
    threading::{CancellationToken, Dispatcher},
};
use ferroui_controls::{
    AppBuilder, Application, Border, Button, Control, ScrollViewer, TextBlock, Window,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    HeadlessWindowExtensions,
};
use std::rc::Rc;
fn run(f: impl FnOnce() + Send + 'static) {
    let s = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>()
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
    });
    let result = s
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
    s.dispose();
    result.unwrap();
}
fn settings() -> Rc<DemoSettings> {
    DemoSettings::new(Rc::new(|| Ok("03:04:05".into())))
}
fn mount(page: &Ref<WordPage>) -> Ref<Window> {
    let w = Window::new();
    w.set_width(1440.);
    w.set_height(1100.);
    w.set_content(Some(Control::boxed(page.clone())));
    w.show();
    Dispatcher::ui_thread().run_jobs(None);
    w
}

#[test]
fn word_document_scrollbars_keep_the_full_viewport() {
    run(|| {
        let page = WordPage::new_with_settings(settings());
        let window = mount(&page);
        let document = page
            .named::<Border>("PageSheet")
            .find_ancestor_of_type::<ScrollViewer>(false)
            .expect("Word document scroll viewer");

        window.set_width(900.);
        window.set_height(680.);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(document.scroll_bar_maximum().y > 0.);
        assert_eq!(document.scroll_bar_maximum().x, 0.);
        assert_eq!(document.viewport().width, document.bounds().width);

        window.mouse_wheel(
            Point::new(100., 300.),
            Vector::new(0., -1.),
            RawInputModifiers::NONE,
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert!(document.offset().y > 0.);

        window.set_width(700.);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(document.scroll_bar_maximum().x > 0.);
        assert_eq!(document.viewport().width, document.bounds().width);
        assert_eq!(document.viewport().height, document.bounds().height);
        window.close();
    });
}

#[test]
fn word_original_full_markup_constructs_and_actual_invocation_updates_bound_document() {
    run(|| {
        let settings = settings();
        let page = WordPage::new_with_settings(settings.clone());
        let w = mount(&page);
        let map: serde_json::Value =
            serde_json::from_str(include_str!("../../../docs/word-page-markup.json")).unwrap();
        for name in map["source_names"].as_array().unwrap() {
            page.named::<ferroui_base::FerroObject>(name.as_str().unwrap());
        }
        assert_eq!(settings.word_ribbon(), Some(page.ribbon()));
        assert_eq!(page.ribbon().tabs().count(), 9);
        let bold = page.named::<ferroribbon::controls::RibbonToggleButton>("Bold");
        super::cad_session::ribbon_item(&bold.upcast::<Control>())
            .unwrap()
            .invoke();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            page.named::<TextBlock>("DocText").font_weight(),
            FontWeight::Bold
        );
        assert_eq!(
            page.view_model().last_command(),
            "Last command: bold (True)"
        );
        let grow = page.ribbon().find_item(Some("growFont".into())).unwrap();
        super::cad_session::ribbon_item(&grow).unwrap().invoke();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(page.named::<TextBlock>("DocText").font_size(), 18.);
        assert_eq!(
            page.named::<ferroribbon::controls::RibbonFontSizeComboBox>("FontSizeBox")
                .text()
                .as_deref(),
            Some("18")
        );
        page.view_model()
            .apply(WordAction::Invoked {
                item_id: Some("alignCenter".into()),
                command_id: None,
                parameter: None,
            })
            .unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            page.named::<TextBlock>("DocText").text_alignment(),
            TextAlignment::Center
        );
        page.named::<Button>("ToggleTable")
            .command()
            .unwrap()
            .execute(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(page.named::<Border>("TableSample").is_visible());
        assert!(
            settings
                .log()
                .borrow()
                .iter()
                .any(|s| s == "03:04:05  Word · bold (True)")
        );
        w.close();
    });
}

#[test]
fn word_parameter_text_matches_actual_uno_core_and_dotnet_values() {
    run(|| {
        use ferroribbon_core::model::RibbonGridSize;
        use ferroui_base::{BoxedValue, media::Color};
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/word-parameters.json"))
                .unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let value: BoxedValue = match case["kind"].as_str().unwrap() {
                "color" => {
                    let c = case["value"].as_u64().unwrap() as u32;
                    Rc::new(Color::from_argb(
                        (c >> 24) as u8,
                        (c >> 16) as u8,
                        (c >> 8) as u8,
                        c as u8,
                    ))
                }
                "noColor" => ferroribbon::controls::RibbonColorPicker::no_color_parameter(),
                "bool" => Rc::new(case["value"].as_bool().unwrap()),
                "int" => Rc::new(case["value"].as_i64().unwrap() as i32),
                "double" => Rc::new(match case["value"].as_str().unwrap() {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    s => s.parse::<f64>().unwrap(),
                }),
                "grid" => Rc::new(RibbonGridSize::new(
                    case["rows"].as_i64().unwrap() as i32,
                    case["columns"].as_i64().unwrap() as i32,
                )),
                _ => panic!("{case}"),
            };
            assert_eq!(
                super::word_page::word_parameter_text(&value).unwrap(),
                case["text"].as_str().unwrap(),
                "{case}"
            );
        }
    });
}

#[test]
fn word_native_commits_gallery_colors_table_zoom_and_settings_keep_source_state() {
    run(|| {
        use ferroribbon::controls::{
            RibbonColorPicker, RibbonFontComboBox, RibbonFontSizeComboBox, RibbonGallery,
            RibbonGridPicker, RibbonZoomControl,
        };
        use ferroribbon_core::{RibbonDensity, RibbonDisplayMode, model::RibbonGridSize};
        let settings = settings();
        let page = WordPage::new_with_settings(settings.clone());
        let w = mount(&page);
        let doc = page.named::<TextBlock>("DocText");
        page.named::<RibbonFontComboBox>("FontFamilyBox")
            .commit_text(Some("Arial".into()));
        page.named::<RibbonFontSizeComboBox>("FontSizeBox")
            .commit_text(Some("2e1".into()));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(doc.font_size(), 20.);
        assert_eq!(doc.font_family().name(), "Arial");
        let gallery = page.named::<RibbonGallery>("Styles");
        gallery.pick(Some(Rc::new("Title".to_owned())));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(page.named::<TextBlock>("DocTitle").font_size(), 40.);
        let color = page.named::<RibbonColorPicker>("FontColor");
        super::cad_session::ribbon_item(&color.upcast::<Control>())
            .unwrap()
            .invoke();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(page.view_model().body_foreground(), 0xffc00000);
        assert!(
            settings
                .log()
                .borrow()
                .iter()
                .any(|s| s.contains("fontColor (#FFC00000)"))
        );
        page.named::<RibbonColorPicker>("PageColor")
            .palette()
            .select(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(page.view_model().page_background(), 0xffffffff);
        assert!(
            settings
                .log()
                .borrow()
                .iter()
                .any(|s| s.contains("pageColor (System.Object)"))
        );
        page.named::<RibbonGridPicker>("TablePicker")
            .pick(RibbonGridSize::new(2, 3));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(page.named::<Border>("TableSample").is_visible());
        assert!(
            settings
                .log()
                .borrow()
                .iter()
                .any(|s| s.contains("Inserted 3×2 table"))
        );
        page.named::<RibbonZoomControl>("Zoom").set_value(
            ferroribbon::controls::RibbonZoomControl::value_property(),
            150.,
        );
        Dispatcher::ui_thread().run_jobs(None);
        let sheet = page.named::<Border>("PageSheet");
        assert_eq!(sheet.render_transform_origin().point.x, 380.);
        let transform = sheet.render_transform().unwrap().value();
        assert_eq!(transform.m11, 1.5);
        assert_eq!(transform.m22, 1.5);
        settings.apply(RibbonDensity::Touch, Some(RibbonDisplayMode::Simplified));
        assert_eq!(page.ribbon().density(), RibbonDensity::Touch);
        assert_eq!(page.ribbon().display_mode(), RibbonDisplayMode::Simplified);
        settings.apply(RibbonDensity::Compact, None);
        assert_eq!(page.ribbon().display_mode(), RibbonDisplayMode::Simplified);
        w.close();
    });
}

#[test]
fn word_view_and_model_release_without_clearing_shared_word_ribbon_manually() {
    run(|| {
        let settings = settings();
        let weak_settings = Rc::downgrade(&settings);
        let page = WordPage::new_with_settings(settings.clone());
        let weak_page = page.downgrade();
        let model = Rc::downgrade(&page.view_model());
        let ribbon = page.ribbon().downgrade();
        let w = mount(&page);
        w.set_content(None);
        w.close();
        drop(page);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak_page.upgrade().is_none());
        // The service retains WordRibbon by source contract, but Ribbon's data context
        // must not point strongly back to that service. Dropping the composition owner
        // must release the whole graph without breaking a cycle by hand.
        drop(settings);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak_settings.upgrade().is_none());
        assert!(model.upgrade().is_none());
        assert!(ribbon.upgrade().is_none());
    });
}

#[test]
fn word_compiled_runtime_class_and_settings_state_use_the_same_real_ribbon() {
    run(|| {
        use ferroui_base::{data::core::ValueTypes, utilities::UriRef};
        use ferroui_markup_xaml::FerroXamlLoader;
        use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
        FerroRuntimeXamlLoader::register_class_document(
            WordPage::TYPE,
            "ferres://ferroribbon_sample/WordPage.xaml",
        );
        for runtime in [false, true] {
            let value = if runtime {
                FerroRuntimeXamlLoader::parse(
                    include_str!("../WordPage.xaml"),
                    Some(&crate::ASSEMBLY),
                )
                .unwrap()
            } else {
                FerroXamlLoader::load(
                    &UriRef::parse_absolute("ferres://ferroribbon_sample/WordPage.xaml")
                        .unwrap()
                        .into(),
                    None,
                )
                .unwrap()
            };
            let page = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<WordPage>()
                .unwrap();
            let w = mount(&page);
            let shared = DemoSettings::current();
            assert_eq!(shared.word_ribbon(), Some(page.ribbon()));
            let settings_page = super::SettingsPage::new_with_settings(shared.clone());
            settings_page
                .view_model()
                .apply_word_action(crate::settings_view_model::SettingsWordAction::ShowState)
                .unwrap();
            assert_eq!(
                settings_page.view_model().state_json(),
                page.ribbon()
                    .save_state_to_json()
                    .unwrap()
                    .to_utf8()
                    .unwrap()
            );
            w.close();
            shared.set_word_ribbon(None);
        }
    });
}

#[test]
fn word_actual_keyboard_bold_executes_one_source_action_and_applies_formatting() {
    run(|| {
        use ferroui_base::input::{PhysicalKey, RawInputModifiers};
        use ferroui_headless::HeadlessWindowExtensions;
        let settings = settings();
        let page = WordPage::new_with_settings(settings.clone());
        let w = mount(&page);
        w.set_width(2000.);
        Dispatcher::ui_thread().run_jobs(None);
        let bold = page.named::<ferroribbon::controls::RibbonToggleButton>("Bold");
        assert!(bold.focus());
        let before = settings.log().borrow().len();
        w.key_press_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        w.key_release_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(settings.log().borrow().len(), before + 1);
        assert_eq!(
            page.named::<TextBlock>("DocText").font_weight(),
            FontWeight::Bold
        );
        w.close();
    });
}

#[test]
fn original_word_settings_cases_apply_to_actual_native_ribbon() {
    run(|| {
        use ferroribbon_core::{RibbonDensity, RibbonDisplayMode};
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/word-page.json")).unwrap();
        let mut compared = 0;
        for case in fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["kind"] == "settings")
        {
            let settings = settings();
            let page = WordPage::new_with_settings(settings.clone());
            settings.apply(
                RibbonDensity::from_raw(case["density"].as_i64().unwrap() as i32),
                case["mode"]
                    .as_i64()
                    .map(|m| RibbonDisplayMode::from_raw(m as i32)),
            );
            assert_eq!(
                page.ribbon().density().raw(),
                case["state"]["density"].as_i64().unwrap() as i32
            );
            assert_eq!(
                page.ribbon().display_mode().raw(),
                case["state"]["display"].as_i64().unwrap() as i32
            );
            compared += 1;
        }
        assert_eq!(compared, 20);
    });
}

#[test]
fn word_gallery_native_focus_previews_restores_and_commits_original_style() {
    run(|| {
        use ferroui_base::input::{KeyModifiers, NavigationMethod, PhysicalKey, RawInputModifiers};
        use ferroui_controls::automation::AutomationProperties;
        use ferroui_headless::HeadlessWindowExtensions;
        let page = WordPage::new_with_settings(settings());
        let w = mount(&page);
        w.set_width(2000.);
        Dispatcher::ui_thread().run_jobs(None);
        let gallery = page.named::<ferroribbon::controls::RibbonGallery>("Styles");
        let title = gallery
            .get_visual_descendants()
            .filter_map(|c| c.cast::<Button>())
            .find(|b| AutomationProperties::get_name(b).as_deref() == Some("Title"))
            .expect("original inline Title gallery item");
        assert!(title.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(page.named::<TextBlock>("DocTitle").font_size(), 40.);
        assert_eq!(page.view_model().selected_style(), "Normal");
        assert!(page.named::<Button>("ToggleTable").focus());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(page.named::<TextBlock>("DocTitle").font_size(), 30.);
        assert!(title.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
        w.key_press_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        w.key_release_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(page.view_model().selected_style(), "Title");
        assert_eq!(page.named::<TextBlock>("DocTitle").font_size(), 40.);
        w.close();
    });
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[test]
fn word_original_gallery_filter_selects_style_once_and_keeps_footer_identity() {
    run(|| {
        use ferroribbon::{controls::RibbonGallery, primitives::RibbonUniformGridPanel};
        use ferroui_base::{
            input::{Key, PhysicalKey, RawInputModifiers},
            metadata::MarkupDelegate,
        };
        use ferroui_controls::{StackPanel, TextBox, TopLevel};
        use ferroui_headless::HeadlessWindowExtensions;
        use std::cell::Cell;
        let page = WordPage::new_with_settings(settings());
        let w = mount(&page);
        let gallery = page.named::<RibbonGallery>("Styles");
        assert!(
            gallery.is_filter_enabled(),
            "original Word XAML enables filtering"
        );
        let footers = gallery
            .footer_items()
            .iter()
            .map(Option::unwrap)
            .collect::<Vec<_>>();
        let count = Rc::new(Cell::new(0));
        let out = count.clone();
        let handler = MarkupDelegate::new(move |_| {
            out.set(out.get() + 1);
            None
        });
        gallery.add_item_click(Some(handler.clone()));
        gallery.open_drop_down();
        Dispatcher::ui_thread().run_jobs(None);
        let focused = w.focus_manager().get_focused_element().unwrap();
        let top = TopLevel::get_top_level(Some(&focused)).unwrap();
        let placeholder = ferroribbon_core::localization::RibbonStrings::current()
            .gallery_filter()
            .unwrap()
            .to_utf8()
            .unwrap();
        let filter = top
            .get_visual_descendants()
            .filter_map(|v| v.cast::<TextBox>())
            .find(|v| v.placeholder_text().as_deref() == Some(placeholder.as_str()))
            .unwrap();
        let root = filter
            .visual_parent()
            .unwrap()
            .cast::<StackPanel>()
            .unwrap();
        filter.set_text(Some("hEaDiNg 1"));
        Dispatcher::ui_thread().run_jobs(None);
        let panel = root
            .get_visual_descendants()
            .find_map(|v| v.cast::<RibbonUniformGridPanel>())
            .unwrap();
        assert_eq!(panel.children().count(), 1);
        for footer in &footers {
            assert!(root.get_visual_descendants().any(|v| v.ptr_eq(footer)));
        }
        let button = panel.children().get(0).cast::<Button>().unwrap();
        assert!(button.focus());
        let input = TopLevel::get_top_level(Some(&button)).unwrap();
        input.key_press(
            Key::Space,
            RawInputModifiers::NONE,
            PhysicalKey::Space,
            None,
        );
        if gallery.is_drop_down_open() {
            input.key_release(
                Key::Space,
                RawInputModifiers::NONE,
                PhysicalKey::Space,
                None,
            );
        } else {
            w.key_release(
                Key::Space,
                RawInputModifiers::NONE,
                PhysicalKey::Space,
                None,
            );
        }
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(count.get(), 1);
        assert_eq!(page.view_model().selected_style(), "Heading1");
        assert!(!gallery.is_drop_down_open());
        assert!(w.is_visible());
        for footer in &footers {
            assert!(footer.visual_parent().is_none());
        }
        gallery.remove_item_click(Some(handler));
        w.close();
        w.set_content(None);
    });
}

#[test]
fn word_native_title_change_reentry_retains_source_selected_style_and_late_log() {
    run(|| {
        use ferroui_base::{FerroObjectExtensions, reactive::AnonymousObserver};
        let page = WordPage::new_with_settings(settings());
        let w = mount(&page);
        let title = page.named::<TextBlock>("DocTitle");
        let gallery = page.named::<ferroribbon::controls::RibbonGallery>("Styles");
        let weak = gallery.downgrade();
        let entered = std::cell::Cell::new(false);
        let subscription = FerroObjectExtensions::get_observable(
            &*title.clone().upcast::<ferroui_base::FerroObject>(),
            TextBlock::font_size_property(),
        )
        .subscribe(Rc::new(AnonymousObserver::new(move |size: f64| {
            if size == 24.
                && !entered.replace(true)
                && let Some(gallery) = weak.upgrade()
            {
                gallery.pick(Some(Rc::new("Strong".to_owned())));
            }
        })));
        gallery.pick(Some(Rc::new("Heading2".to_owned())));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(page.view_model().selected_style(), "Strong");
        assert_eq!(title.font_size(), 30.);
        assert_eq!(title.font_weight(), FontWeight::Normal);
        assert_eq!(page.view_model().title_foreground(), 0xff2f5496);
        subscription.dispose();
        w.close();
    });
}
