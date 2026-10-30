use super::*;
use crate::documents::{CharacterFormat, RichTextDocument};
use ferroui_base::media::{IBrush, ISolidColorBrush, ITransform};
use ferroui_base::threading::Dispatcher;
use ferroui_base::RelativePoint;
use std::any::Any;
use std::cell::Cell;

struct InstallationBrush {
    reads: Cell<usize>,
    fire_at: Cell<Option<usize>>,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl InstallationBrush {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            reads: Cell::new(0),
            fire_at: Cell::new(None),
            callback: RefCell::new(None),
        })
    }

    fn arm(&self, read: usize, callback: impl FnOnce() + 'static) {
        self.reads.set(0);
        self.fire_at.set(Some(read));
        *self.callback.borrow_mut() = Some(Box::new(callback));
    }

    fn disarm(&self) -> usize {
        self.fire_at.set(None);
        let callback = self.callback.borrow_mut().take();
        drop(callback);
        self.reads.get()
    }
}

impl IBrush for InstallationBrush {
    fn opacity(&self) -> f64 {
        1.0
    }
    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }
    fn transform_origin(&self) -> RelativePoint {
        RelativePoint::CENTER
    }
    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_solid_color_brush(&self) -> Option<&dyn ISolidColorBrush> {
        let read = self.reads.get() + 1;
        self.reads.set(read);
        if self.fire_at.get() == Some(read) {
            let callback = self.callback.borrow_mut().take();
            if let Some(callback) = callback {
                callback();
            }
        }
        None
    }
}

fn model_with_brush() -> (RichTextDocument, Rc<InstallationBrush>) {
    let model = RichTextDocument::from_text("alpha beta");
    let brush = InstallationBrush::new();
    model
        .element_at(1, TextGravity::Forward)
        .unwrap()
        .set_foreground(Some(brush.clone()));
    (model, brush)
}

#[test]
fn rich_field_live_model_install_preflight_preserves_every_accepted_callback_write() {
    let _scope = Dispatcher::unit_test_scope();
    let candidate = DocumentSnapshot::from_text("replacement");
    let (probe, brush) = model_with_brush();
    brush.arm(usize::MAX, || {});
    probe.restore(&candidate).unwrap();
    let reads = brush.disarm();
    assert!(reads > 1, "capture and reuse preparation both read the brush");
    let mut accepted_writes = 0;
    for read in 1..=reads {
        let (model, brush) = model_with_brush();
        let callback_model = model.clone();
        let outcome = Rc::new(RefCell::new(None));
        let callback_outcome = outcome.clone();
        brush.arm(read, move || {
            *callback_outcome.borrow_mut() = Some(
                callback_model.replace_text(0..callback_model.text_length(), "external"),
            );
        });
        let result = model.restore(&candidate);
        brush.disarm();
        let callback_result = outcome.borrow_mut().take().expect("measured read reached");
        match callback_result {
            Ok(()) => {
                accepted_writes += 1;
                assert!(matches!(result, Err(DocumentError::EditInProgress)), "read {read}");
                assert_eq!(model.text(), "external", "read {read}");
            }
            Err(DocumentError::EditInProgress) => {
                result.unwrap();
                assert_eq!(model.text(), "replacement");
            }
            Err(error) => panic!("read {read}: unexpected callback error {error}"),
        }
        model.snapshot().validate().unwrap();
    }
    assert!(accepted_writes > 1, "the late reuse-read interval was exercised");
}

#[test]
fn rich_field_live_noop_install_does_not_accept_a_callback_changed_source() {
    let _scope = Dispatcher::unit_test_scope();
    let (model, brush) = model_with_brush();
    let candidate = model.snapshot();
    let callback_model = model.clone();
    brush.arm(1, move || {
        callback_model
            .replace_text(0..callback_model.text_length(), "external")
            .unwrap();
    });
    assert!(matches!(model.restore(&candidate), Err(DocumentError::EditInProgress)));
    brush.disarm();
    assert_eq!(model.text(), "external");
}

fn live_with_brush() -> (RichEditTextDocument, RichEditTextFieldRange, Rc<InstallationBrush>) {
    let (model, brush) = model_with_brush();
    let document = RichEditTextDocument::new(model);
    document.selection().set_range(9, 7).unwrap();
    let range = document.field_story().get_range_from_visible(0..5).unwrap();
    (document, range, brush)
}

#[test]
fn rich_field_live_link_install_read_sweep_preserves_callback_content_and_discards_stale_history() {
    let _scope = Dispatcher::unit_test_scope();
    let (probe, range, brush) = live_with_brush();
    brush.arm(usize::MAX, || {});
    range.set_link(Some("\"target\"")).unwrap();
    let reads = brush.disarm();
    assert!(reads > 0);
    probe.document().snapshot().validate().unwrap();
    let mut accepted_writes = 0;
    for read in 1..=reads {
        let (document, range, brush) = live_with_brush();
        let model = document.document();
        let outcome = Rc::new(RefCell::new(None));
        let callback_outcome = outcome.clone();
        brush.arm(read, move || {
            *callback_outcome.borrow_mut() =
                Some(model.replace_text(0..model.text_length(), "external"));
        });
        let result = range.set_link(Some("\"target\""));
        brush.disarm();
        match outcome.borrow_mut().take().expect("measured read reached") {
            Ok(()) => {
                accepted_writes += 1;
                assert!(
                    matches!(result, Err(TextFieldError::Edit(EditError::ReentrantEdit))),
                    "read {read}: {result:?}"
                );
                assert_eq!(document.text(), "external", "read {read}");
                assert!(!document.can_undo(), "read {read}");
                assert!(!document.can_redo(), "read {read}");
            }
            Err(DocumentError::EditInProgress) => {
                assert!(result.unwrap(), "read {read}");
                assert_eq!(document.text(), "alpha beta");
                assert_eq!(document.editor().selection(), super::super::EditorSelection::new(9, 7));
            }
            Err(error) => panic!("read {read}: unexpected callback error {error}"),
        }
        document.document().snapshot().validate().unwrap();
    }
    assert!(accepted_writes > 1);
}

#[test]
fn rich_field_live_failed_preflight_rejection_marks_the_enclosing_editor_transaction() {
    let _scope = Dispatcher::unit_test_scope();
    let (model, brush) = model_with_brush();
    let document = RichEditTextDocument::new(model.clone());
    let mut candidate = model.snapshot();
    candidate.paragraphs[0].format.character = CharacterFormat {
        font_size: Some(28.0),
        ..Default::default()
    };
    let callback_model = model.clone();
    let result = document.editor().edit_group(|editor| {
        let version = callback_model.mutation_version();
        let external = callback_model.clone();
        brush.arm(1, move || {
            external.replace_text(0..external.text_length(), "external").unwrap();
        });
        editor.install_field_snapshot(&candidate, version)
    });
    brush.disarm();
    assert!(matches!(result, Err(EditError::ReentrantEdit)));
    assert_eq!(document.text(), "external");
    assert!(!document.can_undo());
}
