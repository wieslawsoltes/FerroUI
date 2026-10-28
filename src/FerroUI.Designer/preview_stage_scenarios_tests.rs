use super::*;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, Control, Window};
use ferroui_designer_animation::{Easing, StateSetter, StateTransition, VisualState};
use ferroui_designer_core::Edit;
use std::sync::Arc;

fn fixture() -> Document {
    let mut document = Document::new(
        r#"<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Name="Owner" Width="400" Height="260">
  <Canvas.Resources><x:Double x:Key="ScenarioOpacity">0.35</x:Double></Canvas.Resources>
  <Button Name="A" Width="100" Height="40" Opacity="1"/>
  <Button Name="B" Width="80" Height="40" Canvas.Top="60"/>
  <Button Name="Reference" Width="210" Height="40" Opacity="0.65" Canvas.Top="120"/>
</Canvas>"#,
    )
    .unwrap();
    let owner = document.ast().root();
    let screen = group(
        owner,
        "Screen",
        "A",
        "Opacity",
        &[
            ("Loading", "0.2"),
            ("Content", "1"),
            ("Resource", "{StaticResource ScenarioOpacity}"),
            ("Bound", "{Binding Opacity, ElementName=Reference}"),
        ],
        &[("Resource", 0.3), ("Bound", 0.8)],
    );
    let details = group(
        owner,
        "Details",
        "B",
        "Width",
        &[
            ("Compact", "80"),
            ("Expanded", "160"),
            ("Bound", "{Binding Width, ElementName=Reference}"),
        ],
        &[("Bound", 0.5)],
    );
    for group in [screen, details] {
        document
            .apply("Fixture states", group.plan(&document).unwrap())
            .unwrap();
    }
    document.mark_saved();
    document
}

fn group(
    owner: NodeId,
    name: &str,
    target: &str,
    property: &str,
    states: &[(&str, &str)],
    transitions: &[(&str, f64)],
) -> VisualStateGroup {
    VisualStateGroup {
        owner,
        name: name.into(),
        states: states
            .iter()
            .map(|(name, value)| VisualState {
                name: (*name).into(),
                setters: vec![StateSetter {
                    target: target.into(),
                    property: property.into(),
                    value: (*value).into(),
                }],
            })
            .collect(),
        transitions: transitions
            .iter()
            .map(|(to, duration)| StateTransition {
                from: None,
                to: (*to).into(),
                duration: *duration,
                easing: Easing::Linear,
            })
            .collect(),
    }
}

fn preview(document: &Document) -> RuntimePreview {
    // Stages intentionally share the original source URI, but not a native graph or
    // a hot-reload source owner. Resource and binding context remains stage-local.
    let preview = RuntimePreview::new_isolated(
        ContentControl::new(),
        "memory://preview-stage-scenarios.xaml".into(),
        document,
        Default::default(),
        None,
    )
    .unwrap();
    assert!(
        matches!(preview.status, PreviewStatus::Ready { .. }),
        "{:?}",
        preview.status
    );
    preview
}

fn attach(preview: &mut RuntimePreview, document: &Document) -> Ref<Window> {
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(260.0);
    window.set_content(Some(Control::boxed(&preview.host)));
    window.show();
    window.update_layout();
    preview.rebuild_map(document);
    window
}

fn selection<'a>(document: &Document, group: &'a str, state: &'a str) -> ResolvedStageState<'a> {
    ResolvedStageState {
        owner: document.ast().root(),
        group,
        state,
    }
}

fn button(preview: &RuntimePreview, document: &Document, name: &str) -> Ref<Button> {
    preview
        .visual(document.ast().find_name(name).unwrap())
        .unwrap()
        .cast::<Button>()
        .unwrap()
}

fn classes(preview: &RuntimePreview, document: &Document) -> Rc<Vec<String>> {
    preview
        .visual(document.ast().root())
        .unwrap()
        .cast::<StyledElement>()
        .unwrap()
        .classes()
        .snapshot()
}

fn error(result: Result<StageScenarioGuard, String>) -> String {
    match result {
        Ok(_) => panic!("The scenario should have been rejected"),
        Err(message) => message,
    }
}

#[test]
fn native_stages_hold_independent_group_endpoints_without_changing_the_document() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = fixture();
    let before = (
        document.source().to_owned(),
        document.revision(),
        document.ast_snapshot(),
        document.undo_label().map(str::to_owned),
        document.is_dirty(),
    );
    let mut loading = preview(&document);
    let mut content = preview(&document);
    let first_window = attach(&mut loading, &document);
    let second_window = attach(&mut content, &document);
    let loading_guard = apply_stage_states(
        &document,
        &mut loading,
        &[
            selection(&document, "Screen", "Loading"),
            selection(&document, "Details", "Expanded"),
        ],
    )
    .unwrap();
    let content_guard = apply_stage_states(
        &document,
        &mut content,
        &[
            selection(&document, "Screen", "Content"),
            selection(&document, "Details", "Compact"),
        ],
    )
    .unwrap();

    let loading_a = button(&loading, &document, "A");
    let content_a = button(&content, &document, "A");
    assert!(!loading_a.ptr_eq(&content_a));
    assert!((loading_a.opacity() - 0.2).abs() < 1e-9);
    assert_eq!(content_a.opacity(), 1.0);
    assert_eq!(button(&loading, &document, "B").width(), 160.0);
    assert_eq!(button(&content, &document, "B").width(), 80.0);
    for group in VisualStateGroup::read(&document) {
        let (first, second) = if group.name == "Screen" {
            ("Loading", "Content")
        } else {
            ("Expanded", "Compact")
        };
        assert!(classes(&loading, &document).contains(&group.class_name(first)));
        assert!(classes(&content, &document).contains(&group.class_name(second)));
    }
    assert_eq!(document.source(), before.0);
    assert_eq!(document.revision(), before.1);
    assert!(Arc::ptr_eq(&document.ast_snapshot(), &before.2));
    assert_eq!(document.undo_label(), before.3.as_deref());
    assert_eq!(document.is_dirty(), before.4);

    drop(loading_guard);
    first_window.close();
    drop(loading);
    assert_eq!(content_a.opacity(), 1.0);
    assert_eq!(button(&content, &document, "B").width(), 80.0);
    drop(content_guard);
    second_window.close();
}

#[test]
fn native_resources_and_bindings_reach_their_endpoints_on_each_stages_clock() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = fixture();
    let mut resource = preview(&document);
    let mut binding = preview(&document);
    let resource_window = attach(&mut resource, &document);
    let binding_window = attach(&mut binding, &document);
    // A binding resolves the live object in its own rooted namescope. Sharing the
    // same source URI must not let a sibling stage supply this target or its value.
    button(&binding, &document, "Reference").set_opacity(0.72);
    button(&binding, &document, "Reference").set_width(234.0);
    let resource_guard = apply_stage_states(
        &document,
        &mut resource,
        &[
            selection(&document, "Screen", "Resource"),
            selection(&document, "Details", "Bound"),
        ],
    )
    .unwrap();
    let binding_guard = apply_stage_states(
        &document,
        &mut binding,
        &[
            selection(&document, "Screen", "Bound"),
            selection(&document, "Details", "Bound"),
        ],
    )
    .unwrap();
    assert!((button(&resource, &document, "A").opacity() - 0.35).abs() < 1e-6);
    assert_eq!(button(&resource, &document, "B").width(), 210.0);
    assert!((button(&binding, &document, "A").opacity() - 0.72).abs() < 1e-6);
    assert_eq!(button(&binding, &document, "B").width(), 234.0);
    assert!(document
        .source()
        .contains("{StaticResource ScenarioOpacity}"));
    assert!(document
        .source()
        .contains("{Binding Opacity, ElementName=Reference}"));
    assert_eq!(
        document
            .ast()
            .node(document.ast().find_name("A").unwrap())
            .unwrap()
            .attribute("Opacity"),
        Some("1")
    );
    drop(resource_guard);
    drop(binding_guard);
    resource_window.close();
    binding_window.close();
}

#[test]
fn every_scenario_is_validated_before_native_classes_values_or_clock_change() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = fixture();
    let mut preview = preview(&document);
    let window = attach(&mut preview, &document);
    let previous_clock = preview.host.clock().unwrap();
    let previous_classes = classes(&preview, &document);
    for invalid in [
        vec![
            selection(&document, "Details", "Expanded"),
            selection(&document, "Screen", "Unknown"),
        ],
        vec![
            selection(&document, "Screen", "Loading"),
            selection(&document, "Screen", "Content"),
        ],
        vec![
            selection(&document, "Details", "Expanded"),
            ResolvedStageState {
                owner: document.ast().find_name("A").unwrap(),
                group: "Screen",
                state: "Loading",
            },
        ],
    ] {
        assert!(!error(apply_stage_states(&document, &mut preview, &invalid)).is_empty());
        assert_eq!(classes(&preview, &document), previous_classes);
        assert_eq!(button(&preview, &document, "A").opacity(), 1.0);
        assert_eq!(button(&preview, &document, "B").width(), 80.0);
        assert!(Rc::ptr_eq(&preview.host.clock().unwrap(), &previous_clock));
    }
    window.close();
}

#[test]
fn expression_scenarios_require_rooting_and_can_retry_after_attachment() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = fixture();
    let mut preview = preview(&document);
    let previous_clock = preview.host.clock().unwrap();
    let previous_classes = classes(&preview, &document);
    let scenario = [
        selection(&document, "Details", "Expanded"),
        selection(&document, "Screen", "Resource"),
    ];
    let message = error(apply_stage_states(&document, &mut preview, &scenario));
    assert!(message.contains("rooted logical tree"), "{message}");
    assert_eq!(classes(&preview, &document), previous_classes);
    assert_eq!(button(&preview, &document, "B").width(), 80.0);
    assert!(Rc::ptr_eq(&preview.host.clock().unwrap(), &previous_clock));
    let window = attach(&mut preview, &document);
    let guard = apply_stage_states(&document, &mut preview, &scenario).unwrap();
    assert!((button(&preview, &document, "A").opacity() - 0.35).abs() < 1e-6);
    assert_eq!(button(&preview, &document, "B").width(), 160.0);
    drop(guard);
    window.close();
}

#[test]
fn disconnected_literal_endpoints_dispose_only_the_clock_they_installed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let document = fixture();
    let mut preview = preview(&document);
    let previous_clock = preview.host.clock().unwrap();
    let guard = apply_stage_states(
        &document,
        &mut preview,
        &[selection(&document, "Screen", "Loading")],
    )
    .unwrap();
    assert!((button(&preview, &document, "A").opacity() - 0.2).abs() < 1e-9);
    let installed = preview.host.clock().unwrap();
    assert!(!Rc::ptr_eq(&installed, &previous_clock));
    drop(guard);
    assert_eq!(installed.play_state(), PlayState::Stop);
    assert!(Rc::ptr_eq(&preview.host.clock().unwrap(), &previous_clock));

    // A new host owner may replace the clock before the retired graph is disposed.
    // Releasing its old scenario must not undo that owner's explicit clock choice.
    let guard = apply_stage_states(
        &document,
        &mut preview,
        &[selection(&document, "Screen", "Content")],
    )
    .unwrap();
    let installed = preview.host.clock().unwrap();
    preview.host.set_clock(None);
    drop(guard);
    assert_eq!(installed.play_state(), PlayState::Stop);
    assert!(preview.host.clock().is_none());
}

#[test]
fn empty_scenario_preserves_mode_and_stale_runtime_rejects_nonempty_scenario() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut document = fixture();
    let mut preview = preview(&document);
    preview.set_interactive(true);
    let before = classes(&preview, &document);
    let empty = apply_stage_states(&document, &mut preview, &[]).unwrap();
    assert!(preview.host.clock().is_none());
    drop(empty);
    assert!(preview.host.clock().is_none());
    assert_eq!(classes(&preview, &document), before);
    let owner = document.ast().root();
    document
        .apply(
            "Source changed after loading",
            vec![Edit::SetAttribute {
                node: owner,
                name: "Tag".into(),
                value: "new revision".into(),
            }],
        )
        .unwrap();
    let message = error(apply_stage_states(
        &document,
        &mut preview,
        &[selection(&document, "Screen", "Loading")],
    ));
    assert!(message.contains("current XAML"), "{message}");
    assert!(preview.host.clock().is_none());
    assert_eq!(classes(&preview, &document), before);
    assert_eq!(button(&preview, &document, "A").opacity(), 1.0);
}

#[test]
fn documented_sample_loads_real_controls_and_each_screen_endpoint() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = include_str!("../../docs/designer/samples/PreviewStages.xaml");
    let document = Document::new(source).unwrap();
    let states = ["Loading", "Empty", "Content", "Error"];
    let groups = VisualStateGroup::read(&document);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "Screen");
    assert_eq!(groups[0].states.len(), 4);
    for state in states {
        let mut preview = preview(&document);
        let window = attach(&mut preview, &document);
        let guard = apply_stage_states(
            &document,
            &mut preview,
            &[selection(&document, "Screen", state)],
        )
        .unwrap();
        for target in states {
            let panel = document.ast().find_name(&format!("{target}Panel")).unwrap();
            assert_eq!(
                preview.visual(panel).unwrap().is_visible(),
                target == state,
                "{target} panel in the {state} scenario"
            );
        }
        assert_eq!(document.source(), source);
        assert_eq!(document.revision(), 0);
        assert!(!document.can_undo());
        drop(guard);
        window.close();
    }
}
