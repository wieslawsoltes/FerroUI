use crate::behaviors::VisualTreeLifecycleBehavior;
use crate::controls::overlays::{
    BusyOverlayControl, ConfirmationDialogControl, ConfirmationOverlayControl,
    DialogOverlayControl, DialogShellControl, OverlayHost, OverlayLayer, OverlayLayerStyleKeys,
};
use crate::controls::{
    DockAdornerWindow, DockTarget, DockTargetBase, GlobalDockTarget, MdiLayoutPanel,
    RootDockControl,
};
use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition};
use ferroui_base::{StaticType, TypeInfo};
const NAMESPACES: &[(&str, &str)] = &[
    ("ferrodock_ferro::commands", "FerroDock.Ferro.Commands"),
    ("ferrodock_ferro::mdi", "FerroDock.Ferro.Mdi"),
    ("ferrodock_ferro::controls", "FerroDock.Ferro.Controls"),
    (
        "ferrodock_ferro::controls::overlays",
        "FerroDock.Ferro.Controls.Overlays",
    ),
    ("ferrodock_ferro::behaviors", "FerroDock.Ferro.Behaviors"),
];
/// Native docking assembly identity.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Ferro",
    crate_name: "ferrodock_ferro",
    xmlns_definitions: &[
        XmlnsDefinition {
            xml_namespace: "https://github.com/ferroui",
            namespace: "FerroDock.Ferro",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/ferroui",
            namespace: "FerroDock.Ferro.Controls",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/ferroui",
            namespace: "FerroDock.Ferro.Controls.Overlays",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/ferroui",
            namespace: "FerroDock.Ferro.Behaviors",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/ferroui",
            namespace: "FerroDock.Ferro.Converters",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/ferroui",
            namespace: "FerroDock.Ferro.MarkupExtension",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/ferroui",
            namespace: "FerroDock.Ferro.Themes",
        },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[crate::controls::DelayedUniformTabPanel,crate::controls::HostWindowTitleBar,crate::controls::PinnedDockHostPanel,crate::controls::PinnedDockControl,crate::controls::DocumentControl,crate::controls::ToolControl,crate::controls::DockableControl,crate::controls::DockControl,crate::controls::HostWindow,crate::controls::ToolChromeControl,crate::controls::ToolPinItemControl,crate::controls::ToolPinnedControl,crate::controls::DocumentTabStrip,crate::controls::DocumentTabStripItem,crate::controls::ToolTabStrip,crate::controls::ToolTabStripItem,crate::controls::ManagedWindowLayer,crate::controls::MdiDocumentControl,crate::controls::MdiDocumentWindow,crate::controls::PinnedDockWindow,crate::controls::DockCommandBarHost,crate::controls::DockSelectorOverlay,crate::controls::DragPreviewControl,crate::controls::DragPreviewWindow,crate::controls::ToolDockControl,crate::controls::SplitViewDockControl,crate::controls::GridDockControl,crate::controls::DocumentDockControl,crate::controls::DockDockControl,crate::controls::StackDockControl,crate::controls::WrapDockControl,crate::controls::UniformGridDockControl,crate::controls::ProportionalDockControl,crate::controls::DocumentContentControl,crate::controls::ToolContentControl,crate::controls::MdiLayoutPanel,crate::controls::DockAdornerWindow,crate::controls::DockTargetBase,crate::controls::DockTarget,crate::controls::GlobalDockTarget,crate::controls::RootDockControl,crate::controls::overlays::OverlayHost,crate::controls::overlays::OverlayLayer,crate::controls::overlays::BusyOverlayControl,crate::controls::overlays::DialogOverlayControl,crate::controls::overlays::ConfirmationOverlayControl,crate::controls::overlays::DialogShellControl,crate::controls::overlays::ConfirmationDialogControl],types:[crate::converters::TitleOrTypeNameConverter,crate::themes::DockDensityStyle,crate::converters::IntLessThanConverter,crate::converters::EitherNotNullConverter,crate::converters::SplitViewPanePlacementConverter,crate::converters::SplitViewDisplayModeConverter,crate::converters::OrientationConverter,crate::converters::DockModeConverter,crate::converters::AlignmentConverter,crate::converters::GripModeConverters,crate::converters::ReferenceEqualsMultiConverter,crate::converters::DocumentLayoutModeMatchConverter,crate::converters::DocumentTabDockConverter,crate::converters::DocumentTabOrientationConverter,crate::converters::IsMaximizedConverter,crate::commands::ScrollViewerLineCommand,crate::commands::ScrollViewerLineDirection,crate::converters::ScrollViewerScrollButtonVisibilityConverter,crate::converters::BorderFillMarginConverter,crate::converters::DockCapabilityConverters,crate::converters::CanRemoveDockableConverter,crate::converters::DocumentCloseButtonVisibilityConverter,crate::converters::ManagedDockableIsToolWindowConverter,crate::converters::ManagedDockableToolDockConverter,crate::controls::ManagedDockWindowDocument,crate::controls::ManagedWindowDock,crate::command_bars::DefaultDockCommandBarAdapter,crate::command_bars::DockCommandBarManager,crate::selectors::DockSelectorItem,crate::selectors::DockSelectorMode,crate::converters::NotNullConverter,crate::controls::ManagedDockableBase,crate::mdi::MdiLayoutEntry,crate::mdi::ClassicMdiLayoutManager,crate::mdi::MdiResizeDirection,crate::behaviors::VisualTreeLifecycleBehavior,crate::controls::overlays::OverlayLayerCollection,crate::controls::overlays::OverlayControlList,crate::controls::overlays::OverlayLayerStyleKeys],contracts:[crate::command_bars::IDockCommandBarAdapter,crate::command_bars::DockCommandBarDefinitionMetadata,crate::command_bars::DockCommandBarItemMetadata,crate::command_bars::IDockCommandBarProviderMetadata,crate::selectors::IDockSelectorService,crate::mdi::IMdiLayoutManager,crate::controls::IDockAdornerWindowFactory,crate::controls::IDockTarget,crate::controls::IDockAdornerHostProvider,crate::controls::overlays::IOverlayLayer,crate::IDockBusyServiceMetadata,crate::IDockGlobalBusyServiceMetadata,crate::IDockDialogServiceMetadata,crate::IDockGlobalDialogServiceMetadata,crate::IDockConfirmationServiceMetadata,crate::IDockGlobalConfirmationServiceMetadata,crate::IVisualTreeLifecycleMetadata,crate::ICommandMetadata,crate::DialogRequestCollectionMetadata,crate::ConfirmationRequestCollectionMetadata,crate::DialogRequestMetadata,crate::ConfirmationRequestMetadata,crate::IHostOverlayServicesMetadata],generics:[],generic_contracts:[]}
/// Registers the native docking class/contract descriptors currently implemented.
pub fn register_types() {
    ferrodock_model_ferro::register_types();
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::rc::Rc<dyn ferrodock_model::core::IHostWindow>,
    >();
    ferroui_base::metadata::MarkupType::register(<crate::converters::TitleOrTypeNameConverter as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::TitleOrTypeNameConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|converter| converter.clone());

    TypeInfo::register_all(&[crate::controls::DelayedUniformTabPanel::TYPE]);
    ferroui_base::TypeInfo::register_all(&[crate::controls::HostWindowTitleBar::TYPE]);
    ferroui_base::metadata::MarkupType::register_all(&[
        <crate::themes::DockDensityStyle as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <crate::converters::IntLessThanConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
    ]);
    ferroui_base::data::core::ValueTypes::register_nullable::<crate::themes::DockDensityStyle>();
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::IntLessThanConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(
        <crate::converters::EitherNotNullConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::EitherNotNullConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IMultiValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(<crate::converters::SplitViewPanePlacementConverter as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::SplitViewPanePlacementConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(<crate::converters::SplitViewDisplayModeConverter as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::SplitViewDisplayModeConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(
        <crate::converters::OrientationConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::OrientationConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(
        <crate::converters::DockModeConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::DockModeConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(
        <crate::converters::AlignmentConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::AlignmentConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::TypeInfo::register_all(&[crate::controls::PinnedDockHostPanel::TYPE]);
    ferroui_base::TypeInfo::register_all(&[crate::controls::PinnedDockControl::TYPE]);
    ferroui_base::metadata::MarkupType::register_all(&[<crate::converters::GripModeConverters as ferroui_base::metadata::MarkupTyped>::MARKUP,<crate::converters::ReferenceEqualsMultiConverter as ferroui_base::metadata::MarkupTyped>::MARKUP]);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::ReferenceEqualsMultiConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IMultiValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(<crate::converters::DocumentLayoutModeMatchConverter as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::DocumentLayoutModeMatchConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(<crate::converters::DocumentTabDockConverter as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::DocumentTabDockConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(<crate::converters::DocumentTabOrientationConverter as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::DocumentTabOrientationConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(
        <crate::converters::IsMaximizedConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::IsMaximizedConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    use ferroui_base::{
        data::core::ValueTypes,
        metadata::{MarkupType, MarkupTyped},
    };
    MarkupType::register_all(&[
        crate::commands::ScrollViewerLineCommand::MARKUP,
        crate::commands::ScrollViewerLineDirection::MARKUP,
        crate::converters::ScrollViewerScrollButtonVisibilityConverter::MARKUP,
    ]);
    ValueTypes::register_cast::<
        std::rc::Rc<crate::commands::ScrollViewerLineCommand>,
        std::rc::Rc<dyn ferroui_base::input::ICommand>,
    >(|c| c.clone());
    ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::ScrollViewerScrollButtonVisibilityConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IMultiValueConverter>,
    >(|c| c.clone());
    ferroui_base::TypeInfo::register_all(&[
        crate::controls::DocumentControl::TYPE,
        crate::controls::ToolControl::TYPE,
    ]);
    ferroui_base::metadata::MarkupType::register(<crate::converters::OwnerIsToolDockConverter as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::OwnerIsToolDockConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(<crate::converters::DocumentTabLayoutMatchConverter as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::DocumentTabLayoutMatchConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|c| c.clone());
    ferroui_base::metadata::MarkupType::register(<crate::converters::BorderFillMarginConverter as ferroui_base::metadata::MarkupTyped>::MARKUP);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::BorderFillMarginConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IMultiValueConverter>,
    >(|c| c.clone());
    ferroui_base::TypeInfo::register_all(&[crate::controls::DockableControl::TYPE]);
    ferroui_base::metadata::MarkupType::register_all(&[
        <crate::converters::DockCapabilityConverters as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <crate::converters::CanRemoveDockableConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <crate::converters::DocumentCloseButtonVisibilityConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
    ]);
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::CanRemoveDockableConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IMultiValueConverter>,
    >(|c| c.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::DocumentCloseButtonVisibilityConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IMultiValueConverter>,
    >(|c| c.clone());
    crate::dock_model_registration::register();
    crate::controls::register_managed_models();
    crate::selectors::register_types();
    ferroui_base::metadata::MarkupType::register(
        <dyn crate::contract::IExternalDockSurface as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    crate::command_bars::register_types();
    ferroui_base::metadata::MarkupType::register_all(&[
        <crate::selectors::DockSelectorItem as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <crate::selectors::DockSelectorMode as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::selectors::IDockSelectorService as ferroui_base::metadata::MarkupTyped>::MARKUP,
    ]);
    ferroui_base::metadata::MarkupType::register(
        <crate::converters::NotNullConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::rc::Rc<crate::converters::NotNullConverter>,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::NotNullConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|value| value.clone());

    ferroui_base::metadata::MarkupType::register_all(&[
        <crate::converters::ManagedDockableIsToolWindowConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <crate::converters::ManagedDockableToolDockConverter as ferroui_base::metadata::MarkupTyped>::MARKUP,
    ]);
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::rc::Rc<crate::converters::ManagedDockableIsToolWindowConverter>,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::rc::Rc<crate::converters::ManagedDockableToolDockConverter>,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::ManagedDockableIsToolWindowConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|value| value.clone());
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::converters::ManagedDockableToolDockConverter>,
        std::rc::Rc<dyn ferroui_base::data::converters::IValueConverter>,
    >(|value| value.clone());

    ferroui_controls::register_types();
    ferrodock_settings::register_types();
    ferrodock_controls_deferred_content_control::register_types();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[
        <crate::controls::DockControl as StaticType>::TYPE,
        <crate::controls::HostWindow as StaticType>::TYPE,
        <crate::controls::ToolChromeControl as StaticType>::TYPE,
        <crate::controls::ToolPinItemControl as StaticType>::TYPE,
        <crate::controls::ToolPinnedControl as StaticType>::TYPE,
        <crate::controls::DocumentTabStrip as StaticType>::TYPE,
        <crate::controls::DocumentTabStripItem as StaticType>::TYPE,
        <crate::controls::ToolTabStrip as StaticType>::TYPE,
        <crate::controls::ToolTabStripItem as StaticType>::TYPE,
        <crate::controls::ManagedWindowLayer as StaticType>::TYPE,
        <crate::controls::MdiDocumentControl as StaticType>::TYPE,
        <crate::controls::MdiDocumentWindow as StaticType>::TYPE,
        <crate::controls::PinnedDockWindow as StaticType>::TYPE,
        <crate::controls::DockCommandBarHost as StaticType>::TYPE,
        <crate::controls::DockSelectorOverlay as StaticType>::TYPE,
        <crate::controls::ToolDockControl as StaticType>::TYPE,
        <crate::controls::SplitViewDockControl as StaticType>::TYPE,
        <crate::controls::GridDockControl as StaticType>::TYPE,
        <crate::controls::DocumentDockControl as StaticType>::TYPE,
        <crate::controls::DockDockControl as StaticType>::TYPE,
        <crate::controls::StackDockControl as StaticType>::TYPE,
        <crate::controls::WrapDockControl as StaticType>::TYPE,
        <crate::controls::UniformGridDockControl as StaticType>::TYPE,
        <crate::controls::ProportionalDockControl as StaticType>::TYPE,
        <crate::controls::DocumentContentControl as StaticType>::TYPE,
        <crate::controls::ToolContentControl as StaticType>::TYPE,
        <RootDockControl as StaticType>::TYPE,
        <MdiLayoutPanel as StaticType>::TYPE,
        <DockTargetBase as StaticType>::TYPE,
        <DockTarget as StaticType>::TYPE,
        <DockAdornerWindow as StaticType>::TYPE,
        <crate::controls::DragPreviewControl as StaticType>::TYPE,
        <crate::controls::DragPreviewWindow as StaticType>::TYPE,
        <GlobalDockTarget as StaticType>::TYPE,
        <VisualTreeLifecycleBehavior as StaticType>::TYPE,
        <OverlayHost as StaticType>::TYPE,
        <OverlayLayer as StaticType>::TYPE,
        <OverlayLayerStyleKeys as StaticType>::TYPE,
        <BusyOverlayControl as StaticType>::TYPE,
        <DialogOverlayControl as StaticType>::TYPE,
        <ConfirmationOverlayControl as StaticType>::TYPE,
        <DialogShellControl as StaticType>::TYPE,
        <ConfirmationDialogControl as StaticType>::TYPE,
    ]);
    register_rust_paths();
    ferrodock_controls_recycling::register_types();
    crate::contract_metadata::register_command_bridge();
    crate::contract_metadata::register_service_handles();
    crate::request_items::register_types();
    crate::mutable_items_list::register_types();
    register_mdi_types();
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<dyn crate::controls::ManagedDockableBaseImpl>,
        crate::controls::ManagedDockableBaseReference,
    >(|model| crate::controls::ManagedDockableBaseReference::new(model.clone()));
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::rc::Rc<dyn crate::controls::ManagedDockableBaseImpl>,
    >();
    ferroui_base::metadata::MarkupType::register_all(&[
        <dyn crate::IHostOverlayServicesMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::controls::IDockTarget as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <crate::controls::ManagedDockableBase as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <crate::mdi::MdiLayoutEntry as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <crate::mdi::ClassicMdiLayoutManager as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <crate::mdi::MdiResizeDirection as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::mdi::IMdiLayoutManager as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::controls::IDockAdornerWindowFactory as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::controls::IDockAdornerHostProvider as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::DialogRequestMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::ConfirmationRequestMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::IDockBusyServiceMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::IDockGlobalBusyServiceMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::IDockDialogServiceMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::IDockGlobalDialogServiceMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::IDockConfirmationServiceMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::IDockGlobalConfirmationServiceMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::IVisualTreeLifecycleMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::ICommandMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::DialogRequestCollectionMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
        <dyn crate::ConfirmationRequestCollectionMetadata as ferroui_base::metadata::MarkupTyped>::MARKUP,
    ]);
    ferroui_base::metadata::MarkupAssembly::register(&ASSEMBLY);
}

fn register_mdi_types() {
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::rc::Rc<dyn crate::mdi::IMdiLayoutManager>,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::rc::Rc<crate::mdi::ClassicMdiLayoutManager>,
    >();
    ferroui_base::data::core::ValueTypes::register_nullable::<crate::mdi::MdiLayoutEntry>();
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::rc::Rc<dyn ferrodock_model::controls::IMdiDocument>,
    >();
    ferroui_base::data::core::ValueTypes::register_cast::<
        std::rc::Rc<crate::mdi::ClassicMdiLayoutManager>,
        std::rc::Rc<dyn crate::mdi::IMdiLayoutManager>,
    >(|manager| manager.clone());
}
