#[path = "../../../tests/support/layout_utf16_contract.rs"]
mod contract;
#[test]
fn unchanged_source_layout_component_author_and_key_text_contract() {
    assert_eq!(
        contract::verify::<contract::Core>(include_str!(
            "../../../tests/fixtures/layout-utf16.jsonl"
        )),
        (623, 3, 0)
    );
}
