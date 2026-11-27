//! Actual attached-property binding cleanup through reusable-library metadata.
use crate::tests::{NativeThemeScope, run};
use ferroribbon::controls::IRibbonItemEditable;
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
};
use ferroui_controls::{Control, StackPanel, automation::AutomationProperties};
use std::{cell::Cell, rc::Rc};

fn editable(item: &Control) -> Rc<dyn IRibbonItemEditable> {
    let boxed: BoxedValue = Rc::new(item.to_ref());
    ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItemEditable>>())
        .unwrap()
        .unwrap()
        .downcast_ref::<Rc<dyn IRibbonItemEditable>>()
        .unwrap()
        .clone()
}

#[test]
fn initial_owned_binding_reentry_crosses_compiled_runtime_and_downstream_boundaries() {
    run(|| {
        use ferroui_base::data::{
            BindingBase, CompiledBinding, CompiledBindingPathBuilder, ReflectionBinding,
        };
        let _theme = NativeThemeScope::new();
        for runtime in [false, true] {
            for compiled_binding in [false, true] {
                let root = if runtime {
                    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                        include_str!("OwnedBindings.xaml"),
                        Some(&crate::CONSUMER_ASSEMBLY),
                    )
                    .unwrap()
                } else {
                    crate::compiled_xaml::try_load(
                        None,
                        "ferres://ferroribbon_markup_consumer/OwnedBindings.xaml",
                    )
                    .unwrap()
                    .unwrap()
                };
                let panel = ValueTypes::as_object(&*root)
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap();
                let vm = ValueTypes::as_object(&*panel.data_context().unwrap())
                    .unwrap()
                    .cast::<crate::icon_consumer::IconViewModel>()
                    .unwrap();
                let property = AutomationProperties::name_property();
                let mut handles = Vec::new();
                let mut callbacks = Vec::new();
                for child in panel.children().snapshot().iter() {
                    vm.set_icon_text("Owned".into());
                    let item = child.cast::<Control>().unwrap();
                    editable(&item).set_label(None);
                    editable(&item).set_label(Some("Owned".into()));
                    item.set_value(property, None);
                    let weak = item.downgrade();
                    let once = Cell::new(true);
                    callbacks.push(item.property_changed(move |event| {
                        if event.property().id() == property.id() {
                            let item = weak.upgrade().unwrap();
                            if item.get_value(property).is_some() && once.replace(false) {
                                editable(&item).set_label(None);
                            }
                        }
                    }));
                    let binding: Rc<dyn BindingBase> = if compiled_binding {
                        let b = CompiledBinding::new(
                            CompiledBindingPathBuilder::new()
                                .ferro_property(
                                    crate::icon_consumer::IconViewModel::icon_text_property(),
                                )
                                .build(),
                        );
                        b.set_source(Some(Rc::new(vm.clone())));
                        b
                    } else {
                        let b = ReflectionBinding::new("IconText");
                        b.set_source(Some(Rc::new(vm.clone())));
                        b
                    };
                    handles.push(item.bind_binding(property, &binding));
                    assert_eq!(item.get_value(property), None);
                    assert!(item.has_local_value_binding(property.as_property()));
                }
                assert_eq!(vm.property_changed_subscriber_count(), 5);
                vm.set_icon_text("After".into());
                for child in panel.children().snapshot().iter() {
                    assert_eq!(child.get_value(property).as_deref(), Some("After"));
                }
                for callback in callbacks {
                    callback.dispose();
                }
                for handle in handles {
                    handle.dispose();
                }
                assert_eq!(vm.property_changed_subscriber_count(), 0);
            }
        }
    });
}

#[test]
fn owned_binding_cleanup_compiled_runtime_xaml_and_downstream_subclass() {
    run(|| {
        let _theme = NativeThemeScope::new();
        let compiled = crate::compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/OwnedBindings.xaml",
        )
        .unwrap()
        .unwrap();
        let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("OwnedBindings.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap();
        for value in [compiled, runtime] {
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let vm = ValueTypes::as_object(&*panel.data_context().unwrap())
                .unwrap()
                .cast::<crate::icon_consumer::IconViewModel>()
                .unwrap();
            let property = AutomationProperties::name_property();
            assert_eq!(panel.children().count(), 5);
            assert_eq!(vm.property_changed_subscriber_count(), 5);
            for child in panel.children().snapshot().iter() {
                let item = child.cast::<Control>().unwrap();
                assert_eq!(item.get_value(property).as_deref(), Some("Owned"));
                assert!(item.has_local_value_binding(property.as_property()));
                let changes = Rc::new(Cell::new(0));
                let count = changes.clone();
                let subscription = item.property_changed(move |change| {
                    if change.property().id() == property.id() {
                        count.set(count.get() + 1);
                    }
                });
                let boxed: BoxedValue = Rc::new(item.clone());
                let editable = ValueTypes::try_convert(
                    Some(&boxed),
                    ValueType::of::<Rc<dyn IRibbonItemEditable>>(),
                )
                .unwrap()
                .unwrap();
                editable
                    .downcast_ref::<Rc<dyn IRibbonItemEditable>>()
                    .unwrap()
                    .set_label(None);
                assert_eq!(item.get_value(property), None);
                assert!(!item.has_local_value_binding(property.as_property()));
                assert_eq!(changes.get(), 1);
                subscription.dispose();
            }
            assert_eq!(vm.property_changed_subscriber_count(), 0);
            vm.set_icon_text("After clear".into());
            for child in panel.children().snapshot().iter() {
                assert_eq!(child.get_value(property), None);
            }
        }
    });
}
