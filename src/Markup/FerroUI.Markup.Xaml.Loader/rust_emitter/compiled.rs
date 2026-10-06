//! From documents to a generated file: parse + transform (the sequence of
//! the run-time loader) + emit, and the file that holds the build functions
//! of the eligible documents with the table that registers them as the
//! compiled markup of an assembly (docs/porting/xaml.md, 9.7).
//!
//! # Interim integration
//!
//! The host of the emitter links the framework (the transform runs against
//! the run-time type system), so a build script would have to take the
//! framework as a build dependency and compile it twice. Until the source
//! scanner of section 9.5 exists, a crate keeps the generated file CHECKED
//! IN: a test calls [`generate_file`] and compares the result with the file
//! (and an ignored test rewrites it). The test crate of the XAML stack does
//! exactly that (`tests/FerroUI.Markup.Xaml.UnitTests/emitter`).

use ::ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;

use crate::FerroXamlIlRuntimeCompiler;

use super::emitter::{emit_document, emit_function, namespace_table, root_class_of, DocumentFunctions};
use super::source::{function_name_of, rust_string_literal};

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
}

/// Parses, transforms and emits the documents (`(name, xaml)`) of an assembly
/// with the configuration of the run-time loader. The documents are
/// transformed as ONE group, as upstream's build transforms the documents of
/// an assembly and the run-time loader a group of documents: the group
/// transformers see every document, so includes between them are resolved.
/// With `root_uri` (`ferres://MyApp/`) the base URI of a document is the
/// root URI followed by its name, the URI it is loaded by.
///
/// A document whose generated functions would have the name of a function
/// generated for an earlier document (names that differ only in case or in
/// characters that are not letters or digits, or a name that ends in
/// `_untyped` or `_deferred_<n>`, the names of the functions of its untyped
/// build and of its deferred content) is reported instead of becoming a duplicate definition rustc
/// rejects, and is left out of the group. A group that does not transform
/// makes every document of it not eligible, with the error; a document with
/// an unsupported node is reported with the node, and the others are not
/// affected.
pub fn compile_documents(
    documents: &[(&str, &str)],
    root_uri: Option<&str>,
    configuration: &RuntimeXamlLoaderConfiguration,
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
            compiled.push(CompiledDocument { name: name.to_string(), function_name, source: Err(reason), namespaces: None });
            continue;
        }
        group.push((compiled.len(), name, xaml, root_uri.map(|root| format!("{root}{name}"))));
        compiled.push(CompiledDocument { name: name.to_string(), function_name, source: Err(String::new()), namespaces: None });
    }
    let sources: Vec<(&str, &str, Option<String>, Option<&'static ferroui_base::TypeInfo>)> =
        group.iter().map(|(_, name, xaml, base_uri)| (*name, *xaml, base_uri.clone(), None)).collect();
    if sources.is_empty() {
        return compiled;
    }
    match FerroXamlIlRuntimeCompiler::transform_documents(&sources, configuration) {
        Ok(transformed) => {
            // The namespace information of the documents, one constant per distinct table.
            let mut tables: Vec<String> = Vec::new();
            for ((index, name, _, _), transformed) in group.iter().zip(&transformed) {
                let document = &mut compiled[*index];
                let Some(table) = namespace_table(&transformed.document) else {
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
                document.source = emit_document(
                    &transformed.root,
                    &transformed.configuration,
                    &transformed.document,
                    &constant,
                    &document.function_name,
                    name,
                )
                .map_err(|e| e.to_string());
                document.namespaces = Some((constant, table));
            }
        }
        Err(error) => {
            for (index, _, _, _) in &group {
                compiled[*index].source = Err(format!("the group of documents does not transform: {}", error.message()));
            }
        }
    }
    compiled
}

/// The generated file of an assembly: the build function of every eligible
/// document, the table `DOCUMENTS` (document name, untyped build function),
/// `try_load` (the `CompiledXamlLoader` of the assembly: the document whose
/// URI is `<root_uri><name>`, compared without regard to case as upstream's
/// `string.Equals(.., StringComparison.OrdinalIgnoreCase)` compares it; a failed build
/// is its error, as it is the error of the run-time loader) and
/// `register_compiled_xaml()`.
///
/// `root_uri` ends with `/` (`ferres://MyApp/`). The text is deterministic:
/// the same documents give the same file.
pub fn generate_file(
    assembly_name: &str,
    root_uri: &str,
    documents: &[(&str, &str)],
    configuration: &RuntimeXamlLoaderConfiguration,
) -> GeneratedFile {
    let compiled = compile_documents(documents, Some(root_uri), configuration);
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
    for document in &compiled {
        match &document.source {
            Ok(function) => {
                source.push('\n');
                source.push_str(function);
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
                report.push((document.name.clone(), None));
            }
            Err(reason) => report.push((document.name.clone(), Some(reason.clone()))),
        }
    }

    source.push('\n');
    source.push_str("/// The build function of every eligible document, by document name.\n");
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
    GeneratedFile { source, position_map, documents: report }
}

/// The position map of a generated file (docs/porting/xaml.md, 9.3.6): the
/// link from a line of generated Rust (where rustc reports an error) back to
/// the XAML node it was emitted for. `functions` are the build functions of
/// the file with their documents; the build functions of their deferred
/// content (`<function>_deferred_<n>`) belong to the same documents. Inside
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
            let build = text.strip_prefix("pub fn ").and_then(|rest| rest.strip_prefix(function.as_str()));
            let deferred = || {
                let rest = text.strip_prefix("fn ")?.strip_prefix(function.as_str())?.strip_prefix("_deferred_")?;
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

/// A group of documents with owned texts as the borrowed form
/// [`FerroXamlIlRuntimeCompiler::transform_documents`] takes.
#[allow(clippy::type_complexity)]
fn borrowed<'a>(
    documents: &'a [(String, String, Option<String>, Option<&'static ferroui_base::TypeInfo>)],
) -> Vec<(&'a str, &'a str, Option<String>, Option<&'static ferroui_base::TypeInfo>)> {
    documents.iter().map(|(name, xaml, base_uri, class)| (name.as_str(), xaml.as_str(), base_uri.clone(), *class)).collect()
}

/// Whether the items `a` and `b` of two documents would have the same name:
/// the names are equal, or one is the build function of deferred content
/// (`<function>_deferred_<n>`) of the other.
fn collides(a: &str, b: &str) -> bool {
    let is_deferred_of = |deferred: &str, function: &str| {
        deferred
            .strip_prefix(function)
            .and_then(|rest| rest.strip_prefix("_deferred_"))
            .is_some_and(|index| !index.is_empty() && index.chars().all(|c| c.is_ascii_digit()))
    };
    a == b || is_deferred_of(a, b) || is_deferred_of(b, a)
}

/// The name of the untyped build function that wraps the build function `function_name`.
fn untyped_function_name(function_name: &str) -> String {
    format!("{function_name}_untyped")
}

/// The transformed tree of one document as text (the AST dump of
/// [`crate::testing::objects::dump_tree`]): what the emitter walks, for diagnosing
/// why a document is not eligible. With the `testing` feature.
#[cfg(any(test, feature = "testing"))]
pub fn transformed_tree(name: &str, xaml: &str, configuration: &RuntimeXamlLoaderConfiguration) -> Result<String, String> {
    FerroXamlIlRuntimeCompiler::transform_document(xaml, name, None, configuration)
        .map(|transformed| crate::testing::objects::dump_tree(&transformed.root))
        .map_err(|error| error.message())
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
    let transformed = FerroXamlIlRuntimeCompiler::transform_documents(&borrowed(&documents), &configuration)
        .map_err(|error| error.message())?;
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
/// call. The file is a module of the crate of the class (that crate is
/// named `crate` in it). `Err` lists the documents that are not eligible,
/// with the reasons: a class is compiled whole or not at all.
pub fn generate_class_file(class: &'static ferroui_base::TypeInfo) -> Result<String, String> {
    let uri = crate::FerroRuntimeXamlLoader::class_document(class)
        .ok_or_else(|| format!("no document is registered for {}", class.full_name()))?;
    let group = crate::FerroRuntimeXamlLoader::document_group(&uri, &class.full_name()).map_err(|e| e.message().to_string())?;
    let name_of = |uri: &ferroui_base::utilities::Uri| uri.absolute_path().trim_start_matches('/').to_string();
    let mut documents = vec![(name_of(&group.uri), group.text.clone(), Some(group.uri.to_string()), Some(class))];
    for (uri, text) in &group.included {
        documents.push((name_of(uri), text.clone(), Some(uri.to_string()), None));
    }
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = group.assembly;
    let transformed = FerroXamlIlRuntimeCompiler::transform_documents(&borrowed(&documents), &configuration)
        .map_err(|error| format!("the group does not transform: {}", error.message()))?;

    // The functions of the documents, by their build methods.
    let mut functions = DocumentFunctions::default();
    let mut names = Vec::with_capacity(documents.len());
    for (index, (document, transformed)) in documents.iter().zip(&transformed).enumerate() {
        let function_name = match index {
            0 => function_name_of(&document.0).replacen("build_", "populate_", 1),
            _ => function_name_of(&document.0),
        };
        if index > 0 {
            if let (Some(build), Some(root_class)) = (&transformed.build, root_class_of(&transformed.root)) {
                functions.insert(build, &function_name, root_class);
            }
        }
        names.push(function_name);
    }

    // The document of the class, then the documents its calls reach (a document the group
    // merged into another one is not called and not emitted).
    let mut tables: Vec<String> = Vec::new();
    let mut sources = Vec::with_capacity(documents.len());
    let mut not_eligible = Vec::new();
    let mut emitted: Vec<usize> = Vec::new();
    let mut pending = vec![0usize];
    while let Some(index) = pending.pop() {
        if emitted.contains(&index) {
            continue;
        }
        emitted.push(index);
        let (document, transformed, function_name) = (&documents[index], &transformed[index], &names[index]);
        let table = namespace_table(&transformed.document).ok_or_else(|| format!("{}: no namespace information", document.0))?;
        let table_index = match tables.iter().position(|known| *known == table) {
            Some(known) => known,
            None => {
                tables.push(table);
                tables.len() - 1
            }
        };
        let populate = (index == 0).then_some(class);
        match emit_function(
            &transformed.root,
            &transformed.configuration,
            &transformed.document,
            &format!("XML_NAMESPACES_{table_index}"),
            function_name,
            &document.0,
            &functions,
            populate,
        ) {
            Ok(source) => sources.push(source),
            Err(reason) => not_eligible.push(format!("{}: {reason}", document.0)),
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
    // The file is a module of the crate of the class: that crate is `crate` in it.
    let own_crate = class_path.trim_start_matches("::").split("::").next().unwrap_or_default();
    Ok(source.replace(&format!("::{own_crate}::"), "crate::"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(compiled: &[CompiledDocument], name: &str) -> Option<String> {
        compiled.iter().find(|document| document.name == name).and_then(|document| document.source.clone().err())
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
        let compiled = compile_documents(&documents, None, &RuntimeXamlLoaderConfiguration::new());
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
