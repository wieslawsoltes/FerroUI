use super::*;
use crate::{Color, EditorRenderChanges, Effect, EffectKind, Fill, GradientStop, Point, TextRun};

fn fixture() -> (Editor, NodeId, NodeId, NodeId) {
    let mut document = Document::new("Local styles");
    let a = document
        .insert_node(
            1,
            Node::new(NodeKind::Rectangle, "Card", 20.0, 20.0, 80.0, 60.0),
        )
        .unwrap();
    let b = document
        .insert_node(
            1,
            Node::new(NodeKind::Rectangle, "Badge", 120.0, 20.0, 80.0, 60.0),
        )
        .unwrap();
    let text = document
        .insert_node(
            1,
            Node::new(NodeKind::Text, "Label", 20.0, 120.0, 200.0, 40.0),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![a, b];
    (editor, a, b, text)
}

fn fill(color: Color) -> StyleValue {
    StyleValue::Fill {
        fills: vec![Fill::solid(color)],
    }
}
fn red() -> StyleValue {
    fill(Color::rgb(239, 68, 68))
}
fn blue() -> StyleValue {
    fill(Color::rgb(37, 99, 235))
}
fn text_style() -> StyleValue {
    StyleValue::Text {
        family: "Inter".into(),
        size: 24.0,
        weight: 600,
        line_height: 1.4,
        letter_spacing: 0.2,
    }
}

#[test]
fn create_apply_propagate_delete_is_portable_and_each_action_has_exact_history() {
    let (mut editor, a, b, _) = fixture();
    let initial = editor.to_json().unwrap();
    let style = editor
        .create_local_style("  Brand/Accent  ", "Shared primary", red())
        .unwrap();
    assert_eq!(editor.document.styles[&style].name, "Brand/Accent");
    assert_eq!(editor.apply_local_style(&[a, b, a], style).unwrap(), 2);
    assert_eq!(editor.document.local_style_usage(style), 2);
    let linked = editor.to_json().unwrap();
    let changed = editor
        .update_local_style(style, "Brand/Action", "Updated", blue())
        .unwrap();
    assert_eq!(
        changed,
        LocalStyleUpdate {
            definition_changed: true,
            updated_nodes: 2,
            detached_overrides: 0
        }
    );
    assert_eq!(editor.document.nodes[&a].fill_style, Some(style));
    assert_eq!(
        editor.document.nodes[&b].fills,
        vec![Fill::solid(Color::rgb(37, 99, 235))]
    );
    let updated = editor.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), linked);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), updated);
    assert_eq!(editor.remove_local_style(style).unwrap(), 2);
    let detached = editor.to_json().unwrap();
    assert_eq!(editor.document.nodes[&a].fill_style, None);
    assert_eq!(
        editor.document.nodes[&a].fills,
        editor.document.nodes[&b].fills
    );
    assert!(!editor.document.styles.contains_key(&style));
    // Node revisions are runtime invalidation counters, deliberately excluded
    // from the portable format; every durable field must round-trip exactly.
    assert_eq!(
        Document::from_json(&detached).unwrap().to_json().unwrap(),
        detached
    );
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), updated);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), detached);
    for _ in 0..4 {
        assert!(editor.undo());
    }
    assert_eq!(editor.to_json().unwrap(), initial);
}

#[test]
fn create_from_nodes_preserves_selection_complete_stacks_and_one_undo() {
    let (mut editor, a, b, _) = fixture();
    let paints = vec![
        Fill {
            paint: Paint::LinearGradient {
                start: Point::new(-0.5, 0.0),
                end: Point::new(1.5, 1.0),
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: Color::rgb(9, 20, 40),
                    },
                    GradientStop {
                        offset: 0.3,
                        color: Color::rgba(30, 60, 90, 128),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: Color::rgb(100, 150, 200),
                    },
                ],
            },
            visible: true,
            opacity: 0.7,
        },
        Fill::solid(Color::rgba(255, 255, 255, 30)),
    ];
    editor.document.nodes.get_mut(&a).unwrap().fills = paints.clone();
    editor.refresh();
    let before = editor.to_json().unwrap();
    let style = editor
        .create_local_style_for_nodes(&[a, b, a], LocalStyleKind::Fill, "Surface/Gradient", "")
        .unwrap();
    assert_eq!(editor.document.nodes[&a].fills, paints);
    assert_eq!(editor.document.nodes[&b].fills, paints);
    assert_eq!(
        editor.document.styles[&style].value,
        StyleValue::Fill { fills: paints }
    );
    assert_eq!(editor.selection, vec![a, b]);
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    assert!(!editor.undo());
}

#[test]
fn missing_targets_reject_create_apply_and_detach_without_metadata_or_history_changes() {
    let (mut editor, a, b, _) = fixture();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    editor.apply_local_style(&[a], style).unwrap();
    editor.move_selection(5.0, 0.0);
    editor.undo();
    let before = editor.to_json().unwrap();
    let token = editor.render_change_token();
    assert!(editor
        .create_local_style_for_nodes(&[a, 9999], LocalStyleKind::Fill, "Invalid", "")
        .is_err());
    assert!(editor.apply_local_style(&[b, 9999], style).is_err());
    assert!(editor
        .detach_local_style(&[a, 9999], LocalStyleKind::Fill)
        .is_err());
    assert_eq!(editor.to_json().unwrap(), before);
    assert_eq!(editor.render_change_token(), token);
    assert!(editor.redo());
}

#[test]
fn no_op_and_invalid_style_edits_keep_redo_revision_and_change_feed() {
    let (mut editor, a, _, _) = fixture();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    editor.apply_local_style(&[a], style).unwrap();
    editor.move_selection(3.0, 0.0);
    editor.undo();
    let before = editor.to_json().unwrap();
    let token = editor.render_change_token();
    assert_eq!(editor.apply_local_style(&[a, a], style).unwrap(), 0);
    assert_eq!(
        editor
            .update_local_style(style, "Accent", "", red())
            .unwrap(),
        LocalStyleUpdate::default()
    );
    assert_eq!(
        editor
            .detach_local_style(&[a], LocalStyleKind::Text)
            .unwrap(),
        0
    );
    assert!(editor.update_local_style(style, "", "", blue()).is_err());
    assert!(editor
        .update_local_style(style, "Accent", "", text_style())
        .is_err());
    assert!(editor
        .update_local_style(9999, "Accent", "", blue())
        .is_err());
    assert!(editor.remove_local_style(9999).is_err());
    assert_eq!(editor.to_json().unwrap(), before);
    assert_eq!(editor.render_change_token(), token);
    assert!(editor.redo());
}

#[test]
fn direct_apply_and_detach_respect_locked_ancestry_but_shared_definition_updates_propagate() {
    let (mut editor, a, b, _) = fixture();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    editor.apply_local_style(&[a, b], style).unwrap();
    editor.document.nodes.get_mut(&b).unwrap().locked = true;
    let frame = editor
        .document
        .insert_node(
            1,
            Node::new(NodeKind::Frame, "Locked frame", 0.0, 0.0, 300.0, 200.0),
        )
        .unwrap();
    editor.document.reparent(a, frame, 0).unwrap();
    editor.document.nodes.get_mut(&frame).unwrap().locked = true;
    editor.refresh();
    assert_eq!(
        editor
            .detach_local_style(&[a, b], LocalStyleKind::Fill)
            .unwrap(),
        0
    );
    let other = editor.create_local_style("Other", "", blue()).unwrap();
    assert_eq!(editor.apply_local_style(&[a, b], other).unwrap(), 0);
    assert!(editor
        .create_local_style_for_nodes(&[a, b], LocalStyleKind::Fill, "Blocked", "")
        .is_err());
    let result = editor
        .update_local_style(style, "Accent", "", blue())
        .unwrap();
    assert_eq!(result.updated_nodes, 2);
    assert_eq!(editor.document.nodes[&a].fill_style, Some(style));
    assert_eq!(editor.document.nodes[&b].fill_style, Some(style));
    assert!(blue().matches_node(&editor.document.nodes[&a]));
    assert!(blue().matches_node(&editor.document.nodes[&b]));
}

#[test]
fn text_styles_skip_shapes_preserve_unicode_runs_and_retain_one_history_transaction() {
    let (mut editor, a, _, text) = fixture();
    let node = editor.document.nodes.get_mut(&text).unwrap();
    node.text = "café Ω".into();
    node.text_runs.push(TextRun {
        start: 0,
        end: 5,
        font_family: None,
        font_size: Some(30.0),
        font_weight: None,
        color: Some(Color::rgb(255, 0, 0)),
        italic: None,
        decoration: None,
    });
    editor.refresh();
    let style = editor
        .create_local_style("Typography/Title", "", text_style())
        .unwrap();
    let before = editor.to_json().unwrap();
    let shape = editor.document.nodes[&a].clone();
    let runs = editor.document.nodes[&text].text_runs.clone();
    assert_eq!(editor.apply_local_style(&[a, text], style).unwrap(), 1);
    assert_eq!(editor.document.nodes[&a], shape);
    assert_eq!(editor.document.nodes[&text].text_runs, runs);
    assert_eq!(editor.document.nodes[&text].text_style, Some(style));
    let after = editor.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), after);
}

#[test]
fn effects_apply_and_propagate_all_entries_without_detaching_the_definition() {
    let (mut editor, a, b, _) = fixture();
    let effects = vec![
        Effect::for_kind(EffectKind::InnerShadow),
        Effect::for_kind(EffectKind::BackgroundBlur),
    ];
    let style = editor
        .create_local_style(
            "Elevation",
            "",
            StyleValue::Effect {
                effects: effects.clone(),
            },
        )
        .unwrap();
    assert_eq!(editor.apply_local_style(&[a, b], style).unwrap(), 2);
    let mut edited = effects.clone();
    edited[0].spread = -8.0;
    edited[1].radius = 12.0;
    assert_eq!(
        editor
            .update_local_style(
                style,
                "Elevation",
                "",
                StyleValue::Effect {
                    effects: edited.clone()
                }
            )
            .unwrap()
            .updated_nodes,
        2
    );
    assert_eq!(editor.document.nodes[&a].effects, edited);
    assert_eq!(editor.document.nodes[&b].effect_style, Some(style));
    let before = editor.to_json().unwrap();
    editor
        .detach_local_style(&[a], LocalStyleKind::Effect)
        .unwrap();
    assert_eq!(editor.document.nodes[&a].effects, edited);
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
}

#[test]
fn manual_property_edits_detach_only_changed_style_kinds_and_undo_relinks() {
    let (mut editor, a, _, text) = fixture();
    let fill = editor.create_local_style("Accent", "", red()).unwrap();
    let effects = editor
        .create_local_style(
            "Shadow",
            "",
            StyleValue::Effect {
                effects: vec![Effect::for_kind(EffectKind::DropShadow)],
            },
        )
        .unwrap();
    let typography = editor.create_local_style("Body", "", text_style()).unwrap();
    editor.apply_local_style(&[a, text], fill).unwrap();
    editor.apply_local_style(&[a], effects).unwrap();
    editor.apply_local_style(&[text], typography).unwrap();
    let before = editor.to_json().unwrap();
    editor.update_node(a, "Move", |node| node.x += 7.0);
    assert_eq!(editor.document.nodes[&a].fill_style, Some(fill));
    editor.set_fill(a, Color::rgb(10, 20, 30));
    assert_eq!(editor.document.nodes[&a].fill_style, None);
    assert_eq!(editor.document.nodes[&a].effect_style, Some(effects));
    editor.update_nodes_with_layout(&[text], "Resize text", |node| node.font_size = 42.0);
    assert_eq!(editor.document.nodes[&text].text_style, None);
    assert_eq!(editor.document.nodes[&text].fill_style, Some(fill));
    for _ in 0..3 {
        assert!(editor.undo());
    }
    assert_eq!(editor.to_json().unwrap(), before);
}

#[test]
fn legacy_customized_bindings_are_detached_without_overwriting_custom_values() {
    let (mut editor, a, b, _) = fixture();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    editor.apply_local_style(&[a, b], style).unwrap();
    let custom = vec![Fill::solid(Color::rgb(11, 22, 33))];
    editor.document.nodes.get_mut(&b).unwrap().fills = custom.clone();
    editor.refresh();
    let before = editor.to_json().unwrap();
    let result = editor
        .update_local_style(style, "Accent", "", blue())
        .unwrap();
    assert_eq!(result.updated_nodes, 1);
    assert_eq!(result.detached_overrides, 1);
    assert_eq!(editor.document.nodes[&b].fills, custom);
    assert_eq!(editor.document.nodes[&b].fill_style, None);
    assert_eq!(editor.document.nodes[&a].fill_style, Some(style));
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
}

#[test]
fn renaming_is_metadata_only_even_for_legacy_customized_users() {
    let (mut editor, a, _, _) = fixture();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    editor.apply_local_style(&[a], style).unwrap();
    editor.document.nodes.get_mut(&a).unwrap().fills = vec![Fill::solid(Color::rgb(1, 2, 3))];
    editor.refresh();
    let node = editor.document.nodes[&a].clone();
    let result = editor
        .update_local_style(style, "Renamed", "Description Ω", red())
        .unwrap();
    assert!(result.definition_changed);
    assert_eq!(result.updated_nodes, 0);
    assert_eq!(result.detached_overrides, 0);
    assert_eq!(editor.document.nodes[&a], node);
}

#[test]
fn update_from_a_matching_new_source_keeps_its_existing_binding() {
    let (mut editor, a, b, _) = fixture();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    editor.apply_local_style(&[a, b], style).unwrap();
    editor.document.nodes.get_mut(&a).unwrap().fills = vec![Fill::solid(Color::rgb(37, 99, 235))];
    editor.refresh();
    let result = editor
        .update_local_style(style, "Accent", "", blue())
        .unwrap();
    assert_eq!(result.updated_nodes, 2);
    assert_eq!(result.detached_overrides, 0);
    assert_eq!(editor.document.nodes[&a].fill_style, Some(style));
}

#[test]
fn invalid_style_payloads_reject_atomically_including_hidden_gradient_and_effect_fields() {
    let (mut editor, _, _, _) = fixture();
    let mut invalid_opacity = Fill::solid(Color::rgb(1, 2, 3));
    invalid_opacity.opacity = f64::NAN;
    let mut effect = Effect::for_kind(EffectKind::LayerBlur);
    effect.offset.x = f64::INFINITY;
    let invalid_values = vec![
        StyleValue::Fill {
            fills: vec![invalid_opacity],
        },
        StyleValue::Fill {
            fills: vec![Fill {
                paint: Paint::LinearGradient {
                    start: Point::default(),
                    end: Point::new(1.0, 1.0),
                    stops: vec![
                        GradientStop {
                            offset: 0.8,
                            color: Color::TRANSPARENT,
                        },
                        GradientStop {
                            offset: 0.2,
                            color: Color::TRANSPARENT,
                        },
                    ],
                },
                opacity: 1.0,
                visible: false,
            }],
        },
        StyleValue::Fill {
            fills: vec![Fill {
                paint: Paint::RadialGradient {
                    center: Point::default(),
                    radius: -1.0,
                    stops: vec![GradientStop {
                        offset: 0.0,
                        color: Color::TRANSPARENT,
                    }],
                },
                opacity: 1.0,
                visible: true,
            }],
        },
        StyleValue::Text {
            family: "".into(),
            size: 20.0,
            weight: 400,
            line_height: 0.0,
            letter_spacing: 0.0,
        },
        StyleValue::Text {
            family: "Inter".into(),
            size: 20.0,
            weight: 1001,
            line_height: 0.0,
            letter_spacing: 0.0,
        },
        StyleValue::Text {
            family: "Inter".into(),
            size: 20.0,
            weight: 400,
            line_height: -1.0,
            letter_spacing: 0.0,
        },
        StyleValue::Effect {
            effects: vec![effect],
        },
        StyleValue::Grid {
            size: 0.0,
            color: Color::TRANSPARENT,
        },
    ];
    let before = editor.to_json().unwrap();
    let token = editor.render_change_token();
    for value in invalid_values {
        assert!(editor.create_local_style("Invalid", "", value).is_err());
    }
    assert_eq!(editor.to_json().unwrap(), before);
    assert_eq!(editor.render_change_token(), token);
    assert!(!editor.undo());
}

#[test]
fn imported_invalid_styles_cannot_bypass_application_validation() {
    let (mut editor, a, _, _) = fixture();
    editor.document.styles.insert(
        100,
        Style {
            id: 100,
            name: "Legacy".into(),
            description: String::new(),
            value: StyleValue::Text {
                family: "Inter".into(),
                size: -10.0,
                weight: 400,
                line_height: 1.0,
                letter_spacing: 0.0,
            },
        },
    );
    let before = editor.to_json().unwrap();
    let token = editor.render_change_token();
    assert!(editor.apply_local_style(&[a], 100).is_err());
    assert_eq!(editor.to_json().unwrap(), before);
    assert_eq!(editor.render_change_token(), token);
}

#[test]
fn apply_style_changes_only_bound_nodes_and_retains_incremental_rendering() {
    let (mut editor, a, _, _) = fixture();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    let token = editor.render_change_token();
    editor.apply_local_style(&[a], style).unwrap();
    assert_eq!(
        editor.render_changes_since(&token),
        EditorRenderChanges::Nodes {
            nodes: vec![a],
            ancestors: vec![]
        }
    );
}

#[test]
fn source_and_instance_style_links_survive_component_reconciliation_and_explicit_override() {
    let (mut editor, a, _, _) = fixture();
    editor.selection = vec![a];
    let component = editor.create_component().unwrap();
    let source = editor.document.nodes[&component].children[0];
    let original = editor.create_local_style("Original", "", red()).unwrap();
    editor.apply_local_style(&[source], original).unwrap();
    let instance = editor.create_instance(component, 400.0, 10.0).unwrap();
    let materialized = editor.document.nodes[&instance].children[0];
    let override_style = editor.create_local_style("Override", "", blue()).unwrap();
    editor
        .apply_local_style(&[materialized], override_style)
        .unwrap();
    assert!(editor.document.nodes[&materialized]
        .overridden_properties
        .contains("fills"));
    editor.refresh_component_instances(component).unwrap();
    assert_eq!(
        editor.document.nodes[&materialized].fill_style,
        Some(override_style)
    );
    assert!(blue().matches_node(&editor.document.nodes[&materialized]));
    editor
        .update_local_style(override_style, "Override", "", red())
        .unwrap();
    editor.refresh_component_instances(component).unwrap();
    assert_eq!(
        editor.document.nodes[&materialized].fill_style,
        Some(override_style)
    );
    assert!(red().matches_node(&editor.document.nodes[&materialized]));
}

#[test]
fn explicit_apply_removes_competing_variable_binding_and_undo_restores_it() {
    let (mut editor, a, _, text) = fixture();
    editor
        .document
        .nodes
        .get_mut(&a)
        .unwrap()
        .variable_bindings
        .insert("fill".into(), 900);
    editor
        .document
        .nodes
        .get_mut(&text)
        .unwrap()
        .variable_bindings
        .insert("font_size".into(), 901);
    editor.refresh();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    let before = editor.to_json().unwrap();
    editor.apply_local_style(&[a], style).unwrap();
    assert!(!editor.document.nodes[&a]
        .variable_bindings
        .contains_key("fill"));
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    let style = editor.create_local_style("Text", "", text_style()).unwrap();
    editor.apply_local_style(&[text], style).unwrap();
    assert!(!editor.document.nodes[&text]
        .variable_bindings
        .contains_key("font_size"));
}

#[test]
fn definitions_propagate_across_pages_and_delete_cleans_all_legacy_binding_slots() {
    let (mut editor, a, b, _) = fixture();
    let page = editor.add_page("Second");
    editor.document.reparent(b, page, 0).unwrap();
    editor.refresh();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    editor.apply_local_style(&[a, b], style).unwrap();
    editor.document.nodes.get_mut(&b).unwrap().text_style = Some(style);
    editor.refresh();
    assert_eq!(editor.document.local_style_usage(style), 2);
    editor
        .update_local_style(style, "Accent", "", blue())
        .unwrap();
    assert!(blue().matches_node(&editor.document.nodes[&b]));
    editor.remove_local_style(style).unwrap();
    assert_eq!(editor.document.local_style_usage(style), 0);
    assert_eq!(editor.document.nodes[&b].text_style, None);
}

#[test]
fn grid_definitions_remain_editable_and_skip_incompatible_layers() {
    let (mut editor, a, _, _) = fixture();
    let style = editor
        .create_local_style(
            "Grid",
            "",
            StyleValue::Grid {
                size: 8.0,
                color: Color::rgb(1, 2, 3),
            },
        )
        .unwrap();
    let before = editor.to_json().unwrap();
    assert_eq!(editor.apply_local_style(&[a], style).unwrap(), 0);
    assert_eq!(editor.to_json().unwrap(), before);
    editor
        .update_local_style(
            style,
            "Grid",
            "Updated",
            StyleValue::Grid {
                size: 16.0,
                color: Color::rgb(10, 20, 30),
            },
        )
        .unwrap();
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
}

#[test]
fn exhausted_style_id_space_returns_error_without_panicking_or_writing() {
    let (mut editor, a, _, _) = fixture();
    editor.document.next_id = u64::MAX;
    let before = editor.to_json().unwrap();
    assert!(editor.create_local_style("Accent", "", red()).is_err());
    assert!(editor
        .create_local_style_for_nodes(&[a], LocalStyleKind::Fill, "Accent", "")
        .is_err());
    assert_eq!(editor.to_json().unwrap(), before);
}

#[test]
fn imported_mismatched_style_ids_cannot_create_dangling_bindings() {
    let (mut editor, a, _, _) = fixture();
    editor.document.styles.insert(
        100,
        Style {
            id: 101,
            name: "Mismatched".into(),
            description: String::new(),
            value: red(),
        },
    );
    let before = editor.to_json().unwrap();
    assert!(editor.apply_local_style(&[a], 100).is_err());
    assert_eq!(editor.to_json().unwrap(), before);
    assert_eq!(editor.document.nodes[&a].fill_style, None);
}

#[test]
fn geometry_and_content_edits_keep_large_paint_text_and_effect_bindings() {
    let (mut editor, _, _, text) = fixture();
    let paints = StyleValue::Fill {
        fills: vec![Fill {
            paint: Paint::Image {
                href: format!("data:image/png;base64,{}", "A".repeat(64 * 1024)),
                scale: crate::ImageScale::Fill,
                crop: None,
            },
            ..Fill::solid(Color::rgb(0, 0, 0))
        }],
    };
    let fill_style = editor.create_local_style("Image", "", paints).unwrap();
    let typography = editor
        .create_local_style("Label", "", text_style())
        .unwrap();
    let effects = editor
        .create_local_style(
            "Shadow",
            "",
            StyleValue::Effect {
                effects: vec![Effect::shadow(
                    Color::rgba(0, 0, 0, 64),
                    Point::new(1.0, 2.0),
                    4.0,
                )],
            },
        )
        .unwrap();
    for style in [fill_style, typography, effects] {
        editor.apply_local_style(&[text], style).unwrap();
    }
    let before = editor.to_json().unwrap();
    editor.update_node(text, "Move styled text", |node| {
        node.x += 50.0;
        node.rotation = 30.0;
        node.text = "New café Ω text".into();
    });
    editor.update_nodes(&[text], "Rename styled text", |node| {
        node.name = "Renamed".into()
    });
    editor.update_nodes_with_layout(&[text], "Resize styled text", |node| node.width += 80.0);
    let node = &editor.document.nodes[&text];
    assert_eq!(node.fill_style, Some(fill_style));
    assert_eq!(node.text_style, Some(typography));
    assert_eq!(node.effect_style, Some(effects));
    assert!(editor.undo());
    assert!(editor.undo());
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    editor.update_nodes_with_layout(&[text], "Font size", |node| node.font_size += 2.0);
    assert_eq!(editor.document.nodes[&text].text_style, None);
    assert_eq!(editor.document.nodes[&text].fill_style, Some(fill_style));
    assert_eq!(editor.document.nodes[&text].effect_style, Some(effects));
}

#[test]
fn unrefreshed_legacy_overrides_are_compared_to_current_values() {
    let (mut editor, a, b, _) = fixture();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    editor.apply_local_style(&[a, b], style).unwrap();
    for id in [a, b] {
        editor.document.nodes.get_mut(&id).unwrap().fills = vec![Fill::solid(Color::rgb(1, 2, 3))];
    }
    // Hosts may batch direct document edits before their next refresh. A stale
    // render snapshot must not detach the binding during an unrelated edit.
    editor.update_node(a, "Move legacy override", |node| node.x += 1.0);
    editor.update_nodes(&[b], "Move other legacy override", |node| node.y += 1.0);
    assert_eq!(editor.document.nodes[&a].fill_style, Some(style));
    assert_eq!(editor.document.nodes[&b].fill_style, Some(style));
    let before = editor.to_json().unwrap();
    editor.update_nodes(&[a, b], "Edit legacy paints", |node| {
        node.fills[0].opacity = 0.5
    });
    assert_eq!(editor.document.nodes[&a].fill_style, None);
    assert_eq!(editor.document.nodes[&b].fill_style, None);
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
}

#[test]
fn pending_style_edits_cancel_and_commit_with_exact_bindings() {
    let (mut editor, a, b, _) = fixture();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    editor.apply_local_style(&[a, b], style).unwrap();
    let before = editor.to_json().unwrap();
    editor.begin_node_transaction(&[a, b], "Style gesture");
    editor.update_nodes(&[a, b], "Move preview", |node| node.x += 2.0);
    assert_eq!(editor.document.nodes[&a].fill_style, Some(style));
    editor.update_node(a, "Paint preview", |node| node.fills[0].opacity = 0.5);
    assert_eq!(editor.document.nodes[&a].fill_style, None);
    editor.cancel_transaction();
    assert_eq!(editor.to_json().unwrap(), before);

    editor.begin_node_transaction(&[a, b], "Style gesture");
    editor.update_nodes(&[a, b], "Move preview", |node| node.x += 2.0);
    editor.update_node(a, "Paint preview", |node| node.fills[0].opacity = 0.5);
    assert!(editor.commit_transaction());
    let after = editor.to_json().unwrap();
    assert_eq!(editor.document.nodes[&b].fill_style, Some(style));
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), after);
}

fn variable(editor: &mut Editor, name: &str, value: crate::VariableValue) -> crate::VariableId {
    editor
        .edit_variables(crate::VariableEdit::Upsert {
            id: None,
            name: name.into(),
            collection: "Style ownership".into(),
            mode: "Default".into(),
            value,
            replace_all_modes: false,
        })
        .unwrap()
        .variable
        .unwrap()
}

#[test]
fn same_value_variable_binding_takes_style_ownership_in_one_undoable_action() {
    use crate::{VariableEdit, VariableValue};
    let (mut editor, a, b, _) = fixture();
    let style = editor.create_local_style("Accent", "", red()).unwrap();
    editor.apply_local_style(&[a, b], style).unwrap();
    let color = variable(
        &mut editor,
        "Accent variable",
        VariableValue::Color(Color::rgb(239, 68, 68)),
    );
    let before = editor.to_json().unwrap();
    let depth = editor.history_len();
    editor
        .edit_variables(VariableEdit::Bind {
            nodes: vec![a],
            property: "fill".into(),
            variable: Some(color),
        })
        .unwrap();
    let after = editor.to_json().unwrap();
    assert_eq!(editor.history_len(), depth + 1);
    assert_eq!(editor.document.nodes[&a].fill_style, None);
    assert_eq!(editor.document.nodes[&a].variable_bindings["fill"], color);
    assert!(red().matches_node(&editor.document.nodes[&a]));
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), after);

    editor
        .update_local_style(style, "Accent", "", blue())
        .unwrap();
    assert!(red().matches_node(&editor.document.nodes[&a]));
    assert!(blue().matches_node(&editor.document.nodes[&b]));
    editor
        .edit_variables(VariableEdit::AddMode {
            name: "Dark".into(),
            copy_from: "Default".into(),
        })
        .unwrap();
    editor
        .edit_variables(VariableEdit::Upsert {
            id: Some(color),
            name: "Accent variable".into(),
            collection: "Style ownership".into(),
            mode: "Dark".into(),
            value: VariableValue::Color(Color::BLACK),
            replace_all_modes: false,
        })
        .unwrap();
    assert_eq!(
        editor.document.nodes[&a].fills,
        vec![Fill::solid(Color::BLACK)]
    );
    assert!(blue().matches_node(&editor.document.nodes[&b]));
}

#[test]
fn text_size_variables_detach_typography_only_and_invalid_binding_is_atomic() {
    use crate::{VariableEdit, VariableValue};
    let (mut editor, _, _, text) = fixture();
    let fill = editor.create_local_style("Accent", "", red()).unwrap();
    let typography = editor.create_local_style("Body", "", text_style()).unwrap();
    editor.apply_local_style(&[text], fill).unwrap();
    editor.apply_local_style(&[text], typography).unwrap();
    let size = variable(&mut editor, "Size", VariableValue::Number(24.0));
    let content = variable(
        &mut editor,
        "Content",
        VariableValue::String("café Ω".into()),
    );
    editor
        .edit_variables(VariableEdit::Bind {
            nodes: vec![text],
            property: "text".into(),
            variable: Some(content),
        })
        .unwrap();
    assert_eq!(editor.document.nodes[&text].text_style, Some(typography));
    let before = editor.to_json().unwrap();
    let depth = editor.history_len();
    assert!(editor
        .edit_variables(VariableEdit::Bind {
            nodes: vec![text],
            property: "font_size".into(),
            variable: Some(content)
        })
        .is_err());
    assert_eq!(editor.history_len(), depth);
    assert_eq!(editor.to_json().unwrap(), before);
    editor
        .edit_variables(VariableEdit::Bind {
            nodes: vec![text],
            property: "font_size".into(),
            variable: Some(size),
        })
        .unwrap();
    assert_eq!(editor.document.nodes[&text].text_style, None);
    assert_eq!(editor.document.nodes[&text].fill_style, Some(fill));
    assert_eq!(editor.document.nodes[&text].text, "café Ω");
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
}

#[test]
fn style_and_variable_ownership_survives_component_reconciliation_both_directions() {
    use crate::{VariableEdit, VariableValue};
    let (mut editor, a, _, _) = fixture();
    editor.selection = vec![a];
    let component = editor.create_component().unwrap();
    let source = editor.document.nodes[&component].children[0];
    let color = variable(&mut editor, "Token", VariableValue::Color(Color::BLACK));
    editor
        .edit_variables(VariableEdit::Bind {
            nodes: vec![source],
            property: "fill".into(),
            variable: Some(color),
        })
        .unwrap();
    let instance = editor.create_instance(component, 400.0, 10.0).unwrap();
    let child = editor.document.nodes[&instance].children[0];
    let style = editor.create_local_style("Override", "", red()).unwrap();
    editor.apply_local_style(&[child], style).unwrap();
    editor.refresh_component_instances(component).unwrap();
    assert_eq!(editor.document.nodes[&child].fill_style, Some(style));
    assert!(!editor.document.nodes[&child]
        .variable_bindings
        .contains_key("fill"));
    editor.document.apply_variables();
    assert!(red().matches_node(&editor.document.nodes[&child]));

    editor
        .edit_variables(VariableEdit::Bind {
            nodes: vec![child],
            property: "fill".into(),
            variable: Some(color),
        })
        .unwrap();
    editor.apply_local_style(&[source], style).unwrap();
    editor.refresh_component_instances(component).unwrap();
    assert_eq!(editor.document.nodes[&child].fill_style, None);
    assert_eq!(
        editor.document.nodes[&child].variable_bindings["fill"],
        color
    );
    assert_eq!(
        editor.document.nodes[&child].fills,
        vec![Fill::solid(Color::BLACK)]
    );
}


fn grid_fixture() -> (Editor, NodeId, NodeId, NodeId) {
    let (mut editor, a, b, text) = fixture();
    editor.document.nodes.get_mut(&a).unwrap().kind = NodeKind::Frame;
    editor.document.nodes.get_mut(&b).unwrap().kind = NodeKind::Frame;
    editor.refresh();
    (editor, a, b, text)
}

fn square_style(size: f64) -> StyleValue {
    StyleValue::Grid { size, color: Color::rgba(240, 30, 90, 80) }
}

#[test]
fn grid_create_apply_propagate_detach_delete_preserves_effective_values_and_history() {
    let (mut editor, a, b, text) = grid_fixture();
    let original = editor.to_json().unwrap();
    let style = editor.create_local_style_for_nodes(&[a, text, b, a], LocalStyleKind::Grid, "Spacing/8", "").unwrap();
    assert_eq!(editor.document.local_style_usage(style), 2);
    assert!(editor.document.nodes[&text].layout_grid.is_none());
    assert_eq!(editor.document.nodes[&a].layout_grid, Some(LayoutGrid::default()));
    let created = editor.to_json().unwrap();
    editor.update_local_style(style, "Spacing/16", "Shared guides", square_style(16.0)).unwrap();
    assert_eq!(editor.document.nodes[&a].layout_grid.unwrap().size, 16.0);
    assert_eq!(editor.document.nodes[&b].grid_style, Some(style));
    assert_eq!(editor.detach_local_style(&[a], LocalStyleKind::Grid).unwrap(), 1);
    assert_eq!(editor.document.nodes[&a].grid_style, None);
    assert_eq!(editor.document.nodes[&a].layout_grid, editor.document.nodes[&b].layout_grid);
    let detached = editor.to_json().unwrap();
    assert_eq!(editor.remove_local_style(style).unwrap(), 1);
    assert_eq!(editor.document.nodes[&b].grid_style, None);
    assert_eq!(editor.document.nodes[&b].layout_grid.unwrap().size, 16.0);
    let removed = editor.to_json().unwrap();
    assert_eq!(Document::from_json(&removed).unwrap().to_json().unwrap(), removed);
    assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), detached);
    assert!(editor.redo()); assert_eq!(editor.to_json().unwrap(), removed);
    for _ in 0..3 { assert!(editor.undo()); }
    assert_eq!(editor.to_json().unwrap(), created);
    assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), original);
}

#[test]
fn grid_definition_updates_locked_cross_page_users_but_preserves_visibility_and_custom_values() {
    let (mut editor, a, b, _) = grid_fixture();
    let second = editor.add_page("Other page");
    editor.document.reparent(b, second, 0).unwrap();
    editor.refresh();
    let style = editor.create_local_style("Grid", "", square_style(8.0)).unwrap();
    assert_eq!(editor.apply_local_style(&[a, b], style).unwrap(), 2);
    editor.edit_layout_grids(&[b], LayoutGridEdit::Visible(false)).unwrap();
    assert_eq!(editor.document.nodes[&b].grid_style, Some(style));
    editor.document.nodes.get_mut(&b).unwrap().locked = true;
    editor.refresh();
    let before = editor.to_json().unwrap();
    let updated = editor.update_local_style(style, "Grid", "", square_style(24.0)).unwrap();
    assert_eq!(updated.updated_nodes, 2);
    assert_eq!(editor.document.nodes[&b].layout_grid.unwrap().size, 24.0);
    assert!(!editor.document.nodes[&b].layout_grid.unwrap().visible);
    assert_eq!(editor.detach_local_style(&[b], LocalStyleKind::Grid).unwrap(), 0);
    assert_eq!(editor.edit_layout_grids(&[b], LayoutGridEdit::Remove).unwrap(), 0);
    assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
    editor.document.nodes.get_mut(&b).unwrap().layout_grid.as_mut().unwrap().size = 31.0;
    editor.refresh();
    let updated = editor.update_local_style(style, "Grid", "", square_style(48.0)).unwrap();
    assert_eq!(updated.updated_nodes, 1);
    assert_eq!(updated.detached_overrides, 1);
    assert_eq!(editor.document.nodes[&b].grid_style, None);
    assert_eq!(editor.document.nodes[&b].layout_grid.unwrap().size, 31.0);
}

#[test]
fn grid_direct_edits_detach_only_grid_binding_and_visibility_keeps_style_ownership() {
    let (mut editor, a, _, _) = grid_fixture();
    let grid = editor.create_local_style("Grid", "", square_style(8.0)).unwrap();
    let fill = editor.create_local_style("Fill", "", red()).unwrap();
    editor.apply_local_style(&[a], grid).unwrap();
    editor.apply_local_style(&[a], fill).unwrap();
    editor.update_node(a, "Move", |node| node.x += 10.0);
    assert_eq!(editor.document.nodes[&a].grid_style, Some(grid));
    editor.edit_layout_grids(&[a], LayoutGridEdit::Visible(false)).unwrap();
    assert_eq!(editor.document.nodes[&a].grid_style, Some(grid));
    let hidden = editor.to_json().unwrap();
    editor.edit_layout_grids(&[a], LayoutGridEdit::Size(12.0)).unwrap();
    assert_eq!(editor.document.nodes[&a].grid_style, None);
    assert_eq!(editor.document.nodes[&a].fill_style, Some(fill));
    assert!(!editor.document.nodes[&a].layout_grid.unwrap().visible);
    assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), hidden);
    editor.update_nodes_with_layout(&[a], "Local grid color", |node| node.layout_grid.as_mut().unwrap().color = Color::BLACK);
    assert_eq!(editor.document.nodes[&a].grid_style, None);
    assert_eq!(editor.document.nodes[&a].fill_style, Some(fill));
}

#[test]
fn grid_missing_invalid_and_unchanged_operations_preserve_redo_and_change_feed() {
    let (mut editor, a, b, text) = grid_fixture();
    editor.edit_layout_grids(&[a], LayoutGridEdit::Add).unwrap();
    editor.update_node(a, "Move", |node| node.x += 5.0);
    editor.undo();
    let before = editor.to_json().unwrap();
    let token = editor.render_change_token();
    for edit in [LayoutGridEdit::Size(0.0), LayoutGridEdit::Size(-1.0), LayoutGridEdit::Size(f64::INFINITY), LayoutGridEdit::Size(f64::NAN)] {
        assert!(editor.edit_layout_grids(&[a, b], edit).is_err());
    }
    assert!(editor.edit_layout_grids(&[a, 9999], LayoutGridEdit::Remove).is_err());
    assert_eq!(editor.edit_layout_grids(&[a], LayoutGridEdit::Size(8.0)).unwrap(), 0);
    assert_eq!(editor.edit_layout_grids(&[a], LayoutGridEdit::Add).unwrap(), 0);
    assert_eq!(editor.edit_layout_grids(&[b], LayoutGridEdit::Visible(false)).unwrap(), 0);
    assert_eq!(editor.edit_layout_grids(&[text], LayoutGridEdit::Add).unwrap(), 0);
    assert_eq!(editor.to_json().unwrap(), before);
    assert_eq!(editor.render_change_token(), token);
    assert!(editor.redo());
}

#[test]
fn grid_edits_respect_ancestor_locks_and_remove_is_one_scoped_transaction() {
    let (mut editor, a, b, _) = grid_fixture();
    let child = editor.document.insert_node(a, Node::new(NodeKind::Frame, "Nested", 0.0, 0.0, 20.0, 20.0)).unwrap();
    editor.document.nodes.get_mut(&a).unwrap().locked = true;
    editor.refresh();
    assert_eq!(editor.edit_layout_grids(&[a, b, child, b], LayoutGridEdit::Add).unwrap(), 1);
    assert!(editor.document.nodes[&a].layout_grid.is_none());
    assert!(editor.document.nodes[&child].layout_grid.is_none());
    let before = editor.to_json().unwrap();
    let token = editor.render_change_token();
    assert_eq!(editor.edit_layout_grids(&[b], LayoutGridEdit::Remove).unwrap(), 1);
    assert_eq!(editor.render_changes_since(&token), EditorRenderChanges::Nodes { nodes: vec![b], ancestors: vec![] });
    assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
}

#[test]
fn grid_json_is_backward_compatible_and_rejects_invalid_layer_grids() {
    let (mut editor, a, _, text) = grid_fixture();
    let original = editor.to_json().unwrap();
    assert!(!original.contains("layout_grid"));
    assert!(!original.contains("grid_style"));
    assert_eq!(Document::from_json(&original).unwrap().to_json().unwrap(), original);
    editor.document.nodes.get_mut(&a).unwrap().layout_grid = Some(LayoutGrid::default());
    assert!(editor.document.validate().is_ok());
    editor.document.nodes.get_mut(&a).unwrap().layout_grid.as_mut().unwrap().size = 0.0;
    assert!(editor.document.validate().is_err());
    editor.document.nodes.get_mut(&a).unwrap().layout_grid = None;
    editor.document.nodes.get_mut(&text).unwrap().layout_grid = Some(LayoutGrid::default());
    assert!(editor.document.validate().is_err());
}

#[test]
fn grid_component_reconciliation_preserves_local_visibility_and_explicit_style_ownership() {
    let (mut editor, a, _, _) = grid_fixture();
    editor.selection = vec![a];
    let component = editor.create_component().unwrap();
    let inherited = editor.create_local_style("Source grid", "", square_style(8.0)).unwrap();
    editor.apply_local_style(&[component], inherited).unwrap();
    let instance = editor.create_instance(component, 400.0, 10.0).unwrap();
    editor.edit_layout_grids(&[instance], LayoutGridEdit::Visible(false)).unwrap();
    editor.update_local_style(inherited, "Source grid", "", square_style(16.0)).unwrap();
    editor.refresh_component_instances(component).unwrap();
    assert_eq!(editor.document.nodes[&instance].grid_style, Some(inherited));
    let grid = editor.document.nodes[&instance].layout_grid.unwrap();
    assert_eq!(grid.size, 16.0);
    assert!(!grid.visible);
    let owned = editor.create_local_style("Own grid", "", square_style(32.0)).unwrap();
    editor.apply_local_style(&[instance], owned).unwrap();
    editor.refresh_component_instances(component).unwrap();
    assert_eq!(editor.document.nodes[&instance].grid_style, Some(owned));
    assert_eq!(editor.document.nodes[&instance].layout_grid.unwrap().size, 32.0);
    assert!(!editor.document.nodes[&instance].layout_grid.unwrap().visible);
    editor.edit_layout_grids(&[instance], LayoutGridEdit::Remove).unwrap();
    editor.refresh_component_instances(component).unwrap();
    assert!(editor.document.nodes[&instance].layout_grid.is_none());
    assert!(editor.document.nodes[&instance].grid_style.is_none());
}
