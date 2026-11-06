use super::*;
use serde_json::Value;
#[test]
fn source_shared_geometry_updates_are_visible_after_worker_join_and_clones_stay_independent() {
    use std::sync::Arc;
    for source in include_str!("../../../tests/fixtures/geometry-record.jsonl")
        .lines()
        .map(|v| serde_json::from_str::<Value>(v).unwrap())
        .filter(|v| v["tag"] == "worker")
    {
        let record = Arc::new(KeyboardGeometryOptions::default());
        let copy = (*record).clone();
        let alias = record.clone();
        let name = source["property"].as_str().unwrap().to_string();
        std::thread::spawn(move || match name.as_str() {
            "KeySpacingHorizontal" => alias.set_key_spacing_horizontal(211.75),
            "KeySpacingVertical" => alias.set_key_spacing_vertical(211.75),
            "SplitGap" => alias.set_split_gap(211.75),
            "UnitsPerRow" => alias.set_units_per_row(211.75),
            _ => unreachable!(),
        })
        .join()
        .unwrap();
        let actual = match source["property"].as_str().unwrap() {
            "KeySpacingHorizontal" => record.key_spacing_horizontal(),
            "KeySpacingVertical" => record.key_spacing_vertical(),
            "SplitGap" => record.split_gap(),
            "UnitsPerRow" => record.units_per_row(),
            _ => unreachable!(),
        };
        assert_eq!(format!("{:016x}", actual.to_bits()), source["value"]);
        assert_ne!(*record, copy);
        assert_eq!(copy, KeyboardGeometryOptions::default());
    }
}
