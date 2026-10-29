use super::*;
use crate::{PreviewEnvironment, PreviewStageConfiguration, PreviewStageSize, PreviewStageStatus};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_core::Document;

fn stages() -> Vec<PreviewStageInfo> {
    [(20, "Phone"), (30, "Desktop")]
        .into_iter()
        .map(|(id, name)| PreviewStageInfo {
            id,
            configuration: PreviewStageConfiguration::new(
                name,
                "memory://shared-view.xaml",
                PreviewEnvironment::default(),
            ),
            epoch: 1,
            status: PreviewStageStatus::Ready { revision: 0 },
            layout_size: PreviewStageSize {
                width: 400.0,
                height: 240.0,
            },
            rendered_revision: Some(0),
            source_revision: Some(0),
            notices: Vec::new(),
        })
        .collect()
}

fn elements() -> Vec<PreviewPrototypeElementChoice> {
    let document =
        Document::new("<Canvas xmlns='https://github.com/ferroui'><Button Name='Open'/></Canvas>")
            .unwrap();
    let anchor =
        PreviewSourceAnchor::capture(&document, document.ast().find_name("Open").unwrap()).unwrap();
    [20, 30]
        .into_iter()
        .map(|stage| PreviewPrototypeElementChoice {
            stage,
            label: "Button · Open".into(),
            anchor: anchor.clone(),
        })
        .collect()
}

#[test]
fn missing_flow_start_requires_explicit_selection_before_apply() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = PreviewPrototypeFlowEditor::new(
        Some(PreviewPrototypeFlow {
            name: "Existing journey".into(),
            start: 10,
        }),
        stages(),
    );
    let accepted = Rc::new(RefCell::new(Vec::new()));
    let submitted = accepted.clone();
    editor.on_apply(move |flow| {
        submitted.borrow_mut().push(flow);
        Ok(())
    });

    assert_eq!(editor.start.selected_index(), -1);
    assert!(editor.apply().unwrap_err().contains("starting screen"));
    assert!(accepted.borrow().is_empty());

    editor.start.set_selected_index(1);
    editor.apply().unwrap();
    assert_eq!(accepted.borrow()[0].start, 30);
    assert_eq!(accepted.borrow()[0].name, "Existing journey");
}

#[test]
fn missing_link_source_cannot_retarget_a_compatible_anchor_in_another_stage() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let choices = elements();
    let action = PreviewPrototypeAction::OpenOverlay {
        stage: 30,
        options: PreviewPrototypeOverlayOptions::default(),
    };
    let editor = PreviewPrototypeLinkEditor::new(
        Some(PreviewPrototypeLink {
            name: "Open settings".into(),
            source: 10,
            trigger: choices[0].anchor.clone(),
            action,
        }),
        None,
        stages(),
        choices,
    );
    let accepted = Rc::new(RefCell::new(Vec::new()));
    let submitted = accepted.clone();
    editor.on_apply(move |link| {
        submitted.borrow_mut().push(link);
        Ok(())
    });

    assert_eq!(editor.source.selected_index(), -1);
    assert_eq!(editor.element.selected_index(), -1);
    assert!(editor.apply().unwrap_err().contains("source screen"));
    assert!(accepted.borrow().is_empty());

    // The surviving stages share the same authored anchor. Choosing the stage
    // must still be explicit, followed by choosing its available source element.
    editor.source.set_selected_index(0);
    assert_eq!(editor.element.selected_index(), -1);
    editor.element.set_selected_index(0);
    editor.apply().unwrap();
    assert_eq!(accepted.borrow()[0].source, 20);
    assert_eq!(accepted.borrow()[0].action, action);
}

#[test]
fn missing_requested_source_does_not_default_a_new_link_to_another_stage() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = PreviewPrototypeLinkEditor::new(None, Some(10), stages(), elements());
    editor.name.set_text(Some("New interaction"));
    let submitted = Rc::new(Cell::new(false));
    let called = submitted.clone();
    editor.on_apply(move |_| {
        called.set(true);
        Ok(())
    });

    assert_eq!(editor.source.selected_index(), -1);
    assert_eq!(editor.element.selected_index(), -1);
    assert!(editor.apply().unwrap_err().contains("source screen"));
    assert!(!submitted.get());
}

#[test]
fn new_editors_without_requested_stages_keep_the_first_stage_default() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let flow = PreviewPrototypeFlowEditor::new(None, stages());
    flow.name.set_text(Some("New journey"));
    assert_eq!(flow.value().unwrap().start, 20);

    let link = PreviewPrototypeLinkEditor::new(None, None, stages(), elements());
    link.name.set_text(Some("New interaction"));
    let value = link.value().unwrap();
    assert_eq!(value.source, 20);
    assert_eq!(value.trigger, elements()[0].anchor);
}
