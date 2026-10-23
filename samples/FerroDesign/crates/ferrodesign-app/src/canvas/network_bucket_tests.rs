use super::*;
use ferroui_base::input::{Pointer, PointerType};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::{IPlatformRenderInterface, PixelFormat};
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
    scope
}

fn fixture() -> (Ref<DesignCanvas>, NodeId) {
    let mut network = core::VectorNetwork::default();
    let xs = [0.0, 100.0, 100.000_1, 200.0];
    let top: Vec<_> = xs.iter().map(|&x| network.add_vertex(Point::new(x, 0.0)).unwrap()).collect();
    let bottom: Vec<_> = xs.iter().map(|&x| network.add_vertex(Point::new(x, 120.0)).unwrap()).collect();
    for index in 0..xs.len() {
        network.connect(top[index], bottom[index]).unwrap();
        if index > 0 {
            network.connect(top[index - 1], top[index]).unwrap();
            network.connect(bottom[index - 1], bottom[index]).unwrap();
        }
    }
    let mut document = core::Document::new("Bucket gesture");
    let mut node = Node::new(NodeKind::Vector, "Editable faces", 20.0, 30.0, 200.0, 120.0);
    node.fills = vec![core::Fill::solid(Color::rgb(37, 99, 235))];
    node.vector_network = Some(network);
    let id = document.insert_node(1, node).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(editor)));
    canvas.vector_node.set(Some(id));
    canvas.vector_tool.set(VectorTool::Bucket);
    (canvas, id)
}

fn world(canvas: &DesignCanvas, id: NodeId, x: f64, y: f64) -> Point {
    canvas.editor.borrow().document.world_transform(id).unwrap().apply(Point::new(x, y))
}

fn json(canvas: &DesignCanvas) -> String { canvas.editor.borrow().document.to_json().unwrap() }

#[test]
fn bucket_hover_is_read_only_cached_and_uses_real_native_face_pixels() {
    let scope = backend();
    let (canvas, id) = fixture();
    let before = json(&canvas);
    let token = canvas.editor.borrow().render_change_token();
    for offset in 0..200 {
        assert!(canvas.network_bucket_move(world(&canvas, id, 20.0 + f64::from(offset) * 0.1, 60.0), false));
    }
    assert_eq!(canvas.network_bucket.borrow().preparations, 1);
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().render_change_token(), token);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(!canvas.editor.borrow().has_active_transaction());
    assert!(canvas.network_bucket_state().0.is_some());
    let bitmap = RenderTargetBitmap::new(PixelSize::new(260, 200));
    {
        let mut context = bitmap.create_drawing_context();
        let editor = canvas.editor.borrow();
        canvas.render_bucket_hover(&mut context, &editor, id, editor.document.world_transform(id).unwrap(),
            Viewport { width: 260.0, height: 200.0, zoom: 1.0, pan: Point::default() });
    }
    let mut pixels = vec![0; 260 * 200 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 260, 200), &mut pixels, 260 * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) { pixel.swap(0, 2); }
    }
    let pixel = |x: usize, y: usize| &pixels[(y * 260 + x) * 4..(y * 260 + x) * 4 + 4];
    assert!(pixel(70, 90)[3] > 20, "hover paints its exact native face");
    assert_eq!(pixel(170, 90)[3], 0, "hover cannot paint the neighbouring face");
    canvas.clear_bucket_hover();
    assert_eq!(canvas.network_bucket_state().0, None);
    scope.dispose();
}

#[test]
fn bucket_fast_affine_drag_paints_intermediate_faces_once_and_commits_one_history_step() {
    let scope = backend();
    let (canvas, id) = fixture();
    canvas.editor.borrow_mut().document.nodes.get_mut(&id).unwrap().set_local_transform(
        core::Transform { a: -1.2, b: 0.3, c: 0.4, d: 0.8, e: 250.0, f: 30.0 });
    canvas.editor.borrow_mut().refresh();
    let before = json(&canvas);
    canvas.network_bucket_press(world(&canvas, id, 30.0, 60.0), false);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(canvas.editor.borrow().has_active_transaction());
    canvas.network_bucket_move(world(&canvas, id, 180.0, 60.0), false);
    assert_eq!(canvas.network_bucket_state().2, 3, "the subpixel middle face is not skipped");
    assert_eq!(canvas.editor.borrow().document.nodes[&id].vector_network.as_ref().unwrap().regions.len(), 3);
    let token = canvas.editor.borrow().render_change_token();
    canvas.network_bucket_move(world(&canvas, id, 30.0, 60.0), true);
    assert_eq!(canvas.editor.borrow().render_change_token(), token, "visited faces are not repainted and Shift cannot change an active stroke's paint");
    assert_eq!(canvas.network_bucket.borrow().preparations, 1);
    canvas.finish_network_bucket_gesture(true);
    let after = json(&canvas);
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(json(&canvas), before);
    assert!(canvas.editor.borrow_mut().redo());
    assert_eq!(json(&canvas), after);
    canvas.network_bucket_press(world(&canvas, id, 30.0, 60.0), false);
    canvas.network_bucket_move(world(&canvas, id, 180.0, 60.0), false);
    canvas.finish_network_bucket_gesture(true);
    assert_eq!(canvas.editor.borrow().history_len(), 1, "repeated painting is a no-op");
    assert_eq!(json(&canvas), after);
    scope.dispose();
}

#[test]
fn bucket_clear_and_escape_capture_loss_restore_exact_graph_and_redo() {
    let scope = backend();
    let (canvas, id) = fixture();
    canvas.network_bucket_press(world(&canvas, id, 10.0, 60.0), false);
    canvas.network_bucket_move(world(&canvas, id, 180.0, 60.0), false);
    canvas.finish_network_bucket_gesture(true);
    let painted = json(&canvas);
    for cancel in 0..3 {
        canvas.network_bucket_press(world(&canvas, id, 10.0, 60.0), true);
        canvas.network_bucket_move(world(&canvas, id, 180.0, 60.0), true);
        assert!(canvas.editor.borrow().document.nodes[&id].vector_network.as_ref().unwrap().regions.values()
            .all(|region| region.fills.as_ref().is_some_and(Vec::is_empty)));
        match cancel {
            0 => {
                let mut event = KeyEventArgs::new(); event.key = Key::Escape;
                <DesignCanvas as InputElementImpl>::on_key_down(&canvas, &event);
                assert!(event.handled());
                assert_eq!(canvas.vector_node.get(), Some(id), "Escape cancels the stroke before leaving vector mode");
            }
            1 => {
                let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
                let event = PointerCaptureLostEventArgs::new(canvas.clone(), pointer);
                <DesignCanvas as InputElementImpl>::on_pointer_capture_lost(&canvas, &event);
            }
            _ => { canvas.cancel_active_edit(); }
        }
        assert_eq!(json(&canvas), painted);
        assert_eq!(canvas.editor.borrow().history_len(), 1);
        assert!(!canvas.editor.borrow().has_active_transaction());
    }
    canvas.network_bucket_press(world(&canvas, id, 10.0, 60.0), true);
    canvas.network_bucket_move(world(&canvas, id, 180.0, 60.0), true);
    canvas.finish_network_bucket_gesture(true);
    let cleared = json(&canvas);
    assert_eq!(canvas.editor.borrow().history_len(), 2);
    canvas.editor.borrow_mut().undo();
    assert_eq!(json(&canvas), painted);
    canvas.network_bucket_press(world(&canvas, id, 10.0, 60.0), false);
    canvas.finish_network_bucket_gesture(true);
    assert!(canvas.editor.borrow().can_redo(), "unchanged paint preserves redo");
    canvas.editor.borrow_mut().redo();
    assert_eq!(json(&canvas), cleared);
    scope.dispose();
}

#[test]
fn bucket_cancellation_preserves_new_selection_and_never_owns_replacement_transactions() {
    let scope = backend();
    let (canvas, id) = fixture();
    let before = json(&canvas);
    canvas.network_bucket_press(world(&canvas, id, 10.0, 60.0), false);
    canvas.editor.borrow_mut().selection.clear();
    canvas.reconcile_vector_edit();
    assert_eq!(json(&canvas), before);
    assert!(canvas.editor.borrow().selection.is_empty());
    assert_eq!(canvas.vector_node.get(), None);
    canvas.editor.borrow_mut().selection = vec![id];
    canvas.vector_node.set(Some(id));
    canvas.network_bucket_press(world(&canvas, id, 10.0, 60.0), false);
    canvas.editor.borrow_mut().cancel_transaction();
    canvas.editor.borrow_mut().begin_node_transaction(&[id], "Unrelated inspector edit");
    canvas.editor.borrow_mut().update_node(id, "Unrelated inspector edit", |node| node.name = "Keep unrelated draft".into());
    let token = canvas.editor.borrow().active_transaction_token();
    let external = json(&canvas);
    canvas.cancel_network_bucket_gesture();
    assert_eq!(json(&canvas), external);
    assert_eq!(canvas.editor.borrow().active_transaction_token(), token);
    scope.dispose();
}

#[test]
fn bucket_tool_switch_and_invalid_topology_keep_history_and_cached_failures_stable() {
    let scope = backend();
    let (canvas, id) = fixture();
    let before = json(&canvas);
    canvas.network_bucket_press(world(&canvas, id, 20.0, 60.0), false);
    canvas.vector_tool.set(VectorTool::Point);
    canvas.reconcile_vector_edit();
    assert_eq!(json(&canvas), before);
    assert!(!canvas.editor.borrow().has_active_transaction());
    canvas.vector_tool.set(VectorTool::Bucket);
    {
        let mut editor = canvas.editor.borrow_mut();
        let graph = editor.document.nodes.get_mut(&id).unwrap().vector_network.as_mut().unwrap();
        // A transverse crossing is now prepared automatically. A coincident
        // subsegment remains an ambiguous overlap and must fail atomically.
        let a = graph.add_vertex(Point::new(0.0, 20.0)).unwrap();
        let b = graph.add_vertex(Point::new(0.0, 100.0)).unwrap();
        graph.connect(a, b).unwrap();
        editor.refresh();
    }
    let before = json(&canvas);
    let notices = Rc::new(Cell::new(0));
    let received = notices.clone();
    canvas.on_change(move |event| { if matches!(event, CanvasEvent::Notice(_)) { received.set(received.get() + 1); } });
    canvas.network_bucket_press(world(&canvas, id, 20.0, 60.0), false);
    for _ in 0..100 { canvas.network_bucket_move(world(&canvas, id, 20.0, 60.0), false); }
    assert_eq!(notices.get(), 1, "an invalid press reports once, hover cannot flood notices");
    assert_eq!(canvas.network_bucket.borrow().preparations, 2);
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(!canvas.editor.borrow().has_active_transaction());
    scope.dispose();
}

fn crossed_fixture() -> (Ref<DesignCanvas>, NodeId) {
    let (canvas, id) = fixture();
    let mut network = core::VectorNetwork::default();
    let vertices = [Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(100.0, 100.0), Point::new(0.0, 100.0)]
        .map(|point| network.add_vertex(point).unwrap());
    for i in 0..4 { network.connect(vertices[i], vertices[(i + 1) % 4]).unwrap(); }
    network.connect_curve(vertices[0], vertices[2], core::NetworkCurve::Cubic {
        control1: Point::new(20.0, 60.0), control2: Point::new(80.0, 40.0),
    }).unwrap();
    network.connect_curve(vertices[1], vertices[3], core::NetworkCurve::Cubic {
        control1: Point::new(80.0, 60.0), control2: Point::new(20.0, 40.0),
    }).unwrap();
    canvas.editor.borrow_mut().document.nodes.get_mut(&id).unwrap().vector_network = Some(network);
    canvas.editor.borrow_mut().refresh();
    (canvas, id)
}

#[test]
fn bucket_crossing_hover_is_cached_and_preparation_rolls_back_with_the_paint_stroke() {
    let scope = backend();
    let (canvas, id) = crossed_fixture();
    let before = json(&canvas);
    for _ in 0..50 { canvas.network_bucket_move(world(&canvas, id, 50.0, 10.0), false); }
    assert_eq!(canvas.network_bucket.borrow().preparations, 1);
    assert_eq!(json(&canvas), before);
    assert!(canvas.network_bucket_state().0.is_some(), "unsplit crossings support immutable hover preview");
    canvas.network_bucket_press(world(&canvas, id, 50.0, 10.0), false);
    canvas.network_bucket_move(world(&canvas, id, 50.0, 90.0), false);
    {
        let editor = canvas.editor.borrow();
        let network = editor.document.nodes[&id].vector_network.as_ref().unwrap();
        assert_eq!(network.vertices.len(), 5);
        assert_eq!(network.edges.len(), 8);
        assert_eq!(network.regions.len(), 2);
        assert_eq!(editor.history_len(), 0);
    }
    assert_eq!(canvas.network_bucket.borrow().preparations, 1, "the applied topology retains its prepared map");
    canvas.cancel_network_bucket_gesture();
    assert_eq!(json(&canvas), before, "cancel restores geometry, paints and the original allocator");
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    canvas.network_bucket_press(world(&canvas, id, 50.0, 10.0), false);
    canvas.network_bucket_move(world(&canvas, id, 50.0, 90.0), false);
    canvas.finish_network_bucket_gesture(true);
    let after = json(&canvas);
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(json(&canvas), before);
    assert!(canvas.editor.borrow_mut().redo());
    assert_eq!(json(&canvas), after);
    scope.dispose();
}

#[test]
fn bucket_crossing_outside_press_is_a_noop_and_shift_clear_prepares_one_transparent_region() {
    let scope = backend();
    let (canvas, id) = crossed_fixture();
    let before = json(&canvas);
    canvas.network_bucket_press(world(&canvas, id, -10.0, -10.0), false);
    canvas.finish_network_bucket_gesture(true);
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    canvas.network_bucket_press(world(&canvas, id, 50.0, 10.0), true);
    canvas.finish_network_bucket_gesture(true);
    let editor = canvas.editor.borrow();
    let network = editor.document.nodes[&id].vector_network.as_ref().unwrap();
    assert_eq!(network.edges.len(), 8);
    assert_eq!(network.regions.len(), 1);
    assert_eq!(network.regions.values().next().unwrap().fills, Some(Vec::new()));
    assert_eq!(editor.history_len(), 1);
    drop(editor);
    scope.dispose();
}
