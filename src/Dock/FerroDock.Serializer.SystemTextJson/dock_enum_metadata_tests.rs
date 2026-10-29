// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::core::{
    DockFloatingWindowHostMode, DockMode, IDockSerializer, PinnedDockDisplayMode,
};

#[test]
fn docking_enum_numeric_json_matches_every_original_value() {
    let serializer = DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(
        core_model_type_metadata(),
    )))
    .unwrap();
    let mut count = 0;
    for row in include_str!("../../../tests/Dock/data/serializer-json-audit.dock-enums.values.tsv").lines()
    {
        let fields: Vec<_> = row.split('\t').collect();
        let raw = fields[1].parse::<i32>().unwrap();
        macro_rules! verify {
            ($t:ty) => {{
                let value = <$t>::from_raw(raw);
                assert_eq!(serializer.serialize(value).unwrap(), fields[3]);
                assert_eq!(
                    serializer
                        .deserialize::<$t>(Some(fields[3]))
                        .unwrap()
                        .unwrap(),
                    value
                );
            }};
        }
        match fields[0] {
            "DOCK" => verify!(DockMode),
            "PINNED" => verify!(PinnedDockDisplayMode),
            "HOST" => verify!(DockFloatingWindowHostMode),
            _ => panic!("source enum kind"),
        }
        count += 1;
    }
    assert_eq!(count, 423);
}

#[test]
fn docking_enum_json_token_ranges_and_nullable_values_match_original_serializer() {
    let serializer = DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(
        core_model_type_metadata(),
    )))
    .unwrap();
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/Dock/data/serializer-json-audit.dock-enums.reads.json"
    ))
    .unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 102);
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
            "DOCK" => replay!(DockMode),
            "PINNED" => replay!(PinnedDockDisplayMode),
            "HOST" => replay!(DockFloatingWindowHostMode),
            "OPTION_DOCK" => replay!(Option<DockMode>),
            "OPTION_PINNED" => replay!(Option<PinnedDockDisplayMode>),
            "OPTION_HOST" => replay!(Option<DockFloatingWindowHostMode>),
            _ => panic!("source enum kind"),
        }
    }
}
