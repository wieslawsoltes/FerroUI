use crate::mcp::{DesignerMcp, Permissions};
use crate::DesignerSession;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::ContentControl;
use ferroui_mcp::UiToolProvider;
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn binding_fields_match_studio_and_keep_untouched_source_in_one_edit() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let original = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:f="https://github.com/ferroui">
    <TextBox Name="Input" Text="Café" />
    <!-- preserve -->
    <TextBlock Name="Output" Text="{f:Binding  Text, ElementName=Input, FallbackValue='None' }" />
    <Button Name="Button" Content="Click" />
</StackPanel>"#;
    let session = Rc::new(RefCell::new(
        DesignerSession::new(original, ContentControl::new()).unwrap(),
    ));
    let output = session
        .borrow()
        .document()
        .ast()
        .find_name("Output")
        .unwrap();
    let provider = DesignerMcp::new(&session);
    let result = provider
        .call(
            "designer_binding",
            &json!({"node":output,"property":"Text"}),
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(result["extension"], "f:Binding");
    assert_eq!(
        result["fields"]["source"],
        json!({"kind":"element","name":"Input"})
    );
    assert_eq!(
        result["options"]["FallbackValue"],
        json!({"kind":"text","value":"None"})
    );
    assert!(result["elementNames"]
        .as_array()
        .unwrap()
        .contains(&json!("Input")));
    let mut invalid = json!({"node":output,"property":"Text","expected_revision":result["revision"],
        "changes":[{"field":"path","value":"Ignored"},{"field":"option","name":"Converter","value":{"kind":"text","value":"Wrong"}}]});
    let writes = Permissions {
        allow_property_writes: true,
        ..Default::default()
    };
    assert!(provider
        .call("designer_binding_edit", &invalid, writes)
        .is_err());
    assert_eq!(session.borrow().document().source(), original);
    invalid["changes"] = json!([{"field":"mode","value":"OneTime"},{"field":"delay","value":25}]);
    assert!(provider
        .call("designer_binding_edit", &invalid, Permissions::default())
        .is_err());
    provider
        .call("designer_binding_edit", &invalid, writes)
        .unwrap();

    let source = session.borrow();
    assert_eq!(
        source.document().source(),
        original.replace(
            "FallbackValue='None' }",
            "FallbackValue='None' , Mode='OneTime', Delay='25'}"
        )
    );
    drop(source);
    assert!(provider
        .call("designer_binding_edit", &invalid, writes)
        .unwrap_err()
        .contains("revision"));
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), original);
}

#[test]
fn replacement_and_relative_source_require_a_current_writable_attribute() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let original = r#"<StackPanel xmlns="https://github.com/ferroui">
    <TextBlock Name="Output" Text="Literal" />
    <Button Name="Button" Click="Tap" />
</StackPanel>"#;
    let session = Rc::new(RefCell::new(
        DesignerSession::new(original, ContentControl::new()).unwrap(),
    ));
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Output")
        .unwrap();
    let button = session
        .borrow()
        .document()
        .ast()
        .find_name("Button")
        .unwrap();
    let provider = DesignerMcp::new(&session);
    let result = provider
        .call(
            "designer_binding",
            &json!({"node":target,"property":"Text"}),
            Permissions::default(),
        )
        .unwrap();
    assert!(result["fields"].is_null());
    assert_eq!(result["authored"], "Literal");
    assert_eq!(result["relativeSources"], true);
    let mut edit = json!({"node":target,"property":"Text","expected_revision":result["revision"],"changes":[
        {"field":"source","value":{"kind":"ancestor","typeName":"StackPanel","level":1,"logicalTree":true}},
        {"field":"path","value":"Name"}
    ]});
    let writes = Permissions {
        allow_property_writes: true,
        ..Default::default()
    };
    assert!(provider
        .call("designer_binding_edit", &edit, writes)
        .is_err());
    assert_eq!(session.borrow().document().source(), original);
    edit["replace"] = json!(true);
    provider
        .call("designer_binding_edit", &edit, writes)
        .unwrap();
    let result = provider
        .call(
            "designer_binding",
            &json!({"node":target,"property":"Text"}),
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(result["fields"]["source"], edit["changes"][0]["value"]);
    assert!(provider
        .call(
            "designer_binding",
            &json!({"node":button,"property":"Click"}),
            Permissions::default()
        )
        .is_err());
}
