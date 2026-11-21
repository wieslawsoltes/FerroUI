//! Shared behavioral assertions, separately compiled in the library and consumer.
//! The actual custom control is application-owned and has only public interfaces.
use crate::item_contract_consumer::ContractItem;
use ferroribbon::{commands::adapt_core_command, controls::*};
use ferroribbon_core::commands::{
    RibbonCommandCatalog, RibbonCommandDescriptor, RibbonRelayCommand,
};
use ferroui_base::{
    Ref,
    data::core::{ValueType, ValueTypes},
};
use ferroui_controls::Control;
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};

fn owner(catalog: Rc<RibbonCommandCatalog>, item: &ContractItem) -> Ref<Ribbon> {
    let root = Ribbon::new();
    root.set_command_catalog(Some(catalog));
    let value = Control::boxed(&root);
    let view = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRibbonItemOwner>>()).unwrap();
    let view = view
        .downcast_ref::<Rc<dyn IRibbonItemOwner>>()
        .unwrap()
        .clone();
    assert_eq!(
        view.as_object(),
        root.clone().upcast::<ferroui_base::FerroObject>()
    );
    RibbonItemHelper::set_owner(item, Some(view)).unwrap();
    root
}

pub fn assert_corpus(make: impl Fn() -> Ref<ContractItem>, detach: impl Fn(&Control)) {
    let fixture: Value = serde_json::from_str(include_str!("catalog-item.json")).unwrap();
    for case in fixture["corpus"]["cases"].as_array().unwrap() {
        let commands: Vec<_> = ["A", "B", "C", "User"]
            .into_iter()
            .map(|name| {
                let core = RibbonRelayCommand::new(|| {});
                (name, core.clone(), adapt_core_command(core))
            })
            .collect();
        let catalog = RibbonCommandCatalog::new();
        let descriptor = RibbonCommandDescriptor::new(
            Some("copy".into()),
            Some("Copy".into()),
            Some(commands[0].1.clone()),
        );
        descriptor.set_is_enabled(false);
        descriptor.set_is_checked(Some(true));
        catalog.register(Some(descriptor.clone()));
        let item = make();
        item.set_command_id(Some("COPY".into()));
        if case["userCommand"].as_bool().unwrap() {
            item.set_command(Some(commands[3].2.clone()));
        }
        let _owner = owner(catalog.clone(), &item);
        for expected in case["steps"].as_array().unwrap() {
            let step = expected["step"].as_str().unwrap();
            match step {
                "resolve" | "null_command_resolve" | "resolve_after_unload" => {
                    RibbonItemHelper::resolve_command(&item).unwrap()
                }
                "replace" => descriptor.set_command(Some(commands[1].1.clone())),
                "protect_user" => {
                    item.set_command(Some(commands[3].2.clone()));
                    descriptor.set_command(Some(commands[2].1.clone()));
                }
                "clear_id_user" => {
                    item.set_command_id(None);
                    RibbonItemHelper::resolve_command(&item).unwrap();
                }
                "reapply" => {
                    item.set_command(None);
                    item.set_command_id(Some("copy".into()));
                    RibbonItemHelper::resolve_command(&item).unwrap();
                }
                "null_command_event" => descriptor.set_command(None),
                "checked_null_keeps_value" => {
                    descriptor.set_command(Some(commands[0].1.clone()));
                    descriptor.set_is_checked(None);
                }
                "unloaded" => {
                    detach(&item);
                    descriptor.set_is_enabled(true);
                    descriptor.set_is_checked(Some(false));
                }
                "missing_id_fallback" => {
                    catalog.unregister(Some(&"copy".into()));
                    RibbonItemHelper::resolve_command(&item).unwrap();
                }
                "empty_id" => {
                    item.set_command_id(Some(String::new()));
                    RibbonItemHelper::resolve_command(&item).unwrap();
                }
                _ => panic!("Unmapped original catalog case {step}"),
            }
            let command = item.command().and_then(|current| {
                commands
                    .iter()
                    .find(|(_, _, c)| **c == *current)
                    .map(|(name, _, _)| *name)
            });
            assert_eq!(json!(command), expected["command"], "{expected}");
            assert_eq!(
                json!(item.is_enabled()),
                expected["IsEnabled"],
                "{expected}"
            );
            assert_eq!(
                json!(item.is_checked()),
                expected["IsChecked"],
                "{expected}"
            );
            // FallbackCalls is a reference owner seam, not an actual native counter.
        }
    }
}

pub fn assert_disposal() {
    let catalog = RibbonCommandCatalog::new();
    let descriptor = RibbonCommandDescriptor::new(
        Some("copy".into()),
        None,
        Some(RibbonRelayCommand::new(|| {})),
    );
    catalog.register(Some(descriptor.clone()));
    let item = ContractItem::new();
    item.set_command_id(Some("copy".into()));
    let root = owner(catalog, &item);
    let baseline = descriptor.property_changed().handler_count();
    RibbonItemHelper::resolve_command(&item).unwrap();
    assert_eq!(descriptor.property_changed().handler_count(), baseline + 1);
    let command = item.command().unwrap();
    RibbonItemHelper::unlink(&item);
    assert_eq!(descriptor.property_changed().handler_count(), baseline);
    descriptor.set_is_enabled(false);
    assert!(item.is_enabled());
    assert!(item.command().as_ref() == Some(&command));
    RibbonItemHelper::resolve_command(&item).unwrap();
    assert!(!item.is_enabled());
    assert_eq!(descriptor.property_changed().handler_count(), baseline + 1);
    let weak = item.downgrade();
    drop(item);
    assert!(weak.upgrade().is_none());
    assert_eq!(descriptor.property_changed().handler_count(), baseline);
    drop(root);
}

pub fn assert_reentrant_application_command_is_preserved() {
    let fixture: Value = serde_json::from_str(include_str!("catalog-item.json")).unwrap();
    let catalog = RibbonCommandCatalog::new();
    let descriptor = RibbonCommandDescriptor::new(
        Some("copy".into()),
        None,
        Some(RibbonRelayCommand::new(|| {})),
    );
    catalog.register(Some(descriptor.clone()));
    let item = ContractItem::new();
    item.set_command_id(Some("copy".into()));
    let _root = owner(catalog, &item);
    let baseline = descriptor.property_changed().handler_count();
    let user = adapt_core_command(RibbonRelayCommand::new(|| {}));
    let weak = item.downgrade();
    let once = Rc::new(Cell::new(false));
    let seen = once.clone();
    let replacement = user.clone();
    let subscription = item.property_changed(move |e| {
        if e.property() == ContractItem::command_property().as_property() && !seen.replace(true) {
            let item = weak.upgrade().unwrap();
            item.set_command(Some(replacement.clone()));
            RibbonItemHelper::resolve_command(&item).unwrap();
        }
    });
    let check = |index: usize, step: &str| {
        assert_eq!(
            json!({"step":step,"user":item.command().as_ref()==Some(&user),"IsEnabled":item.is_enabled(),"IsChecked":item.is_checked()}),
            fixture["corpus"]["reentry"][index]
        );
    };
    RibbonItemHelper::resolve_command(&item).unwrap();
    check(0, "resolve");
    assert!(once.get());
    assert!(item.command().as_ref() == Some(&user));
    assert_eq!(descriptor.property_changed().handler_count(), baseline + 1);
    descriptor.set_command(Some(RibbonRelayCommand::new(|| {})));
    check(1, "replace");
    assert!(item.command().as_ref() == Some(&user));
    item.set_command_id(None);
    RibbonItemHelper::resolve_command(&item).unwrap();
    check(2, "clear_id_user");
    assert!(item.command().as_ref() == Some(&user));
    assert_eq!(descriptor.property_changed().handler_count(), baseline);
    subscription.dispose();
    // Native property-change subscribers do not retain the source through a cycle.
    let weak = item.downgrade();
    drop(item);
    assert!(weak.upgrade().is_none());
}

/// Application replacement must break a command's strong capture of its source.
/// An ownership marker must not keep that obsolete command/source cycle alive.
pub fn assert_replaced_capturing_catalog_command_is_released() {
    let item = ContractItem::new();
    item.set_command_id(Some("copy".into()));
    let captured = item.clone();
    let command = RibbonRelayCommand::new(move || {
        std::hint::black_box(&captured);
    });
    let weak_command = Rc::downgrade(&command);
    let catalog = RibbonCommandCatalog::new();
    let descriptor = RibbonCommandDescriptor::new(Some("copy".into()), None, Some(command.clone()));
    catalog.register(Some(descriptor.clone()));
    let _root = owner(catalog, &item);
    RibbonItemHelper::resolve_command(&item).unwrap();
    item.set_command(Some(adapt_core_command(RibbonRelayCommand::new(|| {}))));
    descriptor.set_command(None);
    drop(command);
    RibbonItemHelper::unlink(&item);
    assert!(
        weak_command.upgrade().is_none(),
        "catalog ownership marker retained the replaced command and its source capture"
    );
    let weak_item = item.downgrade();
    drop(item);
    assert!(weak_item.upgrade().is_none());
}

pub fn assert_core_identity_survives_native_adapter_recreation() {
    use ferroribbon_core::commands::ICommand;
    let fixture: Value = serde_json::from_str(include_str!("catalog-item.json")).unwrap();
    let a: Rc<dyn ICommand> = RibbonRelayCommand::new(|| {});
    let b: Rc<dyn ICommand> = RibbonRelayCommand::new(|| {});
    let user: Rc<dyn ICommand> = RibbonRelayCommand::new(|| {});
    let descriptor = RibbonCommandDescriptor::new(Some("copy".into()), None, Some(a.clone()));
    let catalog = RibbonCommandCatalog::new();
    catalog.register(Some(descriptor.clone()));
    let item = ContractItem::new();
    item.set_command_id(Some("copy".into()));
    let _owner = owner(catalog, &item);
    let check = |index: usize, step: &str| {
        let current = item
            .command()
            .as_ref()
            .and_then(ferroribbon::commands::source_core_command);
        let name = current.as_ref().and_then(|current| {
            [("A", &a), ("B", &b), ("User", &user)]
                .into_iter()
                .find(|(_, command)| Rc::ptr_eq(current, command))
                .map(|(name, _)| name)
        });
        assert_eq!(
            json!({"step":step,"command":name}),
            fixture["corpus"]["restoration"][index]
        );
    };
    RibbonItemHelper::resolve_command(&item).unwrap();
    check(0, "resolve");
    let first = item.command().unwrap();
    let weak_view = Rc::downgrade(&first);
    drop(first);
    item.set_command(Some(adapt_core_command(user.clone())));
    check(1, "user");
    assert!(
        weak_view.upgrade().is_none(),
        "ownership marker retained an obsolete native view"
    );
    item.set_command(Some(adapt_core_command(a.clone())));
    check(2, "restore");
    descriptor.set_command(Some(b.clone()));
    check(3, "replace");
    item.set_command_id(None);
    RibbonItemHelper::resolve_command(&item).unwrap();
    check(4, "clear_id");
}

pub fn assert_retired_weak_marker_preserves_command_getter_read() {
    let fixture: Value = serde_json::from_str(include_str!("catalog-item.json")).unwrap();
    let item = ContractItem::new();
    item.set_command_id(Some("copy".into()));
    let a = RibbonRelayCommand::new(|| {});
    let weak = Rc::downgrade(&a);
    let user = adapt_core_command(RibbonRelayCommand::new(|| {}));
    let descriptor = RibbonCommandDescriptor::new(Some("copy".into()), None, Some(a.clone()));
    let catalog = RibbonCommandCatalog::new();
    catalog.register(Some(descriptor.clone()));
    let _root = owner(catalog, &item);
    RibbonItemHelper::resolve_command(&item).unwrap();
    item.set_command(Some(user.clone()));
    descriptor.set_command(None);
    drop(a);
    assert!(weak.upgrade().is_none());
    item.set_command_id(None);
    let before = item.command_reads.get();
    RibbonItemHelper::resolve_command(&item).unwrap();
    let reads = item.command_reads.get() - before;
    assert_eq!(
        json!({"reads":reads,"user":item.command().as_ref()==Some(&user)}),
        fixture["corpus"]["retiredMarkerGetter"]
    );
}
