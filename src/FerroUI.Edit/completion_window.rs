use crate::completion_data::{completion_edit_error, ICompletionData};
use crate::completion_list::CompletionList;
use crate::completion_tip_content_control::CompletionTipContentControl;
use crate::completion_window_base::CompletionWindowBaseImplExt;
use crate::completion_window_base::{CompletionWindowBase, CompletionWindowBaseImpl};
use crate::TextEditor;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, KeyEventArgs, PointerWheelEventArgs,
    TextInputEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, AttachedProperty,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, Thickness, VisualImpl,
};
use ferroui_controls::{Border, Control, ControlImpl, StackPanel};
use ferroui_edit_core::completion::CompletionItem;
use ferroui_edit_core::document::{DocumentEdit, DocumentError};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// Anchored completion assistance with native rich item presentation.
/// Keyboard and text input are forwarded while focus stays in the editor.
#[repr(C)]
pub struct CompletionWindow {
    base: CompletionWindowBase,
    list: OnceCell<Ref<CompletionList>>,
    description: OnceCell<Ref<CompletionTipContentControl>>,
    surface: OnceCell<Ref<Border>>,
    refreshing: Cell<bool>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}
ferro_class!(CompletionWindow: CompletionWindowBase);
ferroui_base::ferro_class_info!(CompletionWindow {
    new: CompletionWindow::new
});
ferro_impl_classes!(CompletionWindow: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ControlImpl);
ferro_properties! { impl CompletionWindow {
    pub fn active_window_property() -> AttachedProperty<Option<Ref<CompletionWindow>>> {
        FerroProperty::register_attached::<CompletionWindow, TextEditor, _>("ActiveWindow", None)
    }
    pub fn close_automatically_property() -> StyledProperty<bool> { FerroProperty::register::<CompletionWindow, _>("CloseAutomatically", true) }
    pub fn close_when_caret_at_beginning_property() -> StyledProperty<bool> { FerroProperty::register::<CompletionWindow, _>("CloseWhenCaretAtBeginning", false) }
} }
impl FerroObjectImpl for CompletionWindow {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.build_content();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::close_automatically_property().as_property() {
            this.set_is_light_dismiss_enabled(this.close_automatically());
            this.refresh();
        }
    }
}
impl InputElementImpl for CompletionWindow {
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        Self::parent_on_key_down(this, event);
        if !event.handled() && !this.is_closed() {
            this.completion_list().handle_key(event);
        }
    }
}
impl CompletionWindowBaseImpl for CompletionWindow {
    fn on_editor_updated(this: &Self) {
        Self::parent_on_editor_updated(this);
        this.refresh_items();
    }
    fn close_on_focus_lost(this: &Self) -> bool {
        this.close_automatically()
    }
    fn on_closed(this: &Self) {
        Self::parent_on_closed(this);
        if let Some(editor) = this.editor() {
            if editor.get_value(Self::active_window_property()).as_ref() == Some(&this.to_ref()) {
                editor.clear_value(Self::active_window_property());
            }
        }
    }
}
impl CompletionWindow {
    pub fn construct() -> Self {
        Self {
            base: CompletionWindowBase::construct(),
            list: OnceCell::new(),
            description: OnceCell::new(),
            surface: OnceCell::new(),
            refreshing: Cell::new(false),
            subscriptions: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn show_for(
        editor: &Ref<TextEditor>,
        start: usize,
        end: usize,
        items: Vec<CompletionItem>,
    ) -> Result<Ref<Self>, DocumentError> {
        Self::show_with_data(
            editor,
            start,
            end,
            items
                .into_iter()
                .map(|item| Rc::new(item) as Rc<dyn ICompletionData>)
                .collect(),
        )
    }
    pub fn show_with_data(
        editor: &Ref<TextEditor>,
        start: usize,
        end: usize,
        items: Vec<Rc<dyn ICompletionData>>,
    ) -> Result<Ref<Self>, DocumentError> {
        let window = Self::new();
        window.completion_list().set_completion_data(items);
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
        if self.is_closed() {
            return Ok(());
        }
        editor.set_value(Self::active_window_property(), Some(self.to_ref()));
        self.apply_editor_style();
        let weak = self.to_ref().downgrade();
        self.track_subscription(editor.actual_theme_variant_changed(move || {
            if let Some(window) = weak.upgrade() {
                window.apply_editor_style();
            }
        }));
        let area = editor.text_area();
        let weak = self.to_ref().downgrade();
        self.track_subscription(area.add_disposable_handler_with(
            InputElement::text_input_event(),
            move |_, event| {
                if let Some(window) = weak.upgrade() {
                    if window.is_closed() || !window.owns_editor_input() {
                        return;
                    }
                    let mut forwarded = TextInputEventArgs::new();
                    forwarded.text = event.text.clone();
                    forwarded.set_routed_event(Some(InputElement::text_input_event()));
                    window.raise_event(&forwarded);
                    if forwarded.handled() {
                        event.set_handled(true);
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        ));
        let weak = self.to_ref().downgrade();
        self.track_subscription(area.add_disposable_handler_with(
            InputElement::pointer_wheel_changed_event(),
            move |_, event| {
                if let Some(window) = weak.upgrade() {
                    if window.is_closed() || !window.owns_editor_input() {
                        return;
                    }
                    if let Some(scroll) = window.completion_list().list_box().scroll() {
                        let point = event.get_current_point(Some(&scroll));
                        let forwarded = PointerWheelEventArgs::new(
                            scroll.clone(),
                            event.pointer().clone(),
                            &scroll,
                            point.position,
                            event.timestamp(),
                            point.properties,
                            event.key_modifiers(),
                            event.delta(),
                        );
                        scroll.raise_event(&forwarded);
                        if forwarded.handled() {
                            event.set_handled(true);
                        }
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        ));
        Ok(())
    }
    pub fn active_for(editor: &TextEditor) -> Option<Ref<Self>> {
        editor
            .get_value(Self::active_window_property())
            .filter(|window| !window.is_closed())
    }
    pub fn has_active(editor: &TextEditor) -> bool {
        Self::active_for(editor).is_some()
    }
    pub fn completion_list(&self) -> Ref<CompletionList> {
        self.list
            .get()
            .expect("constructed completion list")
            .clone()
    }
    pub fn description_presenter(&self) -> Ref<CompletionTipContentControl> {
        self.description
            .get()
            .expect("constructed description")
            .clone()
    }
    pub fn close_automatically(&self) -> bool {
        self.get_value(Self::close_automatically_property())
    }
    pub fn set_close_automatically(&self, value: bool) {
        self.set_value(Self::close_automatically_property(), value);
    }
    pub fn close_when_caret_at_beginning(&self) -> bool {
        self.get_value(Self::close_when_caret_at_beginning_property())
    }
    pub fn set_close_when_caret_at_beginning(&self, value: bool) {
        self.set_value(Self::close_when_caret_at_beginning_property(), value);
    }
    pub fn selected_completion_data(&self) -> Option<Rc<dyn ICompletionData>> {
        self.completion_list().selected_item()
    }
    /// Returns a plain metadata snapshot for callers of the original API.
    pub fn selected_item(&self) -> Option<CompletionItem> {
        self.selected_completion_data().map(|item| CompletionItem {
            text: item.text().to_owned(),
            insertion_text: (item.insertion_text() != item.text())
                .then(|| item.insertion_text().to_owned()),
            description: item
                .description()
                .and_then(|value| value.downcast_ref::<String>().cloned())
                .unwrap_or_default(),
            priority: item.priority(),
        })
    }
    pub fn request_insertion(&self) -> Result<bool, DocumentError> {
        self.request_insertion_from(None)
    }
    pub fn request_insertion_from(
        &self,
        source_event: Option<&dyn IRoutedEventArgs>,
    ) -> Result<bool, DocumentError> {
        if self.is_closed() {
            return Ok(false);
        }
        self.refresh();
        if self.is_closed() {
            return Ok(false);
        }
        let Some(editor) = self.editor() else {
            return Ok(false);
        };
        let Some(item) = self.selected_completion_data() else {
            return Ok(false);
        };
        let segment = self.segment();
        editor
            .session()
            .validate_document_edits(&[DocumentEdit::new(
                segment.offset,
                segment.length,
                item.insertion_text(),
            )])
            .map_err(completion_edit_error)?;
        // Detach before dispatching custom code: an action may open a snippet
        // session or another popup, which must retain its newly installed input.
        self.close_window();
        item.complete(&editor, segment, source_event)
    }
    pub fn refresh(&self) {
        if !self.refreshing.get() {
            self.refresh_from_editor();
        }
    }
    fn refresh_items(&self) {
        if self.is_closed() || self.refreshing.replace(true) {
            return;
        }
        let Some(editor) = self.editor() else {
            self.refreshing.set(false);
            self.dismiss();
            return;
        };
        let caret = editor.caret_offset();
        if caret < self.start_offset()
            || caret > self.end_offset()
            || self.close_when_caret_at_beginning() && caret == self.start_offset()
        {
            self.refreshing.set(false);
            if self.close_automatically() {
                self.dismiss();
            }
            return;
        }
        match editor
            .document()
            .get_text(self.start_offset(), caret - self.start_offset())
        {
            Ok(query) => self.completion_list().select_item(&query),
            Err(error) => {
                editor.report_error(error.to_string());
                self.dismiss();
            }
        }
        let empty = self.completion_list().current_list().is_empty();
        self.refresh_description();
        self.refreshing.set(false);
        if empty && self.close_automatically() {
            self.dismiss();
        }
    }
    fn refresh_description(&self) {
        if let Some(control) = self.description.get() {
            let content = self
                .selected_completion_data()
                .and_then(|item| item.description());
            control.set_is_visible(content.is_some());
            control.set_content(content);
        }
    }
    fn apply_editor_style(&self) {
        if let (Some(border), Some(editor)) = (self.surface.get(), self.editor()) {
            crate::search_panel::style_overlay(border, &editor);
        }
    }
    fn build_content(&self) {
        let content = StackPanel::new();
        content.set_spacing(6.0);
        let list = CompletionList::new();
        content.children().add(list.clone().upcast::<Control>());
        let description = CompletionTipContentControl::new();
        description.set_max_width(480.0);
        content
            .children()
            .add(description.clone().upcast::<Control>());
        let border = Border::new();
        border.set_padding(Thickness::uniform(6.0));
        border.set_child(content);
        self.surface.set(border.clone()).ok();
        self.set_child(border);
        self.list.set(list.clone()).ok();
        self.description.set(description).ok();
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(list.add_disposable_handler(
                CompletionList::selection_changed_event(),
                move |_, _| {
                    if let Some(window) = weak.upgrade() {
                        window.refresh_description();
                    }
                },
            ));
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(list.add_disposable_handler(
                CompletionList::insertion_requested_event(),
                move |_, event| {
                    if let Some(window) = weak.upgrade() {
                        let editor = window.editor();
                        match window.request_insertion_from(event.source_event()) {
                            Ok(true) => event.set_handled(true),
                            Err(error) => {
                                if let Some(editor) = editor {
                                    editor.report_error(error.to_string());
                                }
                                event.set_handled(true);
                            }
                            _ => {}
                        }
                    }
                },
            ));
    }
}
impl Drop for CompletionWindow {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion_data::CompletionData;
    use ferroui_base::input::Key;

    #[test]
    fn custom_action_receives_original_event_after_popup_has_detached() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("Re").unwrap();
        editor.set_caret_offset(2).unwrap();
        let rich = ferroui_controls::TextBlock::new();
        rich.set_text(Some("Rich documentation"));
        let invoked = Rc::new(Cell::new(false));
        let invoked_action = invoked.clone();
        let item = CompletionData {
            text: "Read".to_owned(),
            description: Some(Rc::new(rich.clone().upcast::<Control>())),
            action: Some(Rc::new(move |editor, segment, event| {
                assert!(CompletionWindow::active_for(editor).is_none());
                assert!(editor
                    .get_value(CompletionWindowBase::active_windows_property())
                    .is_empty());
                assert_eq!(
                    event.unwrap().downcast_ref::<KeyEventArgs>().unwrap().key,
                    Key::Tab
                );
                assert_eq!(
                    segment,
                    ferroui_edit_core::document::SimpleSegment::new(0, 2)
                );
                editor
                    .session()
                    .apply_document_edits(&[DocumentEdit::new(
                        segment.offset,
                        segment.length,
                        "Read()",
                    )])
                    .map_err(completion_edit_error)?;
                invoked_action.set(true);
                Ok(true)
            })),
            ..CompletionData::default()
        };
        let window = CompletionWindow::show_with_data(&editor, 0, 2, vec![Rc::new(item)]).unwrap();
        assert_eq!(
            window
                .description_presenter()
                .content()
                .unwrap()
                .downcast_ref::<Ref<Control>>(),
            Some(&rich.upcast::<Control>())
        );
        let mut key = KeyEventArgs::new();
        key.key = Key::Tab;
        key.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&key);
        assert!(key.handled());
        assert!(invoked.get());
        assert!(window.is_closed());
        assert_eq!(editor.text(), "Read()");
        editor.undo().unwrap();
        assert_eq!(editor.text(), "Re");
    }

    #[test]
    fn rejected_completion_never_calls_custom_action_or_changes_document() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("Re").unwrap();
        editor.set_caret_offset(2).unwrap();
        let invoked = Rc::new(Cell::new(false));
        let invoked_action = invoked.clone();
        let item = CompletionData {
            text: "Read".to_owned(),
            action: Some(Rc::new(move |_, _, _| {
                invoked_action.set(true);
                Ok(true)
            })),
            ..CompletionData::default()
        };
        let window = CompletionWindow::show_with_data(&editor, 0, 2, vec![Rc::new(item)]).unwrap();
        editor.set_is_read_only(true);
        assert_eq!(window.request_insertion(), Err(DocumentError::ReadOnly));
        assert!(!invoked.get());
        assert!(!window.is_closed());
        assert_eq!(editor.text(), "Re");
        editor.set_is_read_only(false);
        let protected = Rc::new(
            ferroui_edit_core::editing::TextSegmentReadOnlySectionProvider::new(editor.document()),
        );
        protected.add(0, 2).unwrap();
        editor.session().set_read_only_section_provider(protected);
        assert_eq!(window.request_insertion(), Err(DocumentError::ReadOnly));
        assert!(!invoked.get());
        assert_eq!(editor.text(), "Re");
    }

    #[test]
    fn newer_popup_owns_keys_and_previous_insight_resumes_when_it_closes() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("Re").unwrap();
        editor.set_caret_offset(2).unwrap();
        let insight = crate::insight_window::InsightWindow::show_for(&editor, 0, 2, None).unwrap();
        let completion =
            CompletionWindow::show_for(&editor, 0, 2, vec![CompletionItem::new("Read")]).unwrap();
        let mut escape = KeyEventArgs::new();
        escape.key = Key::Escape;
        escape.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&escape);
        assert!(escape.handled());
        assert!(completion.is_closed());
        assert!(!insight.is_closed());
        let mut second = KeyEventArgs::new();
        second.key = Key::Escape;
        second.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&second);
        assert!(second.handled());
        assert!(insight.is_closed());
    }
    #[test]
    fn completion_popup_commits_through_editor_permission_and_undo() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("Re").unwrap();
        editor.set_caret_offset(2).unwrap();
        let window =
            CompletionWindow::show_for(&editor, 0, 2, vec![CompletionItem::new("ReadLine")])
                .unwrap();
        assert_eq!(window.selected_item().unwrap().text, "ReadLine");
        assert!(window.request_insertion().unwrap());
        assert_eq!(editor.text(), "ReadLine");
        assert!(window.is_closed());
        editor.undo().unwrap();
        assert_eq!(editor.text(), "Re");
    }
    #[test]
    fn replacing_document_closes_completion_without_editing_new_document() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("Re").unwrap();
        editor.set_caret_offset(2).unwrap();
        let window =
            CompletionWindow::show_for(&editor, 0, 2, vec![CompletionItem::new("Read")]).unwrap();
        editor.set_document(ferroui_edit_core::document::TextDocument::from_text(
            "other",
        ));
        assert!(!window.request_insertion().unwrap());
        assert!(window.is_closed());
        assert_eq!(editor.text(), "other");
        assert!(CompletionWindow::active_for(&editor).is_none());
    }

    #[test]
    fn pointer_or_programmatic_caret_movement_dismisses_completion_immediately() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("Re rest").unwrap();
        editor.set_caret_offset(2).unwrap();
        let window =
            CompletionWindow::show_for(&editor, 0, 2, vec![CompletionItem::new("Read")]).unwrap();
        editor.set_caret_offset(7).unwrap();
        assert!(window.is_closed());
        assert!(CompletionWindow::active_for(&editor).is_none());
        assert_eq!(editor.text(), "Re rest");
    }
}
