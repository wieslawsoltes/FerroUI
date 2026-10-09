//! What the language configured the context of a document to be, for a back end that
//! does not link the XAML runtime library: the emitter of Rust source compiles a document
//! only when its context is the one generated code creates (`rt::create_context`), the
//! context of the framework language.

use xamlx::transform::TransformerConfiguration;

/// Which services the context of a document implements: the type mappings of the
/// language that are set. The fields are the ones of the definition the contexts of the
/// runtime library are created with (`XamlIlContextDefinition`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContextDefinition {
    /// `RootObjectProvider` is mapped.
    pub root_object_provider: bool,
    /// `ParentStackProvider` is mapped: the context keeps the parent stack.
    pub parent_stack_provider: bool,
    /// `TypeDescriptorContext` is mapped.
    pub type_descriptor_context: bool,
    /// `ProvideValueTarget` is mapped: the context records the target object and target
    /// property.
    pub provide_value_target: bool,
    /// `UriContextProvider` is mapped.
    pub uri_context_provider: bool,
    /// `XmlNamespaceInfoProvider` is mapped: a document has a namespace information
    /// provider.
    pub xml_namespace_info_provider: bool,
}

/// The context of the framework language: the definition the contexts of generated code
/// are created with (`FRAMEWORK_CONTEXT` of the runtime library, `xaml_il::runtime::compiled`;
/// a test compares the two).
pub const FRAMEWORK_CONTEXT: ContextDefinition = ContextDefinition {
    root_object_provider: true,
    parent_stack_provider: true,
    type_descriptor_context: true,
    provide_value_target: true,
    uri_context_provider: true,
    xml_namespace_info_provider: true,
};

/// What the language configured the context to be: the type mappings of the transformer
/// configuration that are set.
pub fn context_definition(configuration: &TransformerConfiguration) -> ContextDefinition {
    let mappings = &configuration.type_mappings;
    ContextDefinition {
        root_object_provider: mappings.root_object_provider.is_some(),
        parent_stack_provider: mappings.parent_stack_provider.is_some(),
        type_descriptor_context: mappings.type_descriptor_context.is_some(),
        provide_value_target: mappings.provide_value_target.is_some(),
        uri_context_provider: mappings.uri_context_provider.is_some(),
        xml_namespace_info_provider: mappings.xml_namespace_info_provider.is_some(),
    }
}

#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;

    /// Not from upstream: the context the emitter compiles for is the one the runtime
    /// library creates for generated code, field by field.
    #[test]
    fn framework_context_is_the_one_of_the_runtime_library() {
        let runtime = ferroui_markup_xaml::xaml_il::runtime::compiled::FRAMEWORK_CONTEXT;
        let ContextDefinition {
            root_object_provider,
            parent_stack_provider,
            type_descriptor_context,
            provide_value_target,
            uri_context_provider,
            xml_namespace_info_provider,
        } = FRAMEWORK_CONTEXT;
        assert_eq!(
            [root_object_provider, parent_stack_provider, type_descriptor_context, provide_value_target, uri_context_provider, xml_namespace_info_provider],
            [
                runtime.root_object_provider,
                runtime.parent_stack_provider,
                runtime.type_descriptor_context,
                runtime.provide_value_target,
                runtime.uri_context_provider,
                runtime.xml_namespace_info_provider,
            ]
        );
    }
}
