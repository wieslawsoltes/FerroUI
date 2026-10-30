// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original engine roots behind stateless metadata carriers.
use crate::markup_extension_tests::{context, provider, run};
use ferrodock_markup_extension::{LoadExtension, ReferenceExtension};
use ferrodock_model::{core::*, DockCapabilityResolver};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock},
    Factory,
};
use ferroui_base::{
    controls::{INameScope, NameScope},
    data::core::ValueTypes,
    metadata::{MarkupAssembly, MarkupType, MarkupTyped},
    utilities::Uri,
    BoxedValue,
};
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use std::rc::Rc;

#[test]
fn compiled_carrier_roots_return_the_original_engine_types_and_complete_the_scope() {
    run(|| {
        ferrodock_ferro::register_types();
        let scope = Rc::new(NameScope::new());
        let policy: Rc<dyn DockCapabilityPolicyImpl> =
            crate::compiled_xaml::build_capabilitypolicyroot_xaml(Some(provider(
                None,
                Some(scope.clone()),
            )))
            .unwrap();
        assert!(scope.is_completed());
        assert_eq!(policy.can_close(), Some(false));
        assert_eq!(policy.can_float(), Some(true));
        assert_eq!(policy.can_pin(), None);
        let overrides: Rc<DockCapabilityOverrides> =
            crate::compiled_xaml::build_capabilityoverridesroot_xaml(Some(provider(
                None,
                Some(Rc::new(NameScope::new())),
            )))
            .unwrap();
        assert_eq!(overrides.can_pin(), Some(false));
        assert_eq!(overrides.can_drop(), Some(true));
        assert!(overrides.has_any_override());
        overrides.set_can_pin(None);
        overrides.set_can_drop(None);
        assert!(!overrides.has_any_override());
    });
}
fn load(name: &str) -> BoxedValue {
    let extension = LoadExtension::with_source(
        Uri::absolute(&format!("ferres://ferrodock_markup_tests/{name}")).unwrap(),
    );
    extension
        .provide_value(&provider(
            Some(context(Some("ferres://ferrodock_markup_tests/"))),
            None,
        ))
        .unwrap()
        .unwrap()
}
#[test]
fn loaded_policy_roots_feed_the_original_capability_resolver_and_keep_identity() {
    run(|| {
        ferrodock_ferro::register_types();
        let value = load("CapabilityPolicyRoot.xaml");
        let policy = value
            .downcast_ref::<Rc<dyn DockCapabilityPolicyImpl>>()
            .unwrap()
            .clone();
        assert_eq!(policy.can_close(), Some(false));
        let fresh = load("CapabilityPolicyRoot.xaml");
        assert!(!ValueTypes::reference_equals(&value, &fresh));
        let overrides = load("CapabilityOverridesRoot.xaml")
            .downcast_ref::<Rc<DockCapabilityOverrides>>()
            .unwrap()
            .clone();
        let factory = Factory::new();
        let root = factory.create_layout().unwrap();
        let dock = DocumentDock::new();
        let document = Document::new();
        factory.add_dockable(Some(root.clone()), Some(dock.clone()));
        factory.add_dockable(Some(dock), Some(document.clone()));
        factory.init_layout(root.clone());
        root.clone()
            .as_root_dock()
            .unwrap()
            .set_root_dock_capability_policy(Some(policy.clone()));
        assert!(!DockCapabilityResolver::is_enabled(
            Some(document.clone()),
            DockCapability::Close,
            None
        ));
        overrides.set_can_close(Some(true));
        document.set_dock_capability_overrides(Some(overrides));
        assert!(DockCapabilityResolver::is_enabled(
            Some(document.clone()),
            DockCapability::Close,
            None
        ));
        document.set_dock_capability_overrides(None);
        policy.set_can_close(Some(true));
        assert!(DockCapabilityResolver::is_enabled(
            Some(document),
            DockCapability::Close,
            None
        ));
        let scope = Rc::new(NameScope::new());
        scope.register_value("policy", value.clone());
        let contract: Rc<dyn IDockCapabilityPolicy> = policy.clone();
        scope
            .try_register_value("policy", Rc::new(contract))
            .unwrap();
        scope.try_register_value("policy", Rc::new(policy)).unwrap();
        assert!(Rc::ptr_eq(
            &ReferenceExtension::with_name("policy".into())
                .provide_value(&provider(None, Some(scope)))
                .unwrap(),
            &value
        ));
    });
}
#[test]
fn runtime_emitter_uses_exact_public_model_return_paths_for_descriptor_traits() {
    run(|| {
        ferrodock_ferro::register_types();
        let documents = [
            ("Policy.xaml", include_str!("CapabilityPolicyRoot.xaml")),
            (
                "Overrides.xaml",
                include_str!("CapabilityOverridesRoot.xaml"),
            ),
        ];
        let result = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &documents,
            Some("ferres://ferrodock_markup_tests/"),
            &RuntimeXamlLoaderConfiguration::default(),
            &[],
        );
        for (result, expected) in result.iter().zip([
            "Result<::std::rc::Rc<dyn ferrodock_model::core::DockCapabilityPolicyImpl>",
            "Result<::std::rc::Rc<ferrodock_model::core::DockCapabilityOverrides>",
        ]) {
            let source = result.source.as_ref().unwrap();
            assert!(source.contains(expected), "{source}");
            assert_eq!(source.matches("::__markup_new_0()").count(), 1);
            assert!(!source.contains("DockCapabilityPolicyMetadataImpl"));
        }
        let metadata = <dyn ferrodock_ferro::DockCapabilityPolicyMetadata as MarkupTyped>::MARKUP;
        assert_eq!(
            metadata.owner_rust_name.unwrap()(),
            std::any::type_name::<dyn ferrodock_ferro::DockCapabilityPolicyMetadata>()
        );
    });
}
const ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Markup.Tests",
    crate_name: "ferrodock_markup_tests",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
#[test]
fn carrier_prefix_cannot_rewrite_a_different_model_type_and_exact_registration_enables_it() {
    run(|| {
        use crate::carrier_model::{Carrier, CarrierValue};
        MarkupAssembly::register(&ASSEMBLY);
        MarkupType::register(Carrier::MARKUP);
        MarkupType::register_rust_paths(&[(
            Carrier::MARKUP,
            "ferrodock_markup_tests::carrier_model::Carrier",
            false,
        )]);
        let mut configuration = RuntimeXamlLoaderConfiguration::new();
        configuration.local_assembly = Some(&ASSEMBLY);
        let compile = || {
            ferroui_markup_xaml_loader::rust_emitter::compile_documents(
                &[("Carrier.xaml", include_str!("CarrierRoot.xaml"))],
                Some("ferres://ferrodock_markup_tests/"),
                &configuration,
                &[],
            )
        };
        let before = compile();
        assert!(before[0]
            .source
            .as_ref()
            .unwrap_err()
            .contains("no exact public Rust value type"));
        ferroui_base::metadata::register_type_rust_paths(&[(
            || std::any::TypeId::of::<Rc<CarrierValue>>(),
            "std::rc::Rc<ferrodock_markup_tests::carrier_model::CarrierValue>",
        )]);
        let after = compile();
        let source = after[0].source.as_ref().unwrap();
        assert!(source
            .contains("Result<::std::rc::Rc<ferrodock_markup_tests::carrier_model::CarrierValue>"));
        let value: Rc<CarrierValue> = crate::compiled_xaml::build_carrierroot_xaml(Some(provider(
            None,
            Some(Rc::new(NameScope::new())),
        )))
        .unwrap();
        assert_eq!(value.model().can_close(), Some(false));
    });
}
