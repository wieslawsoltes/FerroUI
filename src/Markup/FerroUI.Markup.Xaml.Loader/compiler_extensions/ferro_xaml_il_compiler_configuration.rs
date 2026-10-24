//! Port of `CompilerExtensions/FerroXamlIlCompilerConfiguration.cs`.

use std::ops::Deref;
use std::rc::Rc;

use xamlx::exceptions::XamlResult;
use xamlx::transform::{
    IXamlIdentifierGenerator, TransformerConfiguration, XamlDiagnosticsHandler,
    XamlLanguageTypeMappings, XamlValueConverter, XamlXmlnsMappings,
};
use xamlx::type_system::{IXamlAssembly, IXamlTypeSystem};

use super::transformers::FerroXamlIlWellKnownTypes;

/// `FerroXamlIlCompilerConfiguration : TransformerConfiguration`: a transformer configuration
/// that carries the framework's well-known types as a configuration extra.
///
/// The base class is reachable through `Deref` and, as the shared handle the transformation
/// contexts take, through [`FerroXamlIlCompilerConfiguration::as_transformer_configuration`].
///
/// Upstream the class also owns (and registers as extras) the three IL helper emitters
/// `ClrPropertyEmitter`, `AccessorFactoryEmitter` and `TrampolineBuilder`. They generate IL
/// helper types and have no backend-neutral state; a back end that needs equivalents registers
/// its own with `add_extra`.
pub struct FerroXamlIlCompilerConfiguration {
    base: Rc<TransformerConfiguration>,
}

impl Deref for FerroXamlIlCompilerConfiguration {
    type Target = TransformerConfiguration;

    fn deref(&self) -> &TransformerConfiguration {
        &self.base
    }
}

impl FerroXamlIlCompilerConfiguration {
    pub fn new(
        type_system: Rc<dyn IXamlTypeSystem>,
        default_assembly: Option<Rc<dyn IXamlAssembly>>,
        type_mappings: XamlLanguageTypeMappings,
        xmlns_mappings: Option<XamlXmlnsMappings>,
        custom_value_converter: Option<XamlValueConverter>,
        identifier_generator: Option<Rc<dyn IXamlIdentifierGenerator>>,
        diagnostics_handler: Option<XamlDiagnosticsHandler>,
    ) -> XamlResult<Self> {
        let base = TransformerConfiguration::new(
            type_system,
            default_assembly,
            type_mappings,
            xmlns_mappings,
            custom_value_converter,
            identifier_generator,
            diagnostics_handler,
        )?;
        base.add_extra(Rc::new(FerroXamlIlWellKnownTypes::new(&base.type_system)?));
        Ok(Self {
            base: Rc::new(base),
        })
    }

    /// The base class instance, as the handle `AstTransformationContext::new` takes.
    pub fn as_transformer_configuration(&self) -> &Rc<TransformerConfiguration> {
        &self.base
    }
}
