//! Core diagnostics need no UI host, ambient culture service or global number provider.
use ferroboard_core::{keys::*, layouts::*};
use std::rc::Rc;

#[test]
fn bundled_key_diagnostics_work_without_host_services() {
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    let keyboard = LayoutComputer::new(resources)
        .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
        .unwrap();
    keyboard
        .compute(Rc::new(KeyComputeContext::default()))
        .unwrap();
    let key = keyboard.find_key(KeyCode::Delete).unwrap().unwrap();
    let text = key.to_text().unwrap().to_utf8().unwrap();
    assert!(text.starts_with("Key["));
    assert!(text.contains("code=-7"));
}

#[test]
fn negative_key_codes_use_rust_decimal_formatting_and_retain_utf16_labels() {
    let data = TextKeyData::function(i32::MIN, KeyType::Function, None);
    let label = ferroboard_core::text::Utf16Text::from_units(vec![97, 0xd800]);
    data.data().set_label(Some(label.clone()));
    let text = data.to_text().unwrap().unwrap();
    let mut expected: Vec<u16> = "TextKeyData(code=-2147483648, label=\""
        .encode_utf16()
        .collect();
    expected.extend_from_slice(label.units());
    expected.extend("\", type=function)".encode_utf16());
    assert_eq!(text.units(), expected);
}
