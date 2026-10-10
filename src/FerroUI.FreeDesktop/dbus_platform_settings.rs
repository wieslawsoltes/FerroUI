//! The colour scheme and the accent colour of the desktop, from the
//! settings portal (the port of `DBusPlatformSettings.cs`).

use crate::dbus_helper::DBusHelper;
use crate::signal_watch::watch_stream;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::PointerType;
use ferroui_base::media::Color;
use ferroui_base::platform::{
    DefaultPlatformSettings, IPlatformSettings, PlatformColorValues,
    PlatformThemeVariant,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Size;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
use zbus::proxy::CacheProperties;
use zbus::zvariant::{OwnedValue, Value};
use zbus::Connection;

/// `org.freedesktop.portal.Settings` (the proxy the reference generates
/// from `DBusXml/org.freedesktop.portal.Settings.xml`), with the members
/// that are called and the signal that is listened to.
#[zbus::proxy(interface = "org.freedesktop.portal.Settings", gen_blocking = false, assume_defaults = false)]
pub trait Settings {
    /// Deprecated by the portal; the value is wrapped in a second variant.
    fn read(&self, namespace: &str, key: &str) -> zbus::Result<OwnedValue>;

    fn read_one(&self, namespace: &str, key: &str) -> zbus::Result<OwnedValue>;

    #[zbus(signal)]
    fn setting_changed(&self, namespace: &str, key: &str, value: Value<'_>) -> zbus::Result<()>;

    #[zbus(property, name = "version")]
    fn version(&self) -> zbus::Result<u32>;
}

const APPEARANCE: &str = "org.freedesktop.appearance";

/// The value inside any number of variants.
fn unwrap_variant<'a, 'v>(mut value: &'a Value<'v>) -> &'a Value<'v> {
    while let Value::Value(inner) = value {
        value = inner;
    }
    value
}

/// The theme variant of the value of `color-scheme` (`ToColorScheme`);
/// `None` when the value is not a number.
pub(crate) fn to_color_scheme(value: &Value<'_>) -> Option<PlatformThemeVariant> {
    /*
    0: No preference
    1: Prefer dark appearance
    2: Prefer light appearance
    */
    let Value::U32(value) = unwrap_variant(value) else {
        return None;
    };
    let is_dark = *value == 1;
    Some(if is_dark { PlatformThemeVariant::Dark } else { PlatformThemeVariant::Light })
}

/// The colour of the value of `accent-color` (`ToAccentColor`).
pub(crate) fn to_accent_color(value: &Value<'_>) -> Option<Color> {
    /*
    Indicates the system's preferred accent color as a tuple of RGB values
    in the sRGB color space, in the range [0,1].
    Out-of-range RGB values should be treated as an unset accent color.
     */
    let Value::Structure(structure) = unwrap_variant(value) else {
        return None;
    };
    let component = |index: usize| match structure.fields().get(index).map(unwrap_variant) {
        Some(Value::F64(component)) => Some(*component),
        _ => None,
    };
    let (r, g, b) = (component(0)?, component(1)?, component(2)?);
    let in_range = |component: f64| (0.0..=1.0).contains(&component);
    if !in_range(r) || !in_range(g) || !in_range(b) {
        return None;
    }

    Some(Color::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8))
}

/// The colour values of what the portal told (`BuildPlatformColorValues`).
pub(crate) fn build_platform_color_values(
    nullable_theme_variant: Option<PlatformThemeVariant>,
    nullable_accent_color: Option<Color>,
) -> PlatformColorValues {
    match (nullable_theme_variant, nullable_accent_color) {
        (Some(theme_variant), Some(accent_color)) => {
            PlatformColorValues::new().with_theme_variant(theme_variant).with_accent_color1(accent_color)
        }
        (Some(theme_variant), None) => PlatformColorValues::new().with_theme_variant(theme_variant),
        (None, Some(accent_color)) => PlatformColorValues::new().with_accent_color1(accent_color),
        (None, None) => PlatformColorValues::new().with_theme_variant(PlatformThemeVariant::Light),
    }
}

/// The platform settings of a FreeDesktop session: the defaults of the
/// framework, with the colour values of the settings portal.
///
/// The reference class derives from the default platform settings; here
/// it holds them and answers the other members of the contract with them.
pub struct DBusPlatformSettings {
    base: DefaultPlatformSettings,
    settings: RefCell<Option<SettingsProxy<'static>>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    color_values: Cell<PlatformColorValues>,
    theme_variant: Cell<Option<PlatformThemeVariant>>,
    accent_color: Cell<Option<Color>>,
}

impl DBusPlatformSettings {
    /// The settings over the default connection of the process; the
    /// defaults when there is none.
    ///
    /// # Panics
    /// Panics when called from a thread other than the UI thread.
    pub fn new() -> Rc<Self> {
        Self::with_connection(DBusHelper::default_connection())
    }

    /// The settings over a connection.
    pub fn with_connection(connection: Option<Connection>) -> Rc<Self> {
        let base = DefaultPlatformSettings::new();
        let color_values = base.get_color_values();
        let this = Rc::new(Self {
            base,
            settings: RefCell::new(None),
            subscription: RefCell::new(None),
            color_values: Cell::new(color_values),
            theme_variant: Cell::new(None),
            accent_color: Cell::new(None),
        });

        let Some(conn) = connection else {
            return this;
        };

        // The proxy, the subscription to the changes and then the first
        // values, as one task: the reference starts the subscription and
        // the reads without awaiting either, and its proxy needs no call
        // to be made.
        let weak = Rc::downgrade(&this);
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let settings = SettingsProxy::builder(&conn)
                .destination("org.freedesktop.portal.Desktop")
                .and_then(|builder| builder.path("/org/freedesktop/portal/desktop"))
                .map(|builder| builder.cache_properties(CacheProperties::No));
            let Ok(settings) = settings else {
                return;
            };
            let Ok(settings) = settings.build().await else {
                return;
            };
            match weak.upgrade() {
                Some(this) => *this.settings.borrow_mut() = Some(settings.clone()),
                None => return,
            }

            if let Ok(changes) = settings.receive_setting_changed().await {
                let handler = weak.clone();
                let subscription = watch_stream(changes, move |signal| {
                    if let (Some(this), Ok(args)) = (handler.upgrade(), signal.args()) {
                        this.settings_changed_handler(args.namespace(), args.key(), args.value());
                    }
                });
                match weak.upgrade() {
                    Some(this) => *this.subscription.borrow_mut() = Some(subscription),
                    None => subscription.dispose(),
                }
            }

            let theme_variant = Self::try_get_theme_variant_async(&settings).await;
            let accent_color = Self::try_get_accent_color_async(&settings).await;
            if let Some(this) = weak.upgrade() {
                this.theme_variant.set(theme_variant);
                this.accent_color.set(accent_color);
                this.update_color_values();
            }
        }));

        this
    }

    async fn read(settings: &SettingsProxy<'static>, key: &str) -> zbus::Result<OwnedValue> {
        let version = settings.version().await?;
        if version >= 2 {
            settings.read_one(APPEARANCE, key).await
        } else {
            // The nested variant is unpacked when the value is looked at.
            settings.read(APPEARANCE, key).await
        }
    }

    async fn try_get_theme_variant_async(settings: &SettingsProxy<'static>) -> Option<PlatformThemeVariant> {
        let value = Self::read(settings, "color-scheme").await.ok()?;
        to_color_scheme(&value)
    }

    async fn try_get_accent_color_async(settings: &SettingsProxy<'static>) -> Option<Color> {
        let value = Self::read(settings, "accent-color").await.ok()?;
        to_accent_color(&value)
    }

    fn update_color_values(&self) {
        let old_color_values = self.color_values.get();
        let color_values = build_platform_color_values(self.theme_variant.get(), self.accent_color.get());

        if old_color_values != color_values {
            self.color_values.set(color_values);
            self.base.on_color_values_changed(color_values);
        }
    }

    fn settings_changed_handler(&self, namespace: &str, key: &str, value: &Value<'_>) {
        match (namespace, key) {
            (APPEARANCE, "color-scheme") => {
                // A value that is not a number is not a colour scheme: the
                // change is passed over, where the reference fails in the
                // handler.
                if let Some(theme_variant) = to_color_scheme(value) {
                    self.theme_variant.set(Some(theme_variant));
                    self.update_color_values();
                }
            }
            (APPEARANCE, "accent-color") => {
                self.accent_color.set(to_accent_color(value));
                self.update_color_values();
            }
            _ => {}
        }
    }
}

impl Drop for DBusPlatformSettings {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.borrow_mut().take() {
            subscription.dispose();
        }
    }
}

impl IPlatformSettings for DBusPlatformSettings {
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
        self.base.preferred_application_language()
    }

    fn get_color_values(&self) -> PlatformColorValues {
        self.color_values.get()
    }

    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        self.base.color_values_changed(handler)
    }

    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.preferred_application_language_changed(handler)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this class.
    use super::*;
    use crate::test_support::{log, pump_until, scope, Log, TestConnections};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};
    use zbus::zvariant::Structure;

    const PORTAL: &str = "org.freedesktop.portal.Desktop";
    const PATH: &str = "/org/freedesktop/portal/desktop";
    const INTERFACE: &str = "org.freedesktop.portal.Settings";

    fn rgb(r: f64, g: f64, b: f64) -> Value<'static> {
        Value::Structure(Structure::from((r, g, b)))
    }

    #[test]
    fn a_colour_scheme_is_dark_only_when_the_portal_says_dark() {
        assert_eq!(to_color_scheme(&Value::U32(0)), Some(PlatformThemeVariant::Light));
        assert_eq!(to_color_scheme(&Value::U32(1)), Some(PlatformThemeVariant::Dark));
        assert_eq!(to_color_scheme(&Value::U32(2)), Some(PlatformThemeVariant::Light));
        assert_eq!(to_color_scheme(&Value::U32(7)), Some(PlatformThemeVariant::Light));
        // The deprecated call wraps the value once more.
        assert_eq!(to_color_scheme(&Value::Value(Box::new(Value::U32(1)))), Some(PlatformThemeVariant::Dark));
        assert_eq!(to_color_scheme(&Value::from("dark")), None);
    }

    #[test]
    fn an_accent_colour_is_three_components_between_zero_and_one() {
        assert_eq!(to_accent_color(&rgb(1.0, 0.5, 0.0)), Some(Color::from_rgb(255, 127, 0)));
        assert_eq!(to_accent_color(&rgb(0.0, 0.0, 0.0)), Some(Color::from_rgb(0, 0, 0)));
        assert_eq!(to_accent_color(&Value::Value(Box::new(rgb(0.2, 0.4, 0.6)))), Some(Color::from_rgb(51, 102, 153)));
        // Out of range is "no accent colour".
        assert_eq!(to_accent_color(&rgb(1.5, 0.5, 0.0)), None);
        assert_eq!(to_accent_color(&rgb(0.5, -0.1, 0.0)), None);
        // Not three numbers.
        assert_eq!(to_accent_color(&Value::U32(1)), None);
        assert_eq!(to_accent_color(&Value::Structure(Structure::from((1.0, 0.5)))), None);
        assert_eq!(to_accent_color(&Value::Structure(Structure::from((1u32, 0u32, 0u32)))), None);
    }

    #[test]
    fn colour_values_keep_the_defaults_for_what_the_portal_does_not_tell() {
        let defaults = PlatformColorValues::new();
        let red = Color::from_rgb(255, 0, 0);
        assert_eq!(build_platform_color_values(None, None), defaults);
        assert_eq!(
            build_platform_color_values(Some(PlatformThemeVariant::Dark), None),
            defaults.with_theme_variant(PlatformThemeVariant::Dark)
        );
        assert_eq!(build_platform_color_values(None, Some(red)), defaults.with_accent_color1(red));
        assert_eq!(
            build_platform_color_values(Some(PlatformThemeVariant::Dark), Some(red)),
            defaults.with_theme_variant(PlatformThemeVariant::Dark).with_accent_color1(red)
        );
    }

    /// A double of the settings portal.
    struct Portal {
        version: Arc<AtomicU32>,
        values: Arc<Mutex<Vec<(String, OwnedValue)>>>,
        log: Log,
    }

    impl Portal {
        fn value(&self, key: &str) -> zbus::fdo::Result<OwnedValue> {
            self.values
                .lock()
                .unwrap()
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.try_clone().unwrap())
                .ok_or_else(|| zbus::fdo::Error::Failed("Requested setting not found".to_string()))
        }
    }

    #[zbus::interface(name = "org.freedesktop.portal.Settings")]
    impl Portal {
        fn read(&self, namespace: &str, key: &str) -> zbus::fdo::Result<OwnedValue> {
            log(&self.log, format!("Read({namespace}, {key})"));
            // The value in a variant in the variant of the reply.
            let value = self.value(key)?;
            Ok(OwnedValue::try_from(Value::Value(Box::new(Value::from(value)))).unwrap())
        }

        fn read_one(&self, namespace: &str, key: &str) -> zbus::fdo::Result<OwnedValue> {
            log(&self.log, format!("ReadOne({namespace}, {key})"));
            self.value(key)
        }

        #[zbus(property, name = "version")]
        fn version(&self) -> u32 {
            self.version.load(Ordering::SeqCst)
        }
    }

    struct Fixture {
        connections: TestConnections,
        values: Arc<Mutex<Vec<(String, OwnedValue)>>>,
        log: Log,
    }

    fn fixture(version: u32, values: Vec<(&str, Value<'static>)>) -> Fixture {
        let log: Log = Arc::default();
        let values = Arc::new(Mutex::new(
            values.into_iter().map(|(key, value)| (key.to_string(), OwnedValue::try_from(value).unwrap())).collect(),
        ));
        let portal = Portal { version: Arc::new(AtomicU32::new(version)), values: Arc::clone(&values), log: log.clone() };
        let connections = TestConnections::new(PATH, move |builder| builder.serve_at(PATH, portal));
        connections.start(PORTAL);
        Fixture { connections, values, log }
    }

    fn recorded(settings: &Rc<DBusPlatformSettings>) -> (Rc<RefCell<Vec<PlatformColorValues>>>, Rc<dyn IDisposable>) {
        let changes = Rc::new(RefCell::new(Vec::new()));
        let sink = changes.clone();
        let subscription = settings.color_values_changed(Rc::new(move |values| sink.borrow_mut().push(*values)));
        (changes, subscription)
    }

    #[test]
    fn without_a_connection_the_settings_are_the_defaults() {
        let _scope = scope();
        let settings = DBusPlatformSettings::with_connection(None);
        assert_eq!(settings.get_color_values(), PlatformColorValues::new());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(settings.get_color_values(), PlatformColorValues::new());
        // The rest of the contract is the default one.
        let defaults = DefaultPlatformSettings::new();
        assert_eq!(settings.hold_wait_duration(), defaults.hold_wait_duration());
        assert_eq!(settings.get_tap_size(PointerType::Touch), defaults.get_tap_size(PointerType::Touch));
    }

    #[test]
    fn the_first_values_are_read_from_the_portal_and_changes_follow() {
        let _scope = scope();
        let f = fixture(2, vec![("color-scheme", Value::U32(1)), ("accent-color", rgb(1.0, 0.0, 0.0))]);

        let settings = DBusPlatformSettings::with_connection(Some(f.connections.client.clone()));
        let (changes, _subscription) = recorded(&settings);
        // Before the portal answered: the defaults.
        assert_eq!(settings.get_color_values(), PlatformColorValues::new());

        let dark_red = PlatformColorValues::new()
            .with_theme_variant(PlatformThemeVariant::Dark)
            .with_accent_color1(Color::from_rgb(255, 0, 0));
        pump_until(|| settings.get_color_values() == dark_red);
        assert_eq!(*changes.borrow(), [dark_red]);
        assert_eq!(
            *f.log.lock().unwrap(),
            ["ReadOne(org.freedesktop.appearance, color-scheme)", "ReadOne(org.freedesktop.appearance, accent-color)"]
        );

        // The user switches to the light scheme.
        f.connections.emit(PATH, INTERFACE, "SettingChanged", &(APPEARANCE, "color-scheme", Value::U32(2)));
        let light_red = dark_red.with_theme_variant(PlatformThemeVariant::Light);
        pump_until(|| settings.get_color_values() == light_red);
        assert_eq!(*changes.borrow(), [dark_red, light_red]);

        // The same scheme again, a setting of another namespace and another key: no change.
        f.connections.emit(PATH, INTERFACE, "SettingChanged", &(APPEARANCE, "color-scheme", Value::U32(0)));
        f.connections.emit(PATH, INTERFACE, "SettingChanged", &("org.gnome.desktop.interface", "color-scheme", Value::U32(1)));
        f.connections.emit(PATH, INTERFACE, "SettingChanged", &(APPEARANCE, "contrast", Value::U32(1)));
        f.connections.settle();
        assert_eq!(changes.borrow().len(), 2);

        // A new accent colour; one out of range unsets it.
        f.connections.emit(PATH, INTERFACE, "SettingChanged", &(APPEARANCE, "accent-color", rgb(0.0, 0.0, 1.0)));
        let light_blue = light_red.with_accent_color1(Color::from_rgb(0, 0, 255));
        pump_until(|| settings.get_color_values() == light_blue);
        f.connections.emit(PATH, INTERFACE, "SettingChanged", &(APPEARANCE, "accent-color", rgb(2.0, 0.0, 1.0)));
        let light = PlatformColorValues::new().with_theme_variant(PlatformThemeVariant::Light);
        pump_until(|| settings.get_color_values() == light);
        assert_eq!(*changes.borrow(), [dark_red, light_red, light_blue, light]);
        let _ = &f.values;
    }

    #[test]
    fn an_old_portal_is_read_with_the_deprecated_call() {
        let _scope = scope();
        let f = fixture(1, vec![("color-scheme", Value::U32(1))]);
        let settings = DBusPlatformSettings::with_connection(Some(f.connections.client.clone()));
        let dark = PlatformColorValues::new().with_theme_variant(PlatformThemeVariant::Dark);
        pump_until(|| settings.get_color_values() == dark);
        f.connections.settle();
        // The accent colour is not there: the default stays.
        assert_eq!(settings.get_color_values(), dark);
        assert_eq!(
            *f.log.lock().unwrap(),
            ["Read(org.freedesktop.appearance, color-scheme)", "Read(org.freedesktop.appearance, accent-color)"]
        );
    }

    #[test]
    fn a_portal_that_is_not_there_leaves_the_defaults() {
        let _scope = scope();
        // The connection is there and nobody answers for the portal.
        let connections = TestConnections::new("/org/freedesktop/DBus", Ok);
        let settings = DBusPlatformSettings::with_connection(Some(connections.client.clone()));
        let (changes, _subscription) = recorded(&settings);
        for _ in 0..3 {
            Dispatcher::ui_thread().run_jobs(None);
            std::thread::sleep(Duration::from_millis(20));
        }
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(settings.get_color_values(), PlatformColorValues::new());
        assert!(changes.borrow().is_empty());
    }
}
