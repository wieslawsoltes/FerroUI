use crate::{HandlerDraft, LanguageService, LineIndex, Location, RegisteredPropertyRequest};
use ferroui_build_scan::{
    model::AssemblyModel,
    scanner::{ScanOptions, scan_crate},
};
use ferroui_designer_project::{BuildMetadata, Solution, file_uri};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

const ROOT_MODEL: &str = r#"#[path = "rust/controls.rs"] mod controls;
pub mod data {
    pub struct Customer;
    ferro_markup_type!(struct Customer {
        namespace: "Customer.Views",
        properties: [Address: Option<::customer_data::Place> { get: get_address }],
    });
    pub struct Place;
    ferro_markup_type!(struct Place {
        namespace: "Customer.Views",
        properties: [Street: String { get: unrelated_street }],
    });
}"#;

const CONTROL: &str = r#"// Card 🧭 and Tone are mentioned here.
mod cards {
    pub struct Card;
    ferro_class!(Card: Border);
    ferro_class_info!(Card {
        markup: { namespace: "Customer.Views", properties: [Title: String { get: get_title }] },
    });
    pub struct FeaturedCard;
    ferro_class!(FeaturedCard: Card);
    ferro_class_info!(FeaturedCard { markup: { namespace: "Customer.Views" } });
    ferro_properties! {
        impl Card {
            pub fn tone_property() -> StyledProperty<String> {
                FerroProperty::register::<Card, String>("Tone", StyledPropertyMetadata::default())
            }
        }
    }
}"#;

const PLACE: &str = r#"pub struct Place;
ferro_markup_type!(struct Place as "PostalAddress" {
    namespace: "Customer.Data",
    properties: [Street: String { get: address_street }],
});"#;

const VIEW: &str = r#"<StackPanel xmlns="https://github.com/ferroui"
  xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
  xmlns:local="using:Customer.Views" x:DataType="local:Customer">
    <local:Card Title="Home" Tone="Blue"></local:Card>
    <TextBlock Text="{Binding Address.Street}" />
</StackPanel>"#;

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct CatalogFixture(PathBuf);

impl CatalogFixture {
    fn create() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ferroui-project-navigation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["app/rust", "dep/models", "output/app", "output/dep"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        Self(root.canonicalize().unwrap())
    }

    fn write(&self, path: &str, text: &str) {
        fs::write(self.0.join(path), text).unwrap();
    }

    fn build(&self) -> BuildMetadata {
        self.write(
            "app/Cargo.toml",
            "[package]\nname='customer-app'\nversion='0.1.0'\n[lib]\npath='lib.rs'\n",
        );
        self.write(
            "dep/Cargo.toml",
            "[package]\nname='customer-data'\nversion='0.1.0'\n[lib]\npath='lib.rs'\n",
        );
        self.write("app/lib.rs", ROOT_MODEL);
        self.write("app/rust/controls.rs", &CONTROL.replace('\n', "\r\n"));
        self.write("app/View.xaml", VIEW);
        self.write(
            "dep/lib.rs",
            "#[path=\"models/address.rs\"] mod addresses; pub use addresses::Place;",
        );
        self.write("dep/models/address.rs", PLACE);
        let dependency = self.export("dep", "customer_data", Vec::new());
        self.export("app", "customer_app", vec![dependency]);
        BuildMetadata {
            manifest: self.0.join("app/Cargo.toml"),
            directories: vec![self.0.join("output/app"), self.0.join("output/dep")],
            package: Some("customer-app".into()),
        }
    }

    fn export(&self, folder: &str, name: &str, dependencies: Vec<AssemblyModel>) -> AssemblyModel {
        let options = ScanOptions::new(name, self.0.join(folder).join("lib.rs"))
            .with_dependencies(dependencies)
            .with_authored_symbols();
        let mut scan = scan_crate(&options);
        scan.model.name = name.to_owned();
        let output = self.0.join("output").join(folder);
        scan.authored_symbols(&self.0.join(folder).join("Cargo.toml"))
            .write(&output)
            .unwrap();
        fs::write(
            output.join(format!("{name}.xamlmeta")),
            scan.model.to_json(),
        )
        .unwrap();
        scan.model
    }
}

impl Drop for CatalogFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[track_caller]
fn assert_location(
    definitions: Vec<Location>,
    expected_file: &Path,
    source: &str,
    expected_span: &str,
) {
    let [location] = definitions.as_slice() else {
        panic!("expected one Rust definition for {expected_span}, got {definitions:?}")
    };
    assert_eq!(location.uri, file_uri(expected_file).unwrap());
    let span = LineIndex::new(source).span(location.range).unwrap();
    assert_eq!(&source[span.range()], expected_span);
}

#[test]
fn definitions_follow_build_scoped_controls_and_cross_crate_binding_types() {
    let fixture = CatalogFixture::create();
    let metadata = fixture.build();
    let uri = file_uri(fixture.0.join("app/View.xaml")).unwrap();
    let mut service = LanguageService::default();
    service
        .index_solution(&Solution::discover(&fixture.0).unwrap())
        .unwrap();
    service.set_build_metadata(&metadata).unwrap();
    let definitions = |text: &str| {
        let caret = VIEW.find(text).unwrap() + if text.starts_with("</") { 2 } else { 1 };
        service.definitions(&uri, caret).unwrap()
    };
    let controls = fixture.0.join("app/rust/controls.rs");
    let control_source = fs::read_to_string(&controls).unwrap();
    assert_location(
        definitions("local:Card"),
        &controls,
        &control_source,
        "Card",
    );
    assert_location(
        definitions("</local:Card"),
        &controls,
        &control_source,
        "Card",
    );
    assert_location(definitions("Title="), &controls, &control_source, "Title");
    assert_location(
        definitions("Tone="),
        &controls,
        &control_source,
        "tone_property",
    );
    let tone = VIEW.find("Tone=").unwrap() + 1;
    assert!(
        service
            .definitions_with_source(&uri, tone, |_| {
                Some(control_source.replace("\"Tone\"", "\"Tint\""))
            })
            .unwrap()
            .is_empty()
    );
    assert_location(
        definitions("local:Customer"),
        &fixture.0.join("app/lib.rs"),
        ROOT_MODEL,
        "Customer",
    );
    assert_location(
        definitions("Address.Street"),
        &fixture.0.join("app/lib.rs"),
        ROOT_MODEL,
        "Address",
    );
    assert_location(
        definitions("Street}"),
        &fixture.0.join("dep/models/address.rs"),
        PLACE,
        "Street",
    );
    let binding = "CompiledBinding Street, DataType={x:Type dep:PostalAddress}";
    let view = VIEW
        .replace(
            "x:DataType=\"local:Customer\"",
            "xmlns:dep=\"using:Customer.Data\"",
        )
        .replace("Binding Address.Street", binding);
    service.open(&uri, &view, 1).unwrap();
    for anchor in ["Street,", "dep:PostalAddress"] {
        assert_location(
            service
                .definitions(&uri, view.find(anchor).unwrap() + 2)
                .unwrap(),
            &fixture.0.join("dep/models/address.rs"),
            PLACE,
            if anchor == "Street," {
                "Street"
            } else {
                "Place"
            },
        );
    }
    let foreign = view.replace("CompiledBinding Street", "local:CompiledBinding Street");
    service.open(&uri, &foreign, 2).unwrap();
    assert!(
        service
            .definitions(&uri, foreign.find("dep:PostalAddress").unwrap() + 2)
            .unwrap()
            .is_empty()
    );
    let code_behind = r#"<Border xmlns="https://github.com/ferroui"
        xmlns:q="http://schemas.microsoft.com/winfx/2006/xaml" q:Class="Customer.Views.Card"/>"#;
    service.open(&uri, code_behind, 3).unwrap();
    assert_location(
        service
            .definitions(&uri, code_behind.find("Views.Card").unwrap())
            .unwrap(),
        &controls,
        &control_source,
        "Card",
    );
}

#[test]
fn definitions_revalidate_the_authored_file_and_an_open_rust_draft() {
    let fixture = CatalogFixture::create();
    let metadata = fixture.build();
    let uri = file_uri(fixture.0.join("app/View.xaml")).unwrap();
    let mut service = LanguageService::default();
    service
        .index_solution(&Solution::discover(&fixture.0).unwrap())
        .unwrap();
    service.set_build_metadata(&metadata).unwrap();
    let offset = VIEW.find("Street}").unwrap() + 2;
    let file = fixture.0.join("dep/models/address.rs");
    let saved = format!("// moved externally 🪴\r\n{}", PLACE.replace('\n', "\r\n"));
    fs::write(&file, &saved).unwrap();
    assert_location(
        service.definitions(&uri, offset).unwrap(),
        &file,
        &saved,
        "Street",
    );
    let draft = format!("// open draft\r\n\r\n{saved}");
    assert_location(
        service
            .definitions_with_source(&uri, offset, |rust| {
                assert_eq!(rust, file_uri(&file).unwrap());
                Some(draft.clone())
            })
            .unwrap(),
        &file,
        &draft,
        "Street",
    );
    let definitions = service
        .definitions_with_source(&uri, offset, |_| Some(draft.replace("Street:", "Changed:")))
        .unwrap();
    assert!(definitions.is_empty());
}

#[test]
fn reviewed_rename_changes_the_registration_and_only_its_known_xaml_uses() {
    let fixture = CatalogFixture::create();
    let metadata = fixture.build();
    let origin = VIEW.replace("Title=\"Home\" ", "").replace('\n', "\r\n");
    let unopened = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:local="using:Customer.Views">
        <!-- Tone is ordinary text here 🧭 -->
        <local:Card><local:Card.Tone>Green</local:Card.Tone></local:Card>
        <local:FeaturedCard Tone="Inherited" />
        <local:FeaturedCard><local:Card.Tone>Inherited</local:Card.Tone></local:FeaturedCard>
        <TextBlock Text="{Binding Tone}" />
        <ControlTheme TargetType="local:Card">
            <Setter Property="Tone" Value="Pink" />
            <Setter Property="local:Card.Tone" Value="Red" />
        </ControlTheme>
    </StackPanel>"#;
    fixture.write("app/Unopened.xaml", unopened);
    let other = CatalogFixture::create();
    let other_build = other.build();
    let mut service = LanguageService::default();
    for (folder, build) in [(&fixture.0, &metadata), (&other.0, &other_build)] {
        service
            .index_solution(&Solution::discover(folder).unwrap())
            .unwrap();
        service.set_build_metadata(build).unwrap();
    }
    let uri = file_uri(fixture.0.join("app/View.xaml")).unwrap();
    service.open(&uri, &origin, 1).unwrap();
    let request = RegisteredPropertyRequest {
        version: 1,
        position: LineIndex::new(&origin)
            .position(origin.find("Tone=").unwrap() + 1)
            .unwrap(),
    };
    let target = service.registered_property_target(&uri, &request).unwrap();
    assert_eq!(target.name, "Tone");
    let saved = fs::read_to_string(fixture.0.join("app/rust/controls.rs")).unwrap();
    let draft = HandlerDraft {
        uri: target.rust_uri.clone(),
        version: 9,
        source: format!(
            "// unsaved line 🧭\r\n{}",
            saved.replace("\"Tone\"", "r#\"Tone\"#")
        ),
    };
    let plan = service
        .plan_registered_property_rename(&uri, &request, "Accent", Some(&draft))
        .unwrap();
    assert_eq!(plan.documents.len(), 3);
    let rust = plan
        .documents
        .iter()
        .find(|change| change.edit.uri == target.rust_uri)
        .unwrap();
    assert_eq!(
        rust.after,
        draft.source.replace("r#\"Tone\"#", "r#\"Accent\"#")
    );
    assert_eq!(rust.edit.version, 9);
    let view = plan
        .documents
        .iter()
        .find(|change| change.edit.uri == uri)
        .unwrap();
    assert_eq!(view.after, origin.replace("Tone=", "Accent="));
    let consumer = plan
        .documents
        .iter()
        .find(|change| change.edit.uri.ends_with("/Unopened.xaml"))
        .unwrap();
    assert_eq!(
        consumer.after,
        unopened
            .replace("Card.Tone", "Card.Accent")
            .replace("Property=\"Tone\"", "Property=\"Accent\"")
            .replace(" Tone=", " Accent=")
    );
    assert_eq!(plan.unmapped.len(), 1);
    assert!(plan.unmapped[0].uri.ends_with("/Unopened.xaml"));
    assert_eq!(
        plan.review,
        service
            .plan_registered_property_rename(&uri, &request, "Accent", Some(&draft))
            .unwrap()
            .review
    );
    fixture.write("app/Unopened.xaml", &unopened.replace("Green", "Orange"));
    assert_ne!(
        plan.review,
        service
            .plan_registered_property_rename(&uri, &request, "Accent", Some(&draft))
            .unwrap()
            .review
    );
    assert!(
        service
            .plan_registered_property_rename(&uri, &request, "Title", Some(&draft))
            .unwrap_err()
            .0
            .contains("already resolves")
    );
    let duplicate = HandlerDraft {
        source: draft.source.replace(
            "FerroProperty::register::<Card, String>(r#\"Tone\"#, StyledPropertyMetadata::default())",
            "if condition() { FerroProperty::register::<Card, String>(r#\"Tone\"#, meta()) } else { FerroProperty::register::<Card, String>(r#\"Tone\"#, meta()) }",
        ),
        ..draft
    };
    assert!(
        service
            .plan_registered_property_rename(&uri, &request, "Accent", Some(&duplicate))
            .unwrap_err()
            .0
            .contains("unique direct registration")
    );
}

#[test]
fn reviewed_binding_member_rename_follows_explicit_types_and_rejects_capture() {
    let fixture = CatalogFixture::create();
    let build = fixture.build();
    let mut service = LanguageService::default();
    let uri = file_uri(fixture.0.join("app/View.xaml")).unwrap();
    let mut explicit = r#"<TextBlock xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        xmlns:d="using:Customer.Data" Text="{CompiledBinding Street, DataType={x:Type d:PostalAddress}}"/>"#.to_owned();
    fixture.write("app/Explicit.xaml", &explicit);
    service
        .index_solution(&Solution::discover(&fixture.0).unwrap())
        .unwrap();
    service.set_build_metadata(&build).unwrap();
    service.open(&uri, VIEW, 1).unwrap();
    let request = RegisteredPropertyRequest {
        version: 1,
        position: LineIndex::new(VIEW)
            .position(VIEW.find("Street}").unwrap() + 2)
            .unwrap(),
    };
    let plan = service
        .plan_registered_property_rename(&uri, &request, "Line1", None)
        .unwrap();
    assert!(plan.unmapped.is_empty());
    assert_eq!(plan.documents.len(), 3);
    assert!(
        plan.documents
            .iter()
            .any(|change| change.after == PLACE.replace("Street:", "Line1:"))
    );
    assert!(
        plan.documents
            .iter()
            .any(|change| change.after == explicit.replace("Binding Street,", "Binding Line1,"))
    );
    assert!(
        plan.documents
            .iter()
            .any(|change| change.after == VIEW.replace("Address.Street", "Address.Line1"))
    );
    explicit = explicit.replace("Binding Street", "Binding Line1");
    fixture.write("app/Capture.xaml", &explicit);
    assert!(
        service
            .plan_registered_property_rename(&uri, &request, "Line1", None)
            .unwrap_err()
            .0
            .contains("would capture")
    );
    fs::remove_file(fixture.0.join("app/Capture.xaml")).unwrap();
    let explicit_uri = file_uri(fixture.0.join("app/Explicit.xaml")).unwrap();
    service
        .open(&explicit_uri, "<TextBlock Text=\"", 1)
        .unwrap();
    assert!(
        service
            .plan_registered_property_rename(&uri, &request, "Line1", None)
            .unwrap_err()
            .0
            .contains("incomplete XAML")
    );
    let stale = RegisteredPropertyRequest {
        version: 0,
        ..request
    };
    assert!(
        service
            .plan_registered_property_rename(&uri, &stale, "Line1", None)
            .unwrap_err()
            .0
            .contains("source changed")
    );
}
