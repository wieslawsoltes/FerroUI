//! Every generated key of the theme (`keys.rs`, `scripts/generate_theme_keys.py`) resolves against
//! the loaded theme, and its value is of the declared type in each variant.

use crate::keys::ALL_KEYS;
use crate::{DensityStyle, FluentTheme};
use ferroui_base::controls::{check_theme_keys, ResourceKey};
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::Application;

#[test]
fn every_generated_key_resolves_to_its_declared_type() {
    let _app = super::support::start_application();
    let application = Application::current().expect("an application");
    let theme = FluentTheme::new();
    application.styles().add(theme.as_style());
    let variants = [ThemeVariant::default(), ThemeVariant::light(), ThemeVariant::dark()];
    let find = |key: &ResourceKey, variant: &ThemeVariant| application.try_get_resource(key, Some(variant));
    let normal = check_theme_keys(ALL_KEYS, &variants, &find);
    // Some keys are defined only by the dictionary of the compact density.
    theme.set_density_style(DensityStyle::Compact);
    let compact = check_theme_keys(ALL_KEYS, &variants, &find);
    let missing: Vec<_> = normal.missing.iter().filter(|key| compact.missing.contains(key)).collect();
    assert!(missing.is_empty(), "{} keys in no variant and no density: {missing:?}", missing.len());
    assert!(normal.wrong.is_empty(), "{} values of another type: {:#?}", normal.wrong.len(), normal.wrong);
    assert!(compact.wrong.is_empty(), "{} values of another type: {:#?}", compact.wrong.len(), compact.wrong);
    assert!(normal.checked_values > 0 && compact.checked_values > 0);
}
