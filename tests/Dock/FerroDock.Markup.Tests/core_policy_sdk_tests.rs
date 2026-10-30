//! Downstream consumers of the original Core assembly and source capability/window objects.
use crate::markup_extension_tests::run;
use ferrodock_model::{core::*, DockCapabilityResolver};
use ferrodock_model_ferro::*;
use ferroui_base::{
    metadata::{from_markup_value, MarkupAssembly, MarkupTyped},
    threading::Dispatcher,
    BoxedValue, Ref,
};
use ferroui_controls::{CheckBox, StackPanel, TextBlock};
use std::rc::Rc;
fn load(name: &str) -> BoxedValue {
    crate::compiled_xaml::try_load(
        None,
        &format!("ferres://ferrodock_markup_tests/{name}.xaml"),
    )
    .unwrap()
    .unwrap()
}
#[test]
fn core_qualified_policy_roots_and_legacy_aliases_retain_original_engine_identity_and_owner() {
    run(|| {
        for (core, legacy) in [
            (
                <dyn DockCapabilityPolicyMetadata>::MARKUP,
                <dyn ferrodock_ferro::DockCapabilityPolicyMetadata>::MARKUP,
            ),
            (
                <dyn DockCapabilityOverridesMetadata>::MARKUP,
                <dyn ferrodock_ferro::DockCapabilityOverridesMetadata>::MARKUP,
            ),
            (
                <dyn IDockCapabilityPolicyMetadata>::MARKUP,
                <dyn ferrodock_ferro::IDockCapabilityPolicyMetadata>::MARKUP,
            ),
            (
                <dyn IDockWindowMetadata>::MARKUP,
                <dyn ferrodock_ferro::IDockWindowMetadata>::MARKUP,
            ),
        ] {
            assert!(std::ptr::eq(core, legacy));
            assert_eq!(
                MarkupAssembly::of_module(core.module_path).unwrap().name,
                "FerroDock.Model"
            );
        }
        let value = load("CoreCapabilityPolicyRoot");
        let policy =
            from_markup_value::<Rc<dyn DockCapabilityPolicyImpl>>(&Some(value.clone())).unwrap();
        assert_eq!(policy.can_close(), Some(false));
        assert_eq!(policy.can_float(), Some(true));
        assert!(policy.can_pin().is_none());
        let contract =
            from_markup_value::<Rc<dyn IDockCapabilityPolicy>>(&Some(value.clone())).unwrap();
        contract.set_can_drop(Some(false));
        assert_eq!(policy.get(DockCapability::Drop), Some(false));
        let value = load("CoreCapabilityOverridesRoot");
        let over = from_markup_value::<Rc<DockCapabilityOverrides>>(&Some(value)).unwrap();
        assert_eq!(over.can_pin(), Some(false));
        assert_eq!(over.can_drop(), Some(true));
        assert!(over.has_any_override());
        let factory = ferrodock_model_reactiveui::Factory::new();
        let root = factory.create_root_dock();
        let dock = factory.create_document_dock();
        let item = factory.create_document();
        factory.add_dockable(Some(root.clone()), Some(dock.clone()));
        factory.add_dockable(Some(dock.clone()), Some(item.clone()));
        factory.init_layout(root.clone());
        root.set_root_dock_capability_policy(Some(policy.clone()));
        let denied = DockCapabilityResolver::evaluate(Some(item.clone()), DockCapability::Close);
        assert!(!denied.effective_value());
        assert_eq!(
            denied.effective_source(),
            DockCapabilityValueSource::RootPolicy
        );
        item.set_dock_capability_overrides(Some(over.clone()));
        assert!(
            !DockCapabilityResolver::evaluate(Some(item.clone()), DockCapability::Pin)
                .effective_value()
        );
        over.set_can_close(Some(true));
        let allowed = DockCapabilityResolver::evaluate(Some(item), DockCapability::Close);
        assert!(allowed.effective_value());
        assert_eq!(
            allowed.effective_source(),
            DockCapabilityValueSource::DockableOverride
        );
    });
}
#[test]
fn compiled_window_options_preserve_parent_live_binding_two_way_edits_and_original_factory_application(
) {
    run(|| {
        let value = load("CoreWindowOptionsRoot");
        let options = from_markup_value::<Rc<DockWindowOptions>>(&Some(value.clone())).unwrap();
        assert_eq!(options.owner_mode(), DockWindowOwnerMode::ParentWindow);
        assert!(options.is_modal());
        assert_eq!(options.show_in_taskbar(), Some(false));
        let parent = options.parent_window().unwrap();
        assert_eq!(parent.title(), "Parent title");
        let view = load("CoreWindowOptionsBindings");
        let panel = view.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(Some(value));
        Dispatcher::ui_thread().run_jobs(None);
        let title = panel.children().get(0).cast::<TextBlock>().unwrap();
        assert_eq!(title.text().as_deref(), Some("Parent title"));
        parent.set_title("Changed parent".into());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(title.text().as_deref(), Some("Changed parent"));
        panel
            .children()
            .get(1)
            .cast::<CheckBox>()
            .unwrap()
            .set_is_checked(Some(false));
        panel
            .children()
            .get(2)
            .cast::<CheckBox>()
            .unwrap()
            .set_is_checked(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!options.is_modal());
        assert!(options.show_in_taskbar().is_none());
        let factory = ferrodock_model_reactiveui::Factory::new();
        let root = factory.create_root_dock();
        let dock = factory.create_document_dock();
        let item = factory.create_document();
        factory.add_dockable(Some(root.clone()), Some(dock.clone()));
        factory.add_dockable(Some(dock), Some(item.clone()));
        factory.init_layout(root.clone());
        let window = factory
            .create_window_from_with_options(Some(item), Some(options.clone()))
            .unwrap();
        assert_eq!(window.owner_mode(), DockWindowOwnerMode::ParentWindow);
        assert!(!window.is_modal());
        assert!(Rc::ptr_eq(&window.parent_window().unwrap(), &parent));
        window.set_show_in_taskbar(Some(true));
        options.apply_to(Some(window.as_ref()));
        assert_eq!(window.show_in_taskbar(), Some(true));
        panel.set_data_context(None);
    });
}
#[test]
fn compiled_evaluation_constructor_and_readonly_bindings_keep_supplied_source_values() {
    run(|| {
        let value = load("CoreCapabilityEvaluationRoot");
        let evaluation =
            from_markup_value::<Rc<DockCapabilityEvaluation>>(&Some(value.clone())).unwrap();
        assert_eq!(evaluation.capability(), DockCapability::Pin);
        assert!(!evaluation.base_value());
        assert!(evaluation.root_policy_value().is_none());
        assert_eq!(evaluation.dock_policy_value(), Some(true));
        assert_eq!(evaluation.dockable_override_value(), Some(false));
        assert!(evaluation.effective_value());
        assert_eq!(
            evaluation.effective_source(),
            DockCapabilityValueSource::DockPolicy
        );
        assert_eq!(evaluation.diagnostic_message(), "Supplied diagnostics");
        let view = load("CoreCapabilityEvaluationBindings");
        let panel = view.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(Some(value));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            panel
                .children()
                .get(0)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("Supplied diagnostics")
        );
        assert_eq!(
            panel
                .children()
                .get(1)
                .cast::<CheckBox>()
                .unwrap()
                .is_checked(),
            Some(true)
        );
        assert_eq!(
            panel
                .children()
                .get(2)
                .cast::<CheckBox>()
                .unwrap()
                .is_checked(),
            Some(true)
        );
        panel.set_data_context(None);
    });
}
#[test]
fn runtime_compiler_uses_core_public_paths_for_options_policy_evaluation_and_bindings() {
    run(|| {
        let mut config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        config.local_assembly = Some(&crate::locator_factory::ASSEMBLY);
        let documents = [
            ("Options.xaml", include_str!("CoreWindowOptionsRoot.xaml")),
            ("Policy.xaml", include_str!("CoreCapabilityPolicyRoot.xaml")),
            (
                "Overrides.xaml",
                include_str!("CoreCapabilityOverridesRoot.xaml"),
            ),
            (
                "Evaluation.xaml",
                include_str!("CoreCapabilityEvaluationRoot.xaml"),
            ),
            (
                "OptionBindings.xaml",
                include_str!("CoreWindowOptionsBindings.xaml"),
            ),
            (
                "EvaluationBindings.xaml",
                include_str!("CoreCapabilityEvaluationBindings.xaml"),
            ),
        ];
        let result = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &documents,
            Some("ferres://ferrodock_markup_tests/"),
            &config,
            &[],
        );
        for document in &result {
            assert!(document.source.is_ok(), "{:?}", document.source);
        }
        for (document, public) in result.iter().zip([
            "DockWindowOptionsMetadata",
            "DockCapabilityPolicyMetadata",
            "DockCapabilityOverridesMetadata",
            "DockCapabilityEvaluationMetadata",
        ]) {
            let source = document.source.as_ref().unwrap();
            assert!(source.contains(public));
            assert!(source.contains("ferrodock_model_ferro"));
            assert!(!source.contains(&format!("ferrodock_ferro::{public}")));
        }
    });
}
