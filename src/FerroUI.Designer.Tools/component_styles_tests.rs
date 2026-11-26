use super::*;

#[test]
fn applies_scoped_style_placement_and_reads_explicit_setters() {
    let original = r#"<Grid xmlns="https://github.com/ferroui"><Grid.Styles><Styles>
      <Style Selector="Border.card"><Style.Setters><Setter Property="Padding" Value="4" /></Style.Setters></Style>
    </Styles></Grid.Styles><Border Name="Card" /></Grid>"#;
    let mut document = Document::new(original).unwrap();
    let style = document
        .ast()
        .elements()
        .find(|node| node.local_name() == "Style")
        .unwrap()
        .id;
    assert_eq!(
        authored_setters(&document, style).unwrap()[0]
            .value
            .as_deref(),
        Some("4")
    );
    let root = document.ast().root();
    let edits = plan_style_change(
        &document,
        StyleChange::Create {
            owner: root,
            selector: "Border#Card",
        },
    )
    .unwrap();
    document.apply("Add scoped style", edits).unwrap();
    let new = document
        .ast()
        .elements()
        .find(|node| node.attribute("Selector") == Some("Border#Card"))
        .unwrap();
    assert_eq!(
        document
            .ast()
            .node(new.parent.unwrap())
            .unwrap()
            .local_name(),
        "Styles"
    );
    document.undo().unwrap();
    assert_eq!(document.source(), original);
}

#[test]
fn rejects_theme_cycles_and_accepts_an_explicit_scope_base() {
    let original = r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
      <Grid.Resources><ControlTheme x:Key="First" TargetType="Button" />
        <ControlTheme x:Key="Second" TargetType="Button" BasedOn="{StaticResource First}" />
        <ControlTheme x:Key="Third" TargetType="Button" />
      </Grid.Resources>
    </Grid>"#;
    let mut document = Document::new(original).unwrap();
    let theme = |key: &str| {
        document
            .ast()
            .elements()
            .find(|node| literal_key(node).as_deref() == Some(key))
            .unwrap()
            .id
    };
    let (first, second, third) = (theme("First"), theme("Second"), theme("Third"));
    assert!(plan_style_change(
        &document,
        StyleChange::SetBase {
            theme: first,
            base: Some(second)
        }
    )
    .is_err());
    let edits = plan_style_change(
        &document,
        StyleChange::SetBase {
            theme: third,
            base: Some(second),
        },
    )
    .unwrap();
    document.apply("Connect base theme", edits).unwrap();
    assert_eq!(base_theme(&document, third).unwrap(), Some(second));
    document.undo().unwrap();
    assert_eq!(document.source(), original);
}
