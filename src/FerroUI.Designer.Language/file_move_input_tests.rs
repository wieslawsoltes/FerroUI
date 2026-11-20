use super::{fs, LanguageService, Project, NS};
use crate::FileMoveDraft;
use ferroui_build_scan::{
    compiled_index::{CompiledXamlGroup, CompiledXamlIndex, CompiledXamlInput, CompiledXamlMember},
    compiled_names::CompiledGroupKind,
    model::AssemblyModel,
    xaml_metadata::DocumentModel,
};
use ferroui_designer_project::BuildMetadata;

const PATH: &str = "Themes/Card.xaml";
const URI: &str = "ferres://Customer.Widgets/Themes/Card.xaml";
const REGISTRATION: &str = "file_input!(\"Themes/Card.xaml\", r#\"Themes/Card.xaml\"#)";

#[test]
fn moves_a_compiler_selected_input_from_a_custom_build_module() {
    let (project, mut service, source) = prepare(REGISTRATION);
    let uri = project.uri("tools/inputs.rs");
    let from = project.uri(PATH);
    let to = project.uri("Themes/Archive/New Card.xaml");
    let draft = FileMoveDraft {
        uri: uri.clone(),
        source: format!("{source}// Open draft 🎨\r\n"),
        version: 7,
    };
    let plan = service
        .plan_file_move_with_drafts(&from, &to, std::slice::from_ref(&draft))
        .unwrap();
    let edit = plan
        .documents
        .iter()
        .find(|doc| doc.edit.uri == uri)
        .unwrap();
    assert_eq!(edit.edit.version, 7);
    assert_eq!(edit.edit.edits.len(), 1);
    assert_eq!(edit.before, draft.source);
    assert_eq!(
        edit.after,
        draft.source.replacen(
            "r#\"Themes/Card.xaml\"#",
            "r#\"Themes/Archive/New Card.xaml\"#",
            1
        )
    );
    assert!(plan.generated_rust.is_empty());
    assert_eq!(plan.documents.len(), 2, "{:#?}", plan.documents);
    assert!(plan
        .documents
        .iter()
        .all(|doc| !doc.edit.uri.contains("/target/")));

    let moved_site = FileMoveDraft {
        source: format!("\r\n{}", draft.source),
        ..draft
    };
    let error = service
        .plan_file_move_with_drafts(&from, &to, &[moved_site])
        .unwrap_err();
    assert!(
        error.0.contains("rebuild the project XAML schema"),
        "{error}"
    );
}

#[test]
fn refuses_a_runtime_computed_path_or_a_rebased_compiled_relative_uri() {
    let (project, mut service, _) = prepare("file_input!(\"Themes/Card.xaml\", &computed)");
    let from = project.uri(PATH);
    let to = project.uri("Themes/Archive/New Card.xaml");
    let error = service.plan_file_move(&from, &to).unwrap_err();
    assert!(
        error.0.contains("literal file path in xaml_document_file!"),
        "{error}"
    );

    let (project, mut service, _) = prepare(REGISTRATION);
    project.write(
        "Themes/Styles.xaml",
        &format!("<ResourceDictionary {NS} />"),
    );
    project.write(
        PATH,
        &format!(
            "<ResourceDictionary {NS}><ResourceInclude Source='Styles.xaml'/></ResourceDictionary>"
        ),
    );
    let error = service
        .plan_file_move(&project.uri(PATH), &project.uri("Themes/Archive/Card.xaml"))
        .unwrap_err();
    assert!(error.0.contains("explicit XAML resource URI"), "{error}");
}

#[test]
fn asks_for_the_compiler_index_when_computed_groups_are_present() {
    let (project, mut service, _) = prepare(REGISTRATION);
    fs::remove_file(
        project
            .0
            .join("target/schema")
            .join(CompiledXamlIndex::FILE_NAME),
    )
    .unwrap();
    let error = service
        .plan_file_move(&project.uri(PATH), &project.uri("Themes/Archive/Card.xaml"))
        .unwrap_err();
    assert!(error.0.contains("Build the project XAML schema"), "{error}");
}

fn prepare(constructor: &str) -> (Project, LanguageService, String) {
    let project = Project::new();
    fs::create_dir(project.0.join("tools")).unwrap();
    project.write("Cargo.toml", "[package]\nname='customer-widgets'\nversion='0.1.0'\nbuild='tools/build.rs'\n[lib]\npath='lib.rs'\n");
    project.write("lib.rs", &format!("const CARD: &str = {URI:?};\n"));
    project.write(
        "tools/build.rs",
        r#"
use ferroui_build::{Build, XamlGroup};
mod inputs;
fn main() {
    let mut build = Build::from_env();
    for module in ["main_theme", "alternate"] {
        let files = inputs::selected(module);
        build = build.compile_group(XamlGroup::new(module)
            .class_document("Themes/Card.xaml").document_files(&files));
    }
    build.run();
}
"#,
    );
    let source = format!("use ferroui_build::{{xaml_document_file as file_input, XamlDocumentFile}};\r\npub fn selected(_module: &str) -> [XamlDocumentFile<'static>; 1] {{\r\n    let computed = {PATH:?};\r\n    /* 🎨 */ [{constructor}]\r\n}}\r\nfn dormant() {{ let _ = {REGISTRATION}; }}\r\n");
    project.write("tools/inputs.rs", &source);
    project.write(PATH, &format!("<ResourceDictionary {NS} />"));
    project.write(
        "Consumer.xaml",
        &format!("<ResourceDictionary {NS}><ResourceInclude Source='{URI}'/></ResourceDictionary>"),
    );

    let mut model = AssemblyModel::new("Customer.Widgets", "customer_widgets");
    model.documents.push(DocumentModel {
        uri: URI.into(),
        root_type: "ResourceDictionary".into(),
        class_rust_path: None,
        build_path: None,
        populate_path: Some("::customer_widgets::main_theme::populate".into()),
        public: true,
    });
    let manifest = project.0.join("Cargo.toml");
    let mut index = CompiledXamlIndex::new(manifest.clone());
    index.groups.push(CompiledXamlGroup {
        module: "main_theme".into(),
        kind: CompiledGroupKind::Class,
        discovery_root: None,
        file_backed: true,
        members: vec![CompiledXamlMember {
            key: PATH.into(),
            symbol: Some("populate".into()),
            public: true,
        }],
    });
    index.file_inputs.push(CompiledXamlInput {
        path: PATH.into(),
        rust_file: std::path::PathBuf::from(project.0.file_name().unwrap()).join("tools/inputs.rs"),
        line: 4,
        column: "    /* 🎨 */ [".chars().count() as u32 + 1,
    });
    let directory = project.0.join("target/schema");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join(CompiledXamlIndex::FILE_NAME),
        index.to_json(),
    )
    .unwrap();
    fs::write(directory.join("customer_widgets.xamlmeta"), model.to_json()).unwrap();
    let mut service = LanguageService::default();
    project.index(&mut service);
    service
        .set_build_metadata(&BuildMetadata {
            manifest,
            directories: vec![directory],
            package: Some("customer-widgets".into()),
        })
        .unwrap();
    (project, service, source)
}
