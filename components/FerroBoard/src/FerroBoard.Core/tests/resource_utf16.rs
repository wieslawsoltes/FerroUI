#[path = "../../../tests/support/resource_text_contract.rs"]
mod contract;
#[test]
fn source_punctuation_resource_and_language_null_utf16_contracts() {
    contract::verify::<contract::Core>(include_str!(
        "../../../tests/fixtures/resource-utf16.jsonl"
    ));
}
