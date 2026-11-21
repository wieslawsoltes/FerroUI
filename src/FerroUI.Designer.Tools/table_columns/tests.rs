use super::*;

fn apply(document: &mut Document, table: NodeId, change: ColumnChange) {
    let edits = plan_column_change(document, table, &change).unwrap();
    document.apply(change.label(), edits).unwrap();
}

#[test]
fn creates_the_named_table_column_slot_and_preserves_namespaces() {
    let source = "<ui:TableView xmlns:ui='https://github.com/ferroui' xmlns='using:Demo' />";
    let mut document = Document::new(source).unwrap();
    let table = document.ast().root();
    apply(
        &mut document,
        table,
        ColumnChange::Insert {
            index: 0,
            kind: ColumnKind::Table,
            header: Some("{Pipeline & \"state\"}".into()),
        },
    );
    let catalog = table_columns(&document, table).unwrap();
    assert_eq!(catalog.columns[0].label, "{Pipeline & \"state\"}");
    assert!(document.source().contains("<ui:TableView.Columns><ui:TableViewColumn Header=\"{}{Pipeline &amp; &quot;state&quot;}\" />"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn reorders_a_tree_with_inline_templates_without_reserializing_source() {
    let source = concat!(
        "<TreeDataGrid xmlns=\"https://github.com/ferroui\">\r\n",
        "  <TreeDataGrid.Styles><Style Selector=\"TreeDataGridRow\" /></TreeDataGrid.Styles>\r\n",
        "  <TreeDataGridTextColumn Header=\"Name\" Binding=\"{Binding Name, StringFormat='[ {0} ]'}\" />\r\n",
        "  <!-- 🌿 Keep the authored status template -->\r\n",
        "  <TreeDataGridTemplateColumn Header=\"Status\"><TreeDataGridTemplateColumn.CellTemplate>\r\n",
        "    <DataTemplate><TextBlock Text=\"{Binding Status}\" /></DataTemplate>\r\n",
        "  </TreeDataGridTemplateColumn.CellTemplate></TreeDataGridTemplateColumn>\r\n",
        "</TreeDataGrid>\r\n",
    );
    let mut document = Document::new(source).unwrap();
    let table = document.ast().root();
    let first = table_columns(&document, table).unwrap().columns;
    apply(
        &mut document,
        table,
        ColumnChange::Move {
            column: first[1].node,
            index: 0,
        },
    );
    let ordered = table_columns(&document, table).unwrap().columns;
    assert_eq!(
        ordered.iter().map(|column| column.node).collect::<Vec<_>>(),
        [first[1].node, first[0].node]
    );
    assert!(
        document
            .source()
            .contains("<!-- 🌿 Keep the authored status template -->")
    );
    assert!(
        document
            .source()
            .contains("<DataTemplate><TextBlock Text=\"{Binding Status}\" /></DataTemplate>")
    );
    assert!(!document.source().replace("\r\n", "").contains('\n'));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn handles_nested_source_owned_fields_and_protects_expressions() {
    let source = r#"<TreeDataGrid xmlns="https://github.com/ferroui">
  <TreeDataGrid.ColumnDefinitions>
    <TreeDataGridHierarchicalExpanderColumn Header="Team" Width="{Binding TeamWidth}">
      <TreeDataGridTextColumn Binding="{Binding Name}" />
    </TreeDataGridHierarchicalExpanderColumn>
    <TreeDataGridCheckBoxColumn Header="Active" Binding="{Binding IsActive}" />
  </TreeDataGrid.ColumnDefinitions>
</TreeDataGrid>"#;
    let mut document = Document::new(source).unwrap();
    let table = document.ast().root();
    let model = table_columns(&document, table).unwrap();
    let root = &model.columns[0];
    let nested = &root.children[0];
    let write = |column, field: &str, value: &str| ColumnChange::Set {
        column,
        field: field.into(),
        value: value.into(),
    };
    assert!(plan_column_change(&document, table, &write(root.node, "Width", "180")).is_err());
    assert!(plan_column_change(&document, table, &write(nested.node, "Width", "NaN")).is_err());
    assert!(
        plan_column_change(
            &document,
            table,
            &ColumnChange::Remove {
                column: nested.node
            }
        )
        .is_err()
    );
    assert!(
        plan_column_change(
            &document,
            table,
            &ColumnChange::Insert {
                index: 2,
                kind: ColumnKind::Expander,
                header: None
            }
        )
        .is_err()
    );
    apply(
        &mut document,
        table,
        write(nested.node, "Binding", "{Binding Department, Mode=OneWay}"),
    );
    assert!(
        document
            .source()
            .contains("Binding=\"{Binding Department, Mode=OneWay}\"")
    );
    assert!(document.source().contains("Width=\"{Binding TeamWidth}\""));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn keeps_runtime_owned_and_foreign_collections_read_only() {
    for source in [
        r#"<TableView xmlns="https://github.com/ferroui" Columns="{Binding AppColumns}" />"#,
        r#"<TreeDataGrid xmlns="https://github.com/ferroui" Source="{Binding Model}" />"#,
        r#"<TreeDataGrid xmlns="https://github.com/ferroui" xmlns:local="using:App"><local:TreeDataGridTextColumn /></TreeDataGrid>"#,
    ] {
        let document = Document::new(source).unwrap();
        let table = document.ast().root();
        let model = table_columns(&document, table).unwrap();
        assert!(model.reason.is_some());
        let kind = if model.kind == TableKind::TableView {
            ColumnKind::Table
        } else {
            ColumnKind::Text
        };
        assert!(
            plan_column_change(
                &document,
                table,
                &ColumnChange::Insert {
                    index: 0,
                    kind,
                    header: None
                }
            )
            .is_err()
        );
        assert_eq!(document.source(), source);
    }
}

#[test]
fn generated_columns_keep_the_bindings_the_native_source_requires() {
    let source = r#"<TreeDataGrid xmlns="https://github.com/ferroui" x:CompileBindings="True" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" />"#;
    let mut document = Document::new(source).unwrap();
    let table = document.ast().root();
    for (index, kind) in [ColumnKind::Text, ColumnKind::CheckBox, ColumnKind::Expander]
        .into_iter()
        .enumerate()
    {
        apply(
            &mut document,
            table,
            ColumnChange::Insert {
                index,
                kind,
                header: None,
            },
        );
    }
    let model = table_columns(&document, table).unwrap();
    let expander = &model.columns[2];
    let checks = [
        (&model.columns[0], "Binding", "{ReflectionBinding}"),
        (
            &model.columns[1],
            "Binding",
            "{ReflectionBinding IsChecked}",
        ),
        (expander, "ChildrenBinding", "{ReflectionBinding Children}"),
        (&expander.children[0], "Binding", "{ReflectionBinding}"),
    ];
    for (column, name, expression) in checks {
        let field = column
            .fields
            .iter()
            .find(|field| field.name == name)
            .unwrap();
        assert_eq!(field.value.as_deref(), Some(expression));
        assert!(field.editable);
        assert!(!field.can_clear);
        let clear = ColumnChange::Clear {
            column: column.node,
            field: name.into(),
        };
        assert!(plan_column_change(&document, table, &clear).is_err());
    }
}
