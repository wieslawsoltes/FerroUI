use crate::TextEditor;
use ferroui_base::input::{InputElement, InputElementImpl, Key, KeyEventArgs};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    AttachedProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    Thickness, VisualImpl, WeakRef, ferro_class, ferro_impl_classes, instantiate,
};
use ferroui_controls::primitives::{Popup, SelectingItemsControl};
use ferroui_controls::{Border, ControlImpl, ListBox, PlacementMode, StackPanel, TextBlock};
use ferroui_edit_core::completion::{CompletionItem, CompletionList, CompletionSession};
use ferroui_edit_core::document::DocumentError;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// A completion list positioned at the editor caret. Keyboard handling is
/// tunneled from the text area so typing keeps focus in the editor.
#[repr(C)]
pub struct CompletionWindow {
    base: Popup,
    editor: RefCell<Option<WeakRef<TextEditor>>>,
    session: RefCell<Option<CompletionSession>>,
    list: OnceCell<Ref<ListBox>>,
    description: OnceCell<Ref<TextBlock>>,
    surface: OnceCell<Ref<Border>>,
    refreshing: Cell<bool>,
    closed: Cell<bool>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}
ferro_class!(CompletionWindow:Popup);
ferroui_base::ferro_class_info!(CompletionWindow {
    new: CompletionWindow::new
});
ferro_impl_classes!(CompletionWindow:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferroui_base::ferro_properties! {impl CompletionWindow{
    /// Tracks the current completion popup on its editor, using the framework's
    /// attached-property lifetime instead of a process-global control table.
    pub fn active_window_property()->AttachedProperty<Option<Ref<CompletionWindow>>>{
        FerroProperty::register_attached::<CompletionWindow,TextEditor,_>("ActiveWindow",None)
    }
}}
impl FerroObjectImpl for CompletionWindow {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.build_content();
    }
}
impl CompletionWindow {
    pub fn construct() -> Self {
        Self {
            base: Popup::construct(),
            editor: RefCell::new(None),
            session: RefCell::new(None),
            list: OnceCell::new(),
            description: OnceCell::new(),
            surface: OnceCell::new(),
            refreshing: Cell::new(false),
            closed: Cell::new(false),
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
        let session =
            CompletionSession::new(editor.document(), start, end, CompletionList::new(items))?;
        if let Some(previous) = Self::active_for(editor) {
            previous.close_window();
        }
        let window = Self::new();
        *window.editor.borrow_mut() = Some(editor.downgrade());
        *window.session.borrow_mut() = Some(session);
        let area = editor.text_area();
        window.set_placement_target(area.clone());
        window.set_popup_parent(Some(&editor.clone().upcast::<ferroui_controls::Control>()));
        window.set_placement(PlacementMode::BottomEdgeAlignedLeft);
        window.set_placement_rect(Some(area.cursor_rectangle()));
        window.set_should_use_overlay_layer(true);
        window.set_is_light_dismiss_enabled(true);
        if let Some(border) = window.surface.get() {
            crate::search_panel::style_overlay(border, editor);
        }
        let weak = window.downgrade();
        window
            .subscriptions
            .borrow_mut()
            .push(editor.actual_theme_variant_changed(move || {
                if let Some(window) = weak.upgrade() {
                    if let (Some(border), Some(editor)) = (window.surface.get(), window.editor()) {
                        crate::search_panel::style_overlay(border, &editor);
                    }
                }
            }));
        for event in [
            TextEditor::text_changed_event(),
            TextEditor::selection_changed_event(),
        ] {
            let weak = window.downgrade();
            window
                .subscriptions
                .borrow_mut()
                .push(editor.add_disposable_handler(event, move |_, _| {
                    if let Some(window) = weak.upgrade() {
                        window.refresh();
                    }
                }));
        }
        let weak = window.downgrade();
        window
            .subscriptions
            .borrow_mut()
            .push(area.add_disposable_handler_with(
                InputElement::key_down_event(),
                move |_, e: &KeyEventArgs| {
                    if let Some(window) = weak.upgrade() {
                        window.handle_key(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            ));
        let weak = window.downgrade();
        window
            .subscriptions
            .borrow_mut()
            .push(area.add_disposable_handler_with(
                InputElement::key_up_event(),
                move |_, _| {
                    if let Some(window) = weak.upgrade() {
                        window.refresh();
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            ));
        let weak = window.downgrade();
        window
            .subscriptions
            .borrow_mut()
            .push(window.closed(move || {
                if let Some(window) = weak.upgrade() {
                    window.closed.set(true);
                    window.detach_from_editor();
                }
            }));
        editor.set_value(Self::active_window_property(), Some(window.clone()));
        window.refresh();
        if !window.is_closed() {
            window.open();
        }
        Ok(window)
    }
    fn editor(&self) -> Option<Ref<TextEditor>> {
        self.editor.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub fn is_closed(&self) -> bool {
        self.closed.get()
    }
    pub fn active_for(editor: &TextEditor) -> Option<Ref<Self>> {
        editor
            .get_value(Self::active_window_property())
            .filter(|window| !window.is_closed())
    }
    fn detach_from_editor(&self) {
        if let Some(editor) = self.editor() {
            if editor
                .get_value(Self::active_window_property())
                .is_some_and(|window| window == self.to_ref())
            {
                editor.clear_value(Self::active_window_property());
            }
        }
    }
    pub fn close_window(&self) {
        self.closed.set(true);
        if let Some(session) = self.session.borrow_mut().as_mut() {
            session.close();
        }
        self.close();
        self.detach_from_editor();
        if let Some(editor) = self.editor() {
            editor.text_area().focus();
        }
    }
    pub fn selected_item(&self) -> Option<CompletionItem> {
        self.session
            .borrow()
            .as_ref()
            .and_then(|s| s.list.selected_item().cloned())
    }
    pub fn request_insertion(&self) -> Result<bool, DocumentError> {
        if self.is_closed() {
            return Ok(false);
        }
        let Some(editor) = self.editor() else {
            return Ok(false);
        };
        if editor.is_read_only() {
            return Err(DocumentError::ReadOnly);
        }
        // Take the session out before the document raises editor events.
        let Some(mut session) = self.session.borrow_mut().take() else {
            return Ok(false);
        };
        if session.document() != &editor.document() {
            session.close();
        } else {
            if let Err(error) = session.caret_moved(editor.caret_offset()) {
                *self.session.borrow_mut() = Some(session);
                return Err(error);
            }
        }
        if session.is_closed() {
            *self.session.borrow_mut() = Some(session);
            self.close_window();
            return Ok(false);
        }
        let range = session.segment();
        let item = session.list.selected_item().cloned();
        let result = if let (Some(range), Some(item)) = (range, item) {
            let text = item.insertion_text.as_deref().unwrap_or(&item.text);
            let plan = [ferroui_edit_core::document::DocumentEdit::new(
                range.offset,
                range.length,
                text,
            )];
            let applied =
                editor
                    .session()
                    .apply_document_edits(&plan)
                    .map_err(|error| match error {
                        ferroui_edit_core::editing::EditError::Document(error) => error,
                        _ => DocumentError::ReadOnly,
                    });
            match applied {
                Ok(()) => {
                    let _ = editor.set_caret_offset(range.offset + text.encode_utf16().count());
                    session.close();
                    Ok(true)
                }
                Err(error) => Err(error),
            }
        } else {
            Ok(false)
        };
        *self.session.borrow_mut() = Some(session);
        if result.as_ref().is_ok_and(|changed| *changed) {
            self.close_window();
        }
        result
    }
    pub fn refresh(&self) {
        if self.is_closed() || self.refreshing.get() {
            return;
        }
        let Some(editor) = self.editor() else {
            self.close_window();
            return;
        };
        let (items, selected, closed) = {
            let mut session = self.session.borrow_mut();
            let Some(session) = session.as_mut() else {
                return;
            };
            if session.document() != &editor.document() {
                session.close();
            }
            if let Err(error) = session.caret_moved(editor.caret_offset()) {
                editor.report_error(error.to_string());
                session.close();
            }
            (
                session
                    .list
                    .current_list()
                    .map(|i| i.text.clone())
                    .collect::<Vec<_>>(),
                session.list.selected_index(),
                session.is_closed(),
            )
        };
        if closed {
            self.close_window();
            return;
        }
        self.refreshing.set(true);
        if let Some(list) = self.list.get() {
            list.items().clear();
            for item in items {
                list.items().add(Some(Rc::new(item)));
            }
            list.set_selected_index(selected.map_or(-1, |i| i as i32));
        }
        self.refreshing.set(false);
        self.refresh_description();
        self.set_placement_rect(Some(editor.text_area().cursor_rectangle()));
    }
    fn refresh_description(&self) {
        let description = self
            .selected_item()
            .map(|i| i.description)
            .unwrap_or_default();
        if let Some(control) = self.description.get() {
            control.set_text(Some(&description));
            control.set_is_visible(!description.is_empty());
        }
    }
    fn handle_key(&self, e: &KeyEventArgs) {
        if e.handled() || self.is_closed() {
            return;
        }
        match e.key {
            Key::Escape => {
                self.close_window();
                e.set_handled(true);
            }
            Key::Return | Key::Tab => match self.request_insertion() {
                Ok(true) => e.set_handled(true),
                Err(error) => {
                    if let Some(editor) = self.editor() {
                        editor.report_error(error.to_string());
                    }
                    e.set_handled(true);
                }
                _ => {}
            },
            Key::Up | Key::Down | Key::PageUp | Key::PageDown => {
                {
                    let mut session = self.session.borrow_mut();
                    if let Some(session) = session.as_mut() {
                        session.list.move_selection(
                            match e.key {
                                Key::Up => -1,
                                Key::Down => 1,
                                Key::PageUp => -8,
                                _ => 8,
                            },
                            matches!(e.key, Key::Up | Key::Down),
                        );
                    }
                }
                let selected = self
                    .session
                    .borrow()
                    .as_ref()
                    .and_then(|s| s.list.selected_index());
                if let Some(list) = self.list.get() {
                    list.set_selected_index(selected.map_or(-1, |i| i as i32));
                }
                self.refresh_description();
                e.set_handled(true);
            }
            _ => {}
        }
    }
    fn build_content(&self) {
        let content = StackPanel::new();
        content.set_spacing(6.0);
        let list = ListBox::new();
        list.set_min_width(280.0);
        list.set_max_height(250.0);
        content
            .children()
            .add(list.clone().upcast::<ferroui_controls::Control>());
        let description = TextBlock::new();
        description.set_max_width(480.0);
        content
            .children()
            .add(description.clone().upcast::<ferroui_controls::Control>());
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
            .push(self.add_disposable_handler_with(
                InputElement::key_down_event(),
                move |_, event: &KeyEventArgs| {
                    if let Some(window) = weak.upgrade() {
                        window.handle_key(event);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            ));
        let weak = self.to_ref().downgrade();
        let source = list.downgrade();
        self.subscriptions
            .borrow_mut()
            .push(list.add_disposable_handler(
                SelectingItemsControl::selection_changed_event(),
                move |_, _| {
                    if let (Some(window), Some(list)) = (weak.upgrade(), source.upgrade()) {
                        if window.refreshing.get() {
                            return;
                        }
                        if list.selected_index() >= 0 {
                            if let Some(session) = window.session.borrow_mut().as_mut() {
                                session.list.select_index(list.selected_index() as usize);
                            }
                        }
                        window.refresh_description();
                    }
                },
            ));
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(
                list.add_disposable_handler(InputElement::double_tapped_event(), move |_, e| {
                    if let Some(window) = weak.upgrade() {
                        match window.request_insertion() {
                            Ok(true) => e.set_handled(true),
                            Err(error) => {
                                if let Some(editor) = window.editor() {
                                    editor.report_error(error.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }),
            );
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
