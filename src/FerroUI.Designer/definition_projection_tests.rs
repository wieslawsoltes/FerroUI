use super::*;
use crate::{DesignerSession, PreviewStatus};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, ContentControl, Control, Window};

const SOURCE: &str = r##"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Width="640" Height="400">
  <UserControl.Resources>
    <SolidColorBrush x:Key="Accent" Color="#FF1278CC" />
    <ControlTheme x:Key="CardTheme" TargetType="Button">
      <Setter Property="Template"><ControlTemplate>
        <Border Name="ThemeChrome" Background="{StaticResource Accent}" Padding="12">
          <StackPanel Name="ThemeStack" Spacing="8">
            <TextBlock Name="ThemeTitle" Text="Title" />
            <ContentPresenter Name="PART_ContentPresenter" Content="{TemplateBinding Content}" />
          </StackPanel>
        </Border>
      </ControlTemplate></Setter>
    </ControlTheme>
    <ControlTemplate x:Key="BareTemplate"><Border Name="TemplateChrome" Background="{StaticResource Accent}" /></ControlTemplate>
    <DataTemplate x:Key="ItemTemplate"><Border Name="ItemChrome" Padding="10"><TextBlock Text="{Binding}" /></Border></DataTemplate>
  </UserControl.Resources>
  <StackPanel><Button Name="Instance" Content="Account" Theme="{StaticResource CardTheme}" />
    <ContentControl Content="Sample item" ContentTemplate="{StaticResource ItemTemplate}" /></StackPanel>
</UserControl>"##;

fn definition(document: &Document, key: &str) -> NodeId {
    ComponentCatalog::from_document(document)
        .unwrap()
        .entries
        .into_iter()
        .find(|entry| entry.key.as_deref() == Some(key))
        .unwrap()
        .node
}

#[test]
fn projected_resource_scopes_use_expanded_property_identity_and_qualified_target_types() {
    let source = r#"<Grid xmlns="https://github.com/ferroui" xmlns:ui="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:app="using:Example"><app:Other.Resources><SolidColorBrush x:Key="Unrelated" Color="Red"/></app:Other.Resources><ui:Grid.Resources><ControlTheme x:Key="Action" ui:ControlTheme.TargetType="{x:Type TextBox}"><Setter Property="Template"><ControlTemplate><Border/></ControlTemplate></Setter></ControlTheme></ui:Grid.Resources></Grid>"#;
    let document = Document::new(source).unwrap();
    let theme = definition(&document, "Action");
    let target = target_type(&document, document.ast().node(theme).unwrap(), None).unwrap();
    assert_eq!(target, (FRAMEWORK.into(), "TextBox".into()));
    let projection = project(
        &document,
        &scope_for(&document, theme, Default::default(), vec![]).unwrap(),
    )
    .unwrap();
    assert!(!projection.document.source().contains("Unrelated"));
    assert!(projection.document.source().contains("<dt:TextBox"));
    let custom = Document::new(source.replace("{x:Type TextBox}", "{app:Type TextBox}")).unwrap();
    let theme = definition(&custom, "Action");
    assert!(target_type(&custom, custom.ast().node(theme).unwrap(), None).is_err());
}

#[test]
fn dictionary_preview_does_not_sample_resources_private_to_a_theme() {
    let source = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><ControlTheme x:Key="Action" TargetType="Button"><ControlTheme.Resources><SolidColorBrush x:Key="PrivateBrush" Color="Red"/></ControlTheme.Resources><Setter Property="Background" Value="{StaticResource PrivateBrush}"/></ControlTheme><SolidColorBrush x:Key="PublicBrush" Color="Blue"/></ResourceDictionary>"#;
    let document = Document::new(source).unwrap();
    let scope = scope_for(&document, document.ast().root(), Default::default(), vec![]).unwrap();
    let projection = project(&document, &scope).unwrap();
    assert!(
        projection.document.source().contains("PrivateBrush"),
        "Private resources remain inside their authored theme"
    );
    assert!(
        !projection
            .document
            .source()
            .contains("Text=\"PrivateBrush\""),
        "A private key is not a top-level gallery item"
    );
    assert!(
        projection
            .document
            .source()
            .contains("Text=\"PublicBrush\"")
    );
}

#[test]
fn untyped_templates_keep_their_targets_when_copied_under_the_design_host() {
    let source = r#"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><UserControl.Resources><ControlTemplate x:Key="TextTemplate"><TextBlock Text="{TemplateBinding Text}"/></ControlTemplate><ControlTemplate x:Key="ContentTemplate"><ContentPresenter Content="{TemplateBinding Content}"/></ControlTemplate></UserControl.Resources><StackPanel><TextBox Template="{StaticResource TextTemplate}"/><Button Template="{StaticResource ContentTemplate}"/></StackPanel></UserControl>"#;
    let document = Document::new(source).unwrap();
    let content_template = definition(&document, "ContentTemplate");
    let options = DefinitionPreviewOptions {
        target_type: Some("ContentControl".into()),
        ..Default::default()
    };
    let scope = scope_for(&document, content_template, options, vec![]).unwrap();
    let projection = project(&document, &scope).unwrap();
    let templates = projection
        .document
        .ast()
        .elements()
        .filter(|node| node.local_name() == "ControlTemplate")
        .collect::<Vec<_>>();
    assert_eq!(
        templates.len(),
        3,
        "Both resources and the selected copy are hosted"
    );
    for template in templates {
        let key = template.attribute("x:Key").unwrap();
        let expected = if key == "TextTemplate" {
            "TextBox"
        } else {
            "ContentControl"
        };
        assert_eq!(
            target_type(&projection.document, template, None).unwrap(),
            (FRAMEWORK.into(), expected.into())
        );
    }
    assert_eq!(
        document.source(),
        source,
        "Inferred target annotations are design-only"
    );
    assert!(projection.identities.values().all(|id| {
        *id == content_template || document.ast().is_descendant_of(*id, content_template)
    }));
}

#[test]
fn definition_projection_closes_namespaces_and_keeps_only_original_scope_identities() {
    let document = Document::new(SOURCE).unwrap();
    let node = definition(&document, "CardTheme");
    let scope = scope_for(&document, node, Default::default(), vec![]).unwrap();
    let projection = project(&document, &scope).unwrap();
    assert!(projection.document.is_valid());
    assert!(
        projection
            .document
            .source()
            .contains("__FerroDesignerDefinition")
    );
    assert_eq!(document.source(), SOURCE);
    assert!(!projection.identities.is_empty());
    for original in projection.identities.values() {
        assert!(*original == node || document.ast().is_descendant_of(*original, node));
    }
    let root = projection.document.ast().root();
    assert!(!projection.identities.contains_key(&root));
    // A resource dictionary's containing UserControl is not its template target.
    let node = definition(&document, "BareTemplate");
    let selected = document.ast().node(node).unwrap();
    assert_eq!(
        target_type(&document, selected, None).unwrap(),
        (FRAMEWORK.into(), "Button".into())
    );
}

#[test]
fn definition_target_infers_inline_control_and_preserves_resource_layers() {
    let source = r##"<Grid xmlns="https://github.com/ferroui" xmlns:q="http://schemas.microsoft.com/winfx/2006/xaml"><Grid.Resources><SolidColorBrush q:Key="Accent" Color="Red" /></Grid.Resources><Button Name="Owner"><Button.Resources><SolidColorBrush q:Key="Accent" Color="Blue" /></Button.Resources><Button.Template><ControlTemplate><Border Background="{StaticResource Accent}" /></ControlTemplate></Button.Template></Button></Grid>"##;
    let document = Document::new(source).unwrap();
    let template = document
        .ast()
        .elements()
        .find(|node| node.local_name() == "ControlTemplate")
        .unwrap();
    assert_eq!(
        target_type(&document, template, None).unwrap(),
        (FRAMEWORK.into(), "Button".into())
    );
    let scope = scope_for(&document, template.id, Default::default(), vec![]).unwrap();
    let projection = project(&document, &scope).unwrap();
    assert!(projection.document.source().contains("Color=\"Red\""));
    assert!(projection.document.source().contains("Color=\"Blue\""));
    let count = projection
        .document
        .ast()
        .elements()
        .filter(|node| node.local_name() == "Border")
        .count();
    assert!(
        count >= 5,
        "Resource scope owners need distinct native ancestry"
    );
    assert_eq!(document.source(), source);
}

fn shown_session() -> (DesignerSession, ferroui_base::Ref<Window>) {
    let host = ContentControl::new();
    let session = DesignerSession::new(SOURCE, host.clone()).unwrap();
    let window = Window::new();
    window.set_width(800.0);
    window.set_height(600.0);
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    (session, window)
}

#[test]
fn native_definition_scope_edits_original_theme_parts_and_restores_instance_selection() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (mut session, window) = shown_session();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let theme = definition(session.document(), "CardTheme");
    let instance = session.document().ast().find_name("Instance").unwrap();
    let chrome = session.document().ast().find_name("ThemeChrome").unwrap();
    let revision = session.document().revision();
    session.select([instance]);
    session.enter_definition_scope(theme).unwrap();
    window.update_layout();
    session.preview.rebuild_map(&session.document);
    assert_eq!(session.document().revision(), revision);
    assert_eq!(session.document().source(), SOURCE);
    assert!(!session.can_undo());
    assert!(
        session.preview.visual(theme).is_none(),
        "A synthetic host must never become an authored theme"
    );
    assert!(session.preview.visual(instance).is_none());
    let visual = session
        .preview
        .visual(chrome)
        .expect("Deferred template part must map to its original AST identity");
    assert!(visual.cast::<Border>().is_some());
    session.select([chrome]);
    session.set_property(&[chrome], "Padding", "24").unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(chrome)
            .unwrap()
            .attribute("Padding"),
        Some("24")
    );
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    session.undo().unwrap();
    assert_eq!(session.document().source(), SOURCE);
    session.exit_definition_scope().unwrap();
    assert!(session.definition_scope().is_none());
    assert_eq!(session.selection(), &[instance]);
    window.close();
}

#[test]
fn native_template_and_data_template_scopes_render_actual_parts_without_changing_source() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (mut session, window) = shown_session();
    for (key, name) in [
        ("BareTemplate", "TemplateChrome"),
        ("ItemTemplate", "ItemChrome"),
    ] {
        let node = definition(session.document(), key);
        let part = session.document().ast().find_name(name).unwrap();
        session.enter_definition_scope(node).unwrap();
        window.update_layout();
        session.preview.rebuild_map(&session.document);
        assert!(
            session.preview.visual(part).is_some(),
            "{name} must have a real preview object"
        );
        assert_eq!(session.document().source(), SOURCE);
    }
    session.exit_definition_scope().unwrap();
    window.close();
}

#[test]
fn failed_definition_preview_keeps_accepted_scope_and_read_only_scope_is_inspectable() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (mut session, window) = shown_session();
    let theme = definition(session.document(), "CardTheme");
    session.enter_definition_scope(theme).unwrap();
    let scope = session.definition_scope().cloned().unwrap();
    let previous = session.preview.root().unwrap();
    let options = DefinitionPreviewOptions {
        target_type: Some("MissingPreviewTarget".into()),
        ..Default::default()
    };
    assert!(
        session
            .enter_definition_scope_with_options(theme, options)
            .is_err()
    );
    assert_eq!(session.definition_scope(), Some(&scope));
    assert!(std::ptr::eq(&*previous, &*session.preview.root().unwrap()));
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Ready { .. }
    ));
    session.set_read_only(true);
    session.exit_definition_scope().unwrap();
    session.enter_definition_scope(theme).unwrap();
    assert!(!session.can_manipulate());
    let part = session.document().ast().find_name("ThemeChrome").unwrap();
    assert!(session.set_property(&[part], "Padding", "30").is_err());
    assert_eq!(session.document().source(), SOURCE);
    session.set_read_only(false);
    session
        .open_document(
            "<Border xmlns=\"https://github.com/ferroui\" />".into(),
            session.document_uri().to_owned(),
        )
        .unwrap();
    assert!(session.definition_scope().is_none());
    window.close();
}
