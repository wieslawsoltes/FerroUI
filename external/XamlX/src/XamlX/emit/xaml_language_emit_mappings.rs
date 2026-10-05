//! Port of `Emit/XamlLanguageEmitMappings.cs`.

use std::rc::Rc;

use crate::ast::XamlAstClrProperty;
use crate::exceptions::XamlResult;
use crate::type_system::{IXamlConstructorBuilder, IXamlField, IXamlTypeBuilder};

use super::{IXamlEmitResult, XamlEmitContext, XamlRuntimeContext};

/// `Func<XamlEmitContext<..>, TBackendEmitter, XamlAstClrProperty, bool>`.
pub type XamlProvideValueTargetPropertyEmitter<TBackendEmitter, TEmitResult> = Rc<
    dyn Fn(
        &dyn XamlEmitContext<TBackendEmitter, TEmitResult>,
        &TBackendEmitter,
        &Rc<XamlAstClrProperty>,
    ) -> XamlResult<bool>,
>;

pub type XamlContextTypeBuilderCallback<TBackendEmitter> =
    Rc<dyn Fn(&dyn IXamlILContextDefinition<TBackendEmitter>) -> XamlResult<()>>;

pub type XamlContextFactoryCallback<TBackendEmitter, TEmitResult> = Rc<
    dyn Fn(&XamlRuntimeContext<TBackendEmitter, TEmitResult>, &TBackendEmitter) -> XamlResult<()>,
>;

pub struct XamlLanguageEmitMappings<TBackendEmitter, TEmitResult: IXamlEmitResult> {
    pub provide_value_target_property_emitter:
        Option<XamlProvideValueTargetPropertyEmitter<TBackendEmitter, TEmitResult>>,
    pub context_type_builder_callback: Option<XamlContextTypeBuilderCallback<TBackendEmitter>>,
    pub context_factory_callback: Option<XamlContextFactoryCallback<TBackendEmitter, TEmitResult>>,
}

impl<TBackendEmitter, TEmitResult: IXamlEmitResult> Default
    for XamlLanguageEmitMappings<TBackendEmitter, TEmitResult>
{
    fn default() -> Self {
        Self {
            provide_value_target_property_emitter: None,
            context_type_builder_callback: None,
            context_factory_callback: None,
        }
    }
}

impl<TBackendEmitter, TEmitResult: IXamlEmitResult>
    XamlLanguageEmitMappings<TBackendEmitter, TEmitResult>
{
    pub fn new() -> Self {
        Self::default()
    }
}

/// The definition of the runtime context type a backend generates. Upstream declares this
/// generic interface next to its IL implementation (`IL/RuntimeContext.cs`); only the interface
/// is ported because the emit mappings refer to it.
pub trait IXamlILContextDefinition<TBackendEmitter> {
    fn type_builder(&self) -> Rc<dyn IXamlTypeBuilder<TBackendEmitter>>;
    fn constructor_builder(&self) -> Rc<dyn IXamlConstructorBuilder<TBackendEmitter>>;
    fn parent_list_field(&self) -> Option<Rc<dyn IXamlField>>;
    fn parent_service_provider_field(&self) -> Rc<dyn IXamlField>;
    fn inner_service_provider_field(&self) -> Option<Rc<dyn IXamlField>>;
    fn target_object_field(&self) -> Option<Rc<dyn IXamlField>>;
    fn target_property_field(&self) -> Option<Rc<dyn IXamlField>>;
}
