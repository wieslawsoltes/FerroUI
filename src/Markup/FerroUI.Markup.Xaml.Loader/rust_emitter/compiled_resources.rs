//! The compiled documents of other crates, as the compiler of a crate sees
//! them (docs/porting/xaml.md, 9.5.5, 9.7.3).
//!
//! Upstream's compiler finds a document of a referenced assembly through that
//! assembly's metadata (`XamlIncludeGroupTransformer`): the public static
//! method `Build:<rooted path>` of its type `CompiledFerroXaml.!FerroResources` (upstream's name, with the name of the port)
//! for a document without a class, the type named after the path of the
//! document for a document with `x:Class`. Here the referenced crate
//! describes its documents in its `.xamlmeta` file ([`XamlMetadata`]), and
//! [`CompiledMarkupTypeSystem`] puts the same type on the assembly of each such
//! crate: [`COMPILED_RESOURCES_TYPE_NAME`] with one static method per compiled
//! document without a class, named `Build:<rooted path>`, public when the document
//! is public, that returns the type of the root of the document and takes the
//! service provider. The include transformer finds it there exactly as upstream
//! finds the method, and the emitter calls the function the method stands for
//! ([`CompiledDocumentBuildMethod::rust_path`]).
//!
//! Everything else of the type system is the type system it wraps.

use std::any::Any;
use std::rc::{Rc, Weak};

use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{
    AnonymousParameterInfo, IXamlAssembly, IXamlConstructor, IXamlCustomAttribute, IXamlEventInfo, IXamlField,
    IXamlMember, IXamlMethod, IXamlParameterInfo, IXamlProperty, IXamlType, IXamlTypeSystem, XamlTypeId,
    XamlTypeWellKnownTypes,
};

use super::xaml_metadata::XamlMetadata;
use crate::compiler_extensions::group_transformers::COMPILED_RESOURCES_TYPE_NAME;

/// A type system that knows the compiled documents of other crates: the type system it
/// wraps, with the type [`COMPILED_RESOURCES_TYPE_NAME`] on the assembly of every crate
/// of `dependencies`.
pub struct CompiledMarkupTypeSystem {
    inner: Rc<dyn IXamlTypeSystem>,
    assemblies: Vec<Rc<CompiledAssembly>>,
}

impl CompiledMarkupTypeSystem {
    /// `inner` with the compiled documents of `dependencies`. A document whose root type
    /// `inner` does not know is an error: the dependency is not linked into the compiler.
    pub fn new(inner: Rc<dyn IXamlTypeSystem>, dependencies: &[XamlMetadata]) -> XamlResult<Rc<Self>> {
        let service_provider = inner
            .find_type("System.IServiceProvider")
            .ok_or_else(|| XamlError::type_system_exception("Unable to resolve type System.IServiceProvider"))?;
        let mut assemblies = Vec::with_capacity(dependencies.len());
        for metadata in dependencies {
            // (name, return type, public, Rust path) of each method.
            let mut methods: Vec<(String, Rc<dyn IXamlType>, bool, String)> = Vec::new();
            // The classes of the documents with `x:Class`, by the name upstream finds them by.
            let mut classes: Vec<(String, Rc<dyn IXamlType>)> = Vec::new();
            for document in &metadata.documents {
                if document.class_rust_path.is_some() {
                    if let (Some(class), Some(name)) =
                        (inner.find_type(&document.root_type), document_type_name(&document.uri))
                    {
                        classes.push((name, class));
                    }
                }
                let Some(build_path) = &document.build_path else { continue };
                let root_type = inner.find_type(&document.root_type).ok_or_else(|| {
                    XamlError::type_system_exception(format!(
                        "Unable to resolve type {} of the compiled document {} of the assembly {}",
                        document.root_type, document.uri, metadata.name
                    ))
                })?;
                let rooted_path = rooted_path(&document.uri, &metadata.name).ok_or_else(|| {
                    XamlError::type_system_exception(format!(
                        "The compiled document {} is not a document of the assembly {}",
                        document.uri, metadata.name
                    ))
                })?;
                methods.push((format!("Build:{rooted_path}"), root_type, document.public, build_path.clone()));
            }
            let assembly = Rc::new_cyclic(|assembly: &Weak<CompiledAssembly>| CompiledAssembly {
                name: metadata.name.clone(),
                inner: inner.find_assembly(&metadata.name),
                classes,
                resources: Rc::new_cyclic(|resources: &Weak<CompiledResourcesType>| CompiledResourcesType {
                    id: XamlTypeId::new_unique(),
                    assembly: assembly.clone(),
                    methods: methods
                        .into_iter()
                        .map(|(name, return_type, public, rust_path)| {
                            Rc::new(CompiledDocumentBuildMethod {
                                name,
                                declaring_type: resources.clone(),
                                return_type,
                                parameters: vec![service_provider.clone()],
                                public,
                                rust_path,
                            }) as Rc<dyn IXamlMethod>
                        })
                        .collect(),
                }),
            });
            assemblies.push(assembly);
        }
        Ok(Rc::new(Self { inner, assemblies }))
    }
}

/// The full name of the type upstream's include transformer looks for a document by
/// (`Path.GetFileNameWithoutExtension(assetPath.Replace('/', '.'))`, where the asset path
/// is the URI without its scheme): `ferres://Tests/Folder/Theme.xaml` → `Tests.Folder.Theme`.
fn document_type_name(uri: &str) -> Option<String> {
    let dotted = uri.strip_prefix("ferres://")?.replace('/', ".");
    Some(match dotted.rfind('.') {
        Some(extension) => dotted[..extension].to_string(),
        None => dotted,
    })
}

/// The path of `uri` below the root of the assembly `assembly`, with its leading `/`
/// (`ferres://Tests/Folder/Style.xaml` → `/Folder/Style.xaml`).
fn rooted_path<'a>(uri: &'a str, assembly: &str) -> Option<&'a str> {
    let rest = uri.strip_prefix("ferres://")?;
    let separator = rest.find('/')?;
    rest[..separator].eq_ignore_ascii_case(assembly).then_some(&rest[separator..])
}

impl IXamlTypeSystem for CompiledMarkupTypeSystem {
    fn assemblies(&self) -> Vec<Rc<dyn IXamlAssembly>> {
        self.inner.assemblies()
    }
    fn well_known_types(&self) -> Rc<XamlTypeWellKnownTypes> {
        self.inner.well_known_types()
    }
    fn find_assembly(&self, substring: &str) -> Option<Rc<dyn IXamlAssembly>> {
        // As the type system it wraps compares assembly names (`SreTypeSystem.FindAssembly`).
        match self.assemblies.iter().find(|a| a.name.to_lowercase() == substring.to_lowercase()) {
            Some(compiled) => Some(compiled.clone()),
            None => self.inner.find_assembly(substring),
        }
    }
    fn find_type(&self, name: &str) -> Option<Rc<dyn IXamlType>> {
        self.inner.find_type(name)
    }
    fn find_type_in_assembly(&self, name: &str, assembly: &str) -> Option<Rc<dyn IXamlType>> {
        self.inner.find_type_in_assembly(name, assembly)
    }
}

/// The assembly of a crate with compiled markup: the assembly of the wrapped type system
/// (if it has one by that name) and the compiled documents.
struct CompiledAssembly {
    name: String,
    inner: Option<Rc<dyn IXamlAssembly>>,
    /// The classes of the documents with `x:Class`, by the full name named after the
    /// path of the document.
    classes: Vec<(String, Rc<dyn IXamlType>)>,
    resources: Rc<CompiledResourcesType>,
}

impl IXamlAssembly for CompiledAssembly {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.inner.as_ref().map(|inner| inner.custom_attributes()).unwrap_or_default()
    }
    fn find_type(&self, full_name: &str) -> Option<Rc<dyn IXamlType>> {
        if full_name == COMPILED_RESOURCES_TYPE_NAME {
            return Some(self.resources.clone());
        }
        if let Some(found) = self.inner.as_ref().and_then(|inner| inner.find_type(full_name)) {
            return Some(found);
        }
        // Deviation (xaml.md, 9.7.3): the host of a URI is lower case in the port (`Uri`),
        // where upstream's registered resource URI parser keeps the case of the assembly
        // name, so the name upstream looks the class of a document up by can differ from
        // the class in case. The class of a compiled document is found by the URI of the
        // document, compared without regard to case as the port compares document URIs.
        self.classes
            .iter()
            .find(|(name, _)| name.to_lowercase() == full_name.to_lowercase())
            .map(|(_, class)| class.clone())
    }
    fn equals(&self, other: &dyn IXamlAssembly) -> bool {
        match other.as_any().downcast_ref::<CompiledAssembly>() {
            Some(other) => std::ptr::eq(self, other),
            None => self.inner.as_ref().is_some_and(|inner| inner.equals(other)),
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `CompiledFerroXaml.!FerroResources` of a crate: one `Build:<rooted path>` method per
/// compiled document without a class.
struct CompiledResourcesType {
    id: XamlTypeId,
    assembly: Weak<CompiledAssembly>,
    methods: Vec<Rc<dyn IXamlMethod>>,
}

impl IXamlType for CompiledResourcesType {
    fn id(&self) -> XamlTypeId {
        self.id.clone()
    }
    fn name(&self) -> String {
        COMPILED_RESOURCES_TYPE_NAME.rsplit_once('.').map_or(COMPILED_RESOURCES_TYPE_NAME, |(_, name)| name).to_string()
    }
    fn namespace(&self) -> Option<String> {
        COMPILED_RESOURCES_TYPE_NAME.rsplit_once('.').map(|(namespace, _)| namespace.to_string())
    }
    fn full_name(&self) -> String {
        COMPILED_RESOURCES_TYPE_NAME.to_string()
    }
    fn is_public(&self) -> bool {
        true
    }
    fn is_nested_private(&self) -> bool {
        false
    }
    fn assembly(&self) -> Option<Rc<dyn IXamlAssembly>> {
        self.assembly.upgrade().map(|assembly| assembly as Rc<dyn IXamlAssembly>)
    }
    fn properties(&self) -> Vec<Rc<dyn IXamlProperty>> {
        Vec::new()
    }
    fn events(&self) -> Vec<Rc<dyn IXamlEventInfo>> {
        Vec::new()
    }
    fn fields(&self) -> Vec<Rc<dyn IXamlField>> {
        Vec::new()
    }
    fn methods(&self) -> Vec<Rc<dyn IXamlMethod>> {
        self.methods.clone()
    }
    fn constructors(&self) -> Vec<Rc<dyn IXamlConstructor>> {
        Vec::new()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        Vec::new()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn is_assignable_from(&self, type_: &dyn IXamlType) -> bool {
        type_.id() == self.id
    }
    fn make_generic_type(&self, _type_arguments: &[Rc<dyn IXamlType>]) -> XamlResult<Rc<dyn IXamlType>> {
        Err(XamlError::invalid_operation(format!("{COMPILED_RESOURCES_TYPE_NAME} is not a generic type definition")))
    }
    fn generic_type_definition(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn is_array(&self) -> bool {
        false
    }
    fn array_element_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn make_array_type(&self, _dimensions: i32) -> XamlResult<Rc<dyn IXamlType>> {
        Err(XamlError::not_supported("Specified method is not supported."))
    }
    fn base_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn declaring_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn is_value_type(&self) -> bool {
        false
    }
    fn is_enum(&self) -> bool {
        false
    }
    fn interfaces(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn is_interface(&self) -> bool {
        false
    }
    fn get_enum_underlying_type(&self) -> XamlResult<Rc<dyn IXamlType>> {
        Err(XamlError::invalid_operation("Operation is not valid due to the current state of the object."))
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn is_function_pointer(&self) -> bool {
        false
    }
    fn equals(&self, other: &dyn IXamlType) -> bool {
        other.id() == self.id
    }
    fn get_hash_code(&self) -> u64 {
        match self.id {
            XamlTypeId::Unique(value) => value,
            XamlTypeId::Named(_) => 0,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `static T Build:<rooted path>(IServiceProvider)` of a compiled document of another
/// crate: the function [`Self::rust_path`] of that crate.
pub struct CompiledDocumentBuildMethod {
    name: String,
    declaring_type: Weak<CompiledResourcesType>,
    return_type: Rc<dyn IXamlType>,
    parameters: Vec<Rc<dyn IXamlType>>,
    public: bool,
    rust_path: String,
}

impl CompiledDocumentBuildMethod {
    /// The absolute Rust path of the build function of the document
    /// (`fn(Option<Rc<dyn IServiceProvider>>) -> Result<Ref<T>, XamlLoadException>`).
    pub fn rust_path(&self) -> &str {
        &self.rust_path
    }
}

impl IXamlMember for CompiledDocumentBuildMethod {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        match self.declaring_type.upgrade() {
            Some(declaring_type) => declaring_type,
            None => xamlx::type_system::XamlPseudoType::unknown(),
        }
    }
}

impl IXamlMethod for CompiledDocumentBuildMethod {
    fn is_public(&self) -> bool {
        self.public
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
    fn return_type(&self) -> Rc<dyn IXamlType> {
        self.return_type.clone()
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn make_generic_method(&self, _type_arguments: &[Rc<dyn IXamlType>]) -> XamlResult<Rc<dyn IXamlMethod>> {
        Err(XamlError::invalid_operation(format!("{} is not a generic method definition", self.name)))
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        Vec::new()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        match self.parameters.get(index) {
            Some(parameter) => Ok(Rc::new(AnonymousParameterInfo::with_index(parameter.clone(), index))),
            None => Err(XamlError::internal(
                "ArgumentOutOfRangeException",
                format!("Method {} doesn't have a parameter {index}", self.name),
            )),
        }
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn equals(&self, other: &dyn IXamlMethod) -> bool {
        other.as_any().downcast_ref::<CompiledDocumentBuildMethod>().is_some_and(|other| std::ptr::eq(self, other))
    }
    fn get_hash_code(&self) -> u64 {
        self as *const Self as usize as u64
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::rooted_path;

    #[test]
    fn rooted_path_of_a_document() {
        assert_eq!(rooted_path("ferres://Tests/Folder/Style.xaml", "Tests"), Some("/Folder/Style.xaml"));
        assert_eq!(rooted_path("ferres://tests/Style.xaml", "Tests"), Some("/Style.xaml"));
        assert_eq!(rooted_path("ferres://Other/Style.xaml", "Tests"), None);
        assert_eq!(rooted_path("file:///Style.xaml", "Tests"), None);
    }

    #[test]
    fn type_name_of_a_document() {
        assert_eq!(super::document_type_name("ferres://Tests/Folder/Theme.xaml").as_deref(), Some("Tests.Folder.Theme"));
        assert_eq!(super::document_type_name("ferres://Tests/Theme").as_deref(), Some("Tests"));
    }
}
