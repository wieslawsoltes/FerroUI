//! The type table of this crate: its namespaces, its classes, what it
//! states about itself for markup, its embedded assets and its compiled
//! markup.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};

/// The dotted namespaces of the modules of this crate. A type belongs to the
/// namespace of the longest module path that is a prefix of the path of its
/// declaring module.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_controls_color_picker", "FerroUI.Controls"),
    ("ferroui_controls_color_picker::automation::peers", "FerroUI.Automation.Peers"),
    ("ferroui_controls_color_picker::color_previewer", "FerroUI.Controls.Primitives"),
    ("ferroui_controls_color_picker::color_slider", "FerroUI.Controls.Primitives"),
    ("ferroui_controls_color_picker::color_spectrum::color_spectrum", "FerroUI.Controls.Primitives"),
    ("ferroui_controls_color_picker::color_spectrum::color_spectrum_properties", "FerroUI.Controls.Primitives"),
    ("ferroui_controls_color_picker::converters", "FerroUI.Controls.Converters"),
    ("ferroui_controls_color_picker::converters::accent_color_converter", "FerroUI.Controls.Primitives.Converters"),
    ("ferroui_controls_color_picker::converters::contrast_brush_converter", "FerroUI.Controls.Primitives.Converters"),
    ("ferroui_controls_color_picker::helpers", "FerroUI.Controls.Primitives"),
];

/// What this crate states about itself for markup: its assembly name and
/// the namespaces the XML namespace of the framework maps to (the
/// `XmlnsDefinition` attributes of the upstream project).
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.ColorPicker",
    crate_name: "ferroui_controls_color_picker",
    xmlns_definitions: &[
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Collections" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls.Primitives" },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

const TYPES: &[&TypeInfo] = types![
    // FerroUI.Automation.Peers
    crate::automation::peers::ColorSpectrumAutomationPeer,
    // FerroUI.Controls
    crate::color_picker::ColorPicker,
    crate::color_view::ColorView,
    // FerroUI.Controls.Primitives
    crate::color_previewer::ColorPreviewer,
    crate::color_slider::ColorSlider,
    crate::color_spectrum::ColorSpectrum,
];

/// Registers the namespaces, the types, the assembly and the compiled
/// markup of this crate (and of the crates it is built on), and with the
/// feature `document-assets` the theme documents as assets. Cheap and
/// idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled
        // documents create), and with it the controls and the base crate.
        ferroui_markup_xaml::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
        crate::markup_types::register();
        // The theme documents as assets: what lets a document the run-time loader loads
        // include them (the tests of this crate do).
        #[cfg(any(test, feature = "document-assets"))]
        crate::assets::register();
        // The loader table of the compiled markup, which the build of the crate generates: a
        // load of a theme document by its URI builds it from its compiled markup.
        crate::compiled_markup::register();
    });
}

#[cfg(test)]
mod tests {
    // Not from upstream: checks the type table of the crate.
    use super::*;
    use crate::automation::peers::ColorSpectrumAutomationPeer;
    use crate::primitives::ColorSpectrum;
    use crate::ColorView;

    #[test]
    fn every_type_of_this_crate_has_a_namespace() {
        register_types();

        for type_ in TYPES {
            assert!(type_.namespace().starts_with("FerroUI"), "{type_} has no namespace");
        }
        assert_eq!("FerroUI.Controls.Primitives", ColorSpectrum::TYPE.namespace());
        assert_eq!("FerroUI.Controls", ColorView::TYPE.namespace());
        assert_eq!("FerroUI.Automation.Peers", ColorSpectrumAutomationPeer::TYPE.namespace());
        assert!(TypeInfo::find("FerroUI.Controls", "ColorPicker").is_some());
    }
}
