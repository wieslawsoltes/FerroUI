//! Port of `Emit/XamlRuntimeContext.cs`.

use std::rc::Rc;

use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::{IXamlConstructor, IXamlField, IXamlMethod, IXamlType};

use super::{IXamlEmitResult, XamlLanguageEmitMappings};

/// `Action<XamlRuntimeContext<..>, TBackendEmitter>`.
pub type XamlRuntimeContextFactory<TBackendEmitter, TEmitResult> = Rc<
    dyn Fn(&XamlRuntimeContext<TBackendEmitter, TEmitResult>, &TBackendEmitter) -> XamlResult<()>,
>;

pub struct XamlRuntimeContext<TBackendEmitter, TEmitResult: IXamlEmitResult> {
    pub root_object_field: Option<Rc<dyn IXamlField>>,
    pub intermediate_root_object_field: Option<Rc<dyn IXamlField>>,
    pub parent_list_field: Option<Rc<dyn IXamlField>>,
    pub context_type: Rc<dyn IXamlType>,
    pub property_target_object: Option<Rc<dyn IXamlField>>,
    pub property_target_property: Option<Rc<dyn IXamlField>>,
    pub constructor: Rc<dyn IXamlConstructor>,
    /// The factories run by [`XamlRuntimeContext::factory`], in order (upstream composes them
    /// into the single `Factory` delegate).
    pub factories: Vec<XamlRuntimeContextFactory<TBackendEmitter, TEmitResult>>,
    pub push_parent_method: Option<Rc<dyn IXamlMethod>>,
    pub pop_parent_method: Option<Rc<dyn IXamlMethod>>,
    pub base_url: Option<String>,
}

impl<TBackendEmitter, TEmitResult: IXamlEmitResult>
    XamlRuntimeContext<TBackendEmitter, TEmitResult>
{
    pub fn new(
        definition: &Rc<dyn IXamlType>,
        constructed_type: &Rc<dyn IXamlType>,
        base_uri: Option<String>,
        mappings: &XamlLanguageEmitMappings<TBackendEmitter, TEmitResult>,
        factory: XamlRuntimeContextFactory<TBackendEmitter, TEmitResult>,
    ) -> XamlResult<Self> {
        let context_type = definition.make_generic_type(std::slice::from_ref(constructed_type))?;

        let fields = context_type.fields();
        let methods = context_type.methods();
        let get = |s: &str| fields.iter().find(|f| f.name() == s).cloned();
        let get_method = |s: &str| methods.iter().find(|f| f.name() == s).cloned();

        let constructor = context_type
            .constructors()
            .into_iter()
            .next()
            .ok_or_else(|| {
                XamlError::internal(
                    "ArgumentOutOfRangeException",
                    format!(
                        "Context type {} doesn't have a constructor",
                        context_type.get_fqn()
                    ),
                )
            })?;

        let mut factories = vec![factory];
        if let Some(callback) = &mappings.context_factory_callback {
            factories.push(callback.clone());
        }

        Ok(Self {
            root_object_field: get(XamlRuntimeContextDefintion::ROOT_OBJECT_FIELD_NAME),
            intermediate_root_object_field: get(
                XamlRuntimeContextDefintion::INTERMEDIATE_ROOT_OBJECT_FIELD_NAME,
            ),
            parent_list_field: get(XamlRuntimeContextDefintion::PARENT_LIST_FIELD_NAME),
            property_target_object: get(XamlRuntimeContextDefintion::PROVIDE_TARGET_OBJECT_NAME),
            property_target_property: get(
                XamlRuntimeContextDefintion::PROVIDE_TARGET_PROPERTY_NAME,
            ),
            push_parent_method: get_method(XamlRuntimeContextDefintion::PUSH_PARENT_METHOD_NAME),
            pop_parent_method: get_method(XamlRuntimeContextDefintion::POP_PARENT_METHOD_NAME),
            constructor,
            factories,
            context_type,
            base_url: base_uri,
        })
    }

    /// `Factory(il)`: emits the creation of the runtime context.
    pub fn factory(&self, il: &TBackendEmitter) -> XamlResult<()> {
        for factory in &self.factories {
            factory(self, il)?;
        }
        Ok(())
    }
}

/// Names of the runtime context members (the type name keeps the upstream spelling).
pub struct XamlRuntimeContextDefintion;

impl XamlRuntimeContextDefintion {
    pub const ROOT_OBJECT_FIELD_NAME: &'static str = "RootObject";
    pub const INTERMEDIATE_ROOT_OBJECT_FIELD_NAME: &'static str = "IntermediateRoot";
    pub const PARENT_LIST_FIELD_NAME: &'static str = "ParentsStack";
    pub const PROVIDE_TARGET_OBJECT_NAME: &'static str = "ProvideTargetObject";
    pub const PROVIDE_TARGET_PROPERTY_NAME: &'static str = "ProvideTargetProperty";
    pub const PUSH_PARENT_METHOD_NAME: &'static str = "PushParent";
    pub const POP_PARENT_METHOD_NAME: &'static str = "PopParent";
}
