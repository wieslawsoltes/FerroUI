use ferroboard_core::keys::KeyIcon;
use serde_json::Value;
#[test]
fn original_icon_names_discriminants_and_raw_values_are_retained() {
    assert_eq!(std::mem::size_of::<KeyIcon>(), 4);
    assert_eq!(KeyIcon::default(), KeyIcon::None);
    let mut count = 0;
    for line in include_str!("../../../tests/fixtures/key-icon.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        if row["kind"] != "name" {
            continue;
        }
        let raw = row["raw"].as_i64().unwrap() as i32;
        let icon = KeyIcon(raw);
        assert_eq!(icon.0, raw);
        assert_eq!(icon.to_string(), row["name"].as_str().unwrap());
        assert_eq!(format!("{icon:?}"), row["name"].as_str().unwrap());
        count += 1;
    }
    assert_eq!(count, 73);
}
