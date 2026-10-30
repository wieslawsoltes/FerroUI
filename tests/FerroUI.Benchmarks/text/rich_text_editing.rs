//! Not from upstream: editing a large rich text document shown by a `RichTextBlock` (F1 of
//! `docs/porting/winui-rich-text.md`; #243 review S7 and S9). Each benchmark changes the document
//! or the selection by one step and measures and arranges the view again, as a keystroke does:
//! the time of a step against the size of the document.

use crate::harness::Registry;
use ferroui_base::{Ref, Size};
use ferroui_controls::documents::{Paragraph, RichTextDocument};
use ferroui_controls::rich_text::{EditKind, EditorSelection, RichTextEditor};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::RichTextBlock;
use std::cell::Cell;

const SENTENCE: &str = "Start with a thought, add emphasis, and mark the next step of the plan. ";

pub struct RichTextEditing {
    view: Ref<RichTextBlock>,
    document: RichTextDocument,
    editor: RichTextEditor,
    caret: Cell<usize>,
    length: usize,
    _app: UnitTestApplicationScope,
}

impl RichTextEditing {
    pub fn new(paragraphs: usize) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());
        let view = RichTextBlock::new();
        for index in 0..paragraphs {
            view.blocks().add(Paragraph::with_text(&format!("{index}. {}", SENTENCE.repeat(3))));
        }
        let document = view.rich_document();
        let editor = RichTextEditor::new(document.clone());
        let length = document.text_length();
        let this = Self { view, document, editor, caret: Cell::new(length / 2), length, _app: app };
        this.layout();
        this
    }

    fn layout(&self) {
        self.view.measure(Size::new(600.0, f64::INFINITY));
        let desired = self.view.desired_size();
        self.view.arrange(ferroui_base::Rect::new(0.0, 0.0, 600.0, desired.height));
    }

    /// One typed character in the middle of the document, and its deletion on the next call, so
    /// that the paragraph keeps its size however many invocations a run makes.
    pub fn type_character(&self) {
        let caret = self.caret.get();
        let _ = self.editor.set_selection(EditorSelection::caret(caret));
        if self.document.text_length() > self.length {
            self.editor.delete_backward(false).unwrap();
            self.caret.set(caret - 1);
        } else {
            self.editor.insert_text("x", EditKind::Typing).unwrap();
            self.caret.set(caret + 1);
        }
        self.layout();
    }

    /// One step of the selection of the view.
    pub fn move_selection(&self) {
        let caret = (self.caret.get() + 1) % self.length;
        self.caret.set(caret);
        self.view.select(caret, caret + 1);
        self.layout();
    }

    /// One update of the preview of an input method.
    pub fn update_ime_preview(&self) {
        if !self.editor.is_composing() {
            let _ = self.editor.set_selection(EditorSelection::caret(self.caret.get()));
        }
        let step = self.caret.get() % 4;
        self.editor.set_preedit(&"か".repeat(step + 1), None).unwrap();
        self.caret.set(self.caret.get() + 1);
        self.layout();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "RichTextEditing").min_iteration_time(150);
    for paragraphs in [10usize, 100, 1000] {
        let parameters = format!("Paragraphs={paragraphs}");
        class.benchmark("type_character", parameters.clone(), move || RichTextEditing::new(paragraphs), |b| b.type_character());
        class.benchmark("move_selection", parameters.clone(), move || RichTextEditing::new(paragraphs), |b| b.move_selection());
        class.benchmark("update_ime_preview", parameters, move || RichTextEditing::new(paragraphs), |b| b.update_ime_preview());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rich_text_editing() {
        crate::harness::smoke_class(super::register, "RichTextEditing");
    }
}
