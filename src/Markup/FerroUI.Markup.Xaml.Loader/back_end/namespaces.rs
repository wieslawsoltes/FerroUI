//! The namespace information of a parsed document (`IL/NamespaceInfoProvider.cs`): what
//! the contexts of the run-time loader serve as the namespace information provider, and
//! what the emitter of Rust source writes as the table generated code gives its contexts.

use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

use xamlx::ast::XamlDocument;
use xamlx::transform::{NamespaceInfoHelper, TransformerConfiguration};

/// The CLR namespace an XML namespace prefix of a document maps to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XamlXmlNamespaceInfo {
    pub clr_namespace: String,
    pub clr_assembly_name: Option<String>,
}

/// The namespace information of a document: for each XML namespace prefix
/// the namespaces (and assemblies) it resolves to. Created lazily, once per
/// document.
pub struct XmlNamespaceInfoProvider {
    configuration: Rc<TransformerConfiguration>,
    aliases: Vec<(String, String)>,
    namespaces: OnceCell<Rc<HashMap<String, Vec<XamlXmlNamespaceInfo>>>>,
}

impl XmlNamespaceInfoProvider {
    pub fn new(configuration: Rc<TransformerConfiguration>, document: &XamlDocument) -> Rc<Self> {
        Rc::new(Self {
            configuration,
            aliases: document.namespace_aliases.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            namespaces: OnceCell::new(),
        })
    }

    /// The XML namespace prefixes of the document and what they resolve to.
    pub fn xml_namespaces(&self) -> Rc<HashMap<String, Vec<XamlXmlNamespaceInfo>>> {
        self.namespaces
            .get_or_init(|| {
                let mut namespaces = HashMap::with_capacity(self.aliases.len());
                for (alias, xmlns) in &self.aliases {
                    let resolved = NamespaceInfoHelper::try_resolve(&self.configuration, Some(xmlns.as_str())).unwrap_or_default();
                    let infos = resolved
                        .into_iter()
                        .map(|r| XamlXmlNamespaceInfo {
                            clr_assembly_name: r.assembly_name.or_else(|| r.assembly.map(|a| a.name())),
                            clr_namespace: r.clr_namespace,
                        })
                        .collect();
                    namespaces.insert(alias.clone(), infos);
                }
                Rc::new(namespaces)
            })
            .clone()
    }
}
