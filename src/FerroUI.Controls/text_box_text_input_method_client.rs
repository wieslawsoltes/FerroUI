use crate::presenters::{TextPresenter, TextStyleOverrides};
use crate::TextBox;
use ferroui_base::media::text_formatting::TextParagraphStyles;
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::input::text_input::{
    ContextMenuAction, TextInputMethodClient, TextInputMethodClientEvents, TextSelection,
};
use ferroui_base::input::{InputElement, TappedEventArgs};
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroPropertyChangedEventArgs, Rect, Ref, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The text input method client of a [`TextBox`]: what the text box gives
/// the input method of the platform to work with.
///
/// The text box owns its client, so the client only holds weak handles to
/// the text box and to its presenter.
pub struct TextBoxTextInputMethodClient {
    this: Weak<TextBoxTextInputMethodClient>,
    events: TextInputMethodClientEvents,
    parent: RefCell<Option<WeakRef<TextBox>>>,
    presenter: RefCell<Option<WeakRef<TextPresenter>>>,
    pending_preedit: Cell<bool>,
    confirmed_preedit: RefCell<Option<ConfirmedPreedit>>,
    selection_changed: Cell<bool>,
    is_in_change: Cell<bool>,
    parent_property_changed: RefCell<Option<Rc<dyn IDisposable>>>,
    parent_tapped: RefCell<Option<RoutedEventHandlerToken>>,
    caret_bounds_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

/// Native preedit may provisionally remove the selection from `Text`. Keep the
/// confirmed value until an explicit clear/commit, or restore it when this
/// client detaches while composition is still owned by the input method.
struct ConfirmedPreedit {
    text: Option<String>,
    provisional_text: Option<String>,
    caret: i32,
    selection: TextSelection,
    styles: Option<TextStyleOverrides>,
    paragraph_styles: Option<TextParagraphStyles>,
}

impl ConfirmedPreedit {
    fn capture(parent: &TextBox) -> Self {
        let text = parent.text();
        let selection = TextSelection::new(parent.selection_start(), parent.selection_end());
        let mut units: Vec<_> = text.as_deref().unwrap_or_default().encode_utf16().collect();
        let start = selection.start.min(selection.end).max(0) as usize;
        let end = (selection.start.max(selection.end).max(0) as usize).min(units.len());
        let provisional_text = if start < end && !parent.is_read_only() {
            units.drain(start..end);
            Some(String::from_utf16_lossy(&units))
        } else { text.clone() };
        Self { text, provisional_text, caret: parent.caret_index(), selection, styles: parent.text_style_overrides(),
            paragraph_styles: parent.text_paragraph_styles() }
    }
}

/// The scope of a change of a text box (see
/// [`TextBoxTextInputMethodClient::begin_change`]): selection changes made
/// while it lives are reported once, when it is dropped.
pub(crate) struct TextInputMethodChange(Option<Rc<TextBoxTextInputMethodClient>>);

impl Drop for TextInputMethodChange {
    fn drop(&mut self) {
        if let Some(client) = self.0.take() {
            client.raise_events();
        }
    }
}

impl TextInputMethodClient for TextBoxTextInputMethodClient {
    fn events(&self) -> &TextInputMethodClientEvents {
        &self.events
    }

    fn text_view_visual(&self) -> Option<Ref<Visual>> {
        self.presenter().map(Ref::upcast)
    }

    fn surrounding_text(&self) -> String {
        let Some(parent) = self.parent() else {
            return String::new();
        };

        let presenter = self.presenter();

        if let Some(presenter) = &presenter {
            let caret_index = parent.caret_index();

            if caret_index != presenter.caret_index() {
                presenter.set_current_value(TextPresenter::caret_index_property(), caret_index);
            }
        }

        let text = parent.text();

        if let Some(presenter) = &presenter {
            if text != presenter.text() {
                presenter.set_current_value(TextPresenter::text_property(), text.clone());
            }
        }

        text.unwrap_or_default()
    }

    fn cursor_rectangle(&self) -> Rect {
        let (Some(parent), Some(presenter)) = (self.parent(), self.presenter()) else {
            return Rect::default();
        };

        let Some(transform) = presenter.transform_to_visual(&parent) else {
            return Rect::default();
        };

        presenter.cursor_rectangle().transform_to_aabb(transform)
    }

    fn selection(&self) -> TextSelection {
        let Some(parent) = self.parent() else {
            return TextSelection::default();
        };

        TextSelection::new(parent.selection_start(), parent.selection_end())
    }

    fn set_selection(&self, value: TextSelection) {
        let Some(parent) = self.parent() else {
            return;
        };

        let _change = self.begin_change();
        parent.set_selection_start(value.start);
        parent.set_selection_end(value.end);
    }

    fn supports_preedit(&self) -> bool {
        true
    }

    fn supports_surrounding_text(&self) -> bool {
        true
    }

    fn set_preedit_text(&self, preedit_text: Option<&str>) {
        self.set_preedit_text_with_cursor(preedit_text, None)
    }

    fn set_preedit_text_with_cursor(&self, preedit_text: Option<&str>, cursor_pos: Option<i32>) {
        let (Some(parent), Some(presenter)) = (self.parent(), self.presenter()) else {
            self.pending_preedit.set(false);
            self.confirmed_preedit.borrow_mut().take();
            return;
        };

        // Publishing preedit can synchronously delete the TextBox selection.
        // Its caret/text synchronization can then clear the presenter's visual
        // preedit before TextChanging runs. Keep the input client's state until
        // the input method explicitly clears it, including an empty update.
        if preedit_text.is_some() && !self.pending_preedit.get() {
            *self.confirmed_preedit.borrow_mut() = Some(ConfirmedPreedit::capture(&parent));
        } else if preedit_text.is_none() {
            // Clearing a composition may mean cancellation or may precede its
            // committed TextInput. Restore the confirmed selection first, so
            // either case uses its real text/range rather than a provisional
            // deletion. Keep pending true during synchronous notifications.
            // Take the snapshot before property writes: callbacks can reenter.
            let confirmed = self.confirmed_preedit.borrow_mut().take();
            if let Some(confirmed) = confirmed {
                self.restore_confirmed(&parent, confirmed);
            }
        }
        self.pending_preedit.set(preedit_text.is_some());
        presenter.set_current_value(TextPresenter::preedit_text_property(), preedit_text.map(str::to_owned));
        presenter.set_current_value(TextPresenter::preedit_text_cursor_position_property(), cursor_pos);
    }

    fn execute_context_menu_action(&self, action: ContextMenuAction) {
        let Some(parent) = self.parent() else {
            return;
        };

        match action {
            ContextMenuAction::Copy => parent.copy(),
            ContextMenuAction::Cut => parent.cut(),
            ContextMenuAction::Paste => parent.paste(),
            ContextMenuAction::SelectAll => parent.select_all(),
        }
    }
}

impl TextBoxTextInputMethodClient {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            events: TextInputMethodClientEvents::new(),
            parent: RefCell::new(None),
            presenter: RefCell::new(None),
            pending_preedit: Cell::new(false),
            confirmed_preedit: RefCell::new(None),
            selection_changed: Cell::new(false),
            is_in_change: Cell::new(false),
            parent_property_changed: RefCell::new(None),
            parent_tapped: RefCell::new(None),
            caret_bounds_changed: RefCell::new(None),
        })
    }

    fn parent(&self) -> Option<Ref<TextBox>> {
        let parent = self.parent.borrow().clone();
        parent.and_then(|parent| parent.upgrade())
    }

    fn presenter(&self) -> Option<Ref<TextPresenter>> {
        let presenter = self.presenter.borrow().clone();
        presenter.and_then(|presenter| presenter.upgrade())
    }

    pub(crate) fn has_pending_preedit(&self) -> bool {
        self.pending_preedit.get()
    }

    /// Style ranges still refer to confirmed text while the native TextBox has
    /// provisionally deleted the selection. Do not remap a collection that the
    /// application has already replaced in response to that temporary change.
    pub(crate) fn preedit_removed_style_range(&self, styles: Option<&TextStyleOverrides>) -> Option<(i32, i32)> {
        if !self.pending_preedit.get() { return None; }
        let parent = self.parent()?;
        let confirmed = self.confirmed_preedit.borrow();
        let confirmed = confirmed.as_ref()?;
        if confirmed.styles.as_ref() != styles || confirmed.text == confirmed.provisional_text
            || parent.text() != confirmed.provisional_text { return None; }
        let start = confirmed.selection.start.min(confirmed.selection.end);
        let end = confirmed.selection.start.max(confirmed.selection.end);
        Some((start, end - start))
    }

    pub(crate) fn preedit_paragraph_source(&self, styles: Option<&TextParagraphStyles>)
        -> Option<(ReadOnlyMemory<u16>, std::ops::Range<usize>)> {
        if !self.pending_preedit.get() { return None; }
        let parent = self.parent()?;
        let confirmed = self.confirmed_preedit.borrow();
        let confirmed = confirmed.as_ref()?;
        if confirmed.paragraph_styles.as_ref() != styles || confirmed.text == confirmed.provisional_text
            || parent.text() != confirmed.provisional_text { return None; }
        let start = confirmed.selection.start.min(confirmed.selection.end).max(0) as usize;
        let end = confirmed.selection.start.max(confirmed.selection.end).max(0) as usize;
        Some((ReadOnlyMemory::<u16>::from_str(confirmed.text.as_deref().unwrap_or_default()), start..end))
    }

    fn restore_confirmed(&self, parent: &TextBox, confirmed: ConfirmedPreedit) {
        if parent.text() != confirmed.provisional_text || confirmed.text == confirmed.provisional_text { return; }
        let _change = self.begin_change();
        let styles = parent.text_style_overrides();
        let paragraphs = parent.text_paragraph_styles();
        let selection = (parent.caret_index(), parent.selection_start(), parent.selection_end());
        let generation = parent.text_change_generation().wrapping_add(1);
        parent.set_text_from_internal_synchronization(confirmed.text.clone());
        // TextChanging and style notifications are synchronous. Respect a
        // handler's replacement instead of applying an obsolete IME snapshot.
        if parent.text_change_generation() != generation || parent.text() != confirmed.text { return; }
        if parent.text_style_overrides() == styles {
            parent.set_current_value(TextBox::text_style_overrides_property(), confirmed.styles);
        }
        if parent.text_change_generation() != generation || parent.text() != confirmed.text { return; }
        if parent.text_paragraph_styles() == paragraphs {
            parent.set_current_value(TextBox::text_paragraph_styles_property(), confirmed.paragraph_styles);
        }
        if parent.text_change_generation() == generation && parent.text() == confirmed.text
            && (parent.caret_index(), parent.selection_start(), parent.selection_end()) == selection {
            parent.set_current_value(TextBox::caret_index_property(), confirmed.caret);
            parent.set_current_value(TextBox::selection_start_property(), confirmed.selection.start);
            parent.set_current_value(TextBox::selection_end_property(), confirmed.selection.end);
        }
    }

    pub(crate) fn set_presenter(&self, presenter: Option<&Ref<TextPresenter>>, parent: Option<&Ref<TextBox>>) {
        let confirmed = self.confirmed_preedit.borrow_mut().take();
        if self.pending_preedit.get() {
            if let (Some(old_parent), Some(confirmed)) = (self.parent(), confirmed) {
                // A host may already have restored or replaced Text. Never
                // overwrite that later value with an obsolete IME snapshot.
                self.restore_confirmed(&old_parent, confirmed);
            }
        }
        self.pending_preedit.set(false);
        let property_changed = self.parent_property_changed.borrow_mut().take();
        let tapped = self.parent_tapped.borrow_mut().take();

        if let Some(old_parent) = self.parent() {
            if let Some(property_changed) = property_changed {
                property_changed.dispose();
            }

            if let Some(tapped) = tapped {
                old_parent.remove_handler(InputElement::tapped_event(), tapped);
            }
        }

        *self.parent.borrow_mut() = parent.map(Ref::downgrade);

        if let Some(parent) = parent {
            let weak = self.this.clone();
            let property_changed = parent.property_changed(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.on_parent_property_changed(e);
                }
            });
            *self.parent_property_changed.borrow_mut() = Some(property_changed);

            let weak = self.this.clone();
            let tapped = parent.add_handler(InputElement::tapped_event(), move |_, _: &TappedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_parent_tapped();
                }
            });
            *self.parent_tapped.borrow_mut() = Some(tapped);
        }

        let old_presenter = self.presenter();
        let caret_bounds_changed = self.caret_bounds_changed.borrow_mut().take();

        if let Some(old_presenter) = old_presenter {
            old_presenter.set_current_im_client(None);
            old_presenter.clear_value(TextPresenter::preedit_text_property());

            if let Some(caret_bounds_changed) = caret_bounds_changed {
                caret_bounds_changed.dispose();
            }
        }

        *self.presenter.borrow_mut() = presenter.map(Ref::downgrade);

        if let Some(presenter) = presenter {
            presenter.set_current_im_client(self.this.upgrade());

            let weak = self.this.clone();
            let caret_bounds_changed = presenter.caret_bounds_changed(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_presenter_caret_bounds_changed();
                }
            });
            *self.caret_bounds_changed.borrow_mut() = Some(caret_bounds_changed);
        }

        self.raise_text_view_visual_changed();

        self.raise_cursor_rectangle_changed();
    }

    fn on_presenter_caret_bounds_changed(&self) {
        self.raise_cursor_rectangle_changed();
    }

    fn on_parent_tapped(&self) {
        self.raise_input_pane_activation_requested();
    }

    fn on_parent_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let property = e.property();

        if property == TextBox::text_property().as_property() {
            self.raise_surrounding_text_changed();
        }

        if property == TextBox::selection_start_property().as_property()
            || property == TextBox::selection_end_property().as_property()
        {
            if self.is_in_change.get() {
                self.selection_changed.set(true);
            } else {
                self.raise_selection_changed();
            }
        }
    }

    /// Starts a change of the text box: the selection changes made until
    /// the returned scope is dropped are reported as one.
    pub(crate) fn begin_change(&self) -> TextInputMethodChange {
        if self.is_in_change.get() {
            return TextInputMethodChange(None);
        }

        self.is_in_change.set(true);
        TextInputMethodChange(self.this.upgrade())
    }

    fn raise_events(&self) {
        self.is_in_change.set(false);

        if self.selection_changed.get() {
            self.raise_selection_changed();
        }

        self.selection_changed.set(false);
    }
}
