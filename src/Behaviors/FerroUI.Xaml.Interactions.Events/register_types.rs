//! The type table of this crate: its namespace, its classes and what it states about itself for
//! markup (the `XmlnsDefinition` of `Properties/AssemblyInfo.cs`, which maps the namespace of the
//! crate into the default XML namespace of the framework, so documents name the triggers without
//! a prefix).

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespace of the modules of this crate (the namespace of the reference, with the
/// name of the framework).
const NAMESPACES: &[(&str, &str)] = &[("ferroui_xaml_interactions_events", "FerroUI.Xaml.Interactions.Events")];

/// What this crate states about itself for markup: its assembly name and the namespace the XML
/// namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Xaml.Interactions.Events",
    crate_name: "ferroui_xaml_interactions_events",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroUI.Xaml.Interactions.Events",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

pub(crate) const TYPES: &[&TypeInfo] = types![
    crate::DoubleTappedEventBehavior,
    crate::DoubleTappedEventTrigger,
    crate::DragEnterEventTrigger,
    crate::DragLeaveEventTrigger,
    crate::DragOverEventTrigger,
    crate::DropEventTrigger,
    crate::GotFocusEventBehavior,
    crate::GotFocusEventTrigger,
    crate::KeyDownEventBehavior,
    crate::KeyDownEventTrigger,
    crate::KeyUpEventBehavior,
    crate::KeyUpEventTrigger,
    crate::LostFocusEventBehavior,
    crate::LostFocusEventTrigger,
    crate::PointerCaptureLostEventBehavior,
    crate::PointerCaptureLostEventTrigger,
    crate::PointerEnteredEventBehavior,
    crate::PointerEnteredEventTrigger,
    crate::PointerEventsBehavior,
    crate::PointerEventsTrigger,
    crate::PointerExitedEventBehavior,
    crate::PointerExitedEventTrigger,
    crate::PointerMovedEventBehavior,
    crate::PointerMovedEventTrigger,
    crate::PointerPressedEventBehavior,
    crate::PointerPressedEventTrigger,
    crate::PointerReleasedEventBehavior,
    crate::PointerReleasedEventTrigger,
    crate::PointerWheelChangedEventBehavior,
    crate::PointerWheelChangedEventTrigger,
    crate::RightTappedEventBehavior,
    crate::RightTappedEventTrigger,
    crate::ScrollGestureEndedEventBehavior,
    crate::ScrollGestureEndedEventTrigger,
    crate::ScrollGestureEventBehavior,
    crate::ScrollGestureEventTrigger,
    crate::TappedEventBehavior,
    crate::TappedEventTrigger,
    crate::TextInputEventBehavior,
    crate::TextInputEventTrigger,
    crate::TextInputMethodClientRequestedEventBehavior,
    crate::TextInputMethodClientRequestedEventTrigger,
];

/// Registers the namespace, the types and the assembly of this crate (and of the crates it is
/// built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The behaviors object model, and with it the controls and the base crate.
        ferroui_xaml_interactivity::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
    });
}
