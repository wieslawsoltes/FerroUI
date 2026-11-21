use crate::{RsxProjection, project_rust};
use ferroui_designer_core::{Document, NodeId, TextPatch};
use ferroui_designer_tools::{ItemTemplateChange, ItemTemplateMember::*};

const VIEW: &str = r#"fn unrelated() -> Element { rsx! { TextBlock { "🧭 First" } } }
fn customers() -> Element {
    rsx! { ListBox {
        items_source: customers_at_runtime(),
        item_template: ::ferroui_view::template(|customer: Customer| {
            let detail = rsx! { TextBlock { "Independent" } };
            // Keep this comment with the Rust.
            ::ferroui_view::rsx! { StackPanel {
                TextBlock { text: "Customer 🪴" }
                {detail}
                TextBlock { text: customer.account_name }
            } }
        }),
        name: "Customers"
    } }
}
fn afterward() { let _ = "unchanged"; }
"#;

fn owner<'a>(source: &'a str, name: &str) -> (RsxProjection<'a>, Document, NodeId) {
    let view = project_rust(source, source.find(name).unwrap()).unwrap();
    let document = Document::new(view.xaml()).unwrap();
    let node = document.ast().root();
    (view, document, node)
}

#[test]
fn exact_returned_body_edits_without_rewriting_rust() {
    let source = VIEW.replace('\n', "\r\n");
    let (view, document, node) = owner(&source, "ListBox {");
    let slots = view.item_templates(&document, node);
    assert_eq!(slots.len(), 1);
    let slot = &slots[0];
    assert!(slot.can_reset);
    let body = slot.body.unwrap();
    assert!(source[body.range()].starts_with("rsx! { StackPanel"));
    assert!(!view.can_replace_structure());
    let child = project_rust(&source, body.start).unwrap();
    let patch = child.set_property(1, "Text", "Accounts 🪴").unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, vec![patch]).unwrap(),
        source.replace("Customer 🪴", "Accounts 🪴")
    );
    let reset = view
        .edit_item_template(
            &document,
            node,
            ItemTemplate,
            &ItemTemplateChange::Reset,
            None,
        )
        .unwrap();
    let reset = TextPatch::apply_all(&source, reset).unwrap();
    assert!(reset.contains("// Keep this comment with the Rust.\r\n"));
    assert!(reset.contains("items_source: customers_at_runtime(),\r\n"));
    assert!(reset.ends_with("fn afterward() { let _ = \"unchanged\"; }\r\n"));
    let (view, document, node) = owner(&reset, "ListBox {");
    assert!(view.item_templates(&document, node)[0].can_create);
}

#[test]
fn creates_in_empty_item_slot_and_protects_computed_or_duplicate_assignments() {
    let source = "fn view() -> Element { rsx! { ComboBox { name: \"Customer\" } } }";
    let (view, document, node) = owner(source, "ComboBox {");
    assert_eq!(view.item_templates(&document, node).len(), 2);
    let create = |row_type| {
        view.edit_item_template(
            &document,
            node,
            SelectionBoxItemTemplate,
            &ItemTemplateChange::CreateInline,
            row_type,
        )
    };
    assert!(create(Some("crate::Customer>>")).is_err());
    assert!(create(None).is_err());
    let next = TextPatch::apply_all(source, create(Some("crate::Customer")).unwrap()).unwrap();
    assert!(
        next.contains(
            "selection_box_item_template: ::ferroui_view::template::<crate::Customer, _>"
        )
    );
    let (view, document, node) = owner(&next, "ComboBox {");
    assert!(view.item_templates(&document, node)[1].body.is_some());

    for assignment in [
        "item_template: choose_template()",
        "item_template: my_lib::template(|_: String| rsx! { TextBlock {} })",
        "item_template: earlier(), item_template: ::ferroui_view::template(|_: String| rsx! { TextBlock {} })",
    ] {
        let source = format!("fn view() -> Element {{ rsx! {{ ListBox {{ {assignment} }} }} }}");
        let (view, document, node) = owner(&source, "ListBox {");
        let slots = view.item_templates(&document, node);
        assert!(slots[0].expression.is_some());
        assert!(slots[0].body.is_none());
        assert!(!slots[0].can_reset && !slots[0].can_create);
    }
    let source =
        "rsx! { ListBox { prop(ItemsControl::item_template_property(), choose_template()) } }";
    let (view, document, node) = owner(source, "ListBox {");
    assert!(!view.item_templates(&document, node)[0].can_create);
}

#[test]
fn cell_create_preview_and_reset_preserve_column_and_source_order() {
    let source = r#"fn view() -> Element { rsx! { TableView { columns: ::ferroui_view::table_columns([
        ::ferroui_view::table_column().header("Name"),
        ::ferroui_view::table_column().header("Status").cell_template(runtime_template()),
    ]) } } }"#;
    let (view, document, table) = owner(source, "TableView {");
    let columns = ferroui_designer_tools::table_columns(&document, table)
        .unwrap()
        .columns;
    assert!(!view.item_templates(&document, columns[1].node)[0].can_reset);
    let create = view
        .edit_item_template(
            &document,
            columns[0].node,
            CellTemplate,
            &ItemTemplateChange::CreateInline,
            Some("Customer"),
        )
        .unwrap();
    let next = TextPatch::apply_all(source, create).unwrap();
    let (view, document, table) = owner(&next, "TableView {");
    let column = ferroui_designer_tools::table_columns(&document, table)
        .unwrap()
        .columns[0]
        .node;
    let slots = view.item_templates(&document, column);
    assert!(next[slots[0].body.unwrap().range()].contains("\"Cell\""));
    let reset = view
        .edit_item_template(
            &document,
            column,
            CellTemplate,
            &ItemTemplateChange::Reset,
            None,
        )
        .unwrap();
    assert_eq!(TextPatch::apply_all(&next, reset).unwrap(), source);
}

#[test]
fn resets_builder_call_with_comment_and_guards_qualified_assignments() {
    let source = r#"rsx! { ListBox {
        item_template(::ferroui_view::template(|_: String| rsx! { TextBlock { "Row" } }))
            /* Keep the 🧭 note. */,
        name: "Directory"
    } }"#
        .replace('\n', "\r\n");
    let (view, document, node) = owner(&source, "ListBox {");
    let patch = view
        .edit_item_template(
            &document,
            node,
            ItemTemplate,
            &ItemTemplateChange::Reset,
            None,
        )
        .unwrap();
    let reset = TextPatch::apply_all(&source, patch).unwrap();
    assert!(!reset.contains("item_template"));
    assert!(reset.contains("/* Keep the 🧭 note. */"));
    assert!(reset.contains("name: \"Directory\"\r\n"));
    for field in ["ItemsControl.item_template: compute()", "item_template()"] {
        let source = format!("rsx! {{ ListBox {{ {field} }} }}");
        let (view, document, node) = owner(&source, "ListBox {");
        assert!(!view.item_templates(&document, node)[0].can_create);
    }
}
