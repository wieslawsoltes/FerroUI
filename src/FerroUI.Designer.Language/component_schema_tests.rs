use crate::{LanguageService, Schema, TypeInfo};

const URI: &str = "untitled:/Components.xaml";
const TEMPLATES: &str = "using:FerroUI.Markup.Xaml.Templates";

fn service(source: &str) -> LanguageService {
    let mut service = LanguageService::default();
    service.open(URI, source, 1).unwrap();
    service
}

#[test]
fn native_markup_metadata_covers_templates_resources_and_inherited_members() {
    let schema = Schema::ferroui();
    for name in [
        "ControlTemplate",
        "DataTemplate",
        "TreeDataTemplate",
        "ItemsPanelTemplate",
        "Template",
        "FocusAdornerTemplate",
        "WindowDrawnDecorationsTemplate",
        "ControlTheme",
        "ResourceDictionary",
        "ResourceInclude",
        "MergeResourceInclude",
        "StyleInclude",
        "StaticResourceExtension",
        "DynamicResourceExtension",
    ] {
        assert!(
            schema.type_info(name, None).unwrap().constructible,
            "{name}"
        );
    }
    for (owner, property) in [
        ("ControlTemplate", "TargetType"),
        ("DataTemplate", "DataType"),
        ("TreeDataTemplate", "ItemsSource"),
        ("ItemsPanelTemplate", "Content"),
        ("FocusAdornerTemplate", "Content"),
        ("ControlTheme", "BasedOn"),
        ("ControlTheme", "Resources"),
        ("ControlTheme", "Setters"),
        ("ResourceDictionary", "MergedDictionaries"),
        ("ResourceDictionary", "ThemeDictionaries"),
        ("MergeResourceInclude", "Source"),
        ("StaticResource", "ResourceKey"),
        ("DynamicResource", "ResourceKey"),
        ("OnPlatform", "macOS"),
        ("OnPlatform", "iOS"),
    ] {
        assert!(
            schema.property(owner, None, property).is_some(),
            "{owner}.{property}"
        );
    }
}

#[test]
fn component_documents_accept_native_aliases_owner_qualified_members_and_object_resources() {
    let source = r#"<UserControl xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        xmlns:t="using:FerroUI.Markup.Xaml.Templates"
        xmlns:c="clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
        xmlns:m="using:FerroUI.Markup.Xaml.MarkupExtensions">
      <UserControl.Resources>
        <SolidColorBrush x:Key="Accent" Color="Blue"/>
        <t:ControlTemplate x:Key="Action" t:ControlTemplate.TargetType="{x:Type c:Button}">
          <Border Background="{TemplateBinding Background}"><ContentPresenter/></Border>
        </t:ControlTemplate>
        <ControlTheme x:Key="ActionTheme" ControlTheme.TargetType="c:Button">
          <Setter Setter.Property="Template" Setter.Value="{StaticResource Action}"/>
          <Setter Property="Background"><m:StaticResource m:StaticResource.ResourceKey="Accent"/></Setter>
        </ControlTheme>
        <DataTemplate x:Key="Label" DataTemplate.DataType="c:Button"><TextBlock Text="{Binding Content}"/></DataTemplate>
      </UserControl.Resources>
      <Button Content="Action" Theme="{StaticResource ActionTheme}"/>
    </UserControl>"#;
    let service = service(source);
    let diagnostics = service.diagnostics(URI).unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let hover = service
        .hover(URI, source.find("t:ControlTemplate.TargetType").unwrap())
        .unwrap()
        .unwrap();
    assert!(hover.contents.contains("TargetType"));
}

#[test]
fn builtin_aliases_require_the_declaring_namespace_and_assembly() {
    let mut schema = Schema::ferroui();
    // `ferro_markup_type! ... as` exposes the authored name, not its Rust carrier.
    assert!(schema.type_info("Type", Some("using:System")).is_some());
    assert!(schema.type_info("TypeInfo", Some("using:System")).is_none());
    for namespace in [
        TEMPLATES,
        "clr-namespace:FerroUI.Markup.Xaml.Templates;assembly=FerroUI.Markup.Xaml",
    ] {
        assert_eq!(
            schema
                .type_info("t:ControlTemplate", Some(namespace))
                .unwrap()
                .name,
            "ControlTemplate"
        );
    }
    for namespace in [
        "using:FerroUI.Controls",
        "using:FerroUI.Application",
        "clr-namespace:FerroUI.Markup.Xaml.Templates;assembly=Other",
    ] {
        assert!(
            schema
                .type_info("ControlTemplate", Some(namespace))
                .is_none(),
            "{namespace}"
        );
    }
    schema.register(TypeInfo {
        name: "Button".into(),
        namespace: "using:FerroUI.Application".into(),
        documentation: "Application-specific Button".into(),
        constructible: true,
        ..Default::default()
    });
    assert_eq!(
        schema
            .type_info("Button", Some("using:FerroUI.Application"))
            .unwrap()
            .documentation,
        "Application-specific Button"
    );
    assert!(
        schema
            .type_info("Grid", Some("using:FerroUI.Application"))
            .is_none()
    );
}

#[test]
fn qualified_completion_uses_exact_native_namespaces() {
    let source = format!("<Grid xmlns:t='{TEMPLATES}'><t:C");
    let service = service(&source);
    let completions = service.completions(URI, source.len()).unwrap();
    assert_eq!(
        completions
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["t:ControlTemplate"]
    );
    let source = format!("<t:ControlTemplate xmlns:t='{TEMPLATES}' Tar");
    let service = self::service(&source);
    assert!(
        service
            .completions(URI, source.len())
            .unwrap()
            .iter()
            .any(|item| item.label == "TargetType")
    );
}

#[test]
fn owner_qualified_properties_respect_target_inheritance_and_attached_namespaces() {
    let schema = Schema::ferroui();
    assert!(schema.property("Button", None, "Control.Width").is_some());
    assert!(
        schema
            .property("Button", None, "ControlTemplate.TargetType")
            .is_none()
    );
    assert!(
        schema
            .property("Button", Some("using:FerroUI.Controls"), "Grid.Row")
            .is_some()
    );
    assert!(
        schema
            .property("Button", Some("using:FerroUI.Application"), "Grid.Row")
            .is_none()
    );
    let source = r#"<app:Button xmlns="https://github.com/ferroui" xmlns:app="using:Custom" Grid.Row="1" app:Button.Orientation="Vertical"/>"#;
    let mut service = service(source);
    service.schema_mut().register(TypeInfo {
        name: "Button".into(),
        namespace: "using:Custom".into(),
        base: Some("StackPanel".into()),
        constructible: true,
        ..Default::default()
    });
    assert!(service.diagnostics(URI).unwrap().is_empty());
    assert!(
        service
            .hover(URI, source.find("Grid.Row").unwrap())
            .unwrap()
            .unwrap()
            .contents
            .contains("Row")
    );
    assert!(
        service
            .completions(URI, source.find("Vertical").unwrap() + 1)
            .unwrap()
            .iter()
            .any(|item| item.label == "Vertical")
    );
}

#[test]
fn binding_members_follow_authored_template_target_and_data_type_aliases() {
    for (template, property, expression) in [
        (
            "ControlTemplate",
            "TargetType",
            "{Binding Cont, RelativeSource={RelativeSource TemplatedParent}}",
        ),
        ("DataTemplate", "DataType", "{Binding Cont}"),
        ("TreeDataTemplate", "DataType", "{Binding Cont}"),
    ] {
        let source = format!(
            r#"<t:{template} xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:t="{TEMPLATES}" xmlns:c="using:FerroUI.Controls" t:{template}.{property}="{{x:Type TypeName=c:Button}}"><TextBlock Text="{expression}"/></t:{template}>"#
        );
        let offset = source.find("Binding Cont").unwrap() + "Binding Cont".len();
        let service = service(&source);
        assert!(
            service
                .completions(URI, offset)
                .unwrap()
                .iter()
                .any(|item| item.label == "Content"),
            "{template}"
        );
        let foreign_source = source.replace(
            "http://schemas.microsoft.com/winfx/2006/xaml",
            "using:Custom",
        );
        let foreign_offset = foreign_source.find("Binding Cont").unwrap() + "Binding Cont".len();
        let foreign_type = self::service(&foreign_source);
        // A foreign extension named Type must not invent a binding source type.
        assert!(
            !foreign_type
                .completions(URI, foreign_offset)
                .unwrap()
                .iter()
                .any(|item| item.label == "Content"),
            "{template}"
        );
    }
}
