use crate::frn_string::to_c_string;
use crate::interop::*;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::IPlatformBehaviorInhibition;
use ferroui_microcom::ComPtr;

/// Controls platform behaviours an application may need to inhibit.
pub struct PlatformBehaviorInhibition {
    native: ComPtr<IFrnPlatformBehaviorInhibition>,
}

impl PlatformBehaviorInhibition {
    pub(crate) fn new(native: ComPtr<IFrnPlatformBehaviorInhibition>) -> Self {
        Self { native }
    }
}

impl IPlatformBehaviorInhibition for PlatformBehaviorInhibition {
    /// Prevents (or allows again) the system from putting the application
    /// to sleep; `reason` is shown by the system as the reason.
    ///
    /// The native call completes synchronously: the returned future is
    /// complete.
    fn set_inhibit_app_sleep(&self, inhibit_app_sleep: bool, reason: &str) -> LocalBoxFuture<()> {
        self.native.set_inhibit_app_sleep(inhibit_app_sleep, Some(&to_c_string(reason)));
        Box::pin(std::future::ready(()))
    }
}
