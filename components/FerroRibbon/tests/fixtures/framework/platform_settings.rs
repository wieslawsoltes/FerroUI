//! Test platform settings: reuses actual native event storage and disposal.
use ferroui_base::{
    Size,
    input::{PointerType, platform::PlatformHotkeyConfiguration},
    platform::{DefaultPlatformSettings, IPlatformSettings, PlatformColorValues},
    reactive::{Disposable, IDisposable},
};
use std::{cell::Cell, rc::Rc, time::Duration};
#[derive(Default)]
pub struct SubscriptionCounts {
    pub adds: Cell<usize>,
    pub removes: Cell<usize>,
}
pub struct Settings {
    base: DefaultPlatformSettings,
    values: Cell<PlatformColorValues>,
    pub counts: Rc<SubscriptionCounts>,
}
impl Settings {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: DefaultPlatformSettings::new(),
            values: Cell::new(PlatformColorValues::new()),
            counts: Rc::new(SubscriptionCounts::default()),
        })
    }
    pub fn set_colors(&self, values: PlatformColorValues) {
        self.values.set(values);
        self.base.on_color_values_changed(values);
    }
}
impl IPlatformSettings for Settings {
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
        self.base.preferred_application_language()
    }
    fn get_color_values(&self) -> PlatformColorValues {
        self.values.get()
    }
    fn color_values_changed(&self, h: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        self.counts.adds.set(self.counts.adds.get() + 1);
        let token = self.base.color_values_changed(h);
        let counts = self.counts.clone();
        Disposable::create(move || {
            counts.removes.set(counts.removes.get() + 1);
            token.dispose();
        })
    }
    fn preferred_application_language_changed(&self, h: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.preferred_application_language_changed(h)
    }
}
