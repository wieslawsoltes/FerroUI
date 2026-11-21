use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{state::*, *};
use serde_json::{Value, json};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/state.json")).unwrap()
}
fn read(v: &Value) -> Option<RibbonString> {
    v.as_array().map(|v| {
        RibbonString::from_utf16(
            v.iter()
                .map(|c| c.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn dump(v: Option<RibbonString>) -> Value {
    v.map_or(Value::Null, |v| json!(v.as_utf16()))
}
fn strings(v: &Value) -> Option<RibbonStateStrings> {
    v.as_array()
        .map(|v| RibbonItems::from_values(v.iter().map(read).collect()))
}
fn dump_strings(v: Option<RibbonStateStrings>) -> Value {
    v.map_or(Value::Null, |v| {
        json!(v.iter().map(dump).collect::<Vec<_>>())
    })
}
fn bits(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn group(v: &Value) -> Option<Rc<RibbonCustomGroup>> {
    (!v.is_null()).then(|| {
        let g = RibbonCustomGroup::new();
        g.set_id(read(&v["id"]));
        g.set_label(read(&v["label"]));
        g.set_tab_id(read(&v["tabId"]));
        g.set_item_ids(strings(&v["itemIds"]));
        g
    })
}
fn groups(v: &Value) -> Option<RibbonStateGroups> {
    v.as_array()
        .map(|v| RibbonItems::from_values(v.iter().map(group).collect()))
}
fn custom(v: &Value) -> Option<Rc<RibbonCustomization>> {
    (!v.is_null()).then(|| {
        let c = RibbonCustomization::new();
        c.set_tab_order(strings(&v["tabOrder"]));
        c.set_hidden_tab_ids(strings(&v["hiddenTabIds"]));
        c.set_hidden_group_ids(strings(&v["hiddenGroupIds"]));
        c.set_group_order(v["groupOrder"].as_array().map(|rows| {
            let map = RibbonStateEntries::new();
            for r in rows {
                map.insert(read(&r["key"]).unwrap(), strings(&r["value"]));
            }
            map
        }));
        c.set_labels(v["labels"].as_array().map(|rows| {
            let map = RibbonStateEntries::new();
            for r in rows {
                map.insert(read(&r["key"]).unwrap(), read(&r["value"]));
            }
            map
        }));
        c.set_custom_tabs(v["customTabs"].as_array().map(|rows| {
            RibbonItems::from_values(
                rows.iter()
                    .map(|r| {
                        (!r.is_null()).then(|| {
                            let t = RibbonCustomTab::new();
                            t.set_id(read(&r["id"]));
                            t.set_label(read(&r["label"]));
                            t.set_groups(groups(&r["groups"]));
                            t
                        })
                    })
                    .collect(),
            )
        }));
        c.set_custom_groups(groups(&v["customGroups"]));
        c
    })
}
fn state(v: &Value) -> Option<Rc<RibbonState>> {
    (!v.is_null()).then(|| {
        let s = RibbonState::new();
        s.set_schema_version(v["schemaVersion"].as_i64().unwrap() as i32);
        s.set_selected_tab_id(read(&v["selectedTabId"]));
        s.set_display_mode(RibbonDisplayMode::from_raw(
            v["displayMode"].as_i64().unwrap() as i32,
        ));
        s.set_visibility_mode(RibbonVisibilityMode::from_raw(
            v["visibilityMode"].as_i64().unwrap() as i32,
        ));
        s.set_density(RibbonDensity::from_raw(
            v["density"].as_i64().unwrap() as i32
        ));
        s.set_quick_access_position(RibbonQuickAccessPosition::from_raw(
            v["quickAccessPosition"].as_i64().unwrap() as i32,
        ));
        s.set_minimize_behavior(RibbonMinimizeBehavior::from_raw(
            v["minimizeBehavior"].as_i64().unwrap() as i32,
        ));
        s.set_is_quick_access_visible(v["IsQuickAccessVisible"].as_bool().unwrap());
        s.set_show_quick_access_labels(v["ShowQuickAccessLabels"].as_bool().unwrap());
        s.set_show_group_captions(v["ShowGroupCaptions"].as_bool().unwrap());
        s.set_quick_access_item_ids(strings(&v["quickAccessItemIds"]));
        s.set_recent_search_ids(strings(&v["recentSearchIds"]));
        s.set_recent_colors(strings(&v["recentColors"]));
        s.set_customization(custom(&v["customization"]));
        s.set_floating_groups(v["floatingGroups"].as_array().map(|rows| {
            RibbonItems::from_values(
                rows.iter()
                    .map(|v| {
                        (!v.is_null()).then(|| {
                            let g = RibbonFloatingGroupState::new();
                            g.set_group_id(read(&v["groupId"]));
                            g.set_x(bits(&v["x"]));
                            g.set_y(bits(&v["y"]));
                            g
                        })
                    })
                    .collect(),
            )
        }));
        s
    })
}
fn dump_group(g: Option<Rc<RibbonCustomGroup>>) -> Value {
    g.map_or(Value::Null,|g|json!({"id":dump(g.id()),"label":dump(g.label()),"tabId":dump(g.tab_id()),"itemIds":dump_strings(g.item_ids())}))
}
fn dump_groups(g: Option<RibbonStateGroups>) -> Value {
    g.map_or(Value::Null, |g| {
        json!(g.iter().map(dump_group).collect::<Vec<_>>())
    })
}
fn dump_custom(c: Option<Rc<RibbonCustomization>>) -> Value {
    c.map_or(Value::Null,|c|json!({"tabOrder":dump_strings(c.tab_order()),"hiddenTabIds":dump_strings(c.hidden_tab_ids()),"hiddenGroupIds":dump_strings(c.hidden_group_ids()),"groupOrder":c.group_order().map(|d|d.entries_snapshot().into_iter().map(|(k,v)|json!({"key":dump(Some(k)),"value":dump_strings(v)})).collect::<Vec<_>>()),"labels":c.labels().map(|d|d.entries_snapshot().into_iter().map(|(k,v)|json!({"key":dump(Some(k)),"value":dump(v)})).collect::<Vec<_>>()),"customTabs":c.custom_tabs().map(|v|v.iter().map(|t|t.map_or(Value::Null,|t|json!({"id":dump(t.id()),"label":dump(t.label()),"groups":dump_groups(t.groups())}))).collect::<Vec<_>>()),"customGroups":dump_groups(c.custom_groups())}))
}
fn snapshot(s: Option<&RibbonState>) -> Value {
    s.map_or(Value::Null,|s|json!({"schemaVersion":s.schema_version(),"selectedTabId":dump(s.selected_tab_id()),"displayMode":s.display_mode().raw(),"visibilityMode":s.visibility_mode().raw(),"density":s.density().raw(),"quickAccessPosition":s.quick_access_position().raw(),"IsQuickAccessVisible":s.is_quick_access_visible(),"ShowQuickAccessLabels":s.show_quick_access_labels(),"minimizeBehavior":s.minimize_behavior().raw(),"ShowGroupCaptions":s.show_group_captions(),"quickAccessItemIds":dump_strings(s.quick_access_item_ids()),"recentSearchIds":dump_strings(s.recent_search_ids()),"recentColors":dump_strings(s.recent_colors()),"customization":dump_custom(s.customization()),"floatingGroups":s.floating_groups().map(|v|v.iter().map(|g|g.map_or(Value::Null,|g|json!({"groupId":dump(g.group_id()),"x":format!("{:016X}",g.x().to_bits()),"y":format!("{:016X}",g.y().to_bits())}))).collect::<Vec<_>>())}))
}
#[test]
fn original_state_serialization_matches_property_order_escaping_enums_numbers_nulls_and_indentation()
 {
    for row in corpus()["serialization"].as_array().unwrap() {
        let state = state(&row["state"]);
        let before = snapshot(state.as_deref());
        let result = RibbonStateSerializer::serialize(state.as_deref(), false);
        assert_eq!(result.is_err(), !row["error"].is_null(), "{}", row["name"]);
        match result {
            Err(e) => {
                assert_eq!(e.reference_type_name(), row["error"]);
                assert_eq!(json!(e.parameter), row["param"]);
            }
            Ok(json) => {
                assert_eq!(json.to_utf8().unwrap(), row["compact"], "{}", row["name"]);
                assert_eq!(
                    RibbonStateSerializer::serialize_default(state.as_deref())
                        .unwrap()
                        .to_utf8()
                        .unwrap(),
                    row["indented"],
                    "{}",
                    row["name"]
                );
            }
        }
        assert_eq!(
            snapshot(state.as_deref()),
            before,
            "Serialize must not normalize {}",
            row["name"]
        );
    }
}
#[test]
fn typed_json_reading_migration_duplicates_unknowns_depth_surrogates_and_enums_match_source() {
    for (i, row) in corpus()["json"].as_array().unwrap().iter().enumerate() {
        let input = read(&row["input"]);
        let result = RibbonStateSerializer::deserialize(input.as_ref());
        assert_eq!(
            result.is_err(),
            !row["deserializeError"].is_null(),
            "row {i}: {:?}",
            input.as_ref().map(RibbonString::to_utf8)
        );
        if let Err(e) = result {
            assert_eq!(e.reference_type_name(), row["deserializeError"]);
            continue;
        }
        let state = result.unwrap();
        assert_eq!(
            snapshot(state.as_deref()),
            row["state"],
            "row {i}: {:?}",
            input.as_ref().map(RibbonString::to_utf8)
        );
        if let Some(state) = state {
            let serialized = RibbonStateSerializer::serialize(Some(&state), false);
            assert_eq!(serialized.is_err(), !row["error"].is_null(), "row {i}");
            if let Ok(s) = serialized {
                assert_eq!(s.to_utf8().unwrap(), row["compact"], "row {i}");
                assert_eq!(
                    RibbonStateSerializer::serialize_default(Some(&state))
                        .unwrap()
                        .to_utf8()
                        .unwrap(),
                    row["indented"],
                    "row {i}"
                );
            }
        }
    }
}
#[test]
fn normalization_preserves_aliases_shared_nodes_whitespace_duplicates_and_no_change_enumerators() {
    let data = corpus();
    let expected = &data["normalize"];
    let state = state(&expected["before"]).unwrap();
    let custom = state.customization().unwrap();
    let strings = state.quick_access_item_ids().unwrap();
    let groups = custom.custom_groups().unwrap();
    let group = groups.get(2).unwrap();
    let tabs = custom.custom_tabs().unwrap();
    let tab = tabs.get(2).unwrap();
    // Restore source cross-property aliases; serialized snapshots alone cannot encode them.
    state.set_recent_search_ids(Some(strings.clone()));
    state.set_recent_colors(Some(strings.clone()));
    custom.set_tab_order(Some(strings.clone()));
    custom.set_hidden_tab_ids(Some(strings.clone()));
    group.set_item_ids(Some(strings.clone()));
    tab.groups().unwrap().set(2, Some(group.clone()));
    let labels = custom.labels().unwrap();
    let order = custom
        .group_order()
        .unwrap()
        .get(&"tab".into())
        .unwrap()
        .unwrap();
    let floating = state.floating_groups().unwrap();
    let mut active = strings.iter();
    active.next();
    state.normalize();
    assert!(catch_unwind(AssertUnwindSafe(|| active.next())).is_err());
    assert_eq!(expected["error"], "InvalidOperationException");
    assert_eq!(snapshot(Some(&state)), expected["first"]);
    let mut stable = strings.iter();
    stable.next();
    state.normalize();
    assert_eq!(json!(stable.next().is_some()), expected["resumed"]);
    assert_eq!(snapshot(Some(&state)), expected["second"]);
    assert!(Rc::ptr_eq(&state.customization().unwrap(), &custom));
    assert!(Rc::ptr_eq(
        &strings,
        &state.quick_access_item_ids().unwrap()
    ));
    assert!(Rc::ptr_eq(&strings, &state.recent_search_ids().unwrap()));
    assert!(Rc::ptr_eq(&strings, &state.recent_colors().unwrap()));
    assert!(Rc::ptr_eq(&strings, &custom.hidden_tab_ids().unwrap()));
    assert!(Rc::ptr_eq(&strings, &group.item_ids().unwrap()));
    assert!(Rc::ptr_eq(&labels, &custom.labels().unwrap()));
    assert!(Rc::ptr_eq(
        &order,
        &custom
            .group_order()
            .unwrap()
            .get(&"tab".into())
            .unwrap()
            .unwrap()
    ));
    assert!(Rc::ptr_eq(&group, &tab.groups().unwrap().get(0).unwrap()));
    assert!(Rc::ptr_eq(
        &group,
        &custom.custom_groups().unwrap().get(0).unwrap()
    ));
    assert!(Rc::ptr_eq(&floating, &state.floating_groups().unwrap()));
    let empty = RibbonCustomization::new();
    empty.tab_order().unwrap().add(Some("x".into()));
    empty.set_group_order(None);
    assert_eq!(json!(empty.is_empty()), expected["early"]);
    empty.tab_order().unwrap().clear();
    assert!(catch_unwind(AssertUnwindSafe(|| empty.is_empty())).is_err());
    assert_eq!(expected["emptyError"], "NullReferenceException");
}
#[test]
fn defaults_keep_original_class_and_collection_identity_without_value_equality() {
    let data = corpus();
    let state = RibbonState::new();
    assert_eq!(snapshot(Some(&state)), data["serialization"][1]["state"]);
    assert_eq!(RibbonState::CURRENT_SCHEMA_VERSION, 1);
    assert!(state.customization().unwrap().is_empty());
    let other = RibbonState::new();
    assert!(!Rc::ptr_eq(&state, &other));
    assert!(state != other);
    assert!(!Rc::ptr_eq(
        &state.customization().unwrap().tab_order().unwrap(),
        &other.customization().unwrap().tab_order().unwrap()
    ));
}
#[test]
fn asynchronous_files_boms_cancellation_faults_missing_and_move_failures_match_source() {
    use ferroui_system::threading::CancellationToken;
    use futures_executor::block_on;
    use std::path::Path;
    let root = std::env::temp_dir().join(format!("state-rust-tests-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for row in corpus()["files"].as_array().unwrap() {
        let mode = row["mode"].as_str().unwrap();
        let directory = root.join(mode);
        if directory.exists() {
            std::fs::remove_dir_all(&directory).unwrap();
        }
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("state.json");
        let input = Some(path.as_path());
        let cancel = CancellationToken::new();
        if mode.starts_with("canceled") {
            cancel.cancel();
        }
        let mut loaded = None;
        let mut error = None;
        let mut status = None;
        match mode {
            "normal" => {
                let state = RibbonState::new();
                state.set_selected_tab_id(Some("view".into()));
                block_on(RibbonStateSerializer::save_async_default(
                    Some(&state),
                    input,
                ))
                .unwrap();
                loaded = block_on(RibbonStateSerializer::load_async_default(input)).unwrap();
            }
            "missing" | "canceled-missing" | "canceled-load" => {
                if mode == "canceled-load" {
                    std::fs::write(&path, "{}").unwrap();
                }
                let result = block_on(RibbonStateSerializer::load_async(input, cancel));
                status = Some(reference_status(&result).to_owned());
                match result {
                    Ok(v) => loaded = v,
                    Err(e) => {
                        error = Some(e.reference_type_name().to_owned());
                    }
                }
            }
            "directory" => {
                std::fs::create_dir(&path).unwrap();
                loaded = block_on(RibbonStateSerializer::load_async_default(input)).unwrap();
            }
            "unreadable" => {
                std::fs::write(&path, "{}").unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
                        .unwrap();
                    let result = block_on(RibbonStateSerializer::load_async_default(input));
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                        .unwrap();
                    loaded = result.unwrap();
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::OpenOptionsExt;
                    let _exclusive_read = std::fs::File::options()
                        .read(true)
                        .share_mode(0)
                        .open(&path)
                        .unwrap();
                    loaded = block_on(RibbonStateSerializer::load_async_default(input)).unwrap();
                }
                #[cfg(not(any(unix, windows)))]
                panic!("Source permission fixture needs a platform-specific equivalent");
            }
            mode if mode.starts_with("utf") => {
                let bytes = if mode == "utf8-invalid" {
                    vec![
                        123, 34, 115, 101, 108, 101, 99, 116, 101, 100, 84, 97, 98, 73, 100, 34,
                        58, 34, 0xff, 34, 125,
                    ]
                } else {
                    let text = "{\"selectedTabId\":\"é😀\"}";
                    match mode {
                        "utf8-bom" => [vec![0xef, 0xbb, 0xbf], text.as_bytes().to_vec()].concat(),
                        "utf16-le" => [
                            vec![0xff, 0xfe],
                            text.encode_utf16().flat_map(u16::to_le_bytes).collect(),
                        ]
                        .concat(),
                        "utf16-be" => [
                            vec![0xfe, 0xff],
                            text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
                        ]
                        .concat(),
                        "utf16-invalid" => [
                            vec![0xff, 0xfe],
                            "{\"selectedTabId\":\""
                                .encode_utf16()
                                .chain(std::iter::once(0xd800))
                                .chain("\"}".encode_utf16())
                                .flat_map(u16::to_le_bytes)
                                .collect(),
                        ]
                        .concat(),
                        "utf32-invalid" => [
                            vec![0xff, 0xfe, 0, 0],
                            "{\"selectedTabId\":\""
                                .chars()
                                .map(|c| c as u32)
                                .chain(std::iter::once(0xd800))
                                .chain("\"}".chars().map(|c| c as u32))
                                .flat_map(u32::to_le_bytes)
                                .collect(),
                        ]
                        .concat(),
                        "utf32-le" => [
                            vec![0xff, 0xfe, 0, 0],
                            text.chars()
                                .flat_map(|c| (c as u32).to_le_bytes())
                                .collect(),
                        ]
                        .concat(),
                        _ => [
                            vec![0, 0, 0xfe, 0xff],
                            text.chars()
                                .flat_map(|c| (c as u32).to_be_bytes())
                                .collect(),
                        ]
                        .concat(),
                    }
                };
                std::fs::write(&path, bytes).unwrap();
                loaded = block_on(RibbonStateSerializer::load_async_default(input)).unwrap();
            }
            _ => {
                if mode == "move-directory" {
                    std::fs::create_dir(&path).unwrap();
                }
                let state = if mode == "null-state" {
                    None
                } else {
                    Some(RibbonState::new())
                };
                if mode == "nonfinite" {
                    let g = RibbonFloatingGroupState::new();
                    g.set_group_id(Some("g".into()));
                    g.set_x(f64::NAN);
                    state
                        .as_ref()
                        .unwrap()
                        .set_floating_groups(Some(RibbonItems::from_values(vec![Some(g)])));
                }
                let path_arg = if mode == "null-save" {
                    None
                } else if mode == "empty-save" {
                    Some(Path::new(""))
                } else {
                    input
                };
                let result = block_on(RibbonStateSerializer::save_async(
                    state.as_deref(),
                    path_arg,
                    cancel,
                ));
                status = Some(reference_status(&result).to_owned());
                if let Err(e) = result {
                    error = Some(e.reference_type_name().to_owned());
                }
            }
        }
        assert_eq!(snapshot(loaded.as_deref()), row["loaded"], "{mode}");
        assert_eq!(json!(error), row["error"], "{mode}");
        assert_eq!(json!(status), row["status"], "{mode}");
        assert_eq!(json!(path.is_file()), row["file"], "{mode}");
        assert_eq!(
            json!(path.with_file_name("state.json.tmp").is_file()),
            row["temp"],
            "{mode}"
        );
        assert_eq!(json!(path.is_dir()), row["directory"], "{mode}");
    }
}

// Development-only mapping to unchanged C# fixture vocabulary.
// The shipped Result errors expose Rust kinds, Display and std::error::Error.
trait ReferenceErrorName {
    fn reference_type_name(&self) -> &'static str;
}
impl ReferenceErrorName for RibbonStateError {
    fn reference_type_name(&self) -> &'static str {
        match self.kind {
            RibbonStateErrorKind::ArgumentNull => "ArgumentNullException",
            RibbonStateErrorKind::Argument => "ArgumentException",
            RibbonStateErrorKind::Io => "IOException",
            RibbonStateErrorKind::Unauthorized => "UnauthorizedAccessException",
            RibbonStateErrorKind::Canceled => "TaskCanceledException",
        }
    }
}

// Reference-only vocabulary; no task status object exists in the shipped API.
fn reference_status<T>(result: &Result<T, RibbonStateError>) -> &'static str {
    match result {
        Ok(_) => "RanToCompletion",
        Err(error) if error.kind == RibbonStateErrorKind::Canceled => "Canceled",
        Err(_) => "Faulted",
    }
}
