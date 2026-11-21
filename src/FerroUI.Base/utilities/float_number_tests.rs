//! Frozen .NET 10 floating-point parsing observations.
use super::{number_format::try_parse_double_float, CultureInfo, NumberFormatInfo};
fn decode(s: &str) -> String {
    String::from_utf8(
        s.as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect(),
    )
    .unwrap()
}
#[test]
fn float_culture_scanner_matches_exact_reference_bits() {
    let mut info = NumberFormatInfo::new();
    let mut name = String::new();
    let mut count = 0;
    let mut errors = vec![];
    for line in include_str!("../test_assets/input_number_reference.tsv").lines() {
        let f: Vec<_> = line.split('\t').collect();
        if f[0] == "F" {
            name = decode(f[1]);
            info = NumberFormatInfo::new()
                .with_positive_sign(decode(f[2]))
                .with_negative_sign(decode(f[3]))
                .with_number_decimal_separator(decode(f[4]))
                .with_nan_symbol(decode(f[5]))
                .with_positive_infinity_symbol(decode(f[6]))
                .with_negative_infinity_symbol(decode(f[7]));
            info.set_number_negative_pattern(f[8].parse().unwrap());
        } else {
            count += 1;
            let text = (f[1] != "-").then(|| decode(f[1]));
            let actual = try_parse_double_float(text.as_deref(), &info);
            let expect = (f[2] == "1").then(|| u64::from_str_radix(f[3], 16).unwrap());
            if actual.map(f64::to_bits) != expect {
                errors.push(format!(
                    "{name}: {text:?}: {:x?} != {expect:x?}",
                    actual.map(f64::to_bits)
                ));
            }
        }
    }
    assert!(count >= 8694);
    assert!(
        errors.is_empty(),
        "{} mismatches:\n{}",
        errors.len(),
        errors.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}
#[test]
fn current_info_uses_the_culture_instance_number_override() {
    struct Restore(CultureInfo);
    impl Drop for Restore {
        fn drop(&mut self) {
            CultureInfo::set_current_culture(self.0.clone());
        }
    }
    let _restore = Restore(CultureInfo::current_culture());
    let culture =
        CultureInfo::invariant_culture().with_number_format(NumberFormatInfo::new().with_number_decimal_separator(","));
    let expected = culture.number_format();
    CultureInfo::set_current_culture(culture);
    assert!(NumberFormatInfo::current_info() == expected);
    assert!(NumberFormatInfo::get_instance(None) == expected);
    assert_eq!(
        try_parse_double_float(Some("1,5"), &NumberFormatInfo::current_info()),
        Some(1.5)
    );
}
