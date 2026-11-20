use super::*;
use crate::components::{ComponentResourceContext, resolve_component_resource_in_context};

const NS: &str =
    r#"xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml""#;

#[test]
fn create_dark_layer_inside_implicit_resources_preserves_trivia_and_undo() {
    let original = format!(
        "<Grid {NS}>\r\n  <Grid.Resources>\r\n    <!-- keep 🧩 -->\r\n    <Color x:Key='Shared'>#FFFF0000</Color>\r\n  </Grid.Resources>\r\n  <Border Name='Card' Background='{{DynamicResource Accent}}'/>\r\n</Grid>"
    );
    let mut document = Document::new(&original).unwrap();
    let root = document.ast().root();
    let edits = plan_add_palette_color(
        &document,
        root,
        PaletteLayer::Dark,
        "Accent",
        PaletteColorKind::SolidColorBrush,
        "#FF335577",
    )
    .unwrap();
    document.apply("Add dark color", edits).unwrap();
    assert!(
        document.source().contains(
            "\r\n    <!-- keep 🧩 -->\r\n    <Color x:Key='Shared'>#FFFF0000</Color>\r\n"
        )
    );
    assert!(!document.source().replace("\r\n", "").contains('\n'));
    let dark = &palette_layers(&document, root).unwrap()[3].colors[0];
    assert_eq!(dark.key, "Accent");
    assert_eq!(dark.value.as_deref(), Some("#FF335577"));
    let card = document.ast().find_name("Card").unwrap();
    for (active, resolved) in [("Dark", Some(dark.node)), ("Light", None)] {
        assert_eq!(
            resolve_component_resource_in_context(
                &document,
                card,
                "Accent",
                &ComponentResourceContext {
                    theme: Some(active.into()),
                    inherited_themes: vec![]
                }
            ),
            resolved
        );
    }
    assert!(
        plan_add_palette_color(
            &document,
            root,
            PaletteLayer::Dark,
            "Accent",
            PaletteColorKind::Color,
            "#FFBBCCE0"
        )
        .is_err()
    );
    document.undo().unwrap();
    assert_eq!(document.source(), original);
}

#[test]
fn aliased_variants_and_source_expressions_keep_their_identities() {
    let mut document = Document::new(format!(r#"<Grid {NS} xmlns:d="using:FerroUI.Controls" xmlns:s="using:FerroUI.Styling">
    <Grid.Resources><d:ResourceDictionary>
      <d:ResourceDictionary.ThemeDictionaries>
        <d:ResourceDictionary x:Key="{{x:Static s:ThemeVariant.Dark}}">
          <SolidColorBrush x:Key="Bound" Color="{{DynamicResource Example}}"/>
          <SolidColorBrush x:Key="Editable"><SolidColorBrush.Color>  #aabbcc  <!-- note --></SolidColorBrush.Color></SolidColorBrush>
        </d:ResourceDictionary>
        <ResourceInclude x:Key="Light" Source="Outside.xaml"/>
      </d:ResourceDictionary.ThemeDictionaries>
    </d:ResourceDictionary></Grid.Resources>
    </Grid>"#)).unwrap();
    let root = document.ast().root();
    let layers = palette_layers(&document, root).unwrap();
    assert!(layers[2].unavailable.is_some());
    assert_eq!(layers[3].colors.len(), 2);
    assert!(plan_palette_color(&document, layers[3].colors[0].node, "#FFDDEEFF").is_err());
    let edits = plan_palette_color(&document, layers[3].colors[1].node, "#FFDDEEFF").unwrap();
    document.apply("Change palette color", edits).unwrap();
    assert!(document.source().contains("  #FFDDEEFF  <!-- note -->"));
    let edits = plan_add_palette_color(
        &document,
        root,
        PaletteLayer::Default,
        "A & B",
        PaletteColorKind::Color,
        "#FFDDEEFF",
    )
    .unwrap();
    document.apply("Add default", edits).unwrap();
    assert_eq!(
        palette_layers(&document, root).unwrap()[1].colors[0].key,
        "A & B"
    );
    assert!(
        document
            .source()
            .contains(r#"x:Key="{x:Static s:ThemeVariant.Dark}""#)
    );
}
