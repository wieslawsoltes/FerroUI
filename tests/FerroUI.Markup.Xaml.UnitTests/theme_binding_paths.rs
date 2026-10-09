//! An audit, not a port: every binding path of the documents of the two themes, of the
//! colour picker and of the dialogs is resolved against the declared members of the types it
//! is read on, wherever the type is statically known.
//!
//! A binding of a theme is a reflection binding: its path is text, and a member the type
//! does not declare is found out when the binding runs, as a report in the log and a value
//! that never arrives (the indicators of the indeterminate progress bar had no size that
//! way). The compiler resolves the path of a compiled binding when it transforms the
//! document: against the data type in scope (`x:DataType`, or the type a data template
//! inherits from the items of its control), the target type of the control template for
//! `RelativeSource TemplatedParent`, the type `$parent[Type]` names, the type of the element
//! `#name` names in the name scope, the type of the element for `$self` (in a setter, the
//! type its selector targets), and the type of a `Source`. So the audit transforms every
//! document with compiled bindings as the default, with the transformer that resolves the
//! paths wrapped: a path that does not resolve is recorded and the transform goes on.
//!
//! The types are the ones the crates declare in their sources: the build-time type system
//! over the scanned models of the base crate, the controls, the XAML runtime library, the
//! dialogs, the two themes and the colour picker (`ferroui-build`), which is what the builds
//! of the themes, of the colour picker and of the dialogs compile their documents against.
//! Nothing is registered with the process, so the audit leaves the run-time type system of
//! the other tests as it is.
//!
//! What is not statically known is left out and counted: a path read on the data context
//! where no data type is in scope, and a segment read on a value of type `object`. Every
//! other path that does not resolve is a finding: a member the type does not declare. The
//! findings that are no defect (the static type is a base of the type the value has when the
//! binding runs) are listed in [`ACCEPTED`], each with its reason.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use ferroui_markup_xaml_loader::back_end::adapt_type_mappings;
use ferroui_markup_xaml_loader::compiler_extensions::transformers::{
    FerroXamlIlBindingPathTransformer, FerroXamlIlWellKnownTypesExtensions,
};
use ferroui_markup_xaml_loader::compiler_extensions::{
    FerroXamlDiagnosticCodes, FerroXamlIlCompiler, FerroXamlIlCompilerConfiguration, FerroXamlIlLanguage,
};
use ferroui_build::model::AssemblyModel;
use ferroui_build::scanner::{scan_crate, ScanOptions};
use ferroui_build::type_system::ModelTypeSystem;
use xamlx::ast::{IXamlAstNode, IXamlAstValueNode, XamlAstConstructableObjectNode, XamlAstExtensions, XamlAstNodeExtensions};
use xamlx::diagnostics::XamlDiagnosticSeverity;
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer, XamlDiagnosticsHandler};
use xamlx::type_system::{IXamlAssembly, IXamlType, IXamlTypeSystem};

/// The message of the compiler for a path on the data context where no data type is in scope.
const NO_DATA_TYPE: &str = "without an explicit x:DataType directive";

/// A path that did not resolve: where, and the message of the compiler.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Unresolved {
    document: String,
    line: i32,
    message: String,
}

#[derive(Default)]
struct Audit {
    /// The document being transformed.
    document: RefCell<String>,
    /// The bindings whose paths resolved.
    resolved: RefCell<usize>,
    /// The paths on a data context of unknown type.
    no_data_type: RefCell<usize>,
    /// The paths with a segment read on a value of type `object`.
    untyped: RefCell<Vec<Unresolved>>,
    findings: RefCell<Vec<Unresolved>>,
}

/// The transformer that resolves the paths of the compiled bindings, with a failure to
/// resolve a path recorded instead of ending the transform of the document.
struct AuditedBindingPathTransformer(Rc<Audit>);

impl IXamlAstTransformer for AuditedBindingPathTransformer {
    fn transform(&self, context: &AstTransformationContext, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let is_binding = match node.cast::<XamlAstConstructableObjectNode>() {
            Some(object) => {
                let types = context.try_get_ferro_types()?;
                object.type_().get_clr_type()?.equals(&*types.compiled_binding_extension)
            }
            None => false,
        };
        if !is_binding {
            return FerroXamlIlBindingPathTransformer.transform(context, node);
        }
        let audit = &self.0;
        match FerroXamlIlBindingPathTransformer.transform(context, node.clone()) {
            Ok(node) => {
                *audit.resolved.borrow_mut() += 1;
                Ok(node)
            }
            Err(error) => {
                let message = error.message();
                if message.contains(NO_DATA_TYPE) {
                    *audit.no_data_type.borrow_mut() += 1;
                    return Ok(node);
                }
                let unresolved = Unresolved {
                    document: audit.document.borrow().clone(),
                    line: error.line_number().unwrap_or(0),
                    message: without_position(&message),
                };
                if unresolved.message.ends_with("on type 'System.Object'.") {
                    audit.untyped.borrow_mut().push(unresolved);
                } else {
                    audit.findings.borrow_mut().push(unresolved);
                }
                Ok(node)
            }
        }
    }

    fn transformer_name(&self) -> String {
        String::from("FerroXamlIlBindingPathTransformer")
    }
}

/// The message of an error without the position the compiler appends to it.
fn without_position(message: &str) -> String {
    let message = message.split(" (line ").next().unwrap_or(message);
    let message = message.split(" Line ").next().unwrap_or(message);
    message.trim().to_string()
}

/// The type system of the audit: the build-time one over the models of the crates whose
/// documents are audited and of the crates they are built on, each scanned from its sources.
fn audited_type_system() -> Rc<dyn IXamlTypeSystem> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("src");
    let scan = |name: &str, directory: &Path, dependencies: &[&AssemblyModel]| -> AssemblyModel {
        let options = ScanOptions::new(name, directory.join("lib.rs"))
            .with_dependencies(dependencies.iter().map(|model| (*model).clone()).collect());
        let model = scan_crate(&options).model;
        assert!(!model.types.is_empty(), "the scan of {name} has no types");
        model
    };
    let base = scan("ferroui_base", &source.join("FerroUI.Base"), &[]);
    let controls = scan("ferroui_controls", &source.join("FerroUI.Controls"), &[&base]);
    let markup_xaml = scan("ferroui_markup_xaml", &source.join("Markup").join("FerroUI.Markup.Xaml"), &[&base, &controls]);
    let dialogs = scan("ferroui_dialogs", &source.join("FerroUI.Dialogs"), &[&base, &controls, &markup_xaml]);
    let simple = scan("ferroui_themes_simple", &source.join("FerroUI.Themes.Simple"), &[&base, &controls, &markup_xaml, &dialogs]);
    let fluent = scan("ferroui_themes_fluent", &source.join("FerroUI.Themes.Fluent"), &[&base, &controls, &markup_xaml, &dialogs]);
    let color_picker =
        scan("ferroui_controls_color_picker", &source.join("FerroUI.Controls.ColorPicker"), &[&base, &controls, &markup_xaml]);
    ModelTypeSystem::new(vec![base, controls, markup_xaml, dialogs, simple, fluent, color_picker]).as_type_system()
}

/// A compiler over the type system of the audit that compiles bindings by default and audits
/// their paths; `assembly` is the assembly of the documents.
fn auditing_compiler(type_system: &Rc<dyn IXamlTypeSystem>, assembly: &str, audit: &Rc<Audit>) -> FerroXamlIlCompiler {
    let build = || -> XamlResult<FerroXamlIlCompiler> {
        let (mut mappings, _emit_mappings) = FerroXamlIlLanguage::configure(type_system)?;
        adapt_type_mappings(&mut mappings);
        let local: Option<Rc<dyn IXamlAssembly>> = type_system.assemblies().into_iter().find(|a| a.name() == assembly);
        let diagnostics = XamlDiagnosticsHandler {
            // A warning of a document is not what is audited.
            handle_diagnostic: Some(Box::new(|diagnostic| match diagnostic.severity {
                XamlDiagnosticSeverity::Error | XamlDiagnosticSeverity::Fatal => diagnostic.severity,
                _ => XamlDiagnosticSeverity::None,
            })),
            code_mappings: Box::new(FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro),
            ..XamlDiagnosticsHandler::default()
        };
        let configuration = FerroXamlIlCompilerConfiguration::new(
            type_system.clone(),
            local,
            mappings,
            None,
            Some(FerroXamlIlLanguage::value_converter()),
            None,
            Some(diagnostics),
        )?;
        let mut compiler = FerroXamlIlCompiler::new(configuration.as_transformer_configuration().clone())?;
        compiler.set_default_compile_bindings(true);
        let index = compiler
            .transformers
            .iter()
            .position(|transformer| transformer.transformer_name() == "FerroXamlIlBindingPathTransformer")
            .expect("the pipeline resolves the paths of compiled bindings");
        compiler.transformers[index] = Box::new(AuditedBindingPathTransformer(audit.clone()));
        Ok(compiler)
    };
    build().unwrap_or_else(|error| panic!("the compiler of the audit: {}", error.message()))
}

/// Transforms `xaml` with the audit; a document that does not transform for another reason
/// than a binding path is an error of the audit.
fn audit_document(compiler: &FerroXamlIlCompiler, type_system: &Rc<dyn IXamlTypeSystem>, audit: &Audit, name: &str, xaml: &str) {
    *audit.document.borrow_mut() = name.to_string();
    // The class of the root instance of a document with `x:Class`.
    let class: Option<Rc<dyn IXamlType>> = xaml
        .split("x:Class=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .and_then(|class| type_system.find_type(class));
    let transformed = compiler.parse(xaml, class).and_then(|mut document| {
        document.document = Some(name.to_string());
        compiler.transform(&mut document)
    });
    if let Err(error) = transformed {
        panic!("{name} is not transformed: {}", error.message());
    }
}

fn documents_under(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            documents_under(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "xaml") {
            found.push(path);
        }
    }
}

/// The crates whose documents are audited: the directory below `src` and the assembly.
const CRATES: &[(&str, &str)] = &[
    ("FerroUI.Themes.Fluent", "FerroUI.Themes.Fluent"),
    ("FerroUI.Themes.Simple", "FerroUI.Themes.Simple"),
    ("FerroUI.Controls.ColorPicker", "FerroUI.Controls.ColorPicker"),
    ("FerroUI.Dialogs", "FerroUI.Dialogs"),
];

/// The findings that are no defect: the document (the crate and the path below it), the
/// message of the compiler, and why the path resolves when the binding runs.
const ACCEPTED: &[(&str, &str, &str)] = &[];

/// Every binding path of the documents resolves on the declared members of the types it is
/// read on, where the type is statically known.
#[test]
fn the_binding_paths_of_the_themes_resolve_on_the_declared_members() {
    let type_system = audited_type_system();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("src");

    let audit = Rc::new(Audit::default());
    let mut documents = 0;
    for (directory, assembly) in CRATES {
        let root = source.join(directory);
        let compiler = auditing_compiler(&type_system, assembly, &audit);
        // The documents a theme leaves out (`Controls/excluded.txt`: they name types that
        // are not ported).
        let excluded_list = std::fs::read_to_string(root.join("Controls/excluded.txt")).unwrap_or_default();
        let excluded: Vec<String> = excluded_list
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .filter_map(|line| line.split('|').next())
            .map(|file| format!("Controls/{}", file.trim()))
            .filter(|file| file != "Controls/")
            .collect();
        let mut paths = Vec::new();
        documents_under(&root, &mut paths);
        assert!(!paths.is_empty(), "{directory} has no documents");
        for path in paths {
            let name = path.strip_prefix(&root).unwrap_or(&path).display().to_string().replace('\\', "/");
            if excluded.contains(&name) {
                continue;
            }
            let xaml = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            audit_document(&compiler, &type_system, &audit, &format!("{directory}/{name}"), &xaml);
            documents += 1;
        }
    }

    let findings = audit.findings.borrow();
    let untyped = audit.untyped.borrow();
    println!(
        "{documents} documents: {} bindings with a resolved path, {} on a data context of unknown type, {} with a segment on a value of type object, {} that do not resolve",
        audit.resolved.borrow(),
        audit.no_data_type.borrow(),
        untyped.len(),
        findings.len()
    );
    for unresolved in untyped.iter() {
        println!("on object  {}({}): {}", unresolved.document, unresolved.line, unresolved.message);
    }
    // The audit resolves something: the themes have hundreds of bindings with a typed start.
    assert!(*audit.resolved.borrow() > 100, "{} paths were resolved", audit.resolved.borrow());

    let mut by_finding: BTreeMap<(String, String), Vec<i32>> = BTreeMap::new();
    for unresolved in findings.iter() {
        by_finding.entry((unresolved.document.clone(), unresolved.message.clone())).or_default().push(unresolved.line);
    }
    let accepted: Vec<(String, String)> =
        ACCEPTED.iter().map(|(document, message, _)| (document.to_string(), message.to_string())).collect();
    let mut differences = Vec::new();
    for ((document, message), lines) in &by_finding {
        if !accepted.contains(&(document.clone(), message.clone())) {
            differences.push(format!("{document} (line {lines:?}): {message}"));
        }
    }
    for (document, message) in &accepted {
        if !by_finding.contains_key(&(document.clone(), message.clone())) {
            differences.push(format!("accepted and no longer found: {document}: {message}"));
        }
    }
    assert!(differences.is_empty(), "binding paths that do not resolve on the declared members:\n{}", differences.join("\n"));
}

/// The audit finds what it is for: a member a type does not declare, behind each of the
/// starts a path of a theme has, and nothing where the members are declared.
#[test]
fn the_audit_reports_a_member_a_type_does_not_declare() {
    let type_system = audited_type_system();
    let audit = Rc::new(Audit::default());
    let compiler = auditing_compiler(&type_system, "FerroUI.Themes.Fluent", &audit);
    let document = |content: &str| {
        format!(
            "<ResourceDictionary xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' xmlns:sys='using:System'>
               <ControlTheme x:Key='{{x:Type ProgressBar}}' TargetType='ProgressBar'>
                 <Setter Property='Template'>
                   <ControlTemplate TargetType='ProgressBar'>
                     <Panel>
                       <Border Name='Named'/>
                       {content}
                     </Panel>
                   </ControlTemplate>
                 </Setter>
               </ControlTheme>
             </ResourceDictionary>"
        )
    };

    let declared = [
        "<Border Width='{Binding $parent[ProgressBar].TemplateSettings.ContainerWidth}'/>",
        "<Border Width='{Binding TemplateSettings.Container2Width, RelativeSource={RelativeSource TemplatedParent}}'/>",
        "<Border Width='{Binding #Named.Bounds.Width}'/>",
        "<Border Width='{Binding $self.Bounds.Height}'/>",
        "<TextBlock Text='{Binding Source={x:Static sys:DateTime.Today}, Path=Day}'/>",
    ];
    for content in declared {
        audit_document(&compiler, &type_system, &audit, "declared", &document(content));
    }
    assert_eq!(*audit.resolved.borrow(), declared.len());
    assert!(audit.findings.borrow().is_empty(), "{:?}", audit.findings.borrow());

    let missing = [
        ("<Border Width='{Binding $parent[ProgressBar].TemplateSettings.NoSuchMember}'/>", "ProgressBarTemplateSettings"),
        ("<Border Width='{Binding NoSuchMember, RelativeSource={RelativeSource TemplatedParent}}'/>", "ProgressBar"),
        ("<Border Width='{Binding #Named.NoSuchMember}'/>", "Border"),
        ("<Border Width='{Binding $self.Bounds.NoSuchMember}'/>", "Rect"),
        ("<TextBlock Text='{Binding Source={x:Static sys:DateTime.Today}, Path=NoSuchMember}'/>", "DateTime"),
    ];
    for (index, (content, type_)) in missing.iter().enumerate() {
        audit_document(&compiler, &type_system, &audit, "missing", &document(content));
        let findings = audit.findings.borrow();
        assert_eq!(findings.len(), index + 1, "{content}: {findings:?}");
        let message = &findings[index].message;
        assert!(message.contains("'NoSuchMember'") && message.contains(type_), "{content}: {message}");
    }

    // A path on the data context where no data type is in scope is not statically known.
    audit_document(&compiler, &type_system, &audit, "data context", &document("<TextBlock Text='{Binding NoSuchMember}'/>"));
    assert_eq!(*audit.no_data_type.borrow(), 1);
    assert_eq!(audit.findings.borrow().len(), missing.len());
}
