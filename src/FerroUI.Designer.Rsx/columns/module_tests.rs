use crate::RsxCatalog;
use ferroui_designer_core::{Document, TextPatch};
use ferroui_designer_project::{
    PreviewSourceBuffer, RustModuleSources, canonical_file_uri, file_uri,
};
use ferroui_designer_tools::{ColumnChange, ItemTemplateChange, ItemTemplateMember, table_columns};
use std::{fs, path::PathBuf};

struct Fixture(PathBuf);

impl Fixture {
    fn create(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ferroui-column-module-{name}-{}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn write(&self, name: &str, source: &str) -> String {
        let path = self.0.join(name);
        fs::write(&path, source).unwrap();
        canonical_file_uri(&file_uri(&path).unwrap()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn named_columns_edit_the_captured_companion_buffer() {
    let fixture = Fixture::create("columns");
    fixture.write("main.rs", "mod columns; mod review;\n");
    let disk = r#"use ferroui_view::{table_columns as list, table_column};
pub(crate) fn shared() -> Columns { stored() }
fn stored() -> Columns {
    list([table_column().header(/* named 🧭 */ "Account").binding_path("Customer.Name")])
}
"#
    .replace('\n', "\r\n");
    let columns = fixture.write("columns.rs", &disk);
    let draft = disk.replace("\"Account\"", "\"Draft account\"");
    let source = r#"use crate::columns::shared as accounts;
fn review() { rsx! { TableView { columns: accounts() } }; }
"#;
    let view = fixture.write("review.rs", source);
    let modules = RustModuleSources::capture(
        &fixture.0,
        &fixture.0.join("main.rs"),
        &view,
        source,
        &[PreviewSourceBuffer {
            uri: columns.clone(),
            source: Some(draft.clone()),
        }],
    )
    .unwrap();
    let catalog = RsxCatalog::built_in();
    let caret = source.find("TableView {").unwrap();
    let projection = catalog
        .project_with_modules(source, caret, Some(&modules))
        .unwrap();
    let document = Document::new(projection.xaml()).unwrap();
    let mut model = table_columns(&document, document.ast().root()).unwrap();
    let declaration = projection
        .map_column_declaration(&document, &mut model)
        .unwrap();
    assert_eq!(declaration.location.uri.as_deref(), Some(columns.as_str()));
    assert_eq!(declaration.factory.as_deref(), Some("stored"));
    assert!(draft[declaration.location.span.range()].starts_with("list(["));
    assert_eq!(model.columns[0].label, "Draft account");
    let edit = projection
        .plan_columns(
            &document,
            &model,
            &ColumnChange::Set {
                column: model.columns[0].node,
                field: "Header".into(),
                value: "Customer".into(),
            },
        )
        .unwrap();
    assert_eq!(edit.source, declaration.location);
    let changed = TextPatch::apply_all(&draft, edit.patches).unwrap();
    assert_eq!(changed, draft.replace("\"Draft account\"", "\"Customer\""));
    assert_eq!(
        fs::read_to_string(fixture.0.join("columns.rs")).unwrap(),
        disk
    );
    let modules = modules.with_file_text(&columns, &changed).unwrap();
    let updated = catalog
        .project_with_modules(source, caret, Some(&modules))
        .unwrap();
    assert_eq!(updated.invocation, projection.invocation);
    assert!(updated.xaml().contains("Header=\"Customer\""));
}

#[test]
fn a_cell_assignment_and_its_body_can_belong_to_different_companion_modules() {
    let fixture = Fixture::create("templates");
    fixture.write("main.rs", "mod columns; mod review; mod templates;\n");
    let column_source = r#"use ferroui_view::{table_columns, table_column};
use crate::templates::account_cell as cell;
pub fn columns() -> Columns {
    table_columns([table_column().header("Account").cell_template(cell())])
}
"#;
    let column_uri = fixture.write("columns.rs", column_source);
    let body_source = r#"pub fn account_cell() -> Template {
    ferroui_view::template::<String, _>(|value| rsx! { StackPanel {
        TextBlock { text: "Account 🧭" }
        TextBlock { text: value }
    } })
}
"#
    .replace('\n', "\r\n");
    let body_uri = fixture.write("templates.rs", &body_source);
    let source = r#"fn review() { rsx! { TableView { columns: crate::columns::columns() } }; }"#;
    let consumer = fixture.write("review.rs", source);
    let modules = RustModuleSources::capture(
        &fixture.0,
        &fixture.0.join("main.rs"),
        &consumer,
        source,
        &[],
    )
    .unwrap();
    let catalog = RsxCatalog::built_in();
    let caret = source.find("TableView").unwrap();
    let view = catalog
        .project_with_modules(source, caret, Some(&modules))
        .unwrap();
    let document = Document::new(view.xaml()).unwrap();
    let column = table_columns(&document, document.ast().root())
        .unwrap()
        .columns[0]
        .node;
    let slots = view.item_templates(&document, column);
    let slot = &slots[0];
    let assignment = slot.expression.as_ref().unwrap();
    assert_eq!(assignment.uri.as_ref(), Some(&column_uri));
    assert_eq!(&column_source[assignment.span.range()], "cell()");
    let body = slot.body.as_ref().unwrap();
    assert_eq!(body.factory.as_deref(), Some("account_cell"));
    assert_eq!(body.location.uri.as_ref(), Some(&body_uri));
    assert!(body_source[body.location.span.range()].contains("Account 🧭"));
    assert!(slot.can_reset);
    let plan = view
        .plan_item_template(
            &document,
            column,
            ItemTemplateMember::CellTemplate,
            &ItemTemplateChange::Reset,
            None,
        )
        .unwrap();
    assert_eq!(plan.source.uri, Some(column_uri.clone()));
    let reset = TextPatch::apply_all(column_source, plan.patches).unwrap();
    assert_eq!(reset, column_source.replace(".cell_template(cell())", ""));
    let modules = modules.with_file_text(&column_uri, &reset).unwrap();
    let view = catalog
        .project_with_modules(source, caret, Some(&modules))
        .unwrap();
    let plan = view
        .plan_item_template(
            &document,
            column,
            ItemTemplateMember::CellTemplate,
            &ItemTemplateChange::CreateInline,
            Some("String"),
        )
        .unwrap();
    assert_eq!(plan.source.uri, Some(column_uri));
    assert!(
        TextPatch::apply_all(&reset, plan.patches)
            .unwrap()
            .contains("ferroui_view::template::<String, _>")
    );
    assert_eq!(
        fs::read_to_string(fixture.0.join("templates.rs")).unwrap(),
        body_source
    );
}
