//! Port of `XamlTypes.cs`: the services markup extensions and converters
//! ask the service provider for.

use crate::XamlLoadException;
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::utilities::Uri;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::rc::Rc;

/// Tells a markup extension what its value is provided for.
pub trait IProvideValueTarget {
    /// The object whose property is being set.
    fn target_object(&self) -> Option<BoxedValue>;

    /// The property being set: a registered property
    /// (`&'static FerroProperty`), a plain property
    /// (`Rc<dyn IPropertyInfo>`), or anything else the document provides
    /// (the name of a property).
    fn target_property(&self) -> Option<BoxedValue>;
}

/// Gives access to the root object of the document being loaded.
pub trait IRootObjectProvider {
    /// The root object of the document (for content built from a template:
    /// the root of the document that declares the template).
    fn root_object(&self) -> Option<BoxedValue>;

    /// The root object of the part of the document being built: inside a
    /// template, the root of the template content.
    fn intermediate_root_object(&self) -> Option<BoxedValue>;
}

/// Gives access to the base URI of the document being loaded.
pub trait IUriContext {
    fn base_uri(&self) -> Option<Uri>;

    fn set_base_uri(&self, value: Option<Uri>);
}

/// Resolves type names qualified with a namespace prefix of the document
/// (`local:MyControl`).
pub trait IXamlTypeResolver {
    /// Resolves `qualified_type_name` (`prefix:Name` or `Name`) to a type: a
    /// class of the object model or a value type.
    fn resolve(&self, qualified_type_name: &str) -> Result<CastTarget, XamlLoadException>;
}

macro_rules! identity_eq {
    ($($trait_:ident),*) => {$(
        /// Handles compare by identity, so that they can be held in untyped
        /// values and returned by service providers.
        impl PartialEq for dyn $trait_ {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::addr_eq(self, other)
            }
        }
    )*};
}

identity_eq!(IProvideValueTarget, IRootObjectProvider, IUriContext, IXamlTypeResolver);

ferro_markup_type!(interface dyn IProvideValueTarget as "IProvideValueTarget" {
    this: Rc<dyn IProvideValueTarget>,
    handles: [Rc<dyn IProvideValueTarget>, Option<Rc<dyn IProvideValueTarget>>],
    properties: [
        TargetObject: Option<BoxedValue> { get: |t: &Rc<dyn IProvideValueTarget>| t.target_object() },
        TargetProperty: Option<BoxedValue> { get: |t: &Rc<dyn IProvideValueTarget>| t.target_property() },
    ],
});

ferro_markup_type!(interface dyn IRootObjectProvider as "IRootObjectProvider" {
    this: Rc<dyn IRootObjectProvider>,
    handles: [Rc<dyn IRootObjectProvider>, Option<Rc<dyn IRootObjectProvider>>],
    properties: [
        RootObject: Option<BoxedValue> { get: |t: &Rc<dyn IRootObjectProvider>| t.root_object() },
        IntermediateRootObject: Option<BoxedValue> {
            get: |t: &Rc<dyn IRootObjectProvider>| t.intermediate_root_object()
        },
    ],
});

ferro_markup_type!(interface dyn IUriContext as "IUriContext" {
    this: Rc<dyn IUriContext>,
    handles: [Rc<dyn IUriContext>, Option<Rc<dyn IUriContext>>],
    properties: [
        BaseUri: Option<Uri> {
            get: |t: &Rc<dyn IUriContext>| t.base_uri(),
            set: |t: &Rc<dyn IUriContext>, value: Option<Uri>| t.set_base_uri(value)
        },
    ],
});

ferro_markup_type!(interface dyn IXamlTypeResolver as "IXamlTypeResolver" {
    this: Rc<dyn IXamlTypeResolver>,
    handles: [Rc<dyn IXamlTypeResolver>, Option<Rc<dyn IXamlTypeResolver>>],
});
