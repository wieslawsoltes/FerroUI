//! The platform settings: the colour scheme, the contrast preference and
//! the tint colour of the system, and the preferred language of the user.
//!
//! The settings ask the system through [`ISystemSettings`], which the
//! tests replace with fixed values; on iOS the answers come from the
//! current trait collection, the preferred tint and the locale.

use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::PointerType;
use ferroui_base::media::Color;
use ferroui_base::platform::{
    ColorContrastPreference, DefaultPlatformSettings, IPlatformSettings, PlatformColorValues, PlatformThemeVariant,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Size;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// What the system says of its colours.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SystemColors {
    /// Whether the user interface style of the current traits is dark.
    pub is_dark: bool,
    /// Whether the accessibility contrast of the current traits is high.
    pub is_high_contrast: bool,
    /// The preferred tint colour, as red, green, blue and alpha between 0
    /// and 1, when the system has one.
    pub tint_color: Option<[f64; 4]>,
}

/// What the settings ask the system.
pub trait ISystemSettings {
    /// The colours of the current traits.
    fn colors(&self) -> SystemColors;

    /// The preferred languages of the user, the most preferred first.
    fn preferred_languages(&self) -> Vec<String>;
}

/// The colour values of the framework for the colours of the system.
pub fn color_values(colors: SystemColors) -> PlatformColorValues {
    let theme_variant = if colors.is_dark { PlatformThemeVariant::Dark } else { PlatformThemeVariant::Light };

    let contrast_preference =
        if colors.is_high_contrast { ColorContrastPreference::High } else { ColorContrastPreference::NoPreference };

    let values = PlatformColorValues::new().with_theme_variant(theme_variant).with_contrast_preference(contrast_preference);

    if let Some([red, green, blue, alpha]) = colors.tint_color {
        if red != 0.0 && green != 0.0 && blue != 0.0 && alpha != 0.0 {
            return values.with_accent_color1(Color::from_argb(
                (alpha * 255.0) as u8,
                (red * 255.0) as u8,
                (green * 255.0) as u8,
                (blue * 255.0) as u8,
            ));
        }
    }

    values
}

// TODO: ideally should be created per view/activity.
/// The platform settings of iOS.
pub struct PlatformSettings {
    base: DefaultPlatformSettings,
    color_values: Cell<Option<PlatformColorValues>>,
    last_language: RefCell<Option<String>>,
    system: Box<dyn ISystemSettings>,
}

impl PlatformSettings {
    /// Creates the settings over what answers for the system.
    pub fn with_system(system: Box<dyn ISystemSettings>) -> Rc<Self> {
        Rc::new(Self {
            base: DefaultPlatformSettings::new(),
            color_values: Cell::new(None),
            last_language: RefCell::new(None),
            system,
        })
    }

    fn get_uncached_color_values(&self) -> PlatformColorValues {
        color_values(self.system.colors())
    }

    /// The traits of a view or its tint colour changed.
    pub fn trait_collection_did_change(&self) {
        let old_color_values = self.color_values.get();
        let color_values = self.get_uncached_color_values();

        if old_color_values != Some(color_values) {
            self.color_values.set(Some(color_values));
            self.base.on_color_values_changed(color_values);
        }
    }

    fn query_preferred_application_language(&self) -> String {
        match self.system.preferred_languages().into_iter().next() {
            Some(language) if !language.is_empty() => language,
            _ => self.base.preferred_application_language(),
        }
    }

    /// The current locale of the system changed.
    pub fn on_preferred_language_changed(&self) {
        let old_language = self.last_language.borrow_mut().take();

        if old_language != Some(self.preferred_application_language()) {
            self.base.on_preferred_application_language_changed();
        }
    }
}

impl IPlatformSettings for PlatformSettings {
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
        let last_language = self.last_language.borrow().clone();
        match last_language {
            Some(language) => language,
            None => {
                let language = self.query_preferred_application_language();
                *self.last_language.borrow_mut() = Some(language.clone());
                language
            }
        }
    }

    fn get_color_values(&self) -> PlatformColorValues {
        match self.color_values.get() {
            Some(color_values) => color_values,
            None => {
                let color_values = self.get_uncached_color_values();
                self.color_values.set(Some(color_values));
                color_values
            }
        }
    }

    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        self.base.color_values_changed(handler)
    }

    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.preferred_application_language_changed(handler)
    }
}

#[cfg(target_os = "ios")]
mod uikit {
    use super::{ISystemSettings, PlatformSettings, SystemColors};
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2_core_foundation::CGFloat;
    use objc2_foundation::{
        NSCurrentLocaleDidChangeNotification, NSLocale, NSNotification, NSNotificationCenter, NSOperationQueue,
    };
    use objc2_ui_kit::{
        UIAccessibilityContrast, UIColor, UIConfigurationColorTransformerPreferredTint, UITraitCollection,
        UIUserInterfaceStyle,
    };
    use std::ptr::NonNull;
    use std::rc::Rc;

    /// The settings of the system, read from UIKit and Foundation.
    struct SystemSettings;

    impl ISystemSettings for SystemSettings {
        fn colors(&self) -> SystemColors {
            // SAFETY: the current trait collection is a property of the
            // thread that asks; the settings are an object of the main
            // thread (they are not `Send`), where UIKit sets it.
            let (style, contrast) = unsafe {
                let traits = UITraitCollection::currentTraitCollection();
                (traits.userInterfaceStyle(), traits.accessibilityContrast())
            };

            let clear = UIColor::clearColor();
            // SAFETY: the transformer is a constant block of UIKit (iOS 14
            // and later, the minimum of the port) that takes a colour and
            // returns one that it does not hand over the ownership of, so
            // the result is retained here.
            let tint_color: Option<Retained<UIColor>> = unsafe {
                UIConfigurationColorTransformerPreferredTint
                    .as_ref()
                    .and_then(|transformer| Retained::retain(transformer.call((NonNull::from(&*clear),)).as_ptr()))
            };

            let tint_color = tint_color.map(|tint_color| {
                let (mut red, mut green, mut blue, mut alpha): (CGFloat, CGFloat, CGFloat, CGFloat) =
                    (0.0, 0.0, 0.0, 0.0);
                // SAFETY: the four pointers are to variables of this
                // function, which live for the call.
                unsafe {
                    tint_color.getRed_green_blue_alpha(&mut red, &mut green, &mut blue, &mut alpha);
                }
                [red, green, blue, alpha]
            });

            SystemColors {
                is_dark: style == UIUserInterfaceStyle::Dark,
                is_high_contrast: contrast == UIAccessibilityContrast::High,
                tint_color,
            }
        }

        fn preferred_languages(&self) -> Vec<String> {
            NSLocale::preferredLanguages().iter().map(|language| language.to_string()).collect()
        }
    }

    impl PlatformSettings {
        /// Creates the settings of the system and observes the changes of
        /// its locale.
        pub fn new() -> Rc<Self> {
            let this = Self::with_system(Box::new(SystemSettings));

            let weak = Rc::downgrade(&this);
            let block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
                if let Some(this) = weak.upgrade() {
                    this.on_preferred_language_changed();
                }
            });
            // SAFETY: the name is a constant of Foundation. The block
            // takes the one argument of a notification block and is run
            // on the main queue, which is the thread the settings belong
            // to, whichever thread posts the notification.
            let observer = unsafe {
                NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                    Some(NSCurrentLocaleDidChangeNotification),
                    None,
                    Some(&NSOperationQueue::mainQueue()),
                    &block,
                )
            };
            // The settings live as long as the process, and so does the
            // subscription (the reference does not remove it either).
            std::mem::forget(observer);

            this
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;
    use std::cell::Cell;

    #[derive(Default)]
    struct FakeSystemState {
        colors: Cell<SystemColors>,
        languages: RefCell<Vec<String>>,
        color_queries: Cell<u32>,
        language_queries: Cell<u32>,
    }

    struct FakeSystem(Rc<FakeSystemState>);

    impl ISystemSettings for FakeSystem {
        fn colors(&self) -> SystemColors {
            self.0.color_queries.set(self.0.color_queries.get() + 1);
            self.0.colors.get()
        }

        fn preferred_languages(&self) -> Vec<String> {
            self.0.language_queries.set(self.0.language_queries.get() + 1);
            self.0.languages.borrow().clone()
        }
    }

    fn settings() -> (Rc<PlatformSettings>, Rc<FakeSystemState>) {
        let state = Rc::new(FakeSystemState::default());
        (PlatformSettings::with_system(Box::new(FakeSystem(state.clone()))), state)
    }

    #[test]
    fn the_style_and_the_contrast_of_the_traits_are_the_variant_and_the_preference() {
        let values = color_values(SystemColors::default());
        assert_eq!(PlatformThemeVariant::Light, values.theme_variant());
        assert_eq!(ColorContrastPreference::NoPreference, values.contrast_preference());
        assert_eq!(PlatformColorValues::new().accent_color1(), values.accent_color1());

        let values = color_values(SystemColors { is_dark: true, is_high_contrast: true, tint_color: None });
        assert_eq!(PlatformThemeVariant::Dark, values.theme_variant());
        assert_eq!(ColorContrastPreference::High, values.contrast_preference());
    }

    #[test]
    fn the_tint_colour_is_the_accent_colour() {
        let values =
            color_values(SystemColors { tint_color: Some([0.0, 0.478, 1.0, 1.0]), ..SystemColors::default() });
        // A colour with a component of zero is passed over, as in the
        // reference (the blue of the system has no red).
        assert_eq!(PlatformColorValues::new().accent_color1(), values.accent_color1());

        let values =
            color_values(SystemColors { tint_color: Some([1.0, 0.5, 0.25, 1.0]), ..SystemColors::default() });
        assert_eq!(Color::from_argb(255, 255, 127, 63), values.accent_color1());

        let values =
            color_values(SystemColors { tint_color: Some([1.0, 0.5, 0.25, 0.0]), ..SystemColors::default() });
        assert_eq!(PlatformColorValues::new().accent_color1(), values.accent_color1());
    }

    #[test]
    fn the_colour_values_are_asked_for_once_and_raised_when_they_change() {
        let (settings, state) = settings();
        let raised = Rc::new(RefCell::new(Vec::new()));
        let sink = raised.clone();
        let _subscription = settings
            .color_values_changed(Rc::new(move |values: &PlatformColorValues| sink.borrow_mut().push(*values)));

        assert_eq!(PlatformThemeVariant::Light, settings.get_color_values().theme_variant());
        assert_eq!(PlatformThemeVariant::Light, settings.get_color_values().theme_variant());
        assert_eq!(1, state.color_queries.get());

        // The same traits: nothing is raised.
        settings.trait_collection_did_change();
        assert!(raised.borrow().is_empty());

        state.colors.set(SystemColors { is_dark: true, ..SystemColors::default() });
        // The cached values answer until the view reports the change.
        assert_eq!(PlatformThemeVariant::Light, settings.get_color_values().theme_variant());
        settings.trait_collection_did_change();
        assert_eq!(1, raised.borrow().len());
        assert_eq!(PlatformThemeVariant::Dark, raised.borrow()[0].theme_variant());
        assert_eq!(PlatformThemeVariant::Dark, settings.get_color_values().theme_variant());

        settings.trait_collection_did_change();
        assert_eq!(1, raised.borrow().len());
    }

    #[test]
    fn a_change_of_the_traits_before_the_values_were_asked_for_is_raised() {
        let (settings, _state) = settings();
        let raised = Rc::new(Cell::new(0));
        let sink = raised.clone();
        let _subscription = settings.color_values_changed(Rc::new(move |_| sink.set(sink.get() + 1)));
        settings.trait_collection_did_change();
        assert_eq!(1, raised.get());
    }

    #[test]
    fn the_first_preferred_language_is_the_language_of_the_application() {
        let (settings, state) = settings();
        *state.languages.borrow_mut() = vec!["pl-PL".to_string(), "en-US".to_string()];
        assert_eq!("pl-PL", settings.preferred_application_language());
        assert_eq!("pl-PL", settings.preferred_application_language());
        assert_eq!(1, state.language_queries.get());
    }

    #[test]
    fn without_a_preferred_language_the_default_answers() {
        let (settings, state) = settings();
        let default = DefaultPlatformSettings::new().preferred_application_language();
        assert_eq!(default, settings.preferred_application_language());

        let (settings, state2) = (settings, state);
        *state2.languages.borrow_mut() = vec![String::new()];
        settings.on_preferred_language_changed();
        assert_eq!(default, settings.preferred_application_language());
    }

    #[test]
    fn a_change_of_the_locale_is_raised_when_the_language_changed() {
        let (settings, state) = settings();
        *state.languages.borrow_mut() = vec!["en-US".to_string()];
        let raised = Rc::new(Cell::new(0));
        let sink = raised.clone();
        let _subscription =
            settings.preferred_application_language_changed(Rc::new(move || sink.set(sink.get() + 1)));
        assert_eq!("en-US", settings.preferred_application_language());

        // The locale changed and the language did not.
        settings.on_preferred_language_changed();
        assert_eq!(0, raised.get());

        *state.languages.borrow_mut() = vec!["de-DE".to_string(), "en-US".to_string()];
        // The cached language answers until the notification.
        assert_eq!("en-US", settings.preferred_application_language());
        settings.on_preferred_language_changed();
        assert_eq!(1, raised.get());
        assert_eq!("de-DE", settings.preferred_application_language());
    }
}
