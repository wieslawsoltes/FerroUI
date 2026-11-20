use crate::{LanguageService, LineIndex, Location};
use ferroui_build_scan::{
    model::AssemblyModel,
    scanner::{scan_crate, ScanOptions},
};
use ferroui_designer_project::{file_uri, BuildMetadata, Solution};
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
    assert!(service
        .definitions_with_source(&uri, tone, |_| {
            Some(control_source.replace("\"Tone\"", "\"Tint\""))
        })
        .unwrap()
        .is_empty());
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
    assert!(service
        .definitions(&uri, foreign.find("dep:PostalAddress").unwrap() + 2)
        .unwrap()
        .is_empty());
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
