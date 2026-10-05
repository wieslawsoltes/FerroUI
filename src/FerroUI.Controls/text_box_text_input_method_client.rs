use crate::presenters::TextPresenter;
use crate::TextBox;
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
    /// The last presenter: the contract of the base crate has no "no text
    /// view" value, so the visual of the last presenter is reported while
    /// the client has none.
    last_text_view_visual: RefCell<Option<WeakRef<TextPresenter>>>,
    selection_changed: Cell<bool>,
    is_in_change: Cell<bool>,
    parent_property_changed: RefCell<Option<Rc<dyn IDisposable>>>,
    parent_tapped: RefCell<Option<RoutedEventHandlerToken>>,
    caret_bounds_changed: RefCell<Option<Rc<dyn IDisposable>>>,
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

    fn text_view_visual(&self) -> Ref<Visual> {
        let presenter = self.presenter().or_else(|| {
            let last = self.last_text_view_visual.borrow().clone();
            last.and_then(|last| last.upgrade())
        });

        match presenter {
            Some(presenter) => presenter.upcast(),
            None => panic!("The text input method client has no text view visual."),
        }
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

        presenter.get_cursor_rectangle().transform_to_aabb(transform)
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

        parent.set_selection_start(value.start);
        parent.set_selection_end(value.end);

        self.raise_selection_changed();
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
        let (Some(_parent), Some(presenter)) = (self.parent(), self.presenter()) else {
            return;
        };

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
            last_text_view_visual: RefCell::new(None),
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

    pub(crate) fn set_presenter(&self, presenter: Option<&Ref<TextPresenter>>, parent: Option<&Ref<TextBox>>) {
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
            *self.last_text_view_visual.borrow_mut() = Some(presenter.downgrade());

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
