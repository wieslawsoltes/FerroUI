use crate::completion_tip_content_control::CompletionTipContentControl;
use crate::completion_window_base::CompletionWindowBaseImplExt;
use crate::completion_window_base::{CompletionWindowBase, CompletionWindowBaseImpl};
use crate::TextEditor;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, Thickness, VisualImpl,
};
use ferroui_controls::{Border, ControlImpl};
use ferroui_edit_core::document::DocumentError;
use std::cell::OnceCell;

/// Signature or documentation assistance attached to a tracked document range.
#[repr(C)]
pub struct InsightWindow {
    base: CompletionWindowBase,
    presenter: OnceCell<Ref<CompletionTipContentControl>>,
    surface: OnceCell<Ref<Border>>,
}
ferro_class!(InsightWindow: CompletionWindowBase);
ferroui_base::ferro_class_info!(InsightWindow {
    new: InsightWindow::new
});
ferro_impl_classes!(InsightWindow:
    StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl
);
ferro_properties! { impl InsightWindow {
    pub fn close_automatically_property() -> StyledProperty<bool> {
        FerroProperty::register::<InsightWindow, _>("CloseAutomatically", true)
    }
    pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<InsightWindow, _>("Content", None)
    }
} }
impl FerroObjectImpl for InsightWindow {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let presenter = CompletionTipContentControl::new();
        let surface = Border::new();
        surface.set_padding(Thickness::uniform(3.0));
        surface.set_child(presenter.clone());
        this.set_child(surface.clone());
        this.presenter.set(presenter).ok();
        this.surface.set(surface).ok();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::content_property().as_property() {
            if let Some(presenter) = this.presenter.get() {
                presenter.set_content(this.content());
            }
            this.update_position();
        } else if change.property() == Self::close_automatically_property().as_property() {
            this.set_is_light_dismiss_enabled(this.close_automatically());
            this.refresh_from_editor();
        }
    }
}
impl CompletionWindowBaseImpl for InsightWindow {
    fn on_editor_updated(this: &Self) {
        Self::parent_on_editor_updated(this);
        if this.close_automatically() {
            if let Some(editor) = this.editor() {
                let caret = editor.caret_offset();
                if caret < this.start_offset() || caret > this.end_offset() {
                    this.dismiss();
                }
            }
        }
    }
    fn close_on_focus_lost(this: &Self) -> bool {
        this.close_automatically()
    }
}
impl InsightWindow {
    pub fn construct() -> Self {
        Self {
            base: CompletionWindowBase::construct(),
            presenter: OnceCell::new(),
            surface: OnceCell::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn show_for(
        editor: &Ref<TextEditor>,
        start: usize,
        end: usize,
        content: Option<BoxedValue>,
    ) -> Result<Ref<Self>, DocumentError> {
        let window = Self::new();
        window.set_content(content);
        window.attach_to_editor(editor, start, end)?;
        window.show();
        Ok(window)
    }
    pub fn attach_to_editor(
        &self,
        editor: &Ref<TextEditor>,
        start: usize,
        end: usize,
    ) -> Result<(), DocumentError> {
        CompletionWindowBase::attach_to_editor(self, editor, start, end)?;
        self.apply_editor_style();
        let weak = self.to_ref().downgrade();
        self.track_subscription(editor.actual_theme_variant_changed(move || {
            if let Some(window) = weak.upgrade() {
                window.apply_editor_style();
            }
        }));
        Ok(())
    }
    fn apply_editor_style(&self) {
        if let (Some(surface), Some(editor)) = (self.surface.get(), self.editor()) {
            crate::search_panel::style_overlay(surface, &editor);
        }
    }
    pub fn close_automatically(&self) -> bool {
        self.get_value(Self::close_automatically_property())
    }
    pub fn set_close_automatically(&self, value: bool) {
        self.set_value(Self::close_automatically_property(), value);
    }
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }
    pub fn set_content(&self, content: Option<BoxedValue>) {
        self.set_value(Self::content_property(), content);
    }
    pub fn active_for(editor: &TextEditor) -> Option<Ref<Self>> {
        editor
            .get_value(CompletionWindowBase::active_windows_property())
            .iter()
            .rev()
            .find_map(|window| window.cast::<Self>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    use std::rc::Rc;

    #[test]
    fn insight_tracks_edits_caret_bounds_and_disposes_editor_subscriptions() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("x call(arg) tail").unwrap();
        editor.set_caret_offset(7).unwrap();
        let baseline = editor.property_changed_subscriber_count();
        let window =
            InsightWindow::show_for(&editor, 2, 11, Some(Rc::new("signature".to_string())))
                .unwrap();
        editor.document().insert(0, "😀").unwrap();
        assert_eq!(
            window.segment(),
            ferroui_edit_core::document::SimpleSegment::new(4, 9)
        );
        editor
            .set_caret_offset(editor.document().text_length())
            .unwrap();
        assert!(window.is_closed());
        assert!(InsightWindow::active_for(&editor).is_none());
        assert_eq!(editor.property_changed_subscriber_count(), baseline);
    }

    #[test]
    fn insight_manual_lifetime_escape_and_document_replacement() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("call() tail").unwrap();
        editor.set_caret_offset(3).unwrap();
        let window = InsightWindow::show_for(&editor, 0, 6, None).unwrap();
        window.set_close_automatically(false);
        editor.set_caret_offset(10).unwrap();
        assert!(!window.is_closed());
        window.close_if_focus_lost();
        assert!(!window.is_closed());
        let mut escape = KeyEventArgs::new();
        escape.key = Key::Escape;
        escape.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&escape);
        assert!(escape.handled());
        assert!(window.is_closed());
        let next = InsightWindow::show_for(&editor, 0, 11, None).unwrap();
        editor.set_document(crate::TextDocument::from_text("other"));
        assert!(next.is_closed());
    }

    #[test]
    fn expected_prefix_insertion_is_consumed_once_and_backspace_closes() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("x()").unwrap();
        editor.set_caret_offset(1).unwrap();
        let window = InsightWindow::show_for(&editor, 1, 3, None).unwrap();
        window.set_expect_insertion_before_start(true);
        editor.document().insert(1, "a").unwrap();
        assert_eq!(window.start_offset(), 2);
        assert!(!window.expect_insertion_before_start());
        editor.document().insert(2, "b").unwrap();
        assert_eq!(window.start_offset(), 2);
        editor.document().remove(1, 1).unwrap();
        assert!(window.is_closed());
    }
}
