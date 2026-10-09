//! The methods the back ends add to the type system of a transform: the deferred content
//! customisation of the language as a generic method definition, and the build and
//! populate methods of the documents of a group, which the include group transformer puts
//! into method call nodes. The interpreter runs them
//! ([`crate::runtime::framework`]); the emitter of Rust source reads which document a call
//! names and writes the call of its generated function.

use std::any::Any;
use std::rc::Rc;

use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::XamlLanguageTypeMappings;
use xamlx::type_system::{IXamlCustomAttribute, IXamlMember, IXamlMethod, IXamlParameterInfo, IXamlType};

use crate::compiler_extensions::{FerroXamlIlCompiler, IXamlDocumentTypeBuilderProvider};

/// `XamlIlRuntimeHelpers.DeferredTransformationFactoryV3<T>`: the deferred
/// content customisation of the language as a generic method definition.
///
/// Metadata declares no generic methods, so the method the language found
/// by name is wrapped: [`make_generic_method`](IXamlMethod::make_generic_method)
/// records the type argument, and a back end creates the deferred content
/// with it as the result type.
pub struct DeferredTransformationFactoryMethod {
    inner: Rc<dyn IXamlMethod>,
    type_argument: Option<Rc<dyn IXamlType>>,
}

impl DeferredTransformationFactoryMethod {
    pub fn new(inner: Rc<dyn IXamlMethod>) -> Rc<dyn IXamlMethod> {
        Rc::new(Self { inner, type_argument: None })
    }

    /// The type argument the method was constructed with; none for the definition.
    pub fn type_argument(&self) -> Option<&Rc<dyn IXamlType>> {
        self.type_argument.as_ref()
    }
}

/// Adapts the type mappings of the language to a back end: the deferred content
/// customisation becomes the generic method definition the back end can call
/// ([`DeferredTransformationFactoryMethod`]).
pub fn adapt_type_mappings(mappings: &mut XamlLanguageTypeMappings) {
    if let Some(customization) = mappings.deferred_content_executor_customization.take() {
        mappings.deferred_content_executor_customization = Some(DeferredTransformationFactoryMethod::new(customization));
    }
}

impl IXamlMember for DeferredTransformationFactoryMethod {
    fn name(&self) -> String {
        self.inner.name()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        self.inner.declaring_type()
    }
}

impl IXamlMethod for DeferredTransformationFactoryMethod {
    fn is_public(&self) -> bool {
        self.inner.is_public()
    }
    fn is_private(&self) -> bool {
        self.inner.is_private()
    }
    fn is_family(&self) -> bool {
        self.inner.is_family()
    }
    fn is_static(&self) -> bool {
        true
    }
    fn contains_generic_parameters(&self) -> bool {
        self.type_argument.is_none()
    }
    fn is_generic_method(&self) -> bool {
        true
    }
    fn is_generic_method_definition(&self) -> bool {
        self.type_argument.is_none()
    }
    fn return_type(&self) -> Rc<dyn IXamlType> {
        self.inner.return_type()
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.inner.parameters()
    }
    fn make_generic_method(&self, type_arguments: &[Rc<dyn IXamlType>]) -> XamlResult<Rc<dyn IXamlMethod>> {
        match type_arguments {
            [argument] => {
                Ok(Rc::new(Self { inner: self.inner.clone(), type_argument: Some(argument.clone()) }))
            }
            _ => Err(XamlError::argument(format!(
                "{} takes one type argument, {} were given",
                self.inner.name(),
                type_arguments.len()
            ))),
        }
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.inner.custom_attributes()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        self.inner.get_parameter_info(index)
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        self.type_argument.iter().cloned().collect()
    }
    fn equals(&self, other: &dyn IXamlMethod) -> bool {
        match other.as_any().downcast_ref::<Self>() {
            Some(other) => {
                self.inner.equals(&*other.inner)
                    && match (&self.type_argument, &other.type_argument) {
                        (None, None) => true,
                        (Some(a), Some(b)) => a.equals(&**b),
                        _ => false,
                    }
            }
            None => false,
        }
    }
    fn get_hash_code(&self) -> u64 {
        self.inner.get_hash_code()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The signature of the build or the populate method of a document.
pub(crate) struct DocumentMethodSignature {
    pub(crate) name: &'static str,
    pub(crate) document_name: String,
    pub(crate) declaring_type: Rc<dyn IXamlType>,
    pub(crate) return_type: Rc<dyn IXamlType>,
    pub(crate) parameters: Vec<Rc<dyn IXamlType>>,
}

/// The method contracts of a type with the field `signature`
/// ([`DocumentMethodSignature`]): a static method that is the method of its document alone.
macro_rules! document_method {
    ($type_:ident) => {
        impl xamlx::type_system::IXamlMember for $type_ {
            fn name(&self) -> String {
                self.signature.name.to_string()
            }
            fn declaring_type(&self) -> std::rc::Rc<dyn xamlx::type_system::IXamlType> {
                self.signature.declaring_type.clone()
            }
        }

        impl xamlx::type_system::IXamlMethod for $type_ {
            fn is_public(&self) -> bool {
                true
            }
            fn is_private(&self) -> bool {
                false
            }
            fn is_family(&self) -> bool {
                false
            }
            fn is_static(&self) -> bool {
                true
            }
            fn contains_generic_parameters(&self) -> bool {
                false
            }
            fn is_generic_method(&self) -> bool {
                false
            }
            fn is_generic_method_definition(&self) -> bool {
                false
            }
            fn return_type(&self) -> std::rc::Rc<dyn xamlx::type_system::IXamlType> {
                self.signature.return_type.clone()
            }
            fn parameters(&self) -> Vec<std::rc::Rc<dyn xamlx::type_system::IXamlType>> {
                self.signature.parameters.clone()
            }
            fn make_generic_method(&self, _type_arguments: &[std::rc::Rc<dyn xamlx::type_system::IXamlType>]) -> xamlx::exceptions::XamlResult<std::rc::Rc<dyn xamlx::type_system::IXamlMethod>> {
                Err(xamlx::exceptions::XamlError::invalid_operation(format!(
                    "{} of document {} is not a generic method definition",
                    self.signature.name, self.signature.document_name
                )))
            }
            fn custom_attributes(&self) -> Vec<std::rc::Rc<dyn xamlx::type_system::IXamlCustomAttribute>> {
                Vec::new()
            }
            fn get_parameter_info(&self, index: usize) -> xamlx::exceptions::XamlResult<std::rc::Rc<dyn xamlx::type_system::IXamlParameterInfo>> {
                match self.signature.parameters.get(index) {
                    Some(parameter) => Ok(std::rc::Rc::new(xamlx::type_system::AnonymousParameterInfo::with_index(parameter.clone(), index))),
                    None => Err(xamlx::exceptions::XamlError::internal(
                        "ArgumentOutOfRangeException",
                        format!("Method {} doesn't have a parameter {index}", self.signature.name),
                    )),
                }
            }
            fn generic_parameters(&self) -> Vec<std::rc::Rc<dyn xamlx::type_system::IXamlType>> {
                Vec::new()
            }
            fn generic_arguments(&self) -> Vec<std::rc::Rc<dyn xamlx::type_system::IXamlType>> {
                Vec::new()
            }
            fn equals(&self, other: &dyn xamlx::type_system::IXamlMethod) -> bool {
                other.as_any().downcast_ref::<$type_>().is_some_and(|other| std::ptr::eq(self, other))
            }
            fn get_hash_code(&self) -> u64 {
                self as *const Self as usize as u64
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    };
}

#[cfg(feature = "runtime")]
pub(crate) use document_method;

/// The build method of a document a back end does not run: `T Build(IServiceProvider)`.
pub struct DocumentBuildSignature {
    signature: DocumentMethodSignature,
}

/// The populate method of a document a back end does not run:
/// `void Populate(IServiceProvider, T)`.
pub struct DocumentPopulateSignature {
    signature: DocumentMethodSignature,
}

document_method!(DocumentBuildSignature);
document_method!(DocumentPopulateSignature);

/// The handles of the populate method and (for a document that can be instantiated on
/// its own) of the build method of a document, for a back end that generates the methods
/// instead of running them: the methods have the signatures of the ones the run-time back
/// end defines (`RuntimeDocumentTypeBuilderProvider`) and no body.
pub struct DocumentTypeBuilderProvider {
    populate: Rc<dyn IXamlMethod>,
    build: Option<Rc<dyn IXamlMethod>>,
}

impl DocumentTypeBuilderProvider {
    /// `root_type` is the type of the root object of the document, which stands for the
    /// generated type the methods belong to.
    pub fn new(
        name: &str,
        root_type: Rc<dyn IXamlType>,
        service_provider: Rc<dyn IXamlType>,
        void: Rc<dyn IXamlType>,
        has_build_method: bool,
    ) -> Rc<Self> {
        let populate: Rc<dyn IXamlMethod> = Rc::new(DocumentPopulateSignature {
            signature: DocumentMethodSignature {
                name: FerroXamlIlCompiler::POPULATE_NAME,
                document_name: name.to_string(),
                declaring_type: root_type.clone(),
                return_type: void,
                parameters: vec![service_provider.clone(), root_type.clone()],
            },
        });
        let build: Option<Rc<dyn IXamlMethod>> = has_build_method.then(|| {
            Rc::new(DocumentBuildSignature {
                signature: DocumentMethodSignature {
                    name: FerroXamlIlCompiler::BUILD_NAME,
                    document_name: name.to_string(),
                    declaring_type: root_type.clone(),
                    return_type: root_type,
                    parameters: vec![service_provider],
                },
            }) as Rc<dyn IXamlMethod>
        });
        Rc::new(Self { populate, build })
    }
}

impl IXamlDocumentTypeBuilderProvider for DocumentTypeBuilderProvider {
    fn populate_method(&self) -> Rc<dyn IXamlMethod> {
        self.populate.clone()
    }
    fn build_method(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.build.clone()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
