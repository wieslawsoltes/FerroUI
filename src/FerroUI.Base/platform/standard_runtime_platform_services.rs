use super::{AssetAssembly, IAssetLoader, IRuntimePlatform, StandardAssetLoader, StandardRuntimePlatform};
use crate::FerroLocator;
use std::rc::Rc;

/// Registers the services of the standard runtime platform.
pub struct StandardRuntimePlatformServices;

impl StandardRuntimePlatformServices {
    /// Registers the standard runtime platform and the standard asset
    /// loader with the current service locator. `assembly` is the default
    /// assembly of the asset loader.
    pub fn register(assembly: Option<&AssetAssembly>) {
        let asset_loader: Rc<dyn IAssetLoader> = Rc::new(StandardAssetLoader::new(assembly));
        FerroLocator::current_mutable()
            .bind::<dyn IRuntimePlatform>()
            .to_singleton::<StandardRuntimePlatform>(|instance| instance)
            .bind::<dyn IAssetLoader>()
            .to_constant(asset_loader);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::RuntimePlatformInfo;
    use crate::LocatorExtensions;

    #[test]
    fn register_binds_the_runtime_platform_and_the_asset_loader() {
        let scope = FerroLocator::enter_scope();
        StandardRuntimePlatformServices::register(None);

        let locator = FerroLocator::current();
        let platform = locator.get_service::<dyn IRuntimePlatform>();
        let platform = platform.expect("the runtime platform");
        let again = locator.get_service::<dyn IRuntimePlatform>().expect("the runtime platform");
        assert!(*platform == *again);
        assert_eq!(platform.get_runtime_info(), StandardRuntimePlatform::new().get_runtime_info());
        assert_ne!(platform.get_runtime_info(), RuntimePlatformInfo { is_desktop: true, is_mobile: true, is_tv: true });
        assert!(locator.get_service::<dyn IAssetLoader>().is_some());

        scope.dispose();
    }
}
