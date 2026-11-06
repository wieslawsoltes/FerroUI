//! Port of `XamlIl/Runtime/IFerroXamlIlXmlNamespaceInfoProviderV1.cs`.

use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::ferro_markup_type;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The XML namespaces in scope: prefix (empty for the default namespace) to
/// the namespaces it maps to.
pub type XmlNamespaces = Rc<HashMap<String, Vec<Rc<FerroXamlIlXmlNamespaceInfo>>>>;

/// Provides the XML namespaces in scope at the element being built.
pub trait IFerroXamlIlXmlNamespaceInfoProvider {
    fn xml_namespaces(&self) -> XmlNamespaces;
}

impl PartialEq for dyn IFerroXamlIlXmlNamespaceInfoProvider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

/// The types of a namespace that the compiler resolved for a compiled document: each by
/// the name the document gives it, with the function that yields it as a cast target.
pub type XmlNamespaceTypes = &'static [(&'static str, fn() -> CastTarget)];

/// Where the types of an XML namespace live: a dotted namespace in an
/// assembly.
// Deviation (DEVIATIONS.md, XAML namespace declarations): a compiled document also gives the
// entry the types of the namespace its reflection-binding paths name, so that the type resolver
// finds them without the process-wide tables of types.
#[derive(Debug, Default)]
pub struct FerroXamlIlXmlNamespaceInfo {
    clr_namespace: RefCell<String>,
    clr_assembly_name: RefCell<String>,
    types: XmlNamespaceTypes,
}

impl PartialEq for FerroXamlIlXmlNamespaceInfo {
    /// Two entries are equal when their namespaces, their assemblies and the names of their
    /// resolved types are; the functions of the types are not compared.
    fn eq(&self, other: &Self) -> bool {
        self.clr_namespace == other.clr_namespace
            && self.clr_assembly_name == other.clr_assembly_name
            && self.types.len() == other.types.len()
            && self.types.iter().zip(other.types).all(|(one, other)| one.0 == other.0)
    }
}

impl FerroXamlIlXmlNamespaceInfo {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Creates an entry with both members set.
    pub fn with(clr_namespace: &str, clr_assembly_name: &str) -> Rc<Self> {
        Self::with_types(clr_namespace, clr_assembly_name, &[])
    }

    /// Creates an entry with both members set and the types of the namespace the compiler
    /// resolved for the document.
    pub fn with_types(clr_namespace: &str, clr_assembly_name: &str, types: XmlNamespaceTypes) -> Rc<Self> {
        Rc::new(Self {
            clr_namespace: RefCell::new(clr_namespace.to_string()),
            clr_assembly_name: RefCell::new(clr_assembly_name.to_string()),
            types,
        })
    }

    /// The types of the namespace the compiler resolved for a compiled document; empty for
    /// an entry of the run-time loader.
    pub fn types(&self) -> XmlNamespaceTypes {
        self.types
    }

    /// The type named `name` among [`types`](Self::types), as a cast target.
    pub fn find_type(&self, name: &str) -> Option<CastTarget> {
        self.types.iter().find(|(type_name, _)| *type_name == name).map(|(_, target)| target())
    }

    /// The dotted namespace (`FerroUI.Controls`).
    pub fn clr_namespace(&self) -> String {
        self.clr_namespace.borrow().clone()
    }

    pub fn set_clr_namespace(&self, value: String) {
        *self.clr_namespace.borrow_mut() = value;
    }

    /// The assembly name (`FerroUI.Controls`); empty when unknown.
    pub fn clr_assembly_name(&self) -> String {
        self.clr_assembly_name.borrow().clone()
    }

    pub fn set_clr_assembly_name(&self, value: String) {
        *self.clr_assembly_name.borrow_mut() = value;
    }
}

ferro_markup_type!(interface dyn IFerroXamlIlXmlNamespaceInfoProvider as "IFerroXamlIlXmlNamespaceInfoProvider" {
    this: Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>,
    handles: [
        Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>,
        Option<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>,
    ],
    properties: [
        XmlNamespaces: XmlNamespaces {
            get: |t: &Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>| t.xml_namespaces()
        },
    ],
});

ferro_markup_type!(class FerroXamlIlXmlNamespaceInfo {
    this: Rc<FerroXamlIlXmlNamespaceInfo>,
    handles: [
        FerroXamlIlXmlNamespaceInfo,
        Rc<FerroXamlIlXmlNamespaceInfo>,
        Option<Rc<FerroXamlIlXmlNamespaceInfo>>,
    ],
    constructors: [() => FerroXamlIlXmlNamespaceInfo::new],
    properties: [
        ClrNamespace: String {
            get: |t: &Rc<FerroXamlIlXmlNamespaceInfo>| t.clr_namespace(),
            set: |t: &Rc<FerroXamlIlXmlNamespaceInfo>, value: String| t.set_clr_namespace(value)
        },
        ClrAssemblyName: String {
            get: |t: &Rc<FerroXamlIlXmlNamespaceInfo>| t.clr_assembly_name(),
            set: |t: &Rc<FerroXamlIlXmlNamespaceInfo>, value: String| t.set_clr_assembly_name(value)
        },
    ],
});
