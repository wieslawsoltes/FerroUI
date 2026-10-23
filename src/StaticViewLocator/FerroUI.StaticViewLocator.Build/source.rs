//! What the generator reads of the sources of the crate, beside what the scanner of the framework
//! reads (the classes, their base classes and constructors, the paths): the types (the view
//! model candidates), the `impl` blocks (the traits a type implements, the contracts a view
//! implements, the members a locator declares), the visibility of the modules and the locators
//! with their attributes.
//!
//! The counterpart of the syntax providers of the reference (`CreateSyntaxProvider`,
//! `ForAttributeWithMetadataName`). The files are the ones the scanner read, from the root of the
//! crate along its `mod` items; items and modules under `#[cfg(test)]` are left out, as the
//! scanner leaves them out.

use crate::diagnostics::{self, Diagnostic, Location};
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use syn::parse::ParseStream;
#[cfg(test)]
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{bracketed, Attribute, Item, LitBool, LitStr, Token, Type};

/// The visibility of an item or a module, as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Visibility {
    /// `pub`.
    Public,
    /// `pub(crate)`.
    Crate,
    /// `pub(super)`.
    Super,
    /// `pub(in path)`, `pub(self)` or nothing: visible in the module given, as an absolute module
    /// path (`my_app::views`), and below it.
    In(String),
}

impl Visibility {
    fn of(visibility: &syn::Visibility, module: &str, crate_name: &str) -> Self {
        match visibility {
            syn::Visibility::Public(_) => Visibility::Public,
            syn::Visibility::Inherited => Visibility::In(module.to_string()),
            syn::Visibility::Restricted(restricted) => {
                let path: Vec<String> = restricted.path.segments.iter().map(|segment| segment.ident.to_string()).collect();
                match path.first().map(String::as_str) {
                    Some("crate") if path.len() == 1 => Visibility::Crate,
                    Some("super") if path.len() == 1 => Visibility::Super,
                    Some("self") if path.len() == 1 => Visibility::In(module.to_string()),
                    Some("crate") => Visibility::In(format!("{crate_name}::{}", path[1..].join("::"))),
                    _ => Visibility::Crate,
                }
            }
        }
    }

    /// Whether an item with this visibility, declared in the module `declared_in`, is visible from
    /// the module `from` (absolute module paths of one crate).
    pub fn is_visible(&self, declared_in: &str, from: &str) -> bool {
        match self {
            Visibility::Public | Visibility::Crate => true,
            Visibility::Super => is_within(from, parent_module(declared_in).unwrap_or(declared_in)),
            Visibility::In(module) => is_within(from, module),
        }
    }
}

/// Whether the module `module` is `ancestor` or below it.
pub fn is_within(module: &str, ancestor: &str) -> bool {
    module == ancestor || module.strip_prefix(ancestor).is_some_and(|rest| rest.starts_with("::"))
}

/// The parent of an absolute module path; `None` for the root of a crate.
pub fn parent_module(module: &str) -> Option<&str> {
    module.rfind("::").map(|at| &module[..at])
}

/// The kind of a type of the sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Struct,
    Enum,
    Trait,
}

/// A struct, an enumeration or a trait of the sources.
#[derive(Clone, Debug)]
pub struct SourceType {
    pub name: String,
    /// The absolute module path (`my_app::view_models`).
    pub module: String,
    pub kind: ItemKind,
    /// The names of the generic parameters, lifetimes and constants included.
    pub generics: Vec<String>,
    pub visibility: Visibility,
    /// The bounds of a trait (`trait IDetailsViewModel: IViewModel`), as written.
    pub supertraits: Vec<String>,
    /// The number of associated types of a trait.
    pub associated_types: usize,
    /// Whether a trait can be the type of a trait object (`dyn Trait`), as far as its items tell:
    /// no generic parameter, associated type or constant, no `Sized` bound, and every method not
    /// bounded by `Self: Sized` has a receiver, no type parameter and no `Self` in its signature.
    pub dyn_compatible: bool,
    pub file: PathBuf,
    pub line: usize,
}

/// A method of an `impl` block.
#[derive(Clone, Debug)]
pub struct SourceMethod {
    pub name: String,
    /// `self`, `&self` or `&mut self`, as written; `None` for an associated function.
    pub receiver: Option<String>,
    /// The types of the parameters after the receiver, as written.
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub line: usize,
}

/// An `impl` block of the sources.
#[derive(Clone, Debug)]
pub struct SourceImpl {
    /// The module of the block.
    pub module: String,
    /// The trait, as written without its generic arguments (`IViewFor`, `crate::Contract`);
    /// `None` for an inherent block.
    pub trait_path: Option<String>,
    /// The generic arguments of the trait, as written (`Contract<DashboardModel>`).
    pub trait_arguments: Vec<String>,
    /// The type the block is for, as written.
    pub self_type: String,
    /// The names of the generic parameters of the block.
    pub generics: Vec<String>,
    /// The associated types of a trait block: name and type, as written.
    pub associated_types: Vec<(String, String)>,
    pub methods: Vec<SourceMethod>,
}

/// The options of a locator, as its attribute states them.
#[derive(Clone, Debug, Default)]
pub struct LocatorAttribute {
    pub id: Option<String>,
    pub generate_view_factory_methods: Option<bool>,
    pub generate_runtime_type_fallback_methods: Option<bool>,
    pub generate_view_locator: Option<bool>,
    pub generate_data_template: Option<bool>,
    /// The name of the mode.
    pub generated_adapter_resolution_mode: Option<String>,
    /// The types, as written.
    pub view_model_mapping_contracts: Vec<String>,
    pub data_template_match_types: Vec<String>,
}

/// A locator: a struct with `#[static_view_locator]`.
#[derive(Clone, Debug)]
pub struct SourceLocator {
    pub name: String,
    pub module: String,
    /// The generic parameters of the struct, as written with their bounds (`<T: Clone>`), and
    /// the where clause.
    pub generics: String,
    pub where_clause: String,
    /// The generic arguments the struct is named with (`<T>`).
    pub generic_arguments: String,
    pub attribute: LocatorAttribute,
    /// The explicit mappings (`#[static_view_mapping(VM, View)]`): the two types, as written.
    pub mappings: Vec<(String, String)>,
    pub file: PathBuf,
    pub line: usize,
}

impl SourceLocator {
    pub fn location(&self) -> Location {
        Location::new(&self.file, self.line)
    }
}

/// What the generator reads of the sources.
#[derive(Clone, Debug, Default)]
pub struct SourceCrate {
    pub types: Vec<SourceType>,
    pub impls: Vec<SourceImpl>,
    /// The visibility of each module but the root, by its absolute path.
    pub modules: BTreeMap<String, Visibility>,
    pub locators: Vec<SourceLocator>,
    pub diagnostics: Vec<Diagnostic>,
}

impl SourceCrate {
    /// Reads the files `files`, each with the absolute path of its module, of the crate
    /// `crate_name`.
    pub fn read(crate_name: &str, files: &[(PathBuf, String)]) -> Self {
        let mut source = SourceCrate::default();
        for (path, module) in files {
            let text = match std::fs::read_to_string(path) {
                Ok(text) => text,
                Err(error) => {
                    source.diagnostics.push(Diagnostic::error(
                        diagnostics::INVALID_OPTION,
                        format!("the source file cannot be read: {error}"),
                        &Location::new(path, 0),
                    ));
                    continue;
                }
            };
            // A file the compiler rejects is the compiler's to report.
            let Ok(file) = syn::parse_file(&text) else { continue };
            let mut reader = Reader { crate_name, file: path, source: &mut source };
            reader.items(&file.items, module);
        }
        source
    }

    /// Whether the item named `name`, declared in `module` with `visibility`, is visible from the
    /// module `from`, with every module on its path.
    pub fn is_accessible(&self, module: &str, visibility: &Visibility, from: &str) -> bool {
        if !visibility.is_visible(module, from) {
            return false;
        }
        let mut current = module;
        while let Some(parent) = parent_module(current) {
            if let Some(visibility) = self.modules.get(current) {
                if !visibility.is_visible(parent, from) {
                    return false;
                }
            }
            current = parent;
        }
        true
    }
}

struct Reader<'a> {
    crate_name: &'a str,
    file: &'a Path,
    source: &'a mut SourceCrate,
}

impl Reader<'_> {
    fn items(&mut self, items: &[Item], module: &str) {
        for item in items {
            if is_test_only(item_attributes(item)) {
                continue;
            }
            match item {
                Item::Mod(item_mod) => {
                    let path = format!("{module}::{}", item_mod.ident);
                    self.source.modules.insert(path.clone(), Visibility::of(&item_mod.vis, module, self.crate_name));
                    if let Some((_, items)) = &item_mod.content {
                        self.items(items, &path);
                    }
                }
                Item::Struct(item_struct) => {
                    self.type_(&item_struct.ident, &item_struct.vis, &item_struct.generics, ItemKind::Struct, Vec::new(), 0, module);
                    if let Some(attribute) = item_struct.attrs.iter().find(|attribute| is_named(attribute, "static_view_locator")) {
                        self.locator(item_struct, attribute, module);
                    }
                }
                Item::Enum(item_enum) => {
                    self.type_(&item_enum.ident, &item_enum.vis, &item_enum.generics, ItemKind::Enum, Vec::new(), 0, module);
                    self.not_a_struct(&item_enum.attrs, &item_enum.ident, "an enumeration");
                }
                Item::Trait(item_trait) => {
                    let supertraits = item_trait
                        .supertraits
                        .iter()
                        .filter_map(|bound| match bound {
                            syn::TypeParamBound::Trait(bound) => Some(path_without_arguments(&bound.path)),
                            _ => None,
                        })
                        .collect();
                    let associated_types = item_trait.items.iter().filter(|item| matches!(item, syn::TraitItem::Type(_))).count();
                    self.type_(&item_trait.ident, &item_trait.vis, &item_trait.generics, ItemKind::Trait, supertraits, associated_types, module);
                    if let Some(type_) = self.source.types.last_mut() {
                        type_.dyn_compatible = is_dyn_compatible(item_trait);
                    }
                    self.not_a_struct(&item_trait.attrs, &item_trait.ident, "a trait");
                }
                Item::Union(item_union) => self.not_a_struct(&item_union.attrs, &item_union.ident, "a union"),
                Item::Type(item_type) => self.not_a_struct(&item_type.attrs, &item_type.ident, "a type alias"),
                Item::Fn(item_fn) => self.not_a_struct(&item_fn.attrs, &item_fn.sig.ident, "a function"),
                Item::Impl(item_impl) => self.impl_(item_impl, module),
                _ => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn type_(&mut self, ident: &syn::Ident, visibility: &syn::Visibility, generics: &syn::Generics, kind: ItemKind, supertraits: Vec<String>, associated_types: usize, module: &str) {
        self.source.types.push(SourceType {
            name: ident.to_string(),
            module: module.to_string(),
            kind,
            generics: generic_names(generics),
            visibility: Visibility::of(visibility, module, self.crate_name),
            supertraits,
            associated_types,
            dyn_compatible: false,
            file: self.file.to_path_buf(),
            line: line_of(ident.span()),
        });
    }

    /// SVL0005 for `#[static_view_locator]` on an item that is not a struct (the compiler reports
    /// it too, through the attribute).
    fn not_a_struct(&mut self, attributes: &[Attribute], ident: &syn::Ident, what: &str) {
        if attributes.iter().any(|attribute| is_named(attribute, "static_view_locator")) {
            self.source.diagnostics.push(Diagnostic::error(
                diagnostics::UNSUPPORTED_LOCATOR_TYPE,
                format!("Locator '{ident}' is {what}; StaticViewLocator requires a struct"),
                &Location::new(self.file, line_of(ident.span())),
            ));
        }
    }

    fn impl_(&mut self, item_impl: &syn::ItemImpl, module: &str) {
        let (trait_path, trait_arguments) = match &item_impl.trait_ {
            Some((None, path, _)) => {
                let arguments = match path.segments.last().map(|segment| &segment.arguments) {
                    Some(syn::PathArguments::AngleBracketed(arguments)) => arguments
                        .args
                        .iter()
                        .filter_map(|argument| match argument {
                            syn::GenericArgument::Type(type_) => Some(text_of(type_)),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                (Some(path_without_arguments(path)), arguments)
            }
            // A negative impl states nothing a locator uses.
            Some((Some(_), _, _)) => return,
            None => (None, Vec::new()),
        };
        let mut associated_types = Vec::new();
        let mut methods = Vec::new();
        for item in &item_impl.items {
            match item {
                syn::ImplItem::Type(associated) => associated_types.push((associated.ident.to_string(), text_of(&associated.ty))),
                syn::ImplItem::Fn(function) => {
                    if is_test_only(&function.attrs) {
                        continue;
                    }
                    let mut receiver = None;
                    let mut parameters = Vec::new();
                    for input in &function.sig.inputs {
                        match input {
                            syn::FnArg::Receiver(self_) => {
                                receiver = Some(match (&self_.reference, &self_.mutability) {
                                    (Some(_), Some(_)) => "&mut self".to_string(),
                                    (Some(_), None) => "&self".to_string(),
                                    (None, _) => text_of(&*self_.ty).replace(' ', ""),
                                })
                            }
                            syn::FnArg::Typed(typed) => parameters.push(text_of(&*typed.ty)),
                        }
                    }
                    methods.push(SourceMethod {
                        name: function.sig.ident.to_string(),
                        receiver,
                        parameters,
                        return_type: match &function.sig.output {
                            syn::ReturnType::Default => None,
                            syn::ReturnType::Type(_, type_) => Some(text_of(&**type_)),
                        },
                        visibility: Visibility::of(&function.vis, module, self.crate_name),
                        line: line_of(function.sig.ident.span()),
                    });
                }
                _ => {}
            }
        }
        self.source.impls.push(SourceImpl {
            module: module.to_string(),
            trait_path,
            trait_arguments,
            self_type: text_of(&*item_impl.self_ty),
            generics: generic_names(&item_impl.generics),
            associated_types,
            methods,
        });
    }

    fn locator(&mut self, item: &syn::ItemStruct, attribute: &Attribute, module: &str) {
        let location = Location::new(self.file, line_of(item.ident.span()));
        let parsed = match &attribute.meta {
            syn::Meta::Path(_) => Ok(LocatorAttribute::default()),
            _ => attribute.parse_args_with(parse_locator_attribute),
        };
        let attribute = match parsed {
            Ok(attribute) => attribute,
            Err(error) => {
                self.source.diagnostics.push(Diagnostic::error(
                    diagnostics::INVALID_OPTION,
                    format!("the attribute of the locator '{}' is not understood: {error}", item.ident),
                    &location,
                ));
                return;
            }
        };
        let mut mappings = Vec::new();
        for mapping in item.attrs.iter().filter(|attribute| is_named(attribute, "static_view_mapping")) {
            match mapping.parse_args_with(Punctuated::<Type, Token![,]>::parse_terminated) {
                Ok(types) if types.len() == 2 => mappings.push((text_of(&types[0]), text_of(&types[1]))),
                _ => self.source.diagnostics.push(Diagnostic::error(
                    diagnostics::INVALID_OPTION,
                    format!("a #[static_view_mapping] of the locator '{}' does not state a view model type and a view type", item.ident),
                    &Location::new(self.file, line_of(mapping.span())),
                )),
            }
        }
        let (impl_generics, type_generics, where_clause) = item.generics.split_for_impl();
        self.source.locators.push(SourceLocator {
            name: item.ident.to_string(),
            module: module.to_string(),
            generics: impl_generics.to_token_stream().to_string(),
            where_clause: where_clause.map(|clause| clause.to_token_stream().to_string()).unwrap_or_default(),
            generic_arguments: type_generics.to_token_stream().to_string(),
            attribute,
            mappings,
            file: self.file.to_path_buf(),
            line: location.line,
        });
    }
}

/// The options of `#[static_view_locator(..)]`.
fn parse_locator_attribute(input: ParseStream) -> syn::Result<LocatorAttribute> {
    let mut attribute = LocatorAttribute::default();
    while !input.is_empty() {
        let name: syn::Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        match name.to_string().as_str() {
            "id" => attribute.id = Some(input.parse::<LitStr>()?.value()),
            "generate_view_factory_methods" => attribute.generate_view_factory_methods = Some(input.parse::<LitBool>()?.value),
            "generate_runtime_type_fallback_methods" => attribute.generate_runtime_type_fallback_methods = Some(input.parse::<LitBool>()?.value),
            "generate_view_locator" => attribute.generate_view_locator = Some(input.parse::<LitBool>()?.value),
            "generate_data_template" => attribute.generate_data_template = Some(input.parse::<LitBool>()?.value),
            "generated_adapter_resolution_mode" => {
                let path: syn::Path = input.parse()?;
                attribute.generated_adapter_resolution_mode = path.segments.last().map(|segment| segment.ident.to_string());
            }
            "view_model_mapping_contracts" => attribute.view_model_mapping_contracts = parse_types(input)?,
            "data_template_match_types" => attribute.data_template_match_types = parse_types(input)?,
            other => return Err(syn::Error::new(name.span(), format!("unknown option `{other}`"))),
        }
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(attribute)
}

fn parse_types(input: ParseStream) -> syn::Result<Vec<String>> {
    let content;
    bracketed!(content in input);
    Ok(Punctuated::<Type, Token![,]>::parse_terminated(&content)?.iter().map(text_of).collect())
}

/// Parses the options of a locator attribute written as text (for the tests).
#[cfg(test)]
pub(crate) fn parse_attribute_text(text: &str) -> syn::Result<LocatorAttribute> {
    parse_locator_attribute.parse_str(text)
}

/// The text of tokens, as `quote` prints them (`Contract < Foo >`); the scanner reads it again.
pub fn text_of(tokens: &impl ToTokens) -> String {
    tokens.to_token_stream().to_string()
}

/// A path without the generic arguments of its segments (`a::Contract` of `a::Contract<Foo>`).
fn path_without_arguments(path: &syn::Path) -> String {
    let mut text = String::new();
    if path.leading_colon.is_some() {
        text.push_str("::");
    }
    let segments: Vec<String> = path.segments.iter().map(|segment| segment.ident.to_string()).collect();
    text.push_str(&segments.join("::"));
    text
}

fn generic_names(generics: &syn::Generics) -> Vec<String> {
    generics
        .params
        .iter()
        .map(|parameter| match parameter {
            syn::GenericParam::Type(type_) => type_.ident.to_string(),
            syn::GenericParam::Lifetime(lifetime) => lifetime.lifetime.to_string(),
            syn::GenericParam::Const(constant) => constant.ident.to_string(),
        })
        .collect()
}

/// Whether `TypeId::of::<dyn Trait>()` compiles for the trait (an approximation of the rules of
/// dyn compatibility from the items of the trait).
fn is_dyn_compatible(item: &syn::ItemTrait) -> bool {
    let mentions_self = |tokens: &dyn ToTokens| {
        tokens.to_token_stream().into_iter().any(|token| fn_mentions(&token))
    };
    fn fn_mentions(token: &proc_macro2::TokenTree) -> bool {
        match token {
            proc_macro2::TokenTree::Ident(ident) => ident == "Self",
            proc_macro2::TokenTree::Group(group) => group.stream().into_iter().any(|token| fn_mentions(&token)),
            _ => false,
        }
    }
    let sized = |bounds: &Punctuated<syn::TypeParamBound, Token![+]>| {
        bounds.iter().any(|bound| matches!(bound, syn::TypeParamBound::Trait(bound) if bound.path.is_ident("Sized")))
    };
    if !item.generics.params.is_empty() || sized(&item.supertraits) || item.auto_token.is_some() {
        return false;
    }
    item.items.iter().all(|trait_item| match trait_item {
        syn::TraitItem::Fn(function) => {
            let where_self_sized = function.sig.generics.where_clause.as_ref().is_some_and(|clause| {
                clause.predicates.iter().any(|predicate| match predicate {
                    syn::WherePredicate::Type(predicate) => text_of(&predicate.bounded_ty) == "Self" && sized(&predicate.bounds),
                    _ => false,
                })
            });
            if where_self_sized {
                return true;
            }
            let has_receiver = function.sig.receiver().is_some();
            let type_parameters = function.sig.generics.params.iter().any(|parameter| !matches!(parameter, syn::GenericParam::Lifetime(_)));
            let self_in_signature = function.sig.inputs.iter().any(|input| match input {
                syn::FnArg::Typed(typed) => mentions_self(&typed.ty),
                syn::FnArg::Receiver(_) => false,
            }) || matches!(&function.sig.output, syn::ReturnType::Type(_, type_) if mentions_self(type_));
            let impl_return = matches!(&function.sig.output, syn::ReturnType::Type(_, type_) if matches!(**type_, Type::ImplTrait(_)));
            has_receiver && !type_parameters && !self_in_signature && !impl_return && function.sig.asyncness.is_none()
        }
        syn::TraitItem::Type(_) | syn::TraitItem::Const(_) => false,
        _ => true,
    })
}

fn line_of(span: Span) -> usize {
    span.start().line
}

/// Whether `attribute` is the attribute `name` (`#[name]`, `#[name(..)]` or `#[path::name(..)]`).
fn is_named(attribute: &Attribute, name: &str) -> bool {
    attribute.path().segments.last().is_some_and(|segment| segment.ident == name)
}

/// Whether the attributes put the item under `#[cfg(test)]` or `#[cfg(all(.., test, ..))]` (the
/// scanner's rule).
fn is_test_only(attributes: &[Attribute]) -> bool {
    let names_test = |tokens: &TokenStream| tokens.clone().into_iter().any(|token| matches!(token, proc_macro2::TokenTree::Ident(ident) if ident == "test"));
    attributes.iter().any(|attribute| {
        let syn::Meta::List(list) = &attribute.meta else { return false };
        if !attribute.path().is_ident("cfg") {
            return false;
        }
        let tokens: Vec<proc_macro2::TokenTree> = list.tokens.clone().into_iter().collect();
        match tokens.as_slice() {
            [proc_macro2::TokenTree::Ident(ident)] => ident == "test",
            [proc_macro2::TokenTree::Ident(all), proc_macro2::TokenTree::Group(group)] => all == "all" && names_test(&group.stream()),
            _ => false,
        }
    })
}

fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => &[],
    }
}
