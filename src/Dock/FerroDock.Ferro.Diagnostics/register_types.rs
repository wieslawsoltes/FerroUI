use ferroui_base::{
    metadata::{MarkupAssembly, XmlnsDefinition},
    TypeInfo,
};
ferroui_base::ferro_rust_paths! {classes:[crate::controls::RootDockDebug,crate::controls::DockableDebugView,crate::controls::DockDebugView,crate::controls::DockWindowDebugView,crate::controls::RootDockInfoDebugView,crate::controls::DockDockDebugView,crate::controls::GridDockDebugView,crate::controls::GridDockSplitterDebugView,crate::controls::ProportionalDockDebugView,crate::controls::ProportionalDockSplitterDebugView,crate::controls::StackDockDebugView,crate::controls::WrapDockDebugView,crate::controls::UniformGridDockDebugView,crate::controls::DocumentDockDebugView,crate::controls::ToolDockDebugView,crate::controls::DocumentContentDebugView,crate::controls::ToolContentDebugView,crate::controls::DocumentTemplateDebugView,crate::controls::DocumentDockContentDebugView], types:[crate::controls::RootDockDebugExtensions,crate::behaviors::RootDockDebugBehavior,crate::DockDebugOverlayExtensions], contracts:[], generics:[], generic_contracts:[]}

const NAMESPACES: &[(&str, &str)] = &[
    (
        "ferrodock_ferro_diagnostics::behaviors",
        "FerroDock.Ferro.Diagnostics.Behaviors",
    ),
    (
        "ferrodock_ferro_diagnostics::controls",
        "FerroDock.Ferro.Diagnostics.Controls",
    ),
];
/// Original diagnostic assembly identity and XML namespaces.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Ferro.Diagnostics",
    crate_name: "ferrodock_ferro_diagnostics",
    xmlns_definitions: &[
        XmlnsDefinition {
            xml_namespace: "https://github.com/ferroui",
            namespace: "FerroDock.Ferro.Diagnostics",
        },
        XmlnsDefinition {
            xml_namespace: "https://github.com/ferroui",
            namespace: "FerroDock.Ferro.Diagnostics.Controls",
        },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};
/// Registers the original native diagnostics implemented so far.
pub fn register_types() {
    ferrodock_ferro::register_types();
    ferroui_base::data::core::ValueTypes::register_nullable::<ferroui_base::input::KeyGesture>();
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::rc::Rc<ferroui_base::metadata::MarkupDelegate>,
    >();

    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[crate::controls::RootDockDebugExtensions::TYPE]);
    ferroui_base::metadata::MarkupType::register(
        <crate::controls::RootDockDebugExtensions as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );

    crate::behaviors::root_dock_debug_behavior::register_items();
    crate::behaviors::RootDockDebugBehavior::TYPE.ensure_class_init();
    ferroui_base::metadata::MarkupType::register(
        <crate::behaviors::RootDockDebugBehavior as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );

    ferroui_base::metadata::MarkupAssembly::register(&ASSEMBLY);
    ferroui_base::metadata::MarkupType::register(
        <crate::DockDebugOverlayExtensions as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    TypeInfo::register_namespaces(&[(
        "ferrodock_ferro_diagnostics::dock_debug_overlay_extensions",
        "FerroDock.Ferro.Diagnostics",
    )]);
    TypeInfo::register_all(&[crate::DockDebugOverlayExtensions::TYPE]);
    crate::compiled_markup::register();
    TypeInfo::register_all(&[
        crate::controls::RootDockDebug::TYPE,
        crate::controls::DockableDebugView::TYPE,
        crate::controls::DockDebugView::TYPE,
        crate::controls::DockWindowDebugView::TYPE,
        crate::controls::RootDockInfoDebugView::TYPE,
        crate::controls::DockDockDebugView::TYPE,
        crate::controls::GridDockDebugView::TYPE,
        crate::controls::GridDockSplitterDebugView::TYPE,
        crate::controls::ProportionalDockDebugView::TYPE,
        crate::controls::ProportionalDockSplitterDebugView::TYPE,
        crate::controls::StackDockDebugView::TYPE,
        crate::controls::WrapDockDebugView::TYPE,
        crate::controls::UniformGridDockDebugView::TYPE,
        crate::controls::DocumentDockDebugView::TYPE,
        crate::controls::ToolDockDebugView::TYPE,
        crate::controls::DocumentContentDebugView::TYPE,
        crate::controls::ToolContentDebugView::TYPE,
        crate::controls::DocumentTemplateDebugView::TYPE,
        crate::controls::DocumentDockContentDebugView::TYPE,
    ]);
    register_rust_paths();
    TypeInfo::register_namespaces(&[(
        "ferrodock_ferro_diagnostics::controls::selection_overlay_adorner",
        "FerroDock.Ferro.Diagnostics.Controls",
    )]);
    TypeInfo::register_namespaces(&[(
        "ferrodock_ferro_diagnostics::controls::debug_overlay_adorner",
        "FerroDock.Ferro.Diagnostics.Controls",
    )]);
    TypeInfo::register_all(&[
        crate::controls::SelectionOverlayAdorner::TYPE,
        crate::controls::DebugOverlayAdorner::TYPE,
    ]);
}
