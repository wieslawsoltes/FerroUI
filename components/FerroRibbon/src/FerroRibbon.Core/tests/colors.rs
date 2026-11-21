use ferroribbon_core::{RibbonString, theming::*};
use serde_json::{Value, json};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/colors.json")).unwrap()
}
fn string(v: &Value) -> Option<RibbonString> {
    v.as_array().map(|a| {
        RibbonString::from_utf16(
            a.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn color(hex: &str) -> RibbonColor {
    RibbonColor::parse(Some(&hex.into())).unwrap()
}
fn double(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn bits(v: f64) -> Value {
    json!(format!("{:016x}", v.to_bits()))
}
#[test]
fn original_color_parse_and_format_tests() {
    for (s, expected) in [
        ("#FF0000", RibbonColor::new(255, 255, 0, 0)),
        ("80FF0000", RibbonColor::new(128, 255, 0, 0)),
        ("#abc", RibbonColor::new(255, 170, 187, 204)),
    ] {
        assert_eq!(color(s), expected);
    }
    assert_eq!(color("#185abd").to_hex(false), "#185ABD");
    assert_eq!(color("#80FF0000").to_hex(false), "#80FF0000");
    for s in ["#12345", "zzzzzz"] {
        assert!(RibbonColor::parse(Some(&s.into())).is_err());
    }
}
#[test]
fn original_hsl_lighten_darken_and_contrast_tests() {
    let blue = color("#4472C4");
    let (h, s, l) = blue.to_hsl();
    let back = RibbonColor::from_hsl(h, s, l);
    for (a, b) in [(blue.r, back.r), (blue.g, back.g), (blue.b, back.b)] {
        assert!(a.abs_diff(b) <= 1);
    }
    assert!(blue.lighten(0.4).lightness() > blue.lightness());
    assert!(blue.darken(0.25).lightness() < blue.lightness());
    assert_eq!(RibbonColor::BLACK.lighten(1.0), RibbonColor::WHITE);
    assert_eq!(
        RibbonColor::contrast_ratio(RibbonColor::BLACK, RibbonColor::WHITE).round(),
        21.0
    );
    assert!(color("#FFFF00").prefers_dark_foreground());
    assert!(!color("#185ABD").prefers_dark_foreground());
}
#[test]
fn reference_parsing_keeps_utf16_trim_hash_and_numeric_edges() {
    let data = corpus();
    for row in data["parse"].as_array().unwrap() {
        let input = string(&row["input"]);
        let mut actual = RibbonColor::WHITE;
        assert_eq!(
            RibbonColor::try_parse(input.as_ref(), &mut actual),
            row["ok"].as_bool().unwrap(),
            "{row}"
        );
        assert_eq!(json!(actual.to_hex(true)), row["color"], "{row}");
        if let Err(error) = RibbonColor::parse(input.as_ref()) {
            assert_eq!(Some(error.message), string(&row["error"]));
        }
    }
    let expected = data["prefix"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["unit"].as_u64().unwrap() as u16,
                v["color"].as_str().unwrap(),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    for unit in 0..=u16::MAX {
        let mut input = vec![35, unit];
        input.extend("FFFFFFF".encode_utf16());
        let mut result = RibbonColor::WHITE;
        let ok = RibbonColor::try_parse(Some(&RibbonString::from_utf16(input)), &mut result);
        assert_eq!(ok, expected.contains_key(&unit), "{unit:x}");
        if ok {
            assert_eq!(result.to_hex(true), expected[&unit]);
        }
    }
}
#[test]
fn reference_color_numerics_and_rounding_match_exact_outputs() {
    for row in corpus()["calculations"].as_array().unwrap() {
        let c = color(row["color"].as_str().unwrap());
        let other = color(row["other"].as_str().unwrap());
        let (h, s, l) = c.to_hsl();
        assert_eq!(json!([bits(h), bits(s), bits(l)]), row["hsl"], "{row}");
        assert_eq!(bits(c.luminance()), row["luminance"]);
        assert_eq!(bits(RibbonColor::contrast_ratio(c, other)), row["contrast"]);
        assert_eq!(json!(c.prefers_dark_foreground()), row["prefersDark"]);
        assert_eq!(json!(c.to_hex(false)), row["hex"]);
        for variation in row["variations"].as_array().unwrap() {
            let amount = double(&variation["amount"]);
            assert_eq!(
                json!(c.lighten(amount).to_hex(true)),
                variation["light"],
                "{c} {variation}"
            );
            assert_eq!(
                json!(c.darken(amount).to_hex(true)),
                variation["dark"],
                "{c} {variation}"
            );
            assert_eq!(
                json!(c.blend(other, amount).to_hex(true)),
                variation["blend"],
                "{c} {variation}"
            );
        }
    }
    for row in corpus()["hslCases"].as_array().unwrap() {
        assert_eq!(
            json!(
                RibbonColor::from_hsl_with_alpha(
                    double(&row["h"]),
                    double(&row["s"]),
                    double(&row["l"]),
                    123
                )
                .to_hex(true)
            ),
            row["color"],
            "{row}"
        );
    }
}

fn palettes() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/palettes.json")).unwrap()
}
fn swatch(s: &RibbonColorSwatch) -> Value {
    json!({"color":s.color.to_hex(true),"name":s.name.as_ref().map(|s|s.as_utf16())})
}
fn swatches(s: &[Option<std::rc::Rc<RibbonColorSwatch>>]) -> Value {
    json!(
        s.iter()
            .map(|s| swatch(s.as_deref().unwrap()))
            .collect::<Vec<_>>()
    )
}
fn theme(p: &RibbonThemePalette) -> Value {
    json!({"name":p.name.as_ref().map(|s|s.as_utf16()),"accent":p.accent.to_hex(true),"accentDark":p.accent_dark.to_hex(true),"modes":([false,true].into_iter().map(|dark|json!({"dark":dark,"accent":p.get_accent(dark).to_hex(true),"subtle":p.get_accent_subtle(dark).to_hex(true),"strong":p.get_accent_subtle_strong(dark).to_hex(true),"on":p.get_on_accent(dark).to_hex(true)})).collect::<Vec<_>>())})
}
#[test]
fn source_palette_literals_shades_grid_and_accessible_labels() {
    let source = palettes();
    assert_eq!(
        swatches(&RibbonColorPalette::office_theme_colors()),
        source["office"]
    );
    assert_eq!(
        swatches(&RibbonColorPalette::standard_colors()),
        source["standard"]
    );
    assert_eq!(
        swatches(&RibbonColorPalette::highlight_colors()),
        source["highlight"]
    );
    for row in source["shades"].as_array().unwrap() {
        let swatch = RibbonColorSwatch::new(
            color(row["input"]["color"].as_str().unwrap()),
            string(&row["input"]["name"]),
        );
        assert_eq!(
            swatches(&RibbonColorPalette::generate_shades(Some(&swatch))),
            row["shades"]
        );
    }
    let grid = RibbonColorPalette::build_theme_grid(None);
    assert_eq!(
        json!(grid.iter().map(|row| swatches(row)).collect::<Vec<_>>()),
        source["grid"]
    );
    assert_eq!(
        json!(
            RibbonColorPalette::build_theme_grid(Some(&[]))
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>()
        ),
        source["empty"]
    );
    assert_eq!(source["nullError"], "NullReferenceException");
    assert!(
        std::panic::catch_unwind(|| RibbonColorPalette::build_theme_grid(Some(&[None]))).is_err()
    );
    assert_eq!(source["setError"], "NotSupportedException");
    let office = RibbonColorPalette::office_theme_colors();
    assert_eq!(
        std::rc::Rc::ptr_eq(&office, &RibbonColorPalette::office_theme_colors()),
        source["staticSame"].as_bool().unwrap()
    );
    assert_eq!(
        office
            .iter()
            .zip(&grid[0])
            .all(|(a, b)| std::rc::Rc::ptr_eq(a.as_ref().unwrap(), b.as_ref().unwrap())),
        source["baseSame"].as_bool().unwrap()
    );
}
#[test]
fn source_theme_palette_presets_custom_accents_and_identity() {
    let source = palettes();
    let presets = RibbonThemePalette::presets();
    assert_eq!(
        json!(presets.iter().map(|p| theme(p)).collect::<Vec<_>>()),
        source["themes"]
    );
    let methods = [
        RibbonThemePalette::word(),
        RibbonThemePalette::excel(),
        RibbonThemePalette::power_point(),
        RibbonThemePalette::outlook(),
        RibbonThemePalette::one_note(),
        RibbonThemePalette::access(),
        RibbonThemePalette::visio(),
        RibbonThemePalette::project(),
        RibbonThemePalette::publisher(),
        RibbonThemePalette::teams(),
        RibbonThemePalette::graphite(),
        RibbonThemePalette::cad(),
    ];
    assert_eq!(
        json!(
            methods
                .iter()
                .zip(presets.iter())
                .map(|(a, b)| std::rc::Rc::ptr_eq(a, b))
                .collect::<Vec<_>>()
        ),
        source["identities"]
    );
    for (v, row) in source["customs"].as_array().unwrap().iter().enumerate() {
        let v = v as u8;
        assert_eq!(
            theme(&RibbonThemePalette::from_accent_named(
                RibbonColor::new(v, v, v, v),
                string(&row["name"])
            )),
            *row
        );
    }
    assert!(std::rc::Rc::ptr_eq(
        &presets,
        &RibbonThemePalette::presets()
    ));
}
#[test]
fn original_office_grid_and_accessible_palette_tests() {
    let grid = RibbonColorPalette::build_theme_grid(None);
    assert_eq!(grid.len(), 6);
    assert!(grid.iter().all(|r| r.len() == 10));
    for (row, column, text) in [
        (1, 0, "Darker 5%"),
        (1, 1, "Lighter 50%"),
        (1, 4, "Lighter 80%"),
        (5, 4, "Darker 50%"),
    ] {
        assert!(
            grid[row][column]
                .as_ref()
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .to_utf8()
                .unwrap()
                .contains(text)
        );
    }
    assert_eq!(RibbonColorPalette::standard_colors().len(), 10);
    for palette in RibbonThemePalette::presets().iter() {
        assert!(RibbonColor::contrast_ratio(palette.accent, RibbonColor::WHITE) >= 4.5);
        assert!(RibbonColor::contrast_ratio(palette.accent_dark, color("#292929")) >= 3.0);
    }
    let custom = RibbonThemePalette::from_accent(color("#123456"));
    assert!(custom.accent_dark.lightness() > custom.accent.lightness());
}
