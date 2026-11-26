use super::*;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::{
    Button,
    testing::{TestServices, UnitTestApplication},
};

#[test]
fn studio_formatting_uses_the_selection_preferences_and_one_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = "<Grid><Button Content='🧪' Width='80'/><TextBlock><Run Text='A'/><Run Text='B'/></TextBlock></Grid>";
    let workspace = DesignerWorkspace::new(source).unwrap();
    let root = workspace.control();
    root.get_control::<ComboBox>("SourceFormatAttributes")
        .set_selected_index(2);
    root.get_control::<ComboBox>("SourceFormatClosingSpace")
        .set_selected_index(1);
    root.get_control::<TextBox>("SourceFormatTabSize")
        .set_text(Some("2"));

    let start = source.find("<Button").unwrap();
    let end = source.find("/>").unwrap() + 2;
    let range = LineIndex::new(source)
        .range(SourceSpan::new(start, end))
        .unwrap();
    let expected = source.replace(
        "<Button Content='🧪' Width='80'/>",
        "<Button\n    Content='🧪'\n    Width='80' />",
    );
    workspace.select_source_span(SourceSpan::new(start, end));
    root.get_control::<Button>("FormatSource")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let session = workspace.session();
    assert_eq!(session.borrow().authored_source(), expected);
    assert_eq!(
        session.borrow().document().undo_label(),
        Some("Format XAML")
    );
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    assert!(!session.borrow().can_undo());
    workspace.refresh(true);

    #[cfg(feature = "mcp")]
    {
        use ferroui_mcp::{Permissions, UiToolProvider};
        use serde_json::json;

        let mcp = crate::mcp::DesignerMcp::with_workspace(&workspace);
        let draft = mcp
            .call(
                "designer_language",
                &json!({
                    "action":"format", "range":range, "options":{"closingBracketOnNewLine":false},
                }),
                Permissions::default(),
            )
            .unwrap();
        assert_eq!(session.borrow().authored_source(), source);
        let apply = json!({"expected_revision":draft["revision"],"edit":draft["edit"]});
        let write = Permissions {
            allow_property_writes: true,
            allow_input: false,
        };
        mcp.call("designer_language_apply", &apply, write).unwrap();
        assert_eq!(session.borrow().authored_source(), expected);
        assert!(mcp.call("designer_language_apply", &apply, write).is_err());
        session.borrow_mut().undo().unwrap();
        assert_eq!(session.borrow().authored_source(), source);
        assert!(!session.borrow().can_undo());
    }
    #[cfg(not(feature = "mcp"))]
    let _ = range;
}
