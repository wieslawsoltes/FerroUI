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
//! pub fn build_border(
//!     service_provider: Option<std::rc::Rc<dyn ferroui_base::metadata::IServiceProvider>>,
//! ) -> Result<ferroui_base::Ref<ferroui_controls::Border>, ferroui_markup_xaml::XamlLoadException> {
//!     let v0 = ferroui_controls::Border::new();
//!     let name_scope = ..;                                   // the name scope field of the context
//!     v0.begin_init();
//!     v0.set_value(ferroui_controls::Decorator::padding_property(), ferroui_base::Thickness::new(1.0_f64, ..));
//!     v0.try_end_init().map_err(..)?;
//!     ferroui_base::controls::NameScope::set_name_scope(&v0, ..);
//!     ..
//!     Ok(v0)
//! }
//! ```
//!
//! rustc type-checks every call: a value whose Rust type cannot be stated
//! exactly is never emitted. A document that contains anything outside the
//! supported set is NOT ELIGIBLE ([`UnsupportedNode`]): nothing approximate
//! is ever written, and such a document keeps loading through the run-time
//! loader.
//!
//! # Where the Rust names come from
//!
//! | What | Source |
//! |---|---|
//! | the path of a class | `TypeInfo::rust_path()`: the type tables of the crates (`register_types.rs`) |
//! | the path of an enumeration or value type | `MarkupType::rust_path()` (`MarkupType::register_rust_paths`) |
//! | an enumeration member | `MarkupEnumMember::rust_variant` (the declaration macro) |
//! | a constructor with arguments | `MarkupConstructor::emit` (`stringify!` of the declared callable), used only in the form `<TypeName>::<function>` |
//! | the default constructor of a class | the convention `Type::new()` (porting guide, "Classes") |
//! | the definition of a registered property | the convention `Owner::<snake_name>_property()` (porting guide, "Naming") |
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
//! (non-registered) properties, direct properties, collection adds, markup
//! extensions, bindings, templates and deferred content, styles, resources,
//! event handlers, `x:Type` and everything that needs the parent stack. The
//! transform still runs against the run-time type system (the host of the
//! emitter links the framework); the source scanner of section 9.5 replaces
//! that later.

mod compiled;
mod emitter;
mod source;

pub use compiled::{compile_documents, generate_file, CompiledDocument, GeneratedFile};
pub use emitter::{emit_document, UnsupportedNode};
pub use source::{function_name_of, rust_string_literal};
