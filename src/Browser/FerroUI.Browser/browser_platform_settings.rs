use crate::interop::dom_helper;
use crate::windowing_platform::BrowserWindowingPlatform;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::PointerType;
use ferroui_base::platform::{
    ColorContrastPreference, DefaultPlatformSettings, IPlatformSettings, PlatformColorValues, PlatformThemeVariant,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Size;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// What the page reports: the dark-mode and contrast media queries and the
/// language of the browser. Replaced by fixed values in the tests.
trait IPageSettings {
    fn get_dark_mode(&self) -> Vec<i32>;
    fn get_navigator_language(&self) -> Option<String>;
}

struct PageSettings;

impl IPageSettings for PageSettings {
    fn get_dark_mode(&self) -> Vec<i32> {
        dom_helper::get_dark_mode(&BrowserWindowingPlatform::global_this())
    }

    fn get_navigator_language(&self) -> Option<String> {
        dom_helper::get_navigator_language(&BrowserWindowingPlatform::global_this())
    }
}

/// The platform settings of the browser: the colour scheme, the contrast
/// preference and the language of the page.
pub struct BrowserPlatformSettings {
    base: DefaultPlatformSettings,
    is_dark_mode: Cell<bool>,
    is_high_contrast: Cell<bool>,
    is_initialized: Cell<bool>,
    color_values: Cell<Option<PlatformColorValues>>,
    last_language: RefCell<Option<String>>,
    page: Box<dyn IPageSettings>,
}

impl Default for BrowserPlatformSettings {
    fn default() -> Self {
        Self::with_page(Box::new(PageSettings))
    }
}

impl BrowserPlatformSettings {
    /// Creates the settings. The page is asked on first use.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    fn with_page(page: Box<dyn IPageSettings>) -> Self {
        Self {
            base: DefaultPlatformSettings::new(),
            is_dark_mode: Cell::new(false),
            is_high_contrast: Cell::new(false),
            is_initialized: Cell::new(false),
            color_values: Cell::new(None),
            last_language: RefCell::new(None),
            page,
        }
    }

    fn build_platform_color_values(is_dark_mode: bool, is_high_contrast: bool) -> PlatformColorValues {
        PlatformColorValues::new()
            .with_theme_variant(if is_dark_mode { PlatformThemeVariant::Dark } else { PlatformThemeVariant::Light })
            .with_contrast_preference(if is_high_contrast {
                ColorContrastPreference::High
            } else {
                ColorContrastPreference::NoPreference
            })
    }

    /// The page reported a change of its colour scheme or contrast
    /// preference.
    pub fn on_color_values_changed(&self, is_dark_mode: bool, is_high_contrast: bool) {
        self.is_dark_mode.set(is_dark_mode);
        self.is_high_contrast.set(is_high_contrast);
        self.update_color_values();
    }

    /// The page reported a change of the language of the browser.
    pub fn on_preferred_language_changed(&self, language: Option<&str>) {
        if let Some(language) = language {
            if self.last_language.borrow().as_deref() != Some(language) {
                *self.last_language.borrow_mut() = Some(language.to_string());
                self.base.on_preferred_application_language_changed();
            }
        }
    }

    fn ensure_settings(&self) {
        if !self.is_initialized.get() {
            // The module is initialized asynchronously. The page cannot be asked right away while
            // the components are registered.
            self.is_initialized.set(true);
            let values = self.page.get_dark_mode();
            if values.len() == 2 {
                self.is_dark_mode.set(values[0] > 0);
                self.is_high_contrast.set(values[1] > 0);
            }

            *self.last_language.borrow_mut() = self.page.get_navigator_language();
        }
    }

    fn update_color_values(&self) {
        let old_color_values = self.color_values.get();
        let color_values = Self::build_platform_color_values(self.is_dark_mode.get(), self.is_high_contrast.get());

        if old_color_values != Some(color_values) {
            self.color_values.set(Some(color_values));
            self.base.on_color_values_changed(color_values);
        }
    }
}

impl IPlatformSettings for BrowserPlatformSettings {
    fn get_tap_size(&self, type_: PointerType) -> Size {
        self.base.get_tap_size(type_)
    }

    fn get_double_tap_size(&self, type_: PointerType) -> Size {
        self.base.get_double_tap_size(type_)
    }

    fn get_double_tap_time(&self, type_: PointerType) -> Duration {
        self.base.get_double_tap_time(type_)
    }

    fn hold_wait_duration(&self) -> Duration {
        self.base.hold_wait_duration()
    }

    fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration> {
        self.base.hotkey_configuration()
    }

    fn preferred_application_language(&self) -> String {
        self.ensure_settings();

        match self.last_language.borrow().clone() {
            Some(language) => language,
            None => self.base.preferred_application_language(),
        }
    }

    fn get_color_values(&self) -> PlatformColorValues {
        if let Some(color_values) = self.color_values.get() {
            return color_values;
        }

        self.ensure_settings();
        let color_values = Self::build_platform_color_values(self.is_dark_mode.get(), self.is_high_contrast.get());
        self.color_values.set(Some(color_values));
        color_values
    }

    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        self.ensure_settings();
        self.base.color_values_changed(handler)
    }

    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.ensure_settings();
        self.base.preferred_application_language_changed(handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedPage {
        dark_mode: Vec<i32>,
        language: Option<&'static str>,
        queries: Rc<Cell<u32>>,
    }

    impl IPageSettings for FixedPage {
        fn get_dark_mode(&self) -> Vec<i32> {
            self.queries.set(self.queries.get() + 1);
            self.dark_mode.clone()
        }

        fn get_navigator_language(&self) -> Option<String> {
            self.language.map(str::to_string)
        }
    }

    fn settings(dark_mode: &[i32], language: Option<&'static str>) -> (BrowserPlatformSettings, Rc<Cell<u32>>) {
        let queries = Rc::new(Cell::new(0));
        let page = FixedPage { dark_mode: dark_mode.to_vec(), language, queries: queries.clone() };
        (BrowserPlatformSettings::with_page(Box::new(page)), queries)
    }

    #[test]
    fn the_page_is_asked_once_and_only_when_needed() {
        let (settings, queries) = settings(&[1, 0], Some("de-DE"));
        assert_eq!(0, queries.get());

        settings.get_color_values();
        settings.get_color_values();
        settings.preferred_application_language();

        assert_eq!(1, queries.get());
    }

    #[test]
    fn the_colour_values_follow_the_media_queries() {
        for (dark_mode, theme, contrast) in [
            ([0, 0], PlatformThemeVariant::Light, ColorContrastPreference::NoPreference),
            ([1, 0], PlatformThemeVariant::Dark, ColorContrastPreference::NoPreference),
            ([0, 1], PlatformThemeVariant::Light, ColorContrastPreference::High),
            ([1, 1], PlatformThemeVariant::Dark, ColorContrastPreference::High),
        ] {
            let (settings, _) = settings(&dark_mode, None);

            let values = settings.get_color_values();

            assert_eq!(theme, values.theme_variant());
            assert_eq!(contrast, values.contrast_preference());
        }
    }

    #[test]
    fn an_answer_of_the_wrong_shape_leaves_the_defaults() {
        let (settings, _) = settings(&[1], None);

        assert_eq!(PlatformThemeVariant::Light, settings.get_color_values().theme_variant());
    }

    #[test]
    fn a_change_of_the_colour_scheme_is_raised_once_per_change() {
        let (settings, _) = settings(&[0, 0], None);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = seen.clone();
        let subscription =
            settings.color_values_changed(Rc::new(move |values| log.borrow_mut().push(values.theme_variant())));
        settings.get_color_values();

        settings.on_color_values_changed(true, false);
        settings.on_color_values_changed(true, false);
        settings.on_color_values_changed(false, false);
        subscription.dispose();
        settings.on_color_values_changed(true, false);

        assert_eq!(vec![PlatformThemeVariant::Dark, PlatformThemeVariant::Light], *seen.borrow());
        assert_eq!(PlatformThemeVariant::Dark, settings.get_color_values().theme_variant());
    }

    #[test]
    fn the_language_of_the_browser_is_preferred() {
        let (settings, _) = settings(&[0, 0], Some("pl-PL"));

        assert_eq!("pl-PL", settings.preferred_application_language());
    }

    #[test]
    fn a_change_of_the_language_is_raised_once_per_change() {
        let (settings, _) = settings(&[0, 0], Some("en-US"));
        let count = Rc::new(Cell::new(0));
        let seen = count.clone();
        let _subscription = settings.preferred_application_language_changed(Rc::new(move || seen.set(seen.get() + 1)));

        settings.on_preferred_language_changed(Some("en-US"));
        settings.on_preferred_language_changed(None);
        settings.on_preferred_language_changed(Some("fr-FR"));
        settings.on_preferred_language_changed(Some("fr-FR"));

        assert_eq!(1, count.get());
        assert_eq!("fr-FR", settings.preferred_application_language());
    }
}
