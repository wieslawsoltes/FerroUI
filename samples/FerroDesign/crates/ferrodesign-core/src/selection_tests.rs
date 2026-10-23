use crate::*;

fn layer(document: &mut Document, parent: NodeId, kind: NodeKind, x: f64, y: f64) -> NodeId {
    document
        .insert_node(
            parent,
            Node::new(kind, "Selection layer", x, y, 100.0, 100.0),
        )
        .unwrap()
}

#[test]
fn selection_candidates_follow_exact_geometry_painter_order_and_visibility() {
    let mut document = Document::new("Ordered selection");
    let page = document.pages[0].id;
    let back = layer(&mut document, page, NodeKind::Rectangle, 0.0, 0.0);
    let front = layer(&mut document, page, NodeKind::Rectangle, 0.0, 0.0);
    let locked = layer(&mut document, page, NodeKind::Rectangle, 0.0, 0.0);
    document.nodes.get_mut(&locked).unwrap().locked = true;
    let hidden = layer(&mut document, page, NodeKind::Rectangle, 0.0, 0.0);
    document.nodes.get_mut(&hidden).unwrap().visible = false;
    let ellipse = layer(&mut document, page, NodeKind::Ellipse, 0.0, 0.0);
    let editor = Editor::new(document);
    let corner = Point::new(1.0, 1.0);
    assert_eq!(editor.hit_test_candidates(corner, 0.0), [front, back]);
    assert_eq!(
        editor
            .scene
            .hit_test_candidates(&editor.document, corner, 0.0, true),
        [locked, front, back]
    );
    assert_eq!(
        editor.hit_test_candidates(Point::new(50.0, 50.0), 0.0),
        [ellipse, front, back]
    );
    assert_eq!(editor.hit_test(corner, 0.0), Some(front));
}

#[test]
fn selection_filtered_pick_only_calls_predicate_for_real_hits_and_stops_when_accepted() {
    let mut document = Document::new("Filtered selection");
    let page = document.pages[0].id;
    let back = layer(&mut document, page, NodeKind::Rectangle, 0.0, 0.0);
    let front = layer(&mut document, page, NodeKind::Rectangle, 0.0, 0.0);
    layer(&mut document, page, NodeKind::Ellipse, 0.0, 0.0);
    let editor = Editor::new(document);
    let mut visited = Vec::new();
    let point = Point::new(1.0, 1.0);
    assert_eq!(
        editor.hit_test_filtered(point, 0.0, |id| {
            visited.push(id);
            true
        }),
        Some(front)
    );
    assert_eq!(visited, [front]);
    visited.clear();
    assert_eq!(
        editor.hit_test_filtered(point, 0.0, |id| {
            visited.push(id);
            id != front
        }),
        Some(back)
    );
    assert_eq!(visited, [front, back]);
    assert_eq!(editor.hit_test_filtered(point, 0.0, |_| false), None);
}

#[test]
fn selection_candidates_respect_affine_rounded_clips_and_boolean_ownership() {
    let mut document = Document::new("Clipped selection");
    let page = document.pages[0].id;
    let back = layer(&mut document, page, NodeKind::Rectangle, -20.0, -20.0);
    let frame = layer(&mut document, page, NodeKind::Frame, 0.0, 0.0);
    {
        let node = document.nodes.get_mut(&frame).unwrap();
        node.clip_content = true;
        node.corner_radius = 30.0;
        node.affine_transform = Some(Transform {
            a: 1.0,
            b: 0.3,
            c: 0.2,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        });
    }
    let child = layer(&mut document, frame, NodeKind::Rectangle, 0.0, 0.0);
    let transform = document.world_transform(frame).unwrap();
    let editor = Editor::new(document);
    assert_eq!(
        editor.hit_test_candidates(transform.apply(Point::new(1.0, 1.0)), 0.0),
        [back]
    );
    assert_eq!(
        editor.hit_test_candidates(transform.apply(Point::new(50.0, 50.0)), 0.0),
        [child, frame, back]
    );

    let mut document = Document::new("Boolean selection");
    let page = document.pages[0].id;
    let boolean = layer(&mut document, page, NodeKind::Boolean, 0.0, 0.0);
    let operand = layer(&mut document, boolean, NodeKind::Rectangle, 0.0, 0.0);
    let editor = Editor::new(document);
    let point = Point::new(20.0, 20.0);
    assert_eq!(editor.hit_test_candidates(point, 0.0), [boolean]);
    assert_eq!(
        editor
            .scene
            .hit_test_candidates_in_subtree(&editor.document, operand, point, 0.0, true),
        [operand]
    );
}

#[test]
fn selection_isolated_prototype_candidates_ignore_external_clips_and_other_pages() {
    let mut document = Document::new("Isolated selection");
    let page = document.pages[0].id;
    let clip = layer(&mut document, page, NodeKind::Frame, 0.0, 0.0);
    document.nodes.get_mut(&clip).unwrap().clip_content = true;
    let root = layer(&mut document, clip, NodeKind::Frame, 200.0, 0.0);
    let child = layer(&mut document, root, NodeKind::Rectangle, 0.0, 0.0);
    document.nodes.get_mut(&child).unwrap().locked = true;
    let overlap = layer(&mut document, page, NodeKind::Rectangle, 200.0, 0.0);
    let other_page = document.add_page("Other page");
    let other_root = layer(&mut document, other_page, NodeKind::Frame, 200.0, 0.0);
    let other_child = layer(&mut document, other_root, NodeKind::Rectangle, 0.0, 0.0);
    let editor = Editor::new(document);
    let point = Point::new(240.0, 40.0);
    assert_eq!(editor.hit_test_candidates(point, 0.0), [overlap]);
    assert_eq!(
        editor
            .scene
            .hit_test_candidates_in_subtree(&editor.document, root, point, 0.0, true),
        [child, root]
    );
    assert_eq!(
        editor.prototype_hit_test_filtered(root, point, 0.0, |id| id != child),
        Some(root)
    );
    assert_eq!(
        editor
            .scene
            .hit_test_candidates_in_subtree(&editor.document, other_root, point, 0.0, true),
        [other_child, other_root]
    );
    assert_eq!(
        editor.prototype_hit_test_filtered(other_root, point, 0.0, |_| true),
        Some(other_child)
    );
    for (root, child) in [(root, child), (other_root, other_child)] {
        let index = IsolatedSelectionIndex::new(&editor.document, root).unwrap();
        assert_eq!(index.root(), root);
        assert_eq!(index.len(), 2);
        assert!(!index.is_empty());
        assert_eq!(
            index.hit_test_candidates(&editor.document, point, 0.0, true),
            [child, root]
        );
        assert_eq!(
            index.hit_test_filtered(&editor.document, point, 0.0, true, |id| id != child),
            Some(root)
        );
    }
    assert!(IsolatedSelectionIndex::new(&editor.document, u64::MAX).is_none());
}

#[test]
fn selection_isolated_index_retains_affine_transforms_and_internal_clips() {
    let mut document = Document::new("Isolated affine selection");
    let page = document.pages[0].id;
    let external = layer(&mut document, page, NodeKind::Frame, 40.0, 30.0);
    {
        let node = document.nodes.get_mut(&external).unwrap();
        node.clip_content = true;
        node.affine_transform = Some(Transform {
            a: 1.0,
            b: 0.2,
            c: 0.3,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        });
    }
    let root = layer(&mut document, external, NodeKind::Frame, 200.0, 0.0);
    document.nodes.get_mut(&root).unwrap().corner_radius = 30.0;
    document.nodes.get_mut(&root).unwrap().clip_content = true;
    let child = layer(&mut document, root, NodeKind::Rectangle, 0.0, 0.0);
    let transform = document.world_transform(root).unwrap();
    let editor = Editor::new(document);
    let index = IsolatedSelectionIndex::new(&editor.document, root).unwrap();
    for point in [
        Point::new(1.0, 1.0),
        Point::new(50.0, 50.0),
        Point::new(150.0, 50.0),
    ] {
        let world = transform.apply(point);
        assert_eq!(
            index.hit_test_candidates(&editor.document, world, 0.0, false),
            editor
                .scene
                .hit_test_candidates_in_subtree(&editor.document, root, world, 0.0, false)
        );
    }
    assert_eq!(
        index.hit_test_candidates(
            &editor.document,
            transform.apply(Point::new(50.0, 50.0)),
            0.0,
            false
        ),
        [child, root]
    );
    assert!(index
        .hit_test_candidates(
            &editor.document,
            transform.apply(Point::new(1.0, 1.0)),
            0.0,
            false
        )
        .is_empty());
}

#[test]
fn selection_invalid_queries_are_empty_and_never_reach_the_visibility_filter() {
    let mut document = Document::new("Invalid selection points");
    let page = document.pages[0].id;
    let root = layer(&mut document, page, NodeKind::Frame, 0.0, 0.0);
    let editor = Editor::new(document);
    for (point, tolerance) in [
        (Point::new(f64::NAN, 0.0), 0.0),
        (Point::new(0.0, f64::INFINITY), 0.0),
        (Point::new(1.0, 1.0), f64::NAN),
        (Point::new(1.0, 1.0), -1.0),
    ] {
        assert!(editor.hit_test_candidates(point, tolerance).is_empty());
        assert_eq!(
            editor.hit_test_filtered(point, tolerance, |_| panic!(
                "invalid point reached renderer"
            )),
            None
        );
        assert_eq!(
            editor.prototype_hit_test_filtered(root, point, tolerance, |_| panic!(
                "invalid isolated point reached renderer"
            )),
            None
        );
    }
}

#[test]
fn selection_sparse_ten_thousand_layer_queries_have_one_exact_candidate() {
    let mut document = Document::new("Sparse indexed selection");
    let page = document.pages[0].id;
    let mut wanted = 0;
    for index in 0..10_000 {
        let id = layer(
            &mut document,
            page,
            NodeKind::Rectangle,
            index as f64 * 400.0,
            0.0,
        );
        if index == 5_000 {
            wanted = id;
        }
    }
    let editor = Editor::new(document);
    let point = Point::new(2_000_050.0, 50.0);
    assert_eq!(editor.hit_test_candidates(point, 0.0), [wanted]);
    let mut calls = 0;
    assert_eq!(
        editor.hit_test_filtered(point, 0.0, |_| {
            calls += 1;
            true
        }),
        Some(wanted)
    );
    assert_eq!(calls, 1);
}

#[test]
fn selection_cycle_wraps_deduplicates_and_preserves_document_history_redo_and_render_token() {
    let mut document = Document::new("Selection is not an edit");
    let page = document.pages[0].id;
    let back = layer(&mut document, page, NodeKind::Rectangle, 0.0, 0.0);
    let front = layer(&mut document, page, NodeKind::Rectangle, 0.0, 0.0);
    let mut editor = Editor::new(document);
    editor.selection = vec![front];
    editor.move_selection(10.0, 0.0);
    assert!(editor.undo());
    let before = editor.document.to_json().unwrap();
    let revision = editor.revision;
    let token = editor.render_change_token();
    let history = editor.history_len();
    let candidates = [front, front, back, back, u64::MAX];
    assert!(editor.cycle_selection(&candidates, SelectionCycle::Behind));
    assert_eq!(editor.selection, [back]);
    assert!(editor.cycle_selection(&candidates, SelectionCycle::Behind));
    assert_eq!(editor.selection, [front]);
    assert!(editor.cycle_selection(&candidates, SelectionCycle::InFront));
    assert_eq!(editor.selection, [back]);
    assert!(!editor.cycle_selection(&[back], SelectionCycle::Behind));
    assert!(!editor.cycle_selection(&[], SelectionCycle::Behind));
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.revision, revision);
    assert_eq!(editor.render_change_token(), token);
    assert_eq!(editor.history_len(), history);
    assert!(editor.can_redo());
    assert!(!editor.has_active_transaction());
    assert!(editor.redo());
    assert_eq!(editor.document.nodes[&front].x, 10.0);
}

#[test]
fn selection_navigation_enters_children_in_layer_order_and_wraps_siblings() {
    let mut document = Document::new("Hierarchy navigation");
    let page = document.pages[0].id;
    let group = layer(&mut document, page, NodeKind::Group, 0.0, 0.0);
    let back = layer(&mut document, group, NodeKind::Rectangle, 0.0, 0.0);
    let locked = layer(&mut document, group, NodeKind::Rectangle, 0.0, 0.0);
    document.nodes.get_mut(&locked).unwrap().locked = true;
    let front = layer(&mut document, group, NodeKind::Rectangle, 0.0, 0.0);
    let hidden = layer(&mut document, group, NodeKind::Rectangle, 0.0, 0.0);
    document.nodes.get_mut(&hidden).unwrap().visible = false;
    let mut editor = Editor::new(document);
    editor.selection = vec![group];
    let before = editor.document.to_json().unwrap();
    assert!(editor.navigate_selection(SelectionNavigation::Children));
    assert_eq!(editor.selection, [front, back]);
    assert!(editor.navigate_selection(SelectionNavigation::NextSibling));
    assert_eq!(editor.selection, [front]);
    assert!(editor.navigate_selection(SelectionNavigation::NextSibling));
    assert_eq!(editor.selection, [back]);
    assert!(editor.navigate_selection(SelectionNavigation::NextSibling));
    assert_eq!(editor.selection, [front]);
    assert!(editor.navigate_selection(SelectionNavigation::PreviousSibling));
    assert_eq!(editor.selection, [back]);
    assert!(editor.navigate_selection(SelectionNavigation::Parent));
    assert_eq!(editor.selection, [group]);
    assert!(!editor.navigate_selection(SelectionNavigation::Parent));
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.history_len(), 0);
}

#[test]
fn selection_navigation_preserves_parallel_scopes_and_filters_stale_ids() {
    let mut document = Document::new("Parallel selection scopes");
    let page = document.pages[0].id;
    let a = layer(&mut document, page, NodeKind::Group, 0.0, 0.0);
    let a1 = layer(&mut document, a, NodeKind::Rectangle, 0.0, 0.0);
    let a2 = layer(&mut document, a, NodeKind::Rectangle, 0.0, 0.0);
    let b = layer(&mut document, page, NodeKind::Group, 200.0, 0.0);
    let b1 = layer(&mut document, b, NodeKind::Rectangle, 0.0, 0.0);
    let b2 = layer(&mut document, b, NodeKind::Rectangle, 0.0, 0.0);
    let other_page = document.add_page("Other page");
    let other = layer(&mut document, other_page, NodeKind::Rectangle, 0.0, 0.0);
    let mut editor = Editor::new(document);
    editor.selection = vec![a2, b2];
    assert!(editor.navigate_selection(SelectionNavigation::NextSibling));
    assert_eq!(editor.selection, [a1, b1]);
    assert!(editor.navigate_selection(SelectionNavigation::Parent));
    assert_eq!(editor.selection, [a, b]);
    assert!(editor.navigate_selection(SelectionNavigation::Children));
    assert_eq!(editor.selection, [a2, a1, b2, b1]);
    assert!(editor.navigate_selection(SelectionNavigation::PreviousSibling));
    assert_eq!(editor.selection, [a1, b1]);
    assert!(!editor.cycle_selection(&[other, u64::MAX], SelectionCycle::Behind));
    editor.selection.clear();
    assert!(editor.navigate_selection(SelectionNavigation::NextSibling));
    assert_eq!(editor.selection, [b]);
    editor.selection.clear();
    assert!(editor.navigate_selection(SelectionNavigation::PreviousSibling));
    assert_eq!(editor.selection, [a]);
}

#[test]
fn selection_navigation_and_cycling_do_not_borrow_an_external_edit_transaction() {
    let mut document = Document::new("Selection transaction boundary");
    let page = document.pages[0].id;
    let a = layer(&mut document, page, NodeKind::Rectangle, 0.0, 0.0);
    let b = layer(&mut document, page, NodeKind::Rectangle, 0.0, 0.0);
    let mut editor = Editor::new(document);
    editor.selection = vec![a];
    editor.begin_transaction("External draft");
    let token = editor.active_transaction_token();
    let before = editor.document.to_json().unwrap();
    assert!(!editor.navigate_selection(SelectionNavigation::NextSibling));
    assert!(!editor.cycle_selection(&[a, b], SelectionCycle::Behind));
    assert_eq!(editor.selection, [a]);
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.active_transaction_token(), token);
    assert_eq!(editor.history_len(), 0);
}

#[test]
fn selection_group_context_descends_one_level_and_keeps_selected_ancestor_scopes_open() {
    let mut document = Document::new("Group selection preference");
    let page = document.pages[0].id;
    let section = layer(&mut document, page, NodeKind::Section, 0.0, 0.0);
    let frame = layer(&mut document, section, NodeKind::Frame, 0.0, 0.0);
    let group = layer(&mut document, frame, NodeKind::Group, 0.0, 0.0);
    let leaf = layer(&mut document, group, NodeKind::Rectangle, 0.0, 0.0);
    let sibling = layer(&mut document, group, NodeKind::Rectangle, 0.0, 0.0);
    let other_group = layer(&mut document, frame, NodeKind::Group, 0.0, 0.0);
    let other_leaf = layer(&mut document, other_group, NodeKind::Rectangle, 0.0, 0.0);
    assert_eq!(
        SelectionContext::default().grouped_target(&document, leaf, false),
        Some(frame)
    );
    let context = SelectionContext::new(&document, &[frame]);
    assert_eq!(context.grouped_target(&document, leaf, false), Some(frame));
    assert_eq!(context.grouped_target(&document, leaf, true), Some(group));
    let context = SelectionContext::new(&document, &[group]);
    assert_eq!(context.grouped_target(&document, leaf, true), Some(leaf));
    let context = SelectionContext::new(&document, &[leaf]);
    assert_eq!(
        context.grouped_target(&document, sibling, false),
        Some(sibling)
    );
    assert_eq!(
        context.grouped_target(&document, other_leaf, false),
        Some(other_group)
    );
    assert_eq!(
        context.grouped_target(&document, section, false),
        Some(section)
    );
    assert_eq!(context.grouped_target(&document, u64::MAX, false), None);
}
