use crate::languages::{LanguageCatalog, LanguagePanelViewModel};
#[cfg(target_os = "macos")]
use ferroboard_core::text::locale_text::LocaleText;
use ferroboard_core::{layouts::*, settings::KeyboardSettings, text::Utf16Text};
#[cfg(target_os = "macos")]
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
fn text(v: &Value) -> Option<Utf16Text> {
    v.as_array().map(|a| {
        Utf16Text::from_units(
            a.iter()
                .map(|n| n.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
#[cfg(target_os = "macos")]
fn settings_list(v: &Value) -> ferroboard_core::settings::SubtypeList {
    Rc::new(RefCell::new(
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| text(v).map(|t| t.to_utf8().unwrap()))
            .collect(),
    ))
}
#[cfg(target_os = "macos")]
fn list(settings: &KeyboardSettings) -> Value {
    json!(
        settings
            .subtypes()
            .borrow()
            .iter()
            .map(|s| s.as_deref().map(|s| s.encode_utf16().collect::<Vec<_>>()))
            .collect::<Vec<_>>()
    )
}
#[cfg(target_os = "macos")]
fn rows(panel: &LanguagePanelViewModel) -> Value {
    json!(
        panel
            .rows()
            .iter()
            .map(|r| json!({"name":r.name().units(),"value":r.is_checked()}))
            .collect::<Vec<_>>()
    )
}
fn custom_resources(catalog: &Value) -> Rc<KeyboardResources> {
    let resources = Rc::new(KeyboardResources::create_empty());
    for input in catalog.as_array().unwrap() {
        let subtype = Subtype::new(text(&input["language"]).unwrap());
        subtype.set_display_name(text(&input["display"]));
        let component = ComponentName::new(
            text(&input["extension"]).unwrap(),
            text(&input["component"]).unwrap(),
        );
        subtype.set_layouts(Some(Arc::new(SubtypeLayoutMap::from_values(
            ferroboard_core::layouts::SubtypeLayoutMapValues {
                characters: component.clone(),
                ..SubtypeLayoutMap::default().values()
            },
        ))));
        resources.add_subtype_preset(Arc::new(subtype));
        if !input["layout"].is_null() {
            let layout = KeyboardLayout::new(
                LayoutType::Characters,
                component.extension_id.unwrap(),
                component.component_id.unwrap(),
                Rc::new(LayoutArrangement::from_rows(vec![])),
            );
            layout.set_label(Some(text(&input["layout"]["label"]).unwrap()));
            resources.add_layout(Rc::new(layout));
        }
    }
    resources
}
#[cfg(target_os = "macos")]
#[test]
fn native_language_build_filter_callbacks_snapshots_and_list_identity_match_reference() {
    use std::collections::BTreeMap;
    let mut settings = Rc::new(KeyboardSettings::default());
    let mut panel = None;
    let mut resources = Rc::new(KeyboardResources::create_empty());
    let active_culture = Rc::new(RefCell::new(String::new()));
    let mut previous = String::new();
    let mut count = 0;
    let mut malformed = 0;
    for line in include_str!("../../tests/fixtures/sample-languages.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let kind = row["kind"].as_str().unwrap();
        match kind {
            "build" => {
                settings = Rc::new(KeyboardSettings::default());
                resources = if row["scenario"] == "bundled" {
                    Rc::new(KeyboardResources::create_default().unwrap())
                } else {
                    custom_resources(&row["catalog"])
                };
                let name = row["culture"].as_str().unwrap();
                *active_culture.borrow_mut() = name.into();
                let sort = LocaleText::new(name).unwrap();
                let catalog = LanguageCatalog::snapshot(&resources, |a, b| {
                    sort.compare(a, b, true).map_err(|e| e.to_string())
                })
                .unwrap();
                let culture = active_culture.clone();
                let cache = RefCell::new(BTreeMap::<String, Rc<LocaleText>>::new());
                let search = Rc::new(move |a: &Utf16Text, b: &Utf16Text| {
                    let name = culture.borrow().clone();
                    let compare = {
                        let mut cache = cache.borrow_mut();
                        cache
                            .entry(name.clone())
                            .or_insert_with(|| Rc::new(LocaleText::new(&name).unwrap()))
                            .clone()
                    };
                    compare.contains(a, b, true).map_err(|e| e.to_string())
                });
                let current =
                    LanguagePanelViewModel::new(settings.clone(), catalog, search).unwrap();
                if row["scenario"] == "malformed" {
                    assert!(
                        crate::language_bindings::LanguagePanelBindings::new(current.clone())
                            .is_err()
                    );
                    malformed += 1;
                }
                panel = Some(current);
            }
            "filter" => {
                *active_culture.borrow_mut() = row["searchCulture"].as_str().unwrap().into();
                panel
                    .as_ref()
                    .unwrap()
                    .text_changed(text(&row["query"]))
                    .unwrap();
            }
            "callback" => {
                let current = panel.as_ref().unwrap();
                if previous != "callback" {
                    *active_culture.borrow_mut() = row["culture"].as_str().unwrap().into();
                    current.text_changed(Some("".into())).unwrap();
                }
                settings.set_subtypes(Some(settings_list(&row["initial"])));
                assert_eq!(
                    rows(current),
                    row["beforeItems"],
                    "external settings must leave checks stale: {row}"
                );
                let before = settings.subtypes();
                let events = Rc::new(RefCell::new(Vec::new()));
                let captured = events.clone();
                let token = settings
                    .property_changed()
                    .add(Rc::new(move |s| captured.borrow_mut().push(s.to_string())));
                let target = current.rows()[0].clone();
                assert_eq!(json!(target.name().units()), row["target"]);
                target
                    .dispatch_checked(row["check"].as_bool().unwrap())
                    .unwrap();
                assert_eq!(
                    Rc::ptr_eq(&before, &settings.subtypes()),
                    row["sameList"].as_bool().unwrap(),
                    "{row}"
                );
                assert_eq!(json!(*events.borrow()), row["events"], "{row}");
                settings.property_changed().remove(token);
            }
            "old-row" => {
                let current = panel.as_ref().unwrap();
                settings.set_subtypes(Some(Rc::new(RefCell::new(vec![Some(
                    "en-US/qwerty".into(),
                )]))));
                assert_eq!(rows(current), row["stale"]);
                let old = current.rows()[0].clone();
                assert_eq!(json!(old.name().units()), row["target"]);
                current
                    .text_changed(Some("no such language".into()))
                    .unwrap();
                old.dispatch_checked(true).unwrap();
            }
            "late" => {
                let subtype = Subtype::new("en-US".into());
                subtype.set_display_name(Some("Late preset".into()));
                subtype.set_layouts(Some(Arc::new(SubtypeLayoutMap::from_values(
                    ferroboard_core::layouts::SubtypeLayoutMapValues {
                        characters: ComponentName::new("test", "late"),
                        ..SubtypeLayoutMap::default().values()
                    },
                ))));
                resources.add_subtype_preset(Arc::new(subtype));
                panel
                    .as_ref()
                    .unwrap()
                    .text_changed(Some("".into()))
                    .unwrap();
            }
            other => panic!("unknown source operation {other}"),
        }
        assert_eq!(rows(panel.as_ref().unwrap()), row["items"], "{row}");
        assert_eq!(list(&settings), row["settings"], "{row}");
        previous = kind.into();
        count += 1;
    }
    assert_eq!(count, 1206);
    assert_eq!(malformed, 6);
}

#[test]
fn language_populate_exposes_clear_add_order_partial_failure_and_explicit_adapter_cleanup() {
    use crate::languages::LanguageRowsChanged;
    use std::cell::Cell;
    let resources = custom_resources(&json!([
        {"language":[101,110,45,85,83],"display":[97],"extension":[120],"component":[97],"layout":null},
        {"language":[101,110,45,85,83],"display":[98],"extension":[120],"component":[98],"layout":null},
        {"language":[101,110,45,85,83],"display":[99],"extension":[120],"component":[99],"layout":null}
    ]));
    assert!(LanguageCatalog::snapshot(&resources, |_, _| Err("sort failed".into())).is_err());
    let catalog =
        LanguageCatalog::snapshot(&resources, |a, b| Ok(a.units().cmp(b.units()))).unwrap();
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let model = LanguagePanelViewModel::new(
        Rc::new(KeyboardSettings::default()),
        catalog,
        Rc::new(move |_, _| {
            called.set(called.get() + 1);
            if called.get() == 2 {
                Err("search failed".into())
            } else {
                Ok(true)
            }
        }),
    )
    .unwrap();
    let native = crate::language_bindings::LanguagePanelBindings::new(model.clone()).unwrap();
    let source = native.items();
    assert_eq!(model.rows_changed().handler_count(), 1);
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let token = model.rows_changed().add(Rc::new(move |e| {
        observed.borrow_mut().push(match e {
            LanguageRowsChanged::Reset => "reset",
            LanguageRowsChanged::Add(_) => "add",
        })
    }));
    assert_eq!(
        model.text_changed(Some("needle".into())).unwrap_err(),
        "search failed"
    );
    assert_eq!(*events.borrow(), vec!["reset", "add"]);
    assert_eq!(model.rows().len(), 1);
    assert!(source == native.items());
    model.rows_changed().remove(token);
    native.release();
    native.release();
    assert_eq!(model.rows_changed().handler_count(), 0);
    let native = crate::language_bindings::LanguagePanelBindings::new(model.clone()).unwrap();
    assert_eq!(model.rows_changed().handler_count(), 1);
    drop(native);
    assert_eq!(model.rows_changed().handler_count(), 0);
}

#[test]
fn language_catalog_preserves_null_label_fallback_and_empty_label() {
    // SettingsPanel.BuildLanguages uses loaded Label ?? ComponentId, then interpolation.
    for component in [None, Some(""), Some("fallback")] {
        for label in [None, Some(""), Some("label")] {
            let resources = KeyboardResources::create_empty();
            let name =
                ComponentName::new_nullable(Some("custom".into()), component.map(Into::into));
            let layout = KeyboardLayout::new_nullable(
                LayoutType::Characters,
                name.extension_id.clone(),
                name.component_id.clone(),
                Some(LayoutArrangement::empty()),
            );
            layout.set_label(label.map(Into::into));
            resources.add_layout(Rc::new(layout));
            let subtype = Subtype::from_values(ferroboard_core::layouts::SubtypeValues {
                display_name: Some("Test".into()),
                layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                    ferroboard_core::layouts::SubtypeLayoutMapValues {
                        characters: name,
                        ..Default::default()
                    },
                ))),
                ..Subtype::new("en-US".into()).values()
            });
            resources.add_subtype_preset(Arc::new(subtype));
            let catalog =
                LanguageCatalog::snapshot(&resources, |a, b| Ok(a.units().cmp(b.units()))).unwrap();
            assert_eq!(catalog.presets().len(), 1);
            assert_eq!(
                catalog.presets()[0].name,
                format!("Test (en-US) — {}", label.or(component).unwrap_or(""))
                    .as_str()
                    .into()
            );
        }
    }
}
