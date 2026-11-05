use crate::interop::ferro_module;
use ferroui_base::platform::{
    AssetAssembly, IAssetLoader, IRuntimePlatform, RuntimePlatformInfo, StandardAssetLoader, StandardRuntimePlatform,
};
use ferroui_base::FerroLocator;
use ferroui_controls::AppBuilder;
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// Registers the runtime platform services of the browser.
pub struct BrowserRuntimePlatformServices;

impl BrowserRuntimePlatformServices {
    /// Selects the runtime platform of the browser for the application.
    pub fn use_browser_runtime_platform_subsystem(builder: &AppBuilder) -> AppBuilder {
        builder.use_runtime_platform_subsystem(|| Self::register(None), "BrowserRuntimePlatform")
    }

    /// Registers the runtime platform and the asset loader with the current
    /// service locator. `assembly` is the default assembly of the asset
    /// loader.
    pub fn register(assembly: Option<&AssetAssembly>) {
        let asset_loader: Rc<dyn IAssetLoader> = Rc::new(StandardAssetLoader::new(assembly));
        FerroLocator::current_mutable()
            .bind::<dyn IRuntimePlatform>()
            .to_singleton::<BrowserRuntimePlatform>(|instance| instance)
            .bind::<dyn IAssetLoader>()
            .to_constant(asset_loader);
    }
}

ferroui_base::ferro_thread_local! {
    static INFO: Cell<Option<RuntimePlatformInfo>> = const { Cell::new(None) };
}

/// The runtime platform of the browser: the kind of device comes from the
/// page, and the wall clock is the page's `Date.now()`.
#[derive(Default)]
pub struct BrowserRuntimePlatform {
    base: StandardRuntimePlatform,
}

impl BrowserRuntimePlatform {
    /// The platform information for a device of the given kind.
    fn runtime_info(is_mobile: bool, is_tv: bool) -> RuntimePlatformInfo {
        RuntimePlatformInfo { is_mobile: is_mobile && !is_tv, is_desktop: !is_mobile && !is_tv, is_tv }
    }

    /// The instant of a reading of `Date.now()`, `milliseconds` since the
    /// Unix epoch: `UNIX_EPOCH` plus the duration (minus it before 1970). A
    /// reading that is not a number, or an instant the platform's
    /// `SystemTime` cannot hold (before 1970 on `wasm32-unknown-unknown`), is
    /// the epoch.
    fn system_time_of_date_now(milliseconds: f64) -> SystemTime {
        if !milliseconds.is_finite() {
            return SystemTime::UNIX_EPOCH;
        }
        let magnitude = milliseconds.abs();
        let whole = magnitude.trunc();
        let duration = Duration::from_millis(whole as u64)
            .checked_add(Duration::from_nanos(((magnitude - whole) * 1_000_000.0) as u64));
        let instant = duration.and_then(|duration| {
            if milliseconds < 0.0 {
                SystemTime::UNIX_EPOCH.checked_sub(duration)
            } else {
                SystemTime::UNIX_EPOCH.checked_add(duration)
            }
        });
        instant.unwrap_or(SystemTime::UNIX_EPOCH)
    }
}

impl IRuntimePlatform for BrowserRuntimePlatform {
    fn get_runtime_info(&self) -> RuntimePlatformInfo {
        INFO.with(|info| match info.get() {
            Some(value) => value,
            None => {
                let is_mobile = ferro_module::is_mobile();
                let is_tv = ferro_module::is_tv();
                let result = Self::runtime_info(is_mobile, is_tv);
                info.set(Some(result));
                result
            }
        })
    }

    // CLOCK-SEAM: the wall clock of the browser. A WebAssembly build reads the
    // page's `Date.now()` and never `SystemTime::now()`, which
    // `wasm32-unknown-unknown` does not have; the tests of the crate, which
    // run on the host, read the clock of the standard runtime platform.
    /// The current instant, from `Date.now()` of the page.
    fn get_utc_now(&self) -> SystemTime {
        if cfg!(target_arch = "wasm32") {
            Self::system_time_of_date_now(crate::interop::timer_helper::date_now())
        } else {
            self.base.get_utc_now()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::utilities::DateTime;

    #[test]
    fn the_browser_runtime_platform_reports_utc_now_as_a_system_time() {
        // A reading of `Date.now()` is the epoch plus its milliseconds.
        assert_eq!(
            SystemTime::UNIX_EPOCH + Duration::from_millis(1_717_236_000_123),
            BrowserRuntimePlatform::system_time_of_date_now(1_717_236_000_123.0)
        );
        assert_eq!(SystemTime::UNIX_EPOCH, BrowserRuntimePlatform::system_time_of_date_now(0.0));
        assert_eq!(
            SystemTime::UNIX_EPOCH + Duration::from_micros(1_500),
            BrowserRuntimePlatform::system_time_of_date_now(1.5)
        );
        assert_eq!(SystemTime::UNIX_EPOCH, BrowserRuntimePlatform::system_time_of_date_now(f64::NAN));
        // Before 1970, where the platform's system time can hold it.
        if let Some(before) = SystemTime::UNIX_EPOCH.checked_sub(Duration::from_secs(86_400)) {
            assert_eq!(before, BrowserRuntimePlatform::system_time_of_date_now(-86_400_000.0));
        }
        // The date of the reading.
        let date = DateTime::try_from(BrowserRuntimePlatform::system_time_of_date_now(1_717_236_000_000.0)).unwrap();
        assert_eq!(DateTime::new_with_time(2024, 6, 1, 10, 0, 0), date);

        // Through the trait; on the host, the tests read the system clock.
        let platform: &dyn IRuntimePlatform = &BrowserRuntimePlatform::default();
        let reference = SystemTime::now();
        let now = platform.get_utc_now();
        let distance = now.duration_since(reference).unwrap_or_else(|error| error.duration());
        assert!(distance < Duration::from_secs(60), "{now:?} is not near {reference:?}");
    }

    #[test]
    fn a_device_is_exactly_one_of_desktop_mobile_and_tv() {
        for (is_mobile, is_tv, expected) in [
            (false, false, (true, false, false)),
            (true, false, (false, true, false)),
            (false, true, (false, false, true)),
            (true, true, (false, false, true)),
        ] {
            let info = BrowserRuntimePlatform::runtime_info(is_mobile, is_tv);

            assert_eq!(expected, (info.is_desktop, info.is_mobile, info.is_tv), "mobile={is_mobile} tv={is_tv}");
        }
    }
}
