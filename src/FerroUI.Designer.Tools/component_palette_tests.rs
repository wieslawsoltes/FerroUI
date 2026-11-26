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

#[test]
fn contrast_palette_uses_the_authored_provider_and_dark_fallback() {
    let original = format!(
        r##"<Grid {NS} xmlns:contrast="using:Application">
  <Grid.Resources><ResourceDictionary>
    <ResourceDictionary.ThemeDictionaries>
      <ResourceDictionary x:Key="Default"><Color x:Key="Accent">#FF808080</Color></ResourceDictionary>
      <ResourceDictionary x:Key="Dark"><Color x:Key="Accent">#FF2080A0</Color></ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
  </ResourceDictionary></Grid.Resources>
  <Border Name="Card" Background="{{DynamicResource Accent}}"/>
</Grid>"##
    );
    let mut document = Document::new(&original).unwrap();
    let owner = document.ast().root();
    let options = (
        PaletteLayer::HighContrast,
        PaletteColorKind::Color,
        "#FFFFCC33",
    );
    let resolve = |document: &Document| {
        let card = document.ast().find_name("Card").unwrap();
        resolve_palette_color(document, card, "Accent", PaletteLayer::HighContrast)
            .and_then(|entry| entry.value)
    };
    assert_eq!(resolve(&document).as_deref(), Some("#FF2080A0"));
    assert!(
        plan_add_palette_color(&document, owner, options.0, "Accent", options.1, options.2)
            .is_err()
    );
    let edits = plan_add_palette_color_with_provider(
        &document,
        owner,
        options.0,
        "Accent",
        options.1,
        options.2,
        Some(HighContrastProvider::WinUi),
    )
    .unwrap();
    document.apply("Add High contrast color", edits).unwrap();
    assert!(document
        .source()
        .contains("xmlns:contrast1=\"using:FerroUI.Themes.WinUI\""));
    assert!(document
        .source()
        .contains("{x:Static contrast1:WinUITheme.HighContrast}"));
    assert_eq!(resolve(&document).as_deref(), Some(options.2));
    let layers = palette_layers(&document, owner).unwrap();
    assert!(layers[4].declaration.is_some());
    assert_eq!(layers[4].colors[0].value.as_deref(), Some(options.2));
    document.undo().unwrap();
    assert_eq!(document.source(), original);
}

#[test]
fn contrast_provider_keys_require_native_namespaces() {
    for (namespace, class, expected) in [
        (
            "using:FerroUI.Themes.Material",
            "MaterialTheme",
            Some("HighContrast"),
        ),
        (
            "clr-namespace:FerroUI.Themes.MacOS;assembly=FerroUI.Themes.MacOS",
            "MacOSTheme",
            Some("HighContrast"),
        ),
        ("using:Application", "WinUITheme", None),
        (
            "clr-namespace:FerroUI.Themes.WinUI;assembly=Application",
            "WinUITheme",
            None,
        ),
    ] {
        let markup = format!("<ResourceDictionary {NS} xmlns:t='{namespace}' x:Key='{{x:Static t:{class}.HighContrast}}'/>");
        let source = Document::new(markup).unwrap();
        assert_eq!(
            references::theme_key(&source, source.ast().node(source.ast().root()).unwrap())
                .as_deref(),
            expected
        );
    }
}
