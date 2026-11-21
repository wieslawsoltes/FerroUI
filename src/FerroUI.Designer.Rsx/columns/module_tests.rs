use crate::RsxCatalog;
use ferroui_designer_core::{Document, TextPatch};
use ferroui_designer_project::{
    PreviewSourceBuffer, RustModuleSources, canonical_file_uri, file_uri,
};
use ferroui_designer_tools::{ColumnChange, table_columns};
use std::{fs, path::PathBuf};

struct Fixture(PathBuf);

impl Fixture {
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
    let fixture =
        Fixture(std::env::temp_dir().join(format!("ferroui-column-module-{}", std::process::id())));
    fs::create_dir(&fixture.0).unwrap();
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
