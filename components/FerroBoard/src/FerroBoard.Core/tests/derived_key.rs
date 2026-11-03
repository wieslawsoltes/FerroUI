#[path = "../../../tests/support/derived_key_contract.rs"]
mod contract;
#[test]
fn original_derived_key_array_null_mutation_replacement_copy_and_error_contracts() {
    assert_eq!(
        contract::verify::<contract::Core>(include_str!(
            "../../../tests/fixtures/derived-key.jsonl"
        )),
        (431, 9)
    );
}
