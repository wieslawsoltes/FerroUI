use crate::{HandlerDraft, HandlerRequest, LanguageService, LineIndex};
use ferroui_build_scan::model::{
    AssemblyModel, MemberModel, ParameterModel, RustType, TypeKind, TypeModel,
};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_project::{
    BuildMetadata, CargoTarget, DocumentEntry, DocumentKind, ProjectDescriptor, Solution, file_uri,
};
use std::fs;
use std::path::PathBuf;

const RUST: &str = "// 🦀 Keep this module\r\nmod page {\r\n    struct Settings;\r\n    ferro_class!(Settings: UserControl);\r\n    ferro_class_info!(Settings {\r\n        new: Settings::new,\r\n        markup: {\r\n            namespace: \"Audit\",\r\n            methods: [\r\n                fn Existing() => |_| {} // Keep this method\r\n            ],\r\n        },\r\n    });\r\n}\r\n";
const XAML: &str = concat!(
    "<UserControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' ",
    "xmlns:d='using:Audit' x:Class='Audit.Settings'>\r\n  <!-- 🦀 --> <Button x:Name='ApplyButton' Content='Keep me' />\r\n",
    "  <d:Probe StatusChanged='OnStatus' />\r\n</UserControl>\r\n"
);

#[test]
fn handler_scaffolds_routed_events_in_the_registered_inline_module() {
    let (fixture, service) = Fixture::open(XAML, RUST);
    let request = fixture.request(XAML, "Button x:Name", Some("Click"));
    let target = service.handler_target(&fixture.xaml, &request).unwrap();
    assert_eq!(target.suggested_name, "OnApplyButtonClick");
    assert_eq!(target.declaring_type, "Button");
    assert!(target.parameter_types[1].contains("IRoutedEventArgs"));
    let draft = fixture.draft(&RUST.replace("Keep this module", "An unsaved 📝 edit"));
    let plan = service
        .plan_handler(&fixture.xaml, &request, &draft, "OnApplyButtonClick")
        .unwrap();
    let [markup, code] = plan.edit.document_changes.as_slice() else {
        panic!()
    };
    assert_eq!(markup.version, 7);
    assert_eq!(code.version, 31);
    let xaml = apply(XAML, &markup.edits);
    let rust = apply(&draft.source, &code.edits);
    assert!(xaml.contains("Click='OnApplyButtonClick'"));
    assert_eq!(xaml.replace(" Click='OnApplyButtonClick'", ""), XAML);
    assert!(rust.contains("fn Existing() => |_| {}, // Keep this method\r\n"));
    assert!(rust.contains("An unsaved 📝 edit"));
    assert!(!rust.replace("\r\n", "").contains('\n'));
    let index = LineIndex::new(&rust);
    assert_eq!(
        &rust[index.offset(plan.declaration.range.start).unwrap()
            ..index.offset(plan.declaration.range.end).unwrap()],
        "OnApplyButtonClick"
    );
    assert!(
        service
            .plan_handler(
                &fixture.xaml,
                &request,
                &fixture.draft(&rust),
                "OnApplyButtonClick"
            )
            .unwrap_err()
            .0
            .contains("already registered")
    );
}

#[test]
fn existing_literal_is_preserved_and_unknown_or_stale_edits_are_rejected() {
    let (fixture, service) = Fixture::open(XAML, RUST);
    let mut request = fixture.request(XAML, "OnStatus", None);
    let draft = fixture.draft(RUST);
    let target = service.handler_target(&fixture.xaml, &request).unwrap();
    assert_eq!(target.parameter_types, ["bool"]);
    assert_eq!(target.existing_name.as_deref(), Some("OnStatus"));
    let plan = service
        .plan_handler(&fixture.xaml, &request, &draft, "OnStatus")
        .unwrap();
    assert!(plan.edit.document_changes[0].edits.is_empty());
    let changed = apply(RUST, &plan.edit.document_changes[1].edits);
    assert!(changed.contains("fn OnStatus(bool)"));
    assert_eq!(
        service
            .handler_definition(&fixture.xaml, &request, Some(&fixture.draft(&changed)))
            .unwrap()
            .unwrap()
            .range,
        plan.declaration.range
    );
    assert!(
        service
            .plan_handler(&fixture.xaml, &request, &draft, "Other")
            .is_err()
    );
    request.version -= 1;
    assert!(
        service
            .plan_handler(&fixture.xaml, &request, &draft, "OnStatus")
            .unwrap_err()
            .0
            .contains("XAML buffer changed")
    );
    let click = fixture.request(XAML, "Button x:Name", Some("Gone"));
    assert!(service.handler_target(&fixture.xaml, &click).is_err());
    let click = fixture.request(XAML, "Button x:Name", Some("Click"));
    let mut other = fixture.draft(RUST);
    other.uri = fixture.xaml.clone();
    assert!(
        service
            .plan_handler(&fixture.xaml, &click, &other, "OnApply")
            .is_err()
    );
    let (fixture, service) = Fixture::open(&XAML.replace("OnStatus", "{Binding Current}"), RUST);
    let request = fixture.request(
        &XAML.replace("OnStatus", "{Binding Current}"),
        "StatusChanged",
        None,
    );
    assert!(
        service
            .handler_target(&fixture.xaml, &request)
            .unwrap_err()
            .0
            .contains("event expression")
    );
}

#[test]
fn a_qualified_routed_event_keeps_its_handler_and_blocks_duplicate_scaffolds() {
    let source = XAML.replace(
        "Content='Keep me'",
        "Button.Click='OnQualified' Content='Keep me'",
    );
    let (fixture, service) = Fixture::open(&source, RUST);
    let duplicate = fixture.request(&source, "Button x:Name", Some("Click"));
    assert!(
        service
            .handler_target(&fixture.xaml, &duplicate)
            .unwrap_err()
            .0
            .contains("already has")
    );
    let qualified = fixture.request(&source, "OnQualified", None);
    let edit = service
        .plan_handler(
            &fixture.xaml,
            &qualified,
            &fixture.draft(RUST),
            "OnQualified",
        )
        .unwrap();
    assert!(edit.edit.document_changes[0].edits.is_empty());
    assert!(apply(RUST, &edit.edit.document_changes[1].edits).contains("fn OnQualified("));
}

fn apply(source: &str, edits: &[crate::TextEdit]) -> String {
    let index = LineIndex::new(source);
    TextPatch::apply_all(
        source,
        edits
            .iter()
            .map(|edit| TextPatch {
                span: SourceSpan::new(
                    index.offset(edit.range.start).unwrap(),
                    index.offset(edit.range.end).unwrap(),
                ),
                replacement: edit.new_text.clone(),
            })
            .collect(),
    )
    .unwrap()
}

struct Fixture {
    dir: PathBuf,
    xaml: String,
    rust: String,
}

impl Fixture {
    fn open(xaml: &str, rust: &str) -> (Self, LanguageService) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ferroui-handlers-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(dir.join("out")).unwrap();
        let dir = dir.canonicalize().unwrap();
        let xaml_path = dir.join("Settings.xaml");
        let rust_path = dir.join("lib.rs");
        let manifest = dir.join("Cargo.toml");
        fs::write(&manifest, "[package]\nname='audit-app'\n").unwrap();
        fs::write(&xaml_path, xaml).unwrap();
        fs::write(&rust_path, rust).unwrap();

        let mut base = AssemblyModel::new("FerroUI.Base", "ferroui_base");
        let mut framework = AssemblyModel::new("FerroUI.Controls", "ferroui_controls");
        let mut own = AssemblyModel::new("Audit", "audit_app");
        let mut interactive = class(
            "Interactive",
            "ferroui_base::interactivity",
            "FerroUI.Interactivity",
        );
        interactive.class_markup = true;
        let mut button = class("Button", "ferroui_controls", "FerroUI.Controls");
        button.base = Some(interactive.rust_path.clone());
        button.fields.push(MemberModel { name: "ClickEvent".into(), is_static: true,
            return_type: Some(RustType::resolved("::ferroui_base::interactivity::RoutedEvent<::ferroui_base::interactivity::RoutedEventArgs>")), ..MemberModel::default() });
        let mut settings = class("Settings", "audit_app::page", "Audit");
        settings.explicit_namespace = Some("Audit".into());
        let mut probe = class("Probe", "audit_app", "Audit");
        probe.events.push(MemberModel {
            name: "StatusChanged".into(),
            parameters: vec![ParameterModel {
                type_: RustType::resolved("bool"),
                ..ParameterModel::default()
            }],
            ..MemberModel::default()
        });
        framework.types.push(button);
        framework
            .xmlns_definitions
            .push(ferroui_build_scan::model::XmlnsDefinitionModel {
                xml_namespace: "https://github.com/ferroui".into(),
                namespace: "FerroUI.Controls".into(),
            });
        base.types.push(interactive);
        own.types.extend([settings, probe]);
        for model in [base, framework, own] {
            fs::write(
                dir.join("out")
                    .join(format!("{}.xamlmeta", model.crate_name)),
                model.to_json(),
            )
            .unwrap();
        }

        let fixture = Self {
            dir: dir.clone(),
            xaml: file_uri(&xaml_path).unwrap(),
            rust: file_uri(&rust_path).unwrap(),
        };
        let mut service = LanguageService::default();
        service
            .index_solution(&Solution {
                format_version: 1,
                root: dir.clone(),
                projects: vec![ProjectDescriptor {
                    id: "audit-app".into(),
                    name: "audit-app".into(),
                    root: dir.clone(),
                    manifest_path: manifest.clone(),
                    documents: vec![DocumentEntry {
                        uri: fixture.xaml.clone(),
                        path: xaml_path,
                        kind: DocumentKind::Xaml,
                    }],
                    targets: vec![CargoTarget {
                        name: "audit_app".into(),
                        kind: vec!["lib".into()],
                        source: rust_path,
                    }],
                }],
            })
            .unwrap();
        service
            .set_build_metadata(&BuildMetadata {
                manifest,
                directories: vec![dir.join("out")],
                package: Some("audit-app".into()),
            })
            .unwrap();
        service.open(&fixture.xaml, xaml, 7).unwrap();
        (fixture, service)
    }

    fn request(&self, source: &str, at: &str, event: Option<&str>) -> HandlerRequest {
        HandlerRequest {
            version: 7,
            event: event.map(str::to_owned),
            position: LineIndex::new(source)
                .position(source.find(at).unwrap())
                .unwrap(),
        }
    }

    fn draft(&self, source: &str) -> HandlerDraft {
        HandlerDraft {
            uri: self.rust.clone(),
            version: 31,
            source: source.into(),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn class(name: &str, module: &str, namespace: &str) -> TypeModel {
    let mut class = TypeModel::new(
        name,
        TypeKind::Class,
        RustType::resolved(&format!("::{module}::{name}")),
        module,
    );
    class.namespace = namespace.into();
    class.object_model = true;
    class
}
