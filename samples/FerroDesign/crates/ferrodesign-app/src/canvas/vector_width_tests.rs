use super::*;
use ferroui_base::input::{Pointer, PointerType};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::{IPlatformRenderInterface, PixelFormat};
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
    scope
}

fn fixture() -> (Ref<DesignCanvas>, NodeId) {
    let mut document = core::Document::new("Width handle gestures");
    let mut node = Node::new(NodeKind::Line, "Width line", 20.0, 30.0, 100.0, 0.0);
    node.strokes = vec![core::Stroke::new(Color::BLACK, 20.0)];
    let id = document.insert_node(1, node).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(editor)));
    canvas.viewport.set(Viewport {
        width: 400.0,
        height: 260.0,
        zoom: 1.0,
        pan: Point::default(),
    });
    assert!(canvas.edit_stroke_width());
    (canvas, id)
}

fn world(canvas: &DesignCanvas, id: NodeId, x: f64, y: f64) -> Point {
    canvas
        .editor
        .borrow()
        .document
        .world_transform(id)
        .unwrap()
        .apply(Point::new(x, y))
}
fn json(canvas: &DesignCanvas) -> String {
    canvas.editor.borrow().document.to_json().unwrap()
}
fn points(canvas: &DesignCanvas, id: NodeId) -> Vec<StrokeWidthPoint> {
    canvas.editor.borrow().document.nodes[&id].strokes[0]
        .width_profile
        .control_points()
        .to_vec()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn width_hover_is_read_only_retained_and_native_handles_render_real_pixels() {
    let scope = backend();
    let (canvas, id) = fixture();
    let before = json(&canvas);
    let token = canvas.editor.borrow().render_change_token();
    for index in 0..100 {
        assert!(canvas.stroke_width_move(world(&canvas, id, 40.0 + index as f64 * 0.1, 0.0)));
    }
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().render_change_token(), token);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(!canvas.editor.borrow().has_active_transaction());
    assert_eq!(canvas.stroke_width.borrow().preparations, 1);
    let bitmap = RenderTargetBitmap::new(PixelSize::new(200, 100));
    {
        let mut context = bitmap.create_drawing_context();
        let editor = canvas.editor.borrow();
        assert!(canvas.render_stroke_width(
            &mut context,
            &editor,
            id,
            editor.document.world_transform(id).unwrap(),
            canvas.viewport.get()
        ));
    }
    let mut pixels = vec![0; 200 * 100 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 200, 100), &mut pixels, 200 * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
    let alpha = |x: usize, y: usize| pixels[(y * 200 + x) * 4 + 3];
    assert!(
        alpha(20, 30) > 20,
        "center square uses native DrawingContext"
    );
    assert!(
        alpha(20, 42) > 20,
        "thin widths retain an accessible normal handle"
    );
    assert_eq!(alpha(160, 70), 0);
    canvas.clear_stroke_width_hover();
    assert!(!canvas.stroke_width_state()["hover"].as_bool().unwrap());
    scope.dispose();
}

#[test]
fn insertion_and_many_normal_drag_updates_are_one_exact_history_transaction() {
    let scope = backend();
    let (canvas, id) = fixture();
    let before = json(&canvas);
    assert!(canvas.stroke_width_press(world(&canvas, id, 50.0, 0.0)));
    assert!(canvas.stroke_width_active());
    assert!(canvas.editor.borrow().has_active_transaction());
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    for offset in 1..=15 {
        canvas.stroke_width_move(world(&canvas, id, 50.0, f64::from(offset)));
    }
    assert_eq!(points(&canvas, id).len(), 3);
    close(points(&canvas, id)[1].position, 0.5);
    close(points(&canvas, id)[1].width, 2.5);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(canvas.finish_stroke_width_edit());
    let after = json(&canvas);
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(json(&canvas), before);
    assert!(canvas.editor.borrow_mut().redo());
    assert_eq!(json(&canvas), after);
    assert!(!canvas.finish_stroke_width_edit());
    canvas.stroke_width_move(world(&canvas, id, 50.0, 80.0));
    assert_eq!(
        json(&canvas),
        after,
        "late motion after lifecycle completion is hover only"
    );
    scope.dispose();
}

#[test]
fn center_drag_uses_arc_length_and_clamps_between_neighbors_without_reordering() {
    let scope = backend();
    let (canvas, id) = fixture();
    canvas.stroke_width_press(world(&canvas, id, 50.0, 0.0));
    canvas.finish_stroke_width_edit();
    let before = json(&canvas);
    let depth = canvas.editor.borrow().history_len();
    canvas.stroke_width_press(world(&canvas, id, 50.0, 0.0));
    canvas.stroke_width_move(world(&canvas, id, 75.0, 2.0));
    close(points(&canvas, id)[1].position, 0.75);
    close(points(&canvas, id)[1].width, 1.0);
    canvas.stroke_width_move(world(&canvas, id, 150.0, 20.0));
    close(points(&canvas, id)[1].position, 1.0 - POSITION_GAP);
    canvas.finish_stroke_width_edit();
    let after = json(&canvas);
    assert_eq!(canvas.editor.borrow().history_len(), depth + 1);
    canvas.editor.borrow().document.validate().unwrap();
    canvas.editor.borrow_mut().undo();
    assert_eq!(json(&canvas), before);
    canvas.editor.borrow_mut().redo();
    assert_eq!(json(&canvas), after);
    scope.dispose();
}

#[test]
fn reflected_sheared_multi_contours_repeat_one_profile_without_changing_source_paths() {
    let scope = backend();
    let (canvas, id) = fixture();
    {
        let mut editor = canvas.editor.borrow_mut();
        let node = editor.document.nodes.get_mut(&id).unwrap();
        node.kind = NodeKind::Vector;
        node.width = 200.0;
        node.height = 100.0;
        node.path = VectorPath {
            segments: vec![
                PathSegment::MoveTo {
                    point: Point::new(0.0, 0.0),
                },
                PathSegment::LineTo {
                    point: Point::new(100.0, 0.0),
                },
                PathSegment::MoveTo {
                    point: Point::new(0.0, 100.0),
                },
                PathSegment::LineTo {
                    point: Point::new(200.0, 100.0),
                },
            ],
            ..VectorPath::default()
        };
        node.strokes[0].width_profile = StrokeWidthProfile::Custom {
            points: vec![
                StrokeWidthPoint::new(0.0, 1.0),
                StrokeWidthPoint::new(0.5, 2.0),
                StrokeWidthPoint::new(1.0, 1.0),
            ],
        };
        node.set_local_transform(core::Transform {
            a: -1.2,
            b: 0.3,
            c: 0.4,
            d: 0.9,
            e: 270.0,
            f: 30.0,
        });
        editor.refresh();
    }
    let before = json(&canvas);
    let path = canvas.editor.borrow().document.nodes[&id].path.clone();
    canvas.stroke_width_press(world(&canvas, id, 100.0, 120.0));
    assert_eq!(canvas.stroke_width_state()["contour"], 1);
    canvas.stroke_width_move(world(&canvas, id, 100.0, 130.0));
    close(points(&canvas, id)[1].width, 3.0);
    canvas.finish_stroke_width_edit();
    canvas.stroke_width_press(world(&canvas, id, 100.0, 100.0));
    canvas.stroke_width_move(world(&canvas, id, 150.0, 100.0));
    canvas.finish_stroke_width_edit();
    close(points(&canvas, id)[1].position, 0.75);
    let editor = canvas.editor.borrow();
    let node = &editor.document.nodes[&id];
    assert_eq!(node.path, path);
    assert_eq!(editor.history_len(), 2);
    let contours = core::stroke_width_contours(node, &node.strokes[0]).unwrap();
    close(contours[0].sample(0.75).unwrap().point.x, 75.0);
    close(contours[1].sample(0.75).unwrap().point.x, 150.0);
    drop(editor);
    canvas.editor.borrow_mut().undo();
    canvas.editor.borrow_mut().undo();
    assert_eq!(json(&canvas), before);
    scope.dispose();
}

#[test]
fn escape_capture_loss_and_undo_boundary_restore_profile_and_keep_redo() {
    let scope = backend();
    let (canvas, id) = fixture();
    canvas
        .editor
        .borrow_mut()
        .set_stroke_width_profile(&[id], 0, StrokeWidthProfile::Eye)
        .unwrap();
    canvas.editor.borrow_mut().undo();
    let before = json(&canvas);
    for cancel in 0..3 {
        canvas.stroke_width_press(world(&canvas, id, 50.0, 0.0));
        canvas.stroke_width_move(world(&canvas, id, 50.0, 20.0));
        assert_ne!(json(&canvas), before);
        match cancel {
            0 => {
                let mut event = KeyEventArgs::new();
                event.key = Key::Escape;
                <DesignCanvas as InputElementImpl>::on_key_down(&canvas, &event);
                assert!(event.handled());
            }
            1 => {
                let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
                let event = PointerCaptureLostEventArgs::new(canvas.clone(), pointer);
                <DesignCanvas as InputElementImpl>::on_pointer_capture_lost(&canvas, &event);
            }
            _ => {
                assert!(canvas.cancel_active_edit());
            }
        }
        assert_eq!(json(&canvas), before);
        assert_eq!(canvas.stroke_width_state()["node"], id);
        assert!(!canvas.editor.borrow().has_active_transaction());
        assert!(canvas.editor.borrow().can_redo());
    }
    assert!(canvas.stroke_width_key(Key::Escape, KeyModifiers::empty()));
    assert_eq!(canvas.stroke_width_state()["node"], serde_json::Value::Null);
    scope.dispose();
}

#[test]
fn width_gesture_cannot_cancel_replacement_transaction_and_preserves_new_selection() {
    let scope = backend();
    let (canvas, id) = fixture();
    let before = json(&canvas);
    canvas.stroke_width_press(world(&canvas, id, 50.0, 0.0));
    canvas.editor.borrow_mut().selection.clear();
    canvas.reconcile_vector_edit();
    assert_eq!(json(&canvas), before);
    assert!(canvas.editor.borrow().selection.is_empty());
    assert_eq!(canvas.stroke_width_state()["node"], serde_json::Value::Null);
    canvas.editor.borrow_mut().selection = vec![id];
    assert!(canvas.edit_stroke_width());
    canvas.stroke_width_press(world(&canvas, id, 50.0, 0.0));
    canvas.editor.borrow_mut().cancel_transaction();
    canvas
        .editor
        .borrow_mut()
        .begin_node_transaction(&[id], "Unrelated draft");
    canvas
        .editor
        .borrow_mut()
        .update_node(id, "Unrelated draft", |node| {
            node.name = "Keep unrelated edit".into()
        });
    let token = canvas.editor.borrow().active_transaction_token();
    let external = json(&canvas);
    canvas.cancel_stroke_width_gesture();
    assert_eq!(json(&canvas), external);
    assert_eq!(canvas.editor.borrow().active_transaction_token(), token);
    canvas.editor.borrow_mut().cancel_transaction();
    scope.dispose();
}

#[test]
fn selecting_existing_point_is_not_an_edit_and_deleting_is_one_undoable_command() {
    let scope = backend();
    let (canvas, id) = fixture();
    canvas
        .editor
        .borrow_mut()
        .set_stroke_width_profile(&[id], 0, StrokeWidthProfile::Eye)
        .unwrap();
    canvas.editor.borrow_mut().undo();
    let before = json(&canvas);
    canvas.stroke_width_press(world(&canvas, id, 0.0, 0.0));
    canvas.finish_stroke_width_edit();
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(canvas.editor.borrow().can_redo());
    let mut event = KeyEventArgs::new();
    event.key = Key::Delete;
    <DesignCanvas as InputElementImpl>::on_key_down(&canvas, &event);
    assert!(event.handled());
    assert_eq!(points(&canvas, id), vec![StrokeWidthPoint::new(1.0, 1.0)]);
    let after = json(&canvas);
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    canvas.editor.borrow_mut().undo();
    assert_eq!(json(&canvas), before);
    canvas.editor.borrow_mut().redo();
    assert_eq!(json(&canvas), after);
    canvas.stroke_width_press(world(&canvas, id, 100.0, 0.0));
    canvas.finish_stroke_width_edit();
    canvas.stroke_width_key(Key::Delete, KeyModifiers::empty());
    assert!(canvas.editor.borrow().document.nodes[&id].strokes[0]
        .width_profile
        .is_uniform());
    assert_eq!(
        canvas.editor.borrow().document.nodes.len(),
        1,
        "Delete never removes the artwork layer in width mode"
    );
    scope.dispose();
}

#[test]
fn zero_width_handle_is_accessible_and_negative_or_nonfinite_drags_never_corrupt_data() {
    let scope = backend();
    let (canvas, id) = fixture();
    canvas
        .editor
        .borrow_mut()
        .set_stroke_width_profile(&[id], 0, StrokeWidthProfile::Eye)
        .unwrap();
    let depth = canvas.editor.borrow().history_len();
    canvas.stroke_width_press(world(&canvas, id, 0.0, 12.0));
    canvas.stroke_width_move(world(&canvas, id, 0.0, 22.0));
    close(points(&canvas, id)[0].width, 1.0);
    canvas.stroke_width_move(world(&canvas, id, 0.0, -20.0));
    close(points(&canvas, id)[0].width, 0.0);
    canvas.stroke_width_move(Point::new(f64::INFINITY, f64::NAN));
    close(points(&canvas, id)[0].width, 0.0);
    canvas.finish_stroke_width_edit();
    assert_eq!(
        canvas.editor.borrow().history_len(),
        depth,
        "a gesture returning to its original profile adds no history"
    );
    canvas.editor.borrow().document.validate().unwrap();
    scope.dispose();
}

#[test]
fn arrow_conversion_is_part_of_owned_gesture_and_unsupported_or_locked_entry_is_atomic() {
    let scope = backend();
    let (canvas, id) = fixture();
    canvas.editor.borrow_mut().update_node(id, "Arrow", |node| {
        node.strokes[0].cap = core::StrokeCap::Arrow
    });
    let before = json(&canvas);
    canvas.stroke_width_press(world(&canvas, id, 50.0, 0.0));
    assert_eq!(
        canvas.editor.borrow().document.nodes[&id].strokes[0].cap,
        core::StrokeCap::Butt
    );
    canvas.cancel_stroke_width_gesture();
    assert_eq!(json(&canvas), before);
    canvas.leave_stroke_width_edit();
    canvas
        .editor
        .borrow_mut()
        .update_node(id, "Dashed", |node| node.strokes[0].dash = vec![4.0, 2.0]);
    let before = json(&canvas);
    let depth = canvas.editor.borrow().history_len();
    assert!(!canvas.edit_stroke_width());
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().history_len(), depth);
    assert!(!canvas.editor.borrow().has_active_transaction());
    canvas
        .editor
        .borrow_mut()
        .update_node(id, "Locked", |node| {
            node.strokes[0].dash.clear();
            node.locked = true;
        });
    let before = json(&canvas);
    assert!(!canvas.edit_stroke_width());
    assert_eq!(json(&canvas), before);
    scope.dispose();
}

#[test]
fn tab_selects_overlapping_closed_contour_endpoints_without_mutating_the_profile() {
    let scope = backend();
    let (canvas, id) = fixture();
    {
        let mut editor = canvas.editor.borrow_mut();
        let node = editor.document.nodes.get_mut(&id).unwrap();
        node.kind = NodeKind::Vector;
        node.height = 100.0;
        node.path = VectorPath::polygon(
            &[
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                Point::new(100.0, 100.0),
                Point::new(0.0, 100.0),
            ],
            true,
        );
        editor.refresh();
    }
    let before = json(&canvas);
    assert!(canvas.stroke_width_key(Key::Tab, KeyModifiers::SHIFT));
    assert_eq!(canvas.stroke_width_state()["point"], 1);
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    canvas.stroke_width_press(world(&canvas, id, 0.0, 0.0));
    assert_eq!(
        canvas.stroke_width_state()["point"],
        1,
        "selected endpoint wins an exact geometric tie"
    );
    canvas.stroke_width_move(world(&canvas, id, 20.0, 0.0));
    close(points(&canvas, id)[1].position, 0.05);
    canvas.finish_stroke_width_edit();
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    canvas.editor.borrow_mut().undo();
    assert_eq!(json(&canvas), before);
    scope.dispose();
}

#[test]
fn width_canvas_resource_limits_reject_entry_without_simplifying_the_source() {
    let scope = backend();
    let (canvas, id) = fixture();
    canvas.leave_stroke_width_edit();
    {
        let mut editor = canvas.editor.borrow_mut();
        let node = editor.document.nodes.get_mut(&id).unwrap();
        node.kind = NodeKind::Vector;
        node.height = 400.0;
        node.path.segments.clear();
        for contour in 0..=MAX_EDIT_CONTOURS {
            node.path.segments.push(PathSegment::MoveTo {
                point: Point::new(0.0, contour as f64 * 5.0),
            });
            node.path.segments.push(PathSegment::LineTo {
                point: Point::new(100.0, contour as f64 * 5.0),
            });
        }
        editor.refresh();
    }
    let before = json(&canvas);
    let depth = canvas.editor.borrow().history_len();
    assert!(!canvas.edit_stroke_width());
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().history_len(), depth);
    assert!(!canvas.editor.borrow().has_active_transaction());
    {
        let mut editor = canvas.editor.borrow_mut();
        let node = editor.document.nodes.get_mut(&id).unwrap();
        node.path.segments.truncate(18);
        node.strokes[0].width_profile = StrokeWidthProfile::Custom {
            points: (0..256)
                .map(|index| StrokeWidthPoint::new(f64::from(index) / 255.0, 1.0))
                .collect(),
        };
        editor.refresh();
    }
    let before = json(&canvas);
    assert!(!canvas.edit_stroke_width());
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().history_len(), depth);
    scope.dispose();
}
