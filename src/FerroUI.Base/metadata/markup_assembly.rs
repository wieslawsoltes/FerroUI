//! Assembly-level markup metadata: the counterpart of the
//! `XmlnsDefinition` and `XmlnsPrefix` assembly attributes of the managed
//! original (`Metadata/XmlnsDefinitionAttribute.cs`,
//! `Metadata/XmlnsPrefixAttribute.cs`).

use std::sync::{OnceLock, RwLock};

/// The XML namespace of the framework: the default namespace of its markup
/// documents.
pub const FERRO_XML_NAMESPACE: &str = "https://github.com/ferroui";

/// Maps an XML namespace to a dotted namespace whose types it contains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XmlnsDefinition {
    /// The URL of the XML namespace.
    pub xml_namespace: &'static str,
    /// The dotted namespace (`FerroUI.Controls`).
    pub namespace: &'static str,
}

/// Recommends a prefix for an XML namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XmlnsPrefix {
    /// The URL of the XML namespace.
    pub xml_namespace: &'static str,
    /// The recommended prefix.
    pub prefix: &'static str,
}

/// What a crate states about itself for markup: the assembly name its
/// types are addressed by (`using:Ns;assembly=FerroUI.Controls`, asset
/// URIs) and its XML namespace definitions.
///
/// A crate declares one as a constant and registers it from its
/// `register_types()` function. A type belongs to the assembly whose
/// [`crate_name`](Self::crate_name) is the first segment of the module path
/// of the type.
#[derive(Debug, PartialEq, Eq)]
pub struct MarkupAssembly {
    /// The assembly name (`FerroUI.Controls`).
    pub name: &'static str,
    /// The crate, as module paths spell it (`ferroui_controls`).
    pub crate_name: &'static str,
    pub xmlns_definitions: &'static [XmlnsDefinition],
    pub xmlns_prefixes: &'static [XmlnsPrefix],
    /// Key/value metadata of the assembly (the `AssemblyMetadata` attributes
    /// of the managed original). The markup loader reads
    /// [`CREATE_SOURCE_INFO`](Self::CREATE_SOURCE_INFO).
    pub metadata: &'static [(&'static str, &'static str)],
}

impl MarkupAssembly {
    /// The metadata key that states whether markup compiled for the
    /// assembly records source information by default (`"true"`/`"false"`).
    pub const CREATE_SOURCE_INFO: &'static str = "FerroXamlCreateSourceInfo";

    /// The value of the metadata entry `key`.
    pub fn metadata_value(&self, key: &str) -> Option<&'static str> {
        self.metadata.iter().find(|(k, _)| *k == key).map(|(_, value)| *value)
    }

    /// Adds an assembly to the process-wide table of assemblies.
    /// Registering an assembly again does nothing.
    pub fn register(assembly: &'static MarkupAssembly) {
        let mut assemblies = assemblies().write().unwrap_or_else(|e| e.into_inner());
        if !assemblies.iter().any(|a| std::ptr::eq(*a, assembly)) {
            assemblies.push(assembly);
        }
    }

    /// Every registered assembly, in registration order.
    pub fn registered_assemblies() -> Vec<&'static MarkupAssembly> {
        assemblies().read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The registered assembly named `name`.
    pub fn find(name: &str) -> Option<&'static MarkupAssembly> {
        Self::registered_assemblies().into_iter().find(|a| a.name == name)
    }

    /// The registered assembly that declares the module `module_path`.
    pub fn of_module(module_path: &str) -> Option<&'static MarkupAssembly> {
        let crate_name = module_path.split("::").next().unwrap_or(module_path);
        Self::registered_assemblies().into_iter().find(|a| a.crate_name == crate_name)
    }
}

fn assemblies() -> &'static RwLock<Vec<&'static MarkupAssembly>> {
    static ASSEMBLIES: OnceLock<RwLock<Vec<&'static MarkupAssembly>>> = OnceLock::new();
    ASSEMBLIES.get_or_init(Default::default)
}
