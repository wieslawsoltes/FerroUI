use super::*;
use crate::SvgSemanticValue;
use ferroui_base::media::MatrixTransform;
use ferroui_base::{Point, RelativePoint, StyledElement, Thickness};
use ferroui_controls::automation::peers::AutomationControlType;
use ferroui_controls::documents::{InlineCollection, Run};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Button, TextBox};
use std::cell::Cell;

fn application() -> UnitTestApplicationScope {
    UnitTestApplication::start(TestServices::mock_platform_render_interface())
}

fn visual(bounds: Rect) -> Ref<Visual> {
    let visual = Visual::new();
    visual.set_bounds(bounds);
    visual
}

#[test]
fn snapshot_resolves_all_relations_after_application_ids_are_assigned() {
    let _application = application();
    let root = visual(Rect::new(20.0, 10.0, 100.0, 80.0));
    root.set_name(Some("root".into()));
    let first = visual(Rect::new(4.0, 6.0, 30.0, 20.0));
    first.set_name(Some("first".into()));
    let second = visual(Rect::new(40.0, 6.0, 30.0, 20.0));
    second.set_name(Some("second".into()));
    root.visual_children().add(first.clone());
    root.visual_children().add(second.clone());
    first.set_parent(root.clone().upcast::<StyledElement>());
    second.set_parent(first.clone().upcast::<StyledElement>());
    second.set_templated_parent(root.clone().upcast::<FerroObject>());

    let callback_count = Cell::new(0);
    let mut customize = |source: &Visual, node: &mut SvgSemanticNode| {
        callback_count.set(callback_count.get() + 1);
        node.id = format!("stable-{}", source.name().unwrap());
        node.properties.insert(
            "component".into(),
            SvgSemanticValue::String("fixture".into()),
        );
    };
    let tree = capture_visual_semantics(
        &root,
        &SvgSemanticCaptureOptions::default(),
        Some(&mut customize),
    )
    .unwrap();
    assert_eq!(callback_count.get(), 3);
    let nodes = tree.nodes();
    assert_eq!(nodes[0].id, "stable-root");
    assert_eq!(nodes[1].parent.as_deref(), Some("stable-root"));
    assert_eq!(nodes[1].logical_parent.as_deref(), Some("stable-root"));
    assert_eq!(nodes[2].parent.as_deref(), Some("stable-root"));
    assert_eq!(nodes[2].logical_parent.as_deref(), Some("stable-first"));
    assert_eq!(nodes[2].templated_parent.as_deref(), Some("stable-root"));
    assert!(nodes[0].parent.is_none());
    assert!(nodes[0].type_name.ends_with("Visual"));
    assert_eq!(nodes[0].layout_bounds, root.bounds());
    assert_eq!(nodes[0].local_to_layer, Matrix::IDENTITY);
    assert_eq!(
        nodes[1].local_to_layer,
        Matrix::create_translation(4.0, 6.0)
    );

    // The result owns data, not the original control graph.
    let weak = second.downgrade();
    root.visual_children().clear();
    drop(second);
    assert!(weak.upgrade().is_none());
    assert_eq!(tree.nodes()[2].id, "stable-second");
}

#[test]
fn root_mirror_render_origin_and_manipulation_match_immediate_layer_coordinates() {
    let _application = application();
    let root = visual(Rect::new(100.0, 50.0, 80.0, 40.0));
    root.set_render_transform_origin(RelativePoint::TOP_LEFT);
    root.set_render_transform(Some(
        MatrixTransform::with_matrix(Matrix::create_scale(2.0, 3.0)).into(),
    ));
    root.set_has_mirror_transform(true);
    root.set_manipulation_transform(Some(Matrix::create_translation(5.0, 7.0)));
    root.set_clip_to_bounds(true);
    let child = visual(Rect::new(3.0, 4.0, 10.0, 12.0));
    child.set_render_transform_origin(RelativePoint::TOP_LEFT);
    child.set_render_transform(Some(
        MatrixTransform::with_matrix(Matrix::create_scale(0.5, 0.25)).into(),
    ));
    child.set_has_mirror_transform(true);
    child.set_manipulation_transform(Some(Matrix::create_translation(1.0, 2.0)));
    root.visual_children().add(child.clone());
    child.set_has_mirror_transform(true);

    let tree = capture_visual_semantics(&root, &Default::default(), None).unwrap();
    assert_eq!(
        tree.nodes()[0].local_to_layer,
        Matrix::new(-2.0, 0.0, 0.0, 3.0, 165.0, 7.0)
    );
    assert_eq!(
        tree.nodes()[1].local_to_layer,
        Matrix::new(1.0, 0.0, 0.0, 0.75, 147.0, 25.0)
    );
    assert_eq!(
        Point::new(2.0, 8.0).transform(tree.nodes()[1].local_to_layer),
        Point::new(149.0, 31.0)
    );

    root.set_has_mirror_transform(false);
    root.set_manipulation_transform(None);
    root.set_render_transform_origin(RelativePoint::CENTER);
    let tree = capture_visual_semantics(&root, &Default::default(), None).unwrap();
    // The root's actual parent-space position stays out of its recording, but scaling
    // around its center contributes this translation.
    assert_eq!(
        tree.nodes()[0].local_to_layer,
        Matrix::new(2.0, 0.0, 0.0, 3.0, -40.0, -40.0)
    );
}

#[test]
fn structural_snapshot_includes_layout_input_classes_and_grid_without_content() {
    let _application = application();
    let control = Border::new();
    control.set_name(Some("content".into()));
    control.classes().add("surface");
    control.set_width(25.0);
    control.set_margin(Thickness::new(1.0, 2.0, 3.0, 4.0));
    control.set_focusable(true);
    control.set_is_enabled(false);
    control.set_is_hit_test_visible(false);
    control.set_tab_index(7);
    Grid::set_row(&control, 2);
    Grid::set_column_span(&control, 3);
    let grid = Grid::new();
    grid.children().add(control.clone().upcast::<Control>());
    let tree =
        capture_visual_semantics(&control.clone().upcast(), &Default::default(), None).unwrap();
    let node = &tree.nodes()[0];
    let layout = node.layout.as_ref().unwrap();
    assert_eq!(node.name.as_deref(), Some("content"));
    assert!(node.classes.iter().any(|value| value == "surface"));
    assert!(node.classes.iter().any(|value| value == ":disabled"));
    assert_eq!(layout.width, Some(25.0));
    assert_eq!(layout.height, None);
    assert_eq!(layout.max_width, None);
    assert_eq!(layout.max_height, None);
    assert_eq!(layout.margin, Thickness::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(node.state.enabled, Some(false));
    assert_eq!(node.state.focusable, Some(true));
    assert_eq!(node.state.hit_test_visible, Some(false));
    assert_eq!(node.state.tab_index, Some(7));
    let placement = layout.grid.as_ref().unwrap();
    assert_eq!(placement.row, 2);
    assert_eq!(placement.column, 0);
    assert_eq!(placement.row_span, 1);
    assert_eq!(placement.column_span, 3);
    assert!(node.properties.is_empty());
    assert!(node.accessibility.is_none());
    assert!(node.text.is_none());
    assert!(node.source.is_none());

    let stripped = capture_visual_semantics(
        &control.upcast(),
        &SvgSemanticCaptureOptions {
            include_names: false,
            include_classes: false,
            include_layout: false,
            include_input_state: false,
            ..Default::default()
        },
        None,
    )
    .unwrap();
    let node = &stripped.nodes()[0];
    assert!(node.name.is_none());
    assert!(node.classes.is_empty());
    assert!(node.layout.is_none());
    assert!(node.state.enabled.is_none());
    assert!(node.properties.is_empty());
}

#[test]
fn text_is_opt_in_preserves_inline_content_and_excludes_text_box_subtrees() {
    let _application = application();
    let root = visual(Rect::new(0.0, 0.0, 100.0, 80.0));
    let text = TextBlock::new();
    text.set_text(Some("Private text"));
    let access = AccessText::new();
    access.set_text(Some("_Open __file"));
    let rich = TextBlock::new();
    rich.set_text(Some("unused fallback"));
    let inlines = InlineCollection::new();
    inlines.add(Run::with_text(Some("Rich ")));
    inlines.add(Run::with_text(Some("text")));
    rich.set_inlines(Some(inlines));
    let password = TextBox::new();
    password.set_password_char('*');
    password.set_reveal_password(true);
    password.set_text(Some("Never export"));
    let template_text = TextBlock::new();
    template_text.set_text(Some("Never export"));
    password
        .visual_children()
        .add(template_text.clone().upcast());
    root.visual_children().add(text.upcast());
    root.visual_children().add(access.upcast());
    root.visual_children().add(rich.upcast());
    root.visual_children().add(password.clone().upcast());

    let private = capture_visual_semantics(&root, &Default::default(), None).unwrap();
    assert!(private.nodes().iter().all(|node| node.text.is_none()));
    let options = SvgSemanticCaptureOptions {
        include_text: true,
        ..Default::default()
    };
    let content = capture_visual_semantics(&root, &options, None).unwrap();
    assert_eq!(content.nodes()[1].text.as_deref(), Some("Private text"));
    assert_eq!(content.nodes()[2].text.as_deref(), Some("Open _file"));
    assert_eq!(content.nodes()[3].text.as_deref(), Some("Rich text"));
    assert!(content.nodes()[4].text.is_none());
    assert!(content.nodes()[5].text.is_none());
    let subtree =
        capture_visual_semantics(&template_text.clone().upcast(), &options, None).unwrap();
    assert!(subtree.nodes()[0].text.is_none());

    // Template ownership also protects a separately hosted template child.
    password.visual_children().clear();
    template_text.set_templated_parent(password.upcast::<FerroObject>());
    let template = capture_visual_semantics(&template_text.upcast(), &options, None).unwrap();
    assert!(template.nodes()[0].text.is_none());
}

#[test]
fn authored_accessibility_does_not_create_peers_and_resolved_peer_mode_is_explicit() {
    let _application = application();
    let control = Button::new();
    AutomationProperties::set_automation_id(&control, Some("save-button"));
    AutomationProperties::set_name(&control, Some("Save changes"));
    AutomationProperties::set_help_text(&control, Some("Write the document"));
    AutomationProperties::set_heading_level(&control, 2);
    AutomationProperties::set_is_required_for_form(&control, true);
    AutomationProperties::set_control_type_override(&control, Some(AutomationControlType::Button));
    let root = control.clone().upcast();
    assert!(ControlAutomationPeer::from_element(&control).is_none());
    let structural = capture_visual_semantics(&root, &Default::default(), None).unwrap();
    assert!(structural.nodes()[0].accessibility.is_none());
    assert!(ControlAutomationPeer::from_element(&control).is_none());
    let attached = capture_visual_semantics(
        &root,
        &SvgSemanticCaptureOptions {
            accessibility: SvgSemanticAccessibilityMode::AttachedProperties,
            ..Default::default()
        },
        None,
    )
    .unwrap();
    let accessibility = attached.nodes()[0].accessibility.as_ref().unwrap();
    assert_eq!(accessibility.automation_id.as_deref(), Some("save-button"));
    assert_eq!(accessibility.name.as_deref(), Some("Save changes"));
    assert_eq!(
        accessibility.help_text.as_deref(),
        Some("Write the document")
    );
    assert_eq!(accessibility.role.as_deref(), Some("Button"));
    assert_eq!(accessibility.heading_level, Some(2));
    assert_eq!(accessibility.required, Some(true));
    assert!(ControlAutomationPeer::from_element(&control).is_none());
    let resolved = capture_visual_semantics(
        &root,
        &SvgSemanticCaptureOptions {
            accessibility: SvgSemanticAccessibilityMode::AutomationPeer,
            ..Default::default()
        },
        None,
    )
    .unwrap();
    assert_eq!(
        resolved.nodes()[0]
            .accessibility
            .as_ref()
            .unwrap()
            .role
            .as_deref(),
        Some("Button")
    );
    assert!(ControlAutomationPeer::from_element(&control).is_some());
}

#[test]
fn visibility_filter_preserves_source_indices_and_outside_relationships_are_omitted() {
    let _application = application();
    let outside = visual(Rect::default());
    let root = visual(Rect::default());
    root.set_parent(outside.clone().upcast::<StyledElement>());
    root.set_templated_parent(outside.upcast::<FerroObject>());
    let hidden = visual(Rect::default());
    hidden.set_is_visible(false);
    hidden.visual_children().add(visual(Rect::default()));
    let visible = visual(Rect::default());
    root.visual_children().add(hidden);
    root.visual_children().add(visible);
    let all = capture_visual_semantics(&root, &Default::default(), None).unwrap();
    assert_eq!(all.nodes().len(), 4);
    assert!(all.nodes()[0].logical_parent.is_none());
    assert!(all.nodes()[0].templated_parent.is_none());
    let options = SvgSemanticCaptureOptions {
        include_hidden: false,
        ..Default::default()
    };
    let filtered = capture_visual_semantics(&root, &options, None).unwrap();
    assert_eq!(filtered.nodes().len(), 2);
    assert_eq!(filtered.nodes()[1].id, "v0-1");
    assert_eq!(filtered.nodes()[1].parent.as_deref(), Some("v0"));
    root.set_is_visible(false);
    assert!(capture_visual_semantics(&root, &options, None)
        .unwrap()
        .nodes()
        .is_empty());
}

#[test]
fn budgets_reject_incomplete_capture_before_application_callbacks_run() {
    let _application = application();
    let root = visual(Rect::default());
    root.visual_children().add(visual(Rect::default()));
    let calls = Cell::new(0);
    let mut callback = |_: &Visual, _: &mut SvgSemanticNode| calls.set(calls.get() + 1);
    for options in [
        SvgSemanticCaptureOptions {
            max_nodes: 0,
            ..Default::default()
        },
        SvgSemanticCaptureOptions {
            max_nodes: SvgSemanticTree::MAX_NODES + 1,
            ..Default::default()
        },
        SvgSemanticCaptureOptions {
            max_nodes: 1,
            ..Default::default()
        },
        SvgSemanticCaptureOptions {
            max_depth: 0,
            ..Default::default()
        },
    ] {
        assert!(capture_visual_semantics(&root, &options, Some(&mut callback)).is_err());
    }
    assert_eq!(calls.get(), 0);
    let exact = SvgSemanticCaptureOptions {
        max_nodes: 2,
        max_depth: 1,
        ..Default::default()
    };
    assert_eq!(
        capture_visual_semantics(&root, &exact, None)
            .unwrap()
            .nodes()
            .len(),
        2
    );
    let mut duplicate_ids = |_: &Visual, node: &mut SvgSemanticNode| node.id = "same".into();
    assert!(capture_visual_semantics(&root, &exact, Some(&mut duplicate_ids)).is_err());
}

#[test]
fn nonfinite_bounds_or_composed_transforms_fail_without_silent_numeric_repair() {
    let _application = application();
    let root = visual(Rect::new(0.0, 0.0, 10.0, 10.0));
    root.set_bounds(Rect::new(f64::MAX, 0.0, f64::MAX, 10.0));
    assert!(capture_visual_semantics(&root, &Default::default(), None).is_err());
    root.set_bounds(Rect::new(0.0, 0.0, 10.0, 10.0));
    root.set_render_transform_origin(RelativePoint::TOP_LEFT);
    root.set_render_transform(Some(
        MatrixTransform::with_matrix(Matrix::create_scale(1e200, 1e200)).into(),
    ));
    let child = visual(Rect::new(0.0, 0.0, 1.0, 1.0));
    child.set_render_transform_origin(RelativePoint::TOP_LEFT);
    child.set_render_transform(Some(
        MatrixTransform::with_matrix(Matrix::create_scale(1e200, 1e200)).into(),
    ));
    root.visual_children().add(child);
    assert!(capture_visual_semantics(&root, &Default::default(), None).is_err());
}
