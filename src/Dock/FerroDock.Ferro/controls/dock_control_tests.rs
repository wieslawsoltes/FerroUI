//! Original main-controller static selection, gesture and drag-action scenarios.
use super::*;
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock, SplitViewDock, Tool, ToolDock},
    core::DockWindow,
};
use ferroui_base::input::Key;
#[test]
fn selector_gesture_accepts_only_required_modifiers_and_optional_extra_shift() {
    let gesture = KeyGesture::new(Key::Tab, KeyModifiers::CONTROL);
    let mut event = KeyEventArgs::new();
    event.key = Key::Tab;
    for (modifiers, expected) in [
        (KeyModifiers::NONE, None),
        (KeyModifiers::SHIFT, None),
        (KeyModifiers::CONTROL, Some(false)),
        (KeyModifiers::CONTROL | KeyModifiers::SHIFT, Some(true)),
        (KeyModifiers::CONTROL | KeyModifiers::ALT, None),
        (KeyModifiers::CONTROL | KeyModifiers::META, None),
        (
            KeyModifiers::from_bits_retain(KeyModifiers::CONTROL.bits() | 0x8000),
            None,
        ),
    ] {
        event.key_modifiers = modifiers;
        assert_eq!(matches_selector_gesture(&event, &gesture), expected);
    }
    event.key = Key::Enter;
    event.key_modifiers = KeyModifiers::CONTROL;
    assert_eq!(matches_selector_gesture(&event, &gesture), None);
    event.key = Key::Tab;
    event.key_modifiers = KeyModifiers::CONTROL | KeyModifiers::SHIFT;
    let explicit_shift = KeyGesture::new(Key::Tab, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
    assert_eq!(
        matches_selector_gesture(&event, &explicit_shift),
        Some(false)
    );
    event.key_modifiers = KeyModifiers::SHIFT;
    assert_eq!(
        matches_selector_gesture(&event, &KeyGesture::from_key(Key::Tab)),
        Some(true)
    );
}
#[test]
fn pointer_drag_action_keeps_original_alt_shift_control_precedence() {
    for (modifiers, expected) in [
        (KeyModifiers::NONE, DragAction::MOVE),
        (KeyModifiers::CONTROL, DragAction::COPY),
        (KeyModifiers::SHIFT, DragAction::MOVE),
        (KeyModifiers::ALT, DragAction::LINK),
        (
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            DragAction::MOVE,
        ),
        (KeyModifiers::CONTROL | KeyModifiers::ALT, DragAction::LINK),
        (KeyModifiers::SHIFT | KeyModifiers::ALT, DragAction::LINK),
        (
            KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT,
            DragAction::LINK,
        ),
        (KeyModifiers::META, DragAction::MOVE),
        (KeyModifiers::from_bits_retain(0x8000), DragAction::MOVE),
    ] {
        assert_eq!(to_drag_action(modifiers), expected);
    }
}
#[test]
fn selector_title_order_matches_dotnet_for_bmp_supplementary_null_and_case_boundaries() {
    fn decode(hex: &str) -> String {
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        String::from_utf8(bytes).unwrap()
    }
    let mut count = 0;
    for line in include_str!("../../../../tests/Dock/data/selector-title-order.tsv")
        .lines()
        .skip(1)
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 3);
        let left = decode(fields[0]);
        let right = decode(fields[1]);
        let expected = match fields[2] {
            "-1" => Ordering::Less,
            "0" => Ordering::Equal,
            "1" => Ordering::Greater,
            _ => panic!("reference comparison sign"),
        };
        assert_eq!(
            compare_selector_titles(&left, &right),
            expected,
            "{left:?} versus {right:?}"
        );
        assert_eq!(compare_selector_titles(&right, &left), expected.reverse());
        count += 1;
    }
    assert_eq!(count, 21);
}
#[test]
fn selector_traversal_keeps_original_scope_order_model_identity_and_cycle_deduplication() {
    fn document(title: &str) -> Rc<dyn IDockable> {
        let doc = Document::new();
        doc.set_title(title.into());
        doc
    }
    let root = RootDock::new();
    let hidden = document("same");
    let shared = document("same");
    let left: Rc<dyn IDockable> = Tool::new();
    let right = document("right");
    let top = document("top");
    let bottom = document("bottom");
    root.set_hidden_dockables(Some(ListHandle::from_items(vec![
        Some(hidden.clone()),
        Some(shared.clone()),
    ])));
    root.set_left_pinned_dockables(Some(ListHandle::from_items(vec![
        Some(left.clone()),
        Some(shared.clone()),
    ])));
    root.set_right_pinned_dockables(Some(ListHandle::from_items(vec![Some(right.clone())])));
    root.set_top_pinned_dockables(Some(ListHandle::from_items(vec![Some(top.clone())])));
    root.set_bottom_pinned_dockables(Some(ListHandle::from_items(vec![Some(bottom.clone())])));
    let floating = document("floating");
    let floating_root = RootDock::new();
    floating_root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(floating.clone())])));
    let window = DockWindow::new();
    window.set_layout(Some(floating_root));
    root.set_windows(Some(ListHandle::from_items(vec![Some(window)])));
    let visible = document("visible");
    let dock = DocumentDock::new();
    dock.set_visible_dockables(Some(ListHandle::from_items(vec![
        Some(shared.clone()),
        Some(visible.clone()),
    ])));
    let split_visible = document("split-visible");
    let pane = document("pane");
    let content = document("content");
    let split = SplitViewDock::new();
    split.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
        split_visible.clone(),
    )])));
    split.set_pane_dockable(Some(pane.clone()));
    split.set_content_dockable(Some(content.clone()));
    let excluded = document("not in selector traversal");
    let pinned = ToolDock::new();
    pinned.set_visible_dockables(Some(ListHandle::from_items(vec![Some(excluded)])));
    root.set_pinned_dock(Some(pinned));
    root.set_visible_dockables(Some(ListHandle::from_items(vec![
        Some(dock),
        Some(split),
        Some(root.clone()),
    ])));
    let mut items = Vec::new();
    let mut visited = Dictionary::new();
    collect_dockables(Some(root.clone()), &mut items, &mut visited);
    let expected = [
        hidden,
        shared,
        left,
        right,
        top,
        bottom,
        floating,
        visible,
        split_visible,
        pane,
        content,
    ];
    assert_eq!(items.len(), expected.len());
    for (actual, expected) in items.iter().zip(expected.iter()) {
        assert!(Rc::ptr_eq(actual, expected));
    }
    root.set_visible_dockables(None);
}
#[test]
fn selector_traversal_retains_models_and_visits_a_shared_split_pane_only_once() {
    let split = SplitViewDock::new();
    let model: Rc<dyn IDockable> = Document::new();
    let weak = Rc::downgrade(&model);
    split.set_pane_dockable(Some(model.clone()));
    split.set_content_dockable(Some(model.clone()));
    let mut items = Vec::new();
    let mut visited = Dictionary::new();
    collect_dockables(Some(split.clone()), &mut items, &mut visited);
    assert_eq!(items.len(), 1);
    assert!(Rc::ptr_eq(&items[0], &model));
    drop(model);
    split.set_pane_dockable(None);
    split.set_content_dockable(None);
    drop(split);
    assert!(weak.upgrade().is_some());
    drop(items);
    assert!(weak.upgrade().is_some());
    drop(visited);
    assert!(weak.upgrade().is_none());
}
