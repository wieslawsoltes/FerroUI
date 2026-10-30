use super::*;

#[test]
fn property_registry_reads_and_authors_attached_aliases_without_crossing_namespaces() {
    let source = r#"<Grid xmlns="https://github.com/ferroui" xmlns:ui="https://github.com/ferroui" xmlns:app="urn:application"><Border Name="Card" ui:Grid.Row="{ui:Binding Row}" app:Grid.Row="Keep"><app:Owner.Background><app:Brush/></app:Owner.Background></Border></Grid>"#;
    let mut document = Document::new(source).unwrap();
    let card = document.ast().find_name("Card").unwrap();
    let registry = PropertyRegistry::built_in();
    let state = registry.state(&document, &[card], "Grid.Row").unwrap();
    assert!(state.expression);
    assert_eq!(state.value.as_deref(), Some("{ui:Binding Row}"));
    assert!(
        registry
            .plan(
                &document,
                &[card],
                "Grid.Row",
                Some("1"),
                ExpressionPolicy::PreserveExpressions
            )
            .is_err()
    );
    let edits = registry
        .plan(
            &document,
            &[card],
            "Grid.Row",
            Some("1"),
            ExpressionPolicy::ReplaceExpressions,
        )
        .unwrap();
    document.apply("Change cell", edits).unwrap();
    assert_eq!(
        document.ast().node(card).unwrap().attribute("ui:Grid.Row"),
        Some("1")
    );
    assert_eq!(
        document.ast().node(card).unwrap().attribute("app:Grid.Row"),
        Some("Keep")
    );
    assert!(
        document
            .ast()
            .node(card)
            .unwrap()
            .attribute("Grid.Row")
            .is_none()
    );
    let edits = registry
        .plan(
            &document,
            &[card],
            "Background",
            Some("Red"),
            ExpressionPolicy::ReplaceExpressions,
        )
        .unwrap();
    document.apply("Set native brush", edits).unwrap();
    assert!(document.source().contains("<app:Owner.Background>"));
    assert_eq!(
        document.ast().node(card).unwrap().attribute("Background"),
        Some("Red")
    );
}
