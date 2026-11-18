//! ferroui-view-macros
//!
//! The macros of `ferroui-view`: [`rsx!`] writes an element tree in a form
//! close to XAML and expands to the builders, and [`#[component]`](component)
//! turns a function into a component with a builder of its parameters.
//! Errors point at the user's code: an attribute that a class has no setter
//! for is a missing method of its builder.

use ferroui_view_syntax::{snake_case, Attribute, ElementNode, Node};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{Expr, FnArg, Ident, ItemFn, Pat, Token, Type};

/// An element tree, expanded to the builders of `ferroui-view`.
///
/// ```ignore
/// rsx! {
///     StackPanel { spacing: 8.0,
///         TextBlock { text: move || format!("Count: {}", count.get()) }
///         Button { on_click: move |_| count.update(|c| *c += 1), "Increment" }
///     }
/// }
/// ```
///
/// - `Name { ... }` is an element of the control class or component `Name`
///   in scope (`<Name as Builder>::builder()`): `StackPanel` is
///   `stack_panel()`, `todo::Row` the component `Row` of the module `todo`.
/// - `name` alone gives the setter `name` the variable `name`, as a field of
///   a struct is given (`TextBlock { text }`).
/// - `name: value` calls the setter `.name(value)`: a property
///   (`spacing: 8.0`), a handler (`on_click: ...`), `key`, `name`, `class`.
/// - `Owner.name: value` sets an attached property: `Grid.row: 1` calls
///   `.grid_row(1)`.
/// - `name(a, b)` calls a method of the builder with several arguments:
///   `on_changed(TextBox::text_property(), handler)`,
///   `set(Grid::row_property(), 1)` (the indexer `control[Property] = 1`
///   upstream), `bind(TextBlock::text_property(), binding)`.
/// - `"text"` is a text child, `{expr}` any child (an element, a builder, a
///   list, `for_each(..)`, `show(..)`), and a nested element a child.
/// - `if cond { .. } else { .. }` and `for x in items { .. }` choose and
///   repeat children when the tree is built (for children that follow state,
///   use `{for_each(..)}`, `{show(..)}` or `{dynamic(..)}`).
///
/// Several nodes at the root make a fragment.
#[proc_macro]
pub fn rsx(input: TokenStream) -> TokenStream {
    let nodes = match ferroui_view_syntax::parse(input.into()) {
        Ok(nodes) => nodes,
        Err(error) => return error.to_compile_error().into(),
    };
    let elements: Vec<TokenStream2> = nodes.iter().map(node_element).collect();
    let tokens = if elements.len() == 1 {
        elements.into_iter().next().unwrap_or_default()
    } else {
        quote! { ::ferroui_view::Element::Fragment(::std::vec![#(#elements),*]) }
    };
    tokens.into()
}

/// A literal that hot reload can change (`ferroui_view::hot::lit`).
fn hot_literal(index: u32, literal: &TokenStream2) -> TokenStream2 {
    quote! {
        ::ferroui_view::hot::lit(
            ::ferroui_view::hot::Site { file: file!(), line: line!(), column: column!(), manifest: env!("CARGO_MANIFEST_DIR") },
            #index,
            #literal,
        )
    }
}

/// The node as a child expression (what `.child(..)` takes).
fn node_child(node: &Node) -> TokenStream2 {
    match node {
        Node::Element(element) => element_builder(element),
        Node::Text(text, index) => hot_literal(*index, &text.to_token_stream()),
        Node::Expr(expr, span) => quote_spanned! {*span=> #expr },
        Node::If(..) | Node::For(..) => node_element(node),
    }
}

/// The node as an `Element`.
fn node_element(node: &Node) -> TokenStream2 {
    match node {
        Node::Element(element) => {
            let builder = element_builder(element);
            quote! { ::ferroui_view::IntoElement::into_element(#builder) }
        }
        Node::Text(text, index) => {
            let text = hot_literal(*index, &text.to_token_stream());
            quote! { ::ferroui_view::IntoChild::into_child(#text) }
        }
        Node::Expr(expr, span) => quote_spanned! {*span=> ::ferroui_view::IntoChild::into_child(#expr) },
        Node::If(condition, then, otherwise, span) => {
            let then = fragment(then);
            let otherwise = otherwise.as_ref().map_or_else(|| quote! { ::ferroui_view::Element::Empty }, |nodes| fragment(nodes));
            quote_spanned! {*span=> if #condition { #then } else { #otherwise } }
        }
        Node::For(pattern, items, body, span) => {
            let body = fragment(body);
            quote_spanned! {*span=>
                ::ferroui_view::Element::List(
                    ::std::iter::IntoIterator::into_iter(#items).map(|#pattern| #body).collect::<::std::vec::Vec<_>>()
                )
            }
        }
    }
}

fn element_builder(element: &ElementNode) -> TokenStream2 {
    let path = &element.path;
    let span = path.span();
    let mut tokens = quote_spanned! {span=> <#path as ::ferroui_view::Builder>::builder() };
    for attribute in &element.attributes {
        match attribute {
            Attribute::Value { owner, name, value, hot } => {
                let value = match hot {
                    Some(index) => hot_literal(*index, &value.to_token_stream()),
                    None => value.to_token_stream(),
                };
                let method = match owner {
                    Some(owner) => {
                        format_ident!("{}_{}", snake_case(&owner.unraw().to_string()), name.unraw(), span = name.span())
                    }
                    None => name.clone(),
                };
                tokens = quote_spanned! {method.span()=> #tokens.#method(#value) };
            }
            Attribute::Call { name, arguments } => {
                tokens = quote_spanned! {name.span()=> #tokens.#name(#arguments) };
            }
        }
    }
    for child in &element.children {
        let child = node_child(child);
        tokens = quote! { #tokens.child(#child) };
    }
    tokens
}

fn fragment(nodes: &[Node]) -> TokenStream2 {
    let elements: Vec<TokenStream2> = nodes.iter().map(node_element).collect();
    match elements.len() {
        0 => quote! { ::ferroui_view::Element::Empty },
        1 => elements.into_iter().next().unwrap_or_default(),
        _ => quote! { ::ferroui_view::Element::Fragment(::std::vec![#(#elements),*]) },
    }
}

/// A component: a function whose parameters are its properties, with a
/// builder so that `rsx!` and the builders use it like a control.
///
/// ```ignore
/// #[component]
/// fn Counter(initial: i32, #[prop(default = 1)] step: i32) -> Element {
///     let count = signal(initial);
///     rsx! { Button { on_click: move |_| count.update(|c| *c += step), {move || count.get()} } }
/// }
///
/// rsx! { Counter { initial: 5 } }        // or counter().initial(5)
/// ```
///
/// The name is in upper camel case. Besides the function, the macro declares
/// a type of the name (what `rsx!` names: `Counter { .. }`) and a builder
/// function of the name in snake case (`counter()`), with a setter per parameter that takes what
/// converts into its type (`IntoValue`: the conversions of the setters of
/// controls). A parameter is required unless it has
/// `#[prop(default)]` (the default of its type), `#[prop(default = expr)]`,
/// or an `Option` type. A parameter named `children` of type `Element` takes
/// the children of the element (`.child(..)`). The function runs once, when
/// the element is built.
///
/// `#[component(rerender)]` makes a component that runs again: its function
/// runs whenever a signal it read changes or its parent runs again with
/// other parameters, the elements it returns are patched into the controls it
/// made before, and its state is kept in hooks (`use_signal`, `use_effect`,
/// ...). Its parameters are cloned for each run, so they are `Clone`. When
/// they are also `PartialEq` it is memoized: a parent that runs again with
/// equal parameters does not run it again. `#[component(rerender, memo)]`
/// requires that (a compile error otherwise), and
/// `#[component(rerender, always)]` runs it with its parent every time.
#[proc_macro_attribute]
pub fn component(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let mut options = ComponentOptions::default();
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("rerender") {
            options.rerender = true;
            Ok(())
        } else if meta.path.is_ident("memo") {
            options.memo = true;
            Ok(())
        } else if meta.path.is_ident("always") {
            options.always = true;
            Ok(())
        } else {
            Err(meta.error("expected `rerender`, `memo` or `always`"))
        }
    });
    syn::parse_macro_input!(attribute with parser);
    if (options.memo || options.always) && !options.rerender {
        return syn::Error::new(Span::call_site(), "`memo` and `always` are options of a component that runs again: `#[component(rerender, memo)]`")
            .to_compile_error()
            .into();
    }
    if options.memo && options.always {
        return syn::Error::new(Span::call_site(), "`memo` and `always` contradict each other").to_compile_error().into();
    }
    let function = syn::parse_macro_input!(item as ItemFn);
    match expand_component(function, options) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[derive(Default, Clone, Copy)]
struct ComponentOptions {
    rerender: bool,
    /// Memoized, and its parameters must be `PartialEq`.
    memo: bool,
    /// Never memoized.
    always: bool,
}

struct Param {
    name: Ident,
    ty: Type,
    default: Option<TokenStream2>,
    attrs: Vec<syn::Attribute>,
}

fn is_option(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "Option"))
}

fn expand_component(mut function: ItemFn, options: ComponentOptions) -> syn::Result<TokenStream2> {
    let name = function.sig.ident.clone();
    let name_text = name.to_string();
    if !name_text.chars().next().is_some_and(char::is_uppercase) {
        return Err(syn::Error::new(name.span(), "a component is named in upper camel case (`TodoList`)"));
    }
    let builder_function = Ident::new(&snake_case(&name_text), name.span());
    let builder_type = format_ident!("{}Builder", name);
    let visibility = function.vis.clone();

    let mut params = Vec::new();
    for input in function.sig.inputs.iter_mut() {
        let FnArg::Typed(typed) = input else {
            return Err(syn::Error::new(input.span(), "a component is a free function"));
        };
        let Pat::Ident(pattern) = &*typed.pat else {
            return Err(syn::Error::new(typed.pat.span(), "a parameter of a component is a name"));
        };
        let mut default = None;
        let mut kept = Vec::new();
        for attr in typed.attrs.drain(..) {
            if attr.path().is_ident("prop") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("default") {
                        if meta.input.peek(Token![=]) {
                            let value: Expr = meta.value()?.parse()?;
                            default = Some(value.to_token_stream());
                        } else {
                            default = Some(quote! { ::std::default::Default::default() });
                        }
                        Ok(())
                    } else {
                        Err(meta.error("expected `default` or `default = ...`"))
                    }
                })?;
            } else {
                kept.push(attr);
            }
        }
        typed.attrs = kept.clone();
        let ty = (*typed.ty).clone();
        if default.is_none() && is_option(&ty) {
            default = Some(quote! { ::std::option::Option::None });
        }
        params.push(Param { name: pattern.ident.clone(), ty, default, attrs: kept });
    }

    let fields = params.iter().map(|param| {
        let (name, ty) = (&param.name, &param.ty);
        quote! { #name: ::std::option::Option<#ty> }
    });
    let setters = params.iter().map(|param| {
        let (name, ty) = (&param.name, &param.ty);
        let docs = param.attrs.iter().filter(|attr| attr.path().is_ident("doc"));
        if name == "children" {
            quote! {
                #(#docs)*
                pub fn children(mut self, children: impl ::ferroui_view::IntoChildren) -> Self {
                    let mut items = match self.children.take() {
                        ::std::option::Option::Some(::ferroui_view::Element::Fragment(items)) => items,
                        _ => ::std::vec::Vec::new(),
                    };
                    items.extend(::ferroui_view::IntoChildren::into_children(children));
                    self.children = ::std::option::Option::Some(::ferroui_view::Element::Fragment(items));
                    self
                }

                /// Adds a child.
                pub fn child(mut self, child: impl ::ferroui_view::IntoChild) -> Self {
                    let mut items = match self.children.take() {
                        ::std::option::Option::Some(::ferroui_view::Element::Fragment(items)) => items,
                        _ => ::std::vec::Vec::new(),
                    };
                    items.push(::ferroui_view::IntoChild::into_child(child));
                    self.children = ::std::option::Option::Some(::ferroui_view::Element::Fragment(items));
                    self
                }
            }
        } else {
            quote! {
                #(#docs)*
                pub fn #name(mut self, value: impl ::ferroui_view::IntoValue<#ty>) -> Self {
                    self.#name = ::std::option::Option::Some(::ferroui_view::IntoValue::into_value(value));
                    self
                }
            }
        }
    });
    let arguments = params.iter().map(|param| {
        let name = &param.name;
        let message = format!("`{}` is a required property of `{}`", name, name_text);
        match &param.default {
            Some(default) => quote! { self.#name.unwrap_or_else(|| #default) },
            None if name == "children" => quote! { self.#name.unwrap_or_default() },
            None => quote! { self.#name.unwrap_or_else(|| panic!(#message)) },
        }
    });
    let field_names: Vec<&Ident> = params.iter().map(|param| &param.name).collect();
    let doc = format!("The builder of the component [`{name_text}`].");
    let start_doc = format!("Starts an element of the component [`{name_text}`].");
    let type_doc = format!("The component [`{name_text}`] as the name of an element of `rsx!`.");

    let into_element = if options.rerender {
        let props = if options.memo {
            quote! { ::std::option::Option::Some(::std::rc::Rc::new(::std::clone::Clone::clone(&props)) as ::std::rc::Rc<dyn ::ferroui_view::element::AnyValueEq>) }
        } else if options.always {
            quote! { ::std::option::Option::None }
        } else {
            // Memoized when the parameters can be compared, which the types
            // of the expansion decide (autoref specialization).
            quote! {{
                #[allow(unused_imports)]
                use ::ferroui_view::element::{MemoNever as _, MemoWhenComparable as _};
                (&::ferroui_view::element::MemoProbe(::std::clone::Clone::clone(&props))).memo_props()
            }}
        };
        quote! {
            impl ::ferroui_view::IntoElement for #builder_type {
                fn into_element(self) -> ::ferroui_view::Element {
                    let props = (#(#arguments,)*);
                    let memo = #props;
                    ::ferroui_view::element::component_element::<#name>(
                        #name_text,
                        self.key,
                        move || {
                            let (#(#field_names,)*) = ::std::clone::Clone::clone(&props);
                            #name(#(#field_names),*)
                        },
                        memo,
                    )
                }
            }
        }
    } else {
        quote! {
            impl ::ferroui_view::IntoElement for #builder_type {
                fn into_element(self) -> ::ferroui_view::Element {
                    let element = #name(#(#arguments),*);
                    match self.key {
                        ::std::option::Option::Some(key) => ::ferroui_view::element::with_key(element, key),
                        ::std::option::Option::None => element,
                    }
                }
            }
        }
    };

    function.attrs.push(syn::parse_quote!(#[allow(non_snake_case)]));
    Ok(quote! {
        #function

        #[doc = #doc]
        #[allow(non_camel_case_types)]
        #visibility struct #builder_type {
            #(#fields,)*
            key: ::std::option::Option<::ferroui_view::Key>,
        }

        #[doc = #type_doc]
        #[allow(non_camel_case_types)]
        #visibility struct #name {}

        impl ::ferroui_view::Builder for #name {
            type Output = #builder_type;
            fn builder() -> #builder_type {
                #builder_function()
            }
        }

        #[doc = #start_doc]
        #visibility fn #builder_function() -> #builder_type {
            #builder_type { #(#field_names: ::std::option::Option::None,)* key: ::std::option::Option::None }
        }

        impl #builder_type {
            #(#setters)*

            /// The key of the element among its siblings.
            pub fn key(mut self, key: impl ::std::convert::Into<::ferroui_view::Key>) -> Self {
                self.key = ::std::option::Option::Some(key.into());
                self
            }
        }

        #into_element
    })
}
