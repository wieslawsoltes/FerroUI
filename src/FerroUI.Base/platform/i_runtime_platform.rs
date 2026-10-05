/// Describes the platform the application runs on.
pub trait IRuntimePlatform: 'static {
    /// Information about the platform the application runs on.
    fn get_runtime_info(&self) -> RuntimePlatformInfo;

    // CLOCK-SEAM: the wall clock of the platform. The dispatcher clock
    // (`IDispatcherImpl::now`) is monotonic and has no relation to calendar
    // time, so "now" as a date is asked of the platform here. A platform
    // without `std::time::SystemTime` — the browser (`wasm32-unknown-unknown`) —
    // MUST register a runtime platform that overrides this (`Date.now()`);
    // otherwise every "now" is the Unix epoch.
    /// The current date and time in UTC, read by
    /// [`DateTime::utc_now`](crate::utilities::DateTime::utc_now). The
    /// default is for a platform without a wall clock: it reports the Unix
    /// epoch, 1970-01-01T00:00:00 UTC.
    fn get_utc_now(&self) -> crate::utilities::DateTime {
        crate::utilities::DateTime::UNIX_EPOCH
    }
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IRuntimePlatform {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// Information about the platform the application runs on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RuntimePlatformInfo {
    /// Whether the platform is a desktop platform.
    pub is_desktop: bool,
    /// Whether the platform is a mobile platform.
    pub is_mobile: bool,
    /// Whether the platform is a TV platform.
    pub is_tv: bool,
}

impl RuntimePlatformInfo {
    /// The form factor of the platform.
    pub fn form_factor(&self) -> FormFactorType {
        if self.is_desktop {
            FormFactorType::Desktop
        } else if self.is_mobile {
            FormFactorType::Mobile
        } else if self.is_tv {
            FormFactorType::TV
        } else {
            FormFactorType::Unknown
        }
    }
}

/// The form factor of a device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FormFactorType {
    #[default]
    Unknown = 0,
    Desktop = 1,
    Mobile = 2,
    TV = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_factor_prefers_desktop_then_mobile_then_tv() {
        assert_eq!(RuntimePlatformInfo::default().form_factor(), FormFactorType::Unknown);
        let all = RuntimePlatformInfo { is_desktop: true, is_mobile: true, is_tv: true };
        assert_eq!(all.form_factor(), FormFactorType::Desktop);
        let mobile_tv = RuntimePlatformInfo { is_desktop: false, is_mobile: true, is_tv: true };
        assert_eq!(mobile_tv.form_factor(), FormFactorType::Mobile);
        let tv = RuntimePlatformInfo { is_desktop: false, is_mobile: false, is_tv: true };
        assert_eq!(tv.form_factor(), FormFactorType::TV);
    }
}
