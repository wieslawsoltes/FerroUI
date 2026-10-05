//! Port of `ViewModels/PlatformSettingsViewModel.cs`.

use ferroui_base::animation::TimeSpan;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::PointerType;
use ferroui_base::media::{Color, Colors};
use ferroui_base::platform::{IPlatformSettings, PlatformColorValues};
use ferroui_base::reactive::IDisposable;
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

const NOT_AVAILABLE: &str = "Not available";

/// The view model of the platform settings page: the values of the
/// platform settings it is subscribed to.
pub struct PlatformSettingsViewModel {
    base: ViewModelBase,
    platform_settings: RefCell<Option<Rc<dyn IPlatformSettings>>>,
    color_values: Cell<Option<PlatformColorValues>>,
    preferred_language: RefCell<Option<String>>,
    /// The subscriptions to the two events of the platform settings.
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    this: Weak<PlatformSettingsViewModel>,
}

impl PartialEq for PlatformSettingsViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for PlatformSettingsViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl PlatformSettingsViewModel {
    pub fn new() -> Rc<PlatformSettingsViewModel> {
        Rc::new_cyclic(|this| Self {
            base: ViewModelBase::new(),
            platform_settings: RefCell::new(None),
            color_values: Cell::new(None),
            preferred_language: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            this: this.clone(),
        })
    }

    pub fn subscribe(&self, platform_settings: Option<Rc<dyn IPlatformSettings>>) {
        *self.platform_settings.borrow_mut() = platform_settings.clone();

        if let Some(platform_settings) = platform_settings {
            self.on_color_values_changed(&platform_settings.get_color_values());
            self.on_preferred_language_changed(&platform_settings);

            let color_values_changed = {
                let this = self.this.clone();
                platform_settings.color_values_changed(Rc::new(move |e: &PlatformColorValues| {
                    if let Some(this) = this.upgrade() {
                        this.on_color_values_changed(e);
                    }
                }))
            };
            let preferred_language_changed = {
                let this = self.this.clone();
                let sender = Rc::downgrade(&platform_settings);
                platform_settings.preferred_application_language_changed(Rc::new(move || {
                    if let (Some(this), Some(sender)) = (this.upgrade(), sender.upgrade()) {
                        this.on_preferred_language_changed(&sender);
                    }
                }))
            };
            self.subscriptions.borrow_mut().extend([color_values_changed, preferred_language_changed]);
        }
    }

    pub fn unsubscribe(&self) {
        if self.platform_settings.borrow().is_some() {
            let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
            for subscription in subscriptions {
                subscription.dispose();
            }
            *self.platform_settings.borrow_mut() = None;
        }
    }

    fn on_color_values_changed(&self, e: &PlatformColorValues) {
        self.color_values.set(Some(*e));
        self.base.raise_property_changed("ThemeVariant");
        self.base.raise_property_changed("ContrastPreference");
        self.base.raise_property_changed("AccentColor1");
        self.base.raise_property_changed("AccentColor2");
        self.base.raise_property_changed("AccentColor3");
    }

    fn on_preferred_language_changed(&self, sender: &Rc<dyn IPlatformSettings>) {
        *self.preferred_language.borrow_mut() = Some(sender.preferred_application_language());
        self.base.raise_property_changed("PreferredLanguage");
    }

    fn settings(&self) -> Option<Rc<dyn IPlatformSettings>> {
        self.platform_settings.borrow().clone()
    }

    pub fn preferred_language(&self) -> String {
        self.preferred_language.borrow().clone().unwrap_or_else(|| NOT_AVAILABLE.to_string())
    }

    pub fn theme_variant(&self) -> String {
        self.color_values.get().map_or_else(|| NOT_AVAILABLE.to_string(), |values| format!("{:?}", values.theme_variant()))
    }

    pub fn contrast_preference(&self) -> String {
        self.color_values
            .get()
            .map_or_else(|| NOT_AVAILABLE.to_string(), |values| format!("{:?}", values.contrast_preference()))
    }

    pub fn accent_color1(&self) -> Color {
        self.color_values.get().map_or(Colors::GRAY, |values| values.accent_color1())
    }

    pub fn accent_color2(&self) -> Color {
        self.color_values.get().map_or(Colors::GRAY, |values| values.accent_color2())
    }

    pub fn accent_color3(&self) -> Color {
        self.color_values.get().map_or(Colors::GRAY, |values| values.accent_color3())
    }

    pub fn hold_wait_duration(&self) -> String {
        self.settings().map_or_else(
            || NOT_AVAILABLE.to_string(),
            |settings| TimeSpan::from(settings.hold_wait_duration()).to_string(),
        )
    }

    pub fn tap_size_touch(&self) -> String {
        self.settings()
            .map_or_else(|| NOT_AVAILABLE.to_string(), |settings| settings.get_tap_size(PointerType::Touch).to_string())
    }

    pub fn tap_size_mouse(&self) -> String {
        self.settings()
            .map_or_else(|| NOT_AVAILABLE.to_string(), |settings| settings.get_tap_size(PointerType::Mouse).to_string())
    }

    pub fn double_tap_size_touch(&self) -> String {
        self.settings().map_or_else(
            || NOT_AVAILABLE.to_string(),
            |settings| settings.get_double_tap_size(PointerType::Touch).to_string(),
        )
    }

    pub fn double_tap_size_mouse(&self) -> String {
        self.settings().map_or_else(
            || NOT_AVAILABLE.to_string(),
            |settings| settings.get_double_tap_size(PointerType::Mouse).to_string(),
        )
    }

    pub fn double_tap_time_touch(&self) -> String {
        self.settings().map_or_else(
            || NOT_AVAILABLE.to_string(),
            |settings| TimeSpan::from(settings.get_double_tap_time(PointerType::Touch)).to_string(),
        )
    }

    pub fn double_tap_time_mouse(&self) -> String {
        self.settings().map_or_else(
            || NOT_AVAILABLE.to_string(),
            |settings| TimeSpan::from(settings.get_double_tap_time(PointerType::Mouse)).to_string(),
        )
    }
}

ferro_markup_type!(class PlatformSettingsViewModel {
    this: Rc<PlatformSettingsViewModel>,
    handles: [PlatformSettingsViewModel, Rc<PlatformSettingsViewModel>, Option<Rc<PlatformSettingsViewModel>>],
    constructors: [() => PlatformSettingsViewModel::new],
    properties: [
        PreferredLanguage: String { get: |this: &Rc<PlatformSettingsViewModel>| this.preferred_language() },
        ThemeVariant: String { get: |this: &Rc<PlatformSettingsViewModel>| this.theme_variant() },
        ContrastPreference: String { get: |this: &Rc<PlatformSettingsViewModel>| this.contrast_preference() },
        AccentColor1: Color { get: |this: &Rc<PlatformSettingsViewModel>| this.accent_color1() },
        AccentColor2: Color { get: |this: &Rc<PlatformSettingsViewModel>| this.accent_color2() },
        AccentColor3: Color { get: |this: &Rc<PlatformSettingsViewModel>| this.accent_color3() },
        HoldWaitDuration: String { get: |this: &Rc<PlatformSettingsViewModel>| this.hold_wait_duration() },
        TapSizeTouch: String { get: |this: &Rc<PlatformSettingsViewModel>| this.tap_size_touch() },
        TapSizeMouse: String { get: |this: &Rc<PlatformSettingsViewModel>| this.tap_size_mouse() },
        DoubleTapSizeTouch: String { get: |this: &Rc<PlatformSettingsViewModel>| this.double_tap_size_touch() },
        DoubleTapSizeMouse: String { get: |this: &Rc<PlatformSettingsViewModel>| this.double_tap_size_mouse() },
        DoubleTapTimeTouch: String { get: |this: &Rc<PlatformSettingsViewModel>| this.double_tap_time_touch() },
        DoubleTapTimeMouse: String { get: |this: &Rc<PlatformSettingsViewModel>| this.double_tap_time_mouse() },
    ],
    notify_property_changed: PlatformSettingsViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_base::input::platform::PlatformHotkeyConfiguration;
    use ferroui_base::platform::PlatformThemeVariant;
    use ferroui_base::reactive::Disposable;
    use ferroui_base::Size;
    use std::time::Duration;

    #[derive(Default)]
    struct Settings {
        color_handlers: RefCell<Vec<Rc<dyn Fn(&PlatformColorValues)>>>,
        language_handlers: RefCell<Vec<Rc<dyn Fn()>>>,
        language: RefCell<String>,
        disposed: Rc<Cell<i32>>,
    }

    impl IPlatformSettings for Settings {
        fn get_tap_size(&self, type_: PointerType) -> Size {
            if type_ == PointerType::Touch { Size::new(10.0, 10.0) } else { Size::new(4.0, 4.0) }
        }

        fn get_double_tap_size(&self, _type: PointerType) -> Size {
            Size::new(16.0, 16.0)
        }

        fn get_double_tap_time(&self, _type: PointerType) -> Duration {
            Duration::from_millis(500)
        }

        fn hold_wait_duration(&self) -> Duration {
            Duration::from_millis(300)
        }

        fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration> {
            Rc::new(PlatformHotkeyConfiguration::default())
        }

        fn preferred_application_language(&self) -> String {
            self.language.borrow().clone()
        }

        fn get_color_values(&self) -> PlatformColorValues {
            PlatformColorValues::new().with_theme_variant(PlatformThemeVariant::Dark)
        }

        fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
            self.color_handlers.borrow_mut().push(handler);
            let disposed = self.disposed.clone();
            Disposable::create(move || disposed.set(disposed.get() + 1))
        }

        fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            self.language_handlers.borrow_mut().push(handler);
            let disposed = self.disposed.clone();
            Disposable::create(move || disposed.set(disposed.get() + 1))
        }
    }

    #[test]
    fn nothing_is_available_before_subscribing() {
        let view_model = PlatformSettingsViewModel::new();
        assert_eq!("Not available", view_model.preferred_language());
        assert_eq!("Not available", view_model.theme_variant());
        assert_eq!("Not available", view_model.contrast_preference());
        assert_eq!("Not available", view_model.hold_wait_duration());
        assert_eq!("Not available", view_model.tap_size_touch());
        assert_eq!(Colors::GRAY, view_model.accent_color1());
        view_model.subscribe(None);
        view_model.unsubscribe();
        assert_eq!("Not available", view_model.double_tap_time_mouse());
    }

    #[test]
    fn subscribing_reads_the_settings_and_follows_their_changes() {
        let settings = Rc::new(Settings::default());
        *settings.language.borrow_mut() = "en-US".to_string();
        let view_model = PlatformSettingsViewModel::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        view_model.subscribe(Some(settings.clone()));
        assert_eq!(
            vec!["ThemeVariant", "ContrastPreference", "AccentColor1", "AccentColor2", "AccentColor3", "PreferredLanguage"],
            *seen.borrow()
        );
        assert_eq!("Dark", view_model.theme_variant());
        assert_eq!("NoPreference", view_model.contrast_preference());
        assert_eq!("en-US", view_model.preferred_language());
        assert_eq!("00:00:00.3000000", view_model.hold_wait_duration());
        assert_eq!("00:00:00.5000000", view_model.double_tap_time_touch());
        assert_eq!("10, 10", view_model.tap_size_touch());
        assert_eq!("4, 4", view_model.tap_size_mouse());
        assert_eq!("16, 16", view_model.double_tap_size_mouse());
        assert_eq!(view_model.accent_color1(), view_model.accent_color2());

        *settings.language.borrow_mut() = "pl-PL".to_string();
        let handler = settings.language_handlers.borrow()[0].clone();
        handler();
        assert_eq!("pl-PL", view_model.preferred_language());
        let handler = settings.color_handlers.borrow()[0].clone();
        handler(&PlatformColorValues::new());
        assert_eq!("Light", view_model.theme_variant());

        view_model.unsubscribe();
        assert_eq!(2, settings.disposed.get());
        assert_eq!("Not available", view_model.hold_wait_duration());
        // The values read before stay, as upstream.
        assert_eq!("pl-PL", view_model.preferred_language());
    }
}
