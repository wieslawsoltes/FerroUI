use super::{fs, LanguageService, Project, NS};
use ferroui_build_scan::{
    compiled_index::{CompiledXamlGroup, CompiledXamlIndex, CompiledXamlInput, CompiledXamlMember},
    compiled_names::CompiledGroupKind,
    model::AssemblyModel,
    packaged_assets::{PackagedAsset, PackagedAssetIndex},
};
use ferroui_designer_project::{
    hosted_preview_sources, local_preview_sources_with_resources, resolve_uri, BuildMetadata,
    PreviewSourceBuffer,
};

const CARD: &str = "Themes/Card.xaml";
const COLORS: &str = "Themes/Colors.xaml";
const PALETTE: &str = "Colors.xaml";
const ACTIVE_GROUP: &str = "compiled_views";
const ASSEMBLY: &str = "Customer.Views";

#[test]
fn published_asset_addresses_use_the_selected_package_receipt() {
    let (project, mut service, _) = setup(false);
    let owner = project.uri("View.xaml");
    let badge = "Assets/Badge 100%.png";
    let address = "ferres://CUSTOMER.VIEWS/Assets/Badge%20100%25.png";
    let unpublished = "ferres://Customer.Views/Assets/Hidden.png";
    project.write(badge, "badge bytes");
    project.write("Assets/Hidden.png", "unpublished bytes");
    let source = format!("<Image {NS} Source='{address}' />");
    project.write("View.xaml", &source);
    service.open(&owner, &source, 1).unwrap();

    let output = project.0.join("target/schema");
    let manifest = project.0.join("Cargo.toml");
    let receipt = PackagedAssetIndex {
        manifest: manifest.clone(),
        assembly: ASSEMBLY.into(),
        crate_name: "customer_views".into(),
        assets: vec![PackagedAsset { key: format!("/{badge}"), source: badge.into() }],
    };
    fs::write(output.join(PackagedAssetIndex::FILE_NAME), receipt.to_json()).unwrap();
    service.set_build_metadata(&BuildMetadata {
        manifest,
        directories: vec![output],
        package: Some("customer-views".into()),
    }).unwrap();

    assert_eq!(service.resource_uri(&owner, address).unwrap(), project.uri(badge));
    assert_eq!(service.definitions(&owner, source.find(address).unwrap() + 10).unwrap()[0].uri, project.uri(badge));
    assert_eq!(service.resource_uri(&owner, unpublished).unwrap(), unpublished);
    let wrong_case = address.replace("Badge", "badge");
    assert_eq!(service.resource_uri(&owner, &wrong_case).unwrap(), wrong_case);
    let excluded = source.replace(address, unpublished);
    service.open(&owner, &excluded, 2).unwrap();
    assert!(service.definitions(&owner, excluded.find(unpublished).unwrap() + 10).unwrap().is_empty());
    assert!(service.plan_asset_move(&project.uri(badge), &project.uri("Assets/Moved.png"), &[])
        .unwrap_err().0.contains("published by a Cargo export table"));
    assert!(service.plan_asset_move(&project.uri("Assets/Hidden.png"), &project.uri("Assets/Moved.png"), &[]).is_ok());
}

#[test]
fn moving_a_private_compiled_dictionary_keeps_relative_includes_and_rebases_loose_files() {
    let (project, mut service, main) = setup(false);
    let from = project.uri(COLORS);
    let to = project.uri("Themes/Archive/New Colors.xaml");
    let owner = project.uri(CARD);
    let loose = project.uri("Loose.xaml");
    assert_eq!(service.resource_uri(&owner, PALETTE).unwrap(), from);
    assert!(service
        .file_rename_edits(&from, &to)
        .unwrap()
        .document_changes
        .iter()
        .all(|doc| doc.uri != owner));
    let plan = service.plan_file_move(&from, &to).unwrap();
    assert!(plan.generated_rust.is_empty());
    assert!(plan.documents.iter().all(|doc| doc.edit.uri != owner));
    let edit = |uri: &str| {
        plan.documents
            .iter()
            .find(|doc| doc.edit.uri == uri)
            .unwrap()
            .after
            .as_str()
    };
    assert!(edit(&loose).contains("Themes/Archive/New%20Colors.xaml"));
    assert!(edit(&project.uri("build.rs")).contains("Themes/Archive/New Colors.xaml"));
    project.write("build.rs", edit(&project.uri("build.rs")));

    fs::rename(
        project.0.join(COLORS),
        project.0.join("Themes/Archive/New Colors.xaml"),
    )
    .unwrap();
    service.rename_uri(&from, &to).unwrap();
    assert_eq!(service.resource_uri(&owner, PALETTE).unwrap(), to);
    let draft = format!("<ResourceDictionary {NS}><SolidColorBrush x:Key='Accent'>#275CF5</SolidColorBrush></ResourceDictionary>");
    let resources = service.compiled_resources(&owner).unwrap();
    let graph = local_preview_sources_with_resources(
        Some(&project.0),
        &owner,
        &main,
        &[PreviewSourceBuffer {
            uri: to.clone(),
            source: Some(draft.clone()),
        }],
        Some(&resources),
    );
    assert_eq!(
        graph.accepted[&resolve_uri(&owner, PALETTE).unwrap()],
        draft
    );
    assert_eq!(graph.dependencies.len(), 1);
    assert!(graph.dependencies.contains(&to));
    assert!(graph.unavailable.is_empty());

    let again = service
        .plan_file_move(&to, &project.uri("Themes/Archive/Final.xaml"))
        .unwrap();
    assert!(again
        .documents
        .iter()
        .any(|change| change.edit.uri == project.uri("build.rs")
            && change.after.contains("Themes/Archive/Final.xaml")));

    fs::rename(
        project.0.join("Themes/Archive/New Colors.xaml"),
        project.0.join(COLORS),
    )
    .unwrap();
    service.rename_uri(&to, &from).unwrap();
    assert_eq!(service.resource_uri(&owner, PALETTE).unwrap(), from);
}

#[test]
fn moving_a_compiled_owner_keeps_its_group_base_in_the_preview() {
    let (project, mut service, main) = setup(false);
    let from = project.uri(CARD);
    let to = project.uri("Themes/Archive/Renamed Card.xaml");
    let plan = service.plan_file_move(&from, &to).unwrap();
    let view = plan
        .documents
        .iter()
        .find(|doc| doc.edit.uri == from)
        .unwrap();
    assert!(view.edit.edits.is_empty());
    assert_eq!(view.before, view.after);
    fs::rename(
        project.0.join(CARD),
        project.0.join("Themes/Archive/Renamed Card.xaml"),
    )
    .unwrap();
    service.rename_uri(&from, &to).unwrap();
    let resources = service.compiled_resources(&to).unwrap();
    let graph =
        local_preview_sources_with_resources(Some(&project.0), &to, &main, &[], Some(&resources));
    assert!(graph.accepted[&resolve_uri(&to, PALETTE).unwrap()].contains("#1199CC"));
    assert_eq!(
        graph.dependencies.into_iter().collect::<Vec<_>>(),
        [project.uri(COLORS)]
    );
    let hosted_base = resources.preview_base(&to, "ferres://Customer.Views/Themes/Archive/Renamed%20Card.xaml").unwrap();
    assert_eq!(hosted_base, "ferres://Customer.Views/App/Card.xaml");
    let hosted = hosted_preview_sources(&project.0, &to, hosted_base, &main, &[], Some(&resources));
    assert!(hosted.accepted["ferres://Customer.Views/App/Colors.xaml"].contains("#1199CC"));
}

#[test]
fn different_targets_for_the_same_relative_literal_require_a_new_group_scope() {
    let (project, mut service, _) = setup(true);
    let owner = project.uri(CARD);
    let to = project.uri("Themes/Archive/Moved.xaml");
    assert!(service
        .resource_uri(&owner, PALETTE)
        .unwrap_err()
        .0
        .contains("different inputs"));
    let error = service
        .plan_file_move(&project.uri(COLORS), &to)
        .unwrap_err();
    assert!(error.0.contains("different inputs"), "{error}");
}

fn setup(ambiguous: bool) -> (Project, LanguageService, ferroui_designer_core::Document) {
    let project = Project::new();
    project.write(
        "Cargo.toml",
        "[package]\nname='customer-views'\nversion='0.1.0'\n[lib]\npath='lib.rs'\n",
    );
    project.write("lib.rs", "");
    project.write("build.rs", &format!("use ferroui_build::{{Build, XamlGroup, xaml_document_file}};\nfn main() {{\n    let card = xaml_document_file!(\"App/Card.xaml\", {CARD:?});\n    let colors = xaml_document_file!(\"App/Colors.xaml\", {COLORS:?});\n    Build::from_env().compile_group(XamlGroup::new({ACTIVE_GROUP:?}).document_files(&[card, colors])).run();\n}}\n"));
    let main = format!(
        "<ResourceDictionary {NS}><ResourceInclude Source='./{PALETTE}'/></ResourceDictionary>"
    );
    project.write(CARD, &main);
    project.write(COLORS, &format!("<ResourceDictionary {NS}><SolidColorBrush x:Key='Accent'>#1199CC</SolidColorBrush></ResourceDictionary>"));
    project.write(
        "Loose.xaml",
        &format!(
            "<ResourceDictionary {NS}><ResourceInclude Source='{COLORS}'/></ResourceDictionary>"
        ),
    );
    let mut index = CompiledXamlIndex::new(project.0.join("Cargo.toml"));
    index.assembly = Some(ASSEMBLY.into());
    for (path, line, prefix) in [
        (CARD, 3, "    let card = "),
        (COLORS, 4, "    let colors = "),
    ] {
        index.file_inputs.push(CompiledXamlInput {
            path: path.into(),
            rust_file: "build.rs".into(),
            line,
            column: prefix.len() as u32 + 1,
        });
    }
    index.groups.push(group(ACTIVE_GROUP, COLORS));
    if ambiguous {
        project.write("Themes/Other.xaml", &format!("<ResourceDictionary {NS} />"));
        index.groups.push(group("alternate", "Themes/Other.xaml"));
    }
    let directory = project.0.join("target/schema");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join(CompiledXamlIndex::FILE_NAME),
        index.to_json(),
    )
    .unwrap();
    fs::write(
        directory.join("customer_views.xamlmeta"),
        AssemblyModel::new(ASSEMBLY, "customer_views").to_json(),
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
    (
        project,
        service,
        ferroui_designer_core::Document::new(&main).unwrap(),
    )
}

fn group(module: &str, colors: &str) -> CompiledXamlGroup {
    CompiledXamlGroup {
        module: module.into(),
        kind: CompiledGroupKind::Documents,
        discovery_root: None,
        file_backed: true,
        members: [("App/Card.xaml", CARD), ("App/Colors.xaml", colors)]
            .into_iter()
            .map(|(key, source)| CompiledXamlMember {
                key: key.into(),
                symbol: None,
                public: false,
                source_path: Some(source.into()),
            })
            .collect(),
    }
}
