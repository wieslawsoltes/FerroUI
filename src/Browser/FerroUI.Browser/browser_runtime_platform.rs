use crate::interop::ferro_module;
use ferroui_base::platform::{
    AssetAssembly, IAssetLoader, IRuntimePlatform, RuntimePlatformInfo, StandardAssetLoader, StandardRuntimePlatform,
};
use ferroui_base::utilities::DateTime;
use ferroui_base::FerroLocator;
use ferroui_controls::AppBuilder;
use std::cell::Cell;
use std::rc::Rc;

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

thread_local! {
    static INFO: Cell<Option<RuntimePlatformInfo>> = const { Cell::new(None) };
}

/// The runtime platform of the browser: the kind of device comes from the
/// page.
#[derive(Default)]
pub struct BrowserRuntimePlatform {
    base: StandardRuntimePlatform,
}

impl BrowserRuntimePlatform {
    /// The platform information for a device of the given kind.
    fn runtime_info(is_mobile: bool, is_tv: bool) -> RuntimePlatformInfo {
        RuntimePlatformInfo { is_mobile: is_mobile && !is_tv, is_desktop: !is_mobile && !is_tv, is_tv }
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

    fn get_utc_now(&self) -> DateTime {
        self.base.get_utc_now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
