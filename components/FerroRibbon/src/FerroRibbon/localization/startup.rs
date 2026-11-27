//! Explicit composition-root bridge from native platform settings to UI culture.
use ferroribbon_core::localization::{RibbonLocalizationError, RibbonLocalizationErrorKind};
use ferroui_base::{
    FerroLocator, LocatorExtensions, platform::IPlatformSettings, utilities::CultureInfo,
};

/// Selects the platform's preferred application language for this thread before
/// constructing ribbon UI. Call before register_types/first RibbonStrings access.
/// It does not change numeric CurrentCulture, an existing RibbonStrings.Current,
/// or subscribe an application to later system preference changes.
pub fn initialize_ui_culture_from_platform() -> Result<CultureInfo, RibbonLocalizationError> {
    let settings = FerroLocator::current()
        .get_service::<dyn IPlatformSettings>()
        .ok_or_else(|| {
            RibbonLocalizationError::new(
                RibbonLocalizationErrorKind::Unavailable,
                None,
                "Platform settings must be initialized before selecting the ribbon UI culture",
            )
        })?;
    let culture = CultureInfo::get_culture_info(&settings.preferred_application_language());
    ferroui_base::utilities::set_current_ui_locale(culture.locale().clone());
    Ok(culture)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroribbon_core::localization::RibbonStrings;
    use ferroui_base::{
        Size,
        input::{PointerType, platform::PlatformHotkeyConfiguration},
        platform::{DefaultPlatformSettings, PlatformColorValues},
        reactive::IDisposable,
        utilities::NumberFormatInfo,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};
    struct LanguageSettings {
        base: DefaultPlatformSettings,
        language: RefCell<String>,
    }
    impl IPlatformSettings for LanguageSettings {
        fn get_tap_size(&self, p: PointerType) -> Size {
            self.base.get_tap_size(p)
        }
        fn get_double_tap_size(&self, p: PointerType) -> Size {
            self.base.get_double_tap_size(p)
        }
        fn get_double_tap_time(&self, p: PointerType) -> Duration {
            self.base.get_double_tap_time(p)
        }
        fn hold_wait_duration(&self) -> Duration {
            self.base.hold_wait_duration()
        }
        fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration> {
            self.base.hotkey_configuration()
        }
        fn preferred_application_language(&self) -> String {
            self.language.borrow().clone()
        }
        fn get_color_values(&self) -> PlatformColorValues {
            self.base.get_color_values()
        }
        fn color_values_changed(&self, h: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
            self.base.color_values_changed(h)
        }
        fn preferred_application_language_changed(&self, h: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            self.base.preferred_application_language_changed(h)
        }
    }
    #[test]
    fn initial_platform_language_matches_source_and_later_ambient_changes_do_not_replace_current() {
        if crate::test_process::isolated_case(concat!(
            module_path!(),
            "::initial_platform_language_matches_source_and_later_ambient_changes_do_not_replace_current"
        )) {
            return;
        }
        let _scope = FerroLocator::enter_scope();
        let settings = Rc::new(LanguageSettings {
            base: DefaultPlatformSettings::new(),
            language: RefCell::new("fr-CA".into()),
        });
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformSettings>()
            .to_constant(settings.clone() as Rc<dyn IPlatformSettings>);
        let numeric = CultureInfo::invariant_culture()
            .with_number_format(NumberFormatInfo::new().with_negative_sign("custom"));
        CultureInfo::set_current_formatting(numeric.clone());
        let selected = initialize_ui_culture_from_platform().unwrap();
        assert_eq!(selected.name(), "fr-CA");
        crate::register_types();
        let initial = RibbonStrings::current();
        *settings.language.borrow_mut() = "de-DE".into();
        initialize_ui_culture_from_platform().unwrap();
        let current = RibbonStrings::current();
        let actual = serde_json::json!({"initialCulture":initial.culture().name().unwrap().as_utf16(),"initialFile":initial.file().unwrap().as_utf16(),"laterUi":ferroui_base::utilities::current_ui_locale().to_string(),"laterCulture":current.culture().name().unwrap().as_utf16(),"laterFile":current.file().unwrap().as_utf16(),"same":std::sync::Arc::ptr_eq(&initial,&current)});
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/localization-registry.json"
        ))
        .unwrap();
        assert_eq!(actual, source["cases"]["startup"]);
        assert!(
            Rc::ptr_eq(
                &CultureInfo::current_formatting().number_format(),
                &numeric.number_format()
            ),
            "numeric region and UI language are independent services"
        );
    }
    #[test]
    fn missing_platform_service_is_an_error_without_changing_ambient_language() {
        let previous_culture = ferroui_base::utilities::current_ui_locale();
        ferroui_base::utilities::set_current_ui_locale("pl".parse().unwrap());
        let old = FerroLocator::current();
        let empty = Rc::new(FerroLocator::new());
        FerroLocator::set_current(empty);
        let error = initialize_ui_culture_from_platform().unwrap_err();
        FerroLocator::set_current(old);
        assert_eq!(error.kind, RibbonLocalizationErrorKind::Unavailable);
        assert_eq!(
            ferroui_base::utilities::current_ui_locale().to_string(),
            "pl"
        );
        ferroui_base::utilities::set_current_ui_locale(previous_culture);
    }
}
