//! ferroui-static-view-locator-macros
//!
//! The attributes of ferroui-static-view-locator: [`static_view_locator`] (`[StaticViewLocator]`)
//! and [`static_view_mapping`] (`[StaticViewMapping]`). Use them through
//! `ferroui_static_view_locator`, which re-exports them.
//!
//! The attributes do not generate the locator: a proc macro sees one item and cannot find the
//! view models and views of the crate. The build script of the crate does
//! (`ferroui-static-view-locator-build`), and writes one file per locator below `OUT_DIR`;
//! [`static_view_locator`] checks its options, writes the struct, and includes that file, whose
//! `impl` blocks extend the struct (the `partial class` of the reference).

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{bracketed, Attribute, Item, LitBool, LitStr, Token, Type};

/// The resolution modes of `generated_adapter_resolution_mode` (`ViewResolutionMode`).
const RESOLUTION_MODES: &[&str] =
    &["Exact", "ExactThenBaseTypes", "ExactThenInterfaces", "ExactThenBaseTypesThenInterfaces", "ExactThenInterfacesThenBaseTypes"];

/// The boolean options of [`static_view_locator`].
const BOOLEAN_OPTIONS: &[&str] =
    &["generate_view_factory_methods", "generate_runtime_type_fallback_methods", "generate_view_locator", "generate_data_template"];

/// Makes a struct a view locator generated at build time (`[StaticViewLocator]`).
///
/// ```ignore
/// #[static_view_locator(
///     generate_view_locator = true,
///     generate_data_template = true,
///     generate_runtime_type_fallback_methods = false,
///     generated_adapter_resolution_mode = ExactThenBaseTypesThenInterfaces,
///     data_template_match_types = [ViewModelBase],
/// )]
/// #[derive(Default)]
/// pub struct ViewLocator;
/// ```
///
/// | Option | Default | Behaviour |
/// |---|---|---|
/// | `generate_view_factory_methods` | `false` | `fn try_create_view_exact(TypeId) -> Option<Ref<Control>>`, unless the locator declares it |
/// | `generate_runtime_type_fallback_methods` | `true` | the legacy fallback helpers, when a legacy or declared `build` needs them |
/// | `generate_view_locator` | `false` | `IViewLocator` of ReactiveUI and the automatic `IControlViewFor` mappings (feature `reactiveui`) |
/// | `generate_data_template` | `false` | `IDataTemplate` with `build`, `match_` and the build hooks |
/// | `generated_adapter_resolution_mode` | `ExactThenBaseTypesThenInterfaces` | the fallback precedence of the generated adapters |
/// | `view_model_mapping_contracts` | empty | traits with one view model type that views implement (`Contract<VM>`, or an associated type) |
/// | `data_template_match_types` | empty | the types (classes or traits) a resolved view model must be to match |
/// | `id` | the name of the struct | the name of the generated file, for two locators of one name in a crate |
///
/// The struct must be a top-level item of a module the build script reads (not in a function and
/// not under `#[cfg(test)]`), and the crate must run the generator in its build script.
#[proc_macro_attribute]
pub fn static_view_locator(attribute: TokenStream, item: TokenStream) -> TokenStream {
    expand_locator(attribute.into(), item.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Maps a view model type to a view type on a locator (`[StaticViewMapping(typeof(VM), typeof(V))]`):
/// the final override of the mapping of the view model, for types that do not follow the naming
/// rules. Repeatable; the types are named as in the module of the locator.
///
/// ```ignore
/// #[static_view_locator]
/// #[static_view_mapping(LoginViewModel, LogInView)]
/// #[static_view_mapping(DashboardModel, DashboardScreen)]
/// pub struct ViewLocator;
/// ```
///
/// [`static_view_locator`] reads the mappings below it; a mapping written above it is left in
/// place by this attribute, which only checks it.
#[proc_macro_attribute]
pub fn static_view_mapping(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let checked = parse_mapping.parse2(attribute.into());
    match checked {
        Ok(_) => item,
        Err(error) => {
            let mut tokens: TokenStream2 = error.into_compile_error();
            tokens.extend(TokenStream2::from(item));
            tokens.into()
        }
    }
}

fn expand_locator(attribute: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let (id, mut named) = parse_options.parse2(attribute)?;
    let mut item: Item = syn::parse2(item)?;
    let Item::Struct(locator) = &mut item else {
        return Err(syn::Error::new(
            item.span(),
            "SVL0005: #[static_view_locator] extends a struct; the generated members are an `impl` block of the struct",
        ));
    };
    let mut kept: Vec<Attribute> = Vec::with_capacity(locator.attrs.len());
    for attribute in locator.attrs.drain(..) {
        if is_named(&attribute, "static_view_mapping") {
            let (view_model, view) = attribute.parse_args_with(parse_mapping)?;
            named.push(view_model);
            named.push(view);
        } else if is_named(&attribute, "static_view_locator") {
            return Err(syn::Error::new(attribute.span(), "#[static_view_locator] is stated more than once"));
        } else {
            kept.push(attribute);
        }
    }
    locator.attrs = kept;
    let file = format!("{}.rs", id.unwrap_or_else(|| locator.ident.to_string()));
    // The types the options name are used by the generated source through the build script only:
    // importing them here resolves them where they are written (a misspelled type is an error of
    // the compiler at the attribute) and uses the imports they are named by.
    let uses = named.iter().filter_map(type_path).map(|path| quote! { use #path as _; });
    Ok(quote! {
        #item
        #[allow(unused_imports)]
        const _: () = {
            #(#uses)*
        };
        include!(concat!(env!("OUT_DIR"), "/static_view_locator/", #file));
    })
}

/// The path of a named type without its generic arguments (`a::Widget` of `a::Widget<i32>`), for a
/// `use`; `None` for a type that is not a path or is a primitive type.
fn type_path(type_: &Type) -> Option<syn::Path> {
    let Type::Path(type_path) = type_ else { return None };
    if type_path.qself.is_some() {
        return None;
    }
    let mut path = type_path.path.clone();
    for segment in &mut path.segments {
        segment.arguments = syn::PathArguments::None;
    }
    let primitive = path.segments.len() == 1 && path.segments[0].ident.to_string().starts_with(|character: char| character.is_ascii_lowercase());
    (!primitive).then_some(path)
}

/// Whether `attribute` is the attribute `name` (`#[name(..)]` or `#[path::name(..)]`).
fn is_named(attribute: &Attribute, name: &str) -> bool {
    attribute.path().segments.last().is_some_and(|segment| segment.ident == name)
}

/// The two types of `#[static_view_mapping(VM, View)]`.
fn parse_mapping(input: ParseStream) -> syn::Result<(Type, Type)> {
    let types = Punctuated::<Type, Token![,]>::parse_terminated(input)?;
    if types.len() != 2 {
        return Err(syn::Error::new(
            if types.is_empty() { Span::call_site() } else { types.span() },
            "#[static_view_mapping] takes the view model type and the view type: #[static_view_mapping(SettingsViewModel, SettingsView)]",
        ));
    }
    let mut types = types.into_iter();
    Ok((types.next().expect("two types"), types.next().expect("two types")))
}

/// The options of `#[static_view_locator(..)]`, checked: the `id` when one is stated, and the types
/// the options name.
fn parse_options(input: ParseStream) -> syn::Result<(Option<String>, Vec<Type>)> {
    let mut id = None;
    let mut named = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    while !input.is_empty() {
        let name: syn::Ident = input.parse()?;
        let key = name.to_string();
        if seen.contains(&key) {
            return Err(syn::Error::new(name.span(), format!("the option `{key}` is stated more than once")));
        }
        input.parse::<Token![=]>()?;
        match key.as_str() {
            "id" => {
                let value: LitStr = input.parse()?;
                let text = value.value();
                if text.is_empty() || !text.chars().all(|character| character.is_ascii_alphanumeric() || character == '_') {
                    return Err(syn::Error::new(value.span(), "the id of a locator is a name of letters, digits and underscores"));
                }
                id = Some(text);
            }
            key if BOOLEAN_OPTIONS.contains(&key) => {
                input.parse::<LitBool>()?;
            }
            "generated_adapter_resolution_mode" => {
                let path: syn::Path = input.parse()?;
                let mode = path.segments.last().map(|segment| segment.ident.to_string()).unwrap_or_default();
                if !RESOLUTION_MODES.contains(&mode.as_str()) {
                    return Err(syn::Error::new(path.span(), format!("the resolution mode is one of {}", RESOLUTION_MODES.join(", "))));
                }
            }
            "view_model_mapping_contracts" | "data_template_match_types" => {
                let content;
                bracketed!(content in input);
                named.extend(Punctuated::<Type, Token![,]>::parse_terminated(&content)?);
            }
            _ => {
                return Err(syn::Error::new(
                    name.span(),
                    format!(
                        "unknown option `{key}`; the options are id, {}, generated_adapter_resolution_mode, view_model_mapping_contracts and data_template_match_types",
                        BOOLEAN_OPTIONS.join(", ")
                    ),
                ))
            }
        }
        seen.push(key);
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok((id, named))
}
