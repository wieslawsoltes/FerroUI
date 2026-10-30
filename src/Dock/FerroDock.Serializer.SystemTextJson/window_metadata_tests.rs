// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::core::*;
use ferrodock_model_reactiveui::core::DockWindow;
use std::{cell::RefCell, rc::Rc};
fn serializer() -> Rc<DockSerializer> {
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(
        core_model_type_metadata(),
    )))
    .unwrap()
}
fn check(name: &str, value: bool) {
    assert!(value, "{name}");
    assert!(
        include_str!("../../../tests/Dock/data/serializer-json-audit.window.tsv")
            .lines()
            .any(|row| row == format!("{name}\t1")),
        "missing source case {name}"
    );
}
#[derive(Default)]
struct Pair {
    a: RefCell<Option<Rc<DockWindowOptions>>>,
    b: RefCell<Option<Rc<DockWindowOptions>>>,
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
}
#[test]
fn full_original_window_options_metadata_preserves_shared_parent_and_apply_behavior() {
    let serializer = serializer();
    let wire = include_str!("../../../tests/Dock/data/serializer-json-audit.window.options.json");
    let options = serializer
        .deserialize::<Rc<DockWindowOptions>>(Some(wire))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(options.clone()).unwrap(), wire);
    check(
        "window-options-undefined-owner-value",
        options.owner_mode().to_raw() == -19,
    );
    check(
        "window-options-original-constructor-and-flags",
        options.parent_window().is_none()
            && options.is_modal()
            && options.show_in_taskbar() == Some(false),
    );
    let target = DockWindow::new();
    options.apply_to(Some(target.as_ref()));
    check(
        "window-options-apply-original-unknown-owner",
        target.owner_mode().to_raw() == -19
            && target.is_modal()
            && target.show_in_taskbar() == Some(false),
    );
    let defaults = serializer
        .deserialize::<Rc<DockWindowOptions>>(Some("{}"))
        .unwrap()
        .unwrap();
    check(
        "window-options-source-default-values",
        defaults.owner_mode() == DockWindowOwnerMode::Default
            && defaults.parent_window().is_none()
            && !defaults.is_modal()
            && defaults.show_in_taskbar().is_none(),
    );
    check(
        "window-enum-unknown-values-have-numeric-names",
        DockWindowState::from_raw(-19).to_string() == "-19"
            && DockWindowOwnerMode::from_raw(123).to_string() == "123",
    );
    let pair = JsonTypeInfo::object(
        || Rc::new(Pair::default()),
        |v: &Rc<Pair>| Rc::as_ptr(v) as usize,
        vec![
            JsonPropertyInfo::new(
                "A",
                |v: &Rc<Pair>| v.a.borrow().clone(),
                |v, x| {
                    v.a.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "B",
                |v: &Rc<Pair>| v.b.borrow().clone(),
                |v, x| {
                    v.b.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "Window",
                |v: &Rc<Pair>| v.window.borrow().clone(),
                |v, x| {
                    v.window.replace(x);
                },
            ),
        ],
    );
    let base = super::model_metadata_tests::resolver();
    struct Resolver {
        pair: Rc<JsonTypeInfo>,
        base: Rc<GeneratedJsonTypeInfoResolver>,
    }
    impl IJsonTypeInfoResolver for Resolver {
        fn get_type_info(
            &self,
            id: std::any::TypeId,
            options: &JsonSerializerOptions,
        ) -> Option<Rc<JsonTypeInfo>> {
            if id == std::any::TypeId::of::<Rc<Pair>>() {
                Some(self.pair.clone())
            } else {
                self.base.get_type_info(id, options)
            }
        }
    }
    let serializer = DockSerializer::with_resolver(Some(Rc::new(Resolver { pair, base }))).unwrap();
    let wire = include_str!("../../../tests/Dock/data/serializer-json-audit.window.pair.json");
    let pair = serializer
        .deserialize::<Rc<Pair>>(Some(wire))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(pair.clone()).unwrap(), wire);
    let a = pair.a.borrow().clone().unwrap();
    let b = pair.b.borrow().clone().unwrap();
    let parent = pair.window.borrow().clone().unwrap();
    check(
        "window-options-shared-options-and-parent",
        Rc::ptr_eq(&a, &b) && Rc::ptr_eq(&a.parent_window().unwrap(), &parent),
    );
    a.apply_to(Some(target.as_ref()));
    check(
        "window-options-apply-retains-original-parent",
        Rc::ptr_eq(&target.parent_window().unwrap(), &parent)
            && target.owner_mode().to_raw() == -19,
    );
}
#[test]
fn window_enum_numeric_wire_matches_all_original_signed_values() {
    let serializer = serializer();
    let mut count = 0;
    for row in include_str!("../../../tests/Dock/data/serializer-json-audit.window.values.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        let raw = fields[1].parse::<i32>().unwrap();
        macro_rules! verify {
            ($type:ty) => {{
                let value = <$type>::from_raw(raw);
                assert_eq!(serializer.serialize(value).unwrap(), fields[3]);
                assert_eq!(
                    serializer
                        .deserialize::<$type>(Some(fields[3]))
                        .unwrap()
                        .unwrap(),
                    value
                );
            }};
        }
        match fields[0] {
            "OWNER" => verify!(DockWindowOwnerMode),
            "STATE" => verify!(DockWindowState),
            _ => panic!("source enum kind"),
        }
        count += 1;
    }
    assert_eq!(count, 282);
}
#[test]
fn original_window_enum_and_options_read_corpus_matches() {
    let serializer = serializer();
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/Dock/data/serializer-json-audit.window.reads.json"
    ))
    .unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 50);
    for row in rows {
        let kind = row[0].as_str().unwrap();
        let input = row[1].as_str().unwrap();
        macro_rules! replay {
            ($t:ty) => {{
                let result = serializer.deserialize::<$t>(Some(input));
                if row[2] == "ERR" {
                    assert!(result.is_err(), "{kind} {input}");
                } else {
                    let actual = match result.unwrap_or_else(|e| panic!("{kind} {input}: {e}")) {
                        Some(value) => serializer.serialize(value).unwrap(),
                        None => "null".into(),
                    };
                    assert_eq!(actual, row[3].as_str().unwrap(), "{kind} {input}");
                }
            }};
        }
        match kind {
            "OWNER" => replay!(DockWindowOwnerMode),
            "STATE" => replay!(DockWindowState),
            "OPTIONS" => replay!(Rc<DockWindowOptions>),
            "OPTION_OWNER" => replay!(Option<DockWindowOwnerMode>),
            "OPTION_STATE" => replay!(Option<DockWindowState>),
            _ => panic!("source window kind"),
        }
    }
}
