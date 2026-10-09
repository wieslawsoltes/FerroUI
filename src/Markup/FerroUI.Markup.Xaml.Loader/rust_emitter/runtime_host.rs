//! The run-time host of the emitter: the documents are transformed against the run-time
//! type system of the calling thread ([`EmitterHost::runtime`]: the types the process
//! registered, with [`RuntimeEmitTypes`]) and the group and the class of a class document
//! are the ones the run-time loader finds. A host of this kind links the framework
//! (docs/porting/xaml.md, 9.6): a test of a crate that compares the result with a
//! checked-in file, and the run-time host of a build script.

use std::rc::Rc;

use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use xamlx::exceptions::XamlResult;
use xamlx::type_system::IXamlTypeSystem;

use crate::FerroXamlIlRuntimeCompiler;

use super::compiled::{CLASS_GROUP_NOT_TRANSFORMED, GROUP_NOT_TRANSFORMED};
use super::compiled::{compile_documents_with, file_of, generate_class_file_with, ClassConstructor, ClassFile, ClassGroup, CompiledDocument, EmitterHost, GeneratedFile};
use super::compiled_resources::CompiledMarkupTypeSystem;
use super::runtime_types::RuntimeEmitTypes;
use super::source::function_name_of;
use super::transform::{transform_group, DocumentSource, TransformOptions, TransformedDocument};
use ferroui_build_scan::xaml_metadata::XamlMetadata;

impl EmitterHost<'static> {
    /// The run-time type system of the calling thread (the types the process registered),
    /// with the compiled markup of `dependencies`.
    pub fn runtime(dependencies: &[XamlMetadata]) -> XamlResult<Self> {
        let runtime = FerroXamlIlRuntimeCompiler::type_system();
        let type_system: Rc<dyn IXamlTypeSystem> = match dependencies.is_empty() {
            true => runtime.as_type_system(),
            false => CompiledMarkupTypeSystem::new(runtime.as_type_system(), dependencies)?,
        };
        Ok(Self { type_system, types: &RuntimeEmitTypes, parsers: vec![Rc::new(crate::runtime::RuntimeCompileTimeValueParser)] })
    }
}

/// The documents `(name, xaml, base URI, class of the root instance)` transformed as one
/// group against the run-time type system, with the compiled markup of `dependencies`. A
/// document with the class of its root instance (`x:Class`) is transformed to populate an
/// instance of that class.
#[cfg(any(test, feature = "testing"))]
#[allow(clippy::type_complexity)]
fn transform_with_runtime_types(
    documents: &[(&str, &str, Option<String>, Option<&'static ferroui_base::TypeInfo>)],
    configuration: &RuntimeXamlLoaderConfiguration,
    dependencies: &[XamlMetadata],
) -> XamlResult<Vec<TransformedDocument>> {
    let host = EmitterHost::runtime(dependencies)?;
    let runtime = FerroXamlIlRuntimeCompiler::type_system();
    let sources: Vec<DocumentSource<'_>> = documents
        .iter()
        .map(|(name, xaml, base_uri, class)| DocumentSource {
            name,
            xaml,
            base_uri: base_uri.clone(),
            root_type: class.map(|class| runtime.type_of_class(class)),
        })
        .collect();
    transform_group(host.type_system, &sources, &TransformOptions::of(configuration), &host.parsers)
}

/// Parses, transforms and emits the documents (`(name, xaml)`) of an assembly
/// with the configuration of the run-time loader, against the run-time type system
/// ([`compile_documents_with`] and [`EmitterHost::runtime`]). The documents are
/// transformed as ONE group, as upstream's build transforms the documents of
/// an assembly and the run-time loader a group of documents: the group
/// transformers see every document, so includes between them are resolved.
/// With `root_uri` (`ferres://MyApp/`) the base URI of a document is the
/// root URI followed by its name, the URI it is loaded by.
///
/// A document whose generated functions would have the name of a function
/// generated for an earlier document (names that differ only in case or in
/// characters that are not letters or digits, or a name that ends in
/// `_untyped`, `_deferred_<n>` or `_part_<n>`, the names of the functions of
/// its untyped build, of its deferred content and of the parts of a split
/// function) is reported instead of becoming a duplicate definition rustc
/// rejects, and is left out of the group. A group that does not transform
/// makes every document of it not eligible, with the error; a document with
/// an unsupported node is reported with the node, and the others are not
/// affected.
///
/// `dependencies` are the compiled markup of the crates the documents may include
/// documents of (their `.xamlmeta`, [`XamlMetadata::read`]): such an include calls the
/// build function of the included document in its crate, or creates its class, as
/// upstream links an include of a document of a referenced assembly (docs/porting/xaml.md,
/// 9.7.3).
pub fn compile_documents(
    documents: &[(&str, &str)],
    root_uri: Option<&str>,
    configuration: &RuntimeXamlLoaderConfiguration,
    dependencies: &[XamlMetadata],
) -> Vec<CompiledDocument> {
    match EmitterHost::runtime(dependencies) {
        Ok(host) => compile_documents_with(&host, documents, root_uri, &TransformOptions::of(configuration)),
        Err(error) => documents
            .iter()
            .map(|(name, _)| CompiledDocument {
                name: name.to_string(),
                function_name: function_name_of(name),
                source: Err(format!("{GROUP_NOT_TRANSFORMED}{}", error.message())),
                namespaces: None,
                root_type: None,
                public: true,
            })
            .collect(),
    }
}

/// The generated file of an assembly: the build function of every eligible
/// document, the table `DOCUMENTS` (document name, untyped build function) of
/// the public ones (`x:ClassModifier`; the build function of a document that
/// is not public is `pub(crate)`, upstream's `XamlVisibility.Assembly`),
/// `try_load` (the `CompiledXamlLoader` of the assembly: the document whose
/// URI is `<root_uri><name>`, compared without regard to case as upstream's
/// `string.Equals(.., StringComparison.OrdinalIgnoreCase)` compares it; a failed build
/// is its error, as it is the error of the run-time loader) and
/// `register_compiled_xaml()`.
///
/// `root_uri` ends with `/` (`ferres://MyApp/`). The text is deterministic:
/// the same documents give the same file.
///
/// `dependencies` as for [`compile_documents`].
pub fn generate_file(
    assembly_name: &str,
    root_uri: &str,
    documents: &[(&str, &str)],
    configuration: &RuntimeXamlLoaderConfiguration,
    dependencies: &[XamlMetadata],
) -> GeneratedFile {
    file_of(assembly_name, root_uri, compile_documents(documents, Some(root_uri), configuration, dependencies))
}

/// A group of documents with owned texts as the borrowed form
/// [`transform_with_runtime_types`] takes.
#[cfg(any(test, feature = "testing"))]
#[allow(clippy::type_complexity)]
fn borrowed<'a>(
    documents: &'a [(String, String, Option<String>, Option<&'static ferroui_base::TypeInfo>)],
) -> Vec<(&'a str, &'a str, Option<String>, Option<&'static ferroui_base::TypeInfo>)> {
    documents.iter().map(|(name, xaml, base_uri, class)| (name.as_str(), xaml.as_str(), base_uri.clone(), *class)).collect()
}

/// The transformed tree of one document as text (the AST dump of
/// [`crate::testing::objects::dump_tree`]): what the emitter walks, for diagnosing
/// why a document is not eligible. With the `testing` feature.
#[cfg(any(test, feature = "testing"))]
pub fn transformed_tree(name: &str, xaml: &str, configuration: &RuntimeXamlLoaderConfiguration) -> Result<String, String> {
    let transformed = transform_with_runtime_types(&[(name, xaml, None, None)], configuration, &[]).map_err(|error| error.message())?;
    transformed.first().map(|transformed| crate::testing::objects::dump_tree(&transformed.root)).ok_or_else(|| "The document was not transformed".to_string())
}

/// The transformed trees of the group of the document registered for
/// `class` (the document with every document it includes), as text, by
/// document name: what the emitter walks for a group. With the `testing`
/// feature.
#[cfg(any(test, feature = "testing"))]
pub fn transformed_class_group(class: &'static ferroui_base::TypeInfo) -> Result<Vec<(String, String)>, String> {
    let uri = crate::FerroRuntimeXamlLoader::class_document(class)
        .ok_or_else(|| format!("no document is registered for {}", class.full_name()))?;
    let group = crate::FerroRuntimeXamlLoader::document_group(&uri, &class.full_name()).map_err(|e| e.message().to_string())?;
    let mut documents = vec![(group.uri.absolute_path().trim_start_matches('/').to_string(), group.text, Some(group.uri.to_string()), Some(class))];
    for (uri, text) in group.included {
        documents.push((uri.absolute_path().trim_start_matches('/').to_string(), text, Some(uri.to_string()), None));
    }
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = group.assembly;
    let transformed = transform_with_runtime_types(&borrowed(&documents), &configuration, &[]).map_err(|error| error.message())?;
    Ok(documents
        .iter()
        .zip(transformed)
        .map(|(document, transformed)| (document.0.clone(), crate::testing::objects::dump_tree(&transformed.root)))
        .collect())
}

/// The generated file of the documents of a class: the document registered
/// for `class` (with `x:Class`) and every document it includes, transformed
/// as one group exactly as the run-time loader loads them
/// (`FerroRuntimeXamlLoader::load_object`), each emitted as a function: the
/// document of the class as `populate`, which populates an existing
/// instance, the others as build functions the include calls of the group
/// call, and every public document (`x:ClassModifier`) with a build function
/// even when the group merged it into another one. The file is a module of
/// the crate of the class (that crate is named `crate` in it).
///
/// The file ends with `try_load`, the `CompiledXamlLoader` of the documents
/// (upstream's `!XamlLoader.TryLoad`): a load by URI of
/// the document of the class creates the class with its constructor, a load of
/// another public document calls its build function with a root service
/// provider; a document that is not public has no entry, as upstream.
///
/// `constructor`: `None` lets the compiler pick the constructor of the class as
/// upstream's does ([`ClassConstructor`]); a class without one has no entry, and
/// [`ClassFile::warnings`] holds upstream's warning. `Some` states it, for a class
/// whose markup metadata declares constructors the class of upstream does not have
/// (the two themes: `new()` next to the constructor that takes the service provider,
/// where upstream has the one constructor with an optional parameter).
///
/// `module_path` is the absolute path of the module the file is the body of
/// (`::my_crate::compiled_xaml`), for the `.xamlmeta` of the file
/// ([`ClassFile::metadata`]). `dependencies` are the compiled markup of the
/// crates the documents may include documents of, as for
/// [`compile_documents`]: their documents are not part of the group.
///
/// `Err` lists the documents that are not eligible, with the reasons: a class
/// is compiled whole or not at all.
pub fn generate_class_file(
    class: &'static ferroui_base::TypeInfo,
    constructor: Option<ClassConstructor>,
    module_path: &str,
    dependencies: &[XamlMetadata],
) -> Result<ClassFile, String> {
    let uri = crate::FerroRuntimeXamlLoader::class_document(class)
        .ok_or_else(|| format!("no document is registered for {}", class.full_name()))?;
    // A document of a crate with compiled markup is that crate's: the include calls it there.
    let compiled_elsewhere = |uri: &ferroui_base::utilities::Uri| {
        let assembly = uri.absolute_uri().split_once("://").and_then(|(_, rest)| rest.split('/').next()).unwrap_or_default();
        dependencies.iter().any(|metadata| metadata.name.to_lowercase() == assembly.to_lowercase())
    };
    let group = crate::FerroRuntimeXamlLoader::document_group_without(&uri, &class.full_name(), &compiled_elsewhere)
        .map_err(|e| e.message().to_string())?;
    let name_of = |uri: &ferroui_base::utilities::Uri| uri.absolute_path().trim_start_matches('/').to_string();
    let mut documents = vec![(name_of(&group.uri), group.text.clone(), Some(group.uri.to_string()))];
    for (uri, text) in &group.included {
        documents.push((name_of(uri), text.clone(), Some(uri.to_string())));
    }
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = group.assembly;
    let host = EmitterHost::runtime(dependencies).map_err(|error| format!("{CLASS_GROUP_NOT_TRANSFORMED}{}", error.message()))?;
    let class = ClassGroup {
        class: FerroXamlIlRuntimeCompiler::type_system().type_of_class(class),
        documents: &documents,
        constructor,
        module_path,
    };
    generate_class_file_with(&host, &class, &TransformOptions::of(&configuration))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_build_scan::xaml_metadata::DocumentModel;

    fn reason(compiled: &[CompiledDocument], name: &str) -> Option<String> {
        compiled.iter().find(|document| document.name == name).and_then(|document| document.source.clone().err())
    }

    /// The compiled markup of a crate `Library` (assembly `Library`): a public style and
    /// a style that is not public.
    fn library() -> XamlMetadata {
        let document = |name: &str, public: bool| DocumentModel {
            uri: format!("ferres://library/{name}"),
            root_type: "FerroUI.Styling.Style".to_string(),
            class_rust_path: None,
            build_path: Some(format!("::library::compiled_xaml::{}", function_name_of(name))),
            populate_path: None,
            public,
        };
        XamlMetadata {
            name: "Library".to_string(),
            crate_name: "library".to_string(),
            documents: vec![document("Style.xaml", true), document("Internal/Style.xaml", false)],
            dependencies: Vec::new(),
        }
    }

    fn including(include: &str) -> String {
        format!(
            "<ContentControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n\
             <ContentControl.Resources>{include}</ContentControl.Resources>\n\
             </ContentControl>"
        )
    }

    /// Not from upstream: an include of a public document of another crate calls its
    /// build function there (`FromMethod` over `Build:<path>` of `!FerroResources`).
    #[test]
    fn include_of_a_document_of_another_crate_calls_its_build_function() {
        let xaml = including("<StyleInclude x:Key='Include' Source='ferres://Library/Style.xaml'/>");
        let compiled = compile_documents(&[("Root.xaml", &xaml)], Some("ferres://App/"), &RuntimeXamlLoaderConfiguration::new(), &[library()]);
        let source = compiled[0].source.clone().unwrap_or_else(|reason| panic!("not eligible: {reason}"));
        assert!(source.contains("= ::library::compiled_xaml::build_style_xaml("), "{source}");
        assert!(!source.contains("StyleInclude"), "{source}");
    }

    /// Not from upstream: the diagnostics of `XamlIncludeGroupTransformer` and
    /// `XamlMergeResourceGroupTransformer` for documents of another crate.
    #[test]
    fn include_of_another_crate_reports_upstream_diagnostics() {
        let reason = |xaml: &str| {
            let compiled = compile_documents(&[("Root.xaml", xaml)], Some("ferres://App/"), &RuntimeXamlLoaderConfiguration::new(), &[library()]);
            compiled[0].source.clone().expect_err("the include is an error")
        };
        // A document that is not public, and a document the crate does not have.
        for path in ["Internal/Style.xaml", "Missing.xaml"] {
            let error = reason(&including(&format!("<StyleInclude x:Key='Include' Source='ferres://Library/{path}'/>")));
            assert!(
                error.contains(&format!(
                    "Unable to resolve XAML resource \"ferres://library/{path}\" in the \"library\" assembly. Make sure this file exists and is public."
                )),
                "{error}"
            );
        }
        // Merging works within one compilation only.
        let error = reason(
            "<ResourceDictionary xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\
             <ResourceDictionary.MergedDictionaries><MergeResourceInclude Source='ferres://Library/Style.xaml'/></ResourceDictionary.MergedDictionaries>\
             <x:String x:Key='own'>1</x:String>\
             </ResourceDictionary>",
        );
        assert!(error.contains("Node MergeResourceInclude is unable to resolve \"ferres://library/Style.xaml\" path."), "{error}");
    }

    /// Not from upstream: documents whose build functions would have the same name are
    /// reported as a diagnostic of the later document, not left to rustc.
    #[test]
    fn colliding_function_names_are_reported() {
        let xaml = "<Border xmlns='https://github.com/ferroui'/>";
        let documents = [
            ("a-b.xaml", xaml),
            ("a_b.xaml", xaml),
            ("Case.xaml", xaml),
            ("case.xaml", xaml),
            ("x.xaml", xaml),
            ("x.xaml_untyped", xaml),
            ("y.xaml_deferred_0", xaml),
            ("y.xaml", xaml),
        ];
        let compiled = compile_documents(&documents, None, &RuntimeXamlLoaderConfiguration::new(), &[]);
        for (name, other, item) in [
            ("a_b.xaml", "a-b.xaml", "build_a_b_xaml"),
            ("case.xaml", "Case.xaml", "build_case_xaml"),
            ("x.xaml_untyped", "x.xaml", "build_x_xaml_untyped"),
            ("y.xaml", "y.xaml_deferred_0", "build_y_xaml"),
        ] {
            let reason = reason(&compiled, name).unwrap_or_else(|| panic!("{name} is not reported"));
            assert_eq!(
                reason,
                format!("the generated function `{item}` would also be defined for the document `{other}`; rename one of them")
            );
        }
        for name in ["a-b.xaml", "Case.xaml", "x.xaml", "y.xaml_deferred_0"] {
            let reason = reason(&compiled, name).unwrap_or_default();
            assert!(!reason.contains("would also be defined"), "{name}: {reason}");
        }
    }
}
