use crate::{
    demo_settings::DemoSettings,
    excel_model_native_tests::{invoke, run},
    pages::*,
    shell_page::ShellPage,
};
use ferroribbon::{controls::RibbonToggleButton, theming::RibbonTheme};
use ferroribbon_core::theming::{RibbonChromeStyle, RibbonThemePalette, RibbonThemeStyle};
use ferroui_base::{Ref, styling::ThemeVariant, threading::Dispatcher};
use ferroui_controls::{Control, Window};
use std::{cell::Cell, rc::Rc};
fn settings() -> Rc<DemoSettings> {
    DemoSettings::new(Rc::new(|| Ok("03:04:05".into())))
}
fn mount(p: &Ref<ShellPage>) -> Ref<Window> {
    let w = Window::new();
    w.set_width(1440.);
    w.set_height(1000.);
    w.set_content(Some(Control::boxed(p.clone())));
    w.show();
    Dispatcher::ui_thread().run_jobs(None);
    w
}
fn rail(p: &ShellPage, key: &str) -> Ref<RibbonToggleButton> {
    p.rail()
        .items()
        .iter()
        .flatten()
        .filter_map(|i| i.cast::<RibbonToggleButton>())
        .find(|t| t.id().as_deref() == Some(&format!("rail.{key}")))
        .unwrap()
}

#[cfg(windows)]
#[test]
fn floating_cad_panel_keeps_the_winui_measure_budget_at_125_percent() {
    use ferroui_headless::HeadlessWindowExtensions;

    run(|| {
        let shell = ShellPage::new_with_settings(settings());
        let window = mount(&shell);
        window.set_render_scaling(1.25);
        window.set_width(822.4);
        window.set_height(594.4);
        shell.show("cad").unwrap();
        Dispatcher::ui_thread().run_jobs(None);

        let cad = shell
            .current_content()
            .unwrap()
            .unwrap()
            .cast::<CadPage>()
            .unwrap();
        cad.float_layers_panel();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(cad.layers_panel().is_floating_panel_open());

        let panel = cad.cad_ribbon().selected_tab().unwrap().groups_panel();
        // The WinUI trace reported 776.8 DIPs before the panel's own rounding.
        assert_eq!(
            panel.previous_measure().unwrap().width,
            f64::from(776.8_f32)
        );
        window.close();
    });
}

#[cfg(windows)]
#[test]
fn floating_cad_panel_uses_winui_gallery_font() {
    use ferroui_base::media::FontFamily;
    use ferroui_controls::{Application, TextBlock};

    run(|| {
        crate::initialize_gallery(&Application::current().unwrap());
        let shell = ShellPage::new_with_settings(settings());
        let window = mount(&shell);
        shell.show("cad").unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        let cad = shell
            .current_content()
            .unwrap()
            .unwrap()
            .cast::<CadPage>()
            .unwrap();
        let ribbon = cad.cad_ribbon();
        let customized_font = FontFamily::new("Courier New");
        ribbon.set_font_family(customized_font.clone());
        let properties = ribbon.find_item(Some("layerProperties".into())).unwrap();
        cad.float_layers_panel();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(cad.layers_panel().is_floating_panel_open());

        let caption = properties
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<TextBlock>())
            .find(|text| text.text().as_deref() == Some("Properties"))
            .unwrap();
        assert_eq!(
            caption.font_family(),
            FontFamily::new("Segoe UI Variable, Segoe UI, $Default")
                .with_automatic_optical_sizing()
        );
        assert_eq!(ribbon.font_family(), customized_font);
        window.close();
    });
}

#[cfg(windows)]
#[test]
fn cad_collapsed_group_caption_inherits_winui_variable_font() {
    use ferroribbon_core::RibbonGroupState;
    use ferroui_base::media::FontFamily;
    use ferroui_controls::TextBlock;

    run(|| {
        let shell = ShellPage::new_with_settings(settings());
        let window = mount(&shell);
        shell.show("cad").unwrap();
        Dispatcher::ui_thread().run_jobs(None);

        let cad = shell
            .current_content()
            .unwrap()
            .unwrap()
            .cast::<CadPage>()
            .unwrap();
        let ribbon = cad.cad_ribbon();
        let group = ribbon
            .selected_tab()
            .unwrap()
            .groups()
            .iter()
            .flatten()
            .find(|group| group.id().as_deref() == Some("properties"))
            .unwrap();
        group.measure_width(RibbonGroupState::Collapsed, ribbon.metrics(), 83.);
        let button = group.collapsed_button().unwrap();
        let caption = button
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<TextBlock>())
            .find(|text| text.text() == button.label())
            .unwrap();
        assert_eq!(
            caption.font_family(),
            FontFamily::new("Segoe UI Variable, Segoe UI, $Default").with_automatic_optical_sizing()
        );
        window.close();
    });
}

#[cfg(windows)]
#[test]
fn cad_slide_out_matches_winui_footer_and_row_heights_at_125_percent() {
    use ferroui_controls::{Border, Grid, StackPanel};
    use ferroui_headless::HeadlessWindowExtensions;

    run(|| {
        let shell = ShellPage::new_with_settings(settings());
        let window = mount(&shell);
        window.set_render_scaling(1.25);
        window.set_width(822.4);
        window.set_height(594.4);
        shell.show("cad").unwrap();
        Dispatcher::ui_thread().run_jobs(None);

        let cad = shell
            .current_content()
            .unwrap()
            .unwrap()
            .cast::<CadPage>()
            .unwrap();
        let draw = cad.draw_panel();
        draw.set_is_slide_out_pinned(true);
        draw.open_slide_out();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(draw.is_slide_out_open());

        let popup = window.opened_popups().into_iter().next().unwrap();
        let chrome = popup.child().unwrap().cast::<Border>().unwrap();
        let scale = ferroui_base::layout::LayoutHelper::get_layout_scale(&chrome);
        assert_eq!(scale, 1.25);
        let pixels = |dip: f64| (dip * scale).round() as i32;
        let grid = chrome.child().unwrap().cast::<Grid>().unwrap();
        let host = grid.children().get(0).cast::<Border>().unwrap();
        let rows = host.child().unwrap().cast::<StackPanel>().unwrap();
        let caption = grid.children().get(1);
        let geometry: Vec<_> = rows
            .children()
            .to_vec()
            .iter()
            .map(|row| (pixels(row.bounds().y), pixels(row.bounds().height)))
            .collect();
        assert_eq!(pixels(chrome.bounds().height), 114);
        assert_eq!(pixels(host.bounds().height), 91);
        assert_eq!(pixels(caption.bounds().height), 21);
        assert_eq!(geometry, [(0, 27), (28, 27), (57, 27)]);

        draw.close_slide_out();
        window.close();
    });
}

#[test]
fn shell_loaded_navigation_uses_all_six_real_pages_and_keeps_hidden_pages_loaded() {
    run(|| {
        let s = settings();
        let p = ShellPage::new_with_settings(s.clone());
        assert_eq!(p.current_page().as_deref(), Some("word"));
        assert!(p.current_content().unwrap().is_none());
        let w = mount(&p);
        let word = p
            .current_content()
            .unwrap()
            .unwrap()
            .cast::<WordPage>()
            .unwrap();
        assert!(crate::shell_page::find_ribbon(&word).is_some());
        let unloaded = Rc::new(Cell::new(0));
        let log = unloaded.clone();
        word.unloaded(move |_, _| log.set(log.get() + 1));
        word.ribbon().set_is_backstage_open(true);
        assert!(word.ribbon().is_backstage_open());
        invoke(rail(&p, "excel").upcast());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(p.current_page().as_deref(), Some("excel"));
        assert!(
            p.current_content()
                .unwrap()
                .unwrap()
                .cast::<ExcelPage>()
                .is_some()
        );
        assert!(!word.ribbon().is_backstage_open());
        assert!(word.is_loaded());
        assert!(!word.is_visible());
        assert_eq!(unloaded.get(), 0);
        for key in ["powerpoint", "tools", "cad", "settings", "word"] {
            p.show(key).unwrap();
            Dispatcher::ui_thread().run_jobs(None);
        }
        assert_eq!(p.page_host().children().count(), 6);
        assert!(p.current_content().unwrap().unwrap().ptr_eq(&word));
        assert_eq!(unloaded.get(), 0);
        assert!(s.word_ribbon().unwrap().ptr_eq(&word.ribbon()));
        assert_eq!(
            p.page_host()
                .children()
                .to_vec()
                .iter()
                .filter(|c| c.is_visible())
                .count(),
            1
        );
        assert_eq!(
            RibbonTheme::palette()
                .name
                .as_ref()
                .unwrap()
                .to_utf8()
                .unwrap(),
            "Word"
        );
        w.close();
        w.set_content(None);
    });
}
#[test]
fn shell_style_invalidation_preserves_current_cad_settings_and_recreates_office_pages() {
    run(|| {
        let s = settings();
        let p = ShellPage::new_with_settings(s.clone());
        let w = mount(&p);
        let old = p.current_content().unwrap().unwrap();
        for key in ["excel", "powerpoint", "cad", "settings", "tools"] {
            p.show(key).unwrap();
            Dispatcher::ui_thread().run_jobs(None);
        }
        let tools = p.current_content().unwrap().unwrap();
        assert_eq!(p.page_host().children().count(), 6);
        s.set_surface_style(RibbonThemeStyle::Cad).unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(p.page_host().children().count(), 3);
        assert!(p.current_content().unwrap().unwrap().ptr_eq(&tools));
        assert!(!old.is_loaded());
        p.show("word").unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!p.current_content().unwrap().unwrap().ptr_eq(&old));
        assert_eq!(RibbonTheme::style(), RibbonThemeStyle::Cad);
        w.close();
        w.set_content(None);
    });
}
#[test]
fn shell_cad_rail_theme_office_chrome_palette_override_and_invalid_keys() {
    run(|| {
        let s = settings();
        let p = ShellPage::new_with_settings(s.clone());
        let w = mount(&p);
        RibbonTheme::apply_chrome_style(RibbonChromeStyle::Colorful).unwrap();
        p.show("cad").unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        let cad = p
            .current_content()
            .unwrap()
            .unwrap()
            .cast::<CadPage>()
            .unwrap();
        assert_eq!(RibbonTheme::style(), RibbonThemeStyle::Cad);
        assert_eq!(RibbonTheme::chrome_style(), RibbonChromeStyle::Neutral);
        assert_eq!(
            p.rail()
                .get_value(ThemeVariant::requested_theme_variant_property()),
            cad.actual_theme_variant()
        );
        cad.set_light_theme(true);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            p.rail()
                .get_value(ThemeVariant::requested_theme_variant_property()),
            Some(ThemeVariant::light())
        );
        let palette = RibbonThemePalette::graphite();
        s.set_palette_override(Some(palette.clone()));
        p.show("excel").unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(RibbonTheme::chrome_style(), RibbonChromeStyle::Colorful);
        assert!(Rc::ptr_eq(&RibbonTheme::palette(), &palette));
        assert_eq!(
            p.rail()
                .get_value(ThemeVariant::requested_theme_variant_property()),
            None
        );
        cad.set_light_theme(false);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            p.rail()
                .get_value(ThemeVariant::requested_theme_variant_property()),
            None
        );
        assert!(p.show("EXCEL").is_err());
        assert_eq!(p.current_page().as_deref(), Some("EXCEL"));
        assert!(p.current_content().unwrap().is_none());
        assert!(p.show_optional(None).is_err());
        assert_eq!(p.current_page(), None);
        assert!(p.current_content().is_err());
        assert!(p.show("word").is_err());
        w.close();
        w.set_content(None);
    });
}
#[test]
fn shell_native_view_model_command_settings_and_page_graph_release() {
    run(|| {
        let s = settings();
        let weak_settings = Rc::downgrade(&s);
        let p = ShellPage::new_with_settings(s.clone());
        let weak = p.downgrade();
        let vm = p.view_model();
        let weak_model = Rc::downgrade(&vm);
        let command = vm.navigation();
        let w = mount(&p);
        for key in ["excel", "cad", "settings"] {
            p.show(key).unwrap();
            Dispatcher::ui_thread().run_jobs(None);
        }
        let children = p
            .page_host()
            .children()
            .to_vec()
            .iter()
            .map(|c| c.downgrade())
            .collect::<Vec<_>>();
        w.close();
        w.set_content(None);
        drop(w);
        drop(p);
        drop(vm);
        drop(s);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
        assert!(weak_model.upgrade().is_none());
        assert!(weak_settings.upgrade().is_none());
        assert!(children.iter().all(|c| c.upgrade().is_none()));
        drop(command);
    });
}

#[test]
fn shell_compiled_and_runtime_documents_use_the_same_native_navigation() {
    run(|| {
        use ferroui_base::{data::core::ValueTypes, utilities::UriRef};
        use ferroui_markup_xaml::FerroXamlLoader;
        use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
        FerroRuntimeXamlLoader::register_class_document(
            ShellPage::TYPE,
            "ferres://ferroribbon_sample/ShellPage.xaml",
        );
        for runtime in [false, true] {
            let value = if runtime {
                FerroRuntimeXamlLoader::parse(
                    include_str!("ShellPage.xaml"),
                    Some(&crate::ASSEMBLY),
                )
                .unwrap()
            } else {
                FerroXamlLoader::load(
                    &UriRef::parse_absolute("ferres://ferroribbon_sample/ShellPage.xaml")
                        .unwrap()
                        .into(),
                    None,
                )
                .unwrap()
            };
            let p = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ShellPage>()
                .unwrap();
            let w = mount(&p);
            assert_eq!(p.current_page().as_deref(), Some("word"));
            invoke(rail(&p, "excel").upcast());
            assert!(
                p.current_content()
                    .unwrap()
                    .unwrap()
                    .cast::<ExcelPage>()
                    .is_some()
            );
            w.close();
            w.set_content(None);
        }
    });
}
#[test]
fn shell_visibility_reentry_detects_added_and_restored_native_children_and_unsubscribes() {
    run(|| {
        use ferroui_base::{
            FerroObject, FerroObjectExtensions, Visual, reactive::AnonymousObserver,
        };
        use ferroui_controls::Border;
        for restore in [false, true] {
            let p = ShellPage::new_with_settings(settings());
            let w = mount(&p);
            let word = p.current_content().unwrap().unwrap();
            word.set_is_visible(false);
            let armed = Rc::new(Cell::new(false));
            let flag = armed.clone();
            let host = p.page_host();
            let subscription = FerroObjectExtensions::get_observable(
                &*word.clone().upcast::<FerroObject>(),
                Visual::is_visible_property(),
            )
            .subscribe(Rc::new(AnonymousObserver::new(move |visible| {
                if visible && flag.replace(false) {
                    let extra = Border::new();
                    host.children().add(extra.clone());
                    if restore {
                        host.children().remove(extra);
                    }
                }
            })));
            armed.set(true);
            assert_eq!(
                p.show("word"),
                Err(crate::shell_view_model::ShellError::ChildrenChanged)
            );
            assert_eq!(
                p.page_host().children().count(),
                if restore { 1 } else { 2 }
            );
            subscription.dispose();
            p.show("word").unwrap();
            w.close();
            w.set_content(None);
        }
    });
}

#[test]
fn shell_pointer_keyboard_settings_and_cached_cad_floating_panel_interact() {
    run(|| {
        use ferroui_base::{
            Point,
            input::{MouseButton, PhysicalKey, RawInputModifiers},
        };
        use ferroui_controls::ComboBox;
        use ferroui_headless::HeadlessWindowExtensions;
        let s = settings();
        let p = ShellPage::new_with_settings(s.clone());
        let w = mount(&p);
        let excel = rail(&p, "excel");
        let point = excel.translate_point(Point::new(8., 8.), &w).unwrap();
        w.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        w.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(p.current_page().as_deref(), Some("excel"));
        assert!(rail(&p, "cad").focus());
        w.key_press_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        w.key_release_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        let cad = p
            .current_content()
            .unwrap()
            .unwrap()
            .cast::<CadPage>()
            .unwrap();
        cad.float_layers_panel();
        Dispatcher::ui_thread().run_jobs(None);
        let group = cad.layers_panel();
        assert!(group.is_floating());
        assert!(group.is_floating_panel_open());
        p.show("word").unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(group.is_floating());
        assert!(!group.is_floating_panel_open());
        p.show("cad").unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(p.current_content().unwrap().unwrap().ptr_eq(&cad));
        assert!(group.is_floating_panel_open());
        p.show("settings").unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        let page = p
            .current_content()
            .unwrap()
            .unwrap()
            .cast::<SettingsPage>()
            .unwrap();
        page.named::<ComboBox>("Surface").set_selected_index(1);
        page.named::<ComboBox>("Density").set_selected_index(2);
        page.named::<ComboBox>("Layout").set_selected_index(2);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(p.page_host().children().count(), 2);
        assert_eq!(s.surface_style(), RibbonThemeStyle::Cad);
        p.show("word").unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        let word = p
            .current_content()
            .unwrap()
            .unwrap()
            .cast::<WordPage>()
            .unwrap();
        assert_eq!(
            word.ribbon().density(),
            ferroribbon_core::RibbonDensity::Touch
        );
        assert_eq!(
            word.ribbon().display_mode(),
            ferroribbon_core::RibbonDisplayMode::Simplified
        );
        assert_eq!(RibbonTheme::style(), RibbonThemeStyle::Cad);
        w.close();
        w.set_content(None);
    });
}
