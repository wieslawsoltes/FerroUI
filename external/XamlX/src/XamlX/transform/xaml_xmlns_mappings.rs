//! Port of `Transform/XamlXmlnsMappings.cs`.

use std::collections::HashMap;
use std::rc::Rc;

use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::{IXamlAssembly, IXamlTypeSystem, XamlValue};

use super::XamlLanguageTypeMappings;

#[derive(Default)]
pub struct XamlXmlnsMappings {
    /// XML namespace -> list of (assembly, CLR namespace).
    pub namespaces: HashMap<String, Vec<(Rc<dyn IXamlAssembly>, String)>>,
}

impl XamlXmlnsMappings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn resolve<S: IXamlTypeSystem + ?Sized>(
        type_system: &S,
        type_mappings: &XamlLanguageTypeMappings,
    ) -> XamlResult<Self> {
        let mut rv = XamlXmlnsMappings::new();
        for asm in type_system.assemblies() {
            for attr in asm.custom_attributes() {
                for xmlns_type in &type_mappings.xmlns_attributes {
                    if attr.type_().equals(&**xmlns_type) {
                        let parameters = attr.parameters();
                        match parameters.as_slice() {
                            [XamlValue::String(xmlns), XamlValue::String(clrns)] => {
                                rv.namespaces
                                    .entry(xmlns.clone())
                                    .or_default()
                                    .push((asm.clone(), clrns.clone()));
                            }
                            _ => {
                                return Err(XamlError::parse_exception_at(
                                    format!(
                                        "Unexpected parameters for {} declared on assembly {}",
                                        xmlns_type.get_fqn(),
                                        asm.name()
                                    ),
                                    0,
                                    0,
                                ))
                            }
                        }
                        break;
                    }
                }
            }
        }
        Ok(rv)
    }
}
