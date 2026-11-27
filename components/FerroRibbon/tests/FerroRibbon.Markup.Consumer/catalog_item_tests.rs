use crate::item_contract_consumer::ContractItem;
use ferroribbon::controls::RibbonItemHelper;
use ferroui_base::Ref;
#[test]
fn compiled_runtime_public_item_catalog_state_matches_original_and_unlinks_explicitly() {
    crate::tests::run(|| {
        for runtime in [false, true] {
            crate::catalog_item_cases::assert_corpus(
                || {
                    let value = if runtime {
                        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                            include_str!("CatalogItem.xaml"),
                            Some(&crate::CONSUMER_ASSEMBLY),
                        )
                        .unwrap()
                    } else {
                        crate::compiled_xaml::try_load(
                            None,
                            "ferres://ferroribbon_markup_consumer/CatalogItem.xaml",
                        )
                        .unwrap()
                        .unwrap()
                    };
                    let item = value.downcast_ref::<Ref<ContractItem>>().unwrap().clone();
                    assert_eq!(item.command_id().as_deref(), Some("COPY"));
                    assert_eq!(item.is_checked(), Some(false));
                    assert!(item.command_parameter().is_some());
                    item
                },
                |item| RibbonItemHelper::unlink(item),
            );
        }
    });
}
#[test]
fn independent_public_item_native_subscription_disposal_and_reentrant_command_assignment() {
    crate::tests::run(|| {
        crate::catalog_item_cases::assert_disposal();
        crate::catalog_item_cases::assert_reentrant_application_command_is_preserved();
        crate::catalog_item_cases::assert_replaced_capturing_catalog_command_is_released();
        crate::catalog_item_cases::assert_core_identity_survives_native_adapter_recreation();
        crate::catalog_item_cases::assert_retired_weak_marker_preserves_command_getter_read();
    });
}
