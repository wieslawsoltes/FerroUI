use super::*;
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "ios",
    target_os = "android",
    target_os = "emscripten"
))]
#[test]
fn native_locale_sort_and_search_cover_keyboard_label_text() {
    let locale = LocaleText::new("en-US").unwrap();
    for (left, right, insensitive, expected) in [
        ("Alpha", "alpha", true, Ordering::Equal),
        ("a", "b", false, Ordering::Less),
        ("café", "cafe\u{301}", true, Ordering::Equal),
        ("café", "cafe", true, Ordering::Greater),
    ] {
        assert_eq!(
            locale
                .compare(&left.into(), &right.into(), insensitive)
                .unwrap(),
            expected
        );
    }
    for (text, pattern, ignore_case, expected) in [
        ("English (United States)", "UNITED", true, true),
        ("English", "ENGLISH", false, false),
        ("café", "cafe\u{301}", true, true),
        ("café", "cafe", true, false),
        ("😀English👩‍💻", "English", true, true),
        ("A\0B", "\0B", false, true),
        ("", "", true, true),
        ("", "English", true, false),
        ("English", "", true, true),
    ] {
        assert_eq!(
            locale
                .contains(&text.into(), &pattern.into(), ignore_case)
                .unwrap(),
            expected,
            "{text:?} / {pattern:?}"
        );
    }
    assert!(matches!(
        LocaleText::new("en\0US"),
        Err(CollationError::InvalidLocale)
    ));
}
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "ios",
    target_os = "android",
    target_os = "emscripten"
))]
#[test]
fn native_locale_tailoring_changes_turkish_and_swedish_labels() {
    let tr = LocaleText::new("tr-TR").unwrap();
    assert_eq!(
        tr.compare(&"I".into(), &"ı".into(), true).unwrap(),
        Ordering::Equal
    );
    assert_ne!(
        tr.compare(&"I".into(), &"i".into(), true).unwrap(),
        Ordering::Equal
    );
    assert!(
        tr.contains(&"İstanbul".into(), &"istanbul".into(), true)
            .unwrap()
    );
    let swedish = LocaleText::new("sv-SE").unwrap();
    assert_eq!(
        swedish.compare(&"z".into(), &"å".into(), true).unwrap(),
        Ordering::Less
    );
}
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "emscripten"))]
#[test]
fn native_icu_borrows_utf16_units_including_unpaired_surrogates() {
    let locale = LocaleText::new("").unwrap();
    let text = Utf16Text::from_units(vec![0xd800, b'A' as u16, 0xdc00]);
    let needle = Utf16Text::from_units(vec![0xd800]);
    assert_eq!(
        locale.compare(&text, &text, false).unwrap(),
        Ordering::Equal
    );
    assert!(locale.contains(&text, &needle, false).unwrap());
    assert!(locale.contains(&text, &"a".into(), true).unwrap());
    // Repeated searches have separate RAII handles and cannot retain borrowed caller buffers.
    for _ in 0..20 {
        assert!(
            locale
                .contains(&"Danish".into(), &"dan".into(), true)
                .unwrap()
        );
        assert!(
            !locale
                .contains(&"Swedish".into(), &"dan".into(), true)
                .unwrap()
        );
    }
}
#[cfg(any(target_os = "macos", target_os = "emscripten"))]
#[test]
fn standard_icu_migration_records_the_source_width_differences() {
    use std::collections::BTreeMap;
    let decode = |text: &str| {
        Utf16Text::from_units(if text.is_empty() {
            vec![]
        } else {
            text.split(',')
                .map(|u| u16::from_str_radix(u, 16).unwrap())
                .collect()
        })
    };
    let mut locales = BTreeMap::new();
    let (mut same, mut changed) = (0, 0);
    for line in include_str!("../../../../tests/fixtures/culture-comparison.tsv").lines() {
        let row: Vec<_> = line.split('\t').collect();
        let locale = locales
            .entry(row[0])
            .or_insert_with(|| LocaleText::new(row[0]).unwrap());
        let (left, right) = (decode(row[3]), decode(row[4]));
        let ignore = row[2] == "True";
        let actual = if row[1] == "compare" {
            match locale.compare(&left, &right, ignore).unwrap() {
                Ordering::Less => "-1",
                Ordering::Equal => "0",
                Ordering::Greater => "1",
            }
        } else if locale.contains(&left, &right, ignore).unwrap() {
            "True"
        } else {
            "False"
        };
        if actual == row[5] {
            same += 1;
        } else {
            assert!(
                ignore && (left.units().contains(&0xff21) || right.units().contains(&0xff21)),
                "unreviewed difference: {line} => {actual}"
            );
            changed += 1;
        }
    }
    assert_eq!((same, changed), (140, 10));
}
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "ios",
    target_os = "android",
    target_os = "emscripten"
))]
#[test]
fn system_preference_bootstraps_the_native_text_service() {
    let locale = system_locale_name().expect("configured OS locale");
    let service = LocaleText::new(&locale).expect("native service accepts OS locale");
    assert!(service.contains(&"abc".into(), &"B".into(), true).unwrap());
}
