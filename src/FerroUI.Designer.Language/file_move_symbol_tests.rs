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
        fs::create_dir_all(project.0.join(key).parent().unwrap()).unwrap();
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
                source_path: None,
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

#[test]
fn cargo_file_read_moves_the_raw_path_and_the_escaped_compiler_key() {
    let prior = "Themes/Card.xaml";
    let (project, mut service, names) =
        compiled_project(&[prior], CompiledGroupKind::Documents, false);
    let source = r##"use ferroui_build::{Build, XamlGroup};
const DOCUMENT: &str = r#"Themes/Card.xaml"#;
fn main() {
    // Preserve 🧭 diagnostics and Cargo's physical file watch.
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest"));
    let path = root.join(DOCUMENT);
    println!("cargo::rerun-if-changed={}", path.display());
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{error}"));
    let views = [(DOCUMENT, text.as_str())];
    Build::from_env().compile_group(XamlGroup::new("views").documents(&views)).run();
}
"##
    .replace('\n', "\r\n");
    project.write("build.rs", &source);
    let uri = format!("ferres://customer_views/{prior}");
    let xaml =
        format!("<ResourceDictionary {NS}><ResourceInclude Source='{uri}'/></ResourceDictionary>");
    project.write("Page.xaml", &xaml);
    let rust = format!("const RESOURCE: &str = {uri:?};\r\n");
    project.write("lib.rs", &rust);
    project.index(&mut service);
    for (path, key) in [
        ("Themes/Archive/Client.xaml", "Themes/Archive/Client.xaml"),
        (
            "Themes/Archive/New Card.xaml",
            "Themes/Archive/New%20Card.xaml",
        ),
    ] {
        let review = service
            .plan_file_move(&project.uri(prior), &project.uri(path))
            .unwrap();
        let after = |file| {
            let uri = project.uri(file);
            &review
                .documents
                .iter()
                .find(|entry| entry.edit.uri == uri)
                .unwrap()
                .after
        };
        let mut expected = source.replace(prior, path);
        if path != key {
            expected = expected.replace("(DOCUMENT,", &format!("({key:?},"));
        }
        assert_eq!(after("build.rs"), &expected);
        assert_eq!(after("Page.xaml"), &xaml.replace(prior, key));
        assert_eq!(after("lib.rs"), &rust.replace(prior, key));
        let [rename] = review.generated_rust.as_slice() else {
            panic!("expected an exact generated declaration");
        };
        assert_eq!(rename.old_name, names[0]);
        assert_eq!(
            rename.new_name,
            CompiledGroupKind::Documents.function_names([key])[0]
        );
    }
}

#[test]
fn cargo_file_read_preserves_a_class_alias_and_checks_its_base() {
    let alias = "Published/Card.xaml";
    let (project, mut service, _) = compiled_project(&[alias], CompiledGroupKind::Class, false);
    fs::remove_file(project.0.join(alias)).unwrap();
    let source = r#"use std::{env, fs::read_to_string as read, path::PathBuf};
use ferroui_build::{Build, XamlGroup};
const KEY: &str = "Published/Card.xaml";
const SOURCE: &str = "Themes/Card.xaml";
fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest"));
    let text = read(root.join(SOURCE)).expect("valid XAML");
    let group = XamlGroup::new("views").documents(&[(KEY, &text)]).class_document(KEY);
    Build::from_env().compile_group(group).run();
}
"#;
    project.write("build.rs", source);
    project.write("Themes/Card.xaml", &format!("<ResourceDictionary {NS}/>"));
    let from = project.uri("Themes/Card.xaml");
    let to = project.uri("Themes/Archive/Card.xaml");
    let review = service.plan_file_move(&from, &to).unwrap();
    assert!(review.generated_rust.is_empty());
    let [source_entry, script] = review.documents.as_slice() else {
        panic!("expected exactly the moved file and the Cargo build");
    };
    assert_eq!(source_entry.edit.uri, from);
    assert!(source_entry.edit.edits.is_empty());
    assert_eq!(script.edit.uri, project.uri("build.rs"));
    assert_eq!(
        script.after,
        source.replace("Themes/Card.xaml", "Themes/Archive/Card.xaml")
    );

    project.write("Themes/Colors.xaml", &format!("<ResourceDictionary {NS}/>"));
    project.write(
        "Themes/Card.xaml",
        &format!(
            "<ResourceDictionary {NS}><ResourceInclude Source='Colors.xaml'/></ResourceDictionary>"
        ),
    );
    let error = service.plan_file_move(&from, &to).unwrap_err();
    assert!(error
        .to_string()
        .contains("does not map the compiled target"));
}

#[test]
fn a_cargo_read_used_as_the_class_key_escapes_both_key_uses() {
    let name = "Themes/Card.xaml";
    let (project, mut service, _) = compiled_project(&[name], CompiledGroupKind::Class, false);
    let source = r#"use ferroui_build::{Build, XamlGroup};
const FILE: &str = "Themes/Card.xaml";
fn main() {
    let text = std::fs::read_to_string(FILE).unwrap();
    Build::from_env()
        .compile_group(XamlGroup::new("views").documents(&[(FILE, &text)]).class_document(FILE))
        .run();
}
"#;
    project.write("build.rs", source);
    let review = service
        .plan_file_move(&project.uri(name), &project.uri("Themes/New Card.xaml"))
        .unwrap();
    assert!(review.generated_rust.is_empty());
    let script = review
        .documents
        .iter()
        .find(|entry| entry.edit.uri == project.uri("build.rs"))
        .unwrap();
    assert_eq!(script.edit.edits.len(), 3);
    assert_eq!(
        script.after,
        source
            .replace(name, "Themes/New Card.xaml")
            .replace("[(FILE,", "[(\"Themes/New%20Card.xaml\",")
            .replace(
                ".class_document(FILE)",
                ".class_document(\"Themes/New%20Card.xaml\")"
            )
    );
}

#[test]
fn an_indexed_key_with_an_opaque_cargo_input_requires_a_source_mapping() {
    let key = "Themes/Card.xaml";
    let (project, mut service, _) = compiled_project(&[key], CompiledGroupKind::Documents, false);
    project.write(
        "build.rs",
        r#"use ferroui_build::{Build, XamlGroup};
fn main() {
    Build::from_env().compile_group(XamlGroup::new("views").documents(&files_from_helper())).run();
}
"#,
    );
    let error = service
        .plan_file_move(&project.uri(key), &project.uri("Themes/Archive/Card.xaml"))
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("Cannot trace XAML key Themes/Card.xaml"));
}
