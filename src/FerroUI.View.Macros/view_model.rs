//! `#[derive(ViewModel)]`: the properties of a view model from one declaration, so that the name a
//! change is raised with, the accessors and the typed property of a binding cannot disagree.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, Data, DeriveInput, Field, Fields, GenericArgument, Ident, PathArguments, Token, Type};

/// `#[computed(greeting: String, depends_on(name, title))]`.
struct Computed {
    name: Ident,
    value: Type,
    depends_on: Vec<Ident>,
}

impl Parse for Computed {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let value: Type = input.parse()?;
        input.parse::<Token![,]>()?;
        let keyword: Ident = input.parse()?;
        if keyword != "depends_on" {
            return Err(syn::Error::new(keyword.span(), "expected `depends_on(..)`, the properties this one is made of"));
        }
        let content;
        parenthesized!(content in input);
        let depends_on = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?.into_iter().collect();
        Ok(Self { name, value, depends_on })
    }
}

/// How a property field stores its value.
enum Storage {
    Cell,
    RefCell,
}

/// The storage and the value type of a `Cell<T>` or `RefCell<T>`.
fn storage_of(field: &Field) -> syn::Result<(Storage, &Type)> {
    let error = || syn::Error::new_spanned(&field.ty, "a `#[property]` field is a `Cell<T>` or a `RefCell<T>`");
    let Type::Path(path) = &field.ty else { return Err(error()) };
    let segment = path.path.segments.last().ok_or_else(error)?;
    let storage = match segment.ident.to_string().as_str() {
        "Cell" => Storage::Cell,
        "RefCell" => Storage::RefCell,
        _ => return Err(error()),
    };
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else { return Err(error()) };
    match arguments.args.first() {
        Some(GenericArgument::Type(value)) if arguments.args.len() == 1 => Ok((storage, value)),
        _ => Err(error()),
    }
}

fn is_event(field: &Field) -> bool {
    matches!(&field.ty, Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "Event"))
}

fn constant_of(name: &Ident) -> Ident {
    format_ident!("{}", name.unraw().to_string().to_uppercase(), span = name.span())
}

/// The expansion of `#[derive(ViewModel)]`.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let model = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(&input.generics, "a `ViewModel` has no generic parameters"));
    }
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(model, "`ViewModel` is derived for a struct with named fields"));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(model, "`ViewModel` is derived for a struct with named fields"));
    };
    let mut events = fields.named.iter().filter(|field| is_event(field));
    let Some(event) = events.next() else {
        return Err(syn::Error::new_spanned(model, "a `ViewModel` holds its notification: add `property_changed: Event<str>`"));
    };
    if let Some(second) = events.next() {
        return Err(syn::Error::new_spanned(second, "a `ViewModel` has one `Event<str>` field"));
    }
    let event = &event.ident;

    let properties: Vec<&Field> =
        fields.named.iter().filter(|field| field.attrs.iter().any(|a| a.path().is_ident("property"))).collect();
    let mut computed = Vec::new();
    for attribute in input.attrs.iter().filter(|attribute| attribute.path().is_ident("computed")) {
        computed.push(attribute.parse_args::<Computed>()?);
    }
    for item in &computed {
        for dependency in &item.depends_on {
            if !properties.iter().any(|field| field.ident.as_ref() == Some(dependency)) {
                let message = format!("`{dependency}` is not a `#[property]` of `{model}`");
                return Err(syn::Error::new(dependency.span(), message));
            }
        }
    }

    let mut members = Vec::new();
    for field in &properties {
        let Some(name) = &field.ident else { continue };
        let vis = &field.vis;
        let (storage, value) = storage_of(field)?;
        let text = name.unraw().to_string();
        let setter = format_ident!("set_{}", name.unraw(), span = name.span());
        let constant = constant_of(name);
        let raises = computed
            .iter()
            .filter(|item| item.depends_on.contains(name))
            .map(|item| item.name.unraw().to_string());
        let (get, changed, store) = match storage {
            Storage::Cell => (
                quote! { self.#name.get() },
                quote! { self.#name.get() != value },
                quote! { self.#name.set(value); },
            ),
            Storage::RefCell => (
                quote! { ::std::clone::Clone::clone(&*self.#name.borrow()) },
                quote! { *self.#name.borrow() != value },
                quote! { *self.#name.borrow_mut() = value; },
            ),
        };
        let doc_get = format!("The property `{text}`.");
        let doc_set = format!("Sets the property `{text}` and raises its change when the value differs.");
        let doc_constant = format!("The property `{text}`, for a typed binding.");
        members.push(quote! {
            #[doc = #doc_get]
            #vis fn #name(&self) -> #value {
                #get
            }

            #[doc = #doc_set]
            #vis fn #setter(&self, value: #value) {
                if #changed {
                    #store
                    self.#event.raise(#text);
                    #(self.#event.raise(#raises);)*
                }
            }

            #[doc = #doc_constant]
            #vis const #constant: ::ferroui_view::ModelProperty<Self, #value> =
                ::ferroui_view::ModelProperty::__declared(#text, Self::#name, Self::#setter);
        });
    }
    let vis = &input.vis;
    for item in &computed {
        let (name, value) = (&item.name, &item.value);
        let text = name.unraw().to_string();
        let constant = constant_of(name);
        let doc_constant = format!("The computed property `{text}`, for a typed binding.");
        members.push(quote! {
            #[doc = #doc_constant]
            #vis const #constant: ::ferroui_view::ReadOnlyModelProperty<Self, #value> =
                ::ferroui_view::ReadOnlyModelProperty::__declared(#text, Self::#name);
        });
    }
    Ok(quote! {
        impl ::ferroui_view::__model::INotifyPropertyChanged for #model {
            fn property_changed(&self) -> &::ferroui_view::__model::Event<str> {
                &self.#event
            }
        }

        impl ::std::cmp::PartialEq for #model {
            fn eq(&self, other: &Self) -> bool {
                ::std::ptr::eq(self, other)
            }
        }

        impl #model {
            #(#members)*
        }
    })
}
