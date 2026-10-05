//! The interpreter of the transformed AST: the run-time replacement of the
//! IL back end of the compiler library.
//!
//! | Module | Upstream | Contents |
//! |---|---|---|
//! | [`interpreter`](self) | `IL/XamlIlCompiler.cs`, `Emit/XamlEmitContext.cs`, `IL/ILEmitHelpers.cs` | [`Interpreter`], [`EvalContext`], the evaluator contracts, conversions |
//! | `evaluators` | `IL/Emitters/*.cs`, the `Emit` methods of `Ast/*.cs` | the evaluation of the standard nodes |
//! | `runtime_context` | `IL/RuntimeContext.cs`, `IL/NamespaceInfoProvider.cs` | [`RuntimeContext`], the namespace information of a document |
//! | [`services`] | `XamlX.Runtime/Interfaces.cs` | a self-contained set of the service contracts |

mod evaluators;
mod interpreter;
mod runtime_context;
pub mod services;

pub use evaluators::{constant_value, AssignmentPlan, StandardNodeEvaluator};
pub use interpreter::{
    runtime_error, EvalContext, EvalResult, IXamlAstEvaluableNode, IXamlConstructorEvaluator,
    IXamlEvaluablePropertySetter,
    IXamlEvaluableWrappedMethod, IXamlMethodEvaluator, IXamlNodeEvaluator, IXamlSetterEvaluator,
    IXamlWrappedMethodEvaluator, Interpreter, ProvideValueTargetPropertyEvaluator, RuntimeContextCallback,
    RuntimeDocument,
};
pub use runtime_context::{
    namespace_info_static_provider, IRuntimeContextServices, IStaticServiceProvider, RuntimeContext,
    RuntimeContextDefinition, RuntimeContextService, XamlXmlNamespaceInfo, XmlNamespaceInfoProvider,
};
pub use services::DefaultRuntimeContextServices;

#[cfg(test)]
pub(crate) mod tests;
