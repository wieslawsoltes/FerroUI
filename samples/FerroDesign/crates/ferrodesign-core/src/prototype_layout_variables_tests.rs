use super::*;

struct Fixture {
    document: Document,
    root: NodeId,
    inner: NodeId,
    first: NodeId,
    second: NodeId,
    sibling: NodeId,
    spacing: VariableId,
    alias: VariableId,
}

fn variable(document: &mut Document, value: VariableValue) -> VariableId {
    let id = document.allocate_id();
    document.variables.insert(id, Variable {
        id, name: format!("Variable {id}"), collection: "Spacing".into(),
        values: BTreeMap::from([("Default".into(), value)]),
    });
    id
}

fn fixture() -> Fixture {
    let mut document = Document::new("Prototype spacing");
    let mut outer = Node::new(NodeKind::Frame, "Outer Hug", 10.0, 20.0, 1.0, 1.0);
    outer.layout = Some(AutoLayout {
        padding: Padding::all(5.0), gap: 7.0,
        width: SizingMode::Hug, height: SizingMode::Hug, ..AutoLayout::default()
    });
    let root = document.insert_node(document.pages[0].id, outer).unwrap();
    let mut inner = Node::new(NodeKind::Frame, "Inner Hug", 0.0, 0.0, 1.0, 1.0);
    inner.layout = Some(AutoLayout {
        padding: Padding::all(10.0), gap: 10.0,
        width: SizingMode::Hug, height: SizingMode::Hug, ..AutoLayout::default()
    });
    let inner = document.insert_node(root, inner).unwrap();
    let first = document.insert_node(inner, Node::new(NodeKind::Rectangle, "First", 0.0, 0.0, 40.0, 20.0)).unwrap();
    let second = document.insert_node(inner, Node::new(NodeKind::Rectangle, "Second", 0.0, 0.0, 30.0, 30.0)).unwrap();
    let sibling = document.insert_node(root, Node::new(NodeKind::Rectangle, "Outer sibling", 0.0, 0.0, 20.0, 10.0)).unwrap();
    let spacing = variable(&mut document, VariableValue::Number(10.0));
    let alias = variable(&mut document, VariableValue::Alias(spacing));
    for field in LayoutVariableField::ALL {
        document.nodes.get_mut(&inner).unwrap().variable_bindings.insert(field.key().into(), alias);
    }
    // Preview overrides must also restore the absence of an explicit mode key.
    document.variable_mode = "Unstored preview mode".into();
    document.apply_variables();
    assert!(document.layout_all().errors.is_empty());
    document.validate_variables().unwrap();
    document.validate().unwrap();
    Fixture { document, root, inner, first, second, sibling, spacing, alias }
}

fn action(document: &mut Document, root: NodeId, trigger: Trigger, variable: VariableId, value: VariableValue) {
    document.nodes.get_mut(&root).unwrap().interactions = vec![Interaction {
        trigger, action: PrototypeAction::SetVariable { variable, value },
        transition: Transition::default(), delay_ms: 0, key: None,
    }];
}

#[test]
fn prototype_spacing_action_reflows_nested_hug_geometry_without_changing_authored_modes() {
    let mut fixture = fixture();
    let authored = fixture.document.variables.clone();
    assert_eq!((fixture.document.nodes[&fixture.inner].width, fixture.document.nodes[&fixture.root].width), (100.0, 137.0));
    action(&mut fixture.document, fixture.root, Trigger::Click, fixture.spacing, VariableValue::Number(20.125));
    let mut player = PrototypePlayer::default();
    assert!(player.start_at(&fixture.document, fixture.root));
    assert!(player.trigger(&mut fixture.document, fixture.root, Trigger::Click));
    let document = &fixture.document;
    assert_eq!(document.nodes[&fixture.inner].layout.as_ref().unwrap().padding, Padding::all(20.125));
    assert_eq!((document.nodes[&fixture.first].x, document.nodes[&fixture.first].y), (20.125, 20.125));
    assert_eq!(document.nodes[&fixture.second].x, 80.25);
    assert_eq!((document.nodes[&fixture.inner].width, document.nodes[&fixture.inner].height), (130.375, 70.25));
    assert_eq!((document.nodes[&fixture.root].width, document.nodes[&fixture.root].height), (167.375, 80.25));
    assert_eq!(document.nodes[&fixture.sibling].x, 142.375);
    assert_eq!(document.variables, authored);
    assert_eq!(player.variable_overrides().get(&fixture.spacing), Some(&VariableValue::Number(20.125)));
    for field in LayoutVariableField::ALL {
        assert_eq!(document.nodes[&fixture.inner].variable_bindings[field.key()], fixture.alias);
    }
}

#[test]
fn prototype_spacing_alias_detection_includes_prior_session_overrides() {
    let mut fixture = fixture();
    let alternative = variable(&mut fixture.document, VariableValue::Number(30.0));
    let authored = fixture.document.variables.clone();
    action(&mut fixture.document, fixture.root, Trigger::Click, fixture.alias, VariableValue::Alias(alternative));
    let mut player = PrototypePlayer::default();
    player.start_at(&fixture.document, fixture.root);
    assert!(player.trigger(&mut fixture.document, fixture.root, Trigger::Click));
    assert_eq!(fixture.document.nodes[&fixture.first].x, 30.0);
    action(&mut fixture.document, fixture.root, Trigger::Click, alternative, VariableValue::Number(35.5));
    assert!(player.trigger(&mut fixture.document, fixture.root, Trigger::Click));
    assert_eq!(fixture.document.nodes[&fixture.first].x, 35.5);
    assert_eq!(fixture.document.nodes[&fixture.second].x, 111.0);
    assert_eq!(fixture.document.nodes[&fixture.root].width, 213.5);
    assert_eq!(fixture.document.variables, authored);
    assert!(!variable_affects_layout(&fixture.document, fixture.spacing, player.variable_overrides()));
}

#[test]
fn invalid_prototype_spacing_restores_document_and_existing_or_absent_session_override() {
    let mut fixture = fixture();
    action(&mut fixture.document, fixture.root, Trigger::Click, fixture.spacing, VariableValue::Number(20.125));
    let mut player = PrototypePlayer::default();
    player.start_at(&fixture.document, fixture.root);
    assert!(player.trigger(&mut fixture.document, fixture.root, Trigger::Click));
    for (id, value) in [
        (fixture.spacing, VariableValue::Number(-1.0)),
        (fixture.spacing, VariableValue::Number(f64::MAX)),
        (fixture.spacing, VariableValue::String("Wrong type".into())),
        (fixture.spacing, VariableValue::Alias(fixture.alias)),
        (fixture.alias, VariableValue::Number(-1.0)),
    ] {
        action(&mut fixture.document, fixture.root, Trigger::Click, id, value);
        let before_document = fixture.document.clone();
        let before_player = player.clone();
        assert!(!player.trigger(&mut fixture.document, fixture.root, Trigger::Click));
        assert_eq!(fixture.document, before_document);
        assert_eq!(player, before_player);
    }
}

#[test]
fn transient_spacing_action_restores_every_reflowed_node_and_authored_variable() {
    let mut fixture = fixture();
    action(&mut fixture.document, fixture.root, Trigger::WhileHover, fixture.spacing, VariableValue::Number(24.25));
    let before = fixture.document.clone();
    let mut player = PrototypePlayer::default();
    player.start_at(&fixture.document, fixture.root);
    assert!(player.trigger(&mut fixture.document, fixture.root, Trigger::Hover));
    assert_eq!(fixture.document.nodes[&fixture.first].x, 24.25);
    assert_ne!(fixture.document.nodes[&fixture.sibling].x, before.nodes[&fixture.sibling].x);
    assert!(player.has_transient(Trigger::WhileHover));
    assert!(player.end_transient(&mut fixture.document, Trigger::WhileHover));
    assert_eq!(fixture.document, before);
    assert!(player.variable_overrides().is_empty());
    assert!(!player.has_transients());
}

#[test]
fn unrelated_prototype_color_action_keeps_the_existing_path_without_layout_reflow() {
    let mut fixture = fixture();
    let color = variable(&mut fixture.document, VariableValue::Color(Color::BLACK));
    fixture.document.nodes.get_mut(&fixture.first).unwrap().variable_bindings.insert("fill".into(), color);
    fixture.document.nodes.get_mut(&fixture.first).unwrap().x = 123.75;
    let before_layout = fixture.document.nodes[&fixture.inner].clone();
    let authored = fixture.document.variables.clone();
    action(&mut fixture.document, fixture.root, Trigger::Click, color, VariableValue::Color(Color::WHITE));
    let mut player = PrototypePlayer::default();
    player.start_at(&fixture.document, fixture.root);
    assert!(!variable_affects_layout(&fixture.document, color, player.variable_overrides()));
    assert!(player.trigger(&mut fixture.document, fixture.root, Trigger::Click));
    assert_eq!(fixture.document.nodes[&fixture.first].fill_color(), Color::WHITE);
    assert_eq!(fixture.document.nodes[&fixture.first].x, 123.75);
    assert_eq!(fixture.document.nodes[&fixture.inner], before_layout);
    assert_eq!(fixture.document.variables, authored);
}
