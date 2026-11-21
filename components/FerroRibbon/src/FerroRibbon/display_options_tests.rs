use super::*;
use ferroui_base::{data::core::ValueTypes, metadata::MarkupTypeKind};
use serde_json::{Value, json};
fn rows(menu: &MenuFlyout) -> Vec<Ref<Control>> {
    menu.items()
        .iter()
        .map(|value| {
            ValueTypes::as_object(&*value.unwrap())
                .unwrap()
                .cast::<Control>()
                .unwrap()
        })
        .collect()
}
fn describe(control: &Control) -> Value {
    if let Some(item) = control.to_ref().cast::<RadioMenuFlyoutItem>() {
        json!({"kind":"radio","text":item.text(),"group":item.group_name(),"checked":item.is_checked(),"glyph":item.icon().and_then(|i|ValueTypes::as_object(&*i)).and_then(|i|i.cast::<FontIcon>()).and_then(|i|i.glyph())})
    } else if let Some(item) = control.to_ref().cast::<ToggleMenuFlyoutItem>() {
        json!({"kind":"toggle","text":item.text(),"group":null,"checked":item.is_checked(),"glyph":null})
    } else {
        assert!(control.is::<MenuFlyoutSeparator>());
        json!({"kind":"separator","text":null,"group":null,"checked":null,"glyph":null})
    }
}
#[test]
fn display_options_native_catalog_and_actions_match_336_original_cases() {
    crate::group_tests::run(|| {
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/display-options.json"))
                .unwrap();
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 336);
        for case in cases {
            let ribbon = Ribbon::new();
            let mode = RibbonVisibilityMode::from_raw(case["mode"].as_i64().unwrap() as i32);
            let qat = case["qat"].as_bool().unwrap();
            ribbon.set_minimize_behavior(RibbonMinimizeBehavior::from_raw(
                case["behavior"].as_i64().unwrap() as i32,
            ));
            ribbon.set_visibility_mode(mode);
            ribbon.set_is_minimize_button_visible(case["minimizeVisible"].as_bool().unwrap());
            ribbon.set_is_simplified_mode_available(case["simplified"].as_bool().unwrap());
            ribbon.set_is_quick_access_visible(qat);
            let menu = ribbon.build_display_options_menu();
            let items = rows(&menu);
            let actual = items.iter().map(|item| describe(item)).collect::<Vec<_>>();
            assert_eq!(json!(actual), case["rows"], "{case}");
            for (item, expected) in items.iter().zip(case["actions"].as_array().unwrap()) {
                ribbon.set_visibility_mode(mode);
                ribbon.set_display_mode(RibbonDisplayMode::Classic);
                ribbon.set_is_quick_access_visible(qat);
                if let Some(item) = item.cast::<crate::compat::MenuFlyoutItem>() {
                    item.invoke_menu_core();
                }
                assert_eq!(
                    json!({"visibility":ribbon.visibility_mode().raw(),"display":ribbon.display_mode().raw(),"qat":ribbon.is_quick_access_visible()}),
                    *expected,
                    "{case}"
                );
            }
        }
    });
}
#[test]
fn display_options_native_popup_ownership_theme_placement_and_lifetime() {
    crate::group_tests::run(|| {
        let app = ferroui_controls::Application::current().unwrap();
        let resources = crate::theming::RibbonThemeResources::new();
        let provider: Rc<dyn ferroui_base::controls::IResourceProvider> = resources
            .upcast::<ferroui_base::controls::ResourceDictionary>()
            .into();
        app.resources().merged_dictionaries().add(provider.clone());
        let ribbon = Ribbon::new();
        let window = crate::ribbon_tests::mount(&ribbon);
        let expected_theme = ValueTypes::as_object(
            &*app
                .resources()
                .try_get_resource(&"RibbonMenuFlyoutPresenterStyle".into(), None)
                .flatten()
                .unwrap(),
        )
        .unwrap()
        .cast::<ferroui_base::styling::ControlTheme>()
        .unwrap();
        let button = ribbon
            .display_options_button()
            .expect("source template part");
        let menu = ribbon.show_display_options_menu(&button);
        assert!(menu.is_open());
        assert_eq!(menu.placement(), PlacementMode::BottomEdgeAlignedRight);
        assert_eq!(menu.flyout_presenter_theme(), Some(expected_theme));
        let weak = menu.downgrade();
        drop(menu);
        assert!(weak.upgrade().is_some(), "ribbon owns an open menu");
        weak.upgrade().unwrap().hide();
        assert!(
            weak.upgrade().is_none(),
            "closed menu releases ownership and handlers"
        );
        let menu = ribbon.build_display_options_menu();
        window.close();
        window.set_content(None);
        let root = ribbon.downgrade();
        drop(ribbon);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(
            root.upgrade().is_none(),
            "retained menu callbacks cannot retain the ribbon after native unload jobs"
        );
        drop(menu);
        app.resources().merged_dictionaries().remove(&provider);
        assert_eq!(Ribbon::__MARKUP.unwrap().kind, MarkupTypeKind::Class);
        assert!(
            Ribbon::__MARKUP
                .unwrap()
                .methods
                .iter()
                .any(|method| method.name == "BuildDisplayOptionsMenu")
        );
    });
}
