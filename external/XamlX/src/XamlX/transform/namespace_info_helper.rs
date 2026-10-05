//! Port of `Transform/NamespaceInfoHelper.cs`.

use std::rc::Rc;

use crate::type_system::IXamlAssembly;

use super::TransformerConfiguration;

pub struct NamespaceResolveResult {
    pub clr_namespace: String,
    pub assembly: Option<Rc<dyn IXamlAssembly>>,
    pub assembly_name: Option<String>,
}

impl NamespaceResolveResult {
    pub fn new(clr_namespace: impl Into<String>) -> Self {
        Self {
            clr_namespace: clr_namespace.into(),
            assembly: None,
            assembly_name: None,
        }
    }
}

pub struct NamespaceInfoHelper;

impl NamespaceInfoHelper {
    pub fn try_resolve(
        config: &TransformerConfiguration,
        xmlns: Option<&str>,
    ) -> Option<Vec<NamespaceResolveResult>> {
        let xmlns = xmlns?;
        if let Some(lst) = config.xmlns_mappings.namespaces.get(xmlns) {
            return Some(
                lst.iter()
                    .map(|(asm, ns)| NamespaceResolveResult {
                        clr_namespace: ns.clone(),
                        assembly: Some(asm.clone()),
                        assembly_name: None,
                    })
                    .collect(),
            );
        }

        const CLR_NAMESPACE: &str = "clr-namespace:";
        const ASSEMBLY_NAME_PREFIX: &str = ";assembly=";

        if let Some(ns) = xmlns.strip_prefix(CLR_NAMESPACE) {
            let mut ns = ns;
            let mut asm = config.default_assembly.as_ref().map(|a| a.name());
            if let Some(index_of_assembly_prefix) = ns.find(ASSEMBLY_NAME_PREFIX) {
                asm = Some(
                    ns[index_of_assembly_prefix + ASSEMBLY_NAME_PREFIX.len()..]
                        .trim_matches(char::is_whitespace)
                        .to_string(),
                );
                ns = &ns[..index_of_assembly_prefix];
            }
            return Some(vec![NamespaceResolveResult {
                clr_namespace: ns.to_string(),
                assembly: None,
                assembly_name: asm,
            }]);
        }

        const USING_PREFIX: &str = "using:";
        if let Some(ns) = xmlns.strip_prefix(USING_PREFIX) {
            return Some(vec![NamespaceResolveResult::new(ns)]);
        }

        None
    }
}
