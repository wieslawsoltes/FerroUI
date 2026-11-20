use super::{LanguageService, NS, Project, fs};
use crate::{DocumentEdits, FileMoveDraft, FileMoveRustResolution, LineIndex, TextEdit};
use ferroui_build_scan::{
    compiled_index::{CompiledXamlGroup, CompiledXamlIndex, CompiledXamlMember},
    compiled_names::CompiledGroupKind,
    model::AssemblyModel,
};
use ferroui_designer_core::SourceSpan;
use ferroui_designer_project::BuildMetadata;

fn compiled_project(
    keys: &[&str],
    kind: CompiledGroupKind,
    discovery: bool,
) -> (Project, LanguageService, Vec<String>) {
    let project = Project::new();
    project.write(
        "Cargo.toml",
        "[package]\nname='customer-views'\nversion='0.1.0'\n[lib]\npath='lib.rs'\n",
    );
    project.write("lib.rs", "");
    let documents = keys
        .iter()
        .map(|key| format!("({key:?}, include_str!({key:?}))"))
        .collect::<Vec<_>>();
    let mut group = if discovery {
        "XamlGroup::new(\"views\")".to_owned()
    } else {
        format!(
            "XamlGroup::new(\"views\").documents(&[{}]).internal()",
            documents.join(", ")
        )
    };
    if kind == CompiledGroupKind::Class {
        group.push_str(&format!(".class_document({:?})", keys[0]));
    }
    let root = if discovery { "Themes" } else { "." };
    project.write("build.rs", &format!("use ferroui_build::{{Build, XamlGroup}};\nfn main() {{ Build::from_env().xaml_root({root:?}).compile_group({group}).run(); }}\n"));
    for key in keys {
        project.write(key, &format!("<ResourceDictionary {NS} />"));
    }
    let directory = project.0.join("target/schema");
    fs::create_dir_all(directory.join("xaml")).unwrap();
    let mut names = kind.function_names(keys.iter().copied());
    if kind == CompiledGroupKind::Class {
        names[0] = "populate".into();
    }
    let generated = names
        .iter()
        .map(|name| format!("pub(crate) fn {name}() {{}}\n"))
        .collect::<String>();
    project.write("target/schema/xaml/views.rs", &generated);
    let mut index = CompiledXamlIndex::new(project.0.join("Cargo.toml"));
    index.groups.push(CompiledXamlGroup {
        module: "views".into(),
        kind,
        discovery_root: discovery.then(|| "Themes".into()),
        file_backed: false,
        members: keys
            .iter()
            .zip(&names)
            .map(|(key, name)| CompiledXamlMember {
                key: (*key).into(),
                symbol: Some(name.clone()),
                public: true,
            })
            .collect(),
    });
    fs::write(
        directory.join(CompiledXamlIndex::FILE_NAME),
        index.to_json(),
    )
    .unwrap();
    fs::write(
        directory.join("customer_views.xamlmeta"),
        AssemblyModel::new("customer_views", "customer_views").to_json(),
    )
    .unwrap();
    let mut service = LanguageService::default();
    project.index(&mut service);
    service
        .set_build_metadata(&BuildMetadata {
            manifest: index.manifest,
            directories: vec![directory],
            package: Some("customer-views".into()),
        })
        .unwrap();
    (project, service, names)
}

#[test]
fn move_reviews_both_disambiguated_symbols_and_merges_exact_semantic_edits() {
    let (project, mut service, names) = compiled_project(
        &["Themes/A-B.xaml", "Themes/A_B.xaml"],
        CompiledGroupKind::Documents,
        true,
    );
    let from = project.uri("Themes/A-B.xaml");
    let to = project.uri("Themes/Archive/Clients.xaml");
    let source = format!(
        "const MARK: &str = \"🧭\"; use views::{{{} as build_card, {}}};\r\n// leave {} alone\r\nfn views() {{ build_card(); {}(); }}\r\n",
        names[0], names[1], names[0], names[1]
    );
    let draft = FileMoveDraft {
        uri: project.uri("lib.rs"),
        version: 18,
        source,
    };
    let review = service
        .plan_file_move_with_drafts(&from, &to, std::slice::from_ref(&draft))
        .unwrap();
    assert_eq!(review.generated_rust.len(), 2);
    let first = &review.generated_rust[0];
    assert_eq!(first.uri, project.uri("target/schema/xaml/views.rs"));
    assert_eq!((first.position.line, first.position.character), (0, 14));
    assert_eq!(first.old_name, names[0]);
    assert_eq!(first.new_name, "build_themes_archive_clients_xaml");
    assert_eq!(review.generated_rust[1].new_name, "build_themes_a_b_xaml");
    let source_index = LineIndex::new(&draft.source);
    let mut resolutions: Vec<_> = review
        .generated_rust
        .iter()
        .map(|rename| {
            let edits = draft
                .source
                .match_indices(&rename.old_name)
                .filter(|(start, _)| !draft.source[..*start].ends_with("// leave "))
                .map(|(start, name)| TextEdit {
                    range: source_index
                        .range(SourceSpan::new(start, start + name.len()))
                        .unwrap(),
                    new_text: rename.new_name.clone(),
                })
                .collect();
            FileMoveRustResolution {
                rename: rename.clone(),
                documents: vec![DocumentEdits {
                    uri: draft.uri.clone(),
                    version: draft.version,
                    edits,
                }],
            }
        })
        .collect();
    let resolved = service
        .complete_file_move(
            &from,
            &to,
            std::slice::from_ref(&draft),
            &review.review,
            &resolutions,
        )
        .unwrap();
    let rust = resolved
        .documents
        .iter()
        .find(|document| document.edit.uri == draft.uri)
        .unwrap();
    assert_eq!(rust.edit.edits.len(), 3);
    assert!(
        rust.after
            .contains("build_themes_archive_clients_xaml as build_card, build_themes_a_b_xaml")
    );
    assert!(
        rust.after
            .contains(&format!("// leave {} alone\r\n", names[0]))
    );
    assert_ne!(resolved.review, review.review);

    // Delayed semantic results cannot apply to a new buffer, even when its
    // edits have no effect on the XAML URI patches.
    let stale = service.complete_file_move(
        &from,
        &to,
        &[FileMoveDraft {
            uri: draft.uri.clone(),
            version: 19,
            source: draft.source.clone(),
        }],
        &review.review,
        &resolutions,
    );
    assert!(stale.unwrap_err().to_string().contains("sources changed"));

    let comment = draft.source.rfind(&names[0]).unwrap();
    resolutions[0].documents[0].edits[0].range = source_index
        .range(SourceSpan::new(comment, comment + names[0].len()))
        .unwrap();
    let fabricated = service.complete_file_move(&from, &to, &[draft], &review.review, &resolutions);
    assert!(
        fabricated
            .unwrap_err()
            .to_string()
            .contains("identifier tokens")
    );
}

#[test]
fn a_class_move_keeps_populate_and_reviews_internal_compilation_members() {
    let keys = ["Themes/A.xaml", "Themes/B.xaml"];
    let (project, mut service, _) = compiled_project(&keys, CompiledGroupKind::Class, true);
    let plan = service
        .plan_file_move(
            &project.uri(keys[0]),
            &project.uri("Themes/Archive/Account class.xaml"),
        )
        .unwrap();
    assert!(plan.generated_rust.is_empty());
    let script = plan
        .documents
        .iter()
        .find(|item| item.edit.uri == project.uri("build.rs"))
        .unwrap();
    assert!(
        script
            .after
            .contains("class_document(\"Themes/Archive/Account class.xaml\")")
    );
    let part = service
        .plan_file_move(
            &project.uri(keys[1]),
            &project.uri("Themes/Archive/Part.xaml"),
        )
        .unwrap();
    assert_eq!(
        part.generated_rust[0].new_name,
        "build_themes_archive_part_xaml"
    );

    let (project, mut service, _) =
        compiled_project(&["Themes/A.xaml"], CompiledGroupKind::Documents, false);
    let plan = service
        .plan_file_move(
            &project.uri("Themes/A.xaml"),
            &project.uri("Themes/Archive/A.xaml"),
        )
        .unwrap();
    assert_eq!(plan.generated_rust.len(), 1);
    assert_eq!(
        plan.generated_rust[0].new_name,
        "build_themes_archive_a_xaml"
    );
}
