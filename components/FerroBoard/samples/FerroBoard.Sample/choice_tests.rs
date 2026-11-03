use crate::enum_choices::EnumChoice;
use ferroboard_core::settings::KeyboardSettings;
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};

#[test]
fn unchanged_combo_helper_items_unknown_values_selection_and_suppression_match_source() {
    let mut count = 0;
    for line in include_str!("../../tests/fixtures/sample-choices.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let choice = EnumChoice::ALL
            .into_iter()
            .find(|c| c.property() == row["property"].as_str().unwrap())
            .unwrap();
        assert_eq!(choice.label(), row["label"].as_str().unwrap());
        assert_eq!(
            json!(
                choice
                    .items()
                    .iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
            ),
            row["items"]
        );
        let settings = KeyboardSettings::default();
        choice.set_value(&settings, row["initial"].as_i64().unwrap() as i32);
        assert_eq!(
            choice.selected_index(&settings),
            row["selected"].as_i64().unwrap() as i32
        );
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        settings.property_changed().add(Rc::new(move |name| {
            observed.borrow_mut().push(name.to_string())
        }));
        let result = choice.select(
            &settings,
            row["index"].as_i64().unwrap() as i32,
            row["suppressed"].as_bool().unwrap(),
        );
        assert_eq!(
            json!(result.err().map(|_| "ArgumentOutOfRangeException")),
            row["error"],
            "{row}"
        );
        assert_eq!(
            choice.value(&settings),
            row["result"].as_i64().unwrap() as i32,
            "{row}"
        );
        assert_eq!(json!(*events.borrow()), row["events"], "{row}");
        count += 1;
    }
    assert_eq!(count, 1008);
}

#[cfg(target_os = "macos")]
#[test]
fn native_theme_choices_preserve_source_selection_and_notifications_across_cultures() {
    use crate::theme_choices::{ThemeChoice, ThemeChoices, ThemeProperty, ThemeSelectionError};
    use ferroboard_core::text::Utf16Text;
    use ferroboard_core::text::locale_text::LocaleText;
    fn text(v: &Value) -> Option<Utf16Text> {
        v.as_array().map(|a| {
            Utf16Text::from_units(
                a.iter()
                    .map(|u| u.as_u64().unwrap() as u16)
                    .collect::<Vec<_>>(),
            )
        })
    }
    fn units(t: &Option<Utf16Text>) -> Value {
        json!(t.as_ref().map(Utf16Text::units))
    }
    let mut count = 0;
    for line in include_str!("../../tests/fixtures/sample-theme-choices.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let culture = LocaleText::new(row["culture"].as_str().unwrap()).unwrap();
        let input = row["inputThemes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| ThemeChoice {
                name: text(&v["name"]),
                id: text(&v["id"]),
            })
            .collect::<Vec<_>>();
        let choices = ThemeChoices::snapshot(
            input,
            |t| t.name.clone(),
            |t| t.id.clone(),
            |a, b| culture.compare(a, b, false),
        )
        .unwrap();
        let labels = json!(
            choices
                .items()
                .iter()
                .map(|i| units(&i.name))
                .collect::<Vec<_>>()
        );
        let settings = KeyboardSettings::default();
        if row["kind"] == "snapshot" {
            assert_eq!(labels, row["before"], "{row}");
            assert_eq!(labels, row["after"], "{row}");
            choices
                .select(
                    &settings,
                    ThemeProperty::LightTheme,
                    row["selected"].as_i64().unwrap() as i32,
                    false,
                )
                .unwrap();
            assert_eq!(
                units(&Some(Utf16Text::from(settings.light_theme().as_str()))),
                row["result"]
            );
        } else {
            assert_eq!(labels, row["items"], "{row}");
            let property = match row["property"].as_str().unwrap() {
                "LightTheme" => ThemeProperty::LightTheme,
                "DarkTheme" => ThemeProperty::DarkTheme,
                _ => panic!("source property"),
            };
            property.set(
                &settings,
                text(&row["initial"]).map(|t| t.to_utf8().unwrap()),
            );
            assert_eq!(
                choices.selected_index(&settings, property),
                row["selected"].as_i64().unwrap() as i32,
                "{row}"
            );
            let events = Rc::new(RefCell::new(Vec::new()));
            let captured = events.clone();
            let token = settings
                .property_changed()
                .add(Rc::new(move |s| captured.borrow_mut().push(s.to_owned())));
            let result = choices.select(
                &settings,
                property,
                row["index"].as_i64().unwrap() as i32,
                row["suppressed"].as_bool().unwrap(),
            );
            let error = result.err().map(|e| match e {
                ThemeSelectionError::Index(_) => "ArgumentOutOfRangeException",
                ThemeSelectionError::Encoding => panic!("valid UTF-16 fixture"),
            });
            assert_eq!(json!(error), row["error"], "{row}");
            assert_eq!(
                units(&Some(Utf16Text::from(property.get(&settings).as_str()))),
                row["result"],
                "{row}"
            );
            assert_eq!(json!(*events.borrow()), row["events"], "{row}");
            settings.property_changed().remove(token);
        }
        count += 1;
    }
    assert_eq!(count, 1160);
}

#[test]
fn theme_snapshot_propagates_failures_and_keeps_utf16_values_without_loss() {
    use crate::theme_choices::{ThemeChoice, ThemeChoices, ThemeProperty, ThemeSelectionError};
    use ferroboard_core::text::Utf16Text;
    use std::{cell::Cell, cmp::Ordering};
    let ids_read = Cell::new(0);
    let result = ThemeChoices::snapshot(
        ["b", "a"],
        |n| Some((*n).into()),
        |_| {
            ids_read.set(ids_read.get() + 1);
            None
        },
        |_, _| Err("comparison failed"),
    );
    assert_eq!(result.unwrap_err(), "comparison failed");
    assert_eq!(ids_read.get(), 0);
    let malformed = Utf16Text::from_units(vec![0xd800]);
    let choices = ThemeChoices::snapshot(
        [ThemeChoice {
            name: Some(malformed.clone()),
            id: Some(malformed.clone()),
        }],
        |t| t.name.clone(),
        |t| t.id.clone(),
        |_, _| Ok::<_, ()>(Ordering::Equal),
    )
    .unwrap();
    assert_eq!(choices.items()[0].name, Some(malformed));
    let settings = KeyboardSettings::default();
    let initial = settings.light_theme();
    assert!(matches!(
        choices.select(&settings, ThemeProperty::LightTheme, 0, false),
        Err(ThemeSelectionError::Encoding)
    ));
    assert_eq!(settings.light_theme(), initial);
    assert!(
        choices
            .select(&settings, ThemeProperty::LightTheme, 0, true)
            .is_ok()
    );
    assert!(
        choices
            .select(&settings, ThemeProperty::LightTheme, -1, false)
            .is_ok()
    );
}
