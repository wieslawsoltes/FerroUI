use super::*;
use ferroui_designer_tools::ComponentCatalog;

const SOURCE: &str = r##"<Grid Name="Accounts" xmlns="https://github.com/ferroui"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    xmlns:i="using:FerroUI.Xaml.Interactivity"
    xmlns:b="using:FerroUI.Xaml.Interactions.Core"
    xmlns:custom="using:Accounts.Platform">
    <Grid.Resources>
      <ResourceDictionary>
        <ResourceDictionary.MergedDictionaries>
          <ResourceInclude Source="Shared Palette.xaml"/>
        </ResourceDictionary.MergedDictionaries>
        <SolidColorBrush x:Key="Accent" Color="#D32F52"/>
        <DataTemplate x:Key="AccountRow">
          <Grid Name="Row">
            <Grid.Resources><SolidColorBrush x:Key="Accent" Color="#202D8C"/></Grid.Resources>
            <Button Name="RowAction">
              <i:Interaction.Behaviors>
                <b:EventTriggerBehavior EventName="Click"/>
              </i:Interaction.Behaviors>
            </Button>
          </Grid>
        </DataTemplate>
      </ResourceDictionary>
    </Grid.Resources>
    <Button Name="Save">
      <Button.Template><ControlTemplate><Border/></ControlTemplate></Button.Template>
      <i:Interaction.Behaviors>
        <i:BehaviorCollection>
          <b:EventTriggerBehavior EventName="Click"/>
          <custom:ConfirmBehavior Name="Confirm"/>
        </i:BehaviorCollection>
      </i:Interaction.Behaviors>
      <custom:Interaction.Behaviors><custom:SeparateProperty/></custom:Interaction.Behaviors>
    </Button>
    <Button Name="Cancel" i:Interaction.Behaviors="{Binding CancelActions}"/>
    <ListBox Name="Directory">
      <ListBox.ItemsPanel><ItemsPanelTemplate><StackPanel/></ItemsPanelTemplate></ListBox.ItemsPanel>
    </ListBox>
</Grid>"##;

#[test]
fn scopes_keep_attached_object_and_nested_resource_owners() {
    let document = Document::new(SOURCE).unwrap();
    let definitions = ComponentCatalog::declarations(&document).unwrap();
    let scopes = xaml_scopes(&document, &definitions, None).unwrap();
    let scope = |name| {
        let id = document.ast().find_name(name).unwrap();
        scopes.iter().find(|scope| scope.owner == id).unwrap()
    };
    let save = scope("Save");
    assert_eq!(save.path, "Accounts › Save");
    assert_eq!(&SOURCE[save.owner_range.unwrap().range()], "Button");
    assert_eq!(
        save.objects
            .iter()
            .map(|object| (object.title.as_str(), object.kind))
            .collect::<Vec<_>>(),
        [
            ("ControlTemplate", "Control template"),
            ("Click", "Behavior"),
            ("Confirm", "Behavior")
        ],
    );
    let bound = &scope("Cancel").objects[0];
    assert_eq!(bound.title, "Behavior collection");
    assert_eq!(&SOURCE[bound.source.range()], "{Binding CancelActions}");
    let layout = &scope("Directory").objects[0];
    assert_eq!(layout.title, "ItemsPanelTemplate");
    assert_eq!(layout.kind, "Template");
    assert!(layout.definition.is_none());
    let row = scope("Row");
    assert!(row.path.ends_with("AccountRow › Row"));
    assert_eq!(row.objects[0].title, "Accent");
    let dictionary = scopes
        .iter()
        .find(|scope| scope.path == "Accounts › ResourceDictionary")
        .unwrap();
    assert_eq!(
        dictionary
            .objects
            .iter()
            .map(|object| object.title.as_str())
            .collect::<Vec<_>>(),
        ["Shared Palette.xaml", "Accent", "AccountRow"],
    );
    assert_eq!(
        &SOURCE[dictionary.objects[0].source.range()],
        "ResourceInclude"
    );
    assert_ne!(dictionary.objects[1].definition, row.objects[0].definition);
}

#[test]
fn isolated_definition_lists_only_its_browseable_contents() {
    let document = Document::new(SOURCE).unwrap();
    let definitions = ComponentCatalog::declarations(&document).unwrap();
    let template = definitions
        .iter()
        .find(|entry| entry.key.as_deref() == Some("AccountRow"))
        .unwrap();
    let scopes = xaml_scopes(&document, &definitions, Some(template.node)).unwrap();
    assert_eq!(scopes.len(), 2);
    assert!(scopes
        .iter()
        .all(|scope| scope.path.contains("AccountRow › Row")));
    assert_eq!(scopes[0].objects[0].title, "Accent");
    assert_eq!(scopes[1].objects[0].title, "Click");
}

#[test]
fn mapped_rust_scopes_use_the_selected_invocation_and_declaration_spans() {
    let source = r##"fn other() -> Element { rsx! { ListBox { name: "Other", item_template: other_template() } } }
fn accounts() -> Element {
    rsx! { StackPanel {
        name: "Accounts",
        styles: [::ferroui_view::style("Button#Save")
            .resource("Accent", ferroui_base::Color::from_uint32(0xff101530))],
        ListBox { name: "Directory", item_template: account_row() }
        {runtime_notice()}
    } }
}"##;
    let caret = source.find("StackPanel").unwrap();
    let projection = ferroui_designer_rsx::project_rust(source, caret).unwrap();
    let document = Document::new(projection.xaml()).unwrap();
    let scopes = rsx::scopes(&document, &projection, None);
    assert_eq!(scopes.len(), 2);
    assert_eq!(scopes[0].path, "Accounts");
    let style = &scopes[0].objects[0];
    assert_eq!(style.title, "Button#Save");
    assert_eq!(&source[style.source.range()], "\"Button#Save\"");
    let resource = &scopes[0].objects[1];
    assert_eq!(resource.title, "Accent");
    assert_eq!(resource.detail.as_deref(), Some("Button#Save"));
    assert_eq!(scopes[1].path, "Accounts › Directory");
    let assignment = &scopes[1].objects[0];
    assert_eq!(assignment.title, "ItemTemplate");
    assert_eq!(&source[assignment.source.range()], "account_row()");
}

#[cfg(all(feature = "mcp", feature = "render-smoke"))]
#[test]
fn studio_library_scope_and_mcp_library_reveal_use_the_same_authored_owner() {
    use crate::{mcp::DesignerMcp, DesignerWorkspace, NavigationPane};
    use ferroui_base::animation::IGlobalClock;
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    use ferroui_base::{media::MediaContext, styling::IThemeVariantHost, FerroLocator};
    use ferroui_controls::{
        automation::AutomationProperties,
        testing::{TestServices, UnitTestApplication},
        Application, ComboBox, Control, TextBox, Window,
    };
    use ferroui_mcp::{Permissions, UiToolProvider};
    use serde_json::json;

    let mut services = TestServices::styled_window()
        .with_theme(|| ferroui_themes_winui::WinUITheme::new().as_style());
    let focus = TestServices::real_focus();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    let _app = UnitTestApplication::start(services);
    ferroui_themes_winui::register_types();
    let locator = FerroLocator::current_mutable();
    locator
        .bind::<dyn IGlobalClock>()
        .to_constant(MediaContext::instance().clock());
    locator
        .bind::<dyn IThemeVariantHost>()
        .to_constant(Application::current().unwrap().as_theme_variant_host());
    let source = r#"<StackPanel Name="Order" xmlns="https://github.com/ferroui"
        xmlns:i="using:FerroUI.Xaml.Interactivity" xmlns:b="using:FerroUI.Xaml.Interactions.Core">
        <Button Name="Review" Content="Review">
          <i:Interaction.Behaviors><b:EventTriggerBehavior EventName="Click"/></i:Interaction.Behaviors>
        </Button>
        <Button Name="Cancel"/>
    </StackPanel>"#;
    let workspace = DesignerWorkspace::new(source).unwrap();
    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(720.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    workspace.set_navigation_pane(NavigationPane::Library);
    window.update_layout();
    workspace.refresh(true);
    let browser = workspace.component_library().unwrap();
    let picker = browser
        .control()
        .get_logical_descendants()
        .find(|control| AutomationProperties::get_name(control).as_deref() == Some("Owning scope"))
        .unwrap()
        .cast::<ComboBox>()
        .unwrap();
    assert_eq!(picker.items().count(), 2);
    picker.set_selected_index(1);
    let [row]: [_; 1] = browser.visible_items().try_into().unwrap();
    assert_eq!(row.title, "Click");
    assert!(browser.select(&row.id));
    window.update_layout();
    let row = browser.list().container_from_index(0).unwrap();
    assert!(row.focus());
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    enter.set_source(row.to_ref());
    row.raise_event(&enter);
    assert!(enter.handled());
    let session = workspace.session();
    let owner = session
        .borrow()
        .document()
        .ast()
        .find_name("Review")
        .unwrap();
    let provider = DesignerMcp::with_workspace(&workspace);
    let report = provider
        .call(
            "designer_library_scopes",
            &json!({}),
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(report["scopes"][0]["owner"], owner);
    assert_eq!(report["scopes"][0]["path"], "Order › Review");
    let range = report["scopes"][0]["objects"][0]["range"].clone();
    let range = serde_json::from_value::<ferroui_designer_language::Range>(range).unwrap();
    let span = ferroui_designer_language::LineIndex::new(source)
        .span(range)
        .unwrap();
    let editor = workspace.control().get_control::<TextBox>("Source");
    assert_eq!(&source[span.range()], "b:EventTriggerBehavior");
    assert_eq!(editor.selection_start() as usize, span.start);
    assert_eq!(editor.selection_end() as usize, span.end);
    assert_eq!(session.borrow().authored_source(), source);
    assert!(!session.borrow().can_undo());
    window.close();
}
