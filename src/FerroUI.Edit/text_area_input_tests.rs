use super::*;
use crate::{EditorCommandKind, TextDocument, TextEditor};
use ferroui_base::input::KeyGesture;
use ferroui_edit_core::editing::EditError;

type InputLog = Rc<RefCell<Vec<(&'static str, &'static str)>>>;

struct TestHandler {
    name: &'static str,
    log: InputLog,
    intercept_back: bool,
    bind_selection: bool,
    children: Vec<Rc<dyn EditorInputHandler>>,
}

impl EditorInputHandler for TestHandler {
    fn child_handlers(&self) -> Vec<Rc<dyn EditorInputHandler>> {
        self.children.clone()
    }
    fn attach(&self, area: &TextArea, bindings: &mut EditorInputBindings) {
        self.log.borrow_mut().push((self.name, "attach"));
        let name = self.name;
        let log = self.log.clone();
        bindings.track(
            area.add_disposable_handler(TextArea::text_entered_event(), move |_, _| {
                log.borrow_mut().push((name, "text"));
            }),
        );
        if self.bind_selection {
            let binding = KeyBinding::new();
            binding.set_gesture(Some(KeyGesture::new(Key::F2, KeyModifiers::NONE)));
            binding.set_command(Some(
                area.editor().unwrap().command(EditorCommandKind::SelectAll),
            ));
            bindings.add_key_binding(binding);
        }
    }

    fn detach(&self, _: &TextArea) {
        self.log.borrow_mut().push((self.name, "detach"));
    }

    fn on_key_down(&self, _: &TextArea, event: &KeyEventArgs) {
        self.log.borrow_mut().push((self.name, "down"));
        if self.intercept_back && event.key == Key::Back {
            event.set_handled(true);
        }
    }

    fn on_key_up(&self, _: &TextArea, _: &KeyEventArgs) {
        self.log.borrow_mut().push((self.name, "up"));
    }
}

fn raise_key(area: &TextArea, key: Key, down: bool) -> bool {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.set_routed_event(Some(if down {
        InputElement::key_down_event()
    } else {
        InputElement::key_up_event()
    }));
    area.raise_event(&event);
    event.handled()
}

#[test]
fn native_key_layers_use_lifo_order_and_restore_the_default_editor() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    let area = editor.text_area();
    editor.set_text("abc").unwrap();
    editor.set_caret_offset(3).unwrap();
    let log: InputLog = Rc::default();
    let make = |name, intercept_back, bind_selection| {
        Rc::new(TestHandler {
            name,
            log: log.clone(),
            intercept_back,
            bind_selection,
            children: Vec::new(),
        })
    };
    let changes = log.clone();
    let _mode_changed = area.add_disposable_handler(
        TextArea::active_input_handler_changed_event(),
        move |_, _| {
            changes.borrow_mut().push(("mode", "change"));
        },
    );

    let primary = make("primary", false, true);
    area.set_active_input_mode(EditorInputMode::Custom(primary.clone()))
        .unwrap();
    assert!(!DragDrop::get_allow_drop(&area));
    let lower = make("lower", false, false);
    let bottom = area.push_stacked_input_handler(lower.clone()).unwrap();
    assert!(matches!(
        area.push_stacked_input_handler(lower),
        Err(EditorInputHandlerError::AlreadyAttached)
    ));
    let top = area
        .push_stacked_input_handler(make("top", true, false))
        .unwrap();
    area.set_active_input_mode(EditorInputMode::Custom(primary))
        .unwrap();
    assert_eq!(
        log.take(),
        [
            ("primary", "attach"),
            ("mode", "change"),
            ("lower", "attach"),
            ("top", "attach")
        ]
    );

    assert!(raise_key(&area, Key::Back, true));
    assert_eq!(editor.text(), "abc");
    assert_eq!(log.take(), [("top", "down")]);
    assert!(!raise_key(&area, Key::Back, false));
    assert_eq!(
        log.take(),
        [("top", "up"), ("lower", "up"), ("primary", "up")]
    );
    assert!(raise_key(&area, Key::F2, true));
    assert_eq!(editor.selected_text().unwrap(), "abc");
    assert_eq!(
        log.take(),
        [("top", "down"), ("lower", "down"), ("primary", "down")]
    );

    assert!(bottom.pop().unwrap());
    assert!(!top.pop().unwrap());
    assert_eq!(log.take(), [("top", "detach"), ("lower", "detach")]);
    assert!(area.perform_text_input("z").unwrap());
    assert_eq!(log.take(), [("primary", "text")]);
    area.set_active_input_mode(EditorInputMode::Default)
        .unwrap();
    assert!(DragDrop::get_allow_drop(&area));
    assert!(raise_key(&area, Key::Back, true));
    assert!(editor.text().is_empty());
    assert_eq!(log.take(), [("primary", "detach"), ("mode", "change")]);
}

struct StackChange {
    layer: Rc<RefCell<Option<EditorInputLayer>>>,
}

impl EditorInputHandler for StackChange {
    fn on_key_down(&self, _: &TextArea, _: &KeyEventArgs) {
        let layer = self.layer.borrow().clone().unwrap();
        assert!(layer.pop().unwrap());
    }

    fn detach(&self, area: &TextArea) {
        assert!(matches!(
            area.set_active_input_mode(EditorInputMode::Disabled),
            Err(EditorInputHandlerError::ChangingHandlers)
        ));
    }
}

#[test]
fn a_layer_can_pop_older_layers_during_key_delivery() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    let area = editor.text_area();
    let log: InputLog = Rc::default();
    let layer = area
        .push_stacked_input_handler(Rc::new(TestHandler {
            name: "old",
            log: log.clone(),
            intercept_back: true,
            bind_selection: false,
            children: Vec::new(),
        }))
        .unwrap();
    let state = Rc::new(RefCell::new(Some(layer)));
    area.push_stacked_input_handler(Rc::new(StackChange { layer: state }))
        .unwrap();
    editor.set_text("x").unwrap();
    editor.set_caret_offset(1).unwrap();
    assert!(raise_key(&area, Key::Back, true));
    assert!(editor.text().is_empty());
    assert!(area.stacked_input_handlers().is_empty());
    assert_eq!(log.take(), [("old", "attach"), ("old", "detach")]);
}

#[test]
fn input_notifications_cancel_before_edit_and_publish_after_routed_or_programmatic_input() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    editor.set_text("  a\r\n  b").unwrap();
    editor.set_caret_offset(3).unwrap();
    let area = editor.text_area();
    let observations = Rc::new(RefCell::new(Vec::new()));
    let weak = editor.downgrade();
    let before = observations.clone();
    let _entering =
        area.add_disposable_handler(TextArea::text_entering_event(), move |_, event| {
            before
                .borrow_mut()
                .push(("before", weak.upgrade().unwrap().text()));
            if event.text.as_deref() == Some("blocked") {
                event.set_handled(true);
            }
        });
    let weak = editor.downgrade();
    let after = observations.clone();
    let _entered = area.add_disposable_handler(TextArea::text_entered_event(), move |_, _| {
        after
            .borrow_mut()
            .push(("after", weak.upgrade().unwrap().text()));
    });
    assert!(!area.perform_text_input("blocked").unwrap());
    assert_eq!(observations.take(), [("before", "  a\r\n  b".into())]);
    assert!(raise_key(&area, Key::Return, true));
    assert_eq!(
        observations.take(),
        [
            ("before", "  a\r\n  b".into()),
            ("after", "  a\r\n  \r\n  b".into())
        ]
    );
    let send = |text: &str| {
        let mut event = TextInputEventArgs::new();
        event.text = Some(text.into());
        event.set_routed_event(Some(InputElement::text_input_event()));
        area.raise_event(&event);
        event.handled()
    };
    assert!(!send("\u{7f}"));
    assert!(observations.borrow().is_empty());
    assert!(send("é"));
    assert_eq!(
        observations.take(),
        [
            ("before", "  a\r\n  \r\n  b".into()),
            ("after", "  a\r\n  é\r\n  b".into())
        ]
    );
    editor.set_is_read_only(true);
    assert!(matches!(
        area.perform_text_input("x"),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(
        observations.take(),
        [("before", "  a\r\n  é\r\n  b".into())]
    );
}

#[test]
fn a_text_entering_listener_cannot_send_pending_input_to_a_replacement_document() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    let old = TextDocument::from_text("first");
    editor.set_document(old.clone());
    let area = editor.text_area();
    let weak = editor.downgrade();
    let _entering = area.add_disposable_handler(TextArea::text_entering_event(), move |_, _| {
        weak.upgrade()
            .unwrap()
            .set_document(TextDocument::from_text("second"));
    });
    assert!(matches!(
        area.perform_text_input("x"),
        Err(EditError::StaleEditPlan)
    ));
    assert_eq!(editor.text(), "second");
    assert_eq!(old.text(), "first");
}

#[test]
fn return_uses_each_carets_delimiter_in_a_mixed_document() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    editor.set_text("a\r\nb\nc").unwrap();
    editor
        .set_selection(Selection::from_segments(
            [SelectionSegment::empty(1), SelectionSegment::empty(4)],
            1,
        ))
        .unwrap();
    assert!(raise_key(&editor.text_area(), Key::Return, true));
    assert_eq!(editor.text(), "a\r\n\r\nb\n\nc");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "a\r\nb\nc");
}

#[test]
fn nested_handler_groups_validate_before_replacing_the_current_input() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    let area = editor.text_area();
    let log: InputLog = Rc::default();
    let make = |name, children: Vec<Rc<dyn EditorInputHandler>>| {
        Rc::new(TestHandler {
            name,
            log: log.clone(),
            intercept_back: false,
            bind_selection: false,
            children,
        })
    };
    let child = make("child", Vec::new());
    let parent = make("parent", vec![child.clone(), make("sibling", Vec::new())]);
    area.set_active_input_mode(EditorInputMode::Custom(parent))
        .unwrap();
    assert_eq!(
        log.take(),
        [
            ("parent", "attach"),
            ("child", "attach"),
            ("sibling", "attach")
        ]
    );
    assert!(matches!(
        area.push_stacked_input_handler(child.clone()),
        Err(EditorInputHandlerError::AlreadyAttached)
    ));
    let invalid = make("invalid", vec![child.clone(), child]);
    assert_eq!(
        area.set_active_input_mode(EditorInputMode::Custom(invalid)),
        Err(EditorInputHandlerError::AlreadyAttached)
    );
    assert!(log.borrow().is_empty());
    assert!(!raise_key(&area, Key::F2, true));
    assert_eq!(
        log.take(),
        [("parent", "down"), ("child", "down"), ("sibling", "down")]
    );
    area.set_active_input_mode(EditorInputMode::Disabled)
        .unwrap();
    assert_eq!(
        log.take(),
        [
            ("sibling", "detach"),
            ("child", "detach"),
            ("parent", "detach")
        ]
    );
}

#[test]
fn replacing_the_active_handler_tears_down_live_snippet_and_completion_layers() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    let area = editor.text_area();
    let snippet = crate::core::snippets::Snippet::parse("${1:first} ${2:second}$0").unwrap();
    let controller = editor.insert_snippet(&snippet).unwrap();
    let completion = crate::CompletionWindow::show_for(
        &editor,
        0,
        5,
        vec![crate::core::completion::CompletionItem::new("first_item")],
    )
    .unwrap();
    assert_eq!(area.stacked_input_handlers().len(), 2);
    area.set_active_input_mode(EditorInputMode::Disabled)
        .unwrap();
    assert!(completion.is_closed());
    assert!(!controller.is_active());
    assert!(area.stacked_input_handlers().is_empty());
    assert!(!raise_key(&area, Key::Tab, true));
    assert_eq!(editor.text(), "first second");
}
