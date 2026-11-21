use super::*;

struct Graph(BTreeMap<&'static str, Document>);

impl Graph {
    fn sources(&self) -> Vec<IncludedResourceDocument<'_>> {
        self.0
            .iter()
            .map(|(name, document)| IncludedResourceDocument {
                address: name,
                uri: name,
                document,
            })
            .collect()
    }
}

impl ComponentResourceSources for Graph {
    fn include(&self, _: &str, _: &Document, node: &Node) -> Option<IncludedResourceDocument<'_>> {
        let (address, document) = self.0.get_key_value(node.attribute("Source")?)?;
        Some(IncludedResourceDocument {
            address,
            uri: address,
            document,
        })
    }
}

const VIEW: &str = include_str!("../../editors/vscode/test/fixtures/IncludedTemplateCopies.xaml");
const ROWS: &str = include_str!("../../editors/vscode/test/fixtures/Templates/Copy Rows.xaml");
const INK: &str = include_str!("../../editors/vscode/test/fixtures/Templates/Shared/Ink.xaml");
const CONTROL_VIEW: &str =
    include_str!("../../editors/vscode/test/fixtures/ControlTemplateWorkbench.xaml");
const CONTROL_DICTIONARY: &str =
    include_str!("../../editors/vscode/test/fixtures/Templates/ControlTemplates.xaml");

fn fixture_graph() -> Graph {
    Graph(BTreeMap::from([
        ("Templates/Copy Rows.xaml", Document::new(ROWS).unwrap()),
        ("Shared/Ink.xaml", Document::new(INK).unwrap()),
    ]))
}

#[test]
fn copies_shadowed_transitive_dependencies_and_preserves_the_original_theme_family() {
    let graph = fixture_graph();
    let includes = graph.sources();
    let sources = ResourceCopySources {
        address: "View.xaml",
        graph: &graph,
        includes: &includes,
    };
    let original = VIEW.replace('\n', "\r\n");
    let mut document = Document::new(&original).unwrap();
    let owner = document.ast().find_name("Directory").unwrap();
    let slot = ItemTemplateSlot::read(
        &document,
        owner,
        ItemTemplateMember::ItemTemplate,
        ItemTemplateDeclaration::Attribute("ItemTemplate".into()),
        None,
    )
    .unwrap();
    let edits = slot
        .plan_included_copy(&document, &sources, &[owner], &|_, fragment| {
            Ok(fragment.source().into())
        })
        .unwrap();
    document.apply("Make Inline Copy", edits).unwrap();
    assert!(document.source().contains("<ListBox.ItemTemplate>"));
    assert!(document
        .source()
        .contains("CustomerRow_InkColor\">#FF207991"));
    assert!(document.source().contains("CustomerRow_CustomerInk2\""));
    assert!(document
        .source()
        .contains("StaticResource 'CustomerRow_InkColor'"));
    assert!(document
        .source()
        .contains("StaticResource 'CustomerRow_CustomerInk2'"));
    assert!(document
        .source()
        .contains("Background=\"{DynamicResource RowSurface}\""));
    assert!(document
        .source()
        .contains("<!-- 🌱 Preserve the row markup and its binding. -->"));
    assert_eq!(
        document
            .source()
            .matches("ItemTemplate=\"{StaticResource CustomerRow}\"")
            .count(),
        1
    );
    assert!(!document.source().replace("\r\n", "").contains('\n'));
    document.undo().unwrap();
    assert_eq!(document.source(), original);
}

#[test]
fn refuses_a_copy_that_would_replace_a_theme_family_with_a_nearer_color() {
    let graph = fixture_graph();
    let includes = graph.sources();
    let sources = ResourceCopySources {
        address: "View.xaml",
        graph: &graph,
        includes: &includes,
    };
    let document = Document::new(VIEW.replace(
        "</ResourceDictionary>",
        "<SolidColorBrush x:Key='RowSurface' Color='Red'/></ResourceDictionary>",
    ))
    .unwrap();
    let owner = document.ast().find_name("Directory").unwrap();
    let slot = ItemTemplateSlot::read(
        &document,
        owner,
        ItemTemplateMember::ItemTemplate,
        ItemTemplateDeclaration::Attribute("ItemTemplate".into()),
        None,
    )
    .unwrap();
    let error = slot
        .plan_included_copy(&document, &sources, &[owner], &|_, fragment| {
            Ok(fragment.source().into())
        })
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("theme-dependent resource 'RowSurface'"));
    assert_eq!(document.revision(), 0);
}

#[test]
fn a_cell_template_uses_the_table_as_its_private_resource_owner() {
    let graph = fixture_graph();
    let includes = graph.sources();
    let sources = ResourceCopySources {
        address: "View.xaml",
        graph: &graph,
        includes: &includes,
    };
    let directory = VIEW
        .lines()
        .find(|line| line.contains("Name=\"Directory\""))
        .unwrap();
    let source = VIEW.replace(
        directory,
        r#"<TreeDataGrid Name="Table">
      <TreeDataGridTemplateColumn Name="Column" Header="Customer">
        <TreeDataGridTemplateColumn.CellTemplate>
          <StaticResource ResourceKey="CustomerRow" />
        </TreeDataGridTemplateColumn.CellTemplate>
      </TreeDataGridTemplateColumn>
    </TreeDataGrid>"#,
    );
    let mut document = Document::new(&source).unwrap();
    let table = document.ast().find_name("Table").unwrap();
    let column = document.ast().find_name("Column").unwrap();
    let slot = ItemTemplateSlot::read(
        &document,
        column,
        ItemTemplateMember::CellTemplate,
        ItemTemplateDeclaration::Element(document.ast().element_children(column)[0]),
        None,
    )
    .unwrap();
    let edits = slot
        .plan_included_copy(&document, &sources, &[table], &|_, fragment| {
            Ok(fragment.source().into())
        })
        .unwrap();
    document.apply("Copy cell template", edits).unwrap();
    assert!(document.source().contains("<TreeDataGrid.Resources>"));
    assert!(document
        .source()
        .contains("<TreeDataGridTemplateColumn.CellTemplate><DataTemplate"));
    assert!(!document
        .source()
        .contains("<TreeDataGridTemplateColumn.Resources>"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn a_local_control_template_copies_the_dependency_shadowed_by_its_consumer() {
    let graph = Graph(BTreeMap::from([(
        "Templates/ControlTemplates.xaml",
        Document::new(CONTROL_DICTIONARY).unwrap(),
    )]));
    let includes = graph.sources();
    let sources = ResourceCopySources {
        address: "ControlTemplateWorkbench.xaml",
        graph: &graph,
        includes: &includes,
    };
    let source = CONTROL_VIEW.replace('\n', "\r\n");
    let mut document = Document::new(&source).unwrap();
    let owner = document.ast().find_name("LocalAction").unwrap();
    let slot = ItemTemplateSlot::read(
        &document,
        owner,
        ItemTemplateMember::Template,
        ItemTemplateDeclaration::Attribute("Template".into()),
        None,
    )
    .unwrap();
    assert!(slot.can_copy_inline(&document));
    let edits = slot
        .plan_resource_copy(&document, &sources, &[owner], &|_, _| {
            panic!("the template and its dependency keep their original document address")
        })
        .unwrap();
    document.apply("Make inline copy", edits).unwrap();
    let after = document.source();
    assert!(after.contains("<Button.Template>"));
    assert!(after.contains("x:Key=\"LocalButton_LocalInk\" Color=\"#FF20657B\""));
    assert!(after.contains("StaticResource 'LocalButton_LocalInk'"));
    assert!(after.contains("Color=\"#FFF39776\""));
    assert!(after.contains(
        "Name=\"SecondLocalAction\" Height=\"48\" Template=\"{StaticResource LocalButton}\""
    ));
    assert!(!after.replace("\r\n", "").contains('\n'));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}
