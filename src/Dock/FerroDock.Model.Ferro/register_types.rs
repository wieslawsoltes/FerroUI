//! Original model assembly identity, factory-base and enum/flags registration.
use crate::*;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupAssembly, MarkupType, MarkupTyped},
};
/// Logical identity of the original Dock model assembly.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Model",
    crate_name: "ferrodock_model_ferro",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
/// Registers original model values and factory-base metadata without loading UI controls.
pub fn register_types() {
    ferroui_base::register_types();
    ferrodock_controls_recycling_model_ferro::register_types();
    register_factory_rust_paths();
    crate::model_handles::register();
    crate::control_contract_registration::register();
    crate::local_target_metadata::register();
    crate::global_target_metadata::register();
    crate::host_contract_metadata::register();
    crate::dock_window_host_factory_metadata::register();
    crate::container_metadata::register();
    crate::dockable_metadata::register();
    crate::dock_metadata::register();
    crate::dockable_list_metadata::register();
    ValueTypes::register_nullable::<
        ferrodock_model::collections::ListHandle<
            Option<std::rc::Rc<dyn ferrodock_model::core::IDockable>>,
        >,
    >();

    ferrodock_runtime::register_value_type::<std::rc::Rc<dyn ferrodock_model::core::IDockable>>();
    ferrodock_runtime::register_value_type::<std::rc::Rc<dyn ferrodock_model::core::IDock>>();
    crate::factory_metadata::register();
    crate::factory_locators::register();
    crate::factory_search::register();
    crate::capability_registration::register();
    crate::dock_window_metadata::register();
    crate::core_options_metadata::register();
    ValueTypes::register_nullable::<ferrodock_model::core::DockCapabilityValueSource>();
    ValueTypes::register_nullable::<std::rc::Rc<dyn ferrodock_model::core::IDockable>>();
    ValueTypes::register_nullable::<std::rc::Rc<dyn ferrodock_model::controls::IRootDock>>();
    ValueTypes::register_nullable::<std::rc::Rc<dyn ferrodock_model::core::IDockWindow>>();
    ValueTypes::register_nullable::<std::rc::Rc<dyn ferrodock_model::core::IHostWindow>>();
    crate::event_metadata::register();
    crate::factory_registry_reference::register();
    crate::factory_base_reference::register();
    ValueTypes::register_nullable::<ferrodock_model::core::DockTrackingChangeReason>();
    ValueTypes::register_nullable::<ferrodock_model::core::Alignment>();
    ValueTypes::register_nullable::<ferrodock_model::core::DockCapability>();
    ValueTypes::register_nullable::<ferrodock_model::command_bars::DockCommandBarKind>();
    ValueTypes::register_nullable::<ferrodock_model::command_bars::DockCommandBarMergeMode>();
    ValueTypes::register_nullable::<ferrodock_model::core::DockFloatingWindowHostMode>();
    ValueTypes::register_nullable::<ferrodock_model::core::DockMode>();
    ValueTypes::register_nullable::<ferrodock_model::core::DockOperation>();
    ValueTypes::register_nullable::<ferrodock_model::core::DockWindowOwnerMode>();
    ValueTypes::register_nullable::<ferrodock_model::core::DockWindowState>();
    ValueTypes::register_nullable::<ferrodock_model::core::DockingWindowState>();
    ValueTypes::register_nullable::<ferrodock_model::core::DocumentCloseButtonShowMode>();
    ValueTypes::register_nullable::<ferrodock_model::core::DocumentLayoutMode>();
    ValueTypes::register_nullable::<ferrodock_model::core::DocumentTabLayout>();
    ValueTypes::register_nullable::<ferrodock_model::core::GridResizeDirection>();
    ValueTypes::register_nullable::<ferrodock_model::core::GripMode>();
    ValueTypes::register_nullable::<ferrodock_model::core::MdiWindowState>();
    ValueTypes::register_nullable::<ferrodock_model::core::Orientation>();
    ValueTypes::register_nullable::<ferrodock_model::core::PinnedDockDisplayMode>();
    ValueTypes::register_nullable::<ferrodock_model::core::TrackingMode>();
    ValueTypes::register_nullable::<ferrodock_model::core::SplitViewDisplayMode>();
    ValueTypes::register_nullable::<ferrodock_model::core::SplitViewPanePlacement>();
    ValueTypes::register_display::<ferrodock_model::core::DockingWindowState>();
    ferroui_base::__ferro_compiler_metadata! {
        ferroui_base::metadata::register_type_rust_paths(&[
            (||std::any::TypeId::of::<ferrodock_model::core::Alignment>(), "::ferrodock_model::core::Alignment"),
            (||std::any::TypeId::of::<ferrodock_model::core::DockCapability>(), "::ferrodock_model::core::DockCapability"),
            (||std::any::TypeId::of::<ferrodock_model::command_bars::DockCommandBarKind>(), "::ferrodock_model::command_bars::DockCommandBarKind"),
            (||std::any::TypeId::of::<ferrodock_model::command_bars::DockCommandBarMergeMode>(), "::ferrodock_model::command_bars::DockCommandBarMergeMode"),
            (||std::any::TypeId::of::<ferrodock_model::core::DockFloatingWindowHostMode>(), "::ferrodock_model::core::DockFloatingWindowHostMode"),
            (||std::any::TypeId::of::<ferrodock_model::core::DockMode>(), "::ferrodock_model::core::DockMode"),
            (||std::any::TypeId::of::<ferrodock_model::core::DockOperation>(), "::ferrodock_model::core::DockOperation"),
            (||std::any::TypeId::of::<ferrodock_model::core::DockWindowOwnerMode>(), "::ferrodock_model::core::DockWindowOwnerMode"),
            (||std::any::TypeId::of::<ferrodock_model::core::DockWindowState>(), "::ferrodock_model::core::DockWindowState"),
            (||std::any::TypeId::of::<ferrodock_model::core::DockingWindowState>(), "::ferrodock_model::core::DockingWindowState"),
            (||std::any::TypeId::of::<ferrodock_model::core::DocumentCloseButtonShowMode>(), "::ferrodock_model::core::DocumentCloseButtonShowMode"),
            (||std::any::TypeId::of::<ferrodock_model::core::DocumentLayoutMode>(), "::ferrodock_model::core::DocumentLayoutMode"),
            (||std::any::TypeId::of::<ferrodock_model::core::DocumentTabLayout>(), "::ferrodock_model::core::DocumentTabLayout"),
            (||std::any::TypeId::of::<ferrodock_model::core::GridResizeDirection>(), "::ferrodock_model::core::GridResizeDirection"),
            (||std::any::TypeId::of::<ferrodock_model::core::GripMode>(), "::ferrodock_model::core::GripMode"),
            (||std::any::TypeId::of::<ferrodock_model::core::MdiWindowState>(), "::ferrodock_model::core::MdiWindowState"),
            (||std::any::TypeId::of::<ferrodock_model::core::Orientation>(), "::ferrodock_model::core::Orientation"),
            (||std::any::TypeId::of::<ferrodock_model::core::PinnedDockDisplayMode>(), "::ferrodock_model::core::PinnedDockDisplayMode"),
            (||std::any::TypeId::of::<ferrodock_model::core::TrackingMode>(), "::ferrodock_model::core::TrackingMode"),
            (||std::any::TypeId::of::<ferrodock_model::core::SplitViewDisplayMode>(), "::ferrodock_model::core::SplitViewDisplayMode"),
            (||std::any::TypeId::of::<ferrodock_model::core::SplitViewPanePlacement>(), "::ferrodock_model::core::SplitViewPanePlacement"),
        ]);
        ferroui_base::metadata::register_type_rust_paths(&[(|| std::any::TypeId::of::<ferrodock_model::core::DockTrackingChangeReason>(), "::ferrodock_model::core::DockTrackingChangeReason")]);
        ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<ferrodock_model::core::DockCapabilityValueSource>(),"::ferrodock_model::core::DockCapabilityValueSource")]);
        MarkupType::register_rust_paths(&[
            (DockCapabilityValueSourceMetadata::MARKUP,"::ferrodock_model_ferro::DockCapabilityValueSourceMetadata",false),
            (DockTrackingChangeReasonMetadata::MARKUP, "::ferrodock_model_ferro::DockTrackingChangeReasonMetadata", false),
            (AlignmentMetadata::MARKUP, "::ferrodock_model_ferro::AlignmentMetadata", false),
            (DockCapabilityMetadata::MARKUP, "::ferrodock_model_ferro::DockCapabilityMetadata", false),
            (DockCommandBarKindMetadata::MARKUP, "::ferrodock_model_ferro::DockCommandBarKindMetadata", false),
            (DockCommandBarMergeModeMetadata::MARKUP, "::ferrodock_model_ferro::DockCommandBarMergeModeMetadata", false),
            (DockFloatingWindowHostModeMetadata::MARKUP, "::ferrodock_model_ferro::DockFloatingWindowHostModeMetadata", false),
            (DockModeMetadata::MARKUP, "::ferrodock_model_ferro::DockModeMetadata", false),
            (DockOperationMetadata::MARKUP, "::ferrodock_model_ferro::DockOperationMetadata", false),
            (DockWindowOwnerModeMetadata::MARKUP, "::ferrodock_model_ferro::DockWindowOwnerModeMetadata", false),
            (DockWindowStateMetadata::MARKUP, "::ferrodock_model_ferro::DockWindowStateMetadata", false),
            (DockingWindowStateMetadata::MARKUP, "::ferrodock_model_ferro::DockingWindowStateMetadata", false),
            (DocumentCloseButtonShowModeMetadata::MARKUP, "::ferrodock_model_ferro::DocumentCloseButtonShowModeMetadata", false),
            (DocumentLayoutModeMetadata::MARKUP, "::ferrodock_model_ferro::DocumentLayoutModeMetadata", false),
            (DocumentTabLayoutMetadata::MARKUP, "::ferrodock_model_ferro::DocumentTabLayoutMetadata", false),
            (GridResizeDirectionMetadata::MARKUP, "::ferrodock_model_ferro::GridResizeDirectionMetadata", false),
            (GripModeMetadata::MARKUP, "::ferrodock_model_ferro::GripModeMetadata", false),
            (MdiWindowStateMetadata::MARKUP, "::ferrodock_model_ferro::MdiWindowStateMetadata", false),
            (OrientationMetadata::MARKUP, "::ferrodock_model_ferro::OrientationMetadata", false),
            (PinnedDockDisplayModeMetadata::MARKUP, "::ferrodock_model_ferro::PinnedDockDisplayModeMetadata", false),
            (TrackingModeMetadata::MARKUP, "::ferrodock_model_ferro::TrackingModeMetadata", false),
            (SplitViewDisplayModeMetadata::MARKUP, "::ferrodock_model_ferro::SplitViewDisplayModeMetadata", false),
            (SplitViewPanePlacementMetadata::MARKUP, "::ferrodock_model_ferro::SplitViewPanePlacementMetadata", false),
        ]);
    }
    MarkupType::register_all(&[
        DockCapabilityValueSourceMetadata::MARKUP,
        DockTrackingChangeReasonMetadata::MARKUP,
        AlignmentMetadata::MARKUP,
        DockCapabilityMetadata::MARKUP,
        DockCommandBarKindMetadata::MARKUP,
        DockCommandBarMergeModeMetadata::MARKUP,
        DockFloatingWindowHostModeMetadata::MARKUP,
        DockModeMetadata::MARKUP,
        DockOperationMetadata::MARKUP,
        DockWindowOwnerModeMetadata::MARKUP,
        DockWindowStateMetadata::MARKUP,
        DockingWindowStateMetadata::MARKUP,
        DocumentCloseButtonShowModeMetadata::MARKUP,
        DocumentLayoutModeMetadata::MARKUP,
        DocumentTabLayoutMetadata::MARKUP,
        GridResizeDirectionMetadata::MARKUP,
        GripModeMetadata::MARKUP,
        MdiWindowStateMetadata::MARKUP,
        OrientationMetadata::MARKUP,
        PinnedDockDisplayModeMetadata::MARKUP,
        TrackingModeMetadata::MARKUP,
        SplitViewDisplayModeMetadata::MARKUP,
        SplitViewPanePlacementMetadata::MARKUP,
    ]);
    MarkupAssembly::register(&ASSEMBLY);
}

mod factory_paths {
    ferroui_base::ferro_rust_paths! {classes:[],types:[crate::DockCapabilityResolverMetadata,crate::FactoryExtensionsMetadata,crate::FactoryBaseReference,crate::DockableControlMapReference,crate::DockableObjectMapReference],contracts:[crate::IDockWindowHostFactoryMetadata,crate::IDocumentDockFactoryMetadata,crate::IToolTemplateMetadata,crate::IToolDockContentMetadata,crate::DockWindowListMetadata,crate::IDockDockMetadata,crate::IDocumentContentMetadata,crate::IDocumentDockContentMetadata,crate::IDocumentDockMetadata,crate::IDocumentMetadata,crate::IDocumentTemplateMetadata,crate::IGridDockMetadata,crate::IGridDockSplitterMetadata,crate::IProportionalDockMetadata,crate::IProportionalDockSplitterMetadata,crate::IRootDockMetadata,crate::ISplitViewDockMetadata,crate::ISplitterMetadata,crate::IStackDockMetadata,crate::IToolContentMetadata,crate::IToolDockMetadata,crate::IToolMetadata,crate::IUniformGridDockMetadata,crate::IWrapDockMetadata,crate::IMdiDocumentMetadata,crate::ILocalTargetMetadata,crate::IGlobalTargetMetadata,crate::IDockItemContainerMetadataDescriptor,crate::IHostWindowMetadata,crate::IHostWindowStateMetadata,crate::IDockManagerStateMetadata,crate::IDockableMetadata,crate::IDockMetadata,crate::DockableListMetadata,crate::DockWindowOptionsMetadata,crate::DockCapabilityEvaluationMetadata,crate::IDockWindowMetadata,crate::IDockCapabilityPolicyMetadata,crate::DockCapabilityPolicyMetadata,crate::DockCapabilityOverridesMetadata,crate::IFactoryMetadata,crate::DockControlListMetadata,crate::HostWindowListMetadata],generics:[],generic_contracts:[]}
    pub(super) fn register() {
        register_rust_paths();
    }
}
fn register_factory_rust_paths() {
    factory_paths::register();
}
