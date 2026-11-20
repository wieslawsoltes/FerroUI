use crate::{
    project_navigation_tests::{CatalogFixture, PLACE},
    DocumentEdits, LanguageService, LineIndex, RegisteredPropertyRequest, RustRenameResolution,
    TextEdit,
};
use ferroui_designer_core::SourceSpan;
use ferroui_designer_project::{file_uri, Solution};
use std::fs;

fn request(source: &str, needle: &str) -> RegisteredPropertyRequest {
    RegisteredPropertyRequest {
        version: 1,
        position: LineIndex::new(source)
            .position(source.find(needle).unwrap() + 1)
            .unwrap(),
    }
}

#[test]
fn a_literal_type_alias_changes_xaml_and_the_alias_only() {
    let fixture = CatalogFixture::create();
    let metadata = fixture.build();
    let view = r#"<TextBlock xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:d="using:Customer.Data"
        x:DataType="d:PostalAddress" Text="{CompiledBinding Street, DataType={x:Type d:PostalAddress}}"/>"#;
    let consumer = r#"<DataTemplate xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:dep="using:Customer.Data"
        DataType="{x:Type TypeName=dep:PostalAddress}">
        <TextBlock Text="PostalAddress" />
    </DataTemplate>"#;
    fixture.write("app/Alias.xaml", view);
    fixture.write("app/AliasConsumer.xaml", consumer);
    let uri = file_uri(fixture.0.join("app/Alias.xaml")).unwrap();
    let mut service = LanguageService::default();
    service
        .index_solution(&Solution::discover(&fixture.0).unwrap())
        .unwrap();
    service.set_build_metadata(&metadata).unwrap();
    service.open(&uri, view, 1).unwrap();
    let request = request(view, "PostalAddress");
    let plan = service
        .plan_registered_type_rename(&uri, &request, "MailingAddress", &[])
        .unwrap();
    assert_eq!(plan.name, "PostalAddress");
    assert!(plan.rust_rename.is_none());
    assert!(plan.unmapped.is_empty());
    assert_eq!(plan.documents.len(), 3);
    assert!(plan
        .documents
        .iter()
        .any(|document| document.after == PLACE.replace("PostalAddress", "MailingAddress")));
    assert!(plan
        .documents
        .iter()
        .any(|document| document.after
            == consumer.replace("dep:PostalAddress", "dep:MailingAddress")));
    assert!(plan
        .documents
        .iter()
        .any(|document| document.after == view.replace("d:PostalAddress", "d:MailingAddress")));
}

#[test]
fn a_control_name_updates_class_uses_and_requires_its_rust_declaration() {
    let fixture = CatalogFixture::create();
    let metadata = fixture.build();
    let view = r#"<StackPanel xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:a="using:Customer.Views">
        <a:Card a:Card.Tone="Red">
            <a:Card.Title>Card</a:Card.Title>
        </a:Card>
        <ContentControl Content="{x:Type a:Card}" />
        <Style Selector="a|Card.hot" />
    </StackPanel>"#;
    let code_behind = r#"<Border xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Customer.Views.Card" />"#;
    let style = r##"<Styles xmlns="https://github.com/ferroui" xmlns:ui="using:Customer.Views">
        <!-- 🧭 Card -->
        <Style Selector="ui|Card#Card.Card[Title=Card] &gt; :is(ui|Card):not(ui|Card[Title=Card]), Button[(ui|Card.Badge)=Card]"/>
        <Style>
            <Style.Selector>
                ui|C&#97;rd &gt; Button:not(ui|Card)
            </Style.Selector>
        </Style>
    </Styles>"##.replace('\n', "\r\n");
    let use_source = "use crate::controls::cards::Card;\npub fn inspect(_: &Card) {}\n";
    fixture.write("app/ControlRename.xaml", view);
    fixture.write("app/ClassConsumer.xaml", code_behind);
    fixture.write("app/SelectorConsumer.xaml", &style);
    fixture.write("app/other.rs", use_source);
    let uri = file_uri(fixture.0.join("app/ControlRename.xaml")).unwrap();
    let source_uri = file_uri(fixture.0.join("app/other.rs")).unwrap();
    let mut service = LanguageService::default();
    service
        .index_solution(&Solution::discover(&fixture.0).unwrap())
        .unwrap();
    service.set_build_metadata(&metadata).unwrap();
    service.open(&uri, view, 1).unwrap();
    let request = request(view, "Card.hot");
    let new_name = "SummaryCard";
    let base = service
        .plan_registered_type_rename(&uri, &request, new_name, &[])
        .unwrap();
    assert!(base.unmapped.is_empty());
    assert!(base.documents.iter().any(|document| document.after
        == view
            .replace("a:Card", "a:SummaryCard")
            .replace("a|Card", "a|SummaryCard")));
    assert!(base
        .documents
        .iter()
        .any(|document| document.after == code_behind.replace(".Card", ".SummaryCard")));
    assert!(base.documents.iter().any(|document| {
        document.after
            == style
                .replace("ui|Card", "ui|SummaryCard")
                .replace("ui|C&#97;rd", "ui|SummaryCard")
    }));
    let rust = base.rust_rename.as_ref().unwrap();
    assert!(rust.uri.ends_with("/app/rust/controls.rs"));
    assert_eq!(rust.old_name, "Card");
    let rust_text = fs::read_to_string(fixture.0.join("app/rust/controls.rs")).unwrap();
    let item = rust_text.find("struct Card").unwrap() + "struct ".len();
    assert_eq!(
        LineIndex::new(&rust_text).position(item).unwrap(),
        rust.position
    );
    let no_resolution = RustRenameResolution {
        rename: rust.clone(),
        documents: vec![],
    };
    assert!(service
        .complete_registered_type_rename(
            &uri,
            &request,
            new_name,
            &[],
            &base.review,
            &no_resolution
        )
        .unwrap_err()
        .0
        .contains("did not rename"));
    let edits = |source: &str, offsets: Vec<usize>| {
        let index = LineIndex::new(source);
        offsets
            .into_iter()
            .map(|offset| TextEdit {
                range: index.range(SourceSpan::new(offset, offset + 4)).unwrap(),
                new_text: new_name.into(),
            })
            .collect()
    };
    let resolution = RustRenameResolution {
        rename: rust.clone(),
        documents: vec![
            DocumentEdits {
                uri: rust.uri.clone(),
                version: -1,
                edits: edits(&rust_text, vec![item]),
            },
            DocumentEdits {
                uri: source_uri,
                version: -1,
                edits: edits(
                    use_source,
                    use_source
                        .match_indices("Card")
                        .map(|(offset, _)| offset)
                        .collect(),
                ),
            },
        ],
    };
    let complete = service
        .complete_registered_type_rename(&uri, &request, new_name, &[], &base.review, &resolution)
        .unwrap();
    assert!(complete.rust_rename.is_none());
    assert_ne!(complete.review, base.review);
    assert!(complete
        .documents
        .iter()
        .any(|document| document.after == use_source.replace("Card", new_name)));
    assert!(complete
        .documents
        .iter()
        .any(|document| document.after.contains("ferro_class!(SummaryCard: Border)")));
    assert!(service
        .plan_registered_type_rename(&uri, &request, "FeaturedCard", &[])
        .unwrap_err()
        .0
        .contains("already resolves"));
    fixture.write(
        "app/AddedRustReference.rs",
        "use crate::controls::cards::Card;",
    );
    assert!(service
        .complete_registered_type_rename(&uri, &request, new_name, &[], &base.review, &resolution)
        .unwrap_err()
        .0
        .contains("source changed"));
    fs::remove_file(fixture.0.join("app/AddedRustReference.rs")).unwrap();
    fixture.write(
        "app/ClassConsumer.xaml",
        &format!("<!-- updated 🧭 -->\r\n{code_behind}"),
    );
    assert!(service
        .complete_registered_type_rename(&uri, &request, new_name, &[], &base.review, &resolution)
        .unwrap_err()
        .0
        .contains("source changed"));
}
