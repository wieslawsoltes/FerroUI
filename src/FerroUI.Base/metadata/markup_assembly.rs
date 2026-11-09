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

    /// The metadata key that states whether `{Binding}` in markup of the
    /// assembly is a compiled binding by default (`"True"`/`"False"`): the
    /// `FerroUseCompiledBindingsByDefault` entry the build task of upstream writes
    /// into every assembly it compiles markup for, with the value of its
    /// `DefaultCompileBindings`. The previewer of the designer reads it.
    pub const USE_COMPILED_BINDINGS_BY_DEFAULT: &'static str = "FerroUseCompiledBindingsByDefault";

    /// The value of the metadata entry `key`.
    pub fn metadata_value(&self, key: &str) -> Option<&'static str> {
        self.metadata.iter().find(|(k, _)| *k == key).map(|(_, value)| *value)
    }

    /// Whether `{Binding}` in markup of the assembly is a compiled binding by default, as the
    /// build compiles it: the entry [`USE_COMPILED_BINDINGS_BY_DEFAULT`](Self::USE_COMPILED_BINDINGS_BY_DEFAULT)
    /// (`true` for `"True"` in any casing, with surrounding white space, and `false` for any
    /// other value), and compiled bindings, the default of the build (#603), without the entry.
    ///
    /// Not from upstream (#604): upstream reads `false` without the entry, which its build
    /// always writes. The previewer of the designer and `RuntimeXamlLoaderConfiguration::for_assembly`
    /// read this, so a previewed or reloaded document binds the way the built application does.
    pub fn compiles_bindings_by_default(&self) -> bool {
        self.metadata_value(Self::USE_COMPILED_BINDINGS_BY_DEFAULT)
            .is_none_or(|value| value.trim().eq_ignore_ascii_case("true"))
    }

    /// Adds an assembly to the process-wide table of assemblies.
    /// Registering an assembly again does nothing.
    ///
    /// Always registers. An application registers its own assembly with this
    /// function; a crate of the framework registers its assembly with
    /// [`register_table`](Self::register_table).
    #[inline(always)]
    pub fn register(assembly: &'static MarkupAssembly) {
        Self::register_tables(assembly);
    }

    /// Registers the assembly of a crate of the framework, at start-up from
    /// the crate's `register_types()` function; as
    /// [`register`](Self::register). Without the feature
    /// `type-tables` the table is trimmed: this does nothing, and what only
    /// the table reaches is left out of the binary. An application registers
    /// its own assembly with [`register`](Self::register), which always
    /// registers.
    #[cfg(any(feature = "type-tables", test))]
    #[inline(always)]
    pub fn register_table(assembly: &'static MarkupAssembly) {
        Self::register_tables(assembly);
    }

    /// Trimmed (without `type-tables`): the table is not kept.
    #[cfg(not(any(feature = "type-tables", test)))]
    #[inline(always)]
    pub fn register_table(assembly: &'static MarkupAssembly) {
        let _ = assembly;
    }

    fn register_tables(assembly: &'static MarkupAssembly) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// An assembly with `metadata`.
    const fn assembly(metadata: &'static [(&'static str, &'static str)]) -> MarkupAssembly {
        MarkupAssembly { name: "Tests.Bindings", crate_name: "tests_bindings", xmlns_definitions: &[], xmlns_prefixes: &[], metadata }
    }

    /// Not from upstream (#604): an assembly compiles its bindings unless its metadata says
    /// otherwise, as the build does.
    #[test]
    fn bindings_compile_by_default_unless_the_metadata_says_otherwise() {
        assert!(assembly(&[]).compiles_bindings_by_default());
        assert!(assembly(&[("Other", "False")]).compiles_bindings_by_default());
        assert!(assembly(&[(MarkupAssembly::USE_COMPILED_BINDINGS_BY_DEFAULT, "True")]).compiles_bindings_by_default());
        assert!(assembly(&[(MarkupAssembly::USE_COMPILED_BINDINGS_BY_DEFAULT, " true ")]).compiles_bindings_by_default());
        assert!(!assembly(&[(MarkupAssembly::USE_COMPILED_BINDINGS_BY_DEFAULT, "False")]).compiles_bindings_by_default());
        // A value that is not a boolean is `false`, as `bool.TryParse(..) && value` reads it.
        assert!(!assembly(&[(MarkupAssembly::USE_COMPILED_BINDINGS_BY_DEFAULT, "yes")]).compiles_bindings_by_default());
    }
}
