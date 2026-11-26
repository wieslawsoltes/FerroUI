use super::{LanguageService, NS, Project, fs};
use crate::{FileMoveDraft, apply_text_edits};
use ferroui_build_scan::{model::AssemblyModel, xaml_metadata::DocumentModel};
use ferroui_designer_project::BuildMetadata;

fn build(project: &Project, documents: &str) {
    project.write(
        "Cargo.toml",
        "[package]\nname='acct-widgets'\nversion='0.1.0'\nedition='2021'\n[lib]\npath='lib.rs'\n",
    );
    project.write("lib.rs", "");
    project.write(
        "build.rs",
        &format!(
            r#"use ferroui_build::{{Build, XamlGroup}};
fn main() {{
    Build::from_env().compile_group(XamlGroup::new("published").documents(&[{documents}])).run();
}}
"#
        ),
    );
}

fn dictionary() -> String {
    format!("<ResourceDictionary {NS}/>")
}

fn lookup(uri: &str) -> String {
    format!(
        "<ResourceDictionary {NS}><ResourceDictionary.MergedDictionaries><ResourceInclude Source='{uri}'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"
    )
}

#[test]
fn reviewed_move_preserves_a_published_alias_and_updates_its_actual_include() {
    let project = Project::new();
    build(
        &project,
        r#"("Palette.xaml", include_str!("Themes/Palette source.xaml"))"#,
    );
    project.write("Palette.xaml", &dictionary());
    project.write("Themes/Palette source.xaml", &dictionary());
    project.write(
        "Consumer.xaml",
        &lookup("ferres://acct_widgets/Palette.xaml"),
    );
    project.write(
        "lib.rs",
        "pub const PALETTE: &str = \"ferres://acct_widgets/Palette.xaml\";\n",
    );
    let mut service = LanguageService::default();
    project.index(&mut service);

    let physical = project.uri("Palette.xaml");
    let unrelated = service
        .plan_file_move(&physical, &project.uri("Themes/Archive/Unused.xaml"))
        .unwrap();
    assert_eq!(unrelated.documents.len(), 1);
    assert_eq!(unrelated.documents[0].edit.uri, physical);

    let input = project.uri("Themes/Palette source.xaml");
    let moved = service
        .plan_file_move(
            &input,
            &project.uri("Themes/Archive/Customer palettes.xaml"),
        )
        .unwrap();
    assert_eq!(moved.documents.len(), 2);
    let script = moved
        .documents
        .iter()
        .find(|doc| doc.edit.uri == project.uri("build.rs"))
        .unwrap();
    assert_eq!(script.edit.edits.len(), 1);
    assert!(
        script
            .after
            .contains(r#"("Palette.xaml", include_str!("Themes/Archive/Customer palettes.xaml"))"#)
    );
}

#[test]
fn reviewed_move_retargets_consumers_of_the_built_assembly_and_checks_live_build_drafts() {
    let project = Project::new();
    build(
        &project,
        r#"("Themes/Card.xaml", include_str!("Themes/Card.xaml"))"#,
    );
    project.write("Themes/Card.xaml", &dictionary());
    let other = Project::new();
    let authored = "ferres://Company.Widgets/Themes/Card.xaml";
    other.write("Consumer.xaml", &lookup(&format!("{authored}#card")));
    other.write("consumer.rs", &format!("// Unicode checks exact ranges 🎨\r\npub const CARD: &str = r#\"{authored}?theme=dark#card\"#;\r\n"));

    let directory = project.0.join("target/schema");
    fs::create_dir_all(&directory).unwrap();
    let mut model = AssemblyModel::new("Company.Widgets", "acct_widgets");
    model.documents.push(DocumentModel {
        uri: authored.into(),
        root_type: "FerroUI.Controls.ResourceDictionary".into(),
        class_rust_path: None,
        build_path: Some("::acct_widgets::published::build_themes_card_xaml".into()),
        populate_path: None,
        public: true,
    });
    fs::write(directory.join("acct_widgets.xamlmeta"), model.to_json()).unwrap();
    let mut service = LanguageService::default();
    project.index(&mut service);
    other.index(&mut service);
    service
        .set_build_metadata(&BuildMetadata {
            manifest: project.0.join("Cargo.toml"),
            directories: vec![directory],
            package: Some("acct-widgets".into()),
        })
        .unwrap();

    let from = project.uri("Themes/Card.xaml");
    let to = project.uri("Themes/Archive/Customer & notes.xaml");
    let ready = service.plan_file_move(&from, &to).unwrap();
    assert_eq!(ready.documents.len(), 4);
    let next = "ferres://Company.Widgets/Themes/Archive/Customer%20%26%20notes.xaml";
    for consumer in ["Consumer.xaml", "consumer.rs"] {
        let document = ready
            .documents
            .iter()
            .find(|doc| doc.edit.uri == other.uri(consumer))
            .unwrap();
        assert_eq!(document.edit.edits.len(), 1);
        assert_eq!(document.after, document.before.replace(authored, next));
        assert_eq!(
            apply_text_edits(&document.before, &document.edit.edits).unwrap(),
            document.after
        );
    }

    let script = fs::read_to_string(project.0.join("build.rs")).unwrap();
    let draft = FileMoveDraft {
        uri: project.uri("build.rs"),
        version: 8,
        source: script.replace(r#"("Themes/Card.xaml", "#, r#"("Stable/Card.xaml", "#),
    };
    let changed = service
        .plan_file_move_with_drafts(&from, &to, &[draft])
        .unwrap();
    assert_ne!(ready.review, changed.review);
    assert_eq!(changed.documents.len(), 2);
    let script = changed
        .documents
        .iter()
        .find(|doc| doc.edit.uri == project.uri("build.rs"))
        .unwrap();
    assert_eq!(script.edit.version, 8);
    assert!(
        script.after.contains(
            r#"("Stable/Card.xaml", include_str!("Themes/Archive/Customer & notes.xaml"))"#
        )
    );
}

#[test]
fn reviewed_move_rejects_an_occupied_compiled_address_even_when_the_file_destination_is_free() {
    let project = Project::new();
    build(
        &project,
        r#"
        ("Card.xaml", include_str!("Card.xaml")),
        ("themes/archive/card.xaml", include_str!("Other.xaml")),
    "#,
    );
    project.write("Card.xaml", &dictionary());
    project.write("Other.xaml", &dictionary());
    let mut service = LanguageService::default();
    project.index(&mut service);
    let error = service
        .plan_file_move(
            &project.uri("Card.xaml"),
            &project.uri("Themes/Archive/Card.xaml"),
        )
        .unwrap_err();
    assert!(error.to_string().contains("registered XAML address"));
    assert!(project.0.join("Card.xaml").is_file());
    assert!(!project.0.join("Themes/Archive/Card.xaml").exists());
}

#[test]
fn reviewed_move_preserves_a_same_named_registration_owned_by_another_workspace() {
    let project = Project::new();
    let other = Project::new();
    let uri = "ferres://acct_widgets/Card.xaml";
    for fixture in [&project, &other] {
        build(fixture, r#"("Card.xaml", include_str!("Card.xaml"))"#);
        fixture.write("Card.xaml", &dictionary());
        fixture.write("Consumer.xaml", &lookup(uri));
        fixture.write("lib.rs", &format!("pub const CARD: &str = \"{uri}\";"));
    }
    let mut service = LanguageService::default();
    project.index(&mut service);
    other.index(&mut service);
    let plan = service
        .plan_file_move(
            &project.uri("Card.xaml"),
            &project.uri("Themes/Archive/Card.xaml"),
        )
        .unwrap();
    assert_eq!(plan.documents.len(), 4);
    for document in plan.documents {
        assert!(document.edit.uri.starts_with(&project.uri("")));
    }
}
