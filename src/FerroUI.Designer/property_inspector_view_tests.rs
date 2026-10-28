use super::*;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_core::Document;

const SOURCE: &str = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:q="http://schemas.microsoft.com/winfx/2006/xaml">
  <StackPanel.Resources><SolidColorBrush q:Key="Accent" Color="Blue" /></StackPanel.Resources>
  <TextBox q:Name="Input" Text="Hello" Width="180" />
  <Border q:Name="Card" Width="180" Height="{Binding Width, ElementName=Input}" Background="{StaticResource Accent}">
    <Border.RenderTransform><ScaleTransform ScaleX="1.1" /></Border.RenderTransform>
    <TextBlock Text="Preview" />
  </Border>
  <Border Name="Second" Width="240" />
</StackPanel>"#;

fn select(workspace: &Rc<DesignerWorkspace>, names: &[&str]) {
    let ids = names
        .iter()
        .map(|name| {
            workspace
                .session
                .borrow()
                .document()
                .ast()
                .find_name(name)
                .unwrap()
        })
        .collect::<Vec<_>>();
    workspace.session.borrow_mut().select(ids);
    workspace.refresh(true);
}

fn rows(workspace: &DesignerWorkspace) -> Vec<Rc<PropertyRow>> {
    workspace.inspectors.property_source.items().to_vec()
}

fn names(workspace: &DesignerWorkspace) -> Vec<String> {
    rows(workspace)
        .into_iter()
        .filter(|row| row.category.is_none())
        .map(|row| row.name.clone())
        .collect()
}

#[test]
fn property_inspector_source_classification_respects_lexical_namespaces_and_escaped_literals() {
    let document = Document::new(r#"<ui:Border xmlns:ui="https://github.com/ferroui" xmlns:app="clr-namespace:Application" xmlns:m="using:FerroUI.Markup.Xaml.MarkupExtensions" xmlns:c="clr-namespace:FerroUI.Markup.Xaml.MarkupExtensions;assembly=FerroUI.Markup.Xaml" xmlns="urn:unrelated"><ui:TextBlock xmlns:b="https://github.com/ferroui" /></ui:Border>"#).unwrap();
    let root = document.ast().root();
    let child = document.ast().element_children(root)[0];
    for (node, value, expected) in [
        (root, "{ui:Binding Name}", InspectorSource::Binding),
        (root, "{ui:BindingExtension Name}", InspectorSource::Binding),
        (
            root,
            "{ui:StaticResource Accent}",
            InspectorSource::Resource,
        ),
        (
            child,
            "{b:DynamicResource Accent}",
            InspectorSource::Resource,
        ),
        (root, "{m:ReflectionBinding Name}", InspectorSource::Binding),
        (
            root,
            "{c:CompiledBindingExtension Name}",
            InspectorSource::Binding,
        ),
        (
            root,
            "{m:StaticResourceExtension Accent}",
            InspectorSource::Resource,
        ),
        (root, "{Binding Name}", InspectorSource::Expression),
        (root, "{app:Binding Name}", InspectorSource::Expression),
        (
            root,
            "{app:StaticResource Accent}",
            InspectorSource::Expression,
        ),
        (root, "{}{ui:Binding Name}", InspectorSource::Local),
    ] {
        assert_eq!(
            property_source(&document, node, Some(value), false, false),
            expected,
            "{value}"
        );
    }
    assert_eq!(
        property_source(&document, root, None, true, false),
        InspectorSource::Object
    );
    assert_eq!(
        property_source(&document, root, None, true, true),
        InspectorSource::Mixed
    );
    let bare = Document::new("<Border />").unwrap();
    assert_eq!(
        property_source(
            &bare,
            bare.ast().root(),
            Some("{Binding Name}"),
            false,
            false
        ),
        InspectorSource::Binding
    );
}

#[test]
fn property_inspector_view_filters_native_rows_and_restores_independent_group_expansion() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    select(&workspace, &["Card"]);
    let document = workspace.session.borrow().document().clone();
    let selection = workspace.session.borrow().selection().to_vec();
    let row = rows(&workspace)
        .into_iter()
        .find(|row| row.name == "Layout")
        .unwrap();
    (row.category.as_ref().unwrap().toggle)();
    workspace
        .root
        .get_control::<ComboBox>("PropertySort")
        .set_selected_index(1);
    assert_eq!(
        workspace.property_inspector_view(),
        PropertyInspectorView {
            sort: PropertyInspectorSort::Alphabetical,
            filter: PropertyInspectorFilter::All,
        }
    );
    let ordered = names(&workspace);
    assert!(rows(&workspace).iter().all(|row| row.category.is_none()));
    assert!(
        ordered
            .windows(2)
            .all(|pair| pair[0].to_lowercase() <= pair[1].to_lowercase())
    );
    assert!(ordered.contains(&"Width".into()));
    assert!(
        !ordered.contains(&"q:Name".into()),
        "XAML directive aliases share the Name row"
    );
    assert_eq!(
        rows(&workspace)
            .iter()
            .find(|row| row.name == "Name")
            .unwrap()
            .context
            .as_ref()
            .unwrap()
            .value
            .as_deref(),
        Some("Card")
    );

    for (filter, expected) in [
        (PropertyInspectorFilter::Bindings, "Height"),
        (PropertyInspectorFilter::Resources, "Background"),
        (PropertyInspectorFilter::Objects, "RenderTransform"),
    ] {
        workspace.set_property_inspector_view(PropertyInspectorView {
            sort: PropertyInspectorSort::ValueSource,
            filter,
        });
        assert_eq!(names(&workspace), vec![expected]);
    }
    workspace.set_property_inspector_view(PropertyInspectorView {
        sort: PropertyInspectorSort::ValueSource,
        filter: PropertyInspectorFilter::All,
    });
    let bindings = rows(&workspace)
        .into_iter()
        .find(|row| row.name == "Bindings")
        .unwrap();
    (bindings.category.as_ref().unwrap().toggle)();
    assert!(!names(&workspace).contains(&"Height".into()));
    workspace.set_property_inspector_view(PropertyInspectorView::default());
    assert!(
        !rows(&workspace)
            .iter()
            .find(|row| row.name == "Layout")
            .unwrap()
            .category
            .as_ref()
            .unwrap()
            .expanded
    );
    workspace.set_property_inspector_view(PropertyInspectorView {
        sort: PropertyInspectorSort::ValueSource,
        filter: PropertyInspectorFilter::All,
    });
    assert!(
        !rows(&workspace)
            .iter()
            .find(|row| row.name == "Bindings")
            .unwrap()
            .category
            .as_ref()
            .unwrap()
            .expanded
    );
    assert_eq!(
        workspace.session.borrow().document().source(),
        document.source()
    );
    assert_eq!(
        workspace.session.borrow().document().revision(),
        document.revision()
    );
    assert_eq!(workspace.session.borrow().selection(), selection);
    assert!(!workspace.session.borrow().can_undo());
}

#[test]
fn property_inspector_search_tokens_mixed_values_and_empty_recovery_preserve_source_drafts() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    select(&workspace, &["Card"]);
    let search = workspace.root.get_control::<TextBox>("PropertySearch");
    for (query, expected) in [
        ("@binding height", "Height"),
        ("@resource background", "Background"),
        ("@object transform", "RenderTransform"),
        ("@local width", "Width"),
        ("@default maxwidth", "MaxWidth"),
    ] {
        search.set_text(Some(query));
        workspace.rebuild_properties();
        assert_eq!(names(&workspace), vec![expected]);
    }
    search.set_text(Some("@expression"));
    workspace.rebuild_properties();
    assert!(names(&workspace).contains(&"Background".into()));
    assert!(names(&workspace).contains(&"Height".into()));
    select(&workspace, &["Card", "Second"]);
    search.set_text(Some("@mixed width"));
    workspace.rebuild_properties();
    assert_eq!(names(&workspace), vec!["Width"]);
    search.set_text(Some("@resource width"));
    workspace.rebuild_properties();
    assert!(names(&workspace).is_empty());
    assert!(
        workspace
            .root
            .get_control::<StackPanel>("PropertyEmptyState")
            .is_visible()
    );
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    assert!(!workspace.session.borrow().can_undo());
    workspace.source.set_text(Some("<StackPanel"));
    assert!(workspace.pending_source.get());
    let staged_revision = workspace.session.borrow().document().revision();
    let staged_undo = workspace.session.borrow().can_undo();
    workspace
        .root
        .get_control::<Button>("PropertyEmptyClear")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert!(!names(&workspace).is_empty());
    assert!(
        !workspace
            .root
            .get_control::<StackPanel>("PropertyEmptyState")
            .is_visible()
    );
    assert_eq!(workspace.source.text().as_deref(), Some("<StackPanel"));
    assert!(workspace.pending_source.get());
    assert_eq!(
        workspace.session.borrow().document().source(),
        "<StackPanel"
    );
    assert_eq!(
        workspace.session.borrow().document().revision(),
        staged_revision
    );
    assert_eq!(workspace.session.borrow().can_undo(), staged_undo);
}

#[test]
fn property_inspector_navigation_resolves_resource_and_binding_sources_and_rejects_retired_menus() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    select(&workspace, &["Card"]);
    let selection = workspace.session.borrow().selection().to_vec();
    workspace.reveal_property_source("Name").unwrap();
    let selected_text = || {
        let source = workspace.source.text().unwrap();
        let start = super::super::source_tools::byte_from_utf16(
            &source,
            workspace.source.selection_start(),
        );
        let end =
            super::super::source_tools::byte_from_utf16(&source, workspace.source.selection_end());
        source[start.min(end)..start.max(end)].to_owned()
    };
    assert_eq!(selected_text(), "q:Name=\"Card\"");
    workspace.go_to_property_definition("Background").unwrap();
    assert!(selected_text().contains("q:Key=\"Accent\""));
    workspace.go_to_property_definition("Height").unwrap();
    assert_eq!(selected_text(), "Input");
    assert_eq!(workspace.session.borrow().selection(), selection);
    let actions = rows(&workspace)
        .into_iter()
        .find(|row| row.name == "Width")
        .unwrap()
        .source_actions
        .clone();
    assert_eq!(actions.len(), 1);
    select(&workspace, &["Second"]);
    workspace.set_view_mode(crate::WorkspaceViewMode::Design);
    (actions[0].1)();
    assert_eq!(workspace.view_mode(), crate::WorkspaceViewMode::Design);
    assert!(
        workspace
            .status
            .text()
            .unwrap_or_default()
            .contains("changed")
    );
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    assert!(!workspace.session.borrow().can_undo());
    workspace.source.set_text(Some("<StackPanel"));
    let staged_revision = workspace.session.borrow().document().revision();
    let staged_undo = workspace.session.borrow().can_undo();
    assert!(
        workspace
            .reveal_property_source("Width")
            .unwrap_err()
            .contains("draft")
    );
    assert_eq!(workspace.source.text().as_deref(), Some("<StackPanel"));
    assert_eq!(
        workspace.session.borrow().document().revision(),
        staged_revision
    );
    assert_eq!(workspace.session.borrow().can_undo(), staged_undo);
}

#[test]
fn property_inspector_unset_numeric_defaults_remain_empty_and_explicit_auto_or_nan_is_preserved() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui"><Border Name="Inherited" /><Border Name="Explicit" Canvas.Right="NaN" Width="NaN" /></Canvas>"#;
    let workspace = DesignerWorkspace::new(source).unwrap();
    assert!(matches!(
        workspace.session.borrow().preview_status(),
        PreviewStatus::Ready { .. }
    ));
    select(&workspace, &["Inherited"]);
    workspace.set_property_inspector_view(PropertyInspectorView {
        sort: PropertyInspectorSort::Alphabetical,
        filter: PropertyInspectorFilter::All,
    });
    for name in ["Canvas.Right", "Canvas.Bottom", "Width", "MaxWidth"] {
        let row = rows(&workspace)
            .into_iter()
            .find(|row| row.name == name)
            .unwrap();
        assert!(row.context.as_ref().unwrap().value.is_none());
        let cell = property_cell(&row);
        let number = cell
            .get_visual_descendants()
            .find_map(|visual| visual.cast::<ferroui_controls::NumericUpDown>());
        assert!(
            number.is_some(),
            "{name}: kind={:?}, type={:?}, effective={:?}, controls={:?}",
            row.metadata.as_ref().map(|metadata| &metadata.kind),
            row.context.as_ref().unwrap().runtime_type,
            row.effective,
            cell.get_visual_descendants()
                .map(|visual| visual.get_type().name().to_owned())
                .collect::<Vec<_>>()
        );
        let number = number.unwrap();
        assert!(
            number.value().is_none(),
            "{name} must not seed a nonfinite draft"
        );
        assert!(
            !number
                .text()
                .is_some_and(|text| text == "NaN" || text == "inf")
        );
        assert!(number.placeholder_text().is_some());
    }
    select(&workspace, &["Explicit"]);
    for (name, value) in [("Canvas.Right", "NaN"), ("Width", "NaN")] {
        let row = rows(&workspace)
            .into_iter()
            .find(|row| row.name == name)
            .unwrap();
        assert_eq!(row.context.as_ref().unwrap().value.as_deref(), Some(value));
        let cell = property_cell(&row);
        assert!(
            cell.get_visual_descendants()
                .filter_map(|visual| visual.cast::<TextBox>())
                .any(|text| text.text().as_deref() == Some(value))
        );
    }
    assert_eq!(workspace.session.borrow().document().source(), source);
    assert!(!workspace.session.borrow().can_undo());
    // An existing Auto spelling remains source even when the loader must report
    // a conversion diagnostic. Inspecting it must not replace it with an inherited value.
    let auto_source =
        r#"<Border xmlns="https://github.com/ferroui" Name="AutoSize" Width="Auto" />"#;
    let auto = DesignerWorkspace::new(auto_source).unwrap();
    select(&auto, &["AutoSize"]);
    let row = rows(&auto)
        .into_iter()
        .find(|row| row.name == "Width")
        .unwrap();
    assert_eq!(row.context.as_ref().unwrap().value.as_deref(), Some("Auto"));
    assert_eq!(auto.session.borrow().document().source(), auto_source);
    assert!(!auto.session.borrow().can_undo());
}
