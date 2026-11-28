use ferroribbon_core::{layout::*, *};
use serde_json::Value;

fn f(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn i(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap()
}
fn a(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap()
}
fn equal(actual: f64, expected: &Value, case: usize, field: &str) {
    let expected = f(expected);
    assert!(
        actual.is_nan() && expected.is_nan() || actual.to_bits() == expected.to_bits(),
        "case {case} {field}: {actual:?} != {expected:?}"
    );
}

#[test]
fn unchanged_csharp_layout_corpus() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/layout.json")).unwrap();
    for (n, c) in a(&cases).iter().enumerate() {
        match c["kind"].as_str().unwrap() {
            "adaptive" => {
                let groups: Vec<_> = a(&c["groups"])
                    .iter()
                    .map(|g| {
                        let mut info =
                            RibbonGroupLayoutInfo::new(a(&g["widths"]).iter().map(f).collect());
                        info.reduction_order = i(&g["order"]);
                        info.can_collapse = b(&g["collapse"]);
                        info
                    })
                    .collect();
                let result = RibbonAdaptiveLayout::compute_with_strategy(
                    &groups,
                    f(&c["available"]),
                    f(&c["spacing"]),
                    RibbonReductionStrategy::from_raw(i(&c["strategy"])),
                );
                assert_eq!(
                    result.states.iter().map(|s| s.raw()).collect::<Vec<_>>(),
                    a(&c["states"]).iter().map(i).collect::<Vec<_>>(),
                    "case {n}"
                );
                equal(result.total_width, &c["total"], n, "total");
                assert_eq!(result.fits, b(&c["fits"]), "case {n}");
            }
            "arrange" => {
                let items: Vec<_> = a(&c["items"])
                    .iter()
                    .map(|v| RibbonArrangeItem {
                        width: f(&v["width"]),
                        height: f(&v["height"]),
                        is_full_height: b(&v["full"]),
                        starts_new_column: b(&v["start"]),
                    })
                    .collect();
                let result = RibbonGroupItemsArranger::arrange_with_options(
                    &items,
                    f(&c["height"]),
                    i(&c["rows"]),
                    f(&c["spacing"]),
                    b(&c["distribute"]),
                );
                equal(result.width, &c["width"], n, "width");
                equal(result.height, &c["resultHeight"], n, "height");
                assert_eq!(result.items.len(), a(&c["output"]).len());
                for (r, e) in result.items.iter().zip(a(&c["output"])) {
                    for (key, value) in [
                        ("x", r.x),
                        ("y", r.y),
                        ("width", r.width),
                        ("height", r.height),
                    ] {
                        equal(value, &e[key], n, key);
                    }
                    assert_eq!(r.column, i(&e["column"]), "case {n}");
                    assert_eq!(r.row, i(&e["row"]), "case {n}");
                }
            }
            "simplified" => {
                let groups: Vec<_> = a(&c["groups"])
                    .iter()
                    .map(|g| RibbonSimplifiedGroup {
                        reduction_order: i(&g["order"]),
                        items: a(&g["items"])
                            .iter()
                            .map(|v| RibbonSimplifiedItem {
                                width: f(&v["width"]),
                                visibility: RibbonSimplifiedVisibility::from_raw(i(
                                    &v["visibility"],
                                )),
                            })
                            .collect(),
                    })
                    .collect();
                let result = RibbonSimplifiedLayout::compute_with_spacing(
                    &groups,
                    f(&c["available"]),
                    f(&c["overflow"]),
                    f(&c["spacing"]),
                );
                assert_eq!(
                    result.in_line,
                    a(&c["inline"])
                        .iter()
                        .map(|row| a(row).iter().map(b).collect::<Vec<_>>())
                        .collect::<Vec<_>>(),
                    "case {n}"
                );
                assert_eq!(result.has_overflow, b(&c["hasOverflow"]), "case {n}");
                equal(result.total_width, &c["total"], n, "total");
            }
            "size" => {
                let input = c["inputUtf16"]
                    .as_array()
                    .map(|a| a.iter().map(|x| i(x) as u16).collect::<Vec<_>>());
                let result = RibbonSizeDefinition::try_parse_utf16(input.as_deref());
                if let Ok(decoded) = input.as_deref().map(String::from_utf16).transpose() {
                    assert_eq!(result, RibbonSizeDefinition::try_parse(decoded.as_deref()));
                }
                assert_eq!(result.is_some(), b(&c["ok"]), "case {n}: {input:?}");
                let result = result.unwrap_or_default();
                assert_eq!(
                    [result.large.raw(), result.medium.raw(), result.small.raw()].to_vec(),
                    a(&c["sizes"]).iter().map(i).collect::<Vec<_>>(),
                    "case {n}"
                );
                assert_eq!(
                    result.to_string(),
                    c["formatted"].as_str().unwrap(),
                    "case {n}"
                );
            }
            other => panic!("unknown corpus kind {other}"),
        }
    }
}
