use crate::{
    project_navigation_tests::CatalogFixture, LanguageService, LineIndex, RegisteredPropertyRequest,
};
use ferroui_designer_project::{file_uri, Solution};

fn request(view: &str, anchor: &str) -> RegisteredPropertyRequest {
    RegisteredPropertyRequest {
        version: 1,
        position: LineIndex::new(view)
            .position(view.find(anchor).unwrap() + 1)
            .unwrap(),
    }
}

#[test]
fn reviews_typed_selector_members_and_preserves_decoy_words() {
    let fixture = CatalogFixture::create();
    let metadata = fixture.build();
    let view = r##"<Styles xmlns="https://github.com/ferroui" xmlns:ui="using:Customer.Views">
        <!-- 🧭 Tone and Badge -->
        <Style Selector="ui|FeaturedCard#Tone.Tone[To&#110;e=Tone]:not(ui|Card[Tone=Tone]) &gt; Button[Tag=Tone]"/>
        <Style Selector="ui|Card, ui|FeaturedCard">
            <Setter Property="Tone" Value="Tone"/>
            <Style Selector="^[Tone=Blue]"/>
        </Style>
        <Style Selector="Button[(ui|Card.Badge)=Badge]"/>
    </Styles>"##.replace('\n', "\r\n");
    let source = r#"<Styles xmlns="https://github.com/ferroui" xmlns:ui="using:Customer.Views">
        <Style>
            <Style.Selector>ui|Card[Tone=Purple]</Style.Selector>
            <Setter Property="Tone" Value="Purple"/>
        </Style>
        <Style Selector=".other[Tone=Yellow]"/>
    </Styles>"#;
    fixture.write("app/Styles.xaml", &view);
    fixture.write("app/UnopenedStyles.xaml", source);
    let uri = file_uri(fixture.0.join("app/Styles.xaml")).unwrap();
    let unopened = file_uri(fixture.0.join("app/UnopenedStyles.xaml")).unwrap();
    let mut service = LanguageService::default();
    service
        .index_solution(&Solution::discover(&fixture.0).unwrap())
        .unwrap();
    service.set_build_metadata(&metadata).unwrap();
    service.open(&uri, &view, 1).unwrap();

    let definition = service
        .definitions(&uri, view.find("To&#110;e").unwrap() + 1)
        .unwrap();
    assert_eq!(definition.len(), 1);
    assert!(definition[0].uri.ends_with("/app/rust/controls.rs"));
    let definition = service
        .definitions(&uri, view.find("FeaturedCard").unwrap() + 1)
        .unwrap();
    assert_eq!(definition.len(), 1);
    assert!(definition[0].uri.ends_with("/app/rust/controls.rs"));

    let request_tone = request(&view, "To&#110;e");
    let plan = service
        .plan_registered_property_rename(&uri, &request_tone, "Accent", None)
        .unwrap();
    assert_eq!(plan.unmapped.len(), 1, "{:?}", plan.unmapped);
    assert_eq!(plan.unmapped[0].uri, unopened);
    assert!(plan.documents.iter().any(|document| {
        document.after
            == view
                .replace("[To&#110;e=", "[Accent=")
                .replace("[Tone=", "[Accent=")
                .replace("Property=\"Tone\"", "Property=\"Accent\"")
    }));
    assert!(plan.documents.iter().any(|document| {
        document.after
            == source
                .replace("[Tone=Purple]", "[Accent=Purple]")
                .replace("Property=\"Tone\"", "Property=\"Accent\"")
    }));
    assert!(plan
        .documents
        .iter()
        .any(|document| document.after.contains("String>(\"Accent\",")));

    let attached = service
        .plan_registered_property_rename(&uri, &request(&view, "Badge)="), "Marker", None)
        .unwrap();
    assert!(attached.unmapped.is_empty());
    assert!(attached
        .documents
        .iter()
        .any(|document| document.after == view.replace("Card.Badge)", "Card.Marker)")));
    assert!(attached
        .documents
        .iter()
        .any(|document| document.after.contains("String>(\"Marker\",")));

    fixture.write(
        "app/UnopenedStyles.xaml",
        &source.replace(".other[Tone=Yellow]", "ui|Card[Accent=Yellow]"),
    );
    let captured = service
        .plan_registered_property_rename(&uri, &request_tone, "Accent", None)
        .unwrap_err();
    assert!(captured.0.contains("capture an existing"));
}
