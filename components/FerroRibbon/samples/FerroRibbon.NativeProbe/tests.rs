use super::*;
use ferroui_base::threading::CancellationToken;
use ferroui_controls::{AppBuilder, Application, Slider, TextBox};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "FerroRibbon.NativeProbe.Tests",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn sliders_drive_original_layout_and_retain_edited_children() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                register_types();
                Application::current().unwrap().styles().add(
                    ferroui_themes_simple::SimpleTheme::new()
                        .upcast::<ferroui_base::styling::Styles>(),
                );
                let model =
                    view_model::ProbeViewModel::new(Rc::new(layout_preview::SourceLayoutPreview));
                let window = create_window(model.clone());
                window.show();
                let descendants: Vec<_> = window.get_visual_descendants().collect();
                let sliders: Vec<_> = descendants
                    .iter()
                    .filter_map(|v| v.clone().cast::<Slider>())
                    .collect();
                let panel = descendants
                    .iter()
                    .find_map(|v| {
                        v.clone()
                            .cast::<ferroribbon::primitives::RibbonUniformGridPanel>()
                    })
                    .unwrap();
                assert_eq!(sliders.len(), 2);
                let child = panel.children().get(0);
                let text = child.clone().cast::<TextBox>().unwrap();
                text.set_text(Some("Edited"));
                sliders[1].set_range_value(2.);
                assert_eq!(model.columns(), 2);
                assert_eq!(panel.columns(), 2);
                assert_eq!(panel.row_count(), 6);
                assert!(child.ptr_eq(&panel.children().get(0)));
                assert_eq!(text.text().as_deref(), Some("Edited"));
                sliders[0].set_range_value(600.);
                assert!(model.layout_summary().starts_with("Large / Large / Large"));
                sliders[0].set_range_value(120.);
                assert!(model.layout_summary().contains("Fits: false"));
                let presentation = descendants
                    .iter()
                    .find_map(|v| {
                        v.clone()
                            .cast::<ferroribbon::primitives::RibbonItemContent>()
                    })
                    .unwrap();
                model.set_presentation_label("Paste Special".into());
                assert_eq!(presentation.label().as_deref(), Some("Paste Special"));
                assert!(presentation.foreground().is_some());
                model.set_is_simplified(true);
                assert!(presentation.is_simplified());
                let checkbox = descendants
                    .iter()
                    .find_map(|v| v.clone().cast::<ferroui_controls::CheckBox>())
                    .unwrap();
                checkbox.set_is_checked(Some(false));
                assert!(!model.is_simplified());
                let badge = descendants
                    .iter()
                    .find_map(|v| v.clone().cast::<ferroribbon::controls::RibbonKeyTip>())
                    .unwrap();
                assert_eq!(badge.text().as_deref(), Some("FP"));
                assert!(!badge.is_hit_test_visible());
                let toggle = descendants
                    .iter()
                    .filter_map(|v| {
                        v.clone()
                            .cast::<ferroui_controls::primitives::ToggleButton>()
                    })
                    .find(|v| v.name().as_deref() == Some("SharedToggle"))
                    .unwrap();
                assert!(toggle.presenter().is_some());
                toggle.set_is_checked(Some(true));
                assert!(model.is_simplified());
                assert!(presentation.is_simplified());
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}

#[test]
fn native_command_bridge_executes_actual_reactiveui_command_with_same_parameter() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                register_types();
                let observed = Rc::new(std::cell::Cell::new(0));
                let result = observed.clone();
                let core: Rc<dyn ferroui_reactiveui::ICommand> =
                    Rc::new(ferroui_reactiveui::ReactiveCommand::create_param(
                        move |value: i32| result.set(value),
                    ));
                let native = ferroribbon::commands::adapt_core_command(core.clone());
                let parameter: ferroui_base::BoxedValue = Rc::new(37i32);
                assert!(native.can_execute(Some(&parameter)));
                native.execute(Some(&parameter));
                assert_eq!(observed.get(), 37);
                assert!(Rc::ptr_eq(
                    &ferroribbon::commands::source_core_command(&native).unwrap(),
                    &core
                ));
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
