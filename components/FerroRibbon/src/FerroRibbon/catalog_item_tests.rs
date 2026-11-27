#[test]
fn original_public_command_source_catalog_state_and_unload_steps() {
    crate::tests::run(|| {
        crate::catalog_item_cases::assert_corpus(
            crate::item_contract_consumer::ContractItem::new,
            crate::controls::RibbonItemHelper::on_item_unloaded,
        )
    });
}
#[test]
fn public_command_source_unlink_and_drop_release_native_subscriptions() {
    crate::tests::run(crate::catalog_item_cases::assert_disposal);
}
#[test]
fn public_command_source_reentrant_assignment_resolve_preserves_application_command() {
    crate::tests::run(crate::catalog_item_cases::assert_reentrant_application_command_is_preserved);
}

#[test]
fn replaced_catalog_command_strong_capture_does_not_form_an_ownership_marker_cycle() {
    crate::tests::run(
        crate::catalog_item_cases::assert_replaced_capturing_catalog_command_is_released,
    );
}

#[test]
fn restored_original_core_command_remains_owned_after_native_adapter_recreation() {
    crate::tests::run(
        crate::catalog_item_cases::assert_core_identity_survives_native_adapter_recreation,
    );
}

#[test]
fn expired_catalog_marker_still_reads_command_after_removal() {
    crate::tests::run(
        crate::catalog_item_cases::assert_retired_weak_marker_preserves_command_getter_read,
    );
}
