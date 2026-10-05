//! The test types declared by the upstream test file `Xaml/ThemeDictionariesTests.cs`:
//! the test class itself, as the owner of the static theme variant its
//! documents name with `x:Static`.

use ferroui_base::metadata::MarkupTyped;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_markup_type, ferro_static_type, StaticType};

use crate::support::TypeModule;

/// The owner of the custom theme variant of the tests.
pub struct ThemeDictionariesTests;

ferro_static_type!(ThemeDictionariesTests);

impl ThemeDictionariesTests {
    /// The theme variant `Custom`, which inherits the light one.
    pub fn custom() -> ThemeVariant {
        thread_local! {
            static CUSTOM: ThemeVariant = ThemeVariant::new("Custom", Some(ThemeVariant::light()));
        }
        CUSTOM.with(ThemeVariant::clone)
    }
}

ferro_markup_type!(static ThemeDictionariesTests {
    type_info: ThemeDictionariesTests,
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    fields: [
        Custom: ThemeVariant => ThemeDictionariesTests::custom,
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[<ThemeDictionariesTests as StaticType>::TYPE],
    markup_types: &[<ThemeDictionariesTests as MarkupTyped>::MARKUP],
    value_types: || {},
};
