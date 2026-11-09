use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    text::Utf16Text,
};
use std::{collections::HashMap, sync::Arc};

#[test]
fn every_code_unit_survives_storage_and_clone_without_transcoding() {
    let units: Arc<[u16]> = (0..=u16::MAX).collect();
    let text = Utf16Text::from_units(units.clone());
    let copy = text.clone();
    assert_eq!(text.units(), &*units);
    assert_eq!(text.units().as_ptr(), units.as_ptr());
    assert!(text.ptr_eq(&copy));
    assert!(text.to_utf8().is_err());
    std::thread::spawn(move || assert_eq!(copy.units(), &*units))
        .join()
        .unwrap();
}

#[test]
fn scalar_text_round_trips_and_lengths_remain_utf16_units() {
    for value in [
        "",
        "ASCII",
        "é\u{301}",
        "a😀b",
        "👩🏽‍💻🇵🇱",
        "אב abc العربية",
        "a\r\nb\rc\nd",
        "\0",
    ] {
        let text = Utf16Text::from(value);
        assert_eq!(text.to_utf8().unwrap(), value);
        assert_eq!(text.len(), value.encode_utf16().count());
        assert_eq!(text.is_empty(), value.is_empty());
    }
}

#[test]
fn malformed_units_are_preserved_and_rejected_at_utf8_boundary() {
    for units in [
        vec![0xd800],
        vec![0xdc00],
        vec![0xdc00, 0xd800],
        vec![0x61, 0xd800, 0x62],
    ] {
        let text = Utf16Text::from_units(units.clone());
        assert_eq!(text.units(), units);
        assert!(text.to_utf8().is_err());
    }
}

#[test]
fn equality_and_hash_use_units_without_interning() {
    let left = Utf16Text::from("same 😀");
    let right = Utf16Text::from("same 😀");
    assert!(!left.ptr_eq(&right));
    let mut values = HashMap::new();
    values.insert(left, 42);
    assert_eq!(values.get(&right), Some(&42));
}

#[test]
fn editor_can_select_and_replace_inside_a_surrogate_pair() {
    let target = TextBufferTarget::new("a😀b".into(), None);
    target.select(2, 1);
    assert_eq!(
        (target.selection_start(), target.selection_length()),
        (2, 1)
    );
    target.replace(2, 1, Some(&Utf16Text::from("x"))).unwrap();
    assert_eq!(target.text().units(), &[0x61, 0xd83d, 0x78, 0x62]);
    assert!(target.text().to_utf8().is_err());
}
