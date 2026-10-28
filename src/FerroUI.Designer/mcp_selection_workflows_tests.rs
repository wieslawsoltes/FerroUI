use super::*;
use crate::mcp::{DesignerMcp, Permissions};
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, ContentControl, Control, Window};
use ferroui_mcp::UiToolProvider;
use std::cell::RefCell;
use std::rc::Rc;

const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="600" Height="400">
  <Border Name="Source" Width="180" Height="90" Background="Red" BorderBrush="Blue" CornerRadius="12"/>
  <Border Name="Target" Canvas.Left="220" Width="100" Height="50" Background="White"/>
  <TextBlock Name="Caption" Canvas.Top="160" Text="Keep the content" FontSize="18"/>
</Canvas>"#;

struct Fixture {
    session: Rc<RefCell<DesignerSession>>,
    provider: DesignerMcp,
    window: Ref<Window>,
}

impl Fixture {
    fn new() -> Self {
        let host = ContentControl::new();
        let session = Rc::new(RefCell::new(
            DesignerSession::new(SOURCE, host.clone()).unwrap(),
        ));
        let window = Window::new();
        window.set_width(600.0);
        window.set_height(400.0);
        window.set_content(Some(Control::boxed(host)));
        window.show();
        window.update_layout();
        session.borrow_mut().refresh_runtime_mapping();
        Self {
            provider: DesignerMcp::new(&session),
            session,
            window,
        }
    }

    fn node(&self, name: &str) -> u64 {
        self.session
            .borrow()
            .document()
            .ast()
            .find_name(name)
            .unwrap()
    }

    fn call(&self, name: &str, args: Value, write: bool) -> Result<Value, String> {
        self.provider.call(
            name,
            &args,
            Permissions {
                allow_property_writes: write,
                allow_input: false,
            },
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn selection_authoring_schemas_reject_unreviewed_or_mixed_requests() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    for (name, args) in [
        ("designer_selection_layout", json!({"operation":"unknown"})),
        (
            "designer_selection_layout",
            json!({"operation":"reset_width","nodes":[1]}),
        ),
        (
            "designer_selection_layout_edit",
            json!({"operation":"reset_width","expected_revision":0}),
        ),
        (
            "designer_selection_layout_edit",
            json!({"operation":"reset_width","expected_revision":-1,"expected_model":"x"}),
        ),
        ("designer_property_transfer", json!({"source":0})),
        (
            "designer_property_transfer",
            json!({"source":1,"properties":[1]}),
        ),
        (
            "designer_property_transfer",
            json!({"source":1,"properties":vec!["Background";65]}),
        ),
        (
            "designer_property_transfer_edit",
            json!({"source":1,"properties":[],"expected_revision":0}),
        ),
        (
            "designer_property_transfer_edit",
            json!({"source":1,"properties":[],"expected_revision":0,"expected_model":"x","edits":[]}),
        ),
    ] {
        assert!(
            fixture.provider.validate(name, &args).is_err(),
            "{name}: {args}"
        );
    }
}

#[test]
fn property_transfer_plans_are_observational_and_commit_one_reviewed_transaction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let source = fixture.node("Source");
    let target = fixture.node("Target");
    let caption = fixture.node("Caption");
    fixture.session.borrow_mut().select([target, caption]);
    let generation = fixture.session.borrow().generation();
    let report = fixture
        .call(
            "designer_property_transfer",
            json!({"source":source}),
            false,
        )
        .unwrap();
    assert_eq!(fixture.session.borrow().document().source(), SOURCE);
    assert_eq!(fixture.session.borrow().generation(), generation);
    assert_eq!(fixture.session.borrow().selection(), [target, caption]);
    assert!(report["paste_skipped"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| { issue["target"] == caption && issue["property"] == "CornerRadius" }));
    let args = json!({
        "source":source,"properties":report["properties"],
        "expected_revision":report["revision"],"expected_model":report["model"]
    });
    assert!(fixture
        .call("designer_property_transfer_edit", args.clone(), false)
        .is_err());
    fixture
        .call("designer_property_transfer_edit", args.clone(), true)
        .unwrap();
    let session = fixture.session.borrow();
    assert_eq!(
        session.document().revision(),
        report["revision"].as_u64().unwrap() + 1
    );
    let node = session.document().ast().node(target).unwrap();
    assert_eq!(node.attribute("Background"), Some("Red"));
    assert_eq!(node.attribute("Width"), Some("100"));
    assert_eq!(
        session
            .document()
            .ast()
            .node(caption)
            .unwrap()
            .attribute("Text"),
        Some("Keep the content")
    );
    drop(session);
    assert!(fixture
        .call("designer_property_transfer_edit", args, true)
        .is_err());
    fixture.session.borrow_mut().undo().unwrap();
    assert_eq!(fixture.session.borrow().document().source(), SOURCE);
    assert!(!fixture.session.borrow().can_undo());
}

#[test]
fn transfer_model_rejects_selection_roundtrips_and_viewer_writes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let source = fixture.node("Source");
    let target = fixture.node("Target");
    fixture.session.borrow_mut().select([target]);
    let report = fixture
        .call(
            "designer_property_transfer",
            json!({"source":source,"properties":["Background"]}),
            false,
        )
        .unwrap();
    let args = json!({"source":source,"properties":["Background"],"expected_revision":report["revision"],"expected_model":report["model"]});
    fixture.session.borrow_mut().select([source]);
    fixture.session.borrow_mut().select([target]);
    assert!(fixture
        .call("designer_property_transfer_edit", args, true)
        .is_err());
    fixture.session.borrow_mut().set_read_only(true);
    let report = fixture
        .call(
            "designer_property_transfer",
            json!({"source":source,"properties":["Background"]}),
            false,
        )
        .unwrap();
    let args = json!({"source":source,"properties":["Background"],"expected_revision":report["revision"],"expected_model":report["model"]});
    assert!(fixture
        .call("designer_property_transfer_edit", args, true)
        .is_err());
    assert_eq!(fixture.session.borrow().document().source(), SOURCE);
    assert!(!fixture.session.borrow().can_undo());
}

#[test]
fn transfer_model_rejects_same_source_native_reconstruction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let source = fixture.node("Source");
    let target = fixture.node("Target");
    fixture.session.borrow_mut().select([target]);
    let report = fixture
        .call(
            "designer_property_transfer",
            json!({"source":source}),
            false,
        )
        .unwrap();
    assert!(
        !report["edits"].as_array().unwrap().is_empty(),
        "The reviewed model includes exact validated writes"
    );
    let args = json!({"source":source,"properties":report["properties"],"expected_revision":report["revision"],"expected_model":report["model"]});
    let (document, host, uri, generation, old_target) = {
        let session = fixture.session.borrow();
        (
            session.document().clone(),
            session.preview_host(),
            session.document_uri().to_owned(),
            session.generation(),
            session.preview.visual(target).unwrap(),
        )
    };
    // Release the original reload registration before rebuilding the same URI.
    // Keeping two concurrent sessions for one document is independently rejected.
    let temporary = crate::runtime::RuntimePreview::new(
        ContentControl::new(),
        format!("{uri}.replacement-fixture.xaml"),
        &document,
    )
    .unwrap();
    let previous = std::mem::replace(&mut fixture.session.borrow_mut().preview, temporary);
    drop(previous);
    let replacement = crate::runtime::RuntimePreview::new(host, uri, &document).unwrap();
    fixture.session.borrow_mut().preview = replacement;
    fixture.window.update_layout();
    fixture.session.borrow_mut().refresh_runtime_mapping();
    assert_eq!(fixture.session.borrow().generation(), generation);
    assert!(!std::ptr::eq(
        &*old_target,
        &*fixture.session.borrow().preview.visual(target).unwrap()
    ));
    assert!(fixture
        .call("designer_property_transfer_edit", args, true)
        .is_err());
    assert_eq!(fixture.session.borrow().document().source(), SOURCE);
    assert!(!fixture.session.borrow().can_undo());
}

#[test]
fn layout_model_rechecks_native_measurements_and_reset_removes_only_requested_values() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let source = fixture.node("Source");
    let target = fixture.node("Target");
    fixture.session.borrow_mut().select([source, target]);
    let report = fixture
        .call(
            "designer_selection_layout",
            json!({"operation":"match_width"}),
            false,
        )
        .unwrap();
    let args = json!({"operation":"match_width","expected_revision":report["revision"],"expected_model":report["model"]});
    let visual = fixture
        .session
        .borrow()
        .preview
        .visual(source)
        .unwrap()
        .cast::<Border>()
        .unwrap();
    visual.set_width(210.0);
    fixture.window.update_layout();
    assert_eq!(fixture.session.borrow().document().source(), SOURCE);
    assert!(fixture
        .call("designer_selection_layout_edit", args, true)
        .is_err());
    let report = fixture
        .call(
            "designer_selection_layout",
            json!({"operation":"match_width"}),
            false,
        )
        .unwrap();
    let args = json!({"operation":"match_width","expected_revision":report["revision"],"expected_model":report["model"]});
    assert!(fixture
        .call("designer_selection_layout_edit", args.clone(), false)
        .is_err());
    fixture
        .call("designer_selection_layout_edit", args, true)
        .unwrap();
    assert_eq!(
        fixture
            .session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Width"),
        Some("210")
    );
    fixture.session.borrow_mut().undo().unwrap();
    fixture.session.borrow_mut().select([source, target]);
    let report = fixture
        .call(
            "designer_selection_layout",
            json!({"operation":"reset_width"}),
            false,
        )
        .unwrap();
    fixture.call("designer_selection_layout_edit", json!({
        "operation":"reset_width","expected_revision":report["revision"],"expected_model":report["model"]
    }), true).unwrap();
    for (node, height) in [(source, "90"), (target, "50")] {
        let session = fixture.session.borrow();
        let node = session.document().ast().node(node).unwrap();
        assert!(node.attribute("Width").is_none());
        assert_eq!(node.attribute("Height"), Some(height));
    }
    fixture.session.borrow_mut().undo().unwrap();
    assert_eq!(fixture.session.borrow().document().source(), SOURCE);
}

#[test]
fn viewer_can_read_layout_plans_but_cannot_apply_nonempty_or_noop_commands() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let source = fixture.node("Source");
    let target = fixture.node("Target");
    fixture.session.borrow_mut().select([source, target]);
    fixture.session.borrow_mut().set_read_only(true);
    let generation = fixture.session.borrow().generation();
    let revision = fixture.session.borrow().document().revision();
    for (operation, changes) in [
        ("reset_width", true),
        ("reset_margin", false),
        ("match_width", true),
    ] {
        let report = fixture
            .call(
                "designer_selection_layout",
                json!({"operation":operation}),
                false,
            )
            .unwrap();
        assert_eq!(report["edit_count"].as_u64().unwrap() > 0, changes);
        let error = fixture.call("designer_selection_layout_edit", json!({
            "operation":operation, "expected_revision":report["revision"], "expected_model":report["model"]
        }), true).unwrap_err();
        assert!(error.contains("read-only"), "{operation}: {error}");
    }
    assert_eq!(fixture.session.borrow().generation(), generation);
    assert_eq!(fixture.session.borrow().document().revision(), revision);
    assert_eq!(fixture.session.borrow().document().source(), SOURCE);
    assert!(!fixture.session.borrow().can_undo());
}

#[test]
fn same_type_selection_uses_existing_layer_revision_and_epoch_contracts() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let source = fixture.node("Source");
    let target = fixture.node("Target");
    fixture.session.borrow_mut().select([target]);
    fixture.session.borrow_mut().set_read_only(true);
    let layers = fixture.call("designer_layers", json!({}), false).unwrap();
    let args = json!({
        "action":"select_matching_type","expected_revision":layers["revision"],
        "expected_layer_epoch":layers["layer_epoch"],"expected_scope_epoch":layers["scope_epoch"]
    });
    fixture.call("designer_layer", args, true).unwrap();
    assert_eq!(fixture.session.borrow().selection(), [target, source]);
    assert_eq!(fixture.session.borrow().document().source(), SOURCE);
    assert!(!fixture.session.borrow().can_undo());
}

#[test]
fn workspace_selection_tools_preserve_pending_source_drafts() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = crate::DesignerWorkspace::new(SOURCE).unwrap();
    let session = workspace.session();
    let source = session
        .borrow()
        .document()
        .ast()
        .find_name("Source")
        .unwrap();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    session.borrow_mut().select([target]);
    workspace.refresh(true);
    let provider = DesignerMcp::with_workspace(&workspace);
    let draft = format!("{SOURCE}\n<!-- pending source edit -->");
    workspace.source_editor().set_text(Some(&draft));
    let revision = session.borrow().document().revision();
    let generation = session.borrow().generation();
    for (name, args) in [
        ("designer_property_transfer", json!({"source":source})),
        (
            "designer_selection_layout",
            json!({"operation":"reset_width"}),
        ),
    ] {
        let error = provider
            .call(
                name,
                &args,
                Permissions {
                    allow_property_writes: true,
                    allow_input: false,
                },
            )
            .unwrap_err();
        assert!(error.contains("Apply or cancel the XAML draft"), "{error}");
    }
    assert_eq!(
        workspace.source_editor().text().as_deref(),
        Some(draft.as_str())
    );
    assert_eq!(session.borrow().document().source(), draft);
    assert_eq!(session.borrow().document().revision(), revision);
    assert_eq!(session.borrow().generation(), generation);
}
