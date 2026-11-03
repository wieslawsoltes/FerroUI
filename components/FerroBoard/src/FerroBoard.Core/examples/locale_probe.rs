//! Native locale/resource/engine probe. Runs unchanged on desktop and Emscripten;
//! requires no controls, network, .NET runtime, clipboard or user-data access.
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::KeyboardEngine,
    layouts::{KeyboardResources, LayoutType},
    text::{
        Utf16Text,
        locale_text::{CollationError, LocaleText, system_locale_name},
    },
};
use serde_json::json;
use std::{cmp::Ordering, collections::BTreeMap, error::Error, rc::Rc};

fn decode(value: &str) -> Utf16Text {
    Utf16Text::from_units(if value.is_empty() {
        vec![]
    } else {
        value
            .split(',')
            .map(|unit| u16::from_str_radix(unit, 16).unwrap())
            .collect()
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let locale_name = system_locale_name()?;
    let locale = LocaleText::new("en-US")?;
    for (left, right, ignore_case, expected) in [
        ("Alpha", "alpha", true, Ordering::Equal),
        ("a", "b", false, Ordering::Less),
        ("café", "cafe\u{301}", true, Ordering::Equal),
        ("café", "cafe", true, Ordering::Greater),
    ] {
        assert_eq!(
            locale.compare(&left.into(), &right.into(), ignore_case)?,
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
            locale.contains(&text.into(), &pattern.into(), ignore_case)?,
            expected
        );
    }
    assert!(matches!(
        LocaleText::new("en\0US"),
        Err(CollationError::InvalidLocale)
    ));
    let turkish = LocaleText::new("tr-TR")?;
    assert_eq!(
        turkish.compare(&"I".into(), &"ı".into(), true)?,
        Ordering::Equal
    );
    assert_ne!(
        turkish.compare(&"I".into(), &"i".into(), true)?,
        Ordering::Equal
    );
    assert!(turkish.contains(&"İstanbul".into(), &"istanbul".into(), true)?);
    assert_eq!(
        LocaleText::new("sv-SE")?.compare(&"z".into(), &"å".into(), true)?,
        Ordering::Less
    );
    let unpaired = Utf16Text::from_units(vec![0xd800, 0x41, 0xdc00]);
    assert_eq!(
        locale.compare(&unpaired, &unpaired, false)?,
        Ordering::Equal
    );
    assert!(locale.contains(&unpaired, &Utf16Text::from_units(vec![0xd800]), false)?);
    for _ in 0..20_000 {
        assert!(locale.contains(&"Danish".into(), &"dan".into(), true)?);
    }

    // Freeze actual native differences against the unchanged .NET corpus. Never
    // replace expected source rows with platform results or tolerate ranking changes.
    let mut locales = BTreeMap::new();
    let mut differences = Vec::new();
    let mut matching = 0;
    for line in include_str!("../../../tests/fixtures/culture-comparison.tsv").lines() {
        let row: Vec<_> = line.split('\t').collect();
        if !locales.contains_key(row[0]) {
            locales.insert(row[0], LocaleText::new(row[0])?);
        }
        let service = &locales[row[0]];
        let (left, right) = (decode(row[3]), decode(row[4]));
        let actual = if row[1] == "compare" {
            match service.compare(&left, &right, row[2] == "True")? {
                Ordering::Less => "-1",
                Ordering::Equal => "0",
                Ordering::Greater => "1",
            }
        } else if service.contains(&left, &right, row[2] == "True")? {
            "True"
        } else {
            "False"
        };
        if actual == row[5] {
            matching += 1;
        } else {
            differences.push(json!({"source_row":line,"native_result":actual}));
        }
    }
    assert_eq!(matching + differences.len(), 150);
    let resources = KeyboardResources::create_default()?;
    let metadata = resources.get_layout_metadata(LayoutType::Characters)?;
    assert_eq!(metadata.len(), 77);
    let engine = KeyboardEngine::with_defaults()?;
    let target = Rc::new(TextBufferTarget::default());
    engine.attach(target.clone())?;
    engine.input_text(&"FerroBoard 😀 e\u{301}".into())?;
    assert_eq!(target.text().to_utf8().unwrap(), "FerroBoard 😀 e\u{301}");
    engine.dispose()?;
    println!(
        "{}",
        json!({"status":"passed","platform":std::env::consts::OS,
        "preferred_locale":locale_name,"basic_cases":13,"search_repetitions":20_000,
        "character_layout_metadata":metadata.len(),"default_engine_typing":true,
        "source_comparison_rows":150,"matching_source_rows":matching,"differences":differences})
    );
    Ok(())
}
