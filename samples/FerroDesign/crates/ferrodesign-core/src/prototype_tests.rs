use crate::*;
use std::collections::BTreeMap;

fn fixture() -> (Document, NodeId, NodeId, NodeId, NodeId) {
    let mut document = Document::new("Prototype tests");
    let page = document.pages[0].id;
    let a = document
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "A", 0.0, 0.0, 300.0, 300.0),
        )
        .unwrap();
    let b = document
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "B", 400.0, 0.0, 300.0, 300.0),
        )
        .unwrap();
    let overlay = document
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Overlay", 800.0, 0.0, 100.0, 100.0),
        )
        .unwrap();
    let button = document
        .insert_node(
            a,
            Node::new(NodeKind::Rectangle, "Button", 20.0, 20.0, 50.0, 50.0),
        )
        .unwrap();
    document.pages[0].prototype_start = Some(a);
    (document, a, b, overlay, button)
}

fn interaction(trigger: Trigger, action: PrototypeAction) -> Interaction {
    Interaction {
        trigger,
        action,
        transition: Transition::default(),
        delay_ms: 0,
        key: None,
    }
}

fn delay(delay_ms: u32, action: PrototypeAction) -> Interaction {
    Interaction {
        delay_ms,
        ..interaction(Trigger::AfterDelay, action)
    }
}

fn url(value: &str) -> PrototypeAction {
    PrototypeAction::OpenUrl { url: value.into() }
}

fn transition_scene(document: &Document, from: NodeId, to: NodeId, kind: TransitionKind) -> PrototypeScene {
    PrototypeScene::new(
        PrototypeFrame::capture(document, from, &[]).unwrap(),
        PrototypeFrame::capture(document, to, &[]).unwrap(),
        Transition { kind, duration_ms: 1000, easing: Easing::Linear },
    )
}

#[test]
fn prototype_smart_animate_interpolates_named_geometry_paints_and_short_rotation() {
    let (mut document, a, b, _, button) = fixture();
    let start = document.nodes.get_mut(&button).unwrap();
    start.rotation = 350.0;
    start.fills = vec![Fill::solid(Color::rgb(255, 0, 0))];
    start.corner_radius = 2.0;
    let mut end = Node::new(NodeKind::Rectangle, "Button", 120.0, 80.0, 150.0, 90.0);
    end.rotation = 10.0;
    end.fills = vec![Fill::solid(Color::rgb(0, 0, 255))];
    end.corner_radius = 18.0;
    document.insert_node(b, end).unwrap();
    let authored = document.clone();
    let mut scene = transition_scene(&document, a, b, TransitionKind::SmartAnimate);
    assert!(!scene.sample(500.0));
    let nodes: Vec<_> = scene.document.nodes.values().filter(|node| node.name == "Button").collect();
    assert_eq!(nodes.len(), 1, "a matched layer must morph, not duplicate/crossfade");
    let node = nodes[0];
    assert_eq!((node.x, node.y, node.width, node.height), (70.0, 50.0, 100.0, 70.0));
    assert_eq!(node.rotation, 360.0);
    assert_eq!(node.corner_radius, 10.0);
    assert_eq!(node.fills[0].paint, Paint::Solid(Color::rgb(128, 0, 128)));
    assert_eq!(scene.animated_nodes(), 1, "unchanged ancestors require no property tracks");
    assert!(scene.sample(1000.0));
    assert_eq!(scene.document.nodes.values().find(|node| node.name == "Button").unwrap().x, 120.0);
    assert_eq!(document, authored, "animation never mutates authored nodes");
}

#[test]
fn prototype_smart_animate_matches_duplicate_names_by_sibling_occurrence() {
    let (mut document, a, b, _, _) = fixture();
    for (parent, positions) in [(a, [0.0, 40.0]), (b, [80.0, 160.0])] {
        for x in positions {
            document.insert_node(parent, Node::new(NodeKind::Rectangle, "Repeated", x, 100.0, 10.0, 10.0)).unwrap();
        }
    }
    let mut scene = transition_scene(&document, a, b, TransitionKind::SmartAnimate);
    scene.sample(500.0);
    let mut positions: Vec<_> = scene.document.nodes.values().filter(|node| node.name == "Repeated").map(|node| node.x).collect();
    positions.sort_by(f64::total_cmp);
    assert_eq!(positions, vec![40.0, 100.0]);
}

#[test]
fn prototype_smart_animate_crossfades_changed_text_and_reparented_layers() {
    let (mut document, a, b, _, _) = fixture();
    for (parent, text, group_name) in [(a, "Before", "Old group"), (b, "After", "New group")] {
        let group = document.insert_node(parent, Node::new(NodeKind::Group, group_name, 0.0, 0.0, 100.0, 100.0)).unwrap();
        document.insert_node(group, Node::new(NodeKind::Rectangle, "Shared child", 5.0, 5.0, 10.0, 10.0)).unwrap();
        let mut label = Node::new(NodeKind::Text, "Label", 5.0, 130.0, 100.0, 30.0);
        label.text = text.into();
        document.insert_node(parent, label).unwrap();
    }
    let mut scene = transition_scene(&document, a, b, TransitionKind::SmartAnimate);
    scene.sample(500.0);
    let labels: Vec<_> = scene.document.nodes.values().filter(|node| node.name == "Label").collect();
    assert_eq!(labels.len(), 2);
    assert!(labels.iter().all(|node| node.opacity == 0.5));
    assert_eq!(scene.document.nodes.values().filter(|node| node.name == "Shared child").count(), 2);
    for name in ["Old group", "New group"] {
        assert_eq!(scene.document.nodes.values().find(|node| node.name == name).unwrap().opacity, 0.5);
    }
}

#[test]
fn prototype_slide_tracks_only_two_roots_and_moves_both_inside_a_clip() {
    let (mut document, a, b, _, _) = fixture();
    for i in 0..1000 {
        document.insert_node(a, Node::new(NodeKind::Rectangle, format!("Layer {i}"), 0.0, i as f64, 10.0, 10.0)).unwrap();
    }
    let mut scene = transition_scene(&document, a, b, TransitionKind::SlideLeft);
    let ids = scene.document.nodes[&scene.root].children.clone();
    assert!(scene.document.nodes[&scene.root].clip_content);
    assert_eq!(scene.animated_nodes(), 2);
    let node_count = scene.document.nodes.len();
    scene.sample(500.0);
    assert_eq!(scene.document.nodes[&ids[0]].x, -150.0);
    assert_eq!(scene.document.nodes[&ids[1]].x, 150.0);
    assert_eq!(scene.document.nodes[&ids[0]].opacity, 1.0);
    assert_eq!(scene.document.nodes.len(), node_count);
    assert!(scene.sample(1200.0));
    assert_eq!(scene.document.nodes[&ids[1]].x, 0.0);
}

#[test]
fn prototype_frame_capture_copies_only_active_subtrees_and_positions_overlays() {
    let (document, a, b, overlay, _) = fixture();
    let frame = PrototypeFrame::capture(&document, b, &[(overlay, Point::new(30.0, 40.0))]).unwrap();
    assert_eq!(frame.origin, Point::new(400.0, 0.0));
    assert_eq!(frame.bounds(), Rect::new(0.0, 0.0, 300.0, 300.0));
    assert!(!frame.document.nodes.values().any(|node| node.name == document.nodes[&a].name || node.name == "Button"));
    let overlay = frame.document.nodes.values().find(|node| node.name.starts_with("__prototype_overlay_")).unwrap();
    assert_eq!((overlay.x, overlay.y), (30.0, 40.0));
    assert!(frame.document.nodes.values().all(|node| node.interactions.is_empty()));
}

#[test]
fn prototype_overlay_motion_keeps_the_base_frame_still_and_fades_its_scrim() {
    let (document, a, _, overlay, _) = fixture();
    let mut scene = PrototypeScene::new(
        PrototypeFrame::capture(&document, a, &[]).unwrap(),
        PrototypeFrame::capture(&document, a, &[(overlay, Point::new(30.0, 40.0))]).unwrap(),
        Transition { kind: TransitionKind::SlideLeft, duration_ms: 1000, easing: Easing::Linear },
    );
    scene.sample(500.0);
    let base = scene.document.nodes.values().find(|node| node.name == "__prototype_frame").unwrap();
    assert_eq!((base.x, base.y, base.opacity), (0.0, 0.0, 1.0));
    let panel = scene.document.nodes.values().find(|node| node.name.starts_with("__prototype_overlay_")).unwrap();
    assert_eq!((panel.x, panel.y, panel.opacity), (180.0, 40.0, 1.0));
    let scrim = scene.document.nodes.values().find(|node| node.name.starts_with("__prototype_scrim_")).unwrap();
    assert_eq!((scrim.x, scrim.y, scrim.opacity), (0.0, 0.0, 0.5));
}

#[test]
fn prototype_opaque_dissolve_does_not_dim_the_source_twice() {
    let (document, a, b, _, _) = fixture();
    let mut scene = transition_scene(&document, a, b, TransitionKind::Dissolve);
    scene.sample(500.0);
    let roots = &scene.document.nodes[&scene.root].children;
    assert_eq!(scene.document.nodes[&roots[0]].opacity, 1.0);
    assert_eq!(scene.document.nodes[&roots[1]].opacity, 0.5);
}

#[test]
fn prototype_easing_and_zero_duration_have_finite_exact_endpoints() {
    for easing in [Easing::Linear, Easing::EaseIn, Easing::EaseOut, Easing::EaseInOut, Easing::Spring] {
        assert_eq!(prototype_easing(easing, -1.0), 0.0);
        assert_eq!(prototype_easing(easing, 1.0), 1.0);
        assert_eq!(prototype_easing(easing, f64::NAN), 0.0);
        assert!(prototype_easing(easing, 0.5).is_finite());
    }
    assert!(prototype_easing(Easing::EaseIn, 0.5) < 0.5);
    assert!(prototype_easing(Easing::EaseOut, 0.5) > 0.5);
    assert!((prototype_easing(Easing::EaseInOut, 0.5) - 0.5).abs() < 0.000001);
    assert!((1..100).any(|i| prototype_easing(Easing::Spring, i as f64 / 100.0) > 1.0));
    let (document, a, b, _, _) = fixture();
    let mut scene = transition_scene(&document, a, b, TransitionKind::Dissolve);
    scene.transition.duration_ms = 0;
    assert!(scene.sample(0.0));
    assert_eq!(scene.progress, 1.0);
}

#[test]
fn prototype_vertical_scrolling_clamps_content_and_preserves_the_other_axis() {
    let (mut document, a, _, _, button) = fixture();
    document.nodes.get_mut(&a).unwrap().prototype_overflow = PrototypeOverflow::Vertical;
    document.nodes.get_mut(&button).unwrap().y = 620.0;
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    assert!(player.scroll_by(&mut document, button, Point::new(80.0, 400.0)));
    assert_eq!(player.scroll.offset(a), Point::new(0.0, 370.0));
    assert_eq!((document.nodes[&button].x, document.nodes[&button].y), (20.0, 250.0));
    assert!(!player.scroll_by(&mut document, button, Point::new(0.0, 100.0)));
    assert!(player.scroll_by(&mut document, button, Point::new(0.0, -1000.0)));
    assert_eq!(player.scroll.offset(a), Point::default());
    assert_eq!(document.nodes[&button].y, 620.0);
    let before = document.clone();
    assert!(!player.scroll_by(&mut document, button, Point::new(f64::NAN, 1.0)));
    assert_eq!(document, before);
}

#[test]
fn prototype_nested_scroll_chains_boundaries_and_scroll_to_aligns_the_target() {
    let (mut document, a, _, _, _) = fixture();
    document.nodes.get_mut(&a).unwrap().prototype_overflow = PrototypeOverflow::Vertical;
    document.insert_node(a, Node::new(NodeKind::Rectangle, "Tail", 0.0, 600.0, 20.0, 50.0)).unwrap();
    let mut inner = Node::new(NodeKind::Frame, "Inner", 20.0, 20.0, 200.0, 100.0);
    inner.prototype_overflow = PrototypeOverflow::Vertical;
    let inner = document.insert_node(a, inner).unwrap();
    let target = document.insert_node(inner, Node::new(NodeKind::Rectangle, "Target", 0.0, 260.0, 20.0, 40.0)).unwrap();
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    assert!(player.scroll_by(&mut document, target, Point::new(0.0, 250.0)));
    assert_eq!(player.scroll.offset(inner).y, 200.0);
    assert_eq!(player.scroll.offset(a).y, 50.0);
    assert_eq!(document.world_bounds(target).unwrap().y, 30.0);
    player.scroll.reset_subtree(&mut document, a);
    assert!(player.scroll.scroll_to(&mut document, a, target));
    assert_eq!(player.scroll.offset(inner).y, 200.0);
    assert_eq!(player.scroll.offset(a).y, 80.0);
    assert_eq!(document.world_bounds(target).unwrap().y, 0.0);
    assert_eq!(document.nodes[&inner].height, 100.0);
}

#[test]
fn prototype_navigation_resets_destination_scroll_and_back_keeps_prior_offset() {
    let (mut document, a, b, _, button) = fixture();
    document.nodes.get_mut(&a).unwrap().prototype_overflow = PrototypeOverflow::Vertical;
    document.nodes.get_mut(&button).unwrap().y = 620.0;
    document.nodes.get_mut(&a).unwrap().interactions = vec![interaction(Trigger::Click, PrototypeAction::NavigateTo { target: b })];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    player.scroll_by(&mut document, a, Point::new(0.0, 200.0));
    assert!(player.trigger(&mut document, a, Trigger::Click));
    assert!(player.back(&document));
    assert_eq!(player.scroll.offset(a).y, 200.0);
    document.nodes.get_mut(&a).unwrap().interactions[0].action = PrototypeAction::NavigateTo { target: a };
    assert!(player.trigger(&mut document, a, Trigger::Click));
    assert_eq!(player.scroll.offset(a).y, 0.0);
    assert_eq!(document.nodes[&button].y, 620.0);
}

#[test]
fn prototype_overlay_scroll_cannot_move_the_background_frame() {
    let (mut document, a, _, overlay, button) = fixture();
    document.nodes.get_mut(&a).unwrap().prototype_overflow = PrototypeOverflow::Vertical;
    document.nodes.get_mut(&button).unwrap().y = 620.0;
    document.nodes.get_mut(&a).unwrap().interactions = vec![interaction(Trigger::Click, PrototypeAction::OpenOverlay { target: overlay, position: Point::default() })];
    document.nodes.get_mut(&overlay).unwrap().interactions = vec![interaction(Trigger::Click, PrototypeAction::ScrollTo { target: button })];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    assert!(player.trigger(&mut document, a, Trigger::Click));
    assert!(!player.scroll_by(&mut document, button, Point::new(0.0, 100.0)));
    assert!(!player.trigger(&mut document, overlay, Trigger::Click));
    assert_eq!(document.nodes[&button].y, 620.0);
}

#[test]
fn prototype_while_hover_restores_navigation_and_uses_original_hit_region() {
    let (mut document, a, b, _, button) = fixture();
    document.nodes.get_mut(&button).unwrap().interactions = vec![Interaction {
        transition: Transition { kind: TransitionKind::SlideLeft, duration_ms: 400, easing: Easing::EaseIn },
        ..interaction(Trigger::WhileHover, PrototypeAction::NavigateTo { target: b })
    }];
    document.nodes.get_mut(&a).unwrap().interactions = vec![delay(500, url("https://example.com/timer"))];
    let before = document.clone();
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    let stale = player.pending_timers(&document)[0];
    assert!(player.trigger(&mut document, button, Trigger::Hover));
    assert_eq!(player.current, Some(b));
    assert!(player.transient_contains(Trigger::WhileHover, Point::new(25.0, 25.0)));
    assert!(!player.transient_contains(Trigger::WhileHover, Point::new(200.0, 200.0)));
    assert!(player.end_transient(&mut document, Trigger::WhileHover));
    assert_eq!(player.current, Some(a));
    assert!(player.history.is_empty());
    assert_eq!(document, before);
    assert_eq!(player.last_transition.kind, TransitionKind::SlideRight);
    assert_eq!(player.last_transition.easing, Easing::EaseOut);
    assert!(!player.fire_timer(&mut document, stale, 500));
    assert_ne!(player.pending_timers(&document)[0].activation, stale.activation);
}

#[test]
fn prototype_while_press_restores_inside_hover_then_hover_restores_authored_preview() {
    let (mut document, a, _, overlay, button) = fixture();
    document.nodes.get_mut(&button).unwrap().interactions = vec![interaction(Trigger::WhileHover, PrototypeAction::OpenOverlay { target: overlay, position: Point::new(10.0, 20.0) })];
    document.nodes.get_mut(&overlay).unwrap().interactions = vec![interaction(Trigger::WhilePress, PrototypeAction::CloseOverlay)];
    let before = document.clone();
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    assert!(player.trigger(&mut document, button, Trigger::Hover));
    assert!(player.trigger(&mut document, overlay, Trigger::Press));
    assert!(player.overlays.is_empty());
    assert!(player.end_transient(&mut document, Trigger::WhilePress));
    assert_eq!(player.overlays, vec![(overlay, Point::new(10.0, 20.0))]);
    assert!(player.has_transient(Trigger::WhileHover));
    assert!(player.end_transient(&mut document, Trigger::WhileHover));
    assert!(player.overlays.is_empty());
    assert_eq!(document, before);
    assert!(!player.has_transients());
    let rule = interaction(Trigger::WhileHover, url("https://example.com"));
    assert!(validate_prototype_interaction(&document, button, &rule).is_err());
    document.nodes.get_mut(&button).unwrap().interactions = vec![rule];
    assert!(!player.trigger(&mut document, button, Trigger::Hover));
    assert!(player.pending_url.is_none());
}

#[test]
fn prototype_smart_animate_morphs_gradient_path_and_disabling_blur() {
    let (mut document, a, b, _, _) = fixture();
    for (parent, amount, visible) in [(a, 0.0, true), (b, 20.0, false)] {
        let mut node = Node::new(NodeKind::Vector, "Morph", 100.0, 100.0, 100.0, 100.0);
        node.path.segments = vec![PathSegment::MoveTo { point: Point::default() }, PathSegment::CubicTo { control1: Point::new(amount, 10.0), control2: Point::new(80.0, 90.0), point: Point::new(100.0, 100.0) }];
        node.fills = vec![Fill {
            paint: Paint::LinearGradient {
            start: Point::new(amount, 0.0), end: Point::new(100.0, 100.0),
            stops: vec![GradientStop { offset: amount / 100.0, color: if visible { Color::BLACK } else { Color::WHITE } }],
            },
            ..Fill::solid(Color::BLACK)
        }];
        node.effects = vec![Effect { kind: EffectKind::LayerBlur, color: Color::BLACK, offset: Point::default(), radius: 20.0, spread: 0.0, visible }];
        document.insert_node(parent, node).unwrap();
    }
    let mut scene = transition_scene(&document, a, b, TransitionKind::SmartAnimate);
    scene.sample(500.0);
    let node = scene.document.nodes.values().find(|node| node.name == "Morph").unwrap();
    assert_eq!(node.effects[0].radius, 10.0);
    match &node.path.segments[1] {
        PathSegment::CubicTo { control1, .. } => assert_eq!(*control1, Point::new(10.0, 10.0)),
        _ => panic!("matching cubic paths must interpolate"),
    }
    match &node.fills[0].paint {
        Paint::LinearGradient { start, stops, .. } => {
            assert_eq!(start.x, 10.0);
            assert_eq!(stops[0].offset, 0.1);
            assert_eq!(stops[0].color, Color::rgb(128, 128, 128));
        }
        _ => panic!("matching gradients must interpolate"),
    }
    scene.sample(1000.0);
    assert_eq!(scene.document.nodes.values().find(|node| node.name == "Morph").unwrap().effects[0].radius, 0.0);
}

#[test]
fn prototype_key_chords_match_modifiers_aliases_and_exact_authored_keys() {
    assert_eq!(
        normalize_prototype_key(" Shift + Control + A "),
        Some("ctrl+shift+a".into())
    );
    assert_eq!(
        normalize_prototype_key("Command+Option+Return"),
        Some("alt+meta+enter".into())
    );
    assert_eq!(normalize_prototype_key("Control++"), Some("ctrl++".into()));
    assert_eq!(
        normalize_prototype_key("Ctrl+Shift++"),
        Some("ctrl++".into())
    );
    assert_eq!(normalize_prototype_key("Shift+!"), Some("!".into()));
    assert_eq!(
        normalize_prototype_key("Ctrl+Back"),
        Some("ctrl+backspace".into())
    );
    assert_eq!(
        normalize_prototype_key("Backspace"),
        Some("backspace".into())
    );
    assert_eq!(normalize_prototype_key("Shift+A"), Some("shift+a".into()));
    assert_eq!(normalize_prototype_key("Shift+F1"), Some("shift+f1".into()));
    assert_eq!(normalize_prototype_key(" "), Some("space".into()));
    assert_eq!(normalize_prototype_key("Ctrl+A+B"), None);
    assert_eq!(normalize_prototype_key("Ctrl+Shift"), None);
    let (mut document, a, b, _, _) = fixture();
    document.nodes.get_mut(&a).unwrap().interactions = vec![
        Interaction {
            key: Some("Ctrl+Shift+A".into()),
            ..interaction(Trigger::Key, PrototypeAction::NavigateTo { target: b })
        },
        Interaction {
            key: Some("Escape".into()),
            ..interaction(Trigger::Key, url("https://example.com/escape"))
        },
        Interaction {
            key: Some("Ctrl++".into()),
            ..interaction(Trigger::Key, url("https://example.com/plus"))
        },
        Interaction {
            key: Some("Backspace".into()),
            ..interaction(Trigger::Key, url("https://example.com/backspace"))
        },
    ];
    let mut player = PrototypePlayer::default();
    assert!(player.start_at(&document, a));
    assert!(!player.trigger(&mut document, a, Trigger::Key));
    assert!(!player.trigger_key(&mut document, a, "Ctrl+A"));
    assert!(!player.trigger_key(&mut document, a, "A"));
    assert!(!player.trigger_key(&mut document, a, "Meta+Shift+A"));
    assert!(player.trigger_key(&mut document, a, "Esc"));
    assert_eq!(
        player.pending_url.as_deref(),
        Some("https://example.com/escape")
    );
    assert!(player.trigger_key(&mut document, a, "Ctrl+Shift++"));
    assert_eq!(
        player.pending_url.as_deref(),
        Some("https://example.com/plus")
    );
    assert!(player.trigger_key(&mut document, a, "Backspace"));
    assert_eq!(
        player.pending_url.as_deref(),
        Some("https://example.com/backspace")
    );
    assert!(player.trigger_key(&mut document, a, "SHIFT+CONTROL+a"));
    assert_eq!(player.current, Some(b));
    assert_eq!(player.history, [a]);
}

#[test]
fn prototype_hover_press_drag_respect_visibility_and_allow_locked_design_layers() {
    let (mut document, a, _, _, button) = fixture();
    document.nodes.get_mut(&button).unwrap().locked = true;
    document.nodes.get_mut(&button).unwrap().interactions = vec![
        interaction(Trigger::Hover, url("https://example.com/hover")),
        interaction(Trigger::Press, url("https://example.com/press")),
        interaction(Trigger::Drag, url("https://example.com/drag")),
    ];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    for (trigger, expected) in [
        (Trigger::Hover, "hover"),
        (Trigger::Press, "press"),
        (Trigger::Drag, "drag"),
    ] {
        assert!(player.trigger(&mut document, button, trigger));
        assert_eq!(
            player.pending_url,
            Some(format!("https://example.com/{expected}"))
        );
    }
    document.nodes.get_mut(&button).unwrap().visible = false;
    let before = player.clone();
    assert!(!player.trigger(&mut document, button, Trigger::Hover));
    assert_eq!(player, before);
}

#[test]
fn prototype_delay_requires_elapsed_time_and_fires_only_once_per_activation() {
    let (mut document, a, _, _, button) = fixture();
    document.nodes.get_mut(&button).unwrap().interactions =
        vec![delay(240, url("https://example.com/delayed"))];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    let timers = player.pending_timers(&document);
    assert_eq!(timers.len(), 1);
    let timer = timers[0];
    assert!(!player.trigger(&mut document, button, Trigger::AfterDelay));
    assert!(!player.fire_timer(&mut document, timer, 239));
    assert_eq!(player.pending_timers(&document), timers);
    assert!(player.fire_timer(&mut document, timer, 240));
    assert!(!player.fire_timer(&mut document, timer, 2000));
    assert!(player.pending_timers(&document).is_empty());
    player.stop();
    assert!(!player.fire_timer(&mut document, timer, 2000));
    player.start_at(&document, a);
    let restarted = player.pending_timers(&document)[0];
    assert_ne!(restarted.activation, timer.activation);
    assert!(!player.fire_timer(&mut document, timer, 2000));
    assert!(player.fire_timer(&mut document, restarted, 240));
}

#[test]
fn prototype_navigation_cancels_queued_delays_including_same_frame_reentry() {
    let (mut document, a, b, _, _) = fixture();
    document.nodes.get_mut(&a).unwrap().interactions = vec![
        delay(50, PrototypeAction::NavigateTo { target: b }),
        delay(50, url("https://example.com/stale")),
    ];
    document.nodes.get_mut(&b).unwrap().interactions =
        vec![delay(0, PrototypeAction::NavigateTo { target: b })];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    let old = player.pending_timers(&document);
    assert_eq!(
        old.iter()
            .map(|timer| timer.interaction)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert!(player.fire_timer(&mut document, old[0], 50));
    assert_eq!(player.current, Some(b));
    assert!(!player.fire_timer(&mut document, old[1], 50));
    assert!(player.pending_url.is_none());
    let self_timer = player.pending_timers(&document)[0];
    assert!(player.fire_timer(&mut document, self_timer, 0));
    let next_timer = player.pending_timers(&document)[0];
    assert_ne!(self_timer.activation, next_timer.activation);
    assert!(!player.fire_timer(&mut document, self_timer, 0));
    assert_eq!(player.history, [a]);
}

#[test]
fn prototype_overlay_scopes_preserve_base_timers_and_rearm_only_reopened_instances() {
    let (mut document, a, _, overlay, _) = fixture();
    document.nodes.get_mut(&a).unwrap().interactions = vec![
        delay(100, url("https://example.com/base")),
        interaction(
            Trigger::Click,
            PrototypeAction::OpenOverlay {
                target: overlay,
                position: Point::new(25.0, 30.0),
            },
        ),
    ];
    document.nodes.get_mut(&overlay).unwrap().interactions = vec![
        delay(200, url("https://example.com/overlay")),
        interaction(Trigger::Click, PrototypeAction::CloseOverlay),
    ];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    let base = player.pending_timers(&document)[0];
    assert!(player.trigger(&mut document, a, Trigger::Click));
    assert_eq!(player.interaction_root(), Some(overlay));
    let active = player.pending_timers(&document);
    assert_eq!(active[0], base);
    let stale_overlay = active[1];
    assert!(!player.trigger(&mut document, a, Trigger::Click));
    assert!(player.fire_timer(&mut document, base, 100));
    assert!(player.trigger(&mut document, overlay, Trigger::Click));
    assert!(!player.fire_timer(&mut document, stale_overlay, 200));
    assert!(player.pending_timers(&document).is_empty());
    assert!(player.trigger(&mut document, a, Trigger::Click));
    let reopened = player.pending_timers(&document);
    assert_eq!(reopened.len(), 1);
    assert_ne!(reopened[0].activation, stale_overlay.activation);
    assert!(player.fire_timer(&mut document, reopened[0], 200));
    assert!(player.back(&document));
    assert_eq!(player.current, Some(a));
    assert!(player.overlays.is_empty());
}

#[test]
fn prototype_edited_hidden_and_long_delay_events_never_fire_early_or_stale() {
    let (mut document, a, _, _, button) = fixture();
    document.nodes.get_mut(&button).unwrap().interactions =
        vec![delay(u32::MAX, url("https://example.com/long"))];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    let long = player.pending_timers(&document)[0];
    assert!(!player.fire_timer(&mut document, long, i32::MAX as u64));
    document.nodes.get_mut(&button).unwrap().interactions[0].delay_ms = 100;
    assert!(!player.fire_timer(&mut document, long, u32::MAX as u64));
    let edited = player.pending_timers(&document)[0];
    assert_eq!(edited.delay_ms, 100);
    document.nodes.get_mut(&button).unwrap().visible = false;
    assert!(!player.fire_timer(&mut document, edited, 100));
    assert!(player.pending_timers(&document).is_empty());
    document.nodes.get_mut(&button).unwrap().visible = true;
    assert!(player.fire_timer(&mut document, edited, 100));
}

#[test]
fn prototype_invalid_actions_leave_transition_navigation_and_variables_unchanged() {
    let (mut document, a, _, _, _) = fixture();
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    for action in [
        PrototypeAction::NavigateTo { target: 99999 },
        PrototypeAction::OpenOverlay {
            target: a,
            position: Point::new(f64::NAN, 0.0),
        },
        PrototypeAction::CloseOverlay,
        PrototypeAction::Back,
        PrototypeAction::ScrollTo { target: 99999 },
        PrototypeAction::SetVariable {
            variable: 99999,
            value: VariableValue::Number(1.0),
        },
        url("javascript:alert(1)"),
        url("data:text/html,hello"),
        url("https:///no-host"),
    ] {
        document.nodes.get_mut(&a).unwrap().interactions = vec![Interaction {
            transition: Transition {
                kind: TransitionKind::SlideLeft,
                duration_ms: 1234,
                easing: Easing::Spring,
            },
            ..interaction(Trigger::Click, action)
        }];
        let before = player.clone();
        assert!(!player.trigger(&mut document, a, Trigger::Click));
        assert_eq!(player, before);
    }
}

#[test]
fn prototype_invalid_delayed_action_is_consumed_once_without_a_busy_retry() {
    let (mut document, a, _, _, _) = fixture();
    document.nodes.get_mut(&a).unwrap().interactions =
        vec![delay(0, PrototypeAction::NavigateTo { target: 99999 })];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    let timer = player.pending_timers(&document)[0];
    assert!(!player.fire_timer(&mut document, timer, 0));
    assert!(player.pending_timers(&document).is_empty());
    assert!(!player.fire_timer(&mut document, timer, 0));
    assert_eq!(player.current, Some(a));
}

#[test]
fn prototype_variables_are_preview_overrides_with_authored_aliases_and_modes_intact() {
    let (mut document, a, _, _, button) = fixture();
    let color = document.allocate_id();
    let alias = document.allocate_id();
    document.variables.insert(
        color,
        Variable {
            id: color,
            name: "Accent".into(),
            collection: "Theme".into(),
            values: BTreeMap::from([(
                "Default".into(),
                VariableValue::Color(Color::rgb(255, 0, 0)),
            )]),
        },
    );
    document.variables.insert(
        alias,
        Variable {
            id: alias,
            name: "Button".into(),
            collection: "Theme".into(),
            values: BTreeMap::from([("Default".into(), VariableValue::Alias(color))]),
        },
    );
    document.variable_mode = "Unstored preview mode".into();
    document
        .nodes
        .get_mut(&button)
        .unwrap()
        .variable_bindings
        .insert("fill".into(), alias);
    document.apply_variables();
    let authored_variables = document.variables.clone();
    let authored_document = document.clone();
    document.nodes.get_mut(&a).unwrap().interactions = vec![interaction(
        Trigger::Click,
        PrototypeAction::SetVariable {
            variable: color,
            value: VariableValue::Color(Color::rgb(0, 0, 255)),
        },
    )];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    assert!(player.trigger(&mut document, a, Trigger::Click));
    assert_eq!(document.nodes[&button].fill_color(), Color::rgb(0, 0, 255));
    assert_eq!(document.variables, authored_variables);
    assert_eq!(
        authored_document.nodes[&button].fill_color(),
        Color::rgb(255, 0, 0)
    );
    assert_eq!(
        player.variable_overrides().get(&color),
        Some(&VariableValue::Color(Color::rgb(0, 0, 255)))
    );
    document.nodes.get_mut(&a).unwrap().interactions = vec![interaction(
        Trigger::Click,
        PrototypeAction::SetVariable {
            variable: color,
            value: VariableValue::Alias(alias),
        },
    )];
    assert!(!player.trigger(&mut document, a, Trigger::Click));
    assert_eq!(document.variables, authored_variables);
    document.validate().unwrap();
}

#[test]
fn prototype_text_variable_updates_keep_unicode_rich_text_ranges_valid() {
    let (mut document, a, _, _, _) = fixture();
    let text = document
        .insert_node(
            a,
            Node::new(NodeKind::Text, "Label", 20.0, 100.0, 120.0, 24.0),
        )
        .unwrap();
    let variable = document.allocate_id();
    let original = "Hello 🦀 Ω".to_string();
    document.variables.insert(
        variable,
        Variable {
            id: variable,
            name: "Label".into(),
            collection: "Content".into(),
            values: BTreeMap::from([("Default".into(), VariableValue::String(original.clone()))]),
        },
    );
    {
        let node = document.nodes.get_mut(&text).unwrap();
        node.text = original.clone();
        node.text_runs = vec![TextRun {
            start: 0,
            end: original.len(),
            font_family: None,
            font_size: None,
            font_weight: Some(700),
            color: None,
            italic: None,
            decoration: None,
        }];
        node.variable_bindings.insert("text".into(), variable);
    }
    document.nodes.get_mut(&a).unwrap().interactions = vec![interaction(
        Trigger::Click,
        PrototypeAction::SetVariable {
            variable,
            value: VariableValue::String("🦀".into()),
        },
    )];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    assert!(player.trigger(&mut document, a, Trigger::Click));
    assert_eq!(document.nodes[&text].text, "🦀");
    assert_eq!(document.nodes[&text].text_runs[0].start, 0);
    assert_eq!(document.nodes[&text].text_runs[0].end, 4);
    assert_eq!(document.nodes[&text].text_runs[0].font_weight, Some(700));
    assert_eq!(
        document.variables[&variable].values["Default"],
        VariableValue::String(original)
    );
    document.validate().unwrap();
}

#[test]
fn prototype_external_and_scroll_actions_remain_explicit_host_requests() {
    let (mut document, a, b, _, button) = fixture();
    document.nodes.get_mut(&a).unwrap().interactions = vec![interaction(
        Trigger::Click,
        PrototypeAction::ScrollTo { target: button },
    )];
    let mut player = PrototypePlayer::default();
    player.start_at(&document, a);
    assert!(player.trigger(&mut document, a, Trigger::Click));
    assert_eq!(player.scroll_target, Some(button));
    assert_eq!(player.current, Some(a));
    assert_eq!(
        player.last_action,
        Some(PrototypeAction::ScrollTo { target: button })
    );
    document.nodes.get_mut(&a).unwrap().interactions = vec![interaction(
        Trigger::Click,
        PrototypeAction::ScrollTo { target: b },
    )];
    assert!(!player.trigger(&mut document, a, Trigger::Click));
    for link in [
        "https://example.com/path?q=a#b",
        "http://localhost:8080/",
        "mailto:design@example.com",
        "tel:+48123456789",
    ] {
        assert_eq!(prototype_external_url(link).as_deref(), Some(link));
    }
    for link in [
        "file:///secret",
        "javascript:alert(1)",
        "https://",
        "https://bad host/",
        "https://example.com/\u{0000}tail",
    ] {
        assert!(prototype_external_url(link).is_none());
    }
}

#[test]
fn prototype_hit_filters_before_z_order_and_retains_clipping_and_cross_page_overlays() {
    let (mut document, a, b, _, button) = fixture();
    document.nodes.get_mut(&b).unwrap().x = 0.0;
    document.nodes.get_mut(&button).unwrap().locked = true;
    let mut clip = Node::new(NodeKind::Frame, "Clipped content", 100.0, 100.0, 40.0, 40.0);
    clip.clip_content = true;
    let clip = document.insert_node(a, clip).unwrap();
    let child = document
        .insert_node(
            clip,
            Node::new(NodeKind::Rectangle, "Overflow", 20.0, 20.0, 100.0, 100.0),
        )
        .unwrap();
    let other_page = document.add_page("Overlay page");
    let overlay = document
        .insert_node(
            other_page,
            Node::new(
                NodeKind::Frame,
                "Other page overlay",
                0.0,
                0.0,
                100.0,
                100.0,
            ),
        )
        .unwrap();
    let overlay_child = document
        .insert_node(
            overlay,
            Node::new(NodeKind::Rectangle, "Overlay button", 5.0, 5.0, 30.0, 30.0),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    assert_eq!(editor.hit_test(Point::new(25.0, 25.0), 0.0), Some(b));
    assert_eq!(
        editor.prototype_hit_test(a, Point::new(25.0, 25.0), 0.0),
        Some(button)
    );
    assert_eq!(
        editor.prototype_hit_test(a, Point::new(130.0, 130.0), 0.0),
        Some(child)
    );
    assert_eq!(
        editor.prototype_hit_test(a, Point::new(160.0, 160.0), 0.0),
        Some(a)
    );
    assert_eq!(
        editor.prototype_hit_test(overlay, Point::new(10.0, 10.0), 0.0),
        Some(overlay_child)
    );
    editor
        .document
        .nodes
        .get_mut(&overlay_child)
        .unwrap()
        .visible = false;
    editor.refresh();
    assert_eq!(
        editor.prototype_hit_test(overlay, Point::new(10.0, 10.0), 0.0),
        Some(overlay)
    );
    assert_eq!(
        editor.prototype_hit_test(a, Point::new(f64::NAN, 0.0), 0.0),
        None
    );
}

#[test]
fn prototype_nested_roots_ignore_external_clips_but_preserve_internal_clip_and_boolean_rules() {
    let mut document = Document::new("Nested presentation roots");
    let page = document.pages[0].id;
    let mut ancestor = Node::new(NodeKind::Frame, "Authored clip", 10.0, 20.0, 20.0, 20.0);
    ancestor.clip_content = true;
    let ancestor = document.insert_node(page, ancestor).unwrap();
    let mut root = Node::new(NodeKind::Frame, "Overlay", 100.0, 100.0, 80.0, 80.0);
    root.clip_content = true;
    let root = document.insert_node(ancestor, root).unwrap();
    let button = document
        .insert_node(
            root,
            Node::new(NodeKind::Rectangle, "Button", 5.0, 5.0, 20.0, 20.0),
        )
        .unwrap();
    let overflow = document
        .insert_node(
            root,
            Node::new(NodeKind::Rectangle, "Overflow", 75.0, 75.0, 30.0, 30.0),
        )
        .unwrap();
    let boolean = document
        .insert_node(
            root,
            Node::new(NodeKind::Boolean, "Union", 40.0, 5.0, 20.0, 20.0),
        )
        .unwrap();
    let operand = document
        .insert_node(
            boolean,
            Node::new(NodeKind::Rectangle, "Operand", 0.0, 0.0, 20.0, 20.0),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    let button_point = Point::new(120.0, 130.0);
    let overflow_point = Point::new(200.0, 210.0);
    let operand_point = Point::new(155.0, 130.0);
    // Normal editing still obeys the authored ancestor's clip.
    assert_eq!(editor.hit_test(button_point, 0.0), None);
    assert_eq!(
        editor.prototype_hit_test(root, button_point, 0.0),
        Some(button)
    );
    assert_eq!(editor.prototype_hit_test(root, overflow_point, 0.0), None);
    // A Boolean inside the presented frame owns its operand input, while a
    // directly presented operand is independent of the external Boolean.
    assert_eq!(
        editor.prototype_hit_test(root, operand_point, 0.0),
        Some(boolean)
    );
    assert_eq!(
        editor.prototype_hit_test(operand, operand_point, 0.0),
        Some(operand)
    );
    editor.document.nodes.get_mut(&root).unwrap().clip_content = false;
    editor.refresh();
    assert_eq!(
        editor.prototype_hit_test(root, overflow_point, 0.0),
        Some(overflow)
    );
    editor.document.nodes.get_mut(&root).unwrap().clip_content = true;
    let other_page = editor.document.add_page("Current screen");
    editor.active_page = other_page;
    editor.refresh();
    // The same isolated-root rules apply when an overlay lives on another page
    // and none of its transforms or bounds are in the active scene index.
    assert_eq!(
        editor.prototype_hit_test(root, button_point, 0.0),
        Some(button)
    );
    assert_eq!(editor.prototype_hit_test(root, overflow_point, 0.0), None);
    assert_eq!(
        editor.prototype_hit_test(root, operand_point, 0.0),
        Some(boolean)
    );
    assert_eq!(
        editor.prototype_hit_test(operand, operand_point, 0.0),
        Some(operand)
    );
}

#[test]
fn prototype_authoring_rejects_invalid_references_and_preserves_the_document() {
    let (mut document, a, b, _, button) = fixture();
    let variable = document.allocate_id();
    let alias = document.allocate_id();
    for (id, value) in [
        (variable, VariableValue::Number(1.0)),
        (alias, VariableValue::Alias(variable)),
    ] {
        document.variables.insert(
            id,
            Variable {
                id,
                name: format!("Variable {id}"),
                collection: "Prototype".into(),
                values: BTreeMap::from([("Default".into(), value)]),
            },
        );
    }
    let before = document.to_json().unwrap();
    for action in [
        PrototypeAction::NavigateTo { target: b },
        PrototypeAction::OpenOverlay {
            target: b,
            position: Point::new(-40.0, 25.0),
        },
        PrototypeAction::CloseOverlay,
        PrototypeAction::Back,
        url("https://example.com/design"),
        PrototypeAction::SetVariable {
            variable,
            value: VariableValue::Number(42.0),
        },
        PrototypeAction::ScrollTo { target: button },
    ] {
        let rule = interaction(Trigger::Click, action);
        assert!(validate_prototype_interaction(&document, a, &rule).is_ok());
    }
    let navigate = interaction(Trigger::Click, PrototypeAction::NavigateTo { target: b });
    assert!(validate_prototype_interaction(&document, u64::MAX, &navigate).is_err());
    for action in [
        PrototypeAction::NavigateTo { target: u64::MAX },
        PrototypeAction::OpenOverlay {
            target: b,
            position: Point::new(f64::INFINITY, 0.0),
        },
        PrototypeAction::ScrollTo { target: u64::MAX },
        PrototypeAction::SetVariable {
            variable: u64::MAX,
            value: VariableValue::Boolean(true),
        },
        PrototypeAction::SetVariable {
            variable,
            value: VariableValue::Number(f64::NAN),
        },
        PrototypeAction::SetVariable {
            variable,
            value: VariableValue::Alias(alias),
        },
        url("javascript:alert(1)"),
    ] {
        let rule = interaction(Trigger::Click, action);
        assert!(validate_prototype_interaction(&document, a, &rule).is_err());
    }
    let mut key = interaction(Trigger::Key, PrototypeAction::Back);
    assert!(validate_prototype_interaction(&document, a, &key).is_err());
    key.key = Some("Ctrl+Shift+A".into());
    assert!(validate_prototype_interaction(&document, a, &key).is_ok());
    key.key = Some("Ctrl+A+B".into());
    assert!(validate_prototype_interaction(&document, a, &key).is_err());
    assert_eq!(document.to_json().unwrap(), before);
    document.nodes.get_mut(&a).unwrap().locked = true;
    assert!(validate_prototype_interaction(&document, button, &navigate).is_err());
}

#[test]
fn prototype_authoring_rules_roundtrip_with_one_undo_step_per_transaction() {
    let (document, a, b, _, _) = fixture();
    let mut editor = Editor::new(document);
    let key_rule = Interaction {
        key: Some("ctrl+shift+a".into()),
        transition: Transition {
            kind: TransitionKind::SlideLeft,
            duration_ms: 1234,
            easing: Easing::Spring,
        },
        ..interaction(Trigger::Key, PrototypeAction::NavigateTo { target: b })
    };
    let timed_rule = delay(u32::MAX, PrototypeAction::Back);
    for rule in [&key_rule, &timed_rule] {
        validate_prototype_interaction(&editor.document, a, rule).unwrap();
        editor.transact("Add prototype interaction", |document| {
            document
                .nodes
                .get_mut(&a)
                .unwrap()
                .interactions
                .push(rule.clone());
        });
    }
    let restored = Document::from_json(&editor.document.to_json().unwrap()).unwrap();
    assert_eq!(
        restored.nodes[&a].interactions,
        [key_rule.clone(), timed_rule.clone()]
    );
    assert!(editor.undo());
    assert_eq!(editor.document.nodes[&a].interactions, [key_rule.clone()]);
    assert!(editor.undo());
    assert!(editor.document.nodes[&a].interactions.is_empty());
    assert!(!editor.undo());
    assert!(editor.redo());
    assert!(editor.redo());
    assert_eq!(
        editor.document.nodes[&a].interactions,
        [key_rule, timed_rule]
    );
}
