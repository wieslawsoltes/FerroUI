// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_serializer_system_text_json_generators::CompilationError;
use ferroui_build_scan::{
    model::{RustType, TypeKind, TypeModel},
    model_set::ModelSet,
    scanner::Scan,
};
use quote::ToTokens;
use std::{collections::HashMap, fs};
use syn::spanned::Spanned;

/// The actual kind of a Rust declaration, separate from its logical source class kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RustDeclarationKind {
    /// A struct; shared metadata determines whether it represents a class or a value.
    Struct,
    /// An enumeration.
    Enum,
    /// A trait; shared metadata may expose its logical interface handle.
    Trait,
    /// A union; no JSON value codec is inferred for it.
    Union,
}
/// Declared Rust visibility, before export and emission-scope resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RustDeclarationVisibility {
    /// Public in the declaring module.
    Public,
    /// Restricted to the crate.
    Crate,
    /// Restricted to the named module path, including `self` or `super`.
    Restricted(String),
    /// Private to the declaring module.
    Private,
}
/// The kind of a declared Rust generic parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RustGenericParameterKind {
    /// A type parameter.
    Type,
    /// A const parameter.
    Const,
    /// A lifetime parameter.
    Lifetime,
}
/// One generic parameter in source order, retaining its source constraints/default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustGenericParameter {
    /// Declared parameter name, including the quote for a lifetime.
    pub name: String,
    /// Type, const or lifetime parameter.
    pub kind: RustGenericParameterKind,
    /// Exact token text of the declaration, including bounds and default.
    pub declaration: String,
}
/// Facts read from a real source type declaration, without inferred logical class flags.
#[derive(Clone, Debug)]
pub struct RustSourceDeclaration {
    /// Resolved definition path, without generic arguments.
    pub path: RustType,
    /// Module containing the declaration.
    pub module: String,
    /// Actual Rust declaration kind.
    pub kind: RustDeclarationKind,
    /// Normalized storage type of a literal `repr(transparent)` struct with exactly
    /// one unconditional field. Other layouts and conditional field selections
    /// remain unclassified; this fact alone does not declare an enum or a codec.
    pub transparent_field: Option<RustType>,
    /// Declared visibility; public reexports and enclosing modules remain separate facts.
    pub visibility: RustDeclarationVisibility,
    /// Generic parameters in source order.
    pub parameters: Vec<RustGenericParameter>,
    /// Normalized explicit trait bounds of a trait's supertrait list.
    pub supertraits: Vec<RustType>,
    /// Source token text of the where clause, if any.
    pub where_clause: Option<String>,
    /// All enclosing and declaration cfg/cfg_attr conditions, without target evaluation.
    pub cfg: Vec<String>,
    /// File, line and column of the declaration identifier.
    pub location: String,
}
/// A real trait implementation; generic or unresolved paths are retained as such.
#[derive(Clone, Debug)]
pub struct RustTraitImplementation {
    /// Module containing the implementation.
    pub module: String,
    /// Implementing type as written and normalized in its actual scope.
    pub self_type: RustType,
    /// Implemented trait as written and normalized in its actual scope.
    pub trait_type: RustType,
    /// Whether this is an explicit negative implementation.
    pub negative: bool,
    /// Generic parameters in source order.
    pub parameters: Vec<RustGenericParameter>,
    /// Source where clause, if present.
    pub where_clause: Option<String>,
    /// Enclosing/implementation conditions; not evaluated by this reader.
    pub cfg: Vec<String>,
    /// File, line and column of the impl token.
    pub location: String,
}
/// Source declarations and implementations collected through the shared scan's module graph.
#[derive(Clone, Debug, Default)]
pub struct RustSourceDeclarations {
    /// All actual type declarations in module traversal order.
    pub types: Vec<RustSourceDeclaration>,
    /// Explicit trait implementations, not guessed expansions of macros.
    pub implementations: Vec<RustTraitImplementation>,
}
impl RustSourceDeclarations {
    /// Finds the actual declaration named by shared metadata, including a single
    /// `dyn Trait` interface carrier. This resolves the declared trait itself;
    /// receiver/nullable/shared handles are never stripped or treated as definitions.
    /// Generic, conditional and emission-scope decisions remain separate validation.
    pub fn metadata_definition(&self, metadata: &TypeModel) -> Option<&RustSourceDeclaration> {
        self.definition(&metadata_definition_path(metadata)?)
    }
    /// Finds an unambiguous named definition, dropping only path arguments.
    /// Wrappers, aliases and trait objects are not unwrapped or guessed here.
    /// Input paths must use the reader's same canonical shared model set.
    pub fn definition(&self, type_: &RustType) -> Option<&RustSourceDeclaration> {
        if !type_.is_resolved() {
            return None;
        }
        let syn::Type::Path(mut path) = syn::parse_str::<syn::Type>(&type_.text).ok()? else {
            return None;
        };
        if path.qself.is_some() {
            return None;
        }
        for segment in &mut path.path.segments {
            segment.arguments = syn::PathArguments::None;
        }
        let key = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let mut matches = self
            .types
            .iter()
            .filter(|d| d.path.text.trim_start_matches("::") == key);
        let result = matches.next()?;
        if matches.next().is_some() {
            None
        } else {
            Some(result)
        }
    }
}

pub(crate) fn metadata_definition_path(metadata: &TypeModel) -> Option<RustType> {
    if !metadata.rust_path.is_resolved() {
        return None;
    }
    let parsed = syn::parse_str::<syn::Type>(&metadata.rust_path.text).ok()?;
    if let syn::Type::TraitObject(object) = parsed {
        if metadata.kind != TypeKind::Interface
            || object.dyn_token.is_none()
            || object.bounds.len() != 1
        {
            return None;
        }
        let syn::TypeParamBound::Trait(bound) = object.bounds.first()? else {
            return None;
        };
        if bound.lifetimes.is_some()
            || !matches!(bound.modifier, syn::TraitBoundModifier::None)
            || bound
                .path
                .segments
                .iter()
                .any(|s| !matches!(s.arguments, syn::PathArguments::None))
        {
            return None;
        }
        let path = bound
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        Some(RustType::resolved(&format!(
            "{}{path}",
            if bound.path.leading_colon.is_some() {
                "::"
            } else {
                ""
            }
        )))
    } else {
        Some(metadata.rust_path.clone())
    }
}
fn location(file: &str, span: proc_macro2::Span) -> String {
    let start = span.start();
    format!("{file}:{}:{}", start.line, start.column + 1)
}
fn parameters(generics: &syn::Generics) -> Vec<RustGenericParameter> {
    generics
        .params
        .iter()
        .map(|p| {
            let (name, kind) = match p {
                syn::GenericParam::Type(p) => (p.ident.to_string(), RustGenericParameterKind::Type),
                syn::GenericParam::Const(p) => {
                    (p.ident.to_string(), RustGenericParameterKind::Const)
                }
                syn::GenericParam::Lifetime(p) => (
                    p.lifetime.to_token_stream().to_string(),
                    RustGenericParameterKind::Lifetime,
                ),
            };
            RustGenericParameter {
                name,
                kind,
                declaration: p.to_token_stream().to_string(),
            }
        })
        .collect()
}
fn visibility(value: &syn::Visibility) -> RustDeclarationVisibility {
    match value {
        syn::Visibility::Public(_) => RustDeclarationVisibility::Public,
        syn::Visibility::Inherited => RustDeclarationVisibility::Private,
        syn::Visibility::Restricted(v) if v.path.is_ident("crate") => {
            RustDeclarationVisibility::Crate
        }
        syn::Visibility::Restricted(v) => {
            RustDeclarationVisibility::Restricted(v.path.to_token_stream().to_string())
        }
    }
}
fn conditions(attributes: &[syn::Attribute], inherited: &[String]) -> Vec<String> {
    let mut cfg = inherited.to_vec();
    cfg.extend(
        attributes
            .iter()
            .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
            .map(|a| a.meta.to_token_stream().to_string()),
    );
    cfg
}
pub(crate) fn test_only(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|a| {
        if !a.path().is_ident("cfg") {
            return false;
        }
        match a.parse_args::<syn::Meta>() {
            Ok(syn::Meta::Path(p)) => p.is_ident("test"),
            Ok(syn::Meta::List(p)) if p.path.is_ident("all") => p
                .tokens
                .into_iter()
                .any(|t| matches!(t,proc_macro2::TokenTree::Ident(i) if i=="test")),
            _ => false,
        }
    })
}
struct Reader<'a> {
    scan: &'a Scan,
    models: &'a ModelSet,
    files: HashMap<&'a str, &'a std::path::Path>,
    result: RustSourceDeclarations,
}
impl Reader<'_> {
    fn transparent_field(
        &self,
        module: &str,
        item: &syn::Item,
    ) -> Result<Option<RustType>, CompilationError> {
        let syn::Item::Struct(item) = item else {
            return Ok(None);
        };
        let mut representations = Vec::new();
        for attr in &item.attrs {
            if attr.path().is_ident("repr") {
                let Ok(hints) = attr.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                ) else {
                    return Ok(None);
                };
                representations.extend(hints);
            }
        }
        if !matches!(representations.as_slice(), [syn::Meta::Path(path)] if path.is_ident("transparent"))
            || item.fields.len() != 1
        {
            return Ok(None);
        }
        let field = item.fields.iter().next().expect("one transparent field");
        if !conditions(&field.attrs, &[]).is_empty() {
            return Ok(None);
        }
        Ok(Some(self.normalise(module, &field.ty)?))
    }
    fn normalise(
        &self,
        module: &str,
        tokens: &impl ToTokens,
    ) -> Result<RustType, CompilationError> {
        let mut type_ = self
            .scan
            .normalise(module, &tokens.to_token_stream().to_string())
            .ok_or_else(|| CompilationError(format!("Cannot normalize source type in {module}")))?;
        type_.text = self.models.canonical(&type_.text);
        Ok(type_)
    }
    fn file(&mut self, module: &str, cfg: &[String]) -> Result<(), CompilationError> {
        let path = self.files.remove(module).ok_or_else(|| {
            CompilationError(format!(
                "Source module {module} has no file in the shared scan"
            ))
        })?;
        let text = fs::read_to_string(path)
            .map_err(|e| CompilationError(format!("Cannot read {}: {e}", path.display())))?;
        let parsed = syn::parse_file(&text)
            .map_err(|e| CompilationError(format!("Cannot parse {}: {e}", path.display())))?;
        self.items(
            module,
            &path.display().to_string(),
            &parsed.items,
            &conditions(&parsed.attrs, cfg),
        )
    }
    fn items(
        &mut self,
        module: &str,
        file: &str,
        items: &[syn::Item],
        inherited: &[String],
    ) -> Result<(), CompilationError> {
        for item in items {
            if let syn::Item::Mod(item) = item {
                if test_only(&item.attrs) {
                    continue;
                }
                let cfg = conditions(&item.attrs, inherited);
                let child = format!(
                    "{module}::{}",
                    item.ident.to_string().trim_start_matches("r#")
                );
                if let Some((_, items)) = &item.content {
                    self.items(&child, file, items, &cfg)?;
                } else {
                    self.file(&child, &cfg)?;
                }
                continue;
            }
            if let syn::Item::Impl(item) = item {
                if test_only(&item.attrs) {
                    continue;
                }
                if let Some((negative, trait_, _)) = &item.trait_ {
                    self.result.implementations.push(RustTraitImplementation {
                        module: module.into(),
                        self_type: self.normalise(module, &item.self_ty)?,
                        trait_type: self.normalise(module, trait_)?,
                        negative: negative.is_some(),
                        parameters: parameters(&item.generics),
                        where_clause: item
                            .generics
                            .where_clause
                            .as_ref()
                            .map(|w| w.to_token_stream().to_string()),
                        cfg: conditions(&item.attrs, inherited),
                        location: location(file, item.impl_token.span()),
                    });
                }
                continue;
            }
            let (name, vis, generics, attrs, kind, bounds) = match item {
                syn::Item::Struct(i) => (
                    &i.ident,
                    &i.vis,
                    &i.generics,
                    &i.attrs,
                    RustDeclarationKind::Struct,
                    None,
                ),
                syn::Item::Enum(i) => (
                    &i.ident,
                    &i.vis,
                    &i.generics,
                    &i.attrs,
                    RustDeclarationKind::Enum,
                    None,
                ),
                syn::Item::Trait(i) => (
                    &i.ident,
                    &i.vis,
                    &i.generics,
                    &i.attrs,
                    RustDeclarationKind::Trait,
                    Some(&i.supertraits),
                ),
                syn::Item::Union(i) => (
                    &i.ident,
                    &i.vis,
                    &i.generics,
                    &i.attrs,
                    RustDeclarationKind::Union,
                    None,
                ),
                _ => continue,
            };
            if test_only(attrs) {
                continue;
            }
            let mut supertraits = Vec::new();
            if let Some(bounds) = bounds {
                for bound in bounds {
                    if let syn::TypeParamBound::Trait(t) = bound {
                        supertraits.push(self.normalise(module, &t.path)?);
                    }
                }
            }
            let path = self.normalise(module, name)?;
            if !path.is_resolved() {
                return Err(CompilationError(format!(
                    "{}: Unresolved source declaration {}",
                    location(file, name.span()),
                    path.text
                )));
            }
            self.result.types.push(RustSourceDeclaration {
                path,
                module: module.into(),
                kind,
                transparent_field: self.transparent_field(module, item)?,
                visibility: visibility(vis),
                parameters: parameters(generics),
                supertraits,
                where_clause: generics
                    .where_clause
                    .as_ref()
                    .map(|w| w.to_token_stream().to_string()),
                cfg: conditions(attrs, inherited),
                location: location(file, name.span()),
            });
        }
        Ok(())
    }
}
/// Reads real Rust types/trait impls from the same files and modules as a shared scan.
///
/// Inline modules and out-of-line #[path] modules retain their enclosing conditions.
/// Test-only items follow the shared production scan. Other conditions, generic constraints and
/// unresolved trait paths remain facts for target semantic resolution. Macro-generated
/// types/impls are not synthesized. Logical class abstraction, serialization names,
/// inheritance/property overrides and accessibility through exports are not inferred.
pub fn read_source_declarations(scan: &Scan) -> Result<RustSourceDeclarations, CompilationError> {
    read_source_declarations_with_models(scan, &ModelSet::new(vec![scan.model.clone()]))
}

/// Reads source facts using the compiler's shared model set for path identity.
/// Known public reexports of the crate and its dependencies canonicalize to their
/// declaring paths. Type aliases are not expanded; missing dependency facts and
/// macro expansions are not synthesized. Include the scanned crate in `models`.
pub fn read_source_declarations_with_models(
    scan: &Scan,
    models: &ModelSet,
) -> Result<RustSourceDeclarations, CompilationError> {
    if !models
        .models()
        .iter()
        .any(|m| m.crate_name == scan.model.crate_name)
    {
        return Err(CompilationError(
            "Shared model set omits the source crate".into(),
        ));
    }
    let mut files = HashMap::new();
    for file in &scan.files {
        if files
            .insert(file.module.as_str(), file.path.as_path())
            .is_some()
        {
            return Err(CompilationError(format!(
                "Duplicate source module {}",
                file.module
            )));
        }
    }
    let mut reader = Reader {
        scan,
        models,
        files,
        result: RustSourceDeclarations::default(),
    };
    reader.file(&scan.model.crate_name, &[])?;
    if !reader.files.is_empty() {
        return Err(CompilationError(
            "Shared source scan contains modules not reached by Rust declarations".into(),
        ));
    }
    Ok(reader.result)
}
