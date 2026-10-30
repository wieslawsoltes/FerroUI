// Copyright 2021 The Android Open Source Project (Android tonal role mapping).
// Copyright 2026 FerroUI contributors (Rust adapters).
// SPDX-License-Identifier: Apache-2.0
//! HCT seed schemes and lossless imports of Android dynamic color palettes.
//!
//! Seed schemes follow versioned Google Material Color Utilities phone specs at
//! bd811e4ca2ab94c4ce3cadc89c19b1820b9704b3, using `material-colors` 0.4.2 for HCT.
//! Android imports follow AndroidX Material3 `DynamicTonalPalette.android.kt` at
//! e4d933c98df7baa7ea2b83eea6e086915961e0ff. See `docs/porting/material-dynamic-color.md`
//! for the distinction between seed algorithm revisions and system color imports.

use crate::{MaterialColorRole, MaterialColorScheme};
use ferroui_base::media::Color;
use material_colors::color::Argb;
use material_colors::dynamic_color::Variant;
use material_colors::palette::TonalPalette;
use std::fmt;

#[path = "dynamic_color_spec.rs"]
mod spec;

/// Exact upstream source used for palette, tone-delta, and role-contrast rules.
pub const MATERIAL_COLOR_UTILITIES_VERSION: &str =
    "Google Material Color Utilities bd811e4ca2ab94c4ce3cadc89c19b1820b9704b3 (phone)";

/// The separately pinned implementation of the CAM16/HCT gamut solver.
pub const MATERIAL_HCT_SOLVER_VERSION: &str = "material-colors 0.4.2";

/// Source-spec selection for locally generated phone schemes. The default is
/// exactly Google's `DynamicScheme.DEFAULT_SPEC_VERSION` at the pinned commit.
/// This never changes imported Android colors or the AndroidX baseline schemes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MaterialColorSpec {
    #[default]
    Spec2021,
    Spec2025,
    Spec2026,
}

impl MaterialColorSpec {
    /// Apply Google's supported-variant fallback. The 2026 spec introduces a
    /// separate CMF multi-source variant, which this single-seed API does not
    /// expose. For these nine phone styles its results are identical to 2025.
    pub const fn effective_for(self, style: MaterialPaletteStyle) -> Self {
        match (self, style) {
            (Self::Spec2021, _) => Self::Spec2021,
            (
                _,
                MaterialPaletteStyle::Neutral
                | MaterialPaletteStyle::TonalSpot
                | MaterialPaletteStyle::Vibrant
                | MaterialPaletteStyle::Expressive,
            ) => Self::Spec2025,
            _ => Self::Spec2021,
        }
    }
}

/// Material Color Utilities palette variants. This is independent of the UI motion scheme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MaterialPaletteStyle {
    Monochrome,
    Neutral,
    #[default]
    TonalSpot,
    Vibrant,
    Expressive,
    Fidelity,
    Content,
    Rainbow,
    FruitSalad,
}

impl MaterialPaletteStyle {
    pub const ALL: [Self; 9] = [
        Self::Monochrome,
        Self::Neutral,
        Self::TonalSpot,
        Self::Vibrant,
        Self::Expressive,
        Self::Fidelity,
        Self::Content,
        Self::Rainbow,
        Self::FruitSalad,
    ];

    const fn variant(self) -> Variant {
        match self {
            Self::Monochrome => Variant::Monochrome,
            Self::Neutral => Variant::Neutral,
            Self::TonalSpot => Variant::TonalSpot,
            Self::Vibrant => Variant::Vibrant,
            Self::Expressive => Variant::Expressive,
            Self::Fidelity => Variant::Fidelity,
            Self::Content => Variant::Content,
            Self::Rainbow => Variant::Rainbow,
            Self::FruitSalad => Variant::FruitSalad,
        }
    }
}

/// Inputs for a generated dynamic scheme. Contrast is in [-1, 1]: zero is standard,
/// 0.5 is medium, and 1 is high contrast. NaN/infinite/out-of-range values are rejected.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MaterialDynamicColorOptions {
    pub dark: bool,
    pub contrast_level: f64,
    pub style: MaterialPaletteStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaterialPaletteFamily {
    Primary,
    Secondary,
    Tertiary,
    Neutral,
    NeutralVariant,
}

/// A failed import identifies the exact missing tone or role; it never substitutes
/// an unrelated color or interpolates missing platform tones in RGB.
#[derive(Clone, Debug, PartialEq)]
pub enum MaterialDynamicColorError {
    InvalidContrast(f64),
    InvalidHueOrChroma {
        hue: f64,
        chroma: f64,
    },
    InvalidTone(u8),
    DuplicateTone(u8),
    MissingTone {
        palette: MaterialPaletteFamily,
        tone: u8,
    },
    MissingSystemRole(MaterialColorRole),
}

impl fmt::Display for MaterialDynamicColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContrast(value) => write!(f, "Material contrast must be finite and in [-1, 1], got {value}"),
            Self::InvalidHueOrChroma { hue, chroma } => write!(f, "Material hue must be finite and chroma finite/nonnegative, got hue {hue}, chroma {chroma}"),
            Self::InvalidTone(tone) => write!(f, "Material tone must be in [0, 100], got {tone}"),
            Self::DuplicateTone(tone) => write!(f, "Material tone {tone} was supplied more than once"),
            Self::MissingTone { palette, tone } => write!(f, "Android {palette:?} palette is missing tone {tone}"),
            Self::MissingSystemRole(role) => write!(f, "Android dynamic scheme is missing {}", role.name()),
        }
    }
}

impl std::error::Error for MaterialDynamicColorError {}

/// An imported or HCT-generated tonal palette. Imported ARGB values are preserved
/// exactly. Sparse palettes are supported, but scheme creation fails for missing tones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialTonalPalette {
    tones: [Option<Color>; 101],
}

impl MaterialTonalPalette {
    /// Tones exposed as the thirteen Android 12 system accent/neutral resources.
    pub const ANDROID_SYSTEM_TONES: [u8; 13] = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100];
    /// The complete neutral range read or calculated by AndroidX Material3.
    pub const ANDROID_NEUTRAL_TONES: [u8; 24] = [
        0, 4, 6, 10, 12, 17, 20, 22, 24, 30, 40, 50, 60, 70, 80, 87, 90, 92, 94, 95, 96, 98, 99,
        100,
    ];

    pub fn from_tones(
        tones: impl IntoIterator<Item = (u8, Color)>,
    ) -> Result<Self, MaterialDynamicColorError> {
        let mut result = Self { tones: [None; 101] };
        for (tone, color) in tones {
            let Some(slot) = result.tones.get_mut(usize::from(tone)) else {
                return Err(MaterialDynamicColorError::InvalidTone(tone));
            };
            if slot.replace(color).is_some() {
                return Err(MaterialDynamicColorError::DuplicateTone(tone));
            }
        }
        Ok(result)
    }

    pub fn from_colors(tones: [Color; 101]) -> Self {
        Self {
            tones: tones.map(Some),
        }
    }

    /// Generates all integer tones using the actual HCT gamut solver. Hue is
    /// normalized modulo 360; no RGB blending or fixed channel offsets are used.
    pub fn from_hct(hue: f64, chroma: f64) -> Result<Self, MaterialDynamicColorError> {
        if !hue.is_finite() || !chroma.is_finite() || chroma < 0.0 {
            return Err(MaterialDynamicColorError::InvalidHueOrChroma { hue, chroma });
        }
        Ok(Self::from_mcu(&TonalPalette::of(
            hue.rem_euclid(360.0),
            chroma,
        )))
    }

    pub fn tone(&self, tone: u8) -> Option<Color> {
        self.tones.get(usize::from(tone)).copied().flatten()
    }

    fn from_mcu(palette: &TonalPalette) -> Self {
        Self::from_colors(std::array::from_fn(|tone| {
            from_argb(spec::palette_argb(palette, tone as f64, 1.0))
        }))
    }
}

/// All five Android tonal palettes, including the full neutral and neutral-variant
/// ranges. The Android 31–33 mapping deliberately uses neutral-variant surfaces,
/// as AndroidX does, rather than the older system's lower-chroma neutral palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialDynamicPalette {
    pub primary: MaterialTonalPalette,
    pub secondary: MaterialTonalPalette,
    pub tertiary: MaterialTonalPalette,
    pub neutral: MaterialTonalPalette,
    pub neutral_variant: MaterialTonalPalette,
}

impl MaterialDynamicPalette {
    /// Imports the complete AndroidX tonal palette. A platform bridge supplies
    /// the system tones and the additional neutral tones calculated by AndroidX
    /// `Color.setLuminance`; values are never regenerated on this path.
    pub fn from_android31_colors(
        mut resolve: impl FnMut(MaterialPaletteFamily, u8) -> Option<Color>,
    ) -> Result<Self, MaterialDynamicColorError> {
        fn import(
            family: MaterialPaletteFamily,
            tones: &[u8],
            resolve: &mut impl FnMut(MaterialPaletteFamily, u8) -> Option<Color>,
        ) -> Result<MaterialTonalPalette, MaterialDynamicColorError> {
            let mut result = MaterialTonalPalette { tones: [None; 101] };
            for &tone in tones {
                result.tones[usize::from(tone)] = Some(resolve(family, tone).ok_or(
                    MaterialDynamicColorError::MissingTone {
                        palette: family,
                        tone,
                    },
                )?);
            }
            Ok(result)
        }
        use MaterialPaletteFamily as F;
        Ok(Self {
            primary: import(
                F::Primary,
                &MaterialTonalPalette::ANDROID_SYSTEM_TONES,
                &mut resolve,
            )?,
            secondary: import(
                F::Secondary,
                &MaterialTonalPalette::ANDROID_SYSTEM_TONES,
                &mut resolve,
            )?,
            tertiary: import(
                F::Tertiary,
                &MaterialTonalPalette::ANDROID_SYSTEM_TONES,
                &mut resolve,
            )?,
            neutral: import(
                F::Neutral,
                &MaterialTonalPalette::ANDROID_NEUTRAL_TONES,
                &mut resolve,
            )?,
            neutral_variant: import(
                F::NeutralVariant,
                &MaterialTonalPalette::ANDROID_NEUTRAL_TONES,
                &mut resolve,
            )?,
        })
    }

    /// Generates five full HCT palettes locally. To reproduce a device wallpaper
    /// scheme exactly, import its system palettes or direct color roles instead.
    pub fn from_seed(seed: Color, style: MaterialPaletteStyle) -> Self {
        Self::from_seed_with_spec(seed, style, false, MaterialColorSpec::default())
    }

    /// Generates all five tone palettes for a source specification. In the 2025
    /// phone spec some palette chromas depend on dark mode, so it is explicit.
    pub fn from_seed_with_spec(
        seed: Color,
        style: MaterialPaletteStyle,
        dark: bool,
        source_spec: MaterialColorSpec,
    ) -> Self {
        let scheme = spec::palettes(
            to_seed_argb(seed),
            style.variant(),
            dark,
            0.0,
            source_spec.effective_for(style),
        );
        Self {
            primary: MaterialTonalPalette::from_mcu(&scheme.primary_palette),
            secondary: MaterialTonalPalette::from_mcu(&scheme.secondary_palette),
            tertiary: MaterialTonalPalette::from_mcu(&scheme.tertiary_palette),
            neutral: MaterialTonalPalette::from_mcu(&scheme.neutral_palette),
            neutral_variant: MaterialTonalPalette::from_mcu(&scheme.neutral_variant_palette),
        }
    }

    pub fn palette(&self, family: MaterialPaletteFamily) -> &MaterialTonalPalette {
        match family {
            MaterialPaletteFamily::Primary => &self.primary,
            MaterialPaletteFamily::Secondary => &self.secondary,
            MaterialPaletteFamily::Tertiary => &self.tertiary,
            MaterialPaletteFamily::Neutral => &self.neutral,
            MaterialPaletteFamily::NeutralVariant => &self.neutral_variant,
        }
    }

    /// Exact role selection from `dynamicLightColorScheme31` /
    /// `dynamicDarkColorScheme31` in the pinned AndroidX source. Fixed roles do
    /// not change with light/dark mode. Error roles retain the AndroidX defaults.
    pub fn scheme_android31(
        &self,
        dark: bool,
    ) -> Result<MaterialColorScheme, MaterialDynamicColorError> {
        let mut scheme = if dark {
            MaterialColorScheme::dark()
        } else {
            MaterialColorScheme::light()
        };
        for role in MaterialColorRole::ALL {
            if let Some((palette, tone)) = android31_tone(role, dark) {
                let color = self
                    .palette(palette)
                    .tone(tone)
                    .ok_or(MaterialDynamicColorError::MissingTone { palette, tone })?;
                scheme.set_color(role, color);
            }
        }
        Ok(scheme)
    }
}

impl MaterialColorScheme {
    /// Standard-contrast Tonal Spot scheme, generated with CAM16/HCT. Seed alpha
    /// is ignored, matching the opaque source-color contract of Material utilities.
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        Self::from_seed_with_options(
            seed,
            MaterialDynamicColorOptions {
                dark,
                ..Default::default()
            },
        )
        .expect("zero is a valid Material contrast level")
    }

    /// Generates all 48 roles, including fixed and surface-container roles, with
    /// the Tonal Spot palette and an explicit contrast level in [-1, 1].
    pub fn from_seed_color(
        seed: Color,
        dark: bool,
        contrast_level: f64,
    ) -> Result<Self, MaterialDynamicColorError> {
        Self::from_seed_with_options(
            seed,
            MaterialDynamicColorOptions {
                dark,
                contrast_level,
                ..Default::default()
            },
        )
    }

    pub fn from_seed_with_options(
        seed: Color,
        options: MaterialDynamicColorOptions,
    ) -> Result<Self, MaterialDynamicColorError> {
        Self::from_seed_with_spec(seed, options, MaterialColorSpec::default())
    }

    /// Generates all 48 roles using an explicit source specification and its
    /// official phone-variant fallback. `Spec2025` enables the current expressive
    /// palettes, chroma-sensitive surfaces, and independent tone constraints.
    pub fn from_seed_with_spec(
        seed: Color,
        options: MaterialDynamicColorOptions,
        source_spec: MaterialColorSpec,
    ) -> Result<Self, MaterialDynamicColorError> {
        if !options.contrast_level.is_finite() || !(-1.0..=1.0).contains(&options.contrast_level) {
            return Err(MaterialDynamicColorError::InvalidContrast(
                options.contrast_level,
            ));
        }
        let source_spec = source_spec.effective_for(options.style);
        let scheme = spec::palettes(
            to_seed_argb(seed),
            options.style.variant(),
            options.dark,
            options.contrast_level,
            source_spec,
        );
        let resolver = spec::RoleResolver::new(&scheme, source_spec);
        Ok(Self::new(
            MaterialColorRole::ALL.map(|role| from_argb(resolver.argb(role))),
        ))
    }

    /// Losslessly imports Android 14/API 34+ roles, including system high-contrast
    /// adjustments. The host resolves the corresponding `mc3_light_scheme` or
    /// `mc3_dark_scheme` attributes. As in AndroidX, error roles and scrim retain
    /// their baseline defaults when absent; supplied overrides are preserved.
    /// Every other role is required. This method does not query Android or JNI.
    pub fn from_android_system_roles(
        dark: bool,
        mut resolve: impl FnMut(MaterialColorRole) -> Option<Color>,
    ) -> Result<Self, MaterialDynamicColorError> {
        let mut scheme = if dark { Self::dark() } else { Self::light() };
        for role in MaterialColorRole::ALL {
            if let Some(color) = resolve(role) {
                scheme.set_color(role, color);
            } else if !matches!(
                role,
                MaterialColorRole::Error
                    | MaterialColorRole::OnError
                    | MaterialColorRole::ErrorContainer
                    | MaterialColorRole::OnErrorContainer
                    | MaterialColorRole::Scrim
            ) {
                return Err(MaterialDynamicColorError::MissingSystemRole(role));
            }
        }
        Ok(scheme)
    }
}

fn to_seed_argb(color: Color) -> Argb {
    Argb::new(255, color.r, color.g, color.b)
}
fn from_argb(color: Argb) -> Color {
    Color::from_argb(color.alpha, color.red, color.green, color.blue)
}

fn android31_tone(role: MaterialColorRole, dark: bool) -> Option<(MaterialPaletteFamily, u8)> {
    use MaterialColorRole as R;
    use MaterialPaletteFamily as F;
    let accent = if dark { 80 } else { 40 };
    let on_accent = if dark { 20 } else { 100 };
    let container = if dark { 30 } else { 90 };
    let on_container = if dark { 90 } else { 10 };
    Some(match role {
        R::Primary | R::SurfaceTint => (F::Primary, accent),
        R::OnPrimary => (F::Primary, on_accent),
        R::PrimaryContainer => (F::Primary, container),
        R::OnPrimaryContainer => (F::Primary, on_container),
        R::InversePrimary => (F::Primary, if dark { 40 } else { 80 }),
        R::Secondary => (F::Secondary, accent),
        R::OnSecondary => (F::Secondary, on_accent),
        R::SecondaryContainer => (F::Secondary, container),
        R::OnSecondaryContainer => (F::Secondary, on_container),
        R::Tertiary => (F::Tertiary, accent),
        R::OnTertiary => (F::Tertiary, on_accent),
        R::TertiaryContainer => (F::Tertiary, container),
        R::OnTertiaryContainer => (F::Tertiary, on_container),
        R::PrimaryFixed => (F::Primary, 90),
        R::PrimaryFixedDim => (F::Primary, 80),
        R::OnPrimaryFixed => (F::Primary, 10),
        R::OnPrimaryFixedVariant => (F::Primary, 30),
        R::SecondaryFixed => (F::Secondary, 90),
        R::SecondaryFixedDim => (F::Secondary, 80),
        R::OnSecondaryFixed => (F::Secondary, 10),
        R::OnSecondaryFixedVariant => (F::Secondary, 30),
        R::TertiaryFixed => (F::Tertiary, 90),
        R::TertiaryFixedDim => (F::Tertiary, 80),
        R::OnTertiaryFixed => (F::Tertiary, 10),
        R::OnTertiaryFixedVariant => (F::Tertiary, 30),
        R::Background | R::Surface => (F::NeutralVariant, if dark { 6 } else { 98 }),
        R::OnBackground | R::OnSurface => (F::NeutralVariant, if dark { 90 } else { 10 }),
        R::SurfaceVariant => (F::NeutralVariant, if dark { 30 } else { 90 }),
        R::OnSurfaceVariant => (F::NeutralVariant, if dark { 80 } else { 30 }),
        R::InverseSurface => (F::NeutralVariant, if dark { 90 } else { 20 }),
        R::InverseOnSurface => (F::NeutralVariant, if dark { 20 } else { 95 }),
        R::Outline => (F::NeutralVariant, if dark { 60 } else { 50 }),
        R::OutlineVariant => (F::NeutralVariant, if dark { 30 } else { 80 }),
        R::Scrim => (F::NeutralVariant, 0),
        R::SurfaceBright => (F::NeutralVariant, if dark { 24 } else { 98 }),
        R::SurfaceDim => (F::NeutralVariant, if dark { 6 } else { 87 }),
        R::SurfaceContainer => (F::NeutralVariant, if dark { 12 } else { 94 }),
        R::SurfaceContainerHigh => (F::NeutralVariant, if dark { 17 } else { 92 }),
        R::SurfaceContainerHighest => (F::NeutralVariant, if dark { 22 } else { 90 }),
        R::SurfaceContainerLow => (F::NeutralVariant, if dark { 10 } else { 96 }),
        R::SurfaceContainerLowest => (F::NeutralVariant, if dark { 4 } else { 100 }),
        R::Error | R::OnError | R::ErrorContainer | R::OnErrorContainer => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_reference_spec(specification: MaterialColorSpec, fixture_name: &str) {
        let csv = include_str!("tests/fixtures/dynamic-color-phone.csv");
        let mut lines = csv.lines();
        let header: Vec<_> = lines.next().unwrap().split(',').collect();
        for (column, role) in header[5..].iter().zip(MaterialColorRole::ALL) {
            assert_eq!(*column, role.name(), "fixture role order");
        }
        let mut schemes = 0;
        for line in lines {
            let row: Vec<_> = line.split(',').collect();
            if row[0] != fixture_name {
                continue;
            }
            let style = match row[2] {
                "MONOCHROME" => MaterialPaletteStyle::Monochrome,
                "NEUTRAL" => MaterialPaletteStyle::Neutral,
                "TONAL_SPOT" => MaterialPaletteStyle::TonalSpot,
                "VIBRANT" => MaterialPaletteStyle::Vibrant,
                "EXPRESSIVE" => MaterialPaletteStyle::Expressive,
                "FIDELITY" => MaterialPaletteStyle::Fidelity,
                "CONTENT" => MaterialPaletteStyle::Content,
                "RAINBOW" => MaterialPaletteStyle::Rainbow,
                "FRUIT_SALAD" => MaterialPaletteStyle::FruitSalad,
                other => panic!("unknown reference style {other}"),
            };
            let seed = Color::from_uint32(u32::from_str_radix(row[1], 16).unwrap());
            let options = MaterialDynamicColorOptions {
                style,
                dark: row[3] == "1",
                contrast_level: row[4].parse().unwrap(),
            };
            let actual =
                MaterialColorScheme::from_seed_with_spec(seed, options, specification).unwrap();
            assert_eq!(row.len(), 53);
            for (role, expected) in MaterialColorRole::ALL.into_iter().zip(&row[5..]) {
                let expected = u32::from_str_radix(expected, 16).unwrap();
                assert_eq!(
                    actual.color(role).to_uint32(),
                    expected,
                    "Google MCU {specification:?} seed={} {options:?} {role:?}",
                    row[1]
                );
            }
            schemes += 1;
        }
        assert_eq!(schemes, 1728, "16 seeds × 9 styles × 2 modes × 6 contrasts");
    }

    #[test]
    fn current_2021_spec_matches_all_82944_independent_reference_argbs() {
        assert_reference_spec(MaterialColorSpec::Spec2021, "2021");
    }

    #[test]
    fn expressive_2025_spec_matches_all_82944_independent_reference_argbs() {
        assert_reference_spec(MaterialColorSpec::Spec2025, "2025");
    }

    #[test]
    fn source_default_and_2026_phone_fallback_match_the_upstream_contract() {
        assert_eq!(MaterialColorSpec::default(), MaterialColorSpec::Spec2021);
        let seed = Color::from_uint32(0xff4285f4);
        for style in MaterialPaletteStyle::ALL {
            let effective = if matches!(
                style,
                MaterialPaletteStyle::Neutral
                    | MaterialPaletteStyle::TonalSpot
                    | MaterialPaletteStyle::Vibrant
                    | MaterialPaletteStyle::Expressive
            ) {
                MaterialColorSpec::Spec2025
            } else {
                MaterialColorSpec::Spec2021
            };
            assert_eq!(MaterialColorSpec::Spec2025.effective_for(style), effective);
            assert_eq!(MaterialColorSpec::Spec2026.effective_for(style), effective);
            for dark in [false, true] {
                let options = MaterialDynamicColorOptions {
                    style,
                    dark,
                    contrast_level: 0.25,
                };
                assert_eq!(
                    MaterialColorScheme::from_seed_with_options(seed, options).unwrap(),
                    MaterialColorScheme::from_seed_with_spec(
                        seed,
                        options,
                        MaterialColorSpec::Spec2021
                    )
                    .unwrap()
                );
                assert_eq!(
                    MaterialColorScheme::from_seed_with_spec(
                        seed,
                        options,
                        MaterialColorSpec::Spec2026
                    )
                    .unwrap(),
                    MaterialColorScheme::from_seed_with_spec(
                        seed,
                        options,
                        MaterialColorSpec::Spec2025
                    )
                    .unwrap()
                );
            }
        }
    }

    #[test]
    fn black_fidelity_keeps_the_reference_gray_container_foreground() {
        for style in [
            MaterialPaletteStyle::Fidelity,
            MaterialPaletteStyle::Content,
        ] {
            let options = MaterialDynamicColorOptions {
                style,
                dark: false,
                contrast_level: 0.0,
            };
            let scheme = MaterialColorScheme::from_seed_with_options(
                Color::from_uint32(0xff000000),
                options,
            )
            .unwrap();
            // The dependency's black HCT tone is -2.22e-16 due to fused multiply-add.
            // Passing that to contrast's range validation incorrectly selects white.
            assert_eq!(
                scheme
                    .color(MaterialColorRole::OnPrimaryContainer)
                    .to_uint32(),
                0xff848484
            );
            assert_eq!(
                scheme
                    .color(MaterialColorRole::OnTertiaryContainer)
                    .to_uint32(),
                0xff848484
            );
        }
    }

    // Independently generated by the pinned Google TypeScript implementation.
    // This small readable golden supplements the complete multi-variant fixture.
    #[test]
    fn seed_schemes_match_official_default_contrast_goldens() {
        let expected: [u32; 48] = [
            0xfffdf7ff, 0xffba1a1a, 0xffffdad6, 0xfff5eff7, 0xffcfbdfe, 0xff322f35, 0xff1d1b20,
            0xffffffff, 0xff93000a, 0xffffffff, 0xff4d3d75, 0xff201047, 0xff4d3d75, 0xffffffff,
            0xff4a4458, 0xff1e192b, 0xff4a4458, 0xff1d1b20, 0xff49454e, 0xffffffff, 0xff633b48,
            0xff31101d, 0xff633b48, 0xff7a757f, 0xffcac4cf, 0xff65558f, 0xffe9ddff, 0xffe9ddff,
            0xffcfbdfe, 0xff000000, 0xff625b71, 0xffe8def8, 0xffe8def8, 0xffcbc2db, 0xfffdf7ff,
            0xfffdf7ff, 0xfff2ecf4, 0xffece6ee, 0xffe6e0e9, 0xfff8f2fa, 0xffffffff, 0xffded8e0,
            0xffe7e0eb, 0xff7e5260, 0xffffd9e3, 0xffffd9e3, 0xffefb8c8, 0xff65558f,
        ];
        let actual = MaterialColorScheme::from_seed(Color::from_uint32(0xff6750a4), false);
        for (role, expected) in MaterialColorRole::ALL.into_iter().zip(expected) {
            assert_eq!(actual.color(role).to_uint32(), expected, "{}", role.name());
        }
        let expected: [u32; 48] = [
            0xff141218, 0xffffb4ab, 0xff93000a, 0xff322f35, 0xff65558f, 0xffe6e0e9, 0xffe6e0e9,
            0xff690005, 0xffffdad6, 0xff36275d, 0xffe9ddff, 0xff201047, 0xff4d3d75, 0xff332d41,
            0xffe8def8, 0xff1e192b, 0xff4a4458, 0xffe6e0e9, 0xffcac4cf, 0xff4a2532, 0xffffd9e3,
            0xff31101d, 0xff633b48, 0xff948f99, 0xff49454e, 0xffcfbdfe, 0xff4d3d75, 0xffe9ddff,
            0xffcfbdfe, 0xff000000, 0xffcbc2db, 0xff4a4458, 0xffe8def8, 0xffcbc2db, 0xff141218,
            0xff3b383e, 0xff211f24, 0xff2b292f, 0xff36343a, 0xff1d1b20, 0xff0f0d13, 0xff141218,
            0xff49454e, 0xffefb8c8, 0xff633b48, 0xffffd9e3, 0xffefb8c8, 0xffcfbdfe,
        ];
        let actual = MaterialColorScheme::from_seed(Color::from_uint32(0xff6750a4), true);
        for (role, expected) in MaterialColorRole::ALL.into_iter().zip(expected) {
            assert_eq!(actual.color(role).to_uint32(), expected, "{}", role.name());
        }
    }

    fn luminance(color: Color) -> f64 {
        let channel = |value: u8| {
            let value = f64::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }

    fn contrast(a: Color, b: Color) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn high_contrast_schemes_meet_primary_foreground_pair_contrast() {
        use MaterialColorRole as R;
        let pairs = [
            (R::OnPrimary, R::Primary),
            (R::OnPrimaryContainer, R::PrimaryContainer),
            (R::OnSecondary, R::Secondary),
            (R::OnTertiary, R::Tertiary),
            (R::OnError, R::Error),
            (R::OnSurface, R::Surface),
        ];
        for seed in [
            0xff6750a4, 0xffff0000, 0xff00ff00, 0xff0000ff, 0xff000000, 0xffffffff,
        ] {
            for dark in [false, true] {
                let scheme =
                    MaterialColorScheme::from_seed_color(Color::from_uint32(seed), dark, 1.0)
                        .unwrap();
                for (foreground, background) in pairs {
                    let ratio = contrast(scheme.color(foreground), scheme.color(background));
                    assert!(
                        ratio >= 7.0,
                        "{seed:#x} dark={dark} {foreground:?}/{background:?}: {ratio}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_palette_style_produces_complete_opaque_schemes() {
        for style in MaterialPaletteStyle::ALL {
            for dark in [false, true] {
                let scheme = MaterialColorScheme::from_seed_with_options(
                    Color::from_uint32(0x806750a4),
                    MaterialDynamicColorOptions {
                        style,
                        dark,
                        contrast_level: 0.0,
                    },
                )
                .unwrap();
                assert!(MaterialColorRole::ALL
                    .into_iter()
                    .all(|role| scheme.color(role).a == 255));
                assert_ne!(
                    scheme.color(MaterialColorRole::Primary),
                    scheme.color(MaterialColorRole::OnPrimary)
                );
            }
        }
        assert_eq!(
            MaterialColorScheme::from_seed(Color::from_uint32(0x006750a4), false),
            MaterialColorScheme::from_seed(Color::from_uint32(0xff6750a4), false)
        );
    }

    #[test]
    fn invalid_contrast_and_tonal_inputs_fail_explicitly() {
        for contrast in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.01, 1.01] {
            assert!(matches!(
                MaterialColorScheme::from_seed_color(
                    Color::from_uint32(0xff6750a4),
                    false,
                    contrast
                ),
                Err(MaterialDynamicColorError::InvalidContrast(_))
            ));
        }
        let color = Color::from_uint32(0xff6750a4);
        assert_eq!(
            MaterialTonalPalette::from_tones([(101, color)]),
            Err(MaterialDynamicColorError::InvalidTone(101))
        );
        assert_eq!(
            MaterialTonalPalette::from_tones([(40, color), (40, color)]),
            Err(MaterialDynamicColorError::DuplicateTone(40))
        );
        assert!(MaterialTonalPalette::from_hct(f64::NAN, 20.0).is_err());
        assert!(MaterialTonalPalette::from_hct(100.0, -1.0).is_err());
    }

    fn imported_color(family: MaterialPaletteFamily, tone: u8) -> Color {
        let family = match family {
            MaterialPaletteFamily::Primary => 1,
            MaterialPaletteFamily::Secondary => 2,
            MaterialPaletteFamily::Tertiary => 3,
            MaterialPaletteFamily::Neutral => 4,
            MaterialPaletteFamily::NeutralVariant => 5,
        };
        // Deliberately retain nonopaque alpha to prove imports are lossless.
        Color::from_argb(231, family, tone, 200)
    }

    #[test]
    fn android31_import_uses_neutral_variant_and_preserves_fixed_colors() {
        use MaterialColorRole as R;
        use MaterialPaletteFamily as F;
        let palette = MaterialDynamicPalette::from_android31_colors(|family, tone| {
            Some(imported_color(family, tone))
        })
        .unwrap();
        let light = palette.scheme_android31(false).unwrap();
        let dark = palette.scheme_android31(true).unwrap();
        assert_eq!(
            light.color(R::Surface),
            imported_color(F::NeutralVariant, 98)
        );
        assert_eq!(dark.color(R::Surface), imported_color(F::NeutralVariant, 6));
        assert_eq!(
            light.color(R::SurfaceContainerHighest),
            imported_color(F::NeutralVariant, 90)
        );
        assert_eq!(
            dark.color(R::SurfaceContainerLowest),
            imported_color(F::NeutralVariant, 4)
        );
        assert_eq!(light.color(R::Primary), imported_color(F::Primary, 40));
        assert_eq!(dark.color(R::Primary), imported_color(F::Primary, 80));
        assert_eq!(
            light.color(R::OnSecondaryContainer),
            imported_color(F::Secondary, 10)
        );
        assert_eq!(
            dark.color(R::OnSecondaryContainer),
            imported_color(F::Secondary, 90)
        );
        for role in [
            R::PrimaryFixed,
            R::PrimaryFixedDim,
            R::OnPrimaryFixed,
            R::OnPrimaryFixedVariant,
            R::SecondaryFixed,
            R::SecondaryFixedDim,
            R::OnSecondaryFixed,
            R::OnSecondaryFixedVariant,
            R::TertiaryFixed,
            R::TertiaryFixedDim,
            R::OnTertiaryFixed,
            R::OnTertiaryFixedVariant,
        ] {
            assert_eq!(light.color(role), dark.color(role), "{role:?}");
        }
        assert_eq!(
            light.color(R::Error),
            MaterialColorScheme::light().color(R::Error)
        );
        assert_eq!(
            dark.color(R::Error),
            MaterialColorScheme::dark().color(R::Error)
        );
    }

    #[test]
    fn missing_android_tones_are_not_interpolated() {
        let missing = MaterialDynamicPalette::from_android31_colors(|family, tone| {
            if family == MaterialPaletteFamily::NeutralVariant && tone == 94 {
                None
            } else {
                Some(imported_color(family, tone))
            }
        });
        assert_eq!(
            missing,
            Err(MaterialDynamicColorError::MissingTone {
                palette: MaterialPaletteFamily::NeutralVariant,
                tone: 94,
            })
        );
        let common = MaterialTonalPalette::from_tones(
            MaterialTonalPalette::ANDROID_SYSTEM_TONES
                .map(|tone| (tone, imported_color(MaterialPaletteFamily::Primary, tone))),
        )
        .unwrap();
        let sparse = MaterialDynamicPalette {
            primary: common.clone(),
            secondary: common.clone(),
            tertiary: common.clone(),
            neutral: common.clone(),
            neutral_variant: common,
        };
        assert!(matches!(
            sparse.scheme_android31(false),
            Err(MaterialDynamicColorError::MissingTone { tone: 98, .. })
        ));
    }

    #[test]
    fn android34_roles_keep_device_colors_and_require_complete_roles() {
        let expected =
            MaterialColorScheme::from_seed_color(Color::from_uint32(0xff0057b7), true, 1.0)
                .unwrap();
        let imported =
            MaterialColorScheme::from_android_system_roles(true, |role| Some(expected.color(role)))
                .unwrap();
        assert_eq!(imported, expected);
        assert_eq!(
            MaterialColorScheme::from_android_system_roles(true, |role| {
                if role == MaterialColorRole::SurfaceContainerHigh {
                    None
                } else {
                    Some(expected.color(role))
                }
            }),
            Err(MaterialDynamicColorError::MissingSystemRole(
                MaterialColorRole::SurfaceContainerHigh
            ))
        );
        let defaults = MaterialColorScheme::from_android_system_roles(true, |role| {
            if matches!(
                role,
                MaterialColorRole::Error
                    | MaterialColorRole::OnError
                    | MaterialColorRole::ErrorContainer
                    | MaterialColorRole::OnErrorContainer
                    | MaterialColorRole::Scrim
            ) {
                None
            } else {
                Some(expected.color(role))
            }
        })
        .unwrap();
        assert_eq!(
            defaults.color(MaterialColorRole::Error),
            MaterialColorScheme::dark().color(MaterialColorRole::Error)
        );
        assert_eq!(
            defaults.color(MaterialColorRole::Primary),
            expected.color(MaterialColorRole::Primary)
        );
    }

    #[test]
    fn hct_palette_matches_published_blue_tones() {
        let blue: material_colors::hct::Hct = Argb::from_u32(0xff0000ff).into();
        let palette = MaterialTonalPalette::from_hct(blue.get_hue(), blue.get_chroma()).unwrap();
        assert_eq!(palette.tone(0).unwrap().to_uint32(), 0xff000000);
        assert_eq!(palette.tone(40).unwrap().to_uint32(), 0xff343dff);
        assert_eq!(palette.tone(90).unwrap().to_uint32(), 0xffe0e0ff);
        assert_eq!(palette.tone(100).unwrap().to_uint32(), 0xffffffff);
        assert_eq!(palette.tone(101), None);
    }
}
