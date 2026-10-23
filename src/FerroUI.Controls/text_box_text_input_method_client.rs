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
    preedit_generation: Cell<u64>,
    preparing_preedit: RefCell<Option<PreparingPreedit>>,
    confirmed_preedit: RefCell<Option<ConfirmedPreedit>>,
    selection_changed: Cell<bool>,
    is_in_change: Cell<bool>,
    parent_property_changed: RefCell<Option<Rc<dyn IDisposable>>>,
    parent_tapped: RefCell<Option<RoutedEventHandlerToken>>,
    caret_bounds_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

/// Selection removal and restoration can synchronously receive IME updates.
/// Keep only the latest until the owned text/caret change has finished.
struct PreparingPreedit {
    generation: u64,
    restoring: bool,
    update: PreeditUpdate,
}

struct PreeditUpdate {
    generation: u64,
    text: Option<String>,
    cursor: Option<i32>,
}

#[derive(Clone, Copy)]
pub(crate) enum PreeditSelectionError {
    Superseded,
    TextChanged,
    SelectionChanged,
}

/// Native preedit may provisionally remove the selection from `Text`. Keep the
/// confirmed value until an explicit clear/commit, or restore it when this
/// client detaches while composition is still owned by the input method.
struct ConfirmedPreedit {
    text: Option<String>,
    provisional_text: Option<String>,
    caret: i32,
    selection: TextSelection,
    superseded: Option<PreeditSelectionError>,
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
        Self { text, provisional_text, caret: parent.caret_index(), selection, superseded: None, styles: parent.text_style_overrides(),
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
        let generation = self.next_preedit_generation();
        let (Some(parent), Some(presenter)) = (self.parent(), self.presenter()) else {
            self.pending_preedit.set(false);
            self.preparing_preedit.borrow_mut().take();
            self.confirmed_preedit.borrow_mut().take();
            return;
        };
        let mut update = PreeditUpdate { generation, text: preedit_text.map(str::to_owned), cursor: cursor_pos };
        if preedit_text.is_some() {
            // A host edit superseded this composition during preparation.
            // Ignore its later clauses until the explicit clear; they must
            // not provisionally delete another range from the host's text.
            if self.confirmed_preedit.borrow().as_ref().is_some_and(|value| value.superseded.is_some()) {
                return;
            }
        }
        {
            let mut preparing = self.preparing_preedit.borrow_mut();
            if let Some(preparing) = preparing.as_mut() {
                if preedit_text.is_some() || preparing.restoring {
                    preparing.update = update;
                    return;
                }
            }
        }
        if preedit_text.is_none() {
            // Clearing during selection removal cancels it immediately. A
            // clause received during restoration waits for a fresh snapshot.
            self.preparing_preedit.borrow_mut().take();
            self.clear_preedit(update, &parent, &presenter);
            return;
        }

        // Own composition before provisional TextChanging notifications. An
        // empty update is pending too, but does not remove the selection yet.
        if !self.pending_preedit.get() {
            *self.confirmed_preedit.borrow_mut() = Some(ConfirmedPreedit::capture(&parent));
        }
        if !self.owns_preedit_update(generation, &parent, &presenter) {
            return;
        }
        self.pending_preedit.set(preedit_text.is_some());

        // Remove the selection before publishing the first visible clause:
        // its Text/Caret synchronization deliberately clears existing preedit.
        // Reentrant clauses share this removal; cancellation, template/focus
        // changes and host-owned text/selection changes invalidate it.
        if preedit_text.is_some_and(|text| !text.is_empty())
            && presenter.preedit_text().is_none_or(|text| text.is_empty())
        {
            *self.preparing_preedit.borrow_mut() = Some(PreparingPreedit { generation, restoring: false, update });
            let owns_preparation = || self.owns_preedit_preparation(generation, &parent, &presenter);
            let preparation = parent.prepare_preedit_selection(&owns_preparation);
            let Some(prepared) = self.take_preedit_preparation(generation) else {
                return;
            };
            if !self.owns_preedit_update(prepared.generation, &parent, &presenter) {
                return;
            }
            let text_generation = match preparation {
                Ok(generation) => generation,
                Err(error @ (PreeditSelectionError::SelectionChanged | PreeditSelectionError::TextChanged)) => {
                    // A host edit supersedes this composition until clear.
                    // Keep its chosen text or range instead of restoring the
                    // obsolete confirmed value over that newer decision.
                    if let Some(confirmed) = self.confirmed_preedit.borrow_mut().as_mut() {
                        confirmed.superseded = Some(error);
                    }
                    return;
                }
                Err(PreeditSelectionError::Superseded) => return,
            };
            if parent.text_change_generation() != text_generation {
                return;
            }
            update = prepared;
        }
        let text_generation = parent.text_change_generation();
        presenter.set_current_value(TextPresenter::preedit_text_property(), update.text.clone());
        if self.owns_preedit_update(update.generation, &parent, &presenter)
            && parent.text_change_generation() == text_generation
            && presenter.preedit_text() == update.text
        {
            presenter.set_current_value(TextPresenter::preedit_text_cursor_position_property(), update.cursor);
        }
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
            preedit_generation: Cell::new(0),
            preparing_preedit: RefCell::new(None),
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

    fn next_preedit_generation(&self) -> u64 {
        let generation = self.preedit_generation.get().wrapping_add(1);
        self.preedit_generation.set(generation);
        generation
    }

    fn owns_preedit_update(&self, generation: u64, parent: &Ref<TextBox>, presenter: &Ref<TextPresenter>) -> bool {
        self.preedit_generation.get() == generation
            && self.parent().as_ref() == Some(parent)
            && self.presenter().as_ref() == Some(presenter)
    }

    fn owns_preedit_preparation(&self, generation: u64, parent: &Ref<TextBox>, presenter: &Ref<TextPresenter>) -> bool {
        self.preparing_preedit.borrow().as_ref().is_some_and(|value| value.generation == generation)
            && self.parent().as_ref() == Some(parent)
            && self.presenter().as_ref() == Some(presenter)
    }

    fn take_preedit_preparation(&self, generation: u64) -> Option<PreeditUpdate> {
        let mut preparing = self.preparing_preedit.borrow_mut();
        if preparing.as_ref().is_some_and(|value| value.generation == generation) {
            preparing.take().map(|value| value.update)
        } else {
            None
        }
    }

    fn clear_preedit(&self, update: PreeditUpdate, parent: &Ref<TextBox>, presenter: &Ref<TextPresenter>) {
        let generation = update.generation;
        let cursor = update.cursor;
        *self.preparing_preedit.borrow_mut() = Some(PreparingPreedit { generation, restoring: true, update });
        // Restore before clearing pending: TextChanging observers must still
        // treat this as provisional. A following native TextInput then replaces
        // the original selection and reports its exact native edit range.
        // Release the cell before a notification can replace the template.
        let confirmed = self.confirmed_preedit.borrow_mut().take();
        if let Some(confirmed) = confirmed {
            self.restore_confirmed(parent, confirmed);
        }
        if !self.owns_preedit_preparation(generation, parent, presenter) {
            return;
        }
        self.pending_preedit.set(false);
        let text_generation = parent.text_change_generation();
        presenter.set_current_value(TextPresenter::preedit_text_property(), None);
        if self.owns_preedit_preparation(generation, parent, presenter)
            && parent.text_change_generation() == text_generation && presenter.preedit_text().is_none() {
            presenter.set_current_value(TextPresenter::preedit_text_cursor_position_property(), cursor);
        }
        let Some(latest) = self.take_preedit_preparation(generation) else {
            return;
        };
        if !self.owns_preedit_update(latest.generation, parent, presenter) {
            return;
        }
        if let Some(text) = latest.text {
            // A newer clause arrived while the old confirmed text/selection
            // was being restored. It starts a new composition only now, after
            // the complete restoration, and captures its own confirmed state.
            self.set_preedit_text_with_cursor(Some(&text), latest.cursor);
        } else if parent.text_change_generation() == text_generation && presenter.preedit_text().is_none() {
            presenter.set_current_value(TextPresenter::preedit_text_cursor_position_property(), latest.cursor);
        }
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
        if matches!(confirmed.superseded, Some(PreeditSelectionError::TextChanged))
            || confirmed.styles.as_ref() != styles || confirmed.text == confirmed.provisional_text
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
        if matches!(confirmed.superseded, Some(PreeditSelectionError::TextChanged))
            || confirmed.paragraph_styles.as_ref() != styles || confirmed.text == confirmed.provisional_text
            || parent.text() != confirmed.provisional_text { return None; }
        let start = confirmed.selection.start.min(confirmed.selection.end).max(0) as usize;
        let end = confirmed.selection.start.max(confirmed.selection.end).max(0) as usize;
        Some((ReadOnlyMemory::<u16>::from_str(confirmed.text.as_deref().unwrap_or_default()), start..end))
    }

    fn restore_confirmed(&self, parent: &TextBox, confirmed: ConfirmedPreedit) {
        if matches!(confirmed.superseded, Some(PreeditSelectionError::TextChanged)) { return; }
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
        if confirmed.superseded.is_none() && parent.text_change_generation() == generation && parent.text() == confirmed.text
            && (parent.caret_index(), parent.selection_start(), parent.selection_end()) == selection {
            parent.set_current_value(TextBox::caret_index_property(), confirmed.caret);
            parent.set_current_value(TextBox::selection_start_property(), confirmed.selection.start);
            parent.set_current_value(TextBox::selection_end_property(), confirmed.selection.end);
        }
    }

    pub(crate) fn set_presenter(&self, presenter: Option<&Ref<TextPresenter>>, parent: Option<&Ref<TextBox>>) {
        let generation = self.next_preedit_generation();
        self.preparing_preedit.borrow_mut().take();
        let confirmed = self.confirmed_preedit.borrow_mut().take();
        if self.pending_preedit.get() {
            *self.preparing_preedit.borrow_mut() = Some(PreparingPreedit {
                generation, restoring: true, update: PreeditUpdate { generation, text: None, cursor: None },
            });
            if let (Some(old_parent), Some(confirmed)) = (self.parent(), confirmed) {
                // A host may already have restored or replaced Text. Never
                // overwrite that later value with an obsolete IME snapshot.
                self.restore_confirmed(&old_parent, confirmed);
            }
            // Updates received while detaching still belong to the old input
            // owner. Restore its confirmed text, then discard those updates.
            self.take_preedit_preparation(generation);
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
