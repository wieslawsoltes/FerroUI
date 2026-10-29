// Copyright 2021, 2022, 2025 Google LLC.
// Copyright 2026 FerroUI contributors (Rust port).
// SPDX-License-Identifier: Apache-2.0
//! Phone color specifications ported from Google Material Color Utilities at
//! bd811e4ca2ab94c4ce3cadc89c19b1820b9704b3. See the independently executed
//! TypeScript fixtures and source hashes in tests/fixtures/dynamic-color-provenance.json.
//! The dependency supplies CAM16, the HCT gamut solver, and temperature analysis;
//! this module owns the versioned palette/role rules and their contrast solver.

use super::MaterialColorSpec;
use crate::MaterialColorRole as R;
use material_colors::color::Argb;
use material_colors::contrast::{darker, lighter, ratio_of_tones};
use material_colors::dislike::fix_if_disliked;
use material_colors::dynamic_color::{ContrastCurve, DynamicColor, DynamicScheme, Variant};
use material_colors::hct::Hct;
use material_colors::palette::TonalPalette;
use std::cell::RefCell;

fn curve(values: [f64; 4]) -> ContrastCurve {
    ContrastCurve {
        low: values[0],
        normal: values[1],
        medium: values[2],
        high: values[3],
    }
}

fn curve_2025(normal: f64) -> ContrastCurve {
    curve(match normal {
        1.5 => [1.5, 1.5, 3.0, 5.5],
        3.0 => [3.0, 3.0, 4.5, 7.0],
        4.5 => [4.5, 4.5, 7.0, 11.0],
        6.0 => [6.0, 6.0, 7.0, 11.0],
        7.0 => [7.0, 7.0, 11.0, 21.0],
        9.0 => [9.0, 9.0, 11.0, 21.0],
        11.0 => [11.0, 11.0, 21.0, 21.0],
        21.0 => [21.0, 21.0, 21.0, 21.0],
        _ => [normal, normal, 7.0, 21.0],
    })
}

fn yellow(hue: f64) -> bool {
    (105.0..125.0).contains(&hue)
}
fn blue(hue: f64) -> bool {
    (250.0..270.0).contains(&hue)
}
fn cyan(hue: f64) -> bool {
    (170.0..207.0).contains(&hue)
}

fn piecewise(hue: f64, boundaries: &[f64], values: &[f64]) -> f64 {
    for (index, interval) in boundaries.windows(2).enumerate() {
        if hue >= interval[0] && hue < interval[1] {
            return values[index];
        }
    }
    0.0
}

fn rotate(hue: f64, boundaries: &[f64], rotations: &[f64]) -> f64 {
    (hue + piecewise(hue, boundaries, rotations)).rem_euclid(360.0)
}

/// Constructs palettes using the exact source-spec fallback selected by the caller.
pub(super) fn palettes(
    seed: Argb,
    variant: Variant,
    dark: bool,
    contrast: f64,
    spec: MaterialColorSpec,
) -> DynamicScheme {
    let mut scheme = DynamicScheme::by_variant(seed, &variant, dark, Some(contrast));
    let h = scheme.source_color_hct.get_hue();
    if spec == MaterialColorSpec::Spec2021 {
        // The older Rust port used chroma 10 for Vibrant's variant palette and
        // excluded exact hue interval boundaries from rotation.
        if variant == Variant::Vibrant {
            scheme.neutral_variant_palette = TonalPalette::of(h, 12.0);
        }
        let (breaks, secondary, tertiary): (&[f64], &[f64], &[f64]) = match variant {
            Variant::Expressive => (
                &[0.0, 21.0, 51.0, 121.0, 151.0, 191.0, 271.0, 321.0, 360.0],
                &[45.0, 95.0, 45.0, 20.0, 45.0, 90.0, 45.0, 45.0, 45.0],
                &[120.0, 120.0, 20.0, 45.0, 20.0, 15.0, 20.0, 120.0, 120.0],
            ),
            Variant::Vibrant => (
                &[0.0, 41.0, 61.0, 101.0, 131.0, 181.0, 251.0, 301.0, 360.0],
                &[18.0, 15.0, 10.0, 12.0, 15.0, 18.0, 15.0, 12.0, 12.0],
                &[35.0, 30.0, 20.0, 25.0, 30.0, 35.0, 30.0, 25.0, 25.0],
            ),
            _ => return scheme,
        };
        scheme.secondary_palette = TonalPalette::of(rotate(h, breaks, secondary), 24.0);
        scheme.tertiary_palette = TonalPalette::of(rotate(h, breaks, tertiary), 32.0);
        return scheme;
    }
    let expressive_hue = rotate(
        h,
        &[0.0, 71.0, 124.0, 253.0, 278.0, 300.0, 360.0],
        &[10.0, 0.0, 10.0, 0.0, 10.0, 0.0],
    );
    let vibrant_hue = rotate(
        h,
        &[0.0, 38.0, 105.0, 140.0, 333.0, 360.0],
        &[-14.0, 10.0, -14.0, 10.0, -14.0],
    );
    let (
        primary_c,
        secondary_h,
        secondary_c,
        tertiary_h,
        tertiary_c,
        neutral_h,
        neutral_c,
        variant_c,
        error_c,
    ) = match variant {
        Variant::Neutral => (
            if blue(h) { 12.0 } else { 8.0 },
            h,
            if blue(h) { 6.0 } else { 4.0 },
            rotate(
                h,
                &[0.0, 38.0, 105.0, 161.0, 204.0, 278.0, 333.0, 360.0],
                &[-32.0, 26.0, 10.0, -39.0, 24.0, -15.0, -32.0],
            ),
            20.0,
            h,
            1.4,
            1.4 * 2.2,
            50.0,
        ),
        Variant::TonalSpot => (
            if dark { 26.0 } else { 32.0 },
            h,
            16.0,
            rotate(
                h,
                &[0.0, 20.0, 71.0, 161.0, 333.0, 360.0],
                &[-40.0, 48.0, -32.0, 40.0, -32.0],
            ),
            28.0,
            h,
            5.0,
            5.0 * 1.7,
            60.0,
        ),
        Variant::Expressive => {
            let neutral_c = if dark {
                if yellow(expressive_hue) {
                    6.0
                } else {
                    14.0
                }
            } else {
                18.0
            };
            (
                if dark { 36.0 } else { 48.0 },
                rotate(
                    h,
                    &[0.0, 105.0, 140.0, 204.0, 253.0, 278.0, 300.0, 333.0, 360.0],
                    &[-160.0, 155.0, -100.0, 96.0, -96.0, -156.0, -165.0, -160.0],
                ),
                if dark { 16.0 } else { 24.0 },
                rotate(
                    h,
                    &[0.0, 105.0, 140.0, 204.0, 253.0, 278.0, 300.0, 333.0, 360.0],
                    &[-165.0, 160.0, -105.0, 101.0, -101.0, -160.0, -170.0, -165.0],
                ),
                48.0,
                expressive_hue,
                neutral_c,
                neutral_c * if yellow(expressive_hue) { 1.6 } else { 2.3 },
                64.0,
            )
        }
        Variant::Vibrant => (
            74.0,
            vibrant_hue,
            56.0,
            rotate(
                h,
                &[0.0, 38.0, 71.0, 105.0, 140.0, 161.0, 253.0, 333.0, 360.0],
                &[-72.0, 35.0, 24.0, -24.0, 62.0, 50.0, 62.0, -72.0],
            ),
            56.0,
            vibrant_hue,
            28.0,
            28.0 * 1.29,
            80.0,
        ),
        _ => unreachable!("Unsupported styles must use the upstream 2021 fallback"),
    };
    scheme.primary_palette = TonalPalette::of(h, primary_c);
    scheme.secondary_palette = TonalPalette::of(secondary_h, secondary_c);
    scheme.tertiary_palette = TonalPalette::of(tertiary_h, tertiary_c);
    scheme.neutral_palette = TonalPalette::of(neutral_h, neutral_c);
    scheme.neutral_variant_palette = TonalPalette::of(neutral_h, variant_c);
    scheme.error_palette = TonalPalette::of(
        piecewise(
            h,
            &[0.0, 3.0, 13.0, 23.0, 33.0, 43.0, 153.0, 273.0, 360.0],
            &[12.0, 22.0, 32.0, 12.0, 22.0, 32.0, 22.0, 12.0],
        ),
        error_c,
    );
    scheme
}

/// Upstream's May 2025 yellow-T99 correction is part of both current specs.
/// It deliberately averages the adjacent encoded sRGB colors, unlike ordinary
/// palette tones, which always go through the HCT gamut solver.
pub(super) fn palette_argb(palette: &TonalPalette, tone: f64, multiplier: f64) -> Argb {
    let h = palette.hue();
    let c = palette.chroma() * multiplier;
    if tone == 99.0 && yellow(h) {
        let a: Argb = Hct::from(h, c, 98.0).into();
        let b: Argb = Hct::from(h, c, 100.0).into();
        let average = |a: u8, b: u8| ((u16::from(a) + u16::from(b) + 1) / 2) as u8;
        Argb::new(
            255,
            average(a.red, b.red),
            average(a.green, b.green),
            average(a.blue, b.blue),
        )
    } else {
        Hct::from(h, c, tone).into()
    }
}

#[derive(Clone, Copy)]
enum Delta {
    /// 2021 joint solution; the first role is the near background role unless
    /// this is a fixed-color pair, in which case it is the lighter role.
    Joint { a: R, b: R, fixed: bool },
    /// 2025 independent solution relative to an already resolved reference.
    Relative {
        reference: R,
        amount: f64,
        exact: bool,
    },
}

struct Definition {
    initial_tone: f64,
    background: Option<R>,
    second_background: Option<R>,
    contrast: Option<ContrastCurve>,
    is_background: bool,
    delta: Option<Delta>,
    multiplier: f64,
}

impl Definition {
    fn new(initial_tone: f64) -> Self {
        Self {
            initial_tone,
            background: None,
            second_background: None,
            contrast: None,
            is_background: false,
            delta: None,
            multiplier: 1.0,
        }
    }
    fn on(&mut self, background: R, contrast: ContrastCurve) {
        self.background = Some(background);
        self.contrast = Some(contrast);
    }
}

pub(super) struct RoleResolver<'a> {
    scheme: &'a DynamicScheme,
    spec: MaterialColorSpec,
    dark: bool,
    contrast: f64,
    tones: RefCell<[Option<f64>; 48]>,
}

impl<'a> RoleResolver<'a> {
    pub(super) fn new(scheme: &'a DynamicScheme, spec: MaterialColorSpec) -> Self {
        Self {
            scheme,
            spec,
            dark: scheme.is_dark,
            contrast: scheme.contrast_level,
            tones: RefCell::new([None; 48]),
        }
    }

    fn temporary_light(&self) -> Self {
        Self {
            scheme: self.scheme,
            spec: self.spec,
            dark: false,
            contrast: 0.0,
            tones: RefCell::new([None; 48]),
        }
    }

    fn highest(&self) -> R {
        if self.dark {
            R::SurfaceBright
        } else {
            R::SurfaceDim
        }
    }

    fn normalized(&self, role: R) -> R {
        if self.spec == MaterialColorSpec::Spec2021 {
            return role;
        }
        match role {
            R::Background => R::Surface,
            R::OnBackground => R::OnSurface,
            R::SurfaceVariant => R::SurfaceContainerHighest,
            R::SurfaceTint => R::Primary,
            _ => role,
        }
    }

    fn palette(&self, role: R) -> &TonalPalette {
        match self.normalized(role) {
            R::Primary
            | R::OnPrimary
            | R::PrimaryContainer
            | R::OnPrimaryContainer
            | R::PrimaryFixed
            | R::PrimaryFixedDim
            | R::OnPrimaryFixed
            | R::OnPrimaryFixedVariant
            | R::InversePrimary
            | R::SurfaceTint => &self.scheme.primary_palette,
            R::Secondary
            | R::OnSecondary
            | R::SecondaryContainer
            | R::OnSecondaryContainer
            | R::SecondaryFixed
            | R::SecondaryFixedDim
            | R::OnSecondaryFixed
            | R::OnSecondaryFixedVariant => &self.scheme.secondary_palette,
            R::Tertiary
            | R::OnTertiary
            | R::TertiaryContainer
            | R::OnTertiaryContainer
            | R::TertiaryFixed
            | R::TertiaryFixedDim
            | R::OnTertiaryFixed
            | R::OnTertiaryFixedVariant => &self.scheme.tertiary_palette,
            R::Error | R::OnError | R::ErrorContainer | R::OnErrorContainer => {
                &self.scheme.error_palette
            }
            R::SurfaceVariant | R::OnSurfaceVariant | R::Outline | R::OutlineVariant
                if self.spec == MaterialColorSpec::Spec2021 =>
            {
                &self.scheme.neutral_variant_palette
            }
            _ => &self.scheme.neutral_palette,
        }
    }

    pub(super) fn argb(&self, role: R) -> Argb {
        let role = self.normalized(role);
        palette_argb(self.palette(role), self.tone(role), self.multiplier(role))
    }

    fn tone(&self, role: R) -> f64 {
        let role = self.normalized(role);
        if let Some(value) = self.tones.borrow()[role as usize] {
            return value;
        }
        let definition = self.definition(role);
        let result = if self.spec == MaterialColorSpec::Spec2021 {
            self.resolve_2021(role, &definition)
        } else {
            self.resolve_2025(role, &definition)
        };
        self.tones.borrow_mut()[role as usize] = Some(result);
        result
    }

    fn definition(&self, role: R) -> Definition {
        if self.spec == MaterialColorSpec::Spec2021 {
            self.definition_2021(role)
        } else {
            self.definition_2025(role)
        }
    }

    fn definition_2021(&self, role: R) -> Definition {
        let mono = self.scheme.variant == Variant::Monochrome;
        let fidelity = matches!(self.scheme.variant, Variant::Fidelity | Variant::Content);
        let d = self.dark;
        let choose = |dark, light| if d { dark } else { light };
        let tone = match role {
            R::Background | R::Surface => choose(6.0, 98.0),
            R::OnBackground | R::OnSurface => choose(90.0, 10.0),
            R::SurfaceDim => {
                if d {
                    6.0
                } else {
                    curve([87.0, 87.0, 80.0, 75.0]).get(self.contrast)
                }
            }
            R::SurfaceBright => {
                if d {
                    curve([24.0, 24.0, 29.0, 34.0]).get(self.contrast)
                } else {
                    98.0
                }
            }
            R::SurfaceContainerLowest => {
                if d {
                    curve([4.0, 4.0, 2.0, 0.0]).get(self.contrast)
                } else {
                    100.0
                }
            }
            R::SurfaceContainerLow => curve(if d {
                [10.0, 10.0, 11.0, 12.0]
            } else {
                [96.0, 96.0, 96.0, 95.0]
            })
            .get(self.contrast),
            R::SurfaceContainer => curve(if d {
                [12.0, 12.0, 16.0, 20.0]
            } else {
                [94.0, 94.0, 92.0, 90.0]
            })
            .get(self.contrast),
            R::SurfaceContainerHigh => curve(if d {
                [17.0, 17.0, 21.0, 25.0]
            } else {
                [92.0, 92.0, 88.0, 85.0]
            })
            .get(self.contrast),
            R::SurfaceContainerHighest => curve(if d {
                [22.0, 22.0, 26.0, 30.0]
            } else {
                [90.0, 90.0, 84.0, 80.0]
            })
            .get(self.contrast),
            R::SurfaceVariant => choose(30.0, 90.0),
            R::OnSurfaceVariant => choose(80.0, 30.0),
            R::InverseSurface => choose(90.0, 20.0),
            R::InverseOnSurface => choose(20.0, 95.0),
            R::Outline => choose(60.0, 50.0),
            R::OutlineVariant => choose(30.0, 80.0),
            R::Scrim => 0.0,
            R::SurfaceTint | R::Secondary | R::Error => choose(80.0, 40.0),
            R::Primary => {
                if mono {
                    choose(100.0, 0.0)
                } else {
                    choose(80.0, 40.0)
                }
            }
            R::OnPrimary => {
                if mono {
                    choose(10.0, 90.0)
                } else {
                    choose(20.0, 100.0)
                }
            }
            R::PrimaryContainer => {
                if fidelity {
                    self.scheme.source_color_hct.get_tone().clamp(0.0, 100.0)
                } else if mono {
                    choose(85.0, 25.0)
                } else {
                    choose(30.0, 90.0)
                }
            }
            R::OnPrimaryContainer => {
                if fidelity {
                    DynamicColor::foreground_tone(
                        self.definition_2021(R::PrimaryContainer).initial_tone,
                        4.5,
                    )
                } else if mono {
                    choose(0.0, 100.0)
                } else {
                    choose(90.0, 30.0)
                }
            }
            R::InversePrimary => choose(40.0, 80.0),
            R::OnSecondary => {
                if mono {
                    choose(10.0, 100.0)
                } else {
                    choose(20.0, 100.0)
                }
            }
            R::SecondaryContainer => {
                if mono {
                    choose(30.0, 85.0)
                } else if fidelity {
                    desired_chroma_2021(&self.scheme.secondary_palette, choose(30.0, 90.0), !d)
                } else {
                    choose(30.0, 90.0)
                }
            }
            R::OnSecondaryContainer => {
                if mono {
                    choose(90.0, 10.0)
                } else if fidelity {
                    DynamicColor::foreground_tone(
                        self.definition_2021(R::SecondaryContainer).initial_tone,
                        4.5,
                    )
                } else {
                    choose(90.0, 30.0)
                }
            }
            R::Tertiary => {
                if mono {
                    choose(90.0, 25.0)
                } else {
                    choose(80.0, 40.0)
                }
            }
            R::OnTertiary => {
                if mono {
                    choose(10.0, 90.0)
                } else {
                    choose(20.0, 100.0)
                }
            }
            R::TertiaryContainer => {
                if mono {
                    choose(60.0, 49.0)
                } else if fidelity {
                    fix_if_disliked(
                        palette_argb(
                            &self.scheme.tertiary_palette,
                            self.scheme.source_color_hct.get_tone().clamp(0.0, 100.0),
                            1.0,
                        )
                        .into(),
                    )
                    .get_tone()
                    .clamp(0.0, 100.0)
                } else {
                    choose(30.0, 90.0)
                }
            }
            R::OnTertiaryContainer => {
                if mono {
                    choose(0.0, 100.0)
                } else if fidelity {
                    DynamicColor::foreground_tone(
                        self.definition_2021(R::TertiaryContainer).initial_tone,
                        4.5,
                    )
                } else {
                    choose(90.0, 30.0)
                }
            }
            R::OnError => choose(20.0, 100.0),
            R::ErrorContainer => choose(30.0, 90.0),
            R::OnErrorContainer => {
                if mono {
                    choose(90.0, 10.0)
                } else {
                    choose(90.0, 30.0)
                }
            }
            R::PrimaryFixed | R::TertiaryFixed => {
                if mono {
                    40.0
                } else {
                    90.0
                }
            }
            R::PrimaryFixedDim | R::TertiaryFixedDim => {
                if mono {
                    30.0
                } else {
                    80.0
                }
            }
            R::OnPrimaryFixed | R::OnTertiaryFixed => {
                if mono {
                    100.0
                } else {
                    10.0
                }
            }
            R::OnPrimaryFixedVariant | R::OnTertiaryFixedVariant => {
                if mono {
                    90.0
                } else {
                    30.0
                }
            }
            R::SecondaryFixed => {
                if mono {
                    80.0
                } else {
                    90.0
                }
            }
            R::SecondaryFixedDim => {
                if mono {
                    70.0
                } else {
                    80.0
                }
            }
            R::OnSecondaryFixed => 10.0,
            R::OnSecondaryFixedVariant => {
                if mono {
                    25.0
                } else {
                    30.0
                }
            }
        };
        let mut result = Definition::new(tone);
        match role {
            R::Background
            | R::Surface
            | R::SurfaceDim
            | R::SurfaceBright
            | R::SurfaceContainerLowest
            | R::SurfaceContainerLow
            | R::SurfaceContainer
            | R::SurfaceContainerHigh
            | R::SurfaceContainerHighest
            | R::SurfaceVariant
            | R::InverseSurface
            | R::SurfaceTint => result.is_background = true,
            R::OnBackground => result.on(R::Background, curve([3.0, 3.0, 4.5, 7.0])),
            R::OnSurface => result.on(self.highest(), curve([4.5, 7.0, 11.0, 21.0])),
            R::OnSurfaceVariant => result.on(self.highest(), curve([3.0, 4.5, 7.0, 11.0])),
            R::InverseOnSurface => result.on(R::InverseSurface, curve([4.5, 7.0, 11.0, 21.0])),
            R::Outline => result.on(self.highest(), curve([1.5, 3.0, 4.5, 7.0])),
            R::OutlineVariant => result.on(self.highest(), curve([1.0, 1.0, 3.0, 4.5])),
            R::Primary
            | R::Secondary
            | R::Tertiary
            | R::Error
            | R::PrimaryContainer
            | R::SecondaryContainer
            | R::TertiaryContainer
            | R::ErrorContainer => {
                let (main, container) = accent_pair(role);
                result.is_background = true;
                result.on(
                    self.highest(),
                    curve(if role == main {
                        [3.0, 4.5, 7.0, 7.0]
                    } else {
                        [1.0, 1.0, 3.0, 4.5]
                    }),
                );
                result.delta = Some(Delta::Joint {
                    a: container,
                    b: main,
                    fixed: false,
                });
            }
            R::OnPrimary | R::OnSecondary | R::OnTertiary | R::OnError => {
                result.on(accent_pair(role).0, curve([4.5, 7.0, 11.0, 21.0]))
            }
            R::OnPrimaryContainer
            | R::OnSecondaryContainer
            | R::OnTertiaryContainer
            | R::OnErrorContainer => result.on(accent_pair(role).1, curve([3.0, 4.5, 7.0, 11.0])),
            R::InversePrimary => result.on(R::InverseSurface, curve([3.0, 4.5, 7.0, 7.0])),
            R::PrimaryFixed
            | R::PrimaryFixedDim
            | R::SecondaryFixed
            | R::SecondaryFixedDim
            | R::TertiaryFixed
            | R::TertiaryFixedDim => {
                let (a, b) = fixed_pair(role);
                result.is_background = true;
                result.on(self.highest(), curve([1.0, 1.0, 3.0, 4.5]));
                result.delta = Some(Delta::Joint { a, b, fixed: true });
            }
            R::OnPrimaryFixed
            | R::OnSecondaryFixed
            | R::OnTertiaryFixed
            | R::OnPrimaryFixedVariant
            | R::OnSecondaryFixedVariant
            | R::OnTertiaryFixedVariant => {
                let (a, b) = fixed_pair(role);
                result.on(
                    b,
                    curve(
                        if matches!(
                            role,
                            R::OnPrimaryFixed | R::OnSecondaryFixed | R::OnTertiaryFixed
                        ) {
                            [4.5, 7.0, 11.0, 21.0]
                        } else {
                            [3.0, 4.5, 7.0, 11.0]
                        },
                    ),
                );
                result.second_background = Some(a);
            }
            R::Scrim => {}
        }
        result
    }

    fn definition_2025(&self, role: R) -> Definition {
        let v = self.scheme.variant.clone();
        let d = self.dark;
        let p = self.palette(role);
        let y = yellow(self.scheme.neutral_palette.hue());
        let surface_tone = |dark, normal, vibrant, yellow_tone| {
            if d {
                dark
            } else if y {
                yellow_tone
            } else if v == Variant::Vibrant {
                vibrant
            } else {
                normal
            }
        };
        let tone = match role {
            R::Surface => surface_tone(4.0, 98.0, 97.0, 99.0),
            R::SurfaceDim => surface_tone(4.0, 87.0, 85.0, 90.0),
            R::SurfaceBright => surface_tone(18.0, 98.0, 97.0, 99.0),
            R::SurfaceContainerLowest => {
                if d {
                    0.0
                } else {
                    100.0
                }
            }
            R::SurfaceContainerLow => surface_tone(6.0, 96.0, 95.0, 98.0),
            R::SurfaceContainer => surface_tone(9.0, 94.0, 92.0, 96.0),
            R::SurfaceContainerHigh => surface_tone(12.0, 92.0, 90.0, 94.0),
            R::SurfaceContainerHighest => surface_tone(15.0, 90.0, 88.0, 92.0),
            R::OnSurface if v == Variant::Vibrant => best_chroma_tone(p, true, 0.0, 100.0, 1.1),
            R::OnSurface | R::OnSurfaceVariant | R::Outline | R::OutlineVariant => {
                self.tone(self.highest())
            }
            R::InverseSurface => {
                if d {
                    98.0
                } else {
                    4.0
                }
            }
            R::InverseOnSurface => self.tone(R::InverseSurface),
            R::Primary => match v {
                Variant::Neutral => {
                    if d {
                        80.0
                    } else {
                        40.0
                    }
                }
                Variant::TonalSpot => {
                    if d {
                        80.0
                    } else {
                        best_chroma_tone(p, true, 0.0, 100.0, 1.0)
                    }
                }
                Variant::Expressive => best_chroma_tone(
                    p,
                    true,
                    0.0,
                    if d {
                        if cyan(p.hue()) {
                            88.0
                        } else {
                            98.0
                        }
                    } else if yellow(p.hue()) {
                        25.0
                    } else {
                        98.0
                    },
                    1.0,
                ),
                _ => best_chroma_tone(p, true, 0.0, if cyan(p.hue()) { 88.0 } else { 98.0 }, 1.0),
            },
            R::Secondary => match v {
                Variant::Neutral => best_chroma_tone(p, !d, 0.0, if d { 98.0 } else { 100.0 }, 1.0),
                Variant::Vibrant => {
                    best_chroma_tone(p, true, 0.0, if d { 90.0 } else { 98.0 }, 1.0)
                }
                _ => {
                    if d {
                        80.0
                    } else {
                        best_chroma_tone(p, true, 0.0, 100.0, 1.0)
                    }
                }
            },
            R::Tertiary => best_chroma_tone(
                p,
                true,
                0.0,
                if matches!(v, Variant::Expressive | Variant::Vibrant) && cyan(p.hue()) {
                    88.0
                } else if d {
                    98.0
                } else {
                    100.0
                },
                1.0,
            ),
            R::Error => best_chroma_tone(p, !d, 0.0, if d { 98.0 } else { 100.0 }, 1.0),
            R::PrimaryContainer => match v {
                Variant::Neutral => {
                    if d {
                        30.0
                    } else {
                        90.0
                    }
                }
                Variant::TonalSpot => {
                    if d {
                        best_chroma_tone(p, false, 35.0, 93.0, 1.0)
                    } else {
                        best_chroma_tone(p, true, 0.0, 90.0, 1.0)
                    }
                }
                Variant::Expressive => {
                    if d {
                        best_chroma_tone(p, false, 30.0, 93.0, 1.0)
                    } else {
                        best_chroma_tone(
                            p,
                            true,
                            78.0,
                            if cyan(p.hue()) { 88.0 } else { 90.0 },
                            1.0,
                        )
                    }
                }
                _ => best_chroma_tone(
                    p,
                    !d,
                    66.0,
                    if !d && cyan(p.hue()) { 88.0 } else { 93.0 },
                    1.0,
                ),
            },
            R::SecondaryContainer => match v {
                Variant::Vibrant => {
                    if d {
                        best_chroma_tone(p, false, 30.0, 40.0, 1.0)
                    } else {
                        best_chroma_tone(p, true, 84.0, 90.0, 1.0)
                    }
                }
                Variant::Expressive => {
                    if d {
                        15.0
                    } else {
                        best_chroma_tone(p, true, 90.0, 95.0, 1.0)
                    }
                }
                _ => {
                    if d {
                        25.0
                    } else {
                        90.0
                    }
                }
            },
            R::TertiaryContainer => match v {
                Variant::Neutral => {
                    best_chroma_tone(p, true, 0.0, if d { 93.0 } else { 96.0 }, 1.0)
                }
                Variant::TonalSpot => {
                    best_chroma_tone(p, true, 0.0, if d { 93.0 } else { 100.0 }, 1.0)
                }
                Variant::Expressive => best_chroma_tone(
                    p,
                    true,
                    75.0,
                    if cyan(p.hue()) {
                        88.0
                    } else if d {
                        93.0
                    } else {
                        100.0
                    },
                    1.0,
                ),
                _ => best_chroma_tone(
                    p,
                    true,
                    if d { 0.0 } else { 72.0 },
                    if d { 93.0 } else { 100.0 },
                    1.0,
                ),
            },
            R::ErrorContainer => {
                if d {
                    best_chroma_tone(p, false, 30.0, 93.0, 1.0)
                } else {
                    best_chroma_tone(p, true, 0.0, 90.0, 1.0)
                }
            }
            R::OnPrimary | R::OnSecondary | R::OnTertiary | R::OnError => {
                self.tone(accent_pair(role).0)
            }
            R::OnPrimaryContainer
            | R::OnSecondaryContainer
            | R::OnTertiaryContainer
            | R::OnErrorContainer => self.tone(accent_pair(role).1),
            R::PrimaryFixed | R::SecondaryFixed | R::TertiaryFixed => {
                self.temporary_light().tone(accent_pair(role).1)
            }
            R::PrimaryFixedDim | R::SecondaryFixedDim | R::TertiaryFixedDim => {
                self.tone(fixed_pair(role).0)
            }
            R::OnPrimaryFixed
            | R::OnPrimaryFixedVariant
            | R::OnSecondaryFixed
            | R::OnSecondaryFixedVariant
            | R::OnTertiaryFixed
            | R::OnTertiaryFixedVariant => self.tone(fixed_pair(role).1),
            R::InversePrimary => best_chroma_tone(p, true, 0.0, 100.0, 1.0),
            R::Scrim => 0.0,
            R::Background | R::OnBackground | R::SurfaceVariant | R::SurfaceTint => {
                unreachable!("Aliases are normalized before resolution")
            }
        };
        let mut result = Definition::new(tone);
        match role {
            R::Surface
            | R::SurfaceDim
            | R::SurfaceBright
            | R::SurfaceContainerLowest
            | R::SurfaceContainerLow
            | R::SurfaceContainer
            | R::SurfaceContainerHigh
            | R::SurfaceContainerHighest
            | R::InverseSurface => result.is_background = true,
            R::OnSurface => result.on(self.highest(), curve_2025(if d { 11.0 } else { 9.0 })),
            R::OnSurfaceVariant => result.on(self.highest(), curve_2025(if d { 6.0 } else { 4.5 })),
            R::Outline => result.on(self.highest(), curve_2025(3.0)),
            R::OutlineVariant => result.on(self.highest(), curve_2025(1.5)),
            R::InverseOnSurface => result.on(R::InverseSurface, curve_2025(7.0)),
            R::Primary | R::Secondary | R::Tertiary | R::Error => {
                result.is_background = true;
                result.on(self.highest(), curve_2025(4.5));
                result.delta = Some(Delta::Relative {
                    reference: accent_pair(role).1,
                    amount: if d { 5.0 } else { -5.0 },
                    exact: false,
                });
            }
            R::OnPrimary | R::OnSecondary | R::OnTertiary | R::OnError => {
                result.on(accent_pair(role).0, curve_2025(6.0))
            }
            R::PrimaryContainer
            | R::SecondaryContainer
            | R::TertiaryContainer
            | R::ErrorContainer
            | R::PrimaryFixed
            | R::SecondaryFixed
            | R::TertiaryFixed => {
                result.is_background = true;
                result.background = Some(self.highest());
                if self.contrast > 0.0 {
                    result.contrast = Some(curve_2025(1.5));
                }
            }
            R::OnPrimaryContainer
            | R::OnSecondaryContainer
            | R::OnTertiaryContainer
            | R::OnErrorContainer => result.on(
                accent_pair(role).1,
                curve_2025(if role == R::OnErrorContainer {
                    4.5
                } else {
                    6.0
                }),
            ),
            R::PrimaryFixedDim | R::SecondaryFixedDim | R::TertiaryFixedDim => {
                result.is_background = true;
                result.delta = Some(Delta::Relative {
                    reference: fixed_pair(role).0,
                    amount: -5.0,
                    exact: true,
                });
            }
            R::OnPrimaryFixed | R::OnSecondaryFixed | R::OnTertiaryFixed => {
                result.on(fixed_pair(role).1, curve_2025(7.0))
            }
            R::OnPrimaryFixedVariant | R::OnSecondaryFixedVariant | R::OnTertiaryFixedVariant => {
                result.on(fixed_pair(role).1, curve_2025(4.5))
            }
            R::InversePrimary => result.on(R::InverseSurface, curve_2025(6.0)),
            _ => {}
        }
        result.multiplier = self.multiplier(role);
        result
    }

    fn multiplier(&self, role: R) -> f64 {
        if self.spec == MaterialColorSpec::Spec2021 {
            return 1.0;
        }
        let v = self.scheme.variant.clone();
        let d = self.dark;
        let y = yellow(self.scheme.neutral_palette.hue());
        let select = |neutral, tonal, expressive, expressive_yellow, vibrant| match v {
            Variant::Neutral => neutral,
            Variant::TonalSpot => tonal,
            Variant::Expressive => {
                if y {
                    expressive_yellow
                } else {
                    expressive
                }
            }
            Variant::Vibrant => vibrant,
            _ => 1.0,
        };
        match role {
            R::SurfaceDim if !d => select(2.5, 1.7, 1.75, 2.7, 1.36),
            R::SurfaceBright if d => select(2.5, 1.7, 1.75, 2.7, 1.36),
            R::SurfaceContainerLow => select(1.3, 1.25, 1.15, 1.3, 1.08),
            R::SurfaceContainer => select(1.6, 1.4, 1.3, 1.6, 1.15),
            R::SurfaceContainerHigh => select(1.9, 1.5, 1.45, 1.95, 1.22),
            R::SurfaceContainerHighest => select(2.2, 1.7, 1.6, 2.3, 1.29),
            R::OnSurface | R::OnSurfaceVariant | R::Outline | R::OutlineVariant => {
                select(2.2, 1.7, 1.6, if d { 3.0 } else { 2.3 }, 1.0)
            }
            _ => 1.0,
        }
    }

    fn resolve_2021(&self, role: R, definition: &Definition) -> f64 {
        if let Some(Delta::Joint { a, b, fixed }) = definition.delta {
            let (near, far) = if !fixed || !self.dark { (a, b) } else { (b, a) };
            let n = self.definition_2021(near);
            let f = self.definition_2021(far);
            let mut nt = n.initial_tone;
            let mut ft = f.initial_tone;
            let direction = if self.dark { 1.0 } else { -1.0 };
            if let (Some(background), Some(nc), Some(fc)) =
                (definition.background, n.contrast, f.contrast)
            {
                let bg = self.tone(background);
                let nr = nc.get(self.contrast);
                let fr = fc.get(self.contrast);
                if self.contrast < 0.0 || ratio_of_tones(bg, nt) < nr {
                    nt = DynamicColor::foreground_tone(bg, nr);
                }
                if self.contrast < 0.0 || ratio_of_tones(bg, ft) < fr {
                    ft = DynamicColor::foreground_tone(bg, fr);
                }
            }
            if (ft - nt) * direction < 10.0 {
                ft = (nt + 10.0 * direction).clamp(0.0, 100.0);
                if (ft - nt) * direction < 10.0 {
                    nt = (ft - 10.0 * direction).clamp(0.0, 100.0);
                }
            }
            if (50.0..60.0).contains(&nt) || ((50.0..60.0).contains(&ft) && fixed) {
                if direction > 0.0 {
                    nt = 60.0;
                    ft = ft.max(nt + 10.0 * direction);
                } else {
                    nt = 49.0;
                    ft = ft.min(nt + 10.0 * direction);
                }
            } else if (50.0..60.0).contains(&ft) {
                ft = if direction > 0.0 { 60.0 } else { 49.0 };
            }
            return if role == near { nt } else { ft };
        }
        let mut answer = definition.initial_tone;
        let (Some(background), Some(contrast)) =
            (definition.background, definition.contrast.as_ref())
        else {
            return answer;
        };
        let bg = self.tone(background);
        let ratio = contrast.get(self.contrast);
        if self.contrast < 0.0 || ratio_of_tones(bg, answer) < ratio {
            answer = DynamicColor::foreground_tone(bg, ratio);
        }
        if definition.is_background && (50.0..60.0).contains(&answer) {
            answer = if ratio_of_tones(49.0, bg) >= ratio {
                49.0
            } else {
                60.0
            };
        }
        self.resolve_second_background(definition, bg, ratio, answer)
    }

    fn resolve_2025(&self, role: R, definition: &Definition) -> f64 {
        let mut answer = definition.initial_tone;
        if let Some(Delta::Relative {
            reference,
            amount,
            exact,
        }) = definition.delta
        {
            let target = self.tone(reference) + amount;
            answer = if exact {
                target.clamp(0.0, 100.0)
            } else if amount > 0.0 {
                answer.clamp(target, 100.0)
            } else {
                answer.clamp(0.0, target)
            };
            if let (Some(background), Some(contrast)) =
                (definition.background, definition.contrast.as_ref())
            {
                let bg = self.tone(background);
                let ratio = contrast.get(self.contrast);
                if self.contrast < 0.0 || ratio_of_tones(bg, answer) < ratio {
                    answer = DynamicColor::foreground_tone(bg, ratio);
                }
            }
            return avoid_2025_zone(role, definition.is_background, answer);
        }
        let (Some(background), Some(contrast)) =
            (definition.background, definition.contrast.as_ref())
        else {
            return answer;
        };
        let bg = self.tone(background);
        let ratio = contrast.get(self.contrast);
        if self.contrast < 0.0 || ratio_of_tones(bg, answer) < ratio {
            answer = DynamicColor::foreground_tone(bg, ratio);
        }
        answer = avoid_2025_zone(role, definition.is_background, answer);
        self.resolve_second_background(definition, bg, ratio, answer)
    }

    fn resolve_second_background(
        &self,
        definition: &Definition,
        bg: f64,
        ratio: f64,
        answer: f64,
    ) -> f64 {
        let Some(other) = definition.second_background else {
            return answer;
        };
        let other = self.tone(other);
        let upper = bg.max(other);
        let lower = bg.min(other);
        if ratio_of_tones(upper, answer) >= ratio && ratio_of_tones(lower, answer) >= ratio {
            return answer;
        }
        let light = lighter(upper, ratio);
        let dark = darker(lower, ratio);
        if DynamicColor::tone_prefers_light_foreground(bg)
            || DynamicColor::tone_prefers_light_foreground(other)
        {
            return if light < 0.0 { 100.0 } else { light };
        }
        if light >= 0.0 && dark < 0.0 {
            return light;
        }
        if dark < 0.0 {
            0.0
        } else {
            dark
        }
    }
}

fn avoid_2025_zone(role: R, is_background: bool, tone: f64) -> f64 {
    if !is_background
        || matches!(
            role,
            R::PrimaryFixedDim | R::SecondaryFixedDim | R::TertiaryFixedDim
        )
    {
        return tone;
    }
    if tone >= 57.0 {
        tone.clamp(65.0, 100.0)
    } else {
        tone.clamp(0.0, 49.0)
    }
}

fn accent_pair(role: R) -> (R, R) {
    match role {
        R::Primary
        | R::OnPrimary
        | R::PrimaryContainer
        | R::OnPrimaryContainer
        | R::PrimaryFixed
        | R::PrimaryFixedDim
        | R::OnPrimaryFixed
        | R::OnPrimaryFixedVariant => (R::Primary, R::PrimaryContainer),
        R::Secondary
        | R::OnSecondary
        | R::SecondaryContainer
        | R::OnSecondaryContainer
        | R::SecondaryFixed
        | R::SecondaryFixedDim
        | R::OnSecondaryFixed
        | R::OnSecondaryFixedVariant => (R::Secondary, R::SecondaryContainer),
        R::Tertiary
        | R::OnTertiary
        | R::TertiaryContainer
        | R::OnTertiaryContainer
        | R::TertiaryFixed
        | R::TertiaryFixedDim
        | R::OnTertiaryFixed
        | R::OnTertiaryFixedVariant => (R::Tertiary, R::TertiaryContainer),
        R::Error | R::OnError | R::ErrorContainer | R::OnErrorContainer => {
            (R::Error, R::ErrorContainer)
        }
        _ => unreachable!("Role is not an accent color"),
    }
}

fn fixed_pair(role: R) -> (R, R) {
    match role {
        R::PrimaryFixed | R::PrimaryFixedDim | R::OnPrimaryFixed | R::OnPrimaryFixedVariant => {
            (R::PrimaryFixed, R::PrimaryFixedDim)
        }
        R::SecondaryFixed
        | R::SecondaryFixedDim
        | R::OnSecondaryFixed
        | R::OnSecondaryFixedVariant => (R::SecondaryFixed, R::SecondaryFixedDim),
        R::TertiaryFixed | R::TertiaryFixedDim | R::OnTertiaryFixed | R::OnTertiaryFixedVariant => {
            (R::TertiaryFixed, R::TertiaryFixedDim)
        }
        _ => unreachable!("Role is not a fixed accent color"),
    }
}

fn desired_chroma_2021(palette: &TonalPalette, tone: f64, decreasing: bool) -> f64 {
    let mut answer = tone;
    let mut closest = Hct::from(palette.hue(), palette.chroma(), tone);
    if closest.get_chroma() < palette.chroma() {
        let mut peak = closest.get_chroma();
        while closest.get_chroma() < palette.chroma() {
            answer += if decreasing { -1.0 } else { 1.0 };
            let candidate = Hct::from(palette.hue(), palette.chroma(), answer);
            if peak > candidate.get_chroma()
                || (candidate.get_chroma() - palette.chroma()).abs() < 0.4
            {
                break;
            }
            if (candidate.get_chroma() - palette.chroma()).abs()
                < (closest.get_chroma() - palette.chroma()).abs()
            {
                closest = candidate;
            }
            peak = peak.max(candidate.get_chroma());
        }
    }
    answer
}

fn best_chroma_tone(
    palette: &TonalPalette,
    decreasing: bool,
    lower: f64,
    upper: f64,
    multiplier: f64,
) -> f64 {
    let chroma = palette.chroma() * multiplier;
    let mut tone = if decreasing { 100.0 } else { 0.0 };
    let mut answer = tone;
    let mut best = Hct::from(palette.hue(), chroma, answer);
    while best.get_chroma() < chroma {
        if !(0.0..=100.0).contains(&tone) {
            break;
        }
        tone += if decreasing { -1.0 } else { 1.0 };
        let candidate = Hct::from(palette.hue(), chroma, tone);
        if best.get_chroma() < candidate.get_chroma() {
            best = candidate;
            answer = tone;
        }
    }
    answer.clamp(lower, upper)
}
