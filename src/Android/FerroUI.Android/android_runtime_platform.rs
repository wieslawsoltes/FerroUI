use ferroui_base::platform::RuntimePlatformInfo;

/// The platform information for a device with the given system features.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn runtime_info(is_desktop: bool, is_tv: bool) -> RuntimePlatformInfo {
    RuntimePlatformInfo { is_desktop, is_mobile: !is_tv && !is_desktop, is_tv }
}

#[cfg(target_os = "android")]
pub use imp::{AndroidRuntimePlatform, AndroidRuntimePlatformServices};

#[cfg(target_os = "android")]
mod imp {
    use super::runtime_info;
    use crate::ferro_android_application::FerroAndroidApplication;
    use crate::interop::java::{call_boolean, call_object, JavaObject, JavaValue};
    use ferroui_base::platform::{
        AssetAssembly, IAssetLoader, IRuntimePlatform, RuntimePlatformInfo, StandardAssetLoader,
        StandardRuntimePlatform,
    };
    use ferroui_base::utilities::DateTime;
    use ferroui_base::FerroLocator;
    use ferroui_controls::AppBuilder;
    use std::rc::Rc;
    use std::sync::OnceLock;

    /// Registers the runtime platform services of Android.
    pub struct AndroidRuntimePlatformServices;

    impl AndroidRuntimePlatformServices {
        /// Selects the runtime platform of Android for the application.
        pub fn use_android_runtime_platform_subsystem(builder: &AppBuilder) -> AppBuilder {
            builder.use_runtime_platform_subsystem(|| Self::register(None), "AndroidRuntimePlatform")
        }

        /// Registers the runtime platform and the asset loader with the
        /// current service locator. `assembly` is the default assembly of
        /// the asset loader.
        pub fn register(assembly: Option<&AssetAssembly>) {
            let asset_loader: Rc<dyn IAssetLoader> = Rc::new(StandardAssetLoader::new(assembly));
            FerroLocator::current_mutable()
                .bind::<dyn IRuntimePlatform>()
                .to_singleton::<AndroidRuntimePlatform>(|instance| instance)
                .bind::<dyn IAssetLoader>()
                .to_constant(asset_loader);
        }
    }

    static INFO: OnceLock<RuntimePlatformInfo> = OnceLock::new();

    /// The runtime platform of Android: the kind of device comes from the
    /// features of the system.
    #[derive(Default)]
    pub struct AndroidRuntimePlatform {
        base: StandardRuntimePlatform,
    }

    fn has_system_feature(context: &JavaObject, feature: &str) -> bool {
        match call_object(context, "getPackageManager", "()Landroid/content/pm/PackageManager;", &[]) {
            Some(package_manager) => call_boolean(
                &package_manager,
                "hasSystemFeature",
                "(Ljava/lang/String;)Z",
                &[JavaValue::String(feature)],
            ),
            None => false,
        }
    }

    fn is_running_on_desktop(context: &JavaObject) -> bool {
        has_system_feature(context, "org.chromium.arc")
            || has_system_feature(context, "org.chromium.arc.device_management")
    }

    fn is_running_on_tv(context: &JavaObject) -> bool {
        // PackageManager.FEATURE_LEANBACK
        has_system_feature(context, "android.software.leanback")
    }

    impl IRuntimePlatform for AndroidRuntimePlatform {
        fn get_runtime_info(&self) -> RuntimePlatformInfo {
            *INFO.get_or_init(|| {
                let context = FerroAndroidApplication::context();
                let is_desktop = is_running_on_desktop(&context);
                let is_tv = is_running_on_tv(&context);

                runtime_info(is_desktop, is_tv)
            })
        }

        fn get_utc_now(&self) -> DateTime {
            self.base.get_utc_now()
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the runtime platform.
    use super::*;

    #[test]
    fn a_device_is_mobile_unless_it_is_a_desktop_or_a_tv() {
        for (is_desktop, is_tv, expected) in [
            (false, false, (false, true, false)),
            (true, false, (true, false, false)),
            (false, true, (false, false, true)),
            (true, true, (true, false, true)),
        ] {
            let info = runtime_info(is_desktop, is_tv);

            assert_eq!(expected, (info.is_desktop, info.is_mobile, info.is_tv), "desktop={is_desktop} tv={is_tv}");
        }
    }
}
