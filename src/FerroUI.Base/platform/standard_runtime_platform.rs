use super::{IRuntimePlatform, RuntimePlatformInfo};
use crate::utilities::{DateTime, DateTimeKind};

/// The runtime platform of the standard targets: derives the platform
/// information from the operating system the application was built for.
#[derive(Clone, Copy, Debug, Default)]
pub struct StandardRuntimePlatform;

impl StandardRuntimePlatform {
    /// Creates the runtime platform.
    pub fn new() -> Self {
        Self
    }
}

impl IRuntimePlatform for StandardRuntimePlatform {
    fn get_runtime_info(&self) -> RuntimePlatformInfo {
        // The desktop form of the phone operating system is a desktop
        // platform, not a mobile one.
        let is_mac_catalyst = cfg!(all(target_os = "ios", target_abi = "macabi"));
        RuntimePlatformInfo {
            is_desktop: cfg!(any(
                target_os = "windows",
                target_os = "macos",
                target_os = "linux",
                target_os = "freebsd"
            )) || is_mac_catalyst,
            is_mobile: cfg!(target_os = "android") || (cfg!(target_os = "ios") && !is_mac_catalyst),
            is_tv: cfg!(target_os = "tvos"),
        }
    }

    // CLOCK-SEAM: the browser platform must supply the wall clock (see `IRuntimePlatform::get_utc_now`).
    /// The system clock. A target without one (`wasm32-unknown-unknown`)
    /// reports the Unix epoch; its platform layer registers a runtime
    /// platform of its own.
    fn get_utc_now(&self) -> DateTime {
        if cfg!(all(target_arch = "wasm32", target_os = "unknown")) {
            return DateTime::UNIX_EPOCH;
        }
        let epoch = DateTime::UNIX_EPOCH;
        let ticks = |duration: std::time::Duration| (duration.as_nanos() / 100).min(i64::MAX as u128) as i64;
        let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(elapsed) => epoch.try_add_ticks(ticks(elapsed)).unwrap_or(DateTime::MAX_VALUE),
            Err(error) => epoch.try_add_ticks(-ticks(error.duration())).unwrap_or(DateTime::MIN_VALUE),
        };
        now.specify_kind(DateTimeKind::Utc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::FormFactorType;

    #[test]
    fn reports_the_form_factor_of_the_build_target() {
        let info = StandardRuntimePlatform::new().get_runtime_info();
        if cfg!(any(target_os = "windows", target_os = "macos", target_os = "linux", target_os = "freebsd")) {
            assert!(info.is_desktop);
            assert!(!info.is_mobile);
            assert!(!info.is_tv);
            assert_eq!(info.form_factor(), FormFactorType::Desktop);
        }
    }
}
