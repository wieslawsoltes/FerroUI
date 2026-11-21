use crate::project_rust;
use ferroui_designer_core::{Document, TextPatch};
use ferroui_designer_tools::{ColumnChange, ColumnKind, table_columns};

fn change(
    source: &str,
    change: impl FnOnce(&ferroui_designer_tools::TableColumns) -> ColumnChange,
) -> Result<String, String> {
    let caret = source.find("TableView {").unwrap();
    let projection = project_rust(source, caret)?;
    let document = Document::new(projection.xaml()).map_err(|error| error.to_string())?;
    let table = document.ast().root();
    let mut model = table_columns(&document, table).map_err(|error| error.to_string())?;
    projection.map_column_model(&document, &mut model);
    let patches = projection.edit_columns(&document, &model, &change(&model))?;
    TextPatch::apply_all(source, patches).map_err(|error| error.to_string())
}

const SOURCE: &str = r##"fn other() -> Element { rsx! { TextBlock { "Before 🧭" } } }
fn view() -> Element { rsx! { TableView { name: "Accounts", columns: ::ferroui_view::table_columns([
    // Account name on the CRM overview
    ::ferroui_view::table_column().header(r#"Account 🪴"#).binding_path("Client.Name").width(::ferroui_controls::GridLength::from_pixels(120.0)),
    // Read-only member
    ::ferroui_view::table_column().header("Status").cell_template(status_template())
]), items_source: accounts() } } }
fn after() { let _ = "keep"; }
"##;

#[test]
fn literal_columns_render_and_edits_preserve_computed_fields_and_crlf() {
    let source = SOURCE.replace('\n', "\r\n");
    let projection = project_rust(&source, source.find("TableView {").unwrap()).unwrap();
    let markup = projection.preview_markup();
    assert_eq!(markup.controls, vec![0]);
    assert!(markup.source.contains("<TableView.Columns>"));
    assert!(
        markup
            .source
            .contains("Binding=\"{ReflectionBinding Client.Name}\"")
    );
    assert!(projection.has_projected_columns());
    assert!(!projection.can_replace_structure());
    let result = change(&source, |model| {
        assert_eq!(model.columns[0].label, "Account 🪴");
        assert_eq!(
            model.columns[0]
                .fields
                .iter()
                .find(|field| field.name == "Binding")
                .unwrap()
                .value
                .as_deref(),
            Some("Client.Name")
        );
        assert!(!model.columns[1].can_duplicate);
        ColumnChange::Set {
            column: model.columns[0].node,
            field: "Width".into(),
            value: "2*".into(),
        }
    })
    .unwrap();
    assert_eq!(
        result,
        source.replace(
            "from_pixels(120.0)",
            "new(2.0, ::ferroui_controls::GridUnitType::Star)"
        )
    );
    assert!(
        change(&source, |model| ColumnChange::Duplicate {
            column: model.columns[1].node
        })
        .is_err()
    );
    assert!(
        change(&source, |model| ColumnChange::Clear {
            column: model.columns[1].node,
            field: "CellTemplate".into()
        })
        .is_err()
    );
    let result = change(&source, |model| ColumnChange::Set {
        column: model.columns[0].node,
        field: "Binding".into(),
        value: "Contact.FullName".into(),
    })
    .unwrap();
    assert_eq!(result, source.replace("Client.Name", "Contact.FullName"));
}

#[test]
fn array_actions_keep_comments_and_surrounding_invocations() {
    let moved = change(SOURCE, |model| ColumnChange::Move {
        column: model.columns[1].node,
        index: 0,
    })
    .unwrap();
    assert!(moved.find("Read-only member").unwrap() < moved.find("Account name").unwrap());
    assert!(moved.contains("cell_template(status_template()),"));
    assert!(moved.starts_with(SOURCE.lines().next().unwrap()));
    assert!(moved.ends_with("fn after() { let _ = \"keep\"; }\n"));
    assert_eq!(
        change(&moved, |model| ColumnChange::Move {
            column: model.columns[0].node,
            index: 1
        })
        .unwrap(),
        SOURCE
    );
    let inserted = change(SOURCE, |_| ColumnChange::Insert {
        index: 1,
        kind: ColumnKind::Table,
        header: Some("New".into()),
    })
    .unwrap();
    assert!(inserted.contains(".header(\"New\"),\n    // Read-only member"));
    let duplicated = change(&inserted, |model| ColumnChange::Duplicate {
        column: model.columns[0].node,
    })
    .unwrap();
    assert_eq!(
        duplicated.matches(".binding_path(\"Client.Name\")").count(),
        2
    );
    let without_last = change(&duplicated, |model| ColumnChange::Remove {
        column: model.columns.last().unwrap().node,
    })
    .unwrap();
    assert!(!without_last.contains("status_template()"));
    assert!(without_last.contains("// Read-only member"));
}

#[test]
fn create_first_column_and_protect_computed_collection_values() {
    let source = "fn view() { let view = rsx! { TableView { name: \"Tasks\" } }; }";
    let add = |_: &ferroui_designer_tools::TableColumns| ColumnChange::Insert {
        index: 0,
        kind: ColumnKind::Table,
        header: None,
    };
    let created = change(source, add).unwrap();
    assert!(created.contains("columns: ::ferroui_view::table_columns([::ferroui_view::table_column().header(\"Column\")])"));
    let empty = change(&created, |model| ColumnChange::Remove {
        column: model.columns[0].node,
    })
    .unwrap();
    assert_eq!(
        change(&empty, add)
            .unwrap()
            .matches("table_column()")
            .count(),
        1
    );
    let computed = source.replace("name:", "columns: account_columns(), name:");
    assert!(
        change(&computed, add)
            .unwrap_err()
            .contains("defined by Rust")
    );
}

#[test]
fn moving_a_table_on_the_canvas_retains_the_exact_column_source() {
    use ferroui_designer_core::Edit;

    let source = r#"rsx! { StackPanel {
        Button { name: "Action" }
        TableView { name: "Table", columns: ferroui_view::table_columns([
            /* total column */ ferroui_view::table_column().header("Total").binding_path("Amount"),
        ]) }
    } }"#;
    let projection = project_rust(source, 0).unwrap();
    let original = Document::new(projection.xaml()).unwrap();
    let table = original.ast().find_name("Table").unwrap();
    let edits = [Edit::MoveNode {
        node: table,
        parent: original.ast().root(),
        index: 0,
    }];
    let mut draft = original.snapshot_without_history();
    draft.apply("Move table", edits.to_vec()).unwrap();
    let plan = projection
        .plan_child_edit(&original, &draft, &edits, None)
        .unwrap();
    let updated = TextPatch::apply_all(source, plan.patches).unwrap();
    assert!(updated.find("name: \"Table\"").unwrap() < updated.find("name: \"Action\"").unwrap());
    let columns = source
        .split_once("columns:")
        .unwrap()
        .1
        .split_once("])")
        .unwrap()
        .0;
    assert!(updated.contains(columns));
}
