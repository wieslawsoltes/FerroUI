//! The emitter against the build-time type system, compared with the emitter
//! against the run-time type system (docs/porting/xaml.md, 9.5.12): the
//! corpus is compiled twice as one group with the one emitter
//! (`rust_emitter::compile_documents_with`), against the run-time type system
//! with what it states for the emitter (`RuntimeEmitTypes`, the registries of
//! the process) and against `ModelTypeSystem` over the scanned models with
//! what the models state (`ModelEmitTypes`), and the generated Rust is
//! compared byte for byte, document by document.
//!
//! A document comes out one of three ways against the models:
//!
//! - **the same**: its function, its namespace table, its root type and its
//!   visibility are the ones of the run-time path;
//! - **refused**: the models cannot answer a question the emitter asked for it
//!   (what only the process decides: `EmitTypes::take_unanswered`); it is not
//!   emitted, and the reason names the types. The refused documents are the
//!   asserted list [`REFUSED`], which is empty: every document of the corpus
//!   is emitted the same;
//! - **different**: the models answered and the emitter wrote other code. This
//!   fails the test: it is a declaration the scanner misreads or a rule of
//!   `ModelEmitTypes` that is not the rule of the run-time side.
//!
//! The second test shows the refusal: with one registration of a cast marked as
//! not read in the model of a crate, the documents whose code depends on a cast
//! that does not exist ([`ASK_FOR_A_CAST_THAT_IS_NOT_REGISTERED`]) are refused
//! with the two types named, and every other document is still the same.
//!
//! Not from upstream: upstream has one type system per back end.
//!
//! `cargo test -p ferroui-markup-xaml-tests --lib emitter::model_emit -- --nocapture`

use std::collections::BTreeMap;

use ferroui_build::type_system::{ModelEmitTypes, ModelTypeSystem};
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_build::model::AssemblyModel;
use ferroui_markup_xaml_loader::rust_emitter::{
    compile_documents, compile_documents_with, generate_file, generate_file_with, CompiledDocument, EmitterHost, TransformOptions,
};

use super::corpus::DOCUMENTS;
use super::generated;
use super::model_transform::scanned_models;

/// The documents of the corpus the build-time type system refuses, each with the start
/// of the question it cannot answer. Every other document is emitted the same.
const REFUSED: &[(&str, &str)] = &[];

/// The documents of the corpus whose code depends on a value of one Rust type not being a
/// value of another (the instance of a member is then converted at run time,
/// `rt::argument` or `rt::instance`): what the models answer only when every registration
/// of a cast is read.
const ASK_FOR_A_CAST_THAT_IS_NOT_REGISTERED: &[&str] = &[
    "control_theme_resources.xaml",
    "control_theme_template.xaml",
    "dynamic_resource_local.xaml",
    "flags_value.xaml",
    "resources_many.xaml",
    "static_resource_element.xaml",
    "static_resource_local.xaml",
    "style_resources.xaml",
    "text_block_inlines.xaml",
    "text_block_text_content.xaml",
];

fn first_difference(expected: &str, found: &str) -> String {
    let line = expected.lines().zip(found.lines()).position(|(expected, found)| expected != found).unwrap_or_else(|| expected.lines().count().min(found.lines().count()));
    let show = |text: &str| text.lines().nth(line).unwrap_or("(the text ends)").trim().to_string();
    format!("line {}:\n    at run time:       {}\n    against the model: {}", line + 1, show(expected), show(found))
}

/// What of a compiled document is compared, besides its source.
fn shape(document: &CompiledDocument) -> (&str, &str, &Option<(String, String)>, &Option<String>, bool) {
    (&document.name, &document.function_name, &document.namespaces, &document.root_type, document.public)
}

/// The outcome of the corpus against `models`: the documents emitted the same, the ones
/// emitted differently (with the first difference) and the ones refused (with the reason).
struct Outcome {
    same: Vec<String>,
    different: Vec<(String, String)>,
    refused: BTreeMap<String, String>,
    not_eligible_at_run_time: Vec<String>,
    /// The whole generated file is the one of the run-time path.
    same_file: bool,
}

fn emit_against(models: Vec<AssemblyModel>) -> Outcome {
    let configuration = RuntimeXamlLoaderConfiguration::new();
    let expected = compile_documents(DOCUMENTS, Some(generated::ROOT_URI), &configuration, &[]);

    let system = ModelTypeSystem::new(models);
    for model in system.models().models() {
        let unread: Vec<String> = model.unread_value_types.iter().map(|(registration, count)| format!("{count} register_{registration}")).collect();
        println!(
            "{}: {} registrations with the untyped value conversions read, not read: {}",
            model.crate_name,
            model.value_types.len() + model.casts.len(),
            if unread.is_empty() { "none".to_string() } else { unread.join(", ") }
        );
    }
    let types = ModelEmitTypes::new(system.clone(), Vec::new());
    let host = EmitterHost { type_system: system.as_type_system(), types: &types, parsers: Vec::new() };
    let options = TransformOptions::of(&configuration);
    let found = compile_documents_with(&host, DOCUMENTS, Some(generated::ROOT_URI), &options);
    assert_eq!(expected.len(), found.len());

    let mut outcome = Outcome { same: Vec::new(), different: Vec::new(), refused: BTreeMap::new(), not_eligible_at_run_time: Vec::new(), same_file: false };
    for (expected, found) in expected.iter().zip(&found) {
        let name = expected.name.clone();
        match (&expected.source, &found.source) {
            (Ok(expected_source), Ok(found_source)) => {
                if expected_source != found_source {
                    outcome.different.push((name, first_difference(expected_source, found_source)));
                } else if shape(expected) != shape(found) {
                    outcome.different.push((name, format!("{:?} at run time, {:?} against the model", shape(expected), shape(found))));
                } else {
                    outcome.same.push(name);
                }
            }
            (Ok(_), Err(reason)) => {
                outcome.refused.insert(name, reason.clone());
            }
            (Err(_), Err(_)) => outcome.not_eligible_at_run_time.push(name),
            (Err(reason), Ok(_)) => outcome.different.push((name, format!("not eligible at run time ({reason}) and emitted against the model"))),
        }
    }
    let expected_file = generate_file(generated::ASSEMBLY_NAME, generated::ROOT_URI, DOCUMENTS, &configuration, &[]);
    let found_file = generate_file_with(&host, generated::ASSEMBLY_NAME, generated::ROOT_URI, DOCUMENTS, &options);
    outcome.same_file = expected_file.source == found_file.source && expected_file.position_map == found_file.position_map;

    println!(
        "the corpus emitted against the build-time type system: {} documents; {} the same, {} refused, {} different; {} not eligible at run time either; the generated file is {}",
        DOCUMENTS.len(),
        outcome.same.len(),
        outcome.refused.len(),
        outcome.different.len(),
        outcome.not_eligible_at_run_time.len(),
        if outcome.same_file { "the same" } else { "another" }
    );
    println!("---- different ({}) ----", outcome.different.len());
    for (name, first) in &outcome.different {
        println!("{name}: {first}");
    }
    println!("---- refused ({}) ----", outcome.refused.len());
    for (name, reason) in &outcome.refused {
        println!("{name}: {reason}");
    }
    outcome
}

/// Not from upstream: the generated Rust of the corpus is the same against the build-time
/// type system as against the run-time type system, for every document the models can
/// answer for; the others are the asserted list of refused documents.
#[test]
fn corpus_is_emitted_the_same_against_the_model_type_system() {
    crate::register_types();
    ferroui_controls::register_types();

    let outcome = emit_against(scanned_models());
    assert!(outcome.different.is_empty(), "{} documents are emitted differently against the build-time type system (printed above; run with --nocapture)", outcome.different.len());
    let expected_refused: Vec<&str> = REFUSED.iter().map(|(name, _)| *name).collect();
    let found_refused: Vec<&str> = outcome.refused.keys().map(String::as_str).collect();
    assert_eq!(found_refused, expected_refused, "the documents the build-time type system refuses are not the asserted ones");
    for (name, question) in REFUSED {
        let reason = &outcome.refused[*name];
        assert!(reason.contains(question), "{name} is refused for another reason: {reason}");
    }
    assert_eq!(outcome.same.len() + outcome.refused.len() + outcome.not_eligible_at_run_time.len(), DOCUMENTS.len());
    assert_eq!(outcome.same.len(), DOCUMENTS.len() - REFUSED.len(), "every document that is not refused is emitted the same");
    // With no document refused the whole file, its tables included, is the same text.
    assert_eq!(outcome.same_file, REFUSED.is_empty());
}

/// Not from upstream: a question the models cannot answer refuses the document, and
/// nothing else changes. With one cast of a crate marked as not read, whether a value is
/// a value of another type is not known where no model states the cast: the documents
/// that depend on the cast not existing are refused with the two types, and the others,
/// which depend on casts the models state or on none, are emitted the same.
#[test]
fn a_document_the_models_cannot_answer_for_is_refused() {
    crate::register_types();
    ferroui_controls::register_types();

    let mut models = scanned_models();
    let markup_xaml = models.iter_mut().find(|model| model.crate_name == "ferroui_markup_xaml").expect("the model of the XAML runtime library");
    markup_xaml.unread_value_types.push(("cast".to_string(), 1));

    let outcome = emit_against(models);
    assert!(outcome.different.is_empty(), "{} documents are emitted differently (printed above; run with --nocapture)", outcome.different.len());
    let refused: Vec<&str> = outcome.refused.keys().map(String::as_str).collect();
    assert_eq!(refused, ASK_FOR_A_CAST_THAT_IS_NOT_REGISTERED);
    for (name, reason) in &outcome.refused {
        assert!(
            reason.starts_with("the type system of the host cannot answer: whether a value of `") && reason.contains("a crate registers casts whose types the scanner did not read"),
            "{name} is refused for another reason: {reason}"
        );
    }
    assert_eq!(outcome.same.len(), DOCUMENTS.len() - ASK_FOR_A_CAST_THAT_IS_NOT_REGISTERED.len());
    assert!(!outcome.same_file);
}
