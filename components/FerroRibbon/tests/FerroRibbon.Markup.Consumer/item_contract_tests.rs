use super::{CONSUMER_ASSEMBLY, compiled_xaml, item_contract_consumer::*, tests::run};
use ferroribbon::controls::{
    IRibbonItem, IRibbonItemEditable, RibbonItemHelper, RibbonItemLayout, RibbonKeyTipResult,
    RibbonOverflowMenuItems, RibbonSimplifiedLabel,
};
use ferroribbon::primitives::RibbonMetrics;
use ferroribbon_core::{RibbonGroupState, RibbonItemSize, layout::RibbonSizeDefinition};
use ferroui_base::{
    BoxedValue, Ref,
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupType, MarkupTyped, MarkupValue, from_markup_value, into_markup_value},
    threading::Dispatcher,
};
use ferroui_controls::{ContentControl, Control, StackPanel, TextBlock, TextBox, Window};
use std::{rc::Rc, sync::Arc};
fn get(meta: &MarkupType, name: &str, value: MarkupValue) -> MarkupValue {
    (meta.find_property(name).unwrap().get.unwrap())(&[value]).unwrap()
}
fn call(meta: &MarkupType, name: &str, values: &[MarkupValue]) -> MarkupValue {
    (meta
        .find_methods(name)
        .find(|m| m.parameters.len() + usize::from(!m.is_static) == values.len())
        .unwrap()
        .invoke)(values)
    .unwrap()
}
#[test]
fn interface_xaml_compiled_and_runtime_binding_writes_and_object_identity() {
    run(|| {
        let compiled = compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/ItemContracts.xaml",
        )
        .unwrap()
        .unwrap();
        let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("ItemContracts.xaml"),
            Some(&CONSUMER_ASSEMBLY),
        )
        .unwrap();
        for value in [compiled, runtime] {
            let root = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let data = root.data_context().unwrap();
            let data =
                ValueTypes::try_convert(Some(&data), ValueType::of::<Rc<ItemContractViewModel>>())
                    .flatten()
                    .unwrap();
            let vm = data
                .downcast_ref::<Rc<ItemContractViewModel>>()
                .unwrap()
                .clone();
            let content = root.children().get(0).cast::<ContentControl>().unwrap();
            let editor = root.children().get(1).cast::<TextBox>().unwrap();
            let size = root.children().get(2).cast::<TextBlock>().unwrap();
            let window = Window::new();
            window.set_content(Some(Control::boxed(&root)));
            window.show();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                Control::from_boxed(content.content().as_ref().unwrap()).unwrap(),
                vm.item.clone().upcast::<Control>()
            );
            assert_eq!(editor.text().as_deref(), Some("Original"));
            assert_eq!(size.text().as_deref(), Some("Medium"));
            vm.item.set_label(Some("Source update".into()));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(editor.text().as_deref(), Some("Source update"));
            editor.set_text(Some("Σ ribbon 🦀"));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(vm.item.label().as_deref(), Some("Σ ribbon 🦀"));
            RibbonItemHelper::update_automation(&vm.item).unwrap();
            assert_eq!(
                vm.item
                    .get_value(
                        ferroui_controls::automation::AutomationProperties::automation_id_property(
                        )
                    )
                    .as_deref(),
                Some("public-item")
            );
            window.close();
            Dispatcher::ui_thread().run_jobs(None);
        }
    });
}
#[test]
fn inherited_item_methods_layout_copy_sequence_and_dispatch_cross_crates() {
    run(|| {
        let item = ContractItem::new();
        item.set_label(Some("Copy".into()));
        let boxed: BoxedValue = Rc::new(item.clone());
        let editable =
            ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItemEditable>>())
                .unwrap()
                .unwrap();
        let view = editable
            .downcast_ref::<Rc<dyn IRibbonItemEditable>>()
            .unwrap();
        let base = ValueTypes::try_convert(Some(&editable), ValueType::of::<Rc<dyn IRibbonItem>>())
            .unwrap()
            .unwrap();
        assert_eq!(ValueTypes::as_object(&*base), Some(item.clone().upcast()));
        assert!(
            ValueTypes::try_convert(
                Some(&boxed),
                ValueType::of::<Option<Rc<dyn IRibbonItemEditable>>>()
            )
            .unwrap()
            .is_some()
        );
        let meta = <dyn IRibbonItem as MarkupTyped>::MARKUP;
        let metrics = RibbonMetrics::compact();
        let layout = RibbonItemLayout {
            size: RibbonItemSize::Small,
            metrics: Some(metrics.clone()),
            is_simplified: true,
            show_label: Some(false),
            group_state: RibbonGroupState::Collapsed,
        };
        call(
            meta,
            "ApplyLayout",
            &[Some(base.clone()), into_markup_value(layout.clone())],
        );
        assert_eq!(&*item.layouts.borrow(), &[layout]);
        assert!(Arc::ptr_eq(
            item.layouts.borrow()[0].metrics.as_ref().unwrap(),
            &metrics
        ));
        assert!(from_markup_value::<bool>(&call(meta, "Invoke", &[Some(base.clone())])).unwrap());
        assert_eq!(item.invocations.get(), 1);
        item.set_is_enabled(false);
        assert!(!from_markup_value::<bool>(&call(meta, "Invoke", &[Some(base.clone())])).unwrap());
        assert_eq!(item.invocations.get(), 1);
        let copy = from_markup_value::<Option<Ref<Control>>>(&call(
            meta,
            "CreateLinkedCopy",
            &[Some(base.clone())],
        ))
        .unwrap()
        .unwrap()
        .cast::<ContractItem>()
        .unwrap();
        assert_ne!(copy, item);
        copy.set_label(Some("linked".into()));
        assert_eq!(item.label(), copy.label());
        RibbonItemHelper::unlink(&copy);
        item.set_label(None);
        assert_eq!(copy.label().as_deref(), Some("linked"));
        let sequence = from_markup_value::<RibbonOverflowMenuItems>(&call(
            meta,
            "CreateOverflowMenuItems",
            &[Some(base.clone())],
        ))
        .unwrap();
        let same = sequence.clone();
        assert!(sequence == same);
        assert_eq!(item.enumerations.get(), 0);
        let a = sequence.iter().collect::<Vec<_>>();
        let b = sequence.iter().collect::<Vec<_>>();
        assert_eq!(a, b);
        assert_eq!(item.enumerations.get(), 2);
        assert_eq!(a[0], Some(item.clone().upcast()));
        assert!(a[1].is_none());
        assert_eq!(a[0], a[2]);
        let result = from_markup_value::<Rc<RibbonKeyTipResult>>(&call(
            meta,
            "OnKeyTip",
            &[Some(base.clone())],
        ))
        .unwrap();
        assert!(Rc::ptr_eq(&result, &RibbonKeyTipResult::close()));
        item.malformed.set(true);
        assert!((meta.find_property("Label").unwrap().get.unwrap())(&[Some(base)]).is_err());
        view.set_simplified_label(RibbonSimplifiedLabel::Show);
        assert_eq!(item.simplified_label(), RibbonSimplifiedLabel::Show);
    });
}
#[test]
fn value_descriptors_preserve_defaults_nullable_metrics_unknown_enums_and_identity() {
    run(|| {
        let meta = RibbonItemLayout::MARKUP;
        for ctor in meta.constructors {
            let count = ctor.parameters.len();
            let metrics = RibbonMetrics::touch();
            let mut args = vec![
                into_markup_value(RibbonItemSize::from_raw(-7)),
                into_markup_value(Some(metrics.clone())),
                into_markup_value(true),
                into_markup_value(Some(false)),
                into_markup_value(RibbonGroupState::from_raw(99)),
            ];
            args.truncate(count);
            let value = (ctor.invoke)(&args).unwrap();
            let layout = from_markup_value::<RibbonItemLayout>(&value).unwrap();
            if count == 0 {
                assert_eq!(layout, RibbonItemLayout::default());
                assert!(layout.metrics.is_none())
            } else {
                assert_eq!(layout.size.raw(), -7);
                assert!(Arc::ptr_eq(layout.metrics.as_ref().unwrap(), &metrics));
                assert_eq!(layout.is_simplified, count >= 3);
                assert_eq!(
                    layout.show_label,
                    if count >= 4 { Some(false) } else { None }
                );
                assert_eq!(layout.group_state.raw(), if count == 5 { 99 } else { 0 })
            }
            assert_eq!(
                from_markup_value::<RibbonItemSize>(&get(meta, "Size", value)).unwrap(),
                layout.size
            );
        }
        let key = RibbonKeyTipResult::MARKUP;
        let constructed = (key.constructors[0].invoke)(&[]).unwrap();
        let a = from_markup_value::<Rc<RibbonKeyTipResult>>(&constructed).unwrap();
        let b = RibbonKeyTipResult::close();
        assert!(!Rc::ptr_eq(&a, &b));
        let definition =
            ferroribbon::metadata::item_contracts::RibbonSizeDefinitionMetadata::MARKUP;
        let parsed = call(
            definition,
            "Parse",
            &[into_markup_value("Large,Middle,Small".to_owned())],
        );
        assert_eq!(
            from_markup_value::<RibbonSizeDefinition>(&parsed).unwrap(),
            RibbonSizeDefinition::large_medium_small()
        );
        assert!(
            (definition.find_methods("Parse").next().unwrap().invoke)(&[into_markup_value(
                "bad".to_owned()
            )])
            .is_err()
        );
    });
}

#[test]
fn every_editable_property_round_trips_through_public_interface_metadata() {
    run(|| {
        let item = ContractItem::new();
        let boxed: BoxedValue = Rc::new(item.clone());
        let editable =
            ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItemEditable>>())
                .flatten()
                .unwrap();
        let meta = <dyn IRibbonItemEditable as MarkupTyped>::MARKUP;
        let shared = Control::boxed(TextBlock::new());
        let cases: Vec<(&str, MarkupValue)> = vec![
            ("Id", into_markup_value(Some("Σ𐐀".to_owned()))),
            (
                "Label",
                into_markup_value(Some("Combining e\u{301}".to_owned())),
            ),
            ("Icon", into_markup_value(Some(shared.clone()))),
            ("LargeIcon", into_markup_value(Some(shared.clone()))),
            ("Size", into_markup_value(RibbonItemSize::from_raw(-45))),
            (
                "SizeDefinition",
                into_markup_value(Some("Small,Large".to_owned())),
            ),
            ("KeyTip", into_markup_value(Some("AB".to_owned()))),
            ("ScreenTip", into_markup_value(Some(shared.clone()))),
            ("CommandId", into_markup_value(Some("cmd".to_owned()))),
            ("Shortcut", into_markup_value(Some("Ctrl+Σ".to_owned()))),
            (
                "SimplifiedVisibility",
                into_markup_value(ferroribbon_core::RibbonSimplifiedVisibility::from_raw(-9)),
            ),
            (
                "SimplifiedLabel",
                into_markup_value(RibbonSimplifiedLabel::from_raw(71)),
            ),
            ("ShowLabel", into_markup_value(false)),
            ("CanAddToQuickAccess", into_markup_value(false)),
        ];
        for (name, value) in cases {
            let property = meta.find_property(name).unwrap();
            (property.set.unwrap())(&[Some(editable.clone()), value.clone()]).unwrap();
            assert!(
                (property.get.unwrap())(&[Some(editable.clone())]).unwrap() == value,
                "{name}"
            );
        }
        for name in [
            "Id",
            "Label",
            "SizeDefinition",
            "KeyTip",
            "CommandId",
            "Shortcut",
        ] {
            let property = meta.find_property(name).unwrap();
            (property.set.unwrap())(&[Some(editable.clone()), into_markup_value(None::<String>)])
                .unwrap();
            assert!(
                from_markup_value::<Option<String>>(
                    &(property.get.unwrap())(&[Some(editable.clone())]).unwrap()
                )
                .unwrap()
                .is_none()
            );
        }
        for name in ["Icon", "LargeIcon", "ScreenTip"] {
            let property = meta.find_property(name).unwrap();
            (property.set.unwrap())(&[
                Some(editable.clone()),
                into_markup_value(None::<BoxedValue>),
            ])
            .unwrap();
            assert!(
                from_markup_value::<Option<BoxedValue>>(
                    &(property.get.unwrap())(&[Some(editable.clone())]).unwrap()
                )
                .unwrap()
                .is_none()
            );
        }
    });
}

#[test]
fn source_effective_id_and_editable_raw_id_dispatch_are_distinct() {
    run(|| {
        let item = ContractItem::new();
        item.set_name(Some("name-fallback".to_owned()));
        item.set_command_id(Some("command-fallback".into()));
        let boxed: BoxedValue = Rc::new(item.clone());
        let base = ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItem>>())
            .flatten()
            .unwrap();
        let editable =
            ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItemEditable>>())
                .flatten()
                .unwrap();
        let raw = get(
            <dyn IRibbonItemEditable as MarkupTyped>::MARKUP,
            "Id",
            Some(editable.clone()),
        );
        let effective = get(
            <dyn IRibbonItem as MarkupTyped>::MARKUP,
            "Id",
            Some(base.clone()),
        );
        assert_eq!(
            from_markup_value::<Option<String>>(&effective)
                .unwrap()
                .as_deref(),
            Some("name-fallback")
        );
        assert!(from_markup_value::<Option<String>>(&raw).unwrap().is_none());
        for (id, expected) in [(Some(""), "name-fallback"), (Some("explicit"), "explicit")] {
            item.set_id(id.map(Into::into));
            assert_eq!(
                from_markup_value::<Option<String>>(&get(
                    <dyn IRibbonItemEditable as MarkupTyped>::MARKUP,
                    "Id",
                    Some(editable.clone())
                ))
                .unwrap()
                .as_deref(),
                id
            );
            assert_eq!(
                from_markup_value::<Option<String>>(&get(
                    <dyn IRibbonItem as MarkupTyped>::MARKUP,
                    "Id",
                    Some(base.clone())
                ))
                .unwrap()
                .as_deref(),
                Some(expected)
            );
        }
        item.set_id(None);
        item.set_name(None::<String>);
        assert_eq!(
            from_markup_value::<Option<String>>(&get(
                <dyn IRibbonItem as MarkupTyped>::MARKUP,
                "Id",
                Some(base)
            ))
            .unwrap()
            .as_deref(),
            Some("command-fallback")
        );
    });
}
