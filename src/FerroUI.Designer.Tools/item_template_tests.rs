use super::*;

fn node(document: &Document, name: &str) -> NodeId {
    document
        .ast()
        .nodes_in_source_order()
        .find(|node| node.xaml_name() == Some(name))
        .unwrap()
        .id
}

fn slot(
    document: &Document,
    owner: NodeId,
    declaration: ItemTemplateDeclaration,
) -> ItemTemplateSlot {
    ItemTemplateSlot::read(
        document,
        owner,
        ItemTemplateMember::ItemTemplate,
        declaration,
        None,
    )
    .unwrap()
}

#[test]
fn creates_a_native_inline_template_in_an_aliased_and_self_closing_owner() {
    let source = concat!(
        "<f:Grid xmlns:f='https://github.com/ferroui'>\r\n",
        "  <f:ListBox Name='Items' ItemsSource='{f:Binding Records}' />\r\n",
        "  <!-- keep 🌱 -->\r\n",
        "</f:Grid>",
    );
    let mut document = Document::new(source).unwrap();
    let owner = node(&document, "Items");
    let edits = slot(&document, owner, ItemTemplateDeclaration::Unset)
        .plan(&document, ItemTemplateChange::CreateInline)
        .unwrap();
    document.apply("Create", edits).unwrap();
    assert!(
        document.source().contains(
            "<f:ListBox.ItemTemplate><f:DataTemplate><f:TextBlock Text=\"{f:Binding}\" />"
        )
    );
    assert!(
        document
            .source()
            .contains("ItemsSource='{f:Binding Records}'")
    );
    assert!(document.source().contains("\r\n  <!-- keep 🌱 -->\r\n"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn offers_visible_templates_and_copies_an_object_form_reference_without_changing_the_resource() {
    let source = r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <Grid.Resources>
    <SolidColorBrush x:Key="Accent">#ff001144</SolidColorBrush>
    <DataTemplate x:Key="Row"><TextBlock Text="{Binding Title}" Foreground="{StaticResource Accent}" /></DataTemplate>
  </Grid.Resources>
  <ListBox Name="Items">
    <ItemsControl.ItemTemplate><StaticResource ResourceKey="Row"/></ItemsControl.ItemTemplate>
  </ListBox>
</Grid>"#;
    let mut document = Document::new(source).unwrap();
    let owner = node(&document, "Items");
    let property = document.ast().element_children(owner)[0];
    let model = slot(&document, owner, ItemTemplateDeclaration::Element(property));
    let [resource] = item_template_resources(&document, owner)
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(resource.key, "Row");
    assert_eq!(model.value.definition(), Some(resource.node));
    let edits = model
        .plan(&document, ItemTemplateChange::CopyInline)
        .unwrap();
    document.apply("Copy", edits).unwrap();
    assert_eq!(
        document
            .source()
            .matches("Text=\"{Binding Title}\"")
            .count(),
        2
    );
    assert_eq!(document.source().matches("x:Key=\"Row\"").count(), 1);
    assert!(document.source().contains("<ListBox.ItemTemplate>"));
    assert!(!document.source().contains("<ItemsControl.ItemTemplate>"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn rejects_copying_a_template_that_would_capture_a_nearer_resource() {
    let source = r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <Grid.Resources>
    <SolidColorBrush x:Key="Accent">#ff001144</SolidColorBrush>
    <DataTemplate x:Key="Row"><TextBlock Foreground="{StaticResource Accent}" /></DataTemplate>
  </Grid.Resources>
  <ListBox Name="Items" ItemTemplate="{StaticResource Row}">
    <ListBox.Resources><SolidColorBrush x:Key="Accent">#ff440011</SolidColorBrush></ListBox.Resources>
  </ListBox>
</Grid>"#;
    let document = Document::new(source).unwrap();
    let owner = node(&document, "Items");
    let slot = slot(
        &document,
        owner,
        ItemTemplateDeclaration::Attribute("ItemTemplate".into()),
    );
    assert!(
        slot.plan(&document, ItemTemplateChange::CopyInline)
            .unwrap_err()
            .0
            .contains("Accent")
    );
}

#[test]
fn protects_opaque_expressions_and_preserves_an_existing_resource_attribute_spelling() {
    let source = r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <Grid.Resources><DataTemplate x:Key="Row"><TextBlock /></DataTemplate></Grid.Resources>
  <ListBox Name="Opaque" ItemTemplate="{Binding ChosenTemplate}" />
  <ListBox Name="Items" ItemsControl.ItemTemplate='{DynamicResource Row}' />
</Grid>"#;
    let mut document = Document::new(source).unwrap();
    let owner = node(&document, "Opaque");
    let model = slot(
        &document,
        owner,
        ItemTemplateDeclaration::Attribute("ItemTemplate".into()),
    );
    assert!(model.plan(&document, ItemTemplateChange::Reset).is_err());
    let owner = node(&document, "Items");
    let model = slot(
        &document,
        owner,
        ItemTemplateDeclaration::Attribute("ItemsControl.ItemTemplate".into()),
    );
    let edits = model
        .plan(
            &document,
            ItemTemplateChange::AssignResource {
                resource: model.value.definition().unwrap(),
            },
        )
        .unwrap();
    document.apply("Assign", edits).unwrap();
    assert!(document.source().contains("ItemsControl.ItemTemplate='"));
    let updated = slot(
        &document,
        owner,
        ItemTemplateDeclaration::Attribute("ItemsControl.ItemTemplate".into()),
    );
    assert!(
        matches!(updated.value, ItemTemplateValue::Resource { key, dynamic: false, .. } if key == "Row")
    );
    assert!(
        document
            .source()
            .contains("ItemTemplate=\"{Binding ChosenTemplate}\"")
    );
}
