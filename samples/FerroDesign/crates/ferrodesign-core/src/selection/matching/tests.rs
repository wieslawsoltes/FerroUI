use super::*;
use crate::*;

fn insert(document: &mut Document, parent: NodeId, kind: NodeKind, name: &str) -> NodeId {
    document.insert_node(parent, Node::new(kind, name, 0.0, 0.0, 100.0, 80.0)).unwrap()
}

#[test]
fn matching_walks_visible_unlocked_page_once_in_layers_order_and_unions_seeds() {
    let mut document = Document::new("Matching");
    let page = document.pages[0].id;
    let a = insert(&mut document, page, NodeKind::Rectangle, "Source A");
    let frame = insert(&mut document, page, NodeKind::Frame, "Nested");
    document.nodes.get_mut(&frame).unwrap().fills.clear();
    let child = insert(&mut document, frame, NodeKind::Ellipse, "Same fill");
    let b = insert(&mut document, page, NodeKind::Rectangle, "Source B");
    document.nodes.get_mut(&b).unwrap().fills = vec![Fill::solid(Color::BLACK)];
    for (hidden, locked) in [(true, false), (false, true)] {
        let parent = insert(&mut document, page, NodeKind::Frame, "Excluded");
        let node = document.nodes.get_mut(&parent).unwrap();
        node.visible = !hidden;
        node.locked = locked;
        insert(&mut document, parent, NodeKind::Rectangle, "Excluded descendant");
    }
    let other_page = document.add_page("Elsewhere");
    let other = insert(&mut document, other_page, NodeKind::Rectangle, "Other page");
    let mut editor = Editor::new(document);
    editor.selection = vec![a, b, a, other, u64::MAX];
    assert_eq!(editor.matching_selection(SelectionMatch::Fill), [b, child, a]);
    assert!(editor.select_matching(SelectionMatch::Fill));
    assert_eq!(editor.selection, [b, child, a]);
    assert!(!editor.select_matching(SelectionMatch::Fill));
}

#[test]
fn matching_preserves_document_redo_scene_and_active_transaction_ownership() {
    let mut document = Document::new("Selection only");
    let page = document.pages[0].id;
    let a = insert(&mut document, page, NodeKind::Rectangle, "A");
    let b = insert(&mut document, page, NodeKind::Ellipse, "B");
    let mut editor = Editor::new(document);
    editor.update_node(a, "Move", |node| node.x = 42.0);
    assert!(editor.undo());
    editor.selection = vec![a];
    let document = editor.document.clone();
    let token = editor.render_change_token();
    let revision = editor.revision;
    let history = editor.history_len();
    assert!(editor.select_matching(SelectionMatch::Fill));
    assert_eq!(editor.selection, [b, a]);
    assert_eq!(editor.document, document);
    assert_eq!(editor.render_change_token(), token);
    assert_eq!(editor.revision, revision);
    assert_eq!(editor.history_len(), history);
    assert!(editor.can_redo());
    editor.begin_node_transaction(&[a], "Existing gesture");
    let before = editor.selection.clone();
    assert!(!editor.select_matching(SelectionMatch::Fill));
    assert!(!editor.invert_selection());
    assert_eq!(editor.selection, before);
    assert!(editor.has_active_transaction());
    editor.cancel_transaction();
}

#[test]
fn exact_paint_stacks_respect_order_opacity_visibility_gradients_and_signed_zero() {
    let mut document = Document::new("Paint signatures");
    let page = document.pages[0].id;
    let a = insert(&mut document, page, NodeKind::Rectangle, "A");
    let gradient = Fill { paint: Paint::LinearGradient {
        start: Point::new(-0.0, 0.0), end: Point::new(1.0, 1.0),
        stops: vec![GradientStop { offset: 0.0, color: Color::BLACK },
            GradientStop { offset: 1.0, color: Color::WHITE }],
    }, opacity: 0.75, visible: true };
    document.nodes.get_mut(&a).unwrap().fills = vec![gradient, Fill::solid(Color::WHITE)];
    let mut matching = Vec::new();
    for index in 0..6 {
        let mut node = document.nodes[&a].clone();
        match index {
            0 => { if let Paint::LinearGradient { start, .. } = &mut node.fills[0].paint { start.x = 0.0; } }
            1 => node.fills.reverse(),
            2 => node.fills[0].visible = false,
            3 => node.fills[0].opacity = 0.5,
            4 => { if let Paint::LinearGradient { end, .. } = &mut node.fills[0].paint { end.x = 0.5; } }
            _ => node.fill_style = Some(900), // Binding identity is not paint appearance.
        }
        node.id = 0;
        let id = document.insert_node(page, node).unwrap();
        if index == 0 || index == 5 { matching.push(id); }
    }
    let mut editor = Editor::new(document);
    editor.selection = vec![a];
    assert_eq!(editor.matching_selection(SelectionMatch::Fill), [matching[1], matching[0], a]);
}

#[test]
fn property_matching_includes_complete_strokes_effect_order_and_ignores_geometry() {
    let mut document = Document::new("Appearance");
    let page = document.pages[0].id;
    let a = insert(&mut document, page, NodeKind::Rectangle, "A");
    {
        let node = document.nodes.get_mut(&a).unwrap();
        node.strokes = vec![Stroke::new(Color::BLACK, 2.0)];
        node.effects = vec![Effect::shadow(Color::BLACK, Point::new(1.0, 2.0), 4.0),
            Effect::shadow(Color::WHITE, Point::new(3.0, 4.0), 8.0)];
        node.corner_radius = 8.0;
    }
    let mut clone = document.nodes[&a].clone();
    clone.id = 0; clone.x = 999.0; clone.width = 42.0; clone.name = "Independent".into();
    clone.corner_radii = Some([8.0; 4]);
    let same = document.insert_node(page, clone).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![a];
    assert_eq!(editor.matching_selection(SelectionMatch::Appearance), [same, a]);
    editor.document.nodes.get_mut(&same).unwrap().strokes[0].dash = vec![2.0, 3.0];
    assert_eq!(editor.matching_selection(SelectionMatch::Stroke), [a]);
    assert_eq!(editor.matching_selection(SelectionMatch::Effect), [same, a]);
    editor.document.nodes.get_mut(&same).unwrap().effects.reverse();
    assert_eq!(editor.matching_selection(SelectionMatch::Effect), [a]);
}

fn run(start: usize, end: usize, weight: u16) -> TextRun {
    TextRun { start, end, font_family: None, font_size: None, font_weight: Some(weight),
        color: None, italic: None, decoration: None }
}

#[test]
fn text_matching_resolves_unicode_ranges_defaults_and_opentype_order_without_content() {
    let mut document = Document::new("Typography");
    let page = document.pages[0].id;
    let a = insert(&mut document, page, NodeKind::Text, "A");
    let b = insert(&mut document, page, NodeKind::Text, "B");
    {
        let a = document.nodes.get_mut(&a).unwrap();
        a.text = "A🙂B".into();
        a.text_runs = vec![run(1, 5, 700)];
        a.font_features = vec![FontFeatureSetting { tag: "liga".into(), value: 0 },
            FontFeatureSetting { tag: "kern".into(), value: 1 }];
    }
    {
        let b = document.nodes.get_mut(&b).unwrap();
        b.text = "prefixBoldEnds".into();
        b.text_runs = vec![run(0, 6, 400), run(6, 8, 700), run(8, 10, 700), run(10, 14, 400)];
        b.text_direction = Some(TextDirection::LeftToRight);
        b.font_features = vec![FontFeatureSetting { tag: "kern".into(), value: 1 },
            FontFeatureSetting { tag: "liga".into(), value: 0 }];
    }
    let mut editor = Editor::new(document);
    editor.selection = vec![a];
    assert_eq!(editor.matching_selection(SelectionMatch::TextProperties), [b, a]);
    editor.document.nodes.get_mut(&b).unwrap().letter_spacing = 2.0;
    assert_eq!(editor.matching_selection(SelectionMatch::TextProperties), [a]);
    assert_eq!(editor.matching_selection(SelectionMatch::Font), [b, a]);
    editor.document.nodes.get_mut(&b).unwrap().text_runs[1].italic = Some(true);
    assert_eq!(editor.matching_selection(SelectionMatch::Font), [a]);
}

#[test]
fn paragraph_matching_uses_effective_styles_without_text_or_byte_offset_identity() {
    let mut document = Document::new("Paragraph matching");
    let page = document.pages[0].id;
    let a = insert(&mut document, page, NodeKind::Text, "A");
    let b = insert(&mut document, page, NodeKind::Text, "B");
    for (id, text) in [(a, "A🙂\nRTL"), (b, "Different first paragraph\nAnother\nLast")] {
        let node = document.nodes.get_mut(&id).unwrap();
        node.text = text.into();
        let ranges = text_paragraph_ranges(&node.text);
        node.text_paragraphs = ranges.iter().skip(1).map(|range| TextParagraph {
            start: range.start, end: range.end, direction: Some(TextDirection::RightToLeft),
            space_before: Some(8.0), indent: Some(12.0), ..Default::default()
        }).collect();
    }
    let mut editor = Editor::new(document);
    editor.selection = vec![a];
    assert_eq!(editor.matching_selection(SelectionMatch::TextProperties), [b, a]);
    editor.document.nodes.get_mut(&b).unwrap().text_paragraphs[1].indent = Some(13.0);
    assert_eq!(editor.matching_selection(SelectionMatch::TextProperties), [a]);
    assert_eq!(editor.matching_selection(SelectionMatch::Font), [b, a]);
}

#[test]
fn font_matching_uses_only_rendered_faces_and_first_starting_overlap_precedence() {
    let mut document = Document::new("Rich fonts");
    let page = document.pages[0].id;
    let a = insert(&mut document, page, NodeKind::Text, "A");
    let b = insert(&mut document, page, NodeKind::Text, "B");
    for id in [a, b] {
        let node = document.nodes.get_mut(&id).unwrap();
        node.text = "text".into();
        node.text_runs = vec![TextRun { font_family: Some("Actual face".into()), ..run(0, 4, 500) }];
    }
    let bnode = document.nodes.get_mut(&b).unwrap();
    bnode.font_family = "Unused default".into();
    bnode.text_runs.push(TextRun { font_family: Some("Hidden overlap".into()), ..run(1, 3, 900) });
    let mut editor = Editor::new(document);
    editor.selection = vec![a];
    assert_eq!(editor.matching_selection(SelectionMatch::Font), [b, a]);
    assert_eq!(editor.matching_selection(SelectionMatch::TextProperties), [b, a]);
}

#[test]
fn instances_match_exact_component_and_unsupported_sources_preserve_selection() {
    let mut document = Document::new("Instances");
    let page = document.pages[0].id;
    let master = insert(&mut document, page, NodeKind::Component, "Master");
    let a = insert(&mut document, page, NodeKind::Instance, "A");
    let b = insert(&mut document, page, NodeKind::Instance, "B");
    let different = insert(&mut document, page, NodeKind::Instance, "Variant");
    for id in [a, b] { document.nodes.get_mut(&id).unwrap().component_id = Some(master); }
    document.nodes.get_mut(&different).unwrap().component_id = Some(u64::MAX);
    let mut editor = Editor::new(document);
    editor.selection = vec![a];
    assert_eq!(editor.matching_selection(SelectionMatch::Instance), [b, a]);
    assert!(!editor.select_matching(SelectionMatch::Font));
    assert_eq!(editor.selection, [a]);
    editor.selection = vec![master];
    assert!(!editor.select_matching(SelectionMatch::Instance));
    assert_eq!(editor.selection, [master]);
}

#[test]
fn inversion_excludes_selected_descendants_and_skips_hidden_locked_roots() {
    let mut document = Document::new("Inverse");
    let page = document.pages[0].id;
    let a = insert(&mut document, page, NodeKind::Frame, "Parent");
    let child = insert(&mut document, a, NodeKind::Rectangle, "Child");
    let b = insert(&mut document, page, NodeKind::Rectangle, "Other");
    let hidden = insert(&mut document, page, NodeKind::Rectangle, "Hidden");
    let locked = insert(&mut document, page, NodeKind::Rectangle, "Locked");
    document.nodes.get_mut(&hidden).unwrap().visible = false;
    document.nodes.get_mut(&locked).unwrap().locked = true;
    let mut editor = Editor::new(document);
    editor.selection = vec![child];
    assert!(editor.invert_selection());
    assert_eq!(editor.selection, [b]);
    assert!(editor.invert_selection());
    assert_eq!(editor.selection, [a]);
    editor.selection.clear();
    assert!(editor.invert_selection());
    assert_eq!(editor.selection, [b, a]);
    assert!(editor.invert_selection());
    assert!(editor.selection.is_empty());
    assert_eq!(editor.history_len(), 0);
}

#[test]
fn large_mixed_selection_deduplicates_borrowed_signatures_and_is_deterministic() {
    let mut document = Document::new("Ten thousand layers");
    let page = document.pages[0].id;
    let ids: Vec<_> = (0..10_000).map(|index| {
        let id = insert(&mut document, page, NodeKind::Rectangle, "Layer");
        document.nodes.get_mut(&id).unwrap().fills = vec![Fill::solid(Color::rgb((index % 64) as u8, 0, 0))];
        id
    }).collect();
    let mut editor = Editor::new(document);
    editor.selection = ids.clone();
    assert_eq!(editor.matching_selection(SelectionMatch::Fill), ids.into_iter().rev().collect::<Vec<_>>());
    assert_eq!(editor.history_len(), 0);
}
