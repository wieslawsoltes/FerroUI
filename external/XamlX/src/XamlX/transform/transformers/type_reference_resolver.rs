//! Port of `Transform/Transformers/TypeReferenceResolver.cs`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, IXamlLineInfo, XamlAstClrTypeReference, XamlAstNodeExtensions,
    XamlAstXmlTypeReference,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer, NamespaceInfoHelper};
use crate::type_system::IXamlType;
use crate::xaml_namespaces::XamlNamespaces;

pub struct TypeReferenceResolver;

#[derive(Default)]
struct TypeResolverCache {
    cache_dictionary: RefCell<HashMap<(Option<String>, String, bool), Rc<dyn IXamlType>>>,
}

impl TypeReferenceResolver {
    /// `ResolveType(context, xmlns, name, isMarkupExtension, typeArguments, lineInfo)`.
    pub fn resolve_type(
        context: &AstTransformationContext,
        xmlns: Option<&str>,
        name: &str,
        is_markup_extension: bool,
        type_arguments: &[Rc<XamlAstXmlTypeReference>],
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<Rc<XamlAstClrTypeReference>> {
        if type_arguments.is_empty() {
            let cache = context.get_or_create_item::<TypeResolverCache>();
            let cache_key = (
                xmlns.map(str::to_string),
                name.to_string(),
                is_markup_extension,
            );
            let cached = cache.cache_dictionary.borrow().get(&cache_key).cloned();
            if let Some(type_) = cached {
                return Ok(XamlAstClrTypeReference::new(
                    line_info,
                    type_,
                    is_markup_extension,
                ));
            }

            let res = Self::resolve_type_core(
                context,
                xmlns,
                name,
                is_markup_extension,
                type_arguments,
                line_info,
            )?;
            cache
                .cache_dictionary
                .borrow_mut()
                .insert(cache_key, res.type_.clone());
            Ok(res)
        } else {
            Self::resolve_type_core(
                context,
                xmlns,
                name,
                is_markup_extension,
                type_arguments,
                line_info,
            )
        }
    }

    fn resolve_type_core(
        context: &AstTransformationContext,
        xmlns: Option<&str>,
        name: &str,
        is_markup_extension: bool,
        type_arguments: &[Rc<XamlAstXmlTypeReference>],
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<Rc<XamlAstClrTypeReference>> {
        let mut targs: Vec<Rc<dyn IXamlType>> = Vec::with_capacity(type_arguments.len());
        for ta in type_arguments {
            let generic_arguments = ta.generic_arguments.borrow().clone();
            targs.push(
                Self::resolve_type(
                    context,
                    ta.xml_namespace().as_deref(),
                    &ta.name(),
                    false,
                    &generic_arguments,
                    line_info,
                )?
                .type_
                .clone(),
            );
        }

        let attempt = |cb: &dyn Fn(&str) -> Option<Rc<dyn IXamlType>>,
                       xname: &str|
         -> Option<Rc<dyn IXamlType>> {
            let suffix = if type_arguments.is_empty() {
                String::new()
            } else {
                format!("`{}", type_arguments.len())
            };
            if is_markup_extension {
                cb(&format!("{xname}Extension{suffix}")).or_else(|| cb(&format!("{xname}{suffix}")))
            } else {
                cb(&format!("{xname}{suffix}")).or_else(|| cb(&format!("{xname}Extension{suffix}")))
            }
        };

        let configuration = context.configuration();
        let mut found: Option<Rc<dyn IXamlType>> = None;

        // Try to resolve from system
        if xmlns == Some(XamlNamespaces::XAML2006) {
            found = configuration
                .type_system
                .find_type(&format!("System.{name}"));
        }

        if found.is_none() {
            let resolved_namespaces = NamespaceInfoHelper::try_resolve(configuration, xmlns);
            if let Some(resolved_namespaces) = resolved_namespaces.filter(|r| !r.is_empty()) {
                found = attempt(
                    &|formed_name: &str| {
                        for resolved_ns in &resolved_namespaces {
                            let rname = format!("{}.{}", resolved_ns.clr_namespace, formed_name);
                            let mut sub_res: Option<Rc<dyn IXamlType>> = None;
                            if let Some(assembly) = &resolved_ns.assembly {
                                sub_res = assembly.find_type(&rname);
                            } else if let Some(assembly_name) = &resolved_ns.assembly_name {
                                sub_res = configuration
                                    .type_system
                                    .find_type_in_assembly(&rname, assembly_name);
                            } else {
                                for assembly in configuration.type_system.assemblies() {
                                    sub_res = assembly.find_type(&rname);
                                    if sub_res.is_some() {
                                        break;
                                    }
                                }
                            }

                            if sub_res.is_some() {
                                return sub_res;
                            }
                        }

                        None
                    },
                    name,
                );
            }
        }

        if !type_arguments.is_empty() {
            found = match found {
                Some(f) => Some(f.make_generic_type(&targs)?),
                None => None,
            };
        }
        if let Some(found) = found {
            let is_markup_extension = is_markup_extension || found.name().ends_with("Extension");
            return Ok(XamlAstClrTypeReference::new(
                line_info,
                found,
                is_markup_extension,
            ));
        }

        Err(XamlError::transform_exception(
            format!(
                "Unable to resolve type {} from namespace {}",
                name,
                xmlns.unwrap_or("")
            ),
            Some(line_info),
        ))
    }

    /// `ResolveType(context, string xmlName, isMarkupExtension, lineInfo, strict)`.
    pub fn resolve_type_by_xml_name(
        context: &AstTransformationContext,
        xml_name: &str,
        is_markup_extension: bool,
        line_info: &dyn IXamlLineInfo,
        _strict: bool,
    ) -> XamlResult<Rc<XamlAstClrTypeReference>> {
        let (short_ns, name) = match xml_name.split_once(':') {
            Some((ns, name)) => (ns, name),
            None => ("", xml_name),
        };
        let Some(xmlns) = context.try_get_namespace_alias(short_ns) else {
            return Err(XamlError::transform_exception(
                format!("Unable to resolve type namespace alias {short_ns}"),
                Some(line_info),
            ));
        };

        Self::resolve_type(
            context,
            Some(&xmlns),
            name,
            is_markup_extension,
            &[],
            line_info,
        )
    }

    /// `ResolveType(context, XamlAstXmlTypeReference xmlref)`.
    pub fn resolve_type_reference(
        context: &AstTransformationContext,
        xmlref: &XamlAstXmlTypeReference,
    ) -> XamlResult<Rc<XamlAstClrTypeReference>> {
        let generic_arguments = xmlref.generic_arguments.borrow().clone();
        Self::resolve_type(
            context,
            xmlref.xml_namespace().as_deref(),
            &xmlref.name(),
            xmlref.is_markup_extension.get(),
            &generic_arguments,
            xmlref,
        )
    }
}

impl IXamlAstTransformer for TypeReferenceResolver {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(xmlref) = node.cast::<XamlAstXmlTypeReference>() {
            let resolved = Self::resolve_type_reference(context, &xmlref)?;
            return Ok(resolved);
        }

        Ok(node)
    }
}
