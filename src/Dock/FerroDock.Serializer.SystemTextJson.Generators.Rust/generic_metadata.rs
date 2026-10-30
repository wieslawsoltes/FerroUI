// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{RustGenericParameterKind, RustSourceDeclaration};
use ferrodock_serializer_system_text_json_generators::CompilationError;
use ferroui_build_scan::model::{CallForm, CallableModel, RustType, TypeModel};
use quote::ToTokens;
use std::collections::HashMap;

fn error(message: impl Into<String>) -> CompilationError {
    CompilationError(message.into())
}
pub(crate) fn concrete_argument(ty: &syn::Type) -> Result<(), CompilationError> {
    use syn::visit::Visit;
    struct Check(bool);
    impl<'ast> Visit<'ast> for Check {
        fn visit_type(&mut self, t: &'ast syn::Type) {
            if matches!(
                t,
                syn::Type::Infer(_)
                    | syn::Type::Macro(_)
                    | syn::Type::ImplTrait(_)
                    | syn::Type::Verbatim(_)
            ) {
                self.0 = false;
            }
            syn::visit::visit_type(self, t);
        }
    }
    let mut check = Check(true);
    check.visit_type(ty);
    if check.0 {
        Ok(())
    } else {
        Err(error(
            "Generic argument requires concrete compiler type facts",
        ))
    }
}

pub(crate) fn named_arguments(
    type_: &RustType,
) -> Result<(String, Vec<syn::Type>), CompilationError> {
    let syn::Type::Path(mut ty) = syn::parse_str::<syn::Type>(&type_.text)
        .map_err(|e| error(format!("Invalid generic type {}: {e}", type_.text)))?
    else {
        return Err(error("Generic metadata requires a named Rust type"));
    };
    if ty.qself.is_some() || ty.path.segments.is_empty() {
        return Err(error(
            "Associated generic types require compiler resolution",
        ));
    }
    let last = ty.path.segments.len() - 1;
    let mut args = Vec::new();
    for (index, segment) in ty.path.segments.iter_mut().enumerate() {
        let arguments = std::mem::replace(&mut segment.arguments, syn::PathArguments::None);
        match arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(a) if index == last => {
                for arg in a.args {
                    let syn::GenericArgument::Type(t) = arg else {
                        return Err(error("Generic metadata supports type arguments only"));
                    };
                    args.push(t);
                }
            }
            _ => {
                return Err(error(
                    "Nested/associated generic arguments require compiler resolution",
                ))
            }
        }
    }
    let path = ty
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    Ok((path, args))
}
fn parameter_name(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(ty) = ty else {
        return None;
    };
    if ty.qself.is_none()
        && ty.path.leading_colon.is_none()
        && ty.path.segments.len() == 1
        && matches!(ty.path.segments[0].arguments, syn::PathArguments::None)
    {
        Some(ty.path.segments[0].ident.to_string())
    } else {
        None
    }
}
fn contains_parameter(tokens: proc_macro2::TokenStream, map: &HashMap<String, String>) -> bool {
    tokens.into_iter().any(|t| match t {
        proc_macro2::TokenTree::Ident(i) => map.contains_key(&i.to_string()),
        proc_macro2::TokenTree::Group(g) => contains_parameter(g.stream(), map),
        _ => false,
    })
}
fn path_arguments(
    path: &mut syn::Path,
    map: &HashMap<String, String>,
) -> Result<(), CompilationError> {
    if path.leading_colon.is_none()
        && path
            .segments
            .first()
            .is_some_and(|s| map.contains_key(&s.ident.to_string()))
    {
        return Err(error(
            "Associated type/callable on a parameter requires compiler resolution",
        ));
    }
    for segment in &mut path.segments {
        match &mut segment.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(arguments) => {
                for argument in &mut arguments.args {
                    match argument {
                        syn::GenericArgument::Type(ty) => substitute_type(ty, map)?,
                        syn::GenericArgument::AssocType(binding) => {
                            if binding.generics.is_some() {
                                return Err(error(
                                    "Generic associated bindings require compiler resolution",
                                ));
                            }
                            substitute_type(&mut binding.ty, map)?;
                        }
                        syn::GenericArgument::Lifetime(_) => {}
                        other if contains_parameter(other.to_token_stream(), map) => {
                            return Err(error(
                                "Const/constraint substitution requires compiler resolution",
                            ))
                        }
                        _ => {}
                    }
                }
            }
            syn::PathArguments::Parenthesized(arguments) => {
                for ty in &mut arguments.inputs {
                    substitute_type(ty, map)?;
                }
                if let syn::ReturnType::Type(_, ty) = &mut arguments.output {
                    substitute_type(ty, map)?;
                }
            }
        }
    }
    Ok(())
}
fn substitute_type(
    ty: &mut syn::Type,
    map: &HashMap<String, String>,
) -> Result<(), CompilationError> {
    if let Some(name) = parameter_name(ty) {
        if let Some(argument) = map.get(&name) {
            *ty = syn::parse_str(argument)
                .map_err(|e| error(format!("Invalid substituted type: {e}")))?;
            return Ok(());
        }
    }
    match ty {
        syn::Type::Path(p) => {
            if let Some(q) = &mut p.qself {
                substitute_type(&mut q.ty, map)?;
            }
            path_arguments(&mut p.path, map)?;
        }
        syn::Type::Array(t) => {
            substitute_type(&mut t.elem, map)?;
            if contains_parameter(t.len.to_token_stream(), map) {
                return Err(error(
                    "Array length substitution requires compiler resolution",
                ));
            }
        }
        syn::Type::Reference(t) => substitute_type(&mut t.elem, map)?,
        syn::Type::Ptr(t) => substitute_type(&mut t.elem, map)?,
        syn::Type::Slice(t) => substitute_type(&mut t.elem, map)?,
        syn::Type::Paren(t) => substitute_type(&mut t.elem, map)?,
        syn::Type::Group(t) => substitute_type(&mut t.elem, map)?,
        syn::Type::Tuple(t) => {
            for ty in &mut t.elems {
                substitute_type(ty, map)?;
            }
        }
        syn::Type::BareFn(t) => {
            for arg in &mut t.inputs {
                substitute_type(&mut arg.ty, map)?;
            }
            if let syn::ReturnType::Type(_, ty) = &mut t.output {
                substitute_type(ty, map)?;
            }
        }
        syn::Type::TraitObject(t) => {
            for bound in &mut t.bounds {
                if let syn::TypeParamBound::Trait(t) = bound {
                    path_arguments(&mut t.path, map)?;
                }
            }
        }
        syn::Type::ImplTrait(t) => {
            for bound in &mut t.bounds {
                if let syn::TypeParamBound::Trait(t) = bound {
                    path_arguments(&mut t.path, map)?;
                }
            }
        }
        syn::Type::Never(_) => {}
        _ => {
            return Err(error(
                "Opaque/inferred/macro type requires compiler resolution",
            ))
        }
    }
    Ok(())
}
fn callable_path(text: &str, map: &HashMap<String, String>) -> Result<String, CompilationError> {
    let mut expression = syn::parse_str::<syn::ExprPath>(text)
        .map_err(|e| error(format!("Invalid typed callable {text}: {e}")))?;
    if let Some(q) = &mut expression.qself {
        substitute_type(&mut q.ty, map)?;
    }
    path_arguments(&mut expression.path, map)?;
    Ok(expression.to_token_stream().to_string())
}
fn callable(
    value: &mut CallableModel,
    map: &HashMap<String, String>,
) -> Result<(), CompilationError> {
    for text in [
        &mut value.path,
        &mut value.resolved,
        &mut value.dereferenced,
    ]
    .into_iter()
    .flatten()
    {
        *text = callable_path(text, map)?;
    }
    Ok(())
}
fn call_form(
    value: &mut Option<CallForm>,
    map: &HashMap<String, String>,
) -> Result<(), CompilationError> {
    if let Some(CallForm::Path(text) | CallForm::CratePath(text)) = value {
        *text = callable_path(text, map)?;
    }
    Ok(())
}

/// Instantiates a shared generic metadata template using actual closed Rust type arguments.
///
/// Only declared type parameters are substituted, through parsed Rust type syntax.
/// Scoped names, member names and literal attributes retain their own identities.
/// The normalizer must use the template's actual module and complete shared model set.
/// Source cfg, lifetimes/const parameters, ambiguous associated projections and macro
/// expansion remain explicit compiler work and return errors rather than guessed schemas.
/// Generic constraints are retained by the actual Rust definitions and checked when the
/// emitted typed calls compile. The input template is unchanged on success or failure.
pub fn instantiate_generic_metadata(
    template: &TypeModel,
    source: &RustSourceDeclaration,
    constructed: &RustType,
    normalise: impl Fn(&str) -> Option<RustType>,
) -> Result<TypeModel, CompilationError> {
    if !source.cfg.is_empty() || !template.cfg.is_empty() {
        return Err(error("Generic metadata requires target cfg selection"));
    }
    if source.parameters.is_empty()
        || source
            .parameters
            .iter()
            .any(|p| p.kind != RustGenericParameterKind::Type)
    {
        return Err(error(
            "Generic metadata requires explicit type parameters only",
        ));
    }
    if !constructed.is_resolved() {
        return Err(error("Constructed generic type is unresolved"));
    }
    let (definition, args) = named_arguments(constructed)?;
    let (template_definition, template_args) = named_arguments(&template.rust_path)?;
    if definition != source.path.text.trim_start_matches("::")
        || template_definition != definition
        || args.len() != source.parameters.len()
        || template_args.len() != args.len()
    {
        return Err(error(
            "Generic metadata definition/arity differs from actual source",
        ));
    }
    let generic = template
        .generic
        .as_ref()
        .ok_or_else(|| error("Generic metadata has no definition arguments"))?;
    if generic.definition.is_empty() || generic.arguments.len() != args.len() {
        return Err(error(
            "Generic metadata definition arguments are incomplete",
        ));
    }
    let mut map = HashMap::new();
    for (i, (parameter, argument)) in source.parameters.iter().zip(args).enumerate() {
        concrete_argument(&argument)?;
        let metadata_arg = syn::parse_str::<syn::Type>(&generic.arguments[i].text)
            .map_err(|_| error("Invalid generic metadata parameter"))?;
        if parameter_name(&template_args[i]).as_deref() != Some(parameter.name.as_str())
            || parameter_name(&metadata_arg).as_deref() != Some(parameter.name.as_str())
        {
            return Err(error(
                "Generic template parameters differ from source order",
            ));
        }
        let argument = normalise(&argument.to_token_stream().to_string())
            .filter(RustType::is_resolved)
            .ok_or_else(|| error("Generic argument requires resolved compiler type facts"))?;
        if argument.text == "_" {
            return Err(error(
                "Inferred generic argument requires compiler resolution",
            ));
        }
        if map.insert(parameter.name.clone(), argument.text).is_some() {
            return Err(error("Duplicate generic parameter"));
        }
    }
    let mut result = template.clone();
    // Preserve the exported spelling: canonical normalization here could turn a
    // public owner path back into its inaccessible declaring module.
    for path in [&mut result.public_path, &mut result.stated_path]
        .into_iter()
        .flatten()
    {
        let mut ty = syn::parse_str::<syn::Type>(path)
            .map_err(|e| error(format!("Invalid generic public path: {e}")))?;
        substitute_type(&mut ty, &map)?;
        *path = ty.to_token_stream().to_string();
    }
    let mut failure = None;
    result.visit_types_mut(&mut |ty| {
        if failure.is_some() {
            return;
        }
        let bind = || -> Result<RustType, CompilationError> {
            let mut syntax = syn::parse_str::<syn::Type>(&ty.text)
                .map_err(|e| error(format!("Invalid metadata type {}: {e}", ty.text)))?;
            substitute_type(&mut syntax, &map)?;
            normalise(&syntax.to_token_stream().to_string())
                .filter(RustType::is_resolved)
                .ok_or_else(|| error("Substituted metadata type remains unresolved"))
        };
        match bind() {
            Ok(value) => *ty = value,
            Err(e) => failure = Some(e),
        }
    });
    if let Some(e) = failure {
        return Err(e);
    }
    result.visit_callables_mut(&mut |value| {
        if failure.is_none() {
            if let Err(e) = callable(value, &map) {
                failure = Some(e);
            }
        }
    });
    for member in result
        .constructors
        .iter_mut()
        .chain(&mut result.methods)
        .chain(&mut result.fields)
        .chain(&mut result.events)
    {
        call_form(&mut member.call, &map)?;
    }
    for property in result
        .properties
        .iter_mut()
        .chain(&mut result.static_properties)
        .chain(&mut result.indexers)
    {
        for accessor in [&mut property.getter, &mut property.setter]
            .into_iter()
            .flatten()
        {
            call_form(&mut accessor.call, &map)?;
        }
    }
    call_form(&mut result.parse_call, &map)?;
    if let Some(e) = failure {
        return Err(e);
    }
    if result.rust_path.text != constructed.text {
        return Err(error("Instantiated metadata does not match requested type"));
    }
    Ok(result)
}
