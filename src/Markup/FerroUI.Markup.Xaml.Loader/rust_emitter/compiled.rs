//! From documents to a generated file: parse + transform (the sequence of
//! the run-time loader) + emit, and the file that holds the build functions
//! of the eligible documents with the table that registers them as the
//! compiled markup of an assembly (docs/porting/xaml.md, 9.7).
//!
//! # Hosts of the emitter
//!
//! A host of the emitter states the type system the documents are transformed
//! and emitted against ([`EmitterHost`]). With the run-time type system
//! ([`EmitterHost::runtime`]) the host links the framework, and the types the
//! documents name are the types the host registered. Two such hosts exist
//! (docs/porting/xaml.md, 9.6):
//!
//! - a build script (`ferroui-build`, `src/FerroUI.Build.Tasks`), which takes
//!   the crates of those types as build dependencies and writes the file to
//!   `OUT_DIR`. A build script cannot link the crate it builds, so this host
//!   compiles the documents that name types of other crates only;
//! - a test of the crate, which calls [`generate_file`] or
//!   [`generate_class_file`] and compares the result with the CHECKED-IN
//!   file (an ignored test rewrites it). It links the crate, so it compiles
//!   the documents of the classes of the crate (`x:Class`) and the documents
//!   that name its types.
//!
//! With the build-time type system over the models of the crates (section 9.5;
//! `ferroui-build` with `TypeSystem::Model`) the host links no crate for its
//! types, the crate it builds included: [`generate_file_with`] and
//! [`generate_class_file_with`] compile against the type system the host
//! states, and [`class_document_group`] and [`class_of_document`] read the
//! group and the class of a class document from the documents themselves,
//! where a host with the run-time type system asks the run-time loader.

use std::rc::Rc;

use xamlx::type_system::IXamlTypeSystem;

use crate::compiler_extensions::IXamlCompileTimeValueParser;

use super::emit_types::{EmitClass, EmitTypes, Known};
use super::emitter::{emit_function, root_class_of, DocumentFunctions};
use super::transform::{transform_group, DocumentSource, TransformOptions};
use super::source::{function_name_of, rust_string_literal};
use ferroui_build_scan::xaml_metadata::{DocumentModel, XamlMetadata};

/// How the reason of a document of a group that did not transform starts
/// ([`CompiledDocument::source`], [`GeneratedFile::documents`]): the error of the transform
/// follows. A host tells it from the reason of a document the emitter refuses by it.
pub const GROUP_NOT_TRANSFORMED: &str = "the group of documents does not transform: ";

/// How the reason of a class whose group did not transform starts
/// ([`generate_class_file_with`]).
pub const CLASS_GROUP_NOT_TRANSFORMED: &str = "the group does not transform: ";

/// The result of compiling one document.
pub struct CompiledDocument {
    /// The name of the document (its path below the root URI of the assembly).
    pub name: String,
    /// The name of the generated build function.
    pub function_name: String,
    /// The Rust source of the build function, or why the document is not eligible.
    pub source: Result<String, String>,
    /// The constant the build function reads the namespace information of
    /// the document from, and its value (`rt::XmlNamespaceTable`); documents
    /// with the same information share the constant.
    pub namespaces: Option<(String, String)>,
    /// The full name of the type of the root of the transformed document.
    pub root_type: Option<String>,
    /// Whether the document is public (`x:ClassModifier`; a document without the
    /// directive is public): only a public document has an entry in the loader table
    /// and a build function other crates can call.
    pub public: bool,
}

/// A generated file and what it holds.
pub struct GeneratedFile {
    /// The complete text of the file.
    pub source: String,
    /// The position map of the file (docs/porting/xaml.md, 9.3.6), JSON: for
    /// every range of generated lines that a position marker heads, the
    /// document and the line and position of the node the lines emit
    /// ([`position_map`]).
    pub position_map: String,
    /// For every document, in order: its name and, when it is not eligible, why.
    pub documents: Vec<(String, Option<String>)>,
    /// The assembly and the root URI of the documents.
    assembly_name: String,
    root_uri: String,
    /// For every eligible document, in order: its name, the full name of its root type,
    /// its build function and whether it is public.
    compiled: Vec<(String, String, String, bool)>,
}

impl GeneratedFile {
    /// The `.xamlmeta` of the file (docs/porting/xaml.md, 9.7.3): its compiled documents,
    /// for crates that include them. `module_path` is the absolute path of the module the
    /// file is the body of (`::my_crate::compiled_xaml`); `dependencies` are the
    /// `.xamlmeta` files of the crates the documents include documents of, relative to
    /// the file the metadata is written to.
    pub fn metadata(&self, crate_name: &str, module_path: &str, dependencies: &[&str]) -> XamlMetadata {
        XamlMetadata {
            name: self.assembly_name.clone(),
            crate_name: crate_name.to_string(),
            documents: self
                .compiled
                .iter()
                .map(|(name, root_type, function, public)| DocumentModel {
                    uri: format!("{}{name}", self.root_uri),
                    root_type: root_type.clone(),
                    class_rust_path: None,
                    build_path: Some(format!("{module_path}::{function}")),
                    populate_path: None,
                    public: *public,
                })
                .collect(),
            dependencies: dependencies.iter().map(|path| path.to_string()).collect(),
        }
    }
}

/// What a group of documents is compiled against: the type system of the transform, what
/// that type system states for the emitter, and the compile-time value parsers of the
/// host.
pub struct EmitterHost<'a> {
    pub type_system: Rc<dyn IXamlTypeSystem>,
    pub types: &'a dyn EmitTypes,
    pub parsers: Vec<Rc<dyn IXamlCompileTimeValueParser>>,
}

/// [`compile_documents`] against the type system of `host`: the documents are
/// transformed against it ([`transform_group`]) and emitted from what it states for the
/// emitter. The compiled markup of other crates is part of the type system of the host.
pub fn compile_documents_with(
    host: &EmitterHost<'_>,
    documents: &[(&str, &str)],
    root_uri: Option<&str>,
    options: &TransformOptions,
) -> Vec<CompiledDocument> {
    let mut compiled: Vec<CompiledDocument> = Vec::with_capacity(documents.len());
    // The items of the file each document defines, with the document that defines them.
    let mut items: Vec<(String, &str)> = Vec::with_capacity(documents.len() * 2);
    let mut group: Vec<(usize, &str, &str, Option<String>)> = Vec::with_capacity(documents.len());
    for (name, xaml) in documents {
        let function_name = function_name_of(name);
        let defined = [function_name.clone(), untyped_function_name(&function_name)];
        let collision = defined
            .iter()
            .find_map(|item| items.iter().find(|(known, _)| collides(known, item)).map(|(_, other)| (item.clone(), *other)));
        items.extend(defined.into_iter().map(|item| (item, *name)));
        if let Some((item, other)) = collision {
            let reason = format!("the generated function `{item}` would also be defined for the document `{other}`; rename one of them");
            compiled.push(CompiledDocument {
                name: name.to_string(),
                function_name,
                source: Err(reason),
                namespaces: None,
                root_type: None,
                public: true,
            });
            continue;
        }
        let public = match class_modifier_public(xaml) {
            Ok(public) => public.unwrap_or(true),
            Err(reason) => {
                compiled.push(CompiledDocument {
                    name: name.to_string(),
                    function_name,
                    source: Err(reason),
                    namespaces: None,
                    root_type: None,
                    public: true,
                });
                continue;
            }
        };
        group.push((compiled.len(), name, xaml, root_uri.map(|root| format!("{root}{name}"))));
        compiled.push(CompiledDocument {
            name: name.to_string(),
            function_name,
            source: Err(String::new()),
            namespaces: None,
            root_type: None,
            public,
        });
    }
    let sources: Vec<DocumentSource<'_>> =
        group.iter().map(|(_, name, xaml, base_uri)| DocumentSource { name, xaml, base_uri: base_uri.clone(), root_type: None }).collect();
    if sources.is_empty() {
        return compiled;
    }
    let types = host.types;
    match transform_group(host.type_system.clone(), &sources, options, &host.parsers) {
        Ok(transformed) => {
            // The build methods of the documents of the group, by their functions: what
            // an include the group transformers linked calls.
            let names: Vec<String> = group.iter().map(|(index, _, _, _)| compiled[*index].function_name.clone()).collect();
            let functions_of_group = || {
                let mut functions = DocumentFunctions::default();
                for (name, transformed) in names.iter().zip(&transformed) {
                    if let (Some(build), Some(root_class)) = (&transformed.build, root_class_of(types, &transformed.root)) {
                        functions.insert(build, name, root_class);
                    }
                }
                functions
            };
            // The functions each document calls.
            let mut calls: Vec<(usize, Vec<String>)> = Vec::with_capacity(group.len());
            // The namespace information of the documents, one constant per distinct table.
            let mut tables: Vec<String> = Vec::new();
            for ((index, name, _, _), transformed) in group.iter().zip(&transformed) {
                let functions = functions_of_group();
                let document = &mut compiled[*index];
                let Some(table) = transformed.namespaces.clone() else {
                    document.source = Err("the document has no namespace information".to_string());
                    continue;
                };
                let table_index = match tables.iter().position(|known| *known == table) {
                    Some(known) => known,
                    None => {
                        tables.push(table.clone());
                        tables.len() - 1
                    }
                };
                let constant = format!("XML_NAMESPACES_{table_index}");
                document.root_type = root_type_name(&transformed.root);
                let _ = types.take_unanswered();
                document.source = emit_function(
                    types,
                    &transformed.root,
                    &transformed.configuration,
                    transformed.base_uri.as_deref(),
                    &constant,
                    &document.function_name,
                    name,
                    &functions,
                    None,
                )
                .map(|source| if document.public { source } else { source.replacen("pub fn ", "pub(crate) fn ", 1) })
                .map_err(|e| e.to_string());
                // A question the type system of the host could not answer: the document is
                // refused with it, whatever was emitted from the answer it gave instead.
                let unanswered = types.take_unanswered();
                if !unanswered.is_empty() {
                    document.source = Err(format!("the type system of the host cannot answer: {}", unanswered.join("; ")));
                }
                document.namespaces = Some((constant, table));
                calls.push((*index, functions.called()));
            }
            // A document that calls the function of a document that is not eligible is not
            // eligible either.
            loop {
                let failed: Vec<(usize, String)> = calls
                    .iter()
                    .filter(|(index, _)| compiled[*index].source.is_ok())
                    .filter_map(|(index, called)| {
                        called
                            .iter()
                            .find(|function| compiled.iter().any(|d| d.function_name == **function && d.source.is_err()))
                            .map(|function| (*index, function.clone()))
                    })
                    .collect();
                if failed.is_empty() {
                    break;
                }
                for (index, function) in failed {
                    compiled[index].source = Err(format!("it calls `{function}`, whose document is not eligible"));
                }
            }
        }
        Err(error) => {
            for (index, _, _, _) in &group {
                compiled[*index].source = Err(format!("{GROUP_NOT_TRANSFORMED}{}", error.message()));
            }
        }
    }
    compiled
}

/// [`generate_file`] against the type system of `host` ([`compile_documents_with`]).
pub fn generate_file_with(
    host: &EmitterHost<'_>,
    assembly_name: &str,
    root_uri: &str,
    documents: &[(&str, &str)],
    options: &TransformOptions,
) -> GeneratedFile {
    file_of(assembly_name, root_uri, compile_documents_with(host, documents, Some(root_uri), options))
}

/// The generated file of the compiled documents of an assembly.
pub(super) fn file_of(assembly_name: &str, root_uri: &str, compiled: Vec<CompiledDocument>) -> GeneratedFile {
    let mut source = String::new();
    source.push_str("// @generated by the Rust emitter of ferroui-markup-xaml-loader (rust_emitter::generate_file).\n");
    source.push_str("// Do not edit: regenerate it (see the header of the module that includes this file).\n");
    source.push('\n');
    source.push_str("#[allow(unused_imports)]\n");
    source.push_str("use ::ferroui_markup_xaml::xaml_il::runtime::compiled as rt;\n");
    source.push('\n');
    source.push_str("/// The assembly the documents of this file belong to.\n");
    source.push_str(&format!("pub const ASSEMBLY_NAME: &str = {};\n", rust_string_literal(assembly_name)));
    source.push('\n');
    source.push_str("/// The URI of a document is this followed by its name.\n");
    source.push_str(&format!("pub const ROOT_URI: &str = {};\n", rust_string_literal(root_uri)));
    source.push('\n');
    source.push_str("/// An untyped build function: the root object as the loader returns it.\n");
    source.push_str("pub type BuildDocument = fn(\n");
    source.push_str("    ::core::option::Option<::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>,\n");
    source.push_str(") -> ::core::result::Result<::ferroui_base::BoxedValue, ::ferroui_markup_xaml::XamlLoadException>;\n");

    let mut constants: Vec<&(String, String)> = Vec::new();
    for document in &compiled {
        if let (Ok(_), Some(namespaces)) = (&document.source, &document.namespaces) {
            if !constants.iter().any(|known| known.0 == namespaces.0) {
                constants.push(namespaces);
            }
        }
    }
    for (constant, value) in constants {
        source.push('\n');
        source.push_str("/// The XML namespaces of documents of this file, as the compiler resolved them.\n");
        source.push_str(&format!("const {constant}: rt::XmlNamespaceTable = {value};\n"));
    }

    let mut report = Vec::with_capacity(compiled.len());
    let mut table = Vec::new();
    let mut exported = Vec::new();
    for document in &compiled {
        match &document.source {
            Ok(function) => {
                exported.push((
                    document.name.clone(),
                    document.root_type.clone().unwrap_or_default(),
                    document.function_name.clone(),
                    document.public,
                ));
                source.push('\n');
                source.push_str(function);
                if document.public {
                    source.push('\n');
                    source.push_str(&format!("fn {}(\n", untyped_function_name(&document.function_name)));
                    source.push_str("    service_provider: ::core::option::Option<::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>,\n");
                    source.push_str(") -> ::core::result::Result<::ferroui_base::BoxedValue, ::ferroui_markup_xaml::XamlLoadException> {\n");
                    source.push_str(&format!("    let root = {}(service_provider)?;\n", document.function_name));
                    source.push_str("    ::core::result::Result::Ok(::std::rc::Rc::new(root) as ::ferroui_base::BoxedValue)\n");
                    source.push_str("}\n");
                    table.push(format!(
                        "    ({}, {} as BuildDocument),\n",
                        rust_string_literal(&document.name),
                        untyped_function_name(&document.function_name)
                    ));
                }
                report.push((document.name.clone(), None));
            }
            Err(reason) => report.push((document.name.clone(), Some(reason.clone()))),
        }
    }

    source.push('\n');
    source.push_str("/// The build function of every eligible public document, by document name.\n");
    source.push_str("pub const DOCUMENTS: &[(&str, BuildDocument)] = &[\n");
    for entry in &table {
        source.push_str(entry);
    }
    source.push_str("];\n");
    source.push('\n');
    source.push_str("/// The loader of the compiled markup of the assembly: builds the document with the URI\n");
    source.push_str("/// `uri` (compared as upstream's `OrdinalIgnoreCase`, `rt::uri_equals`); `Ok(None)` if this\n");
    source.push_str("/// file has no such document,\n");
    source.push_str("/// the load error of the build if it fails.\n");
    source.push_str("pub fn try_load(\n");
    source.push_str("    service_provider: ::core::option::Option<&::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>,\n");
    source.push_str("    uri: &str,\n");
    source.push_str(") -> ::core::result::Result<::core::option::Option<::ferroui_base::BoxedValue>, ::ferroui_markup_xaml::XamlLoadException> {\n");
    source.push_str("    let ::core::option::Option::Some((_, build)) = ::core::iter::Iterator::find(&mut DOCUMENTS.iter(), |(document, _)| rt::uri_equals(uri, ROOT_URI, document)) else {\n");
    source.push_str("        return ::core::result::Result::Ok(::core::option::Option::None);\n");
    source.push_str("    };\n");
    source.push_str("    let provider = ::ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v3(\n");
    source.push_str("        service_provider.cloned(),\n");
    source.push_str("    );\n");
    source.push_str("    build(::core::option::Option::Some(provider)).map(::core::option::Option::Some)\n");
    source.push_str("}\n");
    source.push('\n');
    source.push_str("/// Registers the documents of this file as the compiled markup of the assembly.\n");
    source.push_str("pub fn register_compiled_xaml() {\n");
    source.push_str("    ::ferroui_markup_xaml::FerroXamlLoader::register_compiled_xaml(ASSEMBLY_NAME, try_load);\n");
    source.push_str("}\n");
    let names: Vec<(String, String)> = compiled
        .iter()
        .filter(|document| document.source.is_ok())
        .map(|document| (document.function_name.clone(), document.name.clone()))
        .collect();
    let position_map = position_map(&source, &names);
    GeneratedFile {
        source,
        position_map,
        documents: report,
        assembly_name: assembly_name.to_string(),
        root_uri: root_uri.to_string(),
        compiled: exported,
    }
}

/// The full name (`Namespace.Name`) of the type of the root of a transformed document.
fn root_type_name(root: &std::rc::Rc<dyn xamlx::ast::IXamlAstNode>) -> Option<String> {
    use xamlx::ast::{XamlAstExtensions, XamlAstNodeExtensions};
    let value = root.cast::<dyn xamlx::ast::IXamlAstValueNode>()?;
    value.type_().get_clr_type().ok().map(|type_| type_.full_name())
}

/// The position map of a generated file (docs/porting/xaml.md, 9.3.6): the
/// link from a line of generated Rust (where rustc reports an error) back to
/// the XAML node it was emitted for. `functions` are the build functions of
/// the file with their documents; the build functions of their deferred
/// content (`<function>_deferred_<n>`) and the parts of a split function
/// (`<function>_part_<n>`) belong to the same documents. Inside
/// a build function every position
/// marker the emitter writes (`// <document>(<line>,<position>) <what>`)
/// heads the lines up to the next marker or the end of the function; each
/// such range is one entry:
///
/// ```text
/// {"file_lines": [first, last], "document": "name.xaml", "line": 3, "position": 7}
/// ```
///
/// Generated lines are counted from 1. The text is deterministic.
pub fn position_map(source: &str, functions: &[(String, String)]) -> String {
    let mut entries: Vec<String> = Vec::new();
    let mut document: Option<&str> = None;
    // (first line, document line, document position) of the open range.
    let mut open: Option<(usize, i32, i32)> = None;
    let close = |entries: &mut Vec<String>, open: &mut Option<(usize, i32, i32)>, document: &str, last: usize| {
        if let Some((first, line, position)) = open.take() {
            entries.push(format!(
                "    {{\"file_lines\": [{first}, {last}], \"document\": {}, \"line\": {line}, \"position\": {position}}}",
                json_string(document)
            ));
        }
    };
    for (index, text) in source.lines().enumerate() {
        let number = index + 1;
        if let Some(name) = functions.iter().find_map(|(function, name)| {
            let build = text
                .strip_prefix("pub fn ")
                .or_else(|| text.strip_prefix("pub(crate) fn "))
                .and_then(|rest| rest.strip_prefix(function.as_str()));
            let deferred = || {
                let rest = text.strip_prefix("fn ")?.strip_prefix(function.as_str())?;
                let rest = rest.strip_prefix("_deferred_").or_else(|| rest.strip_prefix("_part_"))?;
                rest.trim_start_matches(|c: char| c.is_ascii_digit()).starts_with('(').then_some(rest)
            };
            build.filter(|rest| rest.starts_with('(')).or_else(deferred).map(|_| name)
        }) {
            document = Some(name);
            continue;
        }
        let Some(current) = document else { continue };
        if text == "}" {
            close(&mut entries, &mut open, current, number - 1);
            document = None;
            continue;
        }
        let marker = current.replace(['\r', '\n'], " ");
        if let Some(rest) = text.trim_start().strip_prefix("// ").and_then(|rest| rest.strip_prefix(marker.as_str())) {
            let position = rest
                .strip_prefix('(')
                .and_then(|rest| rest.split_once(')'))
                .and_then(|(numbers, _)| numbers.split_once(','))
                .and_then(|(line, position)| Some((line.parse::<i32>().ok()?, position.parse::<i32>().ok()?)));
            if let Some((line, position)) = position {
                close(&mut entries, &mut open, current, number - 1);
                open = Some((number, line, position));
            }
        }
    }
    let mut map = String::from("{\n  \"entries\": [\n");
    map.push_str(&entries.join(",\n"));
    if !entries.is_empty() {
        map.push('\n');
    }
    map.push_str("  ]\n}\n");
    map
}

/// `text` as a JSON string literal.
fn json_string(text: &str) -> String {
    let mut literal = String::with_capacity(text.len() + 2);
    literal.push('"');
    for character in text.chars() {
        match character {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            c if (c as u32) < 0x20 => literal.push_str(&format!("\\u{:04x}", c as u32)),
            c => literal.push(c),
        }
    }
    literal.push('"');
    literal
}

/// Whether the items `a` and `b` of two documents would have the same name:
/// the names are equal, or one is the build function of deferred content
/// (`<function>_deferred_<n>`) or a part of a split function
/// (`<function>_part_<n>`) of the other.
fn collides(a: &str, b: &str) -> bool {
    let is_deferred_of = |deferred: &str, function: &str| {
        deferred
            .strip_prefix(function)
            .and_then(|rest| rest.strip_prefix("_deferred_").or_else(|| rest.strip_prefix("_part_")))
            .is_some_and(|index| !index.is_empty() && index.chars().all(|c| c.is_ascii_digit()))
    };
    a == b || is_deferred_of(a, b) || is_deferred_of(b, a)
}

/// The name of the untyped build function that wraps the build function `function_name`.
fn untyped_function_name(function_name: &str) -> String {
    format!("{function_name}_untyped")
}

/// The public constructor of a class that the loader table of its document
/// creates the class with (`XamlCompilerTaskExecutor`: a public parameterless
/// constructor, else a public constructor whose single parameter is the
/// service provider). The name is the associated function of the class.
///
/// [`generate_class_file`] picks it as upstream's compiler does unless its
/// caller states it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassConstructor {
    /// `new T()`: a function without parameters.
    Parameterless(&'static str),
    /// `new T(CreateRootServiceProviderV3(serviceProvider))`: a function that takes
    /// `Option<Rc<dyn IServiceProvider>>`.
    ServiceProvider(&'static str),
}

/// [`ClassConstructor`] with the function as owned text: the typed function of a constructor
/// a type system over the models declares is not a text of the program.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Constructor {
    Parameterless(String),
    ServiceProvider(String),
}

impl From<ClassConstructor> for Constructor {
    fn from(constructor: ClassConstructor) -> Self {
        match constructor {
            ClassConstructor::Parameterless(function) => Constructor::Parameterless(function.to_string()),
            ClassConstructor::ServiceProvider(function) => Constructor::ServiceProvider(function.to_string()),
        }
    }
}

impl Constructor {
    /// The constructor upstream's compiler creates the class with in the loader table
    /// (`XamlCompilerTaskExecutor`), read from the constructors the type system projects
    /// for the class: a public parameterless constructor (the default constructor of
    /// the class, `T::new()`), else a public constructor whose single parameter is the
    /// service provider (the typed function of the declared constructor,
    /// `T::__markup_new_<n>`). `Ok(None)` when the class has neither: upstream reports
    /// the document as not reachable through the loader and writes no entry for it.
    ///
    /// `Err` when the constructor exists and cannot be called from generated code: it
    /// has no typed function, the function is fallible, or its parameter is not
    /// `Option<Rc<dyn IServiceProvider>>`.
    fn of(
        types: &dyn EmitTypes,
        class_type: &Rc<dyn xamlx::type_system::IXamlType>,
        class: &dyn EmitClass,
        configuration: &xamlx::transform::TransformerConfiguration,
    ) -> Result<Option<Self>, String> {
        let constructors = class_type.constructors();
        if constructors.iter().any(|c| c.is_public() && !c.is_static() && c.parameters().is_empty()) {
            if !class.has_default_constructor() {
                return Err(format!("the parameterless constructor of {} is not its default constructor", class.full_name()));
            }
            return Ok(Some(Constructor::Parameterless("new".to_string())));
        }
        let service_provider = configuration.type_mappings.service_provider().map_err(|e| e.message())?;
        let Some(constructor) = constructors.iter().find(|c| {
            let parameters = c.parameters();
            c.is_public() && !c.is_static() && parameters.len() == 1 && parameters[0].equals(&*service_provider)
        }) else {
            return Ok(None);
        };
        let optional = types.known(Known::OptionServiceProvider);
        let emit = types
            .constructor(&**constructor)
            .filter(|constructor| constructor.parameter_handles.first().copied().flatten().is_some_and(|handle| handle.id() == optional))
            .and_then(|constructor| constructor.declared.flatten())
            .filter(|emit| !emit.fallible)
            .ok_or_else(|| {
                format!(
                    "the constructor of {} that takes the service provider has no typed function \
                     `fn(Option<Rc<dyn IServiceProvider>>) -> Ref<Self>`",
                    class.full_name()
                )
            })?;
        Ok(Some(Constructor::ServiceProvider(emit.function)))
    }
}

/// The text of the directive `x:<name>` of the root of the document `xaml`: `Ok(None)`
/// when the root has no such directive, `Ok(Some(None))` when its value is not text.
fn root_directive(xaml: &str, name: &str) -> Result<Option<Option<String>>, String> {
    use xamlx::ast::{XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode, XamlAstXmlDirective};
    let parsed = xamlx::parsers::XDocumentXamlParser::parse(xaml, None).map_err(|e| e.message())?;
    let root = parsed.root().map_err(|e| e.message())?;
    let Some(root) = root.cast::<XamlAstObjectNode>() else {
        return Ok(None);
    };
    let children = root.children.borrow();
    let directive = children.iter().filter_map(|child| child.cast::<XamlAstXmlDirective>()).find(|directive| {
        directive.namespace.borrow().as_deref() == Some(xamlx::xaml_namespaces::XamlNamespaces::XAML2006)
            && *directive.name.borrow() == name
    });
    let Some(directive) = directive else {
        return Ok(None);
    };
    let text = directive.values.borrow().first().and_then(|value| value.cast::<XamlAstTextNode>()).map(|text| text.text());
    Ok(Some(text))
}

/// Whether the document `xaml` is public, from the `x:ClassModifier` directive of
/// its root (`XamlCompilerTaskExecutor`): `Public` is public, `NotPublic` and
/// `Internal` are not (compared without regard to case), a document without the
/// directive is public. `Ok(None)` when the directive is absent.
fn class_modifier_public(xaml: &str) -> Result<Option<bool>, String> {
    let Some(text) = root_directive(xaml, "ClassModifier")? else {
        return Ok(None);
    };
    match text.map(|text| text.trim().to_lowercase()).as_deref() {
        Some("public") => Ok(Some(true)),
        // The XAML specification uses "Public" and "NotPublic", the WPF documentation "public" and "internal".
        Some("notpublic") | Some("internal") => Ok(Some(false)),
        _ => Err("Invalid value for x:ClassModifier. Expected value are: Public, NotPublic (internal).".to_string()),
    }
}

/// The full name of the class of the document `xaml` (the `x:Class` directive of its
/// root, as the parser of the compiler reads it), or nothing for a document without one:
/// what a host that does not ask the run-time loader finds the class of a document by.
pub fn class_of_document(xaml: &str) -> Result<Option<String>, String> {
    Ok(root_directive(xaml, "Class")?.flatten().map(|class| class.trim().to_string()))
}

/// The group the document of a class is compiled as, read from the documents of its
/// crate instead of from the run-time loader: the document `class_document` and every
/// document of `documents` (`(name, xaml)`, by their path below `root_uri`) it includes,
/// directly or not, in the order the includes are met. It is the group
/// [`generate_class_file`] gets from `FerroRuntimeXamlLoader::document_group_without`,
/// with `documents` in the place of the assets of the crate: a document of another
/// crate is not among them, so an include of one is left to the compiled markup of that
/// crate. Each document is `(name, xaml, URI)` as [`ClassGroup::documents`] takes it.
pub fn class_document_group(
    root_uri: &str,
    documents: &[(&str, &str)],
    class_document: &str,
) -> Result<Vec<(String, String, Option<String>)>, String> {
    use ferroui_base::utilities::{Uri, UriKind};
    let mut uris: Vec<Uri> = Vec::with_capacity(documents.len());
    for (name, _) in documents {
        let text = format!("{root_uri}{name}");
        uris.push(Uri::new(&text, UriKind::Absolute).map_err(|e| format!("The URI '{text}' of the XAML document {name} is invalid: {e}"))?);
    }
    let start = documents
        .iter()
        .position(|(name, _)| *name == class_document)
        .ok_or_else(|| format!("the document `{class_document}` of the class is not a document of the group"))?;
    // The documents in the order they are found; a document is found once.
    fn collect(documents: &[(&str, &str)], uris: &[Uri], index: usize, found: &mut Vec<usize>) {
        for source in crate::back_end::include_sources(documents[index].1) {
            let Some(relative) = Uri::try_create(&source, UriKind::RelativeOrAbsolute) else { continue };
            let included = match relative.is_absolute_uri() {
                true => relative,
                false => Uri::combine(&uris[index], &relative),
            };
            let Some(next) = uris.iter().position(|uri| uri.absolute_uri() == included.absolute_uri()) else { continue };
            if found.contains(&next) {
                continue;
            }
            found.push(next);
            collect(documents, uris, next, found);
        }
    }
    let mut found = vec![start];
    collect(documents, &uris, start, &mut found);
    Ok(found
        .into_iter()
        .map(|index| (uris[index].absolute_path().trim_start_matches('/').to_string(), documents[index].1.to_string(), Some(uris[index].to_string())))
        .collect())
}

/// The documents of a class, for [`generate_class_file_with`].
pub struct ClassGroup<'a> {
    /// The type of the class in the type system of the host.
    pub class: Rc<dyn xamlx::type_system::IXamlType>,
    /// The documents `(name, xaml, URI)`: the document of the class, then the documents it
    /// includes, directly or not ([`class_document_group`]).
    pub documents: &'a [(String, String, Option<String>)],
    /// The constructor of the class, or nothing to let the compiler pick it
    /// ([`generate_class_file`]).
    pub constructor: Option<ClassConstructor>,
    /// The absolute path of the module the file is the body of.
    pub module_path: &'a str,
}

/// [`generate_class_file`] against the type system of `host`, for the documents `group`
/// states: the document of the class is transformed to populate an instance of
/// [`ClassGroup::class`], and the class, its public Rust path and its constructor are the
/// ones the type system of the host states for that type. A question the type system
/// cannot answer for a document makes the class not eligible, with the question.
pub fn generate_class_file_with(host: &EmitterHost<'_>, group: &ClassGroup<'_>, options: &TransformOptions) -> Result<ClassFile, String> {
    let types = host.types;
    let (documents, module_path) = (group.documents, group.module_path);
    if documents.is_empty() {
        return Err("the group of the class has no document".to_string());
    }
    let class = types
        .class_of(&*group.class)
        .ok_or_else(|| format!("{} is not a class of the object model", group.class.full_name()))?;
    let sources: Vec<DocumentSource<'_>> = documents
        .iter()
        .enumerate()
        .map(|(index, (name, xaml, base_uri))| DocumentSource {
            name,
            xaml,
            base_uri: base_uri.clone(),
            root_type: (index == 0).then(|| group.class.clone()),
        })
        .collect();
    let transformed = transform_group(host.type_system.clone(), &sources, options, &host.parsers)
        .map_err(|error| format!("{CLASS_GROUP_NOT_TRANSFORMED}{}", error.message()))?;
    let constructor = match group.constructor {
        Some(constructor) => Some(Constructor::from(constructor)),
        None => Constructor::of(types, &group.class, class, &transformed[0].configuration)?,
    };
    let mut warnings = Vec::new();
    if constructor.is_none() {
        warnings.push(format!(
            "XAML resource \"{}\" won't be reachable via runtime loader, as no public constructor was found",
            documents[0].2.clone().unwrap_or_default()
        ));
    }

    // Whether each document is public. A class document follows its class, which has a
    // public Rust path; the directive, if present, must agree, as upstream validates it.
    let mut public = Vec::with_capacity(documents.len());
    for (index, document) in documents.iter().enumerate() {
        let modifier = class_modifier_public(&document.1).map_err(|reason| format!("{}: {reason}", document.0))?;
        if index == 0 && modifier == Some(false) {
            return Err(format!("{}: XAML file x:ClassModifier doesn't match the x:Class type modifiers.", document.0));
        }
        public.push(modifier.unwrap_or(true));
    }

    // The functions of the documents, by their build methods.
    let mut functions = DocumentFunctions::default();
    let mut names = Vec::with_capacity(documents.len());
    let mut has_build = vec![false; documents.len()];
    for (index, (document, transformed)) in documents.iter().zip(&transformed).enumerate() {
        let function_name = match index {
            0 => function_name_of(&document.0).replacen("build_", "populate_", 1),
            _ => function_name_of(&document.0),
        };
        if index > 0 {
            if let (Some(build), Some(root_class)) = (&transformed.build, root_class_of(types, &transformed.root)) {
                functions.insert(build, &function_name, root_class);
                has_build[index] = true;
            }
        }
        names.push(function_name);
    }

    // The document of the class, then the documents its calls reach and the public
    // documents with a build function (the entries of the loader table). A document the
    // group merged into another one that is not public is not called and not emitted, as
    // upstream removes it without compiling it.
    let mut tables: Vec<String> = Vec::new();
    let mut sources = Vec::with_capacity(documents.len());
    let mut not_eligible = Vec::new();
    let mut emitted: Vec<usize> = Vec::new();
    let loadable: Vec<usize> =
        (1..documents.len()).filter(|index| public[*index] && has_build[*index]).collect();
    let mut pending: Vec<usize> = loadable.iter().rev().copied().collect();
    pending.push(0);
    while let Some(index) = pending.pop() {
        if emitted.contains(&index) {
            continue;
        }
        emitted.push(index);
        let (document, transformed, function_name) = (&documents[index], &transformed[index], &names[index]);
        let table = transformed.namespaces.clone().ok_or_else(|| format!("{}: no namespace information", document.0))?;
        let table_index = match tables.iter().position(|known| *known == table) {
            Some(known) => known,
            None => {
                tables.push(table);
                tables.len() - 1
            }
        };
        let populate = (index == 0).then_some(class);
        let _ = types.take_unanswered();
        match emit_function(
            types,
            &transformed.root,
            &transformed.configuration,
            transformed.base_uri.as_deref(),
            &format!("XML_NAMESPACES_{table_index}"),
            function_name,
            &document.0,
            &functions,
            populate,
        ) {
            Ok(source) => sources.push(source),
            Err(reason) => not_eligible.push(format!("{}: {reason}", document.0)),
        }
        // A question the type system of the host could not answer: the document is
        // refused with it, whatever was emitted from the answer it gave instead.
        let unanswered = types.take_unanswered();
        if !unanswered.is_empty() {
            not_eligible.push(format!("{}: the type system of the host cannot answer: {}", document.0, unanswered.join("; ")));
        }
        for called in functions.called() {
            if let Some(next) = names.iter().position(|name| *name == called) {
                if !emitted.contains(&next) && !pending.contains(&next) {
                    pending.push(next);
                }
            }
        }
    }
    if !not_eligible.is_empty() {
        return Err(not_eligible.join("\n"));
    }

    let class_path = class.rust_path().ok_or_else(|| format!("no public Rust path is recorded for {}", class.full_name()))?;
    let mut source = String::new();
    source.push_str("// @generated by the Rust emitter of ferroui-markup-xaml-loader (rust_emitter::generate_class_file).\n");
    source.push_str("// Do not edit: regenerate it (see the header of the module that includes this file).\n");
    source.push_str(&format!(
        "// The document of `{}` and the {} it calls, of a group of {} documents.\n",
        class.full_name(),
        sources.len() - 1,
        documents.len()
    ));
    source.push('\n');
    source.push_str("#![allow(dead_code, unused_imports)]\n");
    source.push('\n');
    source.push_str("use ::ferroui_markup_xaml::xaml_il::runtime::compiled as rt;\n");
    for (index, table) in tables.iter().enumerate() {
        source.push('\n');
        source.push_str("/// The XML namespaces of documents of this file, as the compiler resolved them.\n");
        source.push_str(&format!("const XML_NAMESPACES_{index}: rt::XmlNamespaceTable = {table};\n"));
    }
    source.push('\n');
    source.push_str(&format!(
        "/// Populates `root` from the document of the class (`{}`), with a root service provider\n",
        names[0]
    ));
    source.push_str("/// over `service_provider` (`XamlIlRuntimeHelpers.CreateRootServiceProviderV3`), as the\n");
    source.push_str("/// run-time loader populates it.\n");
    source.push_str("pub fn populate(\n");
    source.push_str("    service_provider: ::core::option::Option<::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>,\n");
    source.push_str(&format!("    root: &::ferroui_base::Ref<::{}>,\n", class_path.trim_start_matches("::")));
    source.push_str(") -> ::core::result::Result<(), ::ferroui_markup_xaml::XamlLoadException> {\n");
    source.push_str("    let service_provider =\n");
    source.push_str("        ::ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v3(service_provider);\n");
    source.push_str(&format!("    {}(::core::option::Option::Some(service_provider), root)\n", names[0]));
    source.push_str("}\n");
    for function in &sources {
        source.push('\n');
        source.push_str(function);
    }
    source.push('\n');
    source.push_str(&loader_table(documents, &names, &loadable, class_path, constructor.as_ref()));
    // The file is a module of the crate of the class: that crate is `crate` in it.
    let own_crate = class_path.trim_start_matches("::").split("::").next().unwrap_or_default();
    let source = source.replace(&format!("::{own_crate}::"), "crate::");

    // The emitted documents, as other crates see them: the class, and every other
    // document with its build function.
    let mut exported = Vec::with_capacity(emitted.len());
    for index in std::iter::once(0).chain((1..documents.len()).filter(|index| emitted.contains(index))) {
        let root_type = root_type_name(&transformed[index].root).unwrap_or_default();
        let uri = documents[index].2.clone().unwrap_or_default();
        exported.push(match index {
            0 => DocumentModel {
                uri,
                root_type,
                class_rust_path: Some(format!("::{}", class_path.trim_start_matches("::"))),
                build_path: None,
                populate_path: Some(format!("{module_path}::populate")),
                public: true,
            },
            _ => DocumentModel {
                uri,
                root_type,
                class_rust_path: None,
                build_path: Some(format!("{module_path}::{}", names[index])),
                populate_path: None,
                public: public[index],
            },
        });
    }
    Ok(ClassFile {
        source,
        warnings,
        assembly_name: options.local_assembly.clone().unwrap_or_default(),
        crate_name: own_crate.to_string(),
        documents: exported,
    })
}

/// The generated file of the documents of a class ([`generate_class_file`]).
pub struct ClassFile {
    /// The complete text of the file.
    pub source: String,
    /// The warnings of the compilation, with upstream's texts (`XamlLoaderUnreachable`).
    pub warnings: Vec<String>,
    assembly_name: String,
    crate_name: String,
    documents: Vec<DocumentModel>,
}

impl ClassFile {
    /// The `.xamlmeta` of the file (docs/porting/xaml.md, 9.7.3): its compiled documents,
    /// for crates that include them. `dependencies` are the `.xamlmeta` files of the
    /// crates the documents include documents of, relative to the file the metadata is
    /// written to.
    pub fn metadata(&self, dependencies: &[&str]) -> XamlMetadata {
        XamlMetadata {
            name: self.assembly_name.clone(),
            crate_name: self.crate_name.clone(),
            documents: self.documents.clone(),
            dependencies: dependencies.iter().map(|path| path.to_string()).collect(),
        }
    }
}

/// `try_load` of a class file: one entry per public document, in the order of the
/// group, as `XamlCompilerTaskExecutor` writes `!XamlLoader.TryLoad`.
fn loader_table(
    documents: &[(String, String, Option<String>)],
    names: &[String],
    loadable: &[usize],
    class_path: &str,
    constructor: Option<&Constructor>,
) -> String {
    let provider = "::ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v3(service_provider.cloned())";
    let class_path = class_path.trim_start_matches("::");
    let mut source = String::new();
    source.push_str("/// The loader of the compiled documents of this file (`!XamlLoader.TryLoad`): the object of the\n");
    source.push_str("/// public document with the URI `uri` (compared as upstream's `OrdinalIgnoreCase`,\n");
    source.push_str("/// `rt::uri_equals`); `Ok(None)` if this file has no such document, the load error of the build\n");
    source.push_str("/// if it fails.\n");
    source.push_str("pub fn try_load(\n");
    source.push_str("    service_provider: ::core::option::Option<&::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>,\n");
    source.push_str("    uri: &str,\n");
    source.push_str(") -> ::core::result::Result<::core::option::Option<::ferroui_base::BoxedValue>, ::ferroui_markup_xaml::XamlLoadException> {\n");
    // The class has an entry when it has a constructor the table can call.
    for index in constructor.iter().map(|_| 0).chain(loadable.iter().copied()) {
        let uri = documents[index].2.clone().unwrap_or_default();
        source.push_str(&format!("    if rt::uri_equals(uri, {}, \"\") {{\n", rust_string_literal(&uri)));
        let value = match (index, constructor) {
            (0, Some(Constructor::Parameterless(function))) => format!("::{class_path}::{function}()"),
            (0, Some(Constructor::ServiceProvider(function))) => {
                format!("::{class_path}::{function}(::core::option::Option::Some({provider}))")
            }
            _ => format!("{}(::core::option::Option::Some({provider}))?", names[index]),
        };
        source.push_str(&format!(
            "        return ::core::result::Result::Ok(::ferroui_base::metadata::into_markup_value({value}));\n"
        ));
        source.push_str("    }\n");
    }
    source.push_str("    ::core::result::Result::Ok(::core::option::Option::None)\n");
    source.push_str("}\n");
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not from upstream: `x:ClassModifier` as `XamlCompilerTaskExecutor` reads it.
    #[test]
    fn class_modifier_decides_whether_a_document_is_public() {
        let document = |modifier: &str| {
            format!("<ResourceDictionary xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'{modifier}/>")
        };
        assert_eq!(class_modifier_public(&document("")), Ok(None));
        assert_eq!(class_modifier_public(&document(" x:ClassModifier='Public'")), Ok(Some(true)));
        assert_eq!(class_modifier_public(&document(" x:ClassModifier=' public '")), Ok(Some(true)));
        assert_eq!(class_modifier_public(&document(" x:ClassModifier='NotPublic'")), Ok(Some(false)));
        assert_eq!(class_modifier_public(&document(" x:ClassModifier='internal'")), Ok(Some(false)));
        assert_eq!(
            class_modifier_public(&document(" x:ClassModifier='private'")),
            Err("Invalid value for x:ClassModifier. Expected value are: Public, NotPublic (internal).".to_string())
        );
    }

    /// Not from upstream: the class and the group of a class document are read from the
    /// documents of its crate as the run-time loader finds them among the assets: the
    /// document with the documents it includes, directly or not, each once, in the order
    /// the includes are met; a relative source is resolved against the including document,
    /// and a document of another assembly is not of the group.
    #[test]
    fn class_and_group_of_a_class_document_are_read_from_the_documents() {
        let namespaces = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";
        let theme = format!(
            "<Styles {namespaces} x:Class=' Fixture.Theme '>\
             <StyleInclude Source='/Controls/A.xaml'/>\
             <StyleInclude Source='ferres://Other/Unused.xaml'/>\
             <StyleInclude Source='Controls/B.xaml'/>\
             </Styles>"
        );
        let a = format!("<Styles {namespaces}><StyleInclude Source='B.xaml'/><StyleInclude Source='/Theme.xaml'/></Styles>");
        let plain = format!("<Styles {namespaces}/>");
        assert_eq!(class_of_document(&theme), Ok(Some("Fixture.Theme".to_string())));
        assert_eq!(class_of_document(&plain), Ok(None));

        let documents = [("Unused.xaml", plain.as_str()), ("Controls/B.xaml", plain.as_str()), ("Theme.xaml", theme.as_str()), ("Controls/A.xaml", a.as_str())];
        let group = class_document_group("ferres://Fixture/", &documents, "Theme.xaml").expect("the group");
        let found: Vec<(&str, Option<&str>)> = group.iter().map(|(name, _, uri)| (name.as_str(), uri.as_deref())).collect();
        assert_eq!(
            found,
            [
                ("Theme.xaml", Some("ferres://fixture/Theme.xaml")),
                ("Controls/A.xaml", Some("ferres://fixture/Controls/A.xaml")),
                ("Controls/B.xaml", Some("ferres://fixture/Controls/B.xaml")),
            ]
        );
        assert_eq!(group[0].1, theme);
        assert_eq!(
            class_document_group("ferres://Fixture/", &documents, "Missing.xaml"),
            Err("the document `Missing.xaml` of the class is not a document of the group".to_string())
        );
    }
}
