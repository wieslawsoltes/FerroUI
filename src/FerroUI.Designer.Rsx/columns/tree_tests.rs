use crate::{RsxProjection, project_rust};
use ferroui_designer_core::{Document, NodeId, TextPatch};
use ferroui_designer_tools::{
    ColumnChange, ColumnKind, ItemTemplateChange, ItemTemplateMember, TableColumns, table_columns,
};

const TREE: &str = r##"fn before() -> Element { rsx! { TextBlock { "Before 🧭" } } }
fn accounts() -> Element {
    rsx! {
        TreeDataGrid {
            name: "Accounts",
            items_source: rows(),
            column_definitions: ::ferroui_view::tree_data_grid_columns([
                ::ferroui_view::tree_data_grid_row_header_column().build(),
                // Keep the account hierarchy
                ::ferroui_view::tree_data_grid_expander_column(
                    ::ferroui_view::tree_data_grid_text_column()
                        .header(r#"Account 🪴"#)
                        .binding_path("Customer.Name")
                        .build()
                ).children_path("Departments").build(),
                ::ferroui_view::tree_data_grid_check_box_column()
                    .header("Active").binding(active_binding()).build(),
                ::ferroui_view::tree_data_grid_template_column()
                    .header("Status").cell_template(::ferroui_view::template(|_: Account| {
                        ::ferroui_view::rsx! { TextBlock { text: "Active account" } }
                    })).build(),
            ])
        }
    }
}
fn after() { let _ = "keep"; }
"##;

fn model(source: &str) -> (RsxProjection<'_>, Document, TableColumns, NodeId) {
    let at = source.find("TreeDataGrid {").unwrap();
    let projection = project_rust(source, at).unwrap();
    let xaml = Document::new(projection.xaml()).unwrap();
    let table = xaml.ast().root();
    let mut columns = table_columns(&xaml, table).unwrap();
    projection.map_column_model(&xaml, &mut columns);
    (projection, xaml, columns, table)
}

fn edit(
    source: &str,
    change: impl FnOnce(&TableColumns) -> ColumnChange,
) -> Result<String, String> {
    let (view, xaml, columns, _) = model(source);
    let patches = view.edit_columns(&xaml, &columns, &change(&columns))?;
    TextPatch::apply_all(source, patches).map_err(|error| error.to_string())
}

#[test]
fn tree_grid_projects_native_columns_and_edits_original_nested_source() {
    let source = TREE.replace('\n', "\r\n");
    let (projection, xaml, columns, table) = model(&source);
    assert!(projection.has_projected_columns());
    assert!(
        projection.nodes[0]
            .columns
            .as_ref()
            .unwrap()
            .matches_layout(&xaml, table)
            .unwrap()
    );
    assert_eq!(columns.reason, None);
    assert_eq!(
        columns
            .columns
            .iter()
            .map(|column| column.kind)
            .collect::<Vec<_>>(),
        [
            ColumnKind::RowHeader,
            ColumnKind::Expander,
            ColumnKind::CheckBox,
            ColumnKind::Template,
        ]
    );
    assert_eq!(columns.columns[1].label, "Account 🪴");
    assert!(
        columns
            .available
            .iter()
            .all(|choice| choice.kind != ColumnKind::Expander)
    );
    assert!(!columns.columns[2].can_duplicate);

    let widened = edit(&source, |columns| ColumnChange::Set {
        column: columns.columns[1].children[0].node,
        field: "Width".into(),
        value: "180".into(),
    })
    .unwrap();
    assert_eq!(widened, source.replace(
        ".binding_path(\"Customer.Name\")\r\n                        .build()",
        ".binding_path(\"Customer.Name\")\r\n                        .width(::ferroui_controls::GridLength::from_pixels(180.0)).build()",
    ));
    let moved = edit(&widened, |columns| ColumnChange::Move {
        column: columns.columns[1].node,
        index: 0,
    })
    .unwrap();
    assert!(
        moved.find("// Keep the account hierarchy").unwrap()
            < moved.find("tree_data_grid_row_header_column").unwrap()
    );
    assert!(moved.contains("binding(active_binding())"));
    assert!(moved.ends_with("fn after() { let _ = \"keep\"; }\r\n"));
    assert!(
        edit(&source, |columns| ColumnChange::Insert {
            index: columns.columns.len(),
            kind: ColumnKind::Expander,
            header: None
        })
        .is_err()
    );
}

#[test]
fn new_tree_grid_uses_native_defaults_and_explicit_models_stay_source_owned() {
    let empty = "rsx! { TreeDataGrid { items_source: rows() } }";
    let inserted = edit(empty, |_| ColumnChange::Insert {
        index: 0,
        kind: ColumnKind::Template,
        header: Some("Status".into()),
    })
    .unwrap();
    assert!(inserted.contains("column_definitions: ::ferroui_view::tree_data_grid_columns([::ferroui_view::tree_data_grid_template_column().header(\"Status\").build()])"));
    assert_eq!(model(&inserted).2.columns.len(), 1);
    let protected = inserted.replace("items_source: rows()", "model: account_model()");
    let (view, document, mut columns, table) = model(&protected);
    assert!(columns.reason.is_some());
    let range = view.map_column_model(&document, &mut columns).unwrap();
    assert_eq!(&protected[range.range()], "account_model()");
    assert_eq!(columns.table, table);
}

#[test]
fn tree_grid_template_slots_insert_before_build_and_keep_the_original_body() {
    let (view, document, columns, _) = model(TREE);
    let node = columns.columns[3].node;
    let slots = view.item_templates(&document, node);
    assert_eq!(slots.len(), 2);
    assert!(TREE[slots[0].body.as_ref().unwrap().location.span.range()].contains("Active account"));
    assert!(slots[1].can_create);
    let patches = view
        .edit_item_template(
            &document,
            node,
            ItemTemplateMember::CellEditingTemplate,
            &ItemTemplateChange::CreateInline,
            Some("Account"),
        )
        .unwrap();
    let next = TextPatch::apply_all(TREE, patches).unwrap();
    assert!(next.contains("})).cell_editing_template(::ferroui_view::template::<Account, _>"));
    assert!(next.contains(r#"TextBox { placeholder_text: "Enter a value""#));
    let (view, document, columns, _) = model(&next);
    let node = columns.columns[3].node;
    let patches = view
        .edit_item_template(
            &document,
            node,
            ItemTemplateMember::CellEditingTemplate,
            &ItemTemplateChange::Reset,
            None,
        )
        .unwrap();
    assert_eq!(TextPatch::apply_all(&next, patches).unwrap(), TREE);
}
