//! Creating the Fluent and the Simple theme as the theme of an application,
//! and finding the control theme of a control class in a theme.

use crate::harness::Registry;
use ferroui_base::controls::ResourceKey;
use ferroui_base::styling::{Style, Styles, ThemeVariant};
use ferroui_base::{BoxedValue, Ref, TypeInfo};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Button, DatePicker, TextBox};
use ferroui_themes_fluent::FluentTheme;
use ferroui_themes_simple::SimpleTheme;

pub struct ThemeBenchmark {
    reusable_fluent_theme: Ref<FluentTheme>,
    reusable_simple_theme: Ref<SimpleTheme>,
    /// Disposed when the benchmark is dropped, after the themes (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl ThemeBenchmark {
    pub fn new() -> Self {
        // The initialisers of the fields run before the body of the constructor.
        let reusable_fluent_theme = FluentTheme::new();
        let reusable_simple_theme = SimpleTheme::new();

        let mut services = TestServices::styled_window();
        services.theme = None;
        let app = UnitTestApplication::start(services);
        // Add empty style to override it later
        Self::current().styles().add(&Style::new());

        Self { reusable_fluent_theme, reusable_simple_theme, _app: app }
    }

    pub fn init_fluent_theme(&self) -> bool {
        let application = Self::current();
        application.styles().set(0, FluentTheme::new().as_style());
        application.try_get_resource(&ResourceKey::from("SystemAccentColor"), None).is_some()
    }

    pub fn init_simple_theme(&self) -> bool {
        let application = Self::current();
        application.styles().set(0, SimpleTheme::new().as_style());
        application.try_get_resource(&ResourceKey::from("ThemeAccentColor"), None).is_some()
    }

    pub fn find_fluent_control_theme(&self, type_: &'static TypeInfo) -> Option<BoxedValue> {
        // The lookup of a collection of styles: upstream the lookup of the theme itself, which
        // looks at its density first, is reached through the contract of a resource node only.
        let styles: &Styles = &self.reusable_fluent_theme;
        styles.try_get_resource(&ResourceKey::Type(type_), Some(&ThemeVariant::default())).flatten()
    }

    pub fn find_simple_control_theme(&self, type_: &'static TypeInfo) -> Option<BoxedValue> {
        let styles: &Styles = &self.reusable_simple_theme;
        styles.try_get_resource(&ResourceKey::Type(type_), Some(&ThemeVariant::default())).flatten()
    }

    fn current() -> Ref<UnitTestApplication> {
        UnitTestApplication::current().expect("the application of the benchmark")
    }
}

pub fn register(registry: &mut Registry) {
    let types: [&'static TypeInfo; 3] = [Button::TYPE, TextBox::TYPE, DatePicker::TYPE];

    let mut class = registry.class("themes", "ThemeBenchmark");
    class.benchmark("init_fluent_theme", "", ThemeBenchmark::new, |b| b.init_fluent_theme());
    class.benchmark("init_simple_theme", "", ThemeBenchmark::new, |b| b.init_simple_theme());
    for type_ in types {
        class.benchmark(
            "find_fluent_control_theme",
            format!("type={}", type_.name()),
            ThemeBenchmark::new,
            move |b| b.find_fluent_control_theme(type_),
        );
    }
    for type_ in types {
        class.benchmark(
            "find_simple_control_theme",
            format!("type={}", type_.name()),
            ThemeBenchmark::new,
            move |b| b.find_simple_control_theme(type_),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn theme_benchmark() {
        crate::harness::smoke_class(super::register, "ThemeBenchmark");
    }
}
