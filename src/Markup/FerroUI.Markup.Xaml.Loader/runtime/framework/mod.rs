//! The run-time back end of the framework language: the evaluators of the
//! nodes, property setters and pseudo methods the compiler extensions
//! leave to a back end (the table "Nodes and setters the back ends must
//! implement" of [`crate::compiler_extensions`]), and the run-time context
//! mapped onto the service contracts of the XAML runtime library.
//!
//! | Module | Contents |
//! |---|---|
//! | [`services`] | the context as the services of the runtime library, the name scope field, the eager parent stack |
//! | `nodes` | value and manipulation nodes: constants, registered properties, selectors, queries, name scopes, source info |
//! | `setters` | the property setters |
//! | `methods` | pseudo methods, the deferred content factory, the build and populate methods of documents |
//! | `binding_path` | compiled binding paths |

mod binding_path;
mod helpers;
mod methods;
mod nodes;
pub mod services;
mod setters;

use std::rc::Rc;

use xamlx::exceptions::XamlResult;
use xamlx::transform::{TransformerConfiguration, XamlLanguageTypeMappings};

use crate::compiler_extensions::group_transformers::XamlRuntimeIncludeFallback;
use crate::compiler_extensions::{FerroXamlIlLanguageEmitMappings, XamlCompileTimeValueParsers};
use crate::runtime::interpreter::Interpreter;
use crate::runtime::type_system::RuntimeTypeSystem;
use crate::runtime::RuntimeCompileTimeValueParser;

pub use methods::{
    DeferredTransformationFactoryMethod, DocumentBody, DocumentBuildMethod, DocumentPopulateMethod,
    FrameworkMethodEvaluator, RuntimeDocumentTypeBuilderProvider, XamlMemberException,
};
pub(crate) use methods::load_exception;
pub use nodes::{provide_value_target_property, FrameworkNodeEvaluator};
pub use services::FerroRuntimeContextServices;
pub use setters::FrameworkSetterEvaluator;

/// What implements a row of the table of [`crate::compiler_extensions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameworkEvaluatorKind {
    /// [`FrameworkNodeEvaluator`].
    Node,
    /// [`FrameworkSetterEvaluator`].
    Setter,
    /// [`FrameworkMethodEvaluator`].
    Method,
    /// An element of a compiled binding path, evaluated with its path node.
    BindingPathElement,
    /// A member of the run-time context ([`services`]).
    ContextMember,
    /// The provide-value target property hook of the interpreter.
    EmitMapping,
    /// An IL helper whose work the evaluation of binding paths does itself
    /// (a base `ClrPropertyInfo` over the invokers, the accessor factories of the runtime library).
    EmitHelper,
}

/// Every row of the table "Nodes and setters the back ends must implement"
/// with what evaluates it here. Kept next to the registration in
/// [`configure_interpreter`]; a test compares it with the table.
pub const FRAMEWORK_EVALUATORS: &[(&str, FrameworkEvaluatorKind)] = {
    use FrameworkEvaluatorKind::*;
    &[
        ("FerroXamlIlVectorLikeConstantAstNode", Node),
        ("FerroXamlIlGridLengthAstNode", Node),
        ("FerroXamlIlFontFamilyAstNode", Node),
        ("FerroXamlIlArrayConstantAstNode", Node),
        ("FerroXamlIlFerroListConstantAstNode", Node),
        ("XamlIlFerroPropertyNode", Node),
        ("XamlIlFerroPropertyFieldNode", Node),
        ("XamlIlFerroClassProperty", Node),
        ("BindingSetter", Setter),
        ("BindingWithPrioritySetter", Setter),
        ("SetValueWithPrioritySetter", Setter),
        ("UnsetValueSetter", Setter),
        ("try_get_provide_value_target", EmitMapping),
        ("FerroXamlIlContextNameScopeField", ContextMember),
        ("FerroXamlIlContextEagerParentStackProvider", ContextMember),
        ("XamlIlSelectorInitialNode", Node),
        ("XamlIlTypeSelector", Node),
        ("XamlIlStringSelector", Node),
        ("XamlIlCombinatorSelector", Node),
        ("XamlIlNotSelector", Node),
        ("XamlIlNthChildSelector", Node),
        ("XamlIlPropertyEqualsSelector", Node),
        ("XamlIlAttachedPropertyEqualsSelector", Node),
        ("XamlIlOrSelectorNode", Node),
        ("XamlIlNestingSelector", Node),
        ("XamlIlQueryInitialNode", Node),
        ("XamlIlTypeQuery", Node),
        ("XamlIlStringQuery", Node),
        ("XamlIlCombinatorQuery", Node),
        ("XamlIlWidthQuery", Node),
        ("XamlIlHeightQuery", Node),
        ("XamlIlOrQueryNode", Node),
        ("XamlIlAndQueryNode", Node),
        ("XamlIlDirectCallPropertySetter", Setter),
        ("FerroNameScopeRegistrationXamlIlNode", Node),
        ("ClassValueSetter", Setter),
        ("ClassBindingSetter", Setter),
        ("FerroAttachedInstancePropertySetterMethod", Setter),
        ("FerroAttachedInstancePropertyGetterMethod", Method),
        ("XamlDirectCallAddHandler", Setter),
        ("InjectServiceProviderNode", Node),
        ("OptionsMarkupExtensionMethod", Method),
        ("ResourceAdderSetter", Setter),
        ("EnsureCapacityNode", Node),
        ("HandleRootObjectScopeNode", Node),
        ("XamlSourceInfoValueManipulation", Node),
        ("NewServiceProviderNode", Node),
        ("XamlIlBindingPathNode", Node),
        ("XamlIlNotPathElementNode", BindingPathElement),
        ("XamlIlStreamObservablePathElementNode", BindingPathElement),
        ("XamlIlStreamTaskPathElementNode", BindingPathElement),
        ("SelfPathElementNode", BindingPathElement),
        ("FindAncestorPathElementNode", BindingPathElement),
        ("FindVisualAncestorPathElementNode", BindingPathElement),
        ("ElementNamePathElementNode", BindingPathElement),
        ("TemplatedParentPathElementNode", BindingPathElement),
        ("XamlIlFerroPropertyPropertyPathElementNode", BindingPathElement),
        ("XamlIlClrPropertyPathElementNode", BindingPathElement),
        ("XamlIlClrMethodPathElementNode", BindingPathElement),
        ("XamlIlClrMethodAsCommandPathElementNode", BindingPathElement),
        ("XamlIlClrIndexerPathElementNode", BindingPathElement),
        ("XamlIlArrayIndexerPathElementNode", BindingPathElement),
        ("TypeCastPathElementNode", BindingPathElement),
        ("XamlIlClrPropertyInfoEmitter", EmitHelper),
        ("XamlIlPropertyInfoAccessorFactoryEmitter", EmitHelper),
        ("XamlIlTrampolineBuilder", EmitHelper),
    ]
};

/// Adapts the type mappings of the language to the run-time back end: the
/// deferred content customisation becomes the generic method definition the
/// back end can call ([`DeferredTransformationFactoryMethod`]).
pub fn adapt_type_mappings(mappings: &mut XamlLanguageTypeMappings) {
    if let Some(customization) = mappings.deferred_content_executor_customization.take() {
        mappings.deferred_content_executor_customization =
            Some(DeferredTransformationFactoryMethod::new(customization));
    }
}

/// Registers what the run-time back end adds to a configuration of the
/// framework language: the compile-time value parser over the run-time
/// type system.
pub fn configure_configuration(configuration: &TransformerConfiguration) {
    configuration.get_or_create_extra::<XamlCompileTimeValueParsers>().add(Rc::new(RuntimeCompileTimeValueParser));
    configuration.get_or_create_extra::<XamlRuntimeIncludeFallback>().set(Some(Rc::new(can_load_at_run_time)));
}

/// The predicate of [`XamlRuntimeIncludeFallback`] of the run-time back end: the document
/// with the absolute URI is an asset the asset loader has, and a loader of documents
/// without compiled markup is registered (`IRuntimeXamlLoader`, what
/// `FerroRuntimeXamlLoader::register` installs), so that the `Loaded` of an include left
/// with its `Source` can load it (`FerroXamlLoader::load`).
fn can_load_at_run_time(absolute_uri: &str) -> bool {
    use ferroui_base::platform::IAssetLoader;
    use ferroui_base::utilities::{Uri, UriKind};
    use ferroui_base::{FerroLocator, LocatorExtensions};
    let locator = FerroLocator::current();
    if locator.get_service::<dyn ferroui_markup_xaml::IRuntimeXamlLoader>().is_none() {
        return false;
    }
    let Some(assets) = locator.get_service::<dyn IAssetLoader>() else {
        return false;
    };
    Uri::new(absolute_uri, UriKind::Absolute).is_ok_and(|uri| assets.exists(&uri, None))
}

/// Creates the interpreter of the framework language over `configuration`:
/// the standard evaluators plus the evaluators of the framework nodes,
/// setters and methods, the service contracts of the runtime library, and
/// the context members of the language (`emit_mappings`).
pub fn create_interpreter(
    configuration: Rc<TransformerConfiguration>,
    type_system: Rc<RuntimeTypeSystem>,
    emit_mappings: &FerroXamlIlLanguageEmitMappings,
) -> XamlResult<Rc<Interpreter>> {
    // The members the language adds to the context must be describable, as for
    // the IL back end that defines them: the name scope field and the eager parent
    // stack provider (implemented by the context itself, see `services`).
    let _definition = emit_mappings.context_type_builder_callback(&configuration.type_mappings)?;
    let mut interpreter = Interpreter::new(configuration, type_system);
    interpreter.services = Rc::new(FerroRuntimeContextServices);
    interpreter.node_evaluators.insert(0, Rc::new(FrameworkNodeEvaluator));
    interpreter.setter_evaluators.push(Rc::new(FrameworkSetterEvaluator));
    interpreter.method_evaluators.push(Rc::new(FrameworkMethodEvaluator));
    interpreter.provide_value_target_property_evaluator = Some(Rc::new(provide_value_target_property));
    interpreter.context_initializers.push(Rc::new(services::initialize_name_scope_field));
    Ok(Rc::new(interpreter))
}

#[cfg(test)]
mod evaluator_tests;
#[cfg(test)]
mod tests;
