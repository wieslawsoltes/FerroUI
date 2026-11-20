use crate::{
    project_navigation_tests::CatalogFixture, DocumentEdits, FileMoveDraft, LanguageService,
    LineIndex, RegisteredPropertyRequest, RustRenameResolution, TextEdit,
};
use ferroui_build_scan::model::AssemblyModel;
use ferroui_designer_core::SourceSpan;
use ferroui_designer_project::{file_path, file_uri, rsx_mapping::RsxBuilderMapping, Solution};
use std::fs;

const VIEW: &str = r#"<ui:Card xmlns:ui="using:Customer.Views" Tone="Blue"/>"#;
const ASSEMBLY: &str = r###"
mod mapping {
    // Preserve the originals for other users of these constants. 🪴
    const KEY: &str = "FerroUI.Designer.Rsx.v1:Customer.Views.FeaturedCard";
    const FEATURED: &str = "{\"path\": \"crate::builders::Card\", \"properties\": {\"To\\u006Ee\": \"tone\", \"Title\": \"title\"}}";
    static DESIGN: MarkupAssembly = MarkupAssembly {
        name: "customer_app",
        crate_name: "customer_app",
        metadata: &[
            (
                "FerroUI.Designer.Rsx.v1:Customer.Views.Card",
                r##"{
                    "path": "crate::controls::cards::Card",
                    "properties": { "Tone" : "tone" }
                }"##,
            ),
            (self::KEY, FEATURED),
            ("Unrelated", "Tone Card"),
        ],
    };
}
"###;

fn request(needle: &str) -> RegisteredPropertyRequest {
    RegisteredPropertyRequest {
        version: 1,
        position: LineIndex::new(VIEW)
            .position(VIEW.find(needle).unwrap() + 1)
            .unwrap(),
    }
}

fn fixture(same_file: bool) -> (CatalogFixture, LanguageService, String, String) {
    let fixture = CatalogFixture::create();
    let metadata = fixture.build();
    let source = if same_file {
        let path = "app/rust/controls.rs";
        let controls = fs::read_to_string(fixture.0.join(path)).unwrap();
        fixture.write(path, &(controls + &ASSEMBLY.replace('\n', "\r\n")));
        path
    } else {
        let lib = fs::read_to_string(fixture.0.join("app/lib.rs")).unwrap();
        fixture.write("app/lib.rs", &(lib + "\nmod registry;\n"));
        let path = "app/registry.rs";
        fixture.write(path, &ASSEMBLY.replace('\n', "\r\n"));
        path
    };
    rebuild(&fixture);
    fixture.write("app/Rename.xaml", VIEW);
    let uri = file_uri(fixture.0.join("app/Rename.xaml")).unwrap();
    let rust_uri = file_uri(fixture.0.join(source)).unwrap();
    let mut service = LanguageService::default();
    service
        .index_solution(&Solution::discover(&fixture.0).unwrap())
        .unwrap();
    service.set_build_metadata(&metadata).unwrap();
    service.open(&uri, VIEW, 1).unwrap();
    (fixture, service, uri, rust_uri)
}

fn rebuild(fixture: &CatalogFixture) -> AssemblyModel {
    let dependency =
        fs::read_to_string(fixture.0.join("output/dep/customer_data.xamlmeta")).unwrap();
    fixture.export(
        "app",
        "customer_app",
        vec![AssemblyModel::parse(&dependency).unwrap()],
    )
}

#[test]
fn property_review_uses_unsaved_mapping_source_and_preserves_shared_constants() {
    let (fixture, mut service, uri, rust_uri) = fixture(false);
    let original = ASSEMBLY
        .replace('\n', "\r\n")
        .replace("// Preserve", "// Unsaved 🧭. Preserve");
    let draft = FileMoveDraft {
        uri: rust_uri.clone(),
        version: 32,
        source: original.clone(),
    };
    let plan = service
        .plan_registered_property_rename_with_drafts(&uri, &request("Tone"), "Accent", &[draft])
        .unwrap();
    let mapping = plan
        .documents
        .iter()
        .find(|document| document.edit.uri == rust_uri)
        .unwrap();
    assert_eq!(mapping.edit.version, 32);
    assert_eq!(mapping.before, original);
    let updated = r#"(self::KEY, "{\"path\": \"crate::builders::Card\", \"properties\": {\"Accent\": \"tone\", \"Title\": \"title\"}}")"#;
    assert_eq!(
        mapping.after,
        original
            .replace("\"Tone\" : \"tone\"", "\"Accent\" : \"tone\"")
            .replace("(self::KEY, FEATURED)", updated)
    );
    assert!(plan
        .documents
        .iter()
        .any(|entry| entry.after == VIEW.replace("Tone", "Accent")));
    assert!(plan
        .documents
        .iter()
        .any(|entry| entry.after.contains("String>(\"Accent\",")));

    let collision = original.replace(
        r#""Tone" : "tone""#,
        r#""Tone" : "tone", "Accent" : "already_mapped""#,
    );
    let draft = FileMoveDraft {
        uri: rust_uri,
        version: 33,
        source: collision,
    };
    let error = service
        .plan_registered_property_rename_with_drafts(&uri, &request("Tone"), "Accent", &[draft])
        .unwrap_err();
    assert!(error.0.contains("already maps Accent"));

    for document in &plan.documents {
        fs::write(file_path(&document.edit.uri).unwrap(), &document.after).unwrap();
    }
    let rebuilt = rebuild(&fixture);
    let mappings = rebuilt
        .metadata
        .iter()
        .filter(|(key, _)| key.starts_with("FerroUI.Designer.Rsx.v1:"));
    assert_eq!(mappings.clone().count(), 2);
    for (key, value) in mappings {
        let mapping = RsxBuilderMapping::parse(value).unwrap();
        assert_eq!(
            mapping.properties.get("Accent").map(String::as_str),
            Some("tone"),
            "{key}"
        );
    }
}

#[test]
fn type_review_merges_builder_literals_with_semantic_rust_patches_in_the_same_file() {
    let (fixture, mut service, uri, rust_uri) = fixture(true);
    let new = "OverviewCard";
    let request = request("Card");
    let base = service
        .plan_registered_type_rename(&uri, &request, new, &[])
        .unwrap();
    let rename = base.rust_rename.as_ref().unwrap();
    assert_eq!(rust_uri, rename.uri);
    let original = fs::read_to_string(fixture.0.join("app/rust/controls.rs")).unwrap();
    let index = LineIndex::new(&original);
    let edits = [
        ("struct Card;", "struct "),
        ("ferro_class!(Card:", "ferro_class!("),
        ("ferro_class_info!(Card {", "ferro_class_info!("),
        ("FeaturedCard: Card)", "FeaturedCard: "),
        ("impl Card {", "impl "),
        ("register::<Card,", "register::<"),
        ("register_attached::<Card,", "register_attached::<"),
    ]
    .into_iter()
    .map(|(text, prefix)| {
        let start = original.find(text).unwrap() + prefix.len();
        TextEdit {
            range: index
                .range(SourceSpan::new(start, start + "Card".len()))
                .unwrap(),
            new_text: new.into(),
        }
    })
    .collect();
    let resolution = RustRenameResolution {
        rename: rename.clone(),
        documents: vec![DocumentEdits {
            uri: rust_uri.clone(),
            version: -1,
            edits,
        }],
    };
    let complete = service
        .complete_registered_type_rename(&uri, &request, new, &[], &base.review, &resolution)
        .unwrap();
    let rust = complete
        .documents
        .iter()
        .find(|document| document.edit.uri == rust_uri)
        .unwrap();
    assert!(rust.after.contains("struct OverviewCard;"));
    assert!(rust
        .after
        .contains("FerroUI.Designer.Rsx.v1:Customer.Views.OverviewCard"));
    assert!(rust
        .after
        .contains(r#""path": "crate::controls::cards::OverviewCard""#));
    assert!(rust.after.contains(r#"\"crate::builders::Card\""#));
    assert!(rust.after.contains(r#"("Unrelated", "Tone Card")"#));
    assert!(complete
        .documents
        .iter()
        .any(|document| document.after == VIEW.replace("Card", new)));
    fs::write(file_path(&rust_uri).unwrap(), &rust.after).unwrap();
    let rebuilt = rebuild(&fixture);
    assert!(rebuilt.types.iter().any(|class| class.name == new));
    let (_, descriptor) = rebuilt
        .metadata
        .iter()
        .find(|(key, _)| key.ends_with(".OverviewCard"))
        .unwrap();
    assert_eq!(
        RsxBuilderMapping::parse(descriptor).unwrap().path,
        "crate::controls::cards::OverviewCard"
    );
}
