#[path = "../../../tests/support/popup_mapping_authors_contract.rs"]
mod contract;
#[test]
fn native_custom_authors_retain_identity_utf16_nullable_dispatch_errors_and_replacement() {
    contract::verify();
}
