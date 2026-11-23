//! Every generated key of the theme (`keys.rs`, `scripts/generate_theme_keys.py`) resolves against
//! the loaded theme, and its value is of the declared type in each variant.

use crate::keys::ALL_KEYS;
use crate::SimpleTheme;
use ferroui_base::controls::check_theme_keys;
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::Application;

#[test]
fn every_generated_key_resolves_to_its_declared_type() {
    let _app = super::support::start_application();
    let application = Application::current().expect("an application");
    let theme = SimpleTheme::new();
    application.styles().add(theme.as_style());
    let variants = [ThemeVariant::default(), ThemeVariant::light(), ThemeVariant::dark()];
    let report = check_theme_keys(ALL_KEYS, &variants, &|key, variant| application.try_get_resource(key, Some(variant)));
    assert!(report.missing.is_empty(), "{} keys in no variant: {:?}", report.missing.len(), report.missing);
    assert!(report.wrong.is_empty(), "{} values of another type: {:#?}", report.wrong.len(), report.wrong);
    assert!(report.checked_values > 0);
}
