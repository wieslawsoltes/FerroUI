//! The emitter of Rust source: the build-time back end over the SAME
//! transformed AST the interpreter (`crate::runtime::interpreter`)
//! evaluates (docs/porting/xaml.md, section 9).
//!
//! # What this increment emits
//!
//! For a document, one function that builds its object tree with direct,
//! typed Rust calls, in the order the interpreter performs them:
//!
//! ```ignore
//! pub fn build_border_xaml(
//!     service_provider: ::core::option::Option<::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>,
//! ) -> ::core::result::Result<::ferroui_base::Ref<::ferroui_controls::Border>, ::ferroui_markup_xaml::XamlLoadException> {
//!     // border.xaml(1,2) Border
//!     let border_0 = ::ferroui_controls::Border::new();
//!     let name_scope = rt::name_scope_of(service_provider.as_ref());
//!     border_0.begin_init();
//!     // border.xaml(1,2) Padding
//!     border_0.set_value(::ferroui_controls::Decorator::padding_property(), ::ferroui_base::Thickness::new(1.0_f64, ..));
//!     rt::invoked(border_0.try_end_init(), 1, 2)?;
//!     rt::complete_root_name_scope(::core::option::Option::Some(&border_0), name_scope.as_ref(), 1, 2)?;
//!     ::core::result::Result::Ok(border_0)
//! }
//! ```
//!
//! `rt` is `ferroui_markup_xaml::xaml_il::runtime::compiled`: the steps
//! whose failure is a load error (name registration, the scope of the root,
//! the position of an error) and the conversion of a value to `object`,
//! shared with the run-time loader so that both fail (with the same message
//! and exception type) and convert alike.
//!
//! rustc type-checks every call: a value whose Rust type cannot be stated
//! exactly is never emitted. A document that contains anything outside the
//! supported set is NOT ELIGIBLE ([`UnsupportedNode`]): nothing approximate
//! is ever written, and such a document keeps loading through the run-time
//! loader.
//!
//! The supported nodes: objects of classes of the object model (default
//! constructors, the top-down `BeginInit` of objects usable during
//! initialisation, the compiler locals that carry them) and of markup types
//! (declared constructors, with arguments), object initialisation
//! (`BeginInit` / `EndInit`), assignments of registered properties (styled,
//! attached and direct) through the accessors the type system projects for
//! them, assignments through declared accessors (plain properties, static
//! accessors of attached properties), collection adds (`AdderSetter`, with a
//! declared or registered getter and a declared adder), method calls of
//! declared methods and `Parse`, list constants, text, numeric, boolean,
//! character and enumeration constants, `{x:Null}`, `{x:Static}` of an
//! enumeration member, the vector-like and grid length constants, name
//! registration and the scope of the root object.
//!
//! # Where the Rust names come from
//!
//! | What | Source |
//! |---|---|
//! | the path of a class | `TypeInfo::rust_path()`: the generated `rust_paths.rs` of the crate (`scripts/rust_paths.py`: the shortest public path, from the module tree of the crate) |
//! | the path of an enumeration, value type, contract or generic instantiation | `MarkupType::rust_path()`, from the same file |
//! | an enumeration member | `MarkupEnumMember::rust_variant` (the declaration macro) |
//! | a declared member (constructor, property accessor, method, `Parse`) | its typed function, an associated function of the declared type the declaration macros generate (`MarkupEmit`: `__markup_new_0`, `__markup_get_Child`, `__markup_Add_0`, `__markup_parse`) |
//! | the default constructor of a class | the convention `Type::new()` (porting guide, "Classes"), checked against `TypeInfo::default_constructor` |
//! | the definition of a registered property | a public accessor the declaration macro recorded for it, by the path of the type whose `impl` declares it (`ferroui_base::metadata::property_accessors`, `rust_path_of_type`; feature `compiler-metadata` of the base crate, which the `emitter` feature of this crate enables), the first in declaration order |
//! | a conversion without a static form (an interface handle, a registered cast) | `rt::cast`, the loader's own cast, where `ValueTypes::is_assignable` proves it exists |
//!
//! Everything the emitter reads beyond the run-time metadata (the path
//! tables, the names of the typed functions, the instance and value types
//! of declarations) and the typed functions themselves exist only with the
//! `compiler-metadata` feature of the base crate (`metadata::CompilerMetadata`,
//! `__ferro_compiler_metadata!`): a default build carries none of it. Generated
//! code calls the typed functions, so a crate that compiles generated code
//! enables the feature.
//!
//! The emitter never resolves a member by name itself: the member is the one
//! the transformers put into the AST, and the Rust type of every value is
//! compared (by `TypeId`) with the value type of the registered property it
//! is assigned to.
//!
//! # Not in this increment
//!
//! Call form C (the untyped invokers) is not emitted at all: a node that
//! would need it makes the document not eligible. That covers members of
//! metadata without a typed function, event handlers and indexers.
//!
//! # The type system
//!
//! The transform ([`transform_group`]) runs against any type system, and the
//! emitter reads what it needs beyond the contracts of the compiler through
//! [`emit_types::EmitTypes`]. The host of this crate is the run-time type
//! system (`EmitterHost::runtime`, `runtime_types::RuntimeEmitTypes`: the
//! host links the framework and the types are the ones the process
//! registered); the build-time type system over the models of the crates
//! (`ferroui-build`, section 9.5) is the other implementation.
//!
//! # Features
//!
//! The transform, the emitter and the file of a group against the type system a
//! host states are the `compiler` feature, which links neither the XAML runtime
//! library nor the controls: what is shared with the interpreter is read from the
//! transformed AST alone ([`crate::back_end`]). The run-time host (the functions
//! that take the configuration of the run-time loader, `EmitterHost::runtime`,
//! `runtime_types`) is the `emitter` feature, which adds the `runtime` feature
//! and the accessor table of the base crate.

mod compiled;
mod compiled_resources;
pub mod emit_types;
mod emitter;
/// The run-time host: with the `emitter` feature (and in the tests of the crate with the
/// `runtime` feature), which links the XAML runtime library.
#[cfg(any(feature = "emitter", all(test, feature = "runtime")))]
mod runtime_host;
#[cfg(any(feature = "emitter", all(test, feature = "runtime")))]
pub mod runtime_types;
mod source;
mod transform;

pub use compiled::{
    class_document_group, class_of_document, compile_documents_with, generate_class_file_with, generate_file_with, ClassConstructor, ClassFile,
    ClassGroup, CompiledDocument, EmitterHost, GeneratedFile,
};
#[cfg(any(feature = "emitter", all(test, feature = "runtime")))]
pub use runtime_host::{compile_documents, generate_class_file, generate_file};
#[cfg(all(any(test, feature = "testing"), any(feature = "emitter", all(test, feature = "runtime"))))]
pub use runtime_host::{transformed_class_group, transformed_tree};
pub use transform::{transform_group, DiagnosticHandler, DocumentSource, TransformOptions, TransformedDocument};
pub use compiled_resources::CompiledMarkupTypeSystem;
pub use compiled_resources::CompiledDocumentBuildMethod;
pub use emitter::{emit_document, UnsupportedNode};
pub use ferroui_build_scan::xaml_metadata::{DocumentModel, XamlMetadata};
pub use source::{function_name_of, rust_string_literal};
