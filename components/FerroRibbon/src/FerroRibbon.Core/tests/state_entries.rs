use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{RibbonString, state::*};
use serde_json::{Value, json};
use std::rc::Rc;
fn string(v: &Value) -> Option<RibbonString> {
    v.as_str().map(Into::into)
}
fn keys<V: Clone>(entries: &Rc<RibbonStateEntries<V>>) -> Value {
    json!(
        entries
            .entries_snapshot()
            .into_iter()
            .map(|(k, _)| k.to_utf8().unwrap())
            .collect::<Vec<_>>()
    )
}
#[test]
fn source_edit_history_keeps_exact_member_order_and_compact_and_pretty_json() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/state-entry-order.json"
    ))
    .unwrap();
    let state = RibbonState::new();
    for row in rows.as_array().unwrap() {
        let op = &row["operation"];
        let key = op["key"].as_str().unwrap_or("");
        let custom = state.customization().unwrap();
        let labels = custom.labels().unwrap();
        let groups = custom.group_order().unwrap();
        match op["op"].as_str().unwrap() {
            "label" => {
                labels.insert(key.into(), string(&op["value"]));
            }
            "removeLabel" => {
                labels.remove(&key.into());
            }
            "clearLabels" => labels.clear(),
            "group" => {
                groups.insert(
                    key.into(),
                    op["value"]
                        .as_array()
                        .map(|a| RibbonItems::from_values(a.iter().map(string).collect())),
                );
            }
            "removeGroup" => {
                groups.remove(&key.into());
            }
            "clearGroups" => groups.clear(),
            "aliasGroup" => {
                groups.insert(
                    key.into(),
                    groups.get(&op["value"].as_str().unwrap().into()).unwrap(),
                );
            }
            "appendGroup" => {
                groups
                    .get(&key.into())
                    .unwrap()
                    .unwrap()
                    .add(string(&op["value"]));
            }
            unknown => panic!("unknown source operation {unknown}"),
        }
        assert_eq!(keys(&labels), row["labels"], "{op}");
        assert_eq!(keys(&groups), row["groups"], "{op}");
        for (indented, field) in [(false, "compact"), (true, "indented")] {
            assert_eq!(
                RibbonStateSerializer::serialize(Some(&state), indented)
                    .unwrap()
                    .to_utf8()
                    .unwrap(),
                row[field].as_str().unwrap(),
                "{op}"
            );
        }
    }
}
#[test]
fn state_member_rc_identity_snapshots_replacement_and_last_owner_release() {
    let state = RibbonState::new();
    let custom = state.customization().unwrap();
    let entries: RibbonStateLabels = custom.labels().unwrap();
    let alias = custom.labels().unwrap();
    let weak = Rc::downgrade(&entries);
    assert!(Rc::ptr_eq(&entries, &alias));
    let different: RibbonStateLabels = RibbonStateEntries::new();
    assert!(entries != different);
    entries.insert("a".into(), Some("one".into()));
    let snapshot = entries.entries_snapshot();
    alias.insert("a".into(), Some("two".into()));
    assert_eq!(entries.get(&"a".into()), Some(Some("two".into())));
    assert_eq!(snapshot[0].1, Some("one".into()));
    custom.set_labels(Some(different));
    drop(state);
    drop(custom);
    drop(entries);
    assert!(weak.upgrade().is_some());
    drop(alias);
    assert!(weak.upgrade().is_none());
}
#[test]
fn group_normalization_keeps_the_map_and_shared_mutable_list_allocations() {
    let state = RibbonState::new();
    let custom = state.customization().unwrap();
    let entries = custom.group_order().unwrap();
    let list = RibbonItems::from_values(vec![None, Some("".into()), Some("one".into())]);
    entries.insert("drop".into(), None);
    entries.insert("home".into(), Some(list.clone()));
    entries.insert("copy".into(), Some(list.clone()));
    state.normalize();
    assert!(Rc::ptr_eq(&entries, &custom.group_order().unwrap()));
    assert!(!entries.contains_key(&"drop".into()));
    assert!(Rc::ptr_eq(
        &list,
        &entries.get(&"copy".into()).unwrap().unwrap()
    ));
    list.add(Some("two".into()));
    assert_eq!(
        entries.get(&"home".into()).unwrap().unwrap().to_vec(),
        vec![Some("one".into()), Some("two".into())]
    );
}
#[test]
fn duplicate_json_members_keep_the_first_position_and_last_value() {
    let state=RibbonStateSerializer::deserialize(Some(&r#"{"customization":{"labels":{"b":"first","a":null,"b":"last"},"groupOrder":{"x":["first"],"x":["last"]}}}"#.into())).unwrap().unwrap();
    let custom = state.customization().unwrap();
    let labels = custom.labels().unwrap();
    assert_eq!(keys(&labels), json!(["b", "a"]));
    assert_eq!(labels.get(&"b".into()), Some(Some("last".into())));
    assert_eq!(labels.get(&"a".into()), Some(None));
    assert_eq!(
        custom
            .group_order()
            .unwrap()
            .get(&"x".into())
            .unwrap()
            .unwrap()
            .get(0),
        Some("last".into())
    );
}
#[test]
fn state_member_keys_keep_utf16_units_including_unpaired_surrogates_and_nul() {
    let entries: RibbonStateLabels = RibbonStateEntries::new();
    let key = RibbonString::from_utf16(vec![0xd800, 0, 0x61, 0xdc00]);
    entries.insert(key.clone(), None);
    assert!(entries.contains_key(&key));
    assert_eq!(entries.get(&key), Some(None));
    assert_eq!(entries.entries_snapshot()[0].0.as_utf16(), key.as_utf16());
    assert_eq!(entries.remove(&key), Some(None));
    assert!(entries.is_empty());
}
