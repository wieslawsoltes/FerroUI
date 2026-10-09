//! What the two back ends share: the run-time back end ([`crate::runtime`], the
//! interpreter of the transformed AST) and the build-time back end
//! ([`crate::rust_emitter`], the emitter of Rust source over the same AST).
//!
//! Everything here is decided from the transformed AST and the transformer
//! configuration alone, against any type system: nothing of the XAML runtime library is
//! named, so a host of the emitter that reads its types from the type models of the
//! crates (`ferroui-build`; docs/porting/xaml.md, 9.5) links neither that library nor the
//! controls.
//!
//! | Module | Contents |
//! |---|---|
//! | `assignment` | the plan of a property assignment (the setters it chooses from), the reading of a numeric constant |
//! | `context` | what the language configured the context of a document to be, and the context of the framework language |
//! | `includes` | the sources of the include elements of a document, from its text |
//! | `methods` | the deferred content customisation as a generic method definition, the build and populate methods of the documents of a group |
//! | `namespaces` | the namespace information of a parsed document |

mod assignment;
mod context;
mod includes;
mod methods;
mod namespaces;

pub use assignment::{numeric_constant, plan_setters, AssignmentPlan};
#[cfg(feature = "runtime")]
pub(crate) use assignment::{assignment_plan, last_parameter};
pub use context::{context_definition, ContextDefinition, FRAMEWORK_CONTEXT};
pub use includes::include_sources;
#[cfg(feature = "runtime")]
pub(crate) use methods::{document_method, DocumentMethodSignature};
pub use methods::{
    adapt_type_mappings, DeferredTransformationFactoryMethod, DocumentBuildSignature, DocumentPopulateSignature, DocumentTypeBuilderProvider,
};
pub use namespaces::{XamlXmlNamespaceInfo, XmlNamespaceInfoProvider};
