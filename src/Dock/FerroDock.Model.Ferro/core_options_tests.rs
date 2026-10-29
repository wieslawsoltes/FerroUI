//! Core-only consumers of original capability classes, resolver and window options.
use crate::*;
use ferrodock_model::{core::*, DockCapabilityResolver};
use ferrodock_model_reactiveui::Factory;
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{from_markup_value, MarkupAssembly, MarkupType, MarkupTyped, MarkupValue},
    BoxedValue,
};
use std::rc::Rc;
fn property(
    metadata: &'static MarkupType,
    name: &str,
) -> &'static ferroui_base::metadata::MarkupProperty {
    metadata
        .find_property(name)
        .or_else(|| metadata.base_type().map(|base| property(base, name)))
        .unwrap()
}
fn get<T: Clone + 'static>(metadata: &'static MarkupType, value: &BoxedValue, name: &str) -> T {
    let result = (property(metadata, name).get.unwrap())(&[Some(value.clone())]).unwrap();
    from_markup_value(&result).unwrap()
}
fn set(metadata: &'static MarkupType, value: &BoxedValue, name: &str, argument: MarkupValue) {
    (property(metadata, name).set.unwrap())(&[Some(value.clone()), argument]).unwrap();
}
fn resolve(name: &str, args: &[MarkupValue]) -> MarkupValue {
    let method = DockCapabilityResolverMetadata::MARKUP
        .find_methods(name)
        .find(|method| method.parameters.len() == args.len())
        .unwrap();
    (method.invoke)(args).unwrap()
}
#[test]
fn core_owns_policy_options_evaluation_window_and_value_source_descriptors() {
    crate::register_types();
    for metadata in [
        <dyn IDockCapabilityPolicyMetadata>::MARKUP,
        <dyn DockCapabilityPolicyMetadata>::MARKUP,
        <dyn DockCapabilityOverridesMetadata>::MARKUP,
        <dyn DockWindowOptionsMetadata>::MARKUP,
        <dyn DockCapabilityEvaluationMetadata>::MARKUP,
        <dyn IDockWindowMetadata>::MARKUP,
        DockCapabilityResolverMetadata::MARKUP,
        DockCapabilityValueSourceMetadata::MARKUP,
    ] {
        assert_eq!(
            MarkupAssembly::of_module(metadata.module_path)
                .unwrap()
                .name,
            "FerroDock.Model"
        );
    }
    let exported = ferroui_build_scan::model::AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferrodock_model_ferro.xamlmeta"
    )))
    .unwrap();
    for name in [
        "DockWindowOptions",
        "DockCapabilityEvaluation",
        "DockCapabilityPolicy",
        "DockCapabilityOverrides",
        "IDockCapabilityPolicy",
        "IDockWindow",
        "DockCapabilityResolver",
        "DockCapabilityValueSource",
    ] {
        assert!(
            exported.types.iter().any(|type_| type_.name == name),
            "{name}"
        );
    }
    let metadata = DockCapabilityValueSourceMetadata::MARKUP;
    assert_eq!(metadata.enum_members.len(), 4);
    for (member, value) in metadata.enum_members.iter().zip([
        DockCapabilityValueSource::Dockable,
        DockCapabilityValueSource::RootPolicy,
        DockCapabilityValueSource::DockPolicy,
        DockCapabilityValueSource::DockableOverride,
    ]) {
        assert_eq!(member.value, i64::from(value as i32));
        assert_eq!(
            (member.get)().downcast_ref::<DockCapabilityValueSource>(),
            Some(&value)
        );
    }
}
#[test]
fn original_policy_constructors_properties_and_get_keep_shared_base_interface_identity() {
    crate::register_types();
    for metadata in [
        <dyn DockCapabilityPolicyMetadata>::MARKUP,
        <dyn DockCapabilityOverridesMetadata>::MARKUP,
    ] {
        let value = (metadata.constructors[0].invoke)(&[]).unwrap().unwrap();
        let original =
            from_markup_value::<Rc<dyn DockCapabilityPolicyImpl>>(&Some(value.clone())).unwrap();
        let contract =
            from_markup_value::<Rc<dyn IDockCapabilityPolicy>>(&Some(value.clone())).unwrap();
        assert!(ValueTypes::reference_equals(
            &value,
            &(Rc::new(original.clone()) as BoxedValue)
        ));
        assert!(ValueTypes::reference_equals(
            &value,
            &(Rc::new(contract.clone()) as BoxedValue)
        ));
        for (name, capability) in [
            ("CanClose", DockCapability::Close),
            ("CanPin", DockCapability::Pin),
            ("CanFloat", DockCapability::Float),
            ("CanDrag", DockCapability::Drag),
            ("CanDrop", DockCapability::Drop),
            ("CanDockAsDocument", DockCapability::DockAsDocument),
        ] {
            for expected in [None, Some(false), Some(true), None] {
                set(
                    <dyn IDockCapabilityPolicyMetadata>::MARKUP,
                    &(Rc::new(contract.clone()) as BoxedValue),
                    name,
                    Some(Rc::new(expected)),
                );
                assert_eq!(get::<Option<bool>>(metadata, &value, name), expected);
                let result = (<dyn DockCapabilityPolicyMetadata>::MARKUP
                    .find_methods("Get")
                    .next()
                    .unwrap()
                    .invoke)(&[
                    Some(value.clone()),
                    Some(Rc::new(capability)),
                ])
                .unwrap();
                assert_eq!(
                    from_markup_value::<Option<bool>>(&result).unwrap(),
                    expected
                );
                assert_eq!(original.get(capability), expected);
                if metadata.name == "DockCapabilityOverrides" {
                    assert_eq!(
                        get::<bool>(metadata, &value, "HasAnyOverride"),
                        expected.is_some()
                    );
                }
            }
        }
        let other = (metadata.constructors[0].invoke)(&[]).unwrap().unwrap();
        assert!(!ValueTypes::reference_equals(&value, &other));
    }
}
#[test]
fn window_options_metadata_preserves_nullable_taskbar_parent_identity_and_source_apply_failure() {
    crate::register_types();
    let metadata = <dyn DockWindowOptionsMetadata>::MARKUP;
    let value = (metadata.constructors[0].invoke)(&[]).unwrap().unwrap();
    let options = from_markup_value::<Rc<DockWindowOptions>>(&Some(value.clone())).unwrap();
    assert_eq!(
        get::<DockWindowOwnerMode>(metadata, &value, "OwnerMode"),
        DockWindowOwnerMode::Default
    );
    assert!(!get::<bool>(metadata, &value, "IsModal"));
    assert!(get::<Option<bool>>(metadata, &value, "ShowInTaskbar").is_none());
    let factory = Factory::new();
    let parent = factory.create_dock_window();
    let window = factory.create_dock_window();
    set(
        metadata,
        &value,
        "ParentWindow",
        Some(Rc::new(Some(parent.clone()))),
    );
    set(
        metadata,
        &value,
        "OwnerMode",
        Some(Rc::new(DockWindowOwnerMode::ParentWindow)),
    );
    set(metadata, &value, "IsModal", Some(Rc::new(true)));
    assert!(Rc::ptr_eq(
        &get::<Option<Rc<dyn IDockWindow>>>(metadata, &value, "ParentWindow").unwrap(),
        &parent
    ));
    let method = metadata.find_methods("ApplyTo").next().unwrap();
    assert!((method.invoke)(&[Some(value.clone()), None]).is_err());
    for setting in [None, Some(false), Some(true), None] {
        window.set_show_in_taskbar(Some(true));
        set(metadata, &value, "ShowInTaskbar", Some(Rc::new(setting)));
        (method.invoke)(&[Some(value.clone()), Some(Rc::new(window.clone()))]).unwrap();
        assert_eq!(window.owner_mode(), DockWindowOwnerMode::ParentWindow);
        assert!(window.is_modal());
        assert!(Rc::ptr_eq(&window.parent_window().unwrap(), &parent));
        assert_eq!(window.show_in_taskbar(), setting.or(Some(true)));
    }
    assert!(ValueTypes::reference_equals(
        &value,
        &(Rc::new(options.clone()) as BoxedValue)
    ));
    set(metadata, &value, "ParentWindow", None);
    assert!(options.parent_window().is_none());
}
#[test]
fn evaluation_metadata_constructor_stores_all_original_fields_without_resolving_them() {
    crate::register_types();
    let metadata = <dyn DockCapabilityEvaluationMetadata>::MARKUP;
    for root in [None, Some(false), Some(true)] {
        for dock in [None, Some(false), Some(true)] {
            for over in [None, Some(false), Some(true)] {
                let args: Vec<MarkupValue> = vec![
                    Some(Rc::new(DockCapability::Pin)),
                    Some(Rc::new(false)),
                    Some(Rc::new(root)),
                    Some(Rc::new(dock)),
                    Some(Rc::new(over)),
                    Some(Rc::new(true)),
                    Some(Rc::new(DockCapabilityValueSource::DockPolicy)),
                    Some(Rc::new("unchanged diagnostics".to_owned())),
                ];
                let value = (metadata.constructors[0].invoke)(&args).unwrap().unwrap();
                let model = from_markup_value::<Rc<DockCapabilityEvaluation>>(&Some(value.clone()))
                    .unwrap();
                assert!(ValueTypes::reference_equals(
                    &value,
                    &(Rc::new(model.clone()) as BoxedValue)
                ));
                assert_eq!(
                    get::<DockCapability>(metadata, &value, "Capability"),
                    DockCapability::Pin
                );
                assert!(!get::<bool>(metadata, &value, "BaseValue"));
                assert!(get::<bool>(metadata, &value, "EffectiveValue"));
                assert_eq!(
                    get::<Option<bool>>(metadata, &value, "RootPolicyValue"),
                    root
                );
                assert_eq!(
                    get::<Option<bool>>(metadata, &value, "DockPolicyValue"),
                    dock
                );
                assert_eq!(
                    get::<Option<bool>>(metadata, &value, "DockableOverrideValue"),
                    over
                );
                assert_eq!(
                    get::<DockCapabilityValueSource>(metadata, &value, "EffectiveSource"),
                    DockCapabilityValueSource::DockPolicy
                );
                assert_eq!(
                    get::<String>(metadata, &value, "DiagnosticMessage"),
                    "unchanged diagnostics"
                );
                assert!(metadata
                    .properties
                    .iter()
                    .all(|property| property.set.is_none()));
                let other = (metadata.constructors[0].invoke)(&args).unwrap().unwrap();
                assert!(!ValueTypes::reference_equals(&value, &other));
            }
        }
    }
}
#[test]
fn resolver_metadata_uses_original_precedence_and_distinguishes_default_context_overloads() {
    crate::register_types();
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let dock = factory.create_document_dock();
    let item = factory.create_document();
    factory.add_dockable(Some(root.clone()), Some(dock.clone()));
    factory.add_dockable(Some(dock.clone()), Some(item.clone()));
    factory.init_layout(root.clone());
    let root_policy = DockCapabilityPolicy::new();
    let dock_policy = DockCapabilityPolicy::new();
    let over = DockCapabilityOverrides::new();
    root.set_root_dock_capability_policy(Some(root_policy.clone()));
    dock.set_dock_capability_policy(Some(dock_policy.clone()));
    item.set_dock_capability_overrides(Some(over.clone()));
    let item_arg = Some(Rc::new(item.clone() as Rc<dyn IDockable>) as BoxedValue);
    let capability_arg = Some(Rc::new(DockCapability::Close) as BoxedValue);
    for root_value in [None, Some(false), Some(true)] {
        for dock_value in [None, Some(false), Some(true)] {
            for over_value in [None, Some(false), Some(true)] {
                root_policy.set_can_close(root_value);
                dock_policy.set_can_close(dock_value);
                over.set_can_close(over_value);
                for explicit in [false, true] {
                    let mut args = vec![item_arg.clone(), capability_arg.clone()];
                    if explicit {
                        args.push(None);
                    }
                    let result = resolve("Evaluate", &args);
                    let result =
                        from_markup_value::<Rc<DockCapabilityEvaluation>>(&result).unwrap();
                    let expected = if explicit {
                        DockCapabilityResolver::evaluate_with_context(
                            Some(item.clone()),
                            DockCapability::Close,
                            None,
                        )
                    } else {
                        DockCapabilityResolver::evaluate(Some(item.clone()), DockCapability::Close)
                    };
                    assert_eq!(result.effective_value(), expected.effective_value());
                    assert_eq!(result.effective_source(), expected.effective_source());
                    assert_eq!(result.diagnostic_message(), expected.diagnostic_message());
                    assert_eq!(result.root_policy_value(), root_value);
                    assert_eq!(
                        result.dock_policy_value(),
                        if explicit { None } else { dock_value }
                    );
                    assert_eq!(result.dockable_override_value(), over_value);
                }
                let enabled = resolve("IsEnabled", &[item_arg.clone(), capability_arg.clone()]);
                assert_eq!(
                    from_markup_value::<bool>(&enabled).unwrap(),
                    DockCapabilityResolver::is_enabled(
                        Some(item.clone()),
                        DockCapability::Close,
                        None
                    )
                );
                let enabled = resolve(
                    "IsEnabled",
                    &[
                        item_arg.clone(),
                        capability_arg.clone(),
                        Some(Rc::new(Some(dock.clone() as Rc<dyn IDock>))),
                    ],
                );
                assert_eq!(
                    from_markup_value::<bool>(&enabled).unwrap(),
                    DockCapabilityResolver::is_enabled(
                        Some(item.clone()),
                        DockCapability::Close,
                        Some(dock.clone())
                    )
                );
            }
        }
    }
    for name in ["ResolveOperationDock", "ResolveDropTargetDock"] {
        let result = resolve(name, std::slice::from_ref(&item_arg));
        assert!(Rc::ptr_eq(
            &from_markup_value::<Rc<dyn IDock>>(&result).unwrap(),
            &(dock.clone() as Rc<dyn IDock>)
        ));
    }
    let runtime = MarkupType::find_by_handle(ValueType::of::<Rc<dyn IDockWindow>>().id()).unwrap();
    assert_eq!(
        MarkupAssembly::of_module(runtime.module_path).unwrap().name,
        "FerroDock.Model"
    );
}

struct PolicyProbe {
    base: Rc<DockCapabilityPolicy>,
    calls: std::cell::Cell<usize>,
}
macro_rules! forward_policy_properties {
    ($($get:ident,$set:ident);+ $(;)?)=>{impl IDockCapabilityPolicy for PolicyProbe {$(fn $get(&self)->Option<bool>{self.base.$get()} fn $set(&self,value:Option<bool>){self.base.$set(value)})+}};
}
forward_policy_properties! {can_close,set_can_close;can_pin,set_can_pin;can_float,set_can_float;can_drag,set_can_drag;can_drop,set_can_drop;can_dock_as_document,set_can_dock_as_document;}
impl DockCapabilityPolicyImpl for PolicyProbe {
    fn get(&self, _capability: DockCapability) -> Option<bool> {
        self.calls.set(self.calls.get() + 1);
        Some(true)
    }
}
#[test]
fn policy_metadata_and_resolver_preserve_original_subclass_get_dispatch() {
    crate::register_types();
    let probe = Rc::new(PolicyProbe {
        base: DockCapabilityPolicy::new(),
        calls: Default::default(),
    });
    probe.base.set_can_close(Some(false));
    let model: Rc<dyn DockCapabilityPolicyImpl> = probe.clone();
    let value: BoxedValue = Rc::new(model.clone());
    let metadata = <dyn DockCapabilityPolicyMetadata>::MARKUP;
    assert_eq!(
        get::<Option<bool>>(metadata, &value, "CanClose"),
        Some(false)
    );
    let result = (metadata.find_methods("Get").next().unwrap().invoke)(&[
        Some(value),
        Some(Rc::new(DockCapability::Close)),
    ])
    .unwrap();
    assert_eq!(
        from_markup_value::<Option<bool>>(&result).unwrap(),
        Some(true)
    );
    assert_eq!(probe.calls.get(), 1);
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let item = factory.create_document();
    factory.add_dockable(Some(root.clone()), Some(item.clone()));
    factory.init_layout(root.clone());
    root.set_root_dock_capability_policy(Some(model));
    let result = resolve(
        "Evaluate",
        &[
            Some(Rc::new(item as Rc<dyn IDockable>)),
            Some(Rc::new(DockCapability::Close)),
        ],
    );
    let result = from_markup_value::<Rc<DockCapabilityEvaluation>>(&result).unwrap();
    assert!(result.effective_value());
    assert_eq!(
        result.effective_source(),
        DockCapabilityValueSource::RootPolicy
    );
    assert_eq!(probe.calls.get(), 2);
}
