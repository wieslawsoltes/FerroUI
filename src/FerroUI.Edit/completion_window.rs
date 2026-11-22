use crate::completion_data::{completion_edit_error, ICompletionData};
use crate::completion_list::CompletionList;
use crate::completion_tip_content_control::CompletionTipContentControl;
use crate::completion_window_base::CompletionWindowBaseImplExt;
use crate::completion_window_base::{CompletionWindowBase, CompletionWindowBaseImpl};
use crate::TextEditor;
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, PointerWheelEventArgs,
    TextInputEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, AttachedProperty,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    Point, Rect, StyledElementImpl, StyledProperty, Thickness, VisualImpl,
};
use ferroui_controls::primitives::Popup;
use ferroui_controls::{Border, Control, ControlImpl, PlacementMode};
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
    description_popup: OnceCell<Ref<Popup>>,
    surface: OnceCell<Ref<Border>>,
    refreshing: Cell<bool>,
    fixed_choices: Cell<bool>,
    subscriptions: RefCell<Vec<Subscription>>,
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
            this.refresh();
        }
    }
}
impl InputElementImpl for CompletionWindow {
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        Self::parent_on_key_down(this, event);
        if this.fixed_choices.get()
            && matches!(event.key, Key::Tab | Key::Return)
            && !event.key_modifiers.is_empty()
        {
            return;
        }
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
        if let Some(popup) = this.description_popup.get() {
            popup.close();
        }
        if let Some(presenter) = this.description.get() {
            presenter.set_is_visible(false);
            presenter.set_content(None);
        }
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
            description_popup: OnceCell::new(),
            surface: OnceCell::new(),
            refreshing: Cell::new(false),
            fixed_choices: Cell::new(false),
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

    pub(crate) fn show_choices(
        editor: &Ref<TextEditor>,
        start: usize,
        end: usize,
        items: Vec<Rc<dyn ICompletionData>>,
        selected: usize,
    ) -> Result<Ref<Self>, DocumentError> {
        let window = Self::new();
        window.fixed_choices.set(true);
        let list = window.completion_list();
        list.set_is_filtering(false);
        list.set_completion_data(items);
        list.set_selected_index(Some(selected));
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
        let list = self.completion_list();
        let property = CompletionList::completion_accept_action_property();
        let action = editor.completion_accept_action();
        if list.completion_accept_action() != action && !list.is_set(property.as_property()) {
            if let Some(value) = list.set_value_with_priority(
                property,
                action,
                BindingPriority::Style,
            ) {
                self.track_subscription(Subscription::new(value));
            }
        }
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
            move |sender, event| {
                // The text area owns IME commits. Its document change will
                // update the suggestions after the input method succeeds.
                if event.is_from_input_method()
                    || sender.downcast_ref::<crate::TextArea>()
                        .is_some_and(|area| area.has_preedit() || area.is_hosted_child_event(event))
                {
                    return;
                }
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
            move |sender, event| {
                if sender.downcast_ref::<crate::TextArea>()
                    .is_some_and(|area| area.is_hosted_child_event(event))
                {
                    return;
                }
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
        let target = crate::text_editor::EditTarget::new(&editor);
        let selection = editor.selection();
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
        if !target.is_current(&editor) || editor.selection() != selection {
            return Err(DocumentError::ReentrantChange);
        }
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
        if !self.fixed_choices.get() {
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
        }
        let empty = self.completion_list().current_list().is_empty();
        self.refresh_description();
        self.refreshing.set(false);
        if empty && self.close_automatically() {
            self.dismiss();
        }
    }
    fn refresh_description(&self) {
        if self.is_closed() {
            return;
        }
        if let Some(control) = self.description.get() {
            let content = self
                .selected_completion_data()
                .and_then(|item| item.description());
            if !self.is_closed() {
                control.set_is_visible(content.is_some());
                control.set_content(content);
                self.update_description_position();
            }
        }
    }
    fn update_description_position(&self) {
        let (Some(popup), Some(presenter), Some(surface)) = (
            self.description_popup.get(),
            self.description.get(),
            self.surface.get(),
        ) else {
            return;
        };
        let list = self.completion_list().list_box();
        let index = list.selected_index();
        if self.is_closed() || !self.is_open() || index < 0 || presenter.content().is_none() {
            popup.close();
            return;
        }
        let bounds = surface.bounds();
        if bounds.width <= 0.0 || bounds.height <= 0.0 || !surface.is_effectively_visible() {
            popup.close();
            return;
        }
        let row_y = list.container_from_index(index).and_then(|row| {
            row.translate_point(Point::default(), surface)
                .map(|point| point.y)
        });
        let (placement, top) = match row_y {
            Some(y) if y < bounds.height => (PlacementMode::RightEdgeAlignedTop, y.max(0.0)),
            Some(_) => (PlacementMode::RightEdgeAlignedBottom, 0.0),
            None if list.first_visible_item() < index as usize => {
                (PlacementMode::RightEdgeAlignedBottom, 0.0)
            }
            None => (PlacementMode::RightEdgeAlignedTop, 0.0),
        };
        let anchor = Some(Rect::new(0.0, top, bounds.width, bounds.height - top));
        if popup.placement() != placement {
            popup.set_placement(placement);
        }
        if popup.placement_rect() != anchor {
            popup.set_placement_rect(anchor);
        }
        if !popup.is_open() && !self.is_closed() && self.is_open() {
            popup.open();
        }
    }
    fn apply_editor_style(&self) {
        if let (Some(border), Some(editor)) = (self.surface.get(), self.editor()) {
            crate::search_panel::style_overlay(border, &editor);
            if let Some(tip) = self
                .description_popup
                .get()
                .and_then(|popup| popup.child())
                .and_then(|child| child.cast::<Border>())
            {
                crate::search_panel::style_overlay(&tip, &editor);
            }
        }
    }
    fn build_content(&self) {
        let list = CompletionList::new();
        let description = CompletionTipContentControl::new();
        description.set_max_width(480.0);
        description.set_is_visible(false);
        let border = Border::new();
        border.set_padding(Thickness::uniform(6.0));
        border.set_child(list.clone());
        let tip = Border::new();
        tip.set_child(description.clone());
        let popup = Popup::new();
        popup.set_child(tip);
        popup.set_placement_target(border.clone());
        popup.set_popup_parent(Some(&self.to_ref().upcast::<Control>()));
        popup.set_should_use_overlay_layer(true);
        popup.set_horizontal_offset(2.0);
        popup.set_is_hit_test_visible(false);
        popup.set_takes_focus_from_native_control(false);
        let weak = self.to_ref().downgrade();
        self.subscriptions.borrow_mut().push(border.layout_updated(move || {
            if let Some(window) = weak.upgrade() {
                window.update_description_position();
            }
        }));
        let weak = self.to_ref().downgrade();
        self.subscriptions.borrow_mut().push(self.opened(move || {
            if let Some(window) = weak.upgrade() {
                window.update_description_position();
            }
        }));
        self.surface.set(border.clone()).ok();
        self.set_child(border);
        self.list.set(list.clone()).ok();
        self.description.set(description).ok();
        self.description_popup.set(popup).ok();
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
                        if window.selected_completion_data().is_none() {
                            window.close_window();
                            event.set_handled(true);
                            return;
                        }
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
    use crate::CompletionAcceptAction;
    use ferroui_base::input::Key;
    use ferroui_base::layout::{ILayoutManager, Layoutable};
    use ferroui_base::styling::{ControlTheme, Setter};
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::presenters::ItemsPresenter;
    use ferroui_controls::templates::{FuncControlTemplate, FuncTemplate, FuncTemplateNameScopeExtensions};
    use ferroui_controls::{Canvas, ListBoxItem, StackPanel, Window};

    #[test]
    fn completion_windows_snapshot_the_markup_action_and_preserve_list_overrides() {
        use ferroui_base::metadata::from_markup_value;

        let _scope = crate::tests::test_scope();
        crate::register_types();
        ferroui_markup_xaml::register_types();
        let markup = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
            r#"<edit:TextEditor xmlns="https://github.com/ferroui" xmlns:edit="using:FerroUI.Edit" CompletionAcceptAction="PointerReleased"/>"#,
            Some(&crate::ASSEMBLY), None, None, false,
        ).unwrap();
        let editor = from_markup_value::<Ref<TextEditor>>(&Some(markup)).unwrap();
        assert_eq!(
            TextEditor::new().completion_accept_action(),
            CompletionAcceptAction::PointerPressed
        );
        let list_action = CompletionList::completion_accept_action_property();
        let window = CompletionWindow::new();
        let list = window.completion_list();
        list.set_completion_data(vec![Rc::new(CompletionItem::new("Read"))]);
        window.attach_to_editor(&editor, 0, 0).unwrap();
        assert_eq!(list.completion_accept_action(), CompletionAcceptAction::PointerReleased);

        editor.set_completion_accept_action(CompletionAcceptAction::DoubleTapped);
        assert_eq!(list.completion_accept_action(), CompletionAcceptAction::PointerReleased);
        window.attach_to_editor(&editor, 0, 0).unwrap();
        assert_eq!(list.completion_accept_action(), CompletionAcceptAction::DoubleTapped);

        window.dismiss();
        assert!(!list.is_set(list_action.as_property()));
        list.set_completion_accept_action(CompletionAcceptAction::PointerPressed);
        window.attach_to_editor(&editor, 0, 0).unwrap();
        assert_eq!(list.completion_accept_action(), CompletionAcceptAction::PointerPressed);
        window.dismiss();
        assert!(list.is_set(list_action.as_property()));
    }

    #[test]
    fn accept_dismisses_an_unmatched_unfiltered_list_without_editing() {
        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("miss");
        editor.set_caret_offset(4).unwrap();
        editor.document().undo_stack().clear_all().unwrap();
        let completion = CompletionWindow::new();
        let list = completion.completion_list();
        list.set_is_filtering(false);
        list.set_completion_data(vec![
            Rc::new(CompletionItem::new("Read")),
            Rc::new(CompletionItem::new("Write")),
        ]);
        completion.attach_to_editor(&editor, 0, 4).unwrap();
        completion.refresh();
        assert_eq!(list.current_list().len(), 2);
        assert!(list.selected_item().is_none());

        let mut accept = KeyEventArgs::new();
        accept.key = Key::Return;
        accept.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&accept);
        assert!(accept.handled());
        assert!(completion.is_closed());
        assert_eq!(editor.text(), "miss");
        assert!(!editor.undo().unwrap());
    }

    #[test]
    fn editor_release_action_commits_on_the_realized_completion_row() {
        use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
        use ferroui_base::input::{IInputManager, InputManager, MouseDevice, RawInputModifiers};
        use ferroui_controls::testing::TestServices;

        let input: Rc<dyn IInputManager> = Rc::new(InputManager::new());
        let _scope = crate::tests::test_scope_with_services(
            TestServices::styled_window().with_input_manager(input.clone()),
        );
        let editor = crate::tests::laid_out_editor("Re");
        editor.set_caret_offset(2).unwrap();
        editor.set_completion_accept_action(CompletionAcceptAction::PointerReleased);
        let root = Window::new();
        root.set_width(500.0);
        root.set_height(240.0);
        root.set_content(Some(Control::boxed(editor.clone())));
        root.show();
        editor.text_area().focus();
        let completion =
            CompletionWindow::show_for(&editor, 0, 2, vec![CompletionItem::new("Read")]).unwrap();
        let list = completion.completion_list().list_box();
        list.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let presenter = ItemsPresenter::new();
            presenter.set_name(Some("PART_ItemsPresenter".into()));
            presenter.set_items_panel(FuncTemplate::new(|| Some(StackPanel::new().upcast())));
            presenter.register_in_name_scope(&**scope).upcast()
        })));
        for _ in 0..3 {
            root.layout_manager().execute_layout_pass();
            Dispatcher::ui_thread().run_jobs(None);
        }
        let row = list.container_from_index(0).unwrap();
        assert!(row.bounds().width > 0.0 && row.bounds().height > 0.0);
        let point = row.translate_point(Point::new(4.0, row.bounds().height / 2.0), &root).unwrap();
        let mouse = MouseDevice::new();
        let send = |timestamp, event, modifiers| {
            input.process_input(Rc::new(RawPointerEventArgs::new(
                mouse.clone(), timestamp, root.input_root(), event, point, modifiers,
            )));
            Dispatcher::ui_thread().run_jobs(None);
        };
        send(1, RawPointerEventType::LeftButtonDown, RawInputModifiers::LEFT_MOUSE_BUTTON);
        assert_eq!(editor.text(), "Re");
        assert!(completion.is_open());
        send(2, RawPointerEventType::LeftButtonUp, RawInputModifiers::NONE);
        assert_eq!(editor.text(), "Read");
        assert!(completion.is_closed());
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), "Re");
        root.close();
    }

    #[test]
    fn description_popup_tracks_the_selected_row_and_closes_with_the_list() {
        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("");
        let root = Window::new();
        root.set_width(800.0);
        root.set_height(480.0);
        editor.set_width(300.0);
        editor.set_height(400.0);
        let canvas = Canvas::new();
        canvas.children().add(editor.clone().upcast::<Control>());
        root.set_content(Some(Control::boxed(canvas)));
        root.show();
        editor.text_area().focus();
        let focused = editor.text_area().is_focused();
        let entry = |text: &str, docs: Option<&str>| {
            Rc::new(CompletionData {
                text: text.into(),
                description: docs.map(|docs| Rc::new(docs.to_owned()) as ferroui_base::BoxedValue),
                ..CompletionData::default()
            }) as Rc<dyn ICompletionData>
        };
        let completion = CompletionWindow::show_with_data(&editor, 0, 0, vec![
            entry("Alpha", Some("Alpha documentation")),
            entry("Beta", Some("Beta documentation")),
            entry("Gamma", None),
        ]).unwrap();
        let list = completion.completion_list().list_box();
        list.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let presenter = ItemsPresenter::new();
            presenter.set_name(Some("PART_ItemsPresenter".into()));
            presenter.set_items_panel(FuncTemplate::new(|| Some(StackPanel::new().upcast())));
            presenter.register_in_name_scope(&**scope).upcast()
        })));
        list.set_item_container_theme(Some(ControlTheme::with_setters(
            ListBoxItem::TYPE,
            [Setter::new(Layoutable::min_height_property(), 24.0)],
        )));
        let layout = || {
            for _ in 0..3 {
                root.layout_manager().execute_layout_pass();
                Dispatcher::ui_thread().run_jobs(None);
            }
        };
        layout();

        let tip = completion.description_popup.get().unwrap();
        let surface = completion.surface.get().unwrap();
        assert_eq!(completion.opened_popups(), [tip.clone()]);
        assert!(tip.is_using_overlay_layer());
        assert!(!tip.host().unwrap().as_control().is_hit_test_visible());
        let surface_right = surface.translate_point(Point::new(surface.bounds().width, 0.0), &root).unwrap().x;
        let tip_left = tip.host().unwrap().as_control().translate_point(Point::default(), &root).unwrap().x;
        assert!(tip_left >= surface_right + 1.0, "tip at {tip_left}, list ends at {surface_right}");
        assert_eq!(editor.text_area().is_focused(), focused);
        let first_anchor = tip.placement_rect().unwrap();

        list.select_index(1);
        layout();
        let row = list.container_from_index(1).unwrap();
        let row_top = row.translate_point(Point::default(), surface).unwrap().y;
        let anchor = tip.placement_rect().unwrap();
        assert!(row_top > first_anchor.y);
        assert_eq!(anchor.y, row_top);
        assert_eq!(completion.description_presenter().content().unwrap().downcast_ref::<String>().unwrap(), "Beta documentation");

        list.select_index(2);
        layout();
        assert!(!tip.is_open());
        assert!(completion.is_open());
        assert!(completion.description_presenter().content().is_none());
        list.select_index(0);
        layout();
        assert!(tip.is_open());

        Canvas::set_left(&editor, 490.0);
        layout();
        let surface_left = surface.translate_point(Point::default(), &root).unwrap().x;
        let tip_host = tip.host().unwrap().as_control();
        let tip_right = tip_host.translate_point(Point::new(tip_host.bounds().width, 0.0), &root).unwrap().x;
        assert!(surface_left > 490.0);
        assert!(tip_right <= surface_left + 2.0, "tip ends at {tip_right}, list starts at {surface_left}");

        completion.dismiss();
        assert!(completion.is_closed());
        assert!(completion.opened_popups().is_empty());
        assert!(!tip.is_open());
        assert!(completion.description_presenter().content().is_none());
        root.close();
    }

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
    fn completion_anchors_to_the_opening_side_of_a_soft_wrap() {
        use ferroui_base::{Rect, Size};

        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor(&"abc def ghi ".repeat(9));
        let area = editor.text_area();
        editor.set_word_wrap(true);
        area.measure(Size::new(95.0, 260.0));
        area.arrange(Rect::new(0.0, 0.0, 95.0, 260.0));
        let wrap = (1..editor.document().text_length())
            .find(|&offset| area.rectangle_for_offset(offset).is_some_and(|rect| rect.y > 0.0))
            .unwrap();
        editor.set_caret_offset(wrap).unwrap();
        let leading = editor.caret_position();
        let leading_rect = area.rectangle_for_offset(wrap).unwrap();
        editor
            .set_caret_position(crate::TextViewPosition {
                is_at_end_of_line: true,
                ..leading
            })
            .unwrap();
        let trailing_rect = area.cursor_rectangle();
        assert!(leading_rect.y > trailing_rect.y);

        let popup = CompletionWindow::new();
        popup.completion_list()
            .set_completion_data(vec![Rc::new(CompletionItem::new("completion"))]);
        popup.attach_to_editor(&editor, wrap, wrap + 1).unwrap();
        let opening_rect = popup.placement_rect().unwrap();
        assert_eq!(
            (opening_rect.x, opening_rect.y),
            (trailing_rect.x, trailing_rect.y)
        );

        editor.set_caret_position(leading).unwrap();
        assert!(popup.update_position());
        assert_eq!(popup.placement_rect(), Some(opening_rect));

        popup.attach_to_editor(&editor, wrap, wrap + 1).unwrap();
        assert_eq!(popup.placement_rect(), Some(leading_rect));
    }

    #[test]
    fn insight_retains_the_opening_virtual_cells_at_the_same_offset() {
        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("abc");
        let area = editor.text_area();
        let offset = 3;
        editor.set_caret_offset(offset).unwrap();
        let physical = editor.caret_position();
        let physical_rect = area.rectangle_for_offset(offset).unwrap();
        editor
            .set_caret_position(crate::TextViewPosition {
                visual_column: physical.visual_column.map(|column| column + 2),
                ..physical
            })
            .unwrap();
        let virtual_rect = area.cursor_rectangle();
        assert!(virtual_rect.x > physical_rect.x);

        let insight = crate::InsightWindow::show_for(&editor, offset, offset, None).unwrap();
        let opening_rect = insight.placement_rect().unwrap();
        assert_eq!(opening_rect.x, virtual_rect.x);
        editor.set_caret_position(physical).unwrap();
        assert!(insight.update_position());
        assert_eq!(insight.placement_rect(), Some(opening_rect));
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
