//! The transform of the corpus against the build-time type system
//! (docs/porting/xaml.md, 9.5.10 and 9.5.11): every document of the corpus is
//! parsed and transformed twice with the one sequence of the emitter
//! (`rust_emitter::transform_group`), against the run-time type system and
//! against `ModelTypeSystem` over the scanned models of the base crate, the
//! controls, the XAML runtime library and this crate (the crate whose
//! documents are compiled, scanned without its test code as its build script
//! would scan it), and the transformed trees are compared as text.
//!
//! This is the transform half of the emitter on the model: a document whose
//! tree is the same has resolved every type and member to the same names
//! through both type systems. What the emitter then asks of the type system
//! beyond the contracts of the compiler (`EmitTypes`) is not exercised here.
//!
//! The test prints, for every document that does not come out the same, why
//! (`RUST_TEST_NOCAPTURE=1` shows it), and fails unless every document of the
//! corpus comes out the same ([`IDENTICAL`]).

use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

use ferroui_build::model::AssemblyModel;
use ferroui_build::scanner::{scan_crate, ScanOptions, Severity};
use ferroui_build::type_system::ModelTypeSystem;
use ferroui_markup_xaml_loader::compiler_extensions::IXamlCompileTimeValueParser;
use ferroui_markup_xaml_loader::rust_emitter::{transform_group, DocumentSource, EmitterHost, TransformOptions};
use ferroui_markup_xaml_loader::testing::objects::dump_tree;
use xamlx::type_system::IXamlTypeSystem;

use super::corpus::DOCUMENTS;

/// The number of documents of the corpus whose transformed tree is the same against
/// both type systems: all of them.
///
/// The first measurement was none (the configuration of the language did not find a
/// constructor of `XamlSourceInfo` on the model: the one declaration of the XAML runtime
/// library the scanner did not read, a closure that states its return type); with that
/// crate in the drift test of the type systems (`type_system_drift`) and this crate
/// scanned as the crate that is compiled, every document agrees.
const IDENTICAL: usize = 121;

fn transformed(
    type_system: &Rc<dyn IXamlTypeSystem>,
    parsers: &[Rc<dyn IXamlCompileTimeValueParser>],
    name: &str,
    xaml: &str,
) -> Result<String, String> {
    let sources = [DocumentSource { name, xaml, base_uri: Some(format!("ferres://Tests/{name}")), root_type: None }];
    let documents = transform_group(type_system.clone(), &sources, &TransformOptions::default(), parsers).map_err(|error| error.message())?;
    documents.first().map(|document| dump_tree(&document.root)).ok_or_else(|| "the document was not transformed".to_string())
}

/// The models the documents of this crate are compiled against, as the build scripts of
/// the crates would export them: the base crate, the controls on it, the XAML runtime
/// library on both, and this crate (the crate that is compiled, without its test code) on
/// the three. A scan that does not read a declaration fails the test that asks.
pub(super) fn scanned_models() -> Vec<AssemblyModel> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("src");
    let base = scan_crate(&ScanOptions::new("ferroui_base", source.join("FerroUI.Base").join("lib.rs")));
    let controls =
        scan_crate(&ScanOptions::new("ferroui_controls", source.join("FerroUI.Controls").join("lib.rs")).with_dependencies(vec![base.model.clone()]));
    let markup_xaml = scan_crate(
        &ScanOptions::new("ferroui_markup_xaml", source.join("Markup").join("FerroUI.Markup.Xaml").join("lib.rs"))
            .with_dependencies(vec![base.model.clone(), controls.model.clone()]),
    );
    let own = scan_crate(
        &ScanOptions::new("ferroui_markup_xaml_tests", Path::new(env!("CARGO_MANIFEST_DIR")).join("lib.rs"))
            .with_dependencies(vec![base.model.clone(), controls.model.clone(), markup_xaml.model.clone()]),
    );
    for scan in [&markup_xaml, &own] {
        println!("==== the scan of {} ====\n{}\n", scan.model.crate_name, scan.summary());
        let not_read: Vec<String> = scan.diagnostics_of(Severity::Error).map(|diagnostic| diagnostic.to_string()).collect();
        for diagnostic in &not_read {
            println!("not read: {diagnostic}");
        }
        assert!(not_read.is_empty(), "the scan of {} does not read {} declarations", scan.model.crate_name, not_read.len());
        assert!(!scan.model.types.is_empty(), "the scan of {} has no types", scan.model.crate_name);
    }
    vec![base.model, controls.model, markup_xaml.model, own.model]
}

/// Not from upstream: the transform of the emitter against the build-time type system,
/// measured over the corpus against the transform against the run-time type system.
#[test]
fn corpus_is_transformed_against_the_model_type_system() {
    crate::register_types();
    ferroui_controls::register_types();

    let model_system: Rc<dyn IXamlTypeSystem> = ModelTypeSystem::new(scanned_models()).as_type_system();
    let runtime = EmitterHost::runtime(&[]).unwrap_or_else(|error| panic!("the run-time type system: {}", error.message()));

    let mut identical: Vec<&str> = Vec::new();
    let mut different: Vec<(&str, String)> = Vec::new();
    // The documents the model does not transform, by the error.
    let mut failed: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    let mut not_at_run_time: Vec<&str> = Vec::new();
    for (name, xaml) in DOCUMENTS {
        let Ok(expected) = transformed(&runtime.type_system, &runtime.parsers, name, xaml) else {
            not_at_run_time.push(name);
            continue;
        };
        match transformed(&model_system, &[], name, xaml) {
            Ok(tree) if tree == expected => identical.push(name),
            Ok(tree) => {
                let line = expected.lines().zip(tree.lines()).position(|(expected, found)| expected != found).unwrap_or(0);
                let show = |text: &str| text.lines().nth(line).unwrap_or("(the tree ends)").trim().to_string();
                different.push((name, format!("line {}: `{}` at run time, `{}` against the model", line + 1, show(&expected), show(&tree))));
            }
            Err(error) => failed.entry(error.replace(['\r', '\n'], " ")).or_default().push(name),
        }
    }

    println!(
        "the corpus against the build-time type system: {} documents; {} with the same transformed tree, {} with another tree, {} not transformed ({} kinds of error); {} not transformed at run time either",
        DOCUMENTS.len(),
        identical.len(),
        different.len(),
        failed.values().map(Vec::len).sum::<usize>(),
        failed.len(),
        not_at_run_time.len()
    );
    println!("---- another tree ({}) ----", different.len());
    for (name, first) in &different {
        println!("{name}: {first}");
    }
    println!("---- not transformed, by error ----");
    for (error, names) in &failed {
        println!("{} documents: {error}\n    {}", names.len(), names.join(", "));
    }
    assert_eq!(
        identical.len() + different.len() + failed.values().map(Vec::len).sum::<usize>() + not_at_run_time.len(),
        DOCUMENTS.len()
    );
    assert_eq!(DOCUMENTS.len(), IDENTICAL, "the corpus has {} documents: state the number that agree", DOCUMENTS.len());
    assert_eq!(
        identical.len(),
        IDENTICAL,
        "{} of the {IDENTICAL} documents of the corpus have the same transformed tree against both type systems (the others are printed above; run with --nocapture)",
        identical.len()
    );
}
