//! The emitter of Rust source: the build-time back end over the SAME
//! transformed AST the interpreter ([`crate::runtime::interpreter`])
//! evaluates (docs/porting/xaml.md, section 9).
//!
//! # What this increment emits
//!
//! For a document, one function that builds its object tree with direct,
//! typed Rust calls, in the order the interpreter performs them:
//!
//! ```ignore
//! pub fn build_border_xaml(
//!     service_provider: Option<::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>,
//! ) -> Result<::ferroui_base::Ref<::ferroui_controls::Border>, ::ferroui_markup_xaml::XamlLoadException> {
//!     // border.xaml(1,2) Border
//!     let border_0 = ::ferroui_controls::Border::new();
//!     let name_scope = rt::name_scope_of(service_provider.as_ref());
//!     border_0.begin_init();
//!     // border.xaml(1,2) Padding
//!     border_0.set_value(::ferroui_controls::Decorator::padding_property(), ::ferroui_base::Thickness::new(1.0_f64, ..));
//!     border_0.try_end_init().map_err(|error| rt::at(error, 1, 2))?;
//!     rt::complete_root_name_scope(Some(&border_0), name_scope.as_ref(), 1, 2)?;
//!     Ok(border_0)
//! }
//! ```
//!
//! `rt` is `ferroui_markup_xaml::xaml_il::runtime::compiled`: the steps
//! whose failure is a load error (name registration, the scope of the root,
//! the position of an error) and the conversion of a value to `object`,
//! shared with the run-time loader so that both fail and convert alike.
//!
//! rustc type-checks every call: a value whose Rust type cannot be stated
//! exactly is never emitted. A document that contains anything outside the
//! supported set is NOT ELIGIBLE ([`UnsupportedNode`]): nothing approximate
//! is ever written, and such a document keeps loading through the run-time
//! loader.
//!
//! The supported nodes: the root object and child objects of classes of the
//! object model with a default constructor (with the top-down `BeginInit`
//! of objects usable during initialisation, and the compiler locals that
//! carry them), object initialisation (`BeginInit` / `EndInit`), assignments
//! of registered properties (styled, attached and direct) through the
//! accessors the type system projects for them, text, numeric, boolean,
//! character and enumeration constants, `{x:Null}`, `{x:Static}` of an
//! enumeration member, the vector-like and grid length constants, name
//! registration and the scope of the root object.
//!
//! # Where the Rust names come from
//!
//! | What | Source |
//! |---|---|
//! | the path of a class | `TypeInfo::rust_path()`: the type tables of the crates (`register_types.rs`) |
//! | the path of an enumeration or value type | `MarkupType::rust_path()` (`MarkupType::register_rust_paths`) |
//! | an enumeration member | `MarkupEnumMember::rust_variant` (the declaration macro) |
//! | a constructor with arguments | `MarkupConstructor::emit` (`stringify!` of the declared callable), used only in the form `<TypeName>::<function>` |
//! | the default constructor of a class | the convention `Type::new()` (porting guide, "Classes"), checked against `TypeInfo::default_constructor` |
//! | the definition of a registered property | the accessor the declaration macro recorded for it (`ferroui_base::metadata::property_accessors`, feature `compiler-metadata` of the base crate, which the `emitter` feature of this crate enables) |
//!
//! The emitter never resolves a member by name itself: the member is the one
//! the transformers put into the AST, and the Rust type of every value is
//! compared (by `TypeId`) with the value type of the registered property it
//! is assigned to.
//!
//! # Not in this increment
//!
//! Call form C (the untyped invokers) is not emitted at all: a node that
//! would need it makes the document not eligible. That covers plain
//! (non-registered) properties, declared accessors of attached properties,
//! collection adds, markup extensions, bindings, templates and deferred
//! content, styles, resources, event handlers, `x:Type`, flags
//! enumerations, constructor arguments and everything that needs the parent
//! stack. The transform still runs against the run-time type system (the
//! host of the emitter links the framework); the source scanner of section
//! 9.5 replaces that later.

mod compiled;
mod emitter;
mod source;

pub use compiled::{compile_documents, generate_file, CompiledDocument, GeneratedFile};
#[cfg(any(test, feature = "testing"))]
pub use compiled::transformed_tree;
pub use emitter::{emit_document, UnsupportedNode};
pub use source::{function_name_of, rust_string_literal};
