// Copyright 2026 The Android Open Source Project (system color resource mappings).
// Copyright 2026 FerroUI contributors (native resource bridge and lifecycle).
// SPDX-License-Identifier: Apache-2.0
//! Native Android wallpaper/system colors, without application JNI or Java glue.
//!
//! Resource mappings and neutral-tone calculations follow AndroidX Material3
//! `e4d933c98df7baa7ea2b83eea6e086915961e0ff`, `DynamicTonalPalette.android.kt`
//! and `src/androidMain/res/values-v{31,34}/styles.xml`.

use crate::{MaterialDynamicColorError, MaterialTheme};
use std::fmt;

/// Which native Android resource contract supplied a dynamic color scheme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterialAndroidColorSource {
    /// Android 12/13: the five wallpaper tonal palettes, with AndroidX neutral tones.
    Android31TonalPalette,
    /// Android 14 and later: complete system roles, including the device's contrast adjustments.
    Android34ColorRoles,
}

/// A native import failure. Existing theme colors remain installed when an import fails.
#[derive(Clone, Debug, PartialEq)]
pub enum MaterialAndroidColorError {
    UnsupportedPlatform,
    FeatureDisabled,
    ApplicationNotInitialized,
    ResourcesUnavailable,
    DynamicColorsUnavailable,
    IncompletePalette(MaterialDynamicColorError),
}

impl fmt::Display for MaterialAndroidColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                f.write_str("Android system colors are available on Android only")
            }
            Self::FeatureDisabled => f.write_str(
                "enable the android-dynamic-color feature to import Android system colors",
            ),
            Self::ApplicationNotInitialized => {
                f.write_str("the FerroUI Android application has not been initialized")
            }
            Self::ResourcesUnavailable => {
                f.write_str("the Android application has no Resources object")
            }
            Self::DynamicColorsUnavailable => {
                f.write_str("Android dynamic color resources require Android 12/API 31 or later")
            }
            Self::IncompletePalette(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for MaterialAndroidColorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IncompletePalette(error) => Some(error),
            _ => None,
        }
    }
}

impl From<MaterialDynamicColorError> for MaterialAndroidColorError {
    fn from(value: MaterialDynamicColorError) -> Self {
        Self::IncompletePalette(value)
    }
}

impl MaterialTheme {
    /// Imports the device's light and dark wallpaper color schemes and refreshes them when
    /// FerroUI's platform settings report changed colors or theme configuration.
    ///
    /// Enable the `android-dynamic-color` Cargo feature and call this after the Android
    /// application is initialized. The bridge obtains the existing application context itself;
    /// it requires no Java code, AndroidX library, resource packaging, or JNI setup in the app.
    /// Repeating this call replaces the previous subscription. The subscription is disposed
    /// when the theme is dropped or [`Self::stop_android_dynamic_colors`] is called.
    ///
    /// On API 34+ all system roles, including fixed colors and current system contrast, are
    /// imported directly. API 31–33 uses AndroidX's five tonal palettes and float CAM16/HCT
    /// neutral-tone derivation. Older systems return an error and retain the existing palette.
    /// Desktop builds do not link the Android backend and return `UnsupportedPlatform`.
    ///
    /// Call [`Self::refresh_android_dynamic_colors`] on a custom host notification if that host
    /// changes Android resource overlays without a FerroUI platform color-change notification.
    pub fn use_android_dynamic_colors(
        &self,
    ) -> Result<MaterialAndroidColorSource, MaterialAndroidColorError> {
        let source = self.refresh_android_dynamic_colors()?;
        #[cfg(all(target_os = "android", feature = "android-dynamic-color"))]
        native::subscribe(self);
        Ok(source)
    }

    /// Re-reads the current native palette. Both variants are validated before either is
    /// installed. This is also useful after custom Android resource-overlay changes.
    pub fn refresh_android_dynamic_colors(
        &self,
    ) -> Result<MaterialAndroidColorSource, MaterialAndroidColorError> {
        #[cfg(all(target_os = "android", feature = "android-dynamic-color"))]
        {
            use ferroui_base::styling::ThemeVariant;
            let schemes = native::read()?;
            self.set_color_scheme(ThemeVariant::light(), &schemes.light);
            self.set_color_scheme(ThemeVariant::dark(), &schemes.dark);
            self.set_color_scheme(ThemeVariant::default(), &schemes.dark);
            Ok(schemes.source)
        }
        #[cfg(all(target_os = "android", not(feature = "android-dynamic-color")))]
        {
            Err(MaterialAndroidColorError::FeatureDisabled)
        }
        #[cfg(not(target_os = "android"))]
        {
            Err(MaterialAndroidColorError::UnsupportedPlatform)
        }
    }

    /// Stops following platform changes while retaining the most recently imported colors.
    pub fn stop_android_dynamic_colors(&self) {
        let old = self.android_color_subscription.borrow_mut().take();
        if let Some(subscription) = old {
            subscription.dispose();
        }
    }
}

#[cfg(any(test, all(target_os = "android", feature = "android-dynamic-color")))]
#[path = "android_palette_math.rs"]
mod palette_math;

#[cfg(any(test, all(target_os = "android", feature = "android-dynamic-color")))]
mod mapping {
    use super::{palette_math, MaterialAndroidColorError, MaterialAndroidColorSource};
    use crate::{
        MaterialColorRole, MaterialColorScheme, MaterialDynamicPalette, MaterialPaletteFamily,
        MaterialTonalPalette,
    };
    use ferroui_base::media::Color;

    pub(super) struct Schemes {
        pub light: MaterialColorScheme,
        pub dark: MaterialColorScheme,
        pub source: MaterialAndroidColorSource,
    }

    pub(super) fn read(
        mut resolve: impl FnMut(&str) -> Option<Color>,
    ) -> Result<Schemes, MaterialAndroidColorError> {
        if resolve("system_primary_light").is_some() {
            let mut import = |dark| {
                MaterialColorScheme::from_android_system_roles(dark, |role| {
                    role_resource(role, dark).and_then(|name| resolve(&name))
                })
            };
            return Ok(Schemes {
                light: import(false)?,
                dark: import(true)?,
                source: MaterialAndroidColorSource::Android34ColorRoles,
            });
        }
        if resolve("system_accent1_0").is_none() {
            return Err(MaterialAndroidColorError::DynamicColorsUnavailable);
        }

        // The eleven additional neutral tones all derive from the *same* tone40 color.
        // Keep the platform's thirteen actual tones unchanged, including their alpha.
        let mut neutral40 = [None; 2];
        let palette = MaterialDynamicPalette::from_android31_colors(|family, tone| {
            let name = palette_resource(family, tone);
            if MaterialTonalPalette::ANDROID_SYSTEM_TONES.contains(&tone) {
                return resolve(&name);
            }
            let slot = match family {
                MaterialPaletteFamily::Neutral => 0,
                MaterialPaletteFamily::NeutralVariant => 1,
                _ => return None,
            };
            let base = match neutral40[slot] {
                Some(color) => color,
                None => {
                    let color = resolve(&palette_resource(family, 40))?;
                    neutral40[slot] = Some(color);
                    color
                }
            };
            Some(palette_math::set_luminance(base, tone))
        })?;
        Ok(Schemes {
            light: palette.scheme_android31(false)?,
            dark: palette.scheme_android31(true)?,
            source: MaterialAndroidColorSource::Android31TonalPalette,
        })
    }

    /// API34 public system color names, equivalent to AndroidX's mc3 scheme style mappings.
    pub(super) fn role_resource(role: MaterialColorRole, dark: bool) -> Option<String> {
        use MaterialColorRole as R;
        let (role, dark) = match role {
            R::Error | R::OnError | R::ErrorContainer | R::OnErrorContainer | R::Scrim => {
                return None
            }
            R::InversePrimary => (R::Primary, !dark),
            R::InverseSurface => (R::Surface, !dark),
            R::InverseOnSurface => (R::OnSurface, !dark),
            R::SurfaceTint => (R::Primary, dark),
            _ => (role, dark),
        };
        let mut result = String::from("system_");
        for (index, ch) in role.name().chars().enumerate() {
            if index != 0 && ch.is_ascii_uppercase() {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
        }
        if !role.name().contains("Fixed") {
            result.push_str(if dark { "_dark" } else { "_light" });
        }
        Some(result)
    }

    pub(super) fn palette_resource(family: MaterialPaletteFamily, tone: u8) -> String {
        let family = match family {
            MaterialPaletteFamily::Primary => "accent1",
            MaterialPaletteFamily::Secondary => "accent2",
            MaterialPaletteFamily::Tertiary => "accent3",
            MaterialPaletteFamily::Neutral => "neutral1",
            MaterialPaletteFamily::NeutralVariant => "neutral2",
        };
        format!("system_{family}_{}", (100 - u16::from(tone)) * 10)
    }
}

#[cfg(all(target_os = "android", feature = "android-dynamic-color"))]
mod native {
    use super::{mapping, MaterialAndroidColorError, MaterialTheme};
    use ferroui_android::interop::java::{call_int, call_object, JavaRef, JavaValue};
    use ferroui_android::FerroAndroidApplication;
    use ferroui_base::media::Color;
    use ferroui_base::platform::IPlatformSettings;
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::{FerroLocator, LocatorExtensions};
    use std::cell::Cell;
    use std::rc::Rc;

    pub(super) fn read() -> Result<mapping::Schemes, MaterialAndroidColorError> {
        let context = FerroAndroidApplication::try_context()
            .ok_or(MaterialAndroidColorError::ApplicationNotInitialized)?;
        let resources = call_object(
            &context,
            "getResources",
            "()Landroid/content/res/Resources;",
            &[],
        )
        .ok_or(MaterialAndroidColorError::ResourcesUnavailable)?;
        mapping::read(|name| color(&resources, name))
    }

    fn color(resources: &dyn JavaRef, name: &str) -> Option<Color> {
        let identifier = call_int(
            resources,
            "getIdentifier",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                JavaValue::String(name),
                JavaValue::String("color"),
                JavaValue::String("android"),
            ],
        );
        if identifier == 0 {
            return None;
        }
        Some(Color::from_uint32(call_int(
            resources,
            "getColor",
            "(ILandroid/content/res/Resources$Theme;)I",
            &[JavaValue::Int(identifier), JavaValue::Object(None)],
        ) as u32))
    }

    pub(super) fn subscribe(theme: &MaterialTheme) {
        theme.stop_android_dynamic_colors();
        let Some(settings) = FerroLocator::current().get_service::<dyn IPlatformSettings>() else {
            return;
        };
        let weak = theme.to_ref().downgrade();
        let active = Rc::new(Cell::new(true));
        let handler_active = active.clone();
        let subscription = settings.color_values_changed(Rc::new(move |_| {
            if handler_active.get() {
                if let Some(theme) = weak.upgrade() {
                    // Validation occurs before installation: retain the last complete palette
                    // if the host is temporarily between resource configurations.
                    let _ = theme.refresh_android_dynamic_colors();
                }
            }
        }));
        *theme.android_color_subscription.borrow_mut() = Some(Rc::new(Subscription {
            subscription,
            active,
        }));
    }

    struct Subscription {
        subscription: Rc<dyn IDisposable>,
        active: Rc<Cell<bool>>,
    }

    impl IDisposable for Subscription {
        fn dispose(&self) {
            if self.active.replace(false) {
                self.subscription.dispose();
            }
        }
    }

    impl Drop for Subscription {
        fn drop(&mut self) {
            self.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MaterialColorRole as R, MaterialColorScheme, MaterialPaletteFamily as F};
    use ferroui_base::media::Color;

    #[test]
    fn native_resource_names_match_androidx_inverse_fixed_and_surface_mappings() {
        assert_eq!(
            mapping::role_resource(R::Primary, false).as_deref(),
            Some("system_primary_light")
        );
        assert_eq!(
            mapping::role_resource(R::InversePrimary, false).as_deref(),
            Some("system_primary_dark")
        );
        assert_eq!(
            mapping::role_resource(R::InverseOnSurface, true).as_deref(),
            Some("system_on_surface_light")
        );
        assert_eq!(
            mapping::role_resource(R::SurfaceTint, true).as_deref(),
            Some("system_primary_dark")
        );
        assert_eq!(
            mapping::role_resource(R::SurfaceContainerHighest, false).as_deref(),
            Some("system_surface_container_highest_light")
        );
        assert_eq!(
            mapping::role_resource(R::OnTertiaryFixedVariant, true).as_deref(),
            Some("system_on_tertiary_fixed_variant")
        );
        assert_eq!(mapping::role_resource(R::Error, false), None);
        assert_eq!(
            mapping::palette_resource(F::Primary, 99),
            "system_accent1_10"
        );
        assert_eq!(
            mapping::palette_resource(F::NeutralVariant, 40),
            "system_neutral2_600"
        );
        assert_eq!(
            mapping::palette_resource(F::Tertiary, 95),
            "system_accent3_50"
        );
    }

    #[test]
    fn android34_import_is_lossless_and_keeps_androidx_error_defaults() {
        let device = Color::from_argb(231, 51, 102, 153);
        let schemes = mapping::read(|_| Some(device)).unwrap();
        assert_eq!(
            schemes.source,
            MaterialAndroidColorSource::Android34ColorRoles
        );
        for scheme in [&schemes.light, &schemes.dark] {
            assert_eq!(scheme.color(R::Primary), device);
            assert_eq!(scheme.color(R::SurfaceContainerHighest), device);
            assert_eq!(scheme.color(R::OnTertiaryFixedVariant), device);
        }
        assert_eq!(
            schemes.light.color(R::Error),
            MaterialColorScheme::light().color(R::Error)
        );
        assert_eq!(
            schemes.dark.color(R::Scrim),
            MaterialColorScheme::dark().color(R::Scrim)
        );
    }

    #[test]
    fn incomplete_android34_resources_are_not_replaced_by_the_older_algorithm() {
        let result = mapping::read(|name| {
            if name == "system_surface_container_high_dark" {
                None
            } else {
                Some(Color::from_rgb(10, 20, 30))
            }
        });
        assert!(matches!(
            result,
            Err(MaterialAndroidColorError::IncompletePalette(
                MaterialDynamicColorError::MissingSystemRole(R::SurfaceContainerHigh)
            ))
        ));
    }

    #[test]
    fn android31_reads_only_public_system_tones_and_derives_added_neutrals() {
        let device = Color::from_uint32(0xff605d66);
        let schemes = mapping::read(|name| {
            if ![
                "system_accent1_",
                "system_accent2_",
                "system_accent3_",
                "system_neutral1_",
                "system_neutral2_",
            ]
            .iter()
            .any(|prefix| name.starts_with(prefix))
            {
                return None;
            }
            let suffix = name.rsplit('_').next().unwrap().parse::<u16>().unwrap();
            assert!(
                [0, 10, 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000].contains(&suffix)
            );
            Some(device)
        })
        .unwrap();
        assert_eq!(
            schemes.source,
            MaterialAndroidColorSource::Android31TonalPalette
        );
        assert_eq!(schemes.light.color(R::Primary), device);
        assert_eq!(schemes.dark.color(R::Primary), device);
        assert_ne!(schemes.light.color(R::Surface), device);
        assert_ne!(schemes.dark.color(R::Surface), device);
        assert_eq!(
            schemes.light.color(R::Surface),
            palette_math::set_luminance(device, 98)
        );
    }

    #[test]
    fn unavailable_dynamic_resources_report_a_failure() {
        assert!(matches!(
            mapping::read(|_| None),
            Err(MaterialAndroidColorError::DynamicColorsUnavailable)
        ));
    }
}
