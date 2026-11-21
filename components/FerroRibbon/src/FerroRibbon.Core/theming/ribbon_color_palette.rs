use super::RibbonColor;
use crate::RibbonString;
use std::rc::Rc;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RibbonColorSwatch {
    pub color: RibbonColor,
    pub name: Option<RibbonString>,
}
impl RibbonColorSwatch {
    pub fn new(color: RibbonColor, name: Option<RibbonString>) -> Rc<Self> {
        Rc::new(Self { color, name })
    }
}
pub type RibbonColorSwatches = Rc<[Option<Rc<RibbonColorSwatch>>]>;
pub struct RibbonColorPalette;
struct Presets {
    office: RibbonColorSwatches,
    standard: RibbonColorSwatches,
    highlight: RibbonColorSwatches,
}
fn colors(values: &[(&str, &str)]) -> RibbonColorSwatches {
    values
        .iter()
        .map(|&(hex, name)| {
            Some(RibbonColorSwatch::new(
                RibbonColor::parse(Some(&hex.into())).expect("source palette color"),
                Some(name.into()),
            ))
        })
        .collect::<Vec<_>>()
        .into()
}
ferroui_system::ferro_thread_local! {static PRESETS:Presets=Presets{office:colors(super::palette_data::OFFICE),standard:colors(super::palette_data::STANDARD),highlight:colors(super::palette_data::HIGHLIGHT)};}
impl RibbonColorPalette {
    pub fn office_theme_colors() -> RibbonColorSwatches {
        PRESETS.with(|p| p.office.clone())
    }
    pub fn standard_colors() -> RibbonColorSwatches {
        PRESETS.with(|p| p.standard.clone())
    }
    pub fn highlight_colors() -> RibbonColorSwatches {
        PRESETS.with(|p| p.highlight.clone())
    }
    pub fn generate_shades(
        swatch: Option<&RibbonColorSwatch>,
    ) -> Vec<Option<Rc<RibbonColorSwatch>>> {
        let swatch = swatch.expect("NullReference: swatch");
        let l = swatch.color.lightness();
        let steps = if l >= 0.99 {
            [
                (false, 0.05),
                (false, 0.15),
                (false, 0.25),
                (false, 0.35),
                (false, 0.50),
            ]
        } else if l <= 0.01 {
            [
                (true, 0.50),
                (true, 0.35),
                (true, 0.25),
                (true, 0.15),
                (true, 0.05),
            ]
        } else if l < 0.20 {
            [
                (true, 0.90),
                (true, 0.75),
                (true, 0.50),
                (true, 0.25),
                (true, 0.10),
            ]
        } else if l > 0.80 {
            [
                (false, 0.10),
                (false, 0.25),
                (false, 0.50),
                (false, 0.75),
                (false, 0.90),
            ]
        } else {
            [
                (true, 0.80),
                (true, 0.60),
                (true, 0.40),
                (false, 0.25),
                (false, 0.50),
            ]
        };
        steps
            .into_iter()
            .map(|(lighter, amount)| {
                let mut name = swatch
                    .name
                    .as_ref()
                    .map_or_else(Vec::new, |s| s.as_utf16().to_vec());
                name.extend(
                    format!(
                        ", {} {}%",
                        if lighter { "Lighter" } else { "Darker" },
                        (amount * 100.0_f64).round_ties_even() as i32
                    )
                    .encode_utf16(),
                );
                Some(RibbonColorSwatch::new(
                    if lighter {
                        swatch.color.lighten(amount)
                    } else {
                        swatch.color.darken(amount)
                    },
                    Some(RibbonString::from_utf16(name)),
                ))
            })
            .collect()
    }
    pub fn build_theme_grid(
        theme_colors: Option<&[Option<Rc<RibbonColorSwatch>>]>,
    ) -> Vec<Vec<Option<Rc<RibbonColorSwatch>>>> {
        let defaults = theme_colors.is_none().then(Self::office_theme_colors);
        let theme_colors = theme_colors
            .or(defaults.as_deref())
            .expect("theme colors resolved");
        let shades = theme_colors
            .iter()
            .map(|s| Self::generate_shades(s.as_deref()))
            .collect::<Vec<_>>();
        let mut rows = vec![theme_colors.to_vec()];
        for row in 0..5 {
            rows.push(shades.iter().map(|column| column[row].clone()).collect());
        }
        rows
    }
}
