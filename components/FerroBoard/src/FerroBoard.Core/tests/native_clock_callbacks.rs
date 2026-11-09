use ferroboard_core::{editor::*, engine::KeyboardEngine, layouts::KeyboardResources};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

struct RetiredClock {
    editor: Weak<EditorController>,
    engine: Weak<KeyboardEngine>,
    dropped: Rc<Cell<bool>>,
}
impl Drop for RetiredClock {
    fn drop(&mut self) {
        if let Some(editor) = self.editor.upgrade() {
            assert!(editor.clock().is_none());
            editor.set_clock_nullable(None);
        }
        if let Some(engine) = self.engine.upgrade() {
            assert!(engine.clock().is_none());
            engine.set_clock_nullable(None);
        }
        self.dropped.set(true);
    }
}
#[test]
fn replaced_clock_captures_drop_after_state_borrows_are_released() {
    let editor = Rc::new(EditorController::new(Rc::new(TextBufferTarget::default())));
    let engine =
        KeyboardEngine::new(None, Some(Rc::new(KeyboardResources::create_empty())), None).unwrap();
    for use_editor in [true, false] {
        let dropped = Rc::new(Cell::new(false));
        let captured = RetiredClock {
            editor: if use_editor {
                Rc::downgrade(&editor)
            } else {
                Weak::new()
            },
            engine: if use_editor {
                Weak::new()
            } else {
                Rc::downgrade(&engine)
            },
            dropped: dropped.clone(),
        };
        let callback: EditorClock = Rc::new(move || {
            let _retained = &captured;
            std::time::UNIX_EPOCH
        });
        if use_editor {
            editor.set_clock(callback);
            editor.set_clock_nullable(None);
        } else {
            engine.set_clock(callback);
            engine.set_clock_nullable(None);
        }
        assert!(dropped.get());
    }
    engine.dispose().unwrap();
}
