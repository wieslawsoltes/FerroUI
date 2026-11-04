//! Additional structural-archive assertion for the template companion.
//!
//! The retained activation fixture is preserved separately. This check fixes
//! the expected added value at 31 instead of making verification optional or
//! allowing an inherited environment value to choose the expectation.

use ferroui_base::{metadata::from_markup_value, styling::ThemeVariant};
use ferroui_themes_winui::WinUITheme;

pub(crate) fn assert_additional_edit_probe(theme: &WinUITheme) {
    let value = theme.try_get_resource(
        &"AdditionalEditProbe".into(),
        Some(&ThemeVariant::dark()),
    ).flatten();
    assert_eq!(
        from_markup_value::<f64>(&value),
        Some(31.0),
        "structural archive must expose the added AdditionalEditProbe resource",
    );
    println!("EDITABILITY AdditionalEditProbe 31");
}
