use super::*;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::FerroLocator;

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
    scope
}

fn layer(document: &mut core::Document, parent: NodeId, kind: NodeKind) -> NodeId {
    document
        .insert_node(
            parent,
            Node::new(kind, "Native selection layer", 0.0, 0.0, 100.0, 100.0)
                .with_fill(Color::WHITE),
        )
        .unwrap()
}

fn key(canvas: &DesignCanvas, key: Key, modifiers: KeyModifiers) {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.key_modifiers = modifiers;
    <DesignCanvas as InputElementImpl>::on_key_down(canvas, &event);
    assert!(event.handled());
}

#[test]
fn native_selection_keyboard_navigation_and_idle_escape_never_open_history() {
    let mut document = core::Document::new("Native hierarchy keys");
    let page = document.pages[0].id;
    let group = layer(&mut document, page, NodeKind::Group);
    let a = layer(&mut document, group, NodeKind::Rectangle);
    let b = layer(&mut document, group, NodeKind::Rectangle);
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(Editor::new(document))));
    canvas.editor.borrow_mut().selection = vec![group];
    let before = canvas.editor.borrow().document.to_json().unwrap();
    let token = canvas.editor.borrow().render_change_token();
    key(&canvas, Key::Enter, KeyModifiers::empty());
    assert_eq!(canvas.editor.borrow().selection, [b, a]);
    key(&canvas, Key::Tab, KeyModifiers::empty());
    assert_eq!(canvas.editor.borrow().selection, [b]);
    key(&canvas, Key::Tab, KeyModifiers::empty());
    assert_eq!(canvas.editor.borrow().selection, [a]);
    key(&canvas, Key::Tab, KeyModifiers::SHIFT);
    assert_eq!(canvas.editor.borrow().selection, [b]);
    key(&canvas, Key::Enter, KeyModifiers::SHIFT);
    assert_eq!(canvas.editor.borrow().selection, [group]);
    key(&canvas, Key::Escape, KeyModifiers::empty());
    assert!(canvas.editor.borrow().selection.is_empty());
    assert_eq!(canvas.editor.borrow().document.to_json().unwrap(), before);
    assert_eq!(canvas.editor.borrow().render_change_token(), token);
    assert!(!canvas.editor.borrow().has_active_transaction());
    assert_eq!(canvas.editor.borrow().history_len(), 0);
}

#[test]
fn native_selection_group_preference_keeps_control_and_meta_deep_and_context_fresh() {
    let scope = backend();
    let mut document = core::Document::new("Native group picking");
    let page = document.pages[0].id;
    let frame = layer(&mut document, page, NodeKind::Frame);
    let group = layer(&mut document, frame, NodeKind::Group);
    let leaf = layer(&mut document, group, NodeKind::Rectangle);
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(Editor::new(document))));
    let point = Point::new(50.0, 50.0);
    assert!(!canvas.select_groups.get());
    assert_eq!(canvas.pick(point), Some(leaf));
    assert!(canvas.selection_command("select-groups"));
    assert!(canvas.select_groups.get());
    assert_eq!(canvas.pick(point), Some(frame));
    canvas.editor.borrow_mut().selection = vec![frame];
    assert_eq!(
        canvas.pick_with_modifiers(point, KeyModifiers::empty(), true),
        Some(group)
    );
    canvas.editor.borrow_mut().selection = vec![group];
    assert_eq!(
        canvas.pick_with_modifiers(point, KeyModifiers::empty(), true),
        Some(leaf)
    );
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::META] {
        canvas.editor.borrow_mut().selection.clear();
        assert_eq!(
            canvas.pick_with_modifiers(point, modifiers, false),
            Some(leaf)
        );
    }
    canvas.editor.borrow_mut().selection = vec![leaf];
    assert_eq!(canvas.pick(point), Some(leaf));
    let mut replacement = Editor::new(canvas.editor.borrow().document.clone());
    replacement.selection = vec![frame];
    *canvas.editor.borrow_mut() = replacement;
    assert_eq!(canvas.pick(point), Some(frame));
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    scope.dispose();
}

#[test]
fn native_selection_picking_and_cycles_skip_masked_pixels_and_refresh_before_repaint() {
    let scope = backend();
    let mut document = core::Document::new("Native mask-aware selection");
    let page = document.pages[0].id;
    let back = layer(&mut document, page, NodeKind::Rectangle);
    let source = layer(&mut document, page, NodeKind::Rectangle);
    {
        let node = document.nodes.get_mut(&source).unwrap();
        node.width = 40.0;
        node.is_mask = true;
        node.mask_type = core::MaskType::Alpha;
        node.locked = true;
    }
    let target = layer(&mut document, page, NodeKind::Rectangle);
    let hidden = layer(&mut document, page, NodeKind::Rectangle);
    document.nodes.get_mut(&hidden).unwrap().visible = false;
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(Editor::new(document))));
    let inside = Point::new(20.0, 50.0);
    let outside = Point::new(75.0, 50.0);
    assert_eq!(canvas.pick(inside), Some(target));
    assert_eq!(canvas.pick(outside), Some(back));
    assert_eq!(
        canvas.selection_candidates_at(inside, false),
        [target, back]
    );
    assert_eq!(
        canvas.selection_candidates_at(inside, true),
        [target, source, back]
    );
    canvas.editor.borrow_mut().selection = vec![target];
    canvas.cycle_selection_at(inside, core::SelectionCycle::Behind);
    assert_eq!(canvas.editor.borrow().selection, [back]);
    canvas.cycle_selection_at(inside, core::SelectionCycle::Behind);
    assert_eq!(canvas.editor.borrow().selection, [target]);
    canvas
        .editor
        .borrow_mut()
        .update_node(source, "Move selection mask", |node| node.x = 50.0);
    assert_eq!(canvas.pick(inside), Some(back));
    assert_eq!(canvas.pick(outside), Some(target));
    canvas.editor.borrow_mut().undo();
    assert_eq!(canvas.pick(inside), Some(target));
    assert_eq!(canvas.pick(outside), Some(back));
    assert!(canvas.editor.borrow().can_redo());
    scope.dispose();
}

#[test]
fn native_selection_capture_loss_escape_and_commands_leave_foreign_history_owned() {
    use ferroui_base::input::{IPointer, Pointer as NativePointer, PointerType};
    let mut document = core::Document::new("Selection owns no transaction");
    let page = document.pages[0].id;
    let a = layer(&mut document, page, NodeKind::Rectangle);
    layer(&mut document, page, NodeKind::Rectangle);
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(Editor::new(document))));
    canvas.editor.borrow_mut().selection = vec![a];
    canvas
        .editor
        .borrow_mut()
        .begin_transaction("External inspector draft");
    canvas
        .editor
        .borrow_mut()
        .document
        .nodes
        .get_mut(&a)
        .unwrap()
        .name = "External draft".into();
    let token = canvas.editor.borrow().active_transaction_token();
    let before = canvas.editor.borrow().document.to_json().unwrap();
    let pointer = NativePointer::new(NativePointer::get_next_free_id(), PointerType::Mouse, true);
    pointer.capture(Some(&canvas.clone().upcast::<InputElement>()));
    pointer.platform_capture_lost();
    key(&canvas, Key::Escape, KeyModifiers::empty());
    key(&canvas, Key::Tab, KeyModifiers::empty());
    key(&canvas, Key::Enter, KeyModifiers::empty());
    assert!(canvas.selection_command("select-behind"));
    assert_eq!(canvas.editor.borrow().selection, [a]);
    assert_eq!(canvas.editor.borrow().document.to_json().unwrap(), before);
    assert_eq!(canvas.editor.borrow().active_transaction_token(), token);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
}

#[test]
fn native_selection_enter_text_uses_existing_native_editor_event() {
    let mut document = core::Document::new("Text selection Enter");
    let page = document.pages[0].id;
    let text = layer(&mut document, page, NodeKind::Text);
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(Editor::new(document))));
    canvas.editor.borrow_mut().selection = vec![text];
    let events = Rc::new(RefCell::new(Vec::new()));
    let captured = events.clone();
    canvas.on_change(move |event| {
        if let CanvasEvent::EditText(id) = event {
            captured.borrow_mut().push(id);
        }
    });
    key(&canvas, Key::Enter, KeyModifiers::empty());
    assert_eq!(*events.borrow(), [text]);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(!canvas.editor.borrow().has_active_transaction());
}

#[test]
fn native_selection_prototype_reuses_one_isolated_index_for_thousand_layer_pointer_queries() {
    let scope = backend();
    let mut document = core::Document::new("Retained presentation picking");
    let other_page = document.add_page("Presentation page");
    let external = layer(&mut document, other_page, NodeKind::Frame);
    {
        let node = document.nodes.get_mut(&external).unwrap();
        node.clip_content = true;
        node.affine_transform = Some(core::Transform {
            a: 1.0,
            b: 0.2,
            c: 0.3,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        });
    }
    let root = layer(&mut document, external, NodeKind::Frame);
    document.nodes.get_mut(&root).unwrap().x = 200.0;
    document.nodes.get_mut(&root).unwrap().width = 256_000.0;
    let mut target = 0;
    for index in 0..1_000 {
        let child = layer(&mut document, root, NodeKind::Rectangle);
        document.nodes.get_mut(&child).unwrap().x = index as f64 * 256.0;
        if index == 500 {
            target = child;
        }
    }
    document.nodes.get_mut(&target).unwrap().locked = true;
    let point = document
        .world_transform(target)
        .unwrap()
        .apply(Point::new(50.0, 50.0));
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(Editor::new(document))));
    canvas.presentation.set(Some(root));
    assert!(
        canvas.editor.borrow().scene.spatial.is_empty(),
        "The active authoring page has none of these layers"
    );
    for _ in 0..250 {
        assert_eq!(canvas.prototype_pick(point), Some(target));
    }
    let stats = canvas.prototype_selection_index_stats();
    assert_eq!(stats["builds"], 1);
    assert_eq!(stats["indexedNodes"], 1_001);
    assert_eq!(stats["root"], root);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    canvas
        .editor
        .borrow_mut()
        .update_node(target, "Move prototype hit before repaint", |node| {
            node.x += 150.0
        });
    assert_eq!(canvas.prototype_pick(point), Some(root));
    assert_eq!(canvas.prototype_selection_index_stats()["builds"], 2);
    canvas.editor.borrow_mut().undo();
    assert_eq!(canvas.prototype_pick(point), Some(target));
    assert_eq!(canvas.prototype_selection_index_stats()["builds"], 3);
    let revision = canvas.editor.borrow().revision;
    let mut replacement = Editor::new(canvas.editor.borrow().document.clone());
    replacement.revision = revision;
    replacement.document.revision = revision;
    *canvas.editor.borrow_mut() = replacement;
    assert_eq!(canvas.prototype_pick(point), Some(target));
    assert_eq!(canvas.prototype_selection_index_stats()["builds"], 4);
    canvas.presentation.set(Some(target));
    assert_eq!(canvas.prototype_pick(point), Some(target));
    assert_eq!(canvas.prototype_selection_index_stats()["builds"], 5);
    assert_eq!(canvas.prototype_selection_index_stats()["indexedNodes"], 1);
    scope.dispose();
}
