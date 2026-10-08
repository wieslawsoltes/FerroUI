//! The type table of the fixture: its namespaces and its assembly.

use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, XmlnsPrefix, FERRO_XML_NAMESPACE};

const OWN_NAMESPACE: &str = "https://example.org/fixture";

const NAMESPACES: &[(&str, &str)] = &[
    ("fixture", "Fixture"),
    ("fixture::controls", "Fixture.Controls"),
    ("fixture::controls::text_block::documents", "Fixture.Controls.Documents"),
    ("fixture::media", "Fixture.Media"),
];

pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "Fixture",
    crate_name: "fixture",
    xmlns_definitions: &[
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "Fixture.Controls" },
        XmlnsDefinition { xml_namespace: OWN_NAMESPACE, namespace: "Fixture.Media" },
        XmlnsDefinition { xml_namespace: computed(), namespace: "Fixture.Lost" },
    ],
    xmlns_prefixes: &[XmlnsPrefix { xml_namespace: OWN_NAMESPACE, prefix: "f" }],
    metadata: &[(MarkupAssembly::CREATE_SOURCE_INFO, "true")],
};
