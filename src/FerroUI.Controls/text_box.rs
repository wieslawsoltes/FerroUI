use crate::presenters::TextPresenter;
use crate::primitives::{
    ScrollBarVisibility, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
};
use crate::text_box_text_input_method_client::TextBoxTextInputMethodClient;
use crate::utils::{ClipboardHelper, IUndoRedoHost, PrimarySelectionHelper, StringUtils, UndoRedoHelper};
use crate::{
    Application, Border, ContentControl, ControlImpl, Decorator, PastingFromClipboardEventArgs, ScrollViewer,
    SelectableTextBlock, TextBlock, TextChangedEventArgs, TextChangingEventArgs, TopLevel,
};
use ferroui_base::input::platform::{ClipboardError, ClipboardExtensions, ClipboardType, IClipboard};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use std::any::Any;
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::{BindingMode, BindingPriority};
use ferroui_base::input::{
    ContextRequestedEventArgs, FocusChangedEventArgs, HoldingRoutedEventArgs, HoldingState, InputElement,
    InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyGesture, KeyModifiers, MouseButton, NavigationMethod,
    PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, TappedEventArgs,
    TextInputEventArgs,
};
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl, LayoutableImplExt, VerticalAlignment};
use ferroui_base::media::text_formatting::unicode::GraphemeEnumerator;
use ferroui_base::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, ITextSource, LogicalDirection, TextEndOfLine,
    TextLayout, TextParagraphProperties, TextRun, TextRunProperties,
};
use ferroui_base::media::{
    BaselineAlignment, CharacterHit, FlowDirection, IBrush, TextAlignment, TextWrapping, Typeface,
};
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::reactive::{CompositeDisposable, IDisposable, IObserver, Observable, ObservableExt};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTask};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, AttachedProperty, BoxedValue,
    DirectProperty, FerroLocator, FerroObject, LocatorExtensions, VisualImplExt, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyMetadata, StyledPropertyOptions, Thickness, Visual, VisualImpl, VisualTreeAttachmentEventArgs,
    WeakRef,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// Stores the state information for the available actions of the undo/redo
/// helper.
///
/// Two states are equal when their texts are: the caret position does not
/// take part in the comparison.
#[derive(Clone, Debug)]
pub(crate) struct UndoRedoState {
    text: Option<String>,
    caret_position: i32,
}

impl UndoRedoState {
    fn new(text: Option<String>, caret_position: i32) -> Self {
        Self { text, caret_position }
    }
}

impl PartialEq for UndoRedoState {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextMutationKind {
    ExternalReplacement,
    Edit,
    InternalSynchronization,
}

/// The characters removed from every text input.
const INVALID_CHARACTERS: [u16; 1] = [0x007f];

const MAX_CHARS_BEFORE_UNDO_SNAPSHOT: i32 = 7;

const CARRIAGE_RETURN: u16 = b'\r' as u16;
const LINE_FEED: u16 = b'\n' as u16;
const SPACE: u16 = b' ' as u16;

/// Represents a control that can be used to display or edit unformatted
/// text.
///
/// Template parts: `PART_TextPresenter` (a [`TextPresenter`], required) and
/// `PART_ScrollViewer` (a [`ScrollViewer`]). Pseudo-classes: `:empty`,
/// `:touch-mode`.
///
/// Every text position (`CaretIndex`, `SelectionStart`, `SelectionEnd`,
/// `MaxLength`) is a UTF-16 code unit index.
#[repr(C)]
pub struct TextBox {
    base: TemplatedControl,
    /// The radius for touch input. Used to determine if the selection should
    /// change from moving a touch pointer.
    touch_radius: OnceCell<i32>,
    presenter: RefCell<Option<Ref<TextPresenter>>>,
    scroll_viewer: RefCell<Option<Ref<ScrollViewer>>>,
    im_client: Rc<TextBoxTextInputMethodClient>,
    undo_redo_helper: UndoRedoHelper<UndoRedoState>,
    /// The handle through which the undo/redo helper reaches this text box.
    undo_redo_host: RefCell<Option<Rc<dyn IUndoRedoHost<UndoRedoState>>>>,
    is_undoing_redoing: Cell<bool>,
    text_mutation_kind: Cell<TextMutationKind>,
    // Coercion runs before the new value is committed, so a snapshot taken there would capture the old text.
    needs_undo_redo_snapshot_after_text_change: Cell<bool>,
    can_cut: Cell<bool>,
    can_copy: Cell<bool>,
    can_paste: Cell<bool>,
    can_undo: Cell<bool>,
    can_redo: Cell<bool>,

    word_selection_start: Cell<i32>,
    selection_at_pointer_press: Cell<(i32, i32)>,
    selected_text_changes_made_since_last_undo_snapshot: Cell<i32>,
    has_done_snapshot_once: Cell<bool>,
    current_click_count: Cell<i32>,
    is_double_tapped: Cell<bool>,
    is_in_touch_mode: Cell<bool>,
    last_point: Cell<Point>,
    is_in_touch_selection_mode: Cell<bool>,
    is_in_touch_caret_mode: Cell<bool>,
    has_touch_selection: Cell<bool>,
    /// The subscription of `presenter_property_changed` to the presenter.
    presenter_property_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    TextBox: TemplatedControl, virtuals TextBoxImpl: TemplatedControlImpl {
        /// Coerces the current text.
        ///
        /// This method also manages the internal undo/redo state whenever
        /// the text changes: if overridden, ensure that the base is called
        /// or undo/redo won't work correctly.
        fn coerce_text(this, value: Option<String>) -> Option<String>;
    }
}
ferroui_base::ferro_class_info!(TextBox { new: TextBox::new });

ferro_impl_classes!(TextBox: StyledElementImpl, InteractiveImpl);

impl ControlImpl for TextBox {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::TextBoxAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for TextBox {
    fn constructed(this: &Self) {
        // The static constructor runs before the instance constructors.
        Self::parent_constructed(this);

        let object: &FerroObject = this;
        let accepts_return = FerroObjectExtensions::get_observable(object, Self::accepts_return_property());
        let text_wrapping = FerroObjectExtensions::get_observable(object, Self::text_wrapping_property());

        // The latest value of each property combined into the visibility of
        // the horizontal scroll bar, once both have produced a value.
        let horizontal_scroll_bar_visibility =
            Observable::create(move |observer: Rc<dyn IObserver<ScrollBarVisibility>>| {
                let latest: Rc<(Cell<Option<bool>>, Cell<Option<TextWrapping>>)> =
                    Rc::new((Cell::new(None), Cell::new(None)));

                let publish = {
                    let latest = latest.clone();
                    move || {
                        if let (Some(accepts_return), Some(wrapping)) = (latest.0.get(), latest.1.get()) {
                            observer.on_next(if wrapping != TextWrapping::NoWrap {
                                ScrollBarVisibility::Disabled
                            } else if accepts_return {
                                ScrollBarVisibility::Auto
                            } else {
                                ScrollBarVisibility::Hidden
                            });
                        }
                    }
                };

                let accepts_return = {
                    let (latest, publish) = (latest.clone(), publish.clone());
                    accepts_return.subscribe_fn(move |value| {
                        latest.0.set(Some(value));
                        publish();
                    })
                };

                let text_wrapping = text_wrapping.subscribe_fn(move |value| {
                    latest.1.set(Some(value));
                    publish();
                });

                let subscription: Rc<dyn IDisposable> =
                    Rc::new(CompositeDisposable::from_disposables([accepts_return, text_wrapping]));
                subscription
            });

        FerroObjectExtensions::bind_typed(
            object,
            ScrollViewer::horizontal_scroll_bar_visibility_property(),
            horizontal_scroll_bar_visibility,
            BindingPriority::Style,
        );

        let host: Rc<dyn IUndoRedoHost<UndoRedoState>> = Rc::new(TextBoxUndoRedoHost(this.to_ref().downgrade()));
        this.undo_redo_helper.set_host(Rc::downgrade(&host));
        *this.undo_redo_host.borrow_mut() = Some(host);

        this.selected_text_changes_made_since_last_undo_snapshot.set(0);
        this.has_done_snapshot_once.set(false);
        this.update_command_states();
        this.update_pseudoclasses();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Self::text_property().as_property() {
            if this.needs_undo_redo_snapshot_after_text_change.get() {
                this.needs_undo_redo_snapshot_after_text_change.set(false);
                this.snapshot_undo_redo(true);
            }

            this.coerce_value(Self::caret_index_property().as_property());
            this.coerce_value(Self::selection_start_property().as_property());
            this.coerce_value(Self::selection_end_property().as_property());

            this.raise_text_change_events();

            this.update_pseudoclasses();
            this.update_command_states();
        } else if property == Self::is_read_only_property().as_property()
            || property == Self::password_char_property().as_property()
            || property == Self::reveal_password_property().as_property()
        {
            this.update_command_states();
        } else if property == Self::caret_index_property().as_property() {
            this.on_caret_index_changed(change);
        } else if property == Self::selection_start_property().as_property() {
            this.on_selection_start_changed(change);
        } else if property == Self::selection_end_property().as_property() {
            this.on_selection_end_changed(change);
        } else if property == Self::max_lines_property().as_property() {
            this.invalidate_measure();
        } else if property == Self::min_lines_property().as_property() {
            this.invalidate_measure();
        } else if property == Self::undo_limit_property().as_property() {
            this.on_undo_limit_changed(change.get_new_value::<i32>());
        } else if property == Self::is_undo_enabled_property().as_property() && !change.get_new_value::<bool>() {
            // "Setting this property to false clears the undo stack.
            // Therefore, if you disable undo and then re-enable it, undo commands still do not work
            // because the undo stack was emptied when you disabled undo."
            this.clear_undo_redo();
        }
    }
}

impl VisualImpl for TextBox {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if let Some(presenter) = this.presenter() {
            if this.is_focused() {
                presenter.show_caret();
            } else if this.is_inactive_selection_highlight_enabled() {
                presenter.set_show_selection_highlight(true);
            }

            let weak = this.to_ref().downgrade();
            let subscription = presenter.property_changed(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.presenter_property_changed(e);
                }
            });

            let previous = this.presenter_property_changed.borrow_mut().replace(subscription);

            if let Some(previous) = previous {
                previous.dispose();
            }
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        if let Some(presenter) = this.presenter() {
            presenter.hide_caret();

            let subscription = this.presenter_property_changed.borrow_mut().take();

            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }

        this.im_client.set_presenter(None, None);
    }
}

impl LayoutableImpl for TextBox {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        if let Some(scroll_viewer) = this.scroll_viewer() {
            let mut max_height = f64::INFINITY;

            let max_lines = this.max_lines();

            if max_lines > 0 && this.height().is_nan() {
                let text_layout = this.create_lines_text_layout(max_lines);
                let vertical_space = this.get_vertical_space_between_scroll_viewer_and_presenter();

                max_height = (text_layout.height() + vertical_space).ceil();
            }

            scroll_viewer.set_current_value(Layoutable::max_height_property(), max_height);

            let mut min_height = 0.0;

            let min_lines = this.min_lines();

            if min_lines > 0 && this.height().is_nan() {
                let text_layout = this.create_lines_text_layout(min_lines);
                let vertical_space = this.get_vertical_space_between_scroll_viewer_and_presenter();

                min_height = (text_layout.height() + vertical_space).ceil();
            }

            scroll_viewer.set_current_value(Layoutable::min_height_property(), min_height);
        }

        Self::parent_measure_override(this, available_size)
    }
}

impl InputElementImpl for TextBox {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);

        let presenter = this.presenter();

        if let Some(presenter) = &presenter {
            presenter.set_show_selection_highlight(true);
        }

        // When navigating to a text box via the tab key, select all text if
        //   1) this text box is *not* a multiline text box
        //   2) this text box has any text to select
        if e.navigation_method == NavigationMethod::Tab
            && !this.accepts_return()
            && this.text().is_some_and(|text| !text.is_empty())
        {
            this.select_all();
        }

        this.update_command_states();

        let presenter = this.presenter();

        this.im_client.set_presenter(presenter.as_ref(), Some(&this.to_ref()));

        if let Some(presenter) = &presenter {
            presenter.show_caret();
        }

        if this.selection_start() != this.selection_end() {
            if let Some(canvas) = presenter.as_ref().and_then(|presenter| presenter.text_selection_handle_canvas()) {
                canvas.show(false);
            }
        }
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);

        if !this.context_flyout().is_some_and(|flyout| flyout.is_open())
            && !this.context_menu().is_some_and(|menu| menu.is_open())
        {
            if this.clear_selection_on_lost_focus() {
                this.clear_selection();
            }

            this.set_current_value(Self::reveal_password_property(), false);
            if let Some(presenter) = this.presenter() {
                presenter.remove_text_selection_canvas();
            }
        }

        this.update_command_states();

        let presenter = this.presenter();

        if let Some(presenter) = &presenter {
            presenter.hide_caret();
        }

        this.im_client.set_presenter(None, None);

        if let Some(presenter) = &presenter {
            if !this.is_inactive_selection_highlight_enabled() {
                presenter.set_show_selection_highlight(false);
            }
        }
    }

    fn on_text_input(this: &Self, e: &TextInputEventArgs) {
        if !e.handled() {
            this.handle_text_input(e.text.as_deref());
            e.set_handled(true);
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        let Some(presenter) = this.presenter() else {
            return;
        };

        if presenter.preedit_text().is_some_and(|preedit_text| !preedit_text.is_empty()) {
            return;
        }

        let mut text = this.text_utf16();
        let caret_index = this.caret_index();
        let mut movement = false;
        let mut selection = false;
        let mut handled = false;
        let modifiers = e.key_modifiers;

        let keymap = Application::current()
            .expect("the application")
            .platform_settings()
            .expect("the platform settings of the application")
            .hotkey_configuration();

        let _change = this.im_client.begin_change();

        let matches = |gestures: &[KeyGesture]| gestures.iter().any(|g| g.matches(Some(e)));
        let detect_selection = || e.key_modifiers.contains(keymap.selection_modifiers);

        if matches(&keymap.select_all) {
            this.select_all();
            handled = true;
        } else if matches(&keymap.copy) {
            if this.can_copy() {
                this.copy();
            }

            handled = true;
        } else if matches(&keymap.cut) {
            if this.can_cut() {
                this.cut();
            }

            handled = true;
        } else if matches(&keymap.paste) {
            if this.can_paste() {
                this.paste();
            }

            handled = true;
        } else if matches(&keymap.undo) && this.is_undo_enabled() {
            if !this.is_read_only() {
                this.undo();
            }

            handled = true;
        } else if matches(&keymap.redo) && this.is_undo_enabled() {
            if !this.is_read_only() {
                this.redo();
            }

            handled = true;
        } else if matches(&keymap.move_cursor_to_the_start_of_document) {
            this.move_home(true);
            movement = true;
            selection = false;
            handled = true;
            this.set_current_value(Self::caret_index_property(), presenter.caret_index());
        } else if matches(&keymap.move_cursor_to_the_end_of_document) {
            this.move_end(true);
            movement = true;
            selection = false;
            handled = true;
            this.set_current_value(Self::caret_index_property(), presenter.caret_index());
        } else if matches(&keymap.move_cursor_to_the_start_of_line) {
            this.move_home(false);
            movement = true;
            selection = false;
            handled = true;
            this.set_current_value(Self::caret_index_property(), presenter.caret_index());
        } else if matches(&keymap.move_cursor_to_the_end_of_line) {
            this.move_end(false);
            movement = true;
            selection = false;
            handled = true;
            this.set_current_value(Self::caret_index_property(), presenter.caret_index());
        } else if matches(&keymap.move_cursor_to_the_start_of_document_with_selection) {
            this.set_current_value(Self::selection_start_property(), caret_index);
            this.move_home(true);
            this.set_current_value(Self::selection_end_property(), presenter.caret_index());
            movement = true;
            selection = true;
            handled = true;
        } else if matches(&keymap.move_cursor_to_the_end_of_document_with_selection) {
            this.set_current_value(Self::selection_start_property(), caret_index);
            this.move_end(true);
            this.set_current_value(Self::selection_end_property(), presenter.caret_index());
            movement = true;
            selection = true;
            handled = true;
        } else if matches(&keymap.move_cursor_to_the_start_of_line_with_selection) {
            this.set_current_value(Self::selection_start_property(), caret_index);
            this.move_home(false);
            this.set_current_value(Self::selection_end_property(), presenter.caret_index());
            movement = true;
            selection = true;
            handled = true;
        } else if matches(&keymap.move_cursor_to_the_end_of_line_with_selection) {
            this.set_current_value(Self::selection_start_property(), caret_index);
            this.move_end(false);
            this.set_current_value(Self::selection_end_property(), presenter.caret_index());
            movement = true;
            selection = true;
            handled = true;
        } else if matches(&keymap.page_left) {
            this.move_page_left();
            movement = true;
            selection = false;
            handled = true;
        } else if matches(&keymap.page_right) {
            this.move_page_right();
            movement = true;
            selection = false;
            handled = true;
        } else if matches(&keymap.page_up) {
            this.move_page_up();
            movement = true;
            selection = false;
            handled = true;
        } else if matches(&keymap.page_down) {
            this.move_page_down();
            movement = true;
            selection = false;
            handled = true;
        } else {
            // It's not secure to rely on password field content when moving.
            let has_whole_word_modifiers =
                modifiers.contains(keymap.whole_word_text_action_modifiers) && !this.is_password_box();

            match e.key {
                Key::Left => {
                    selection = detect_selection();
                    this.move_horizontal(-1, has_whole_word_modifiers, selection, true);
                    if caret_index != presenter.caret_index() {
                        movement = true;
                    }
                }

                Key::Right => {
                    selection = detect_selection();
                    this.move_horizontal(1, has_whole_word_modifiers, selection, true);
                    if caret_index != presenter.caret_index() {
                        movement = true;
                    }
                }

                Key::Up => {
                    selection = detect_selection();
                    this.move_vertical(LogicalDirection::Backward, selection);
                    if caret_index != presenter.caret_index() {
                        movement = true;
                    }
                }

                Key::Down => {
                    selection = detect_selection();
                    this.move_vertical(LogicalDirection::Forward, selection);
                    if caret_index != presenter.caret_index() {
                        movement = true;
                    }
                }

                Key::Back => {
                    if !this.is_read_only() {
                        this.snapshot_undo_redo(true);

                        if has_whole_word_modifiers && this.selection_start() == this.selection_end() {
                            this.set_selection_for_control_backspace();
                        }

                        if !this.delete_selection() {
                            let character_hit = presenter.get_next_character_hit(LogicalDirection::Backward);

                            let mut backspace_position =
                                character_hit.first_character_index() + character_hit.trailing_length();

                            let text_layout = presenter.text_layout();

                            let line_index = text_layout.get_line_index_from_character_index(caret_index, true);

                            let backspace_character_hit = text_layout.text_lines()[line_index as usize]
                                .get_backspace_caret_character_hit(CharacterHit::new(caret_index));

                            if backspace_character_hit.first_character_index() > backspace_position
                                && backspace_character_hit.first_character_index() < caret_index
                            {
                                backspace_position = backspace_character_hit.first_character_index();
                            }

                            if caret_index != backspace_position {
                                let start = backspace_position.min(caret_index);
                                let end = backspace_position.max(caret_index);

                                remove_range(&mut text, start, end - start);

                                this.set_text_from_edit(Some(String::from_utf16_lossy(&text)));

                                this.set_current_value(Self::caret_index_property(), start);

                                presenter.move_caret_to_text_position(start, false);
                            }
                        }

                        this.snapshot_undo_redo(true);
                    }

                    handled = true;
                }

                Key::Delete => {
                    if !this.is_read_only() {
                        this.snapshot_undo_redo(true);

                        if has_whole_word_modifiers && this.selection_start() == this.selection_end() {
                            this.set_selection_for_control_delete();
                        }

                        if !this.delete_selection() {
                            let character_hit = presenter.get_next_character_hit(LogicalDirection::Forward);

                            let next_position =
                                character_hit.first_character_index() + character_hit.trailing_length();

                            if next_position != caret_index {
                                let start = next_position.min(caret_index);
                                let end = next_position.max(caret_index);

                                remove_range(&mut text, start, end - start);

                                this.set_text_from_edit(Some(String::from_utf16_lossy(&text)));
                            }
                        }

                        this.snapshot_undo_redo(true);
                    }

                    handled = true;
                }

                Key::Enter => {
                    if this.accepts_return() {
                        if !this.is_read_only() {
                            this.snapshot_undo_redo(true);
                            this.handle_text_input(Some(&this.new_line()));
                        }

                        handled = true;
                    }
                }

                Key::Tab => {
                    if this.accepts_tab() {
                        if !this.is_read_only() {
                            this.snapshot_undo_redo(true);
                            this.handle_text_input(Some("\t"));
                        }

                        handled = true;
                    } else {
                        Self::parent_on_key_down(this, e);
                    }
                }

                Key::Space => {
                    if !this.is_read_only() {
                        this.snapshot_undo_redo(true); // always snapshot in between words
                    }
                }

                _ => {
                    handled = false;
                }
            }
        }

        if movement && !selection {
            this.clear_selection();
        }

        if handled || movement {
            e.set_handled(true);
        }
    }

    fn on_holding(this: &Self, e: &HoldingRoutedEventArgs) {
        Self::parent_on_holding(this, e);

        let presenter = match this.presenter() {
            Some(presenter) if e.holding_state() == HoldingState::Started => presenter,
            _ => {
                this.is_in_touch_selection_mode.set(e.holding_state() == HoldingState::Canceled);
                this.has_touch_selection.set(false);
                return;
            }
        };

        let text = this.text();

        let _change = this.im_client.begin_change();

        if let Some(text) = text {
            let text: Vec<u16> = text.encode_utf16().collect();
            let position = e.pointer_event_args().get_position(Some(&presenter));
            let selection_start = this.selection_start();
            let selection_end = this.selection_end();
            presenter.move_caret_to_point(position);
            let caret_index = presenter.caret_index();
            let is_in_selection =
                selection_start != selection_end && caret_index >= selection_start && caret_index <= selection_end;

            if is_in_selection {
                presenter.raise_event(&ContextRequestedEventArgs::from_pointer_event_args(e.pointer_event_args()));
            } else {
                // We select the current held word, or the whole hidden content.
                if this.is_password_box() {
                    this.word_selection_start.set(-1);

                    this.select_all();
                } else {
                    this.select_word(&text, caret_index, caret_index, caret_index);
                }

                presenter.ensure_text_selection_layer();

                if this.selection_start() != this.selection_end() {
                    if let Some(canvas) = presenter.text_selection_handle_canvas() {
                        canvas.show(true);
                    }
                } else {
                    presenter
                        .raise_event(&ContextRequestedEventArgs::from_pointer_event_args(e.pointer_event_args()));
                }
            }

            this.has_touch_selection.set(true);

            e.set_handled(true);
            let element: &InputElement = this;
            crate::platform::PlatformFeedbackExtensions::perform_feedback(element, crate::platform::FeedbackAction::hold());
        }
    }

    fn on_tapped(this: &Self, e: &TappedEventArgs) {
        Self::parent_on_tapped(this, e);

        if e.pointer().type_() != PointerType::Mouse {
            if let Some(presenter) = this.presenter() {
                presenter.ensure_text_selection_layer();
                if let Some(canvas) = presenter.text_selection_handle_canvas() {
                    canvas.show(false);
                }
            }
        }
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        let Some(presenter) = this.presenter() else {
            return;
        };

        let text = this.text();
        let click_info = e.get_current_point(Some(this));

        let _change = this.im_client.begin_change();

        this.is_in_touch_mode.set(false);
        this.is_in_touch_selection_mode.set(false);
        this.is_double_tapped.set(e.click_count() == 2);
        this.selection_at_pointer_press.set(this.get_selection_range());

        if let Some(text) = text.filter(|_| !click_info.pointer.captured().is_some_and(|c| c.is::<Border>())) {
            let text: Vec<u16> = text.encode_utf16().collect();

            if e.pointer().type_() == PointerType::Mouse && click_info.properties.is_left_button_pressed {
                if let Some(canvas) = presenter.text_selection_handle_canvas() {
                    canvas.hide();
                }
                this.current_click_count.set(e.click_count());
                let point = e.get_position(Some(&presenter));

                presenter.move_caret_to_point(point);

                let caret_index = presenter.caret_index();
                let click_to_select = e.key_modifiers().contains(KeyModifiers::SHIFT);
                let mut selection_start = this.selection_start();
                let mut selection_end = this.selection_end();

                match e.click_count() {
                    1 => {
                        if click_to_select {
                            if this.word_selection_start.get() >= 0 {
                                this.update_word_selection_range(
                                    caret_index,
                                    &mut selection_start,
                                    &mut selection_end,
                                );

                                this.set_current_value(Self::selection_start_property(), selection_start);
                                this.set_current_value(Self::selection_end_property(), selection_end);
                            } else {
                                this.set_current_value(Self::selection_end_property(), caret_index);
                            }
                        } else {
                            this.set_current_value(Self::selection_start_property(), caret_index);
                            this.set_current_value(Self::selection_end_property(), caret_index);
                            this.word_selection_start.set(-1);
                        }
                    }
                    2 => {
                        this.select_word(&text, caret_index, selection_start, selection_end);
                    }
                    3 => {
                        this.word_selection_start.set(-1);

                        this.select_all();
                    }
                    _ => {}
                }
            } else if e.pointer().type_() != PointerType::Mouse {
                this.is_in_touch_mode.set(true);
                this.last_point.set(e.get_current_point(Some(&presenter)).position);

                if this.is_double_tapped.get() {
                    presenter.move_caret_to_point(this.last_point.get());
                    let caret_index = presenter.caret_index();

                    let selection_start = this.selection_start();
                    let selection_end = this.selection_end();

                    this.select_word(&text, caret_index, selection_start, selection_end);
                    presenter.ensure_text_selection_layer();
                    if let Some(canvas) = presenter.text_selection_handle_canvas() {
                        canvas.show(false);
                    }
                }
            }
        }

        this.update_pseudoclasses();

        let element: Ref<InputElement> = presenter.upcast();
        e.pointer().capture(Some(&element));
        e.set_handled(true);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        let Some(presenter) = this.presenter().filter(|presenter| is_captured_by(e, presenter)) else {
            return;
        };

        let _change = this.im_client.begin_change();
        let mut point = e.get_position(Some(&presenter));

        if e.pointer().type_() == PointerType::Mouse {
            // The selection should not change during a pointer move if the user right clicks.
            if is_captured_by(e, &presenter) && e.get_current_point(Some(this)).properties.is_left_button_pressed {
                point = clamp_to_bounds(point, &presenter);

                let previous_index = presenter.caret_index();

                presenter.move_caret_to_point(point);

                let caret_index = presenter.caret_index();

                if (caret_index - previous_index).abs() == 1 {
                    e.prevent_gesture_recognition();
                }

                if e.pointer().type_() == PointerType::Mouse || this.is_double_tapped.get() {
                    let mut selection_start = this.selection_start();
                    let mut selection_end = this.selection_end();

                    if this.word_selection_start.get() >= 0 {
                        this.update_word_selection_range(caret_index, &mut selection_start, &mut selection_end);

                        this.set_current_value(Self::selection_start_property(), selection_start);
                        this.set_current_value(Self::selection_end_property(), selection_end);
                    } else {
                        this.set_current_value(Self::selection_end_property(), caret_index);
                    }
                } else {
                    this.set_current_value(Self::selection_start_property(), caret_index);
                    this.set_current_value(Self::selection_end_property(), caret_index);
                }
            }
        } else if this.is_in_touch_mode.get() {
            if this.is_in_touch_selection_mode.get() {
                point = clamp_to_bounds(point, &presenter);

                let previous_index = presenter.caret_index();

                presenter.move_caret_to_point(point);

                let caret_index = presenter.caret_index();

                if (caret_index - previous_index).abs() == 1 {
                    e.prevent_gesture_recognition();
                }

                let mut selection_start = this.selection_start();
                let mut selection_end = this.selection_end();

                if this.word_selection_start.get() >= 0 {
                    this.update_word_selection_range(caret_index, &mut selection_start, &mut selection_end);

                    this.set_current_value(Self::selection_start_property(), selection_start);
                    this.set_current_value(Self::selection_end_property(), selection_end);
                } else {
                    this.set_current_value(Self::selection_end_property(), caret_index);
                }
            } else {
                if !this.is_in_touch_caret_mode.get() {
                    let last_point = this.last_point.get();
                    let touch_rect =
                        Rect::new(last_point.x, last_point.y, 0.0, 0.0).inflate(f64::from(this.touch_radius()));
                    let is_in_rect = touch_rect.x < point.x
                        && touch_rect.y < point.y
                        && touch_rect.right() > point.x
                        && touch_rect.bottom() > point.y;

                    if !is_in_rect {
                        this.is_in_touch_caret_mode.set(true);
                    }
                }

                if this.is_in_touch_caret_mode.get() {
                    e.prevent_gesture_recognition();
                    presenter.move_caret_to_point(point);
                    let caret_index = presenter.caret_index();
                    this.set_current_value(Self::selection_start_property(), caret_index);
                    this.set_current_value(Self::selection_end_property(), caret_index);
                }
            }
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        let Some(presenter) = this.presenter() else {
            return;
        };

        if !is_captured_by(e, &presenter) {
            return;
        }

        let _change = this.im_client.begin_change();

        if e.pointer().type_() != PointerType::Mouse
            && !this.is_in_touch_selection_mode.get()
            && !this.is_double_tapped.get()
            && !this.has_touch_selection.get()
        {
            let point = e.get_position(Some(&presenter));

            presenter.move_caret_to_point(point);

            let caret_index = presenter.caret_index();
            this.set_current_value(Self::caret_index_property(), caret_index);
            this.set_current_value(Self::selection_end_property(), caret_index);
            this.set_current_value(Self::selection_start_property(), caret_index);
        }

        if e.initial_press_mouse_button() == MouseButton::Right {
            let point = e.get_position(Some(&presenter));

            presenter.move_caret_to_point(point);

            let caret_index = presenter.caret_index();

            // See if the mouse clicked inside the current selection:
            // if it did not, the selection moves to where the user clicked.
            let first_selection = this.selection_start().min(this.selection_end());
            let last_selection = this.selection_start().max(this.selection_end());
            let did_click_in_selection = this.selection_start() != this.selection_end()
                && caret_index >= first_selection
                && caret_index <= last_selection;
            if !did_click_in_selection {
                this.set_current_value(Self::caret_index_property(), caret_index);
                this.set_current_value(Self::selection_end_property(), caret_index);
                this.set_current_value(Self::selection_start_property(), caret_index);
            }
        }

        if e.initial_press_mouse_button() == MouseButton::Middle {
            // Middle-click pastes the primary selection at the click position on platforms supporting it.
            if !this.is_read_only() {
                if let Some(primary_selection) = TopLevel::get_top_level(Some(this))
                    .and_then(|top_level| top_level.try_get_clipboard(ClipboardType::PrimarySelection))
                {
                    presenter.move_caret_to_point(e.get_position(Some(&presenter)));

                    let caret_index = presenter.caret_index();
                    this.set_current_value(Self::caret_index_property(), caret_index);
                    this.set_current_value(Self::selection_start_property(), caret_index);
                    this.set_current_value(Self::selection_end_property(), caret_index);

                    // Nobody observes the outcome of this paste: a failure it
                    // does not log stays in the dropped task.
                    drop(this.paste_core_async(Some(primary_selection)));
                    e.set_handled(true);
                }
            }
        } else if e.initial_press_mouse_button() == MouseButton::Left {
            let selection = this.get_selection_range();
            if !this.is_password_box()
                && selection.0 != selection.1
                && selection != this.selection_at_pointer_press.get()
            {
                // The pointer gesture changed the selection, publish it to the primary selection.
                let target = this.to_ref();
                drop(PrimarySelectionHelper::publish_text_async(this, move || Some(target.get_selection())));
            }
        }

        this.is_in_touch_selection_mode.set(false);
        this.is_in_touch_caret_mode.set(false);
        this.has_touch_selection.set(false);
    }
}

impl TemplatedControlImpl for TextBox {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let presenter = e.name_scope().get_as::<TextPresenter>("PART_TextPresenter");
        *this.presenter.borrow_mut() = Some(presenter.clone());

        let scroll_viewer = e.name_scope().find_as::<ScrollViewer>("PART_ScrollViewer");
        *this.scroll_viewer.borrow_mut() = scroll_viewer;

        this.im_client.set_presenter(Some(&presenter), Some(&this.to_ref()));

        if this.is_focused() {
            presenter.show_caret();
        }
    }
}

impl TextBoxImpl for TextBox {
    fn coerce_text(this: &Self, value: Option<String>) -> Option<String> {
        if !this.is_undoing_redoing.get() {
            match this.text_mutation_kind.get() {
                TextMutationKind::Edit => {
                    this.snapshot_undo_redo(true);

                    if !this.undo_redo_helper.can_undo() && this.text() != value {
                        this.needs_undo_redo_snapshot_after_text_change.set(true);
                    }
                }

                TextMutationKind::InternalSynchronization => {}

                TextMutationKind::ExternalReplacement => {
                    this.clear_undo_redo();
                    this.needs_undo_redo_snapshot_after_text_change.set(true);
                }
            }
        }

        value
    }
}

/// The handle through which the undo/redo helper reaches its text box.
struct TextBoxUndoRedoHost(WeakRef<TextBox>);

impl IUndoRedoHost<UndoRedoState> for TextBoxUndoRedoHost {
    fn undo_redo_state(&self) -> UndoRedoState {
        match self.0.upgrade() {
            Some(text_box) => UndoRedoState::new(text_box.text(), text_box.caret_index()),
            None => UndoRedoState::new(None, 0),
        }
    }

    fn set_undo_redo_state(&self, value: UndoRedoState) {
        if let Some(text_box) = self.0.upgrade() {
            text_box.set_current_value(TextBox::text_property(), value.text);
            text_box.set_current_value(TextBox::caret_index_property(), value.caret_position);
            text_box.clear_selection();
        }
    }

    /// Called from the undo/redo helper when the undo stack is modified.
    fn on_undo_stack_changed(&self) {
        if let Some(text_box) = self.0.upgrade() {
            text_box.set_can_undo(text_box.undo_redo_helper.can_undo());
        }
    }

    /// Called from the undo/redo helper when the redo stack is modified.
    fn on_redo_stack_changed(&self) {
        if let Some(text_box) = self.0.upgrade() {
            text_box.set_can_redo(text_box.undo_redo_helper.can_redo());
        }
    }
}

/// The text source of a number of empty lines: it measures the height of
/// `MinLines` and `MaxLines`.
struct LineTextSource {
    lines: i32,
}

impl ITextSource for LineTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        if text_source_index >= self.lines {
            return None;
        }

        Some(Rc::new(TextEndOfLine::with_length(1)))
    }
}

impl TextBox {
    /// A platform-specific [`KeyGesture`] for the cut action.
    pub fn cut_gesture() -> Option<KeyGesture> {
        Application::current()?.platform_settings()?.hotkey_configuration().cut.first().cloned()
    }

    /// A platform-specific [`KeyGesture`] for the copy action.
    pub fn copy_gesture() -> Option<KeyGesture> {
        Application::current()?.platform_settings()?.hotkey_configuration().copy.first().cloned()
    }

    /// A platform-specific [`KeyGesture`] for the paste action.
    pub fn paste_gesture() -> Option<KeyGesture> {
        Application::current()?.platform_settings()?.hotkey_configuration().paste.first().cloned()
    }
}

ferroui_base::ferro_properties! { impl TextBox, also [
    TextBox::use_floating_placeholder_property,
    TextBox::placeholder_foreground_property,
    TextBox::new_line_property,
    TextBox::inner_left_content_property,
    TextBox::inner_right_content_property,
    TextBox::reveal_password_property,
    TextBox::can_cut_property,
    TextBox::can_copy_property,
    TextBox::can_paste_property,
    TextBox::is_undo_enabled_property,
    TextBox::undo_limit_property,
    TextBox::can_undo_property,
    TextBox::can_redo_property,
] {
    ferro_property!(
        /// Defines the `IsInactiveSelectionHighlightEnabled` property.
        pub fn is_inactive_selection_highlight_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<TextBox, _>("IsInactiveSelectionHighlightEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `ClearSelectionOnLostFocus` property.
        pub fn clear_selection_on_lost_focus_property() -> StyledProperty<bool> {
            FerroProperty::register::<TextBox, _>("ClearSelectionOnLostFocus", true)
        }
    );

    ferro_property!(
        /// Defines the `AcceptsReturn` property.
        pub fn accepts_return_property() -> StyledProperty<bool> {
            FerroProperty::register::<TextBox, _>("AcceptsReturn", false)
        }
    );

    ferro_property!(
        /// Defines the `AcceptsTab` property.
        pub fn accepts_tab_property() -> StyledProperty<bool> {
            FerroProperty::register::<TextBox, _>("AcceptsTab", false)
        }
    );

    ferro_property!(
        /// Defines the `CaretIndex` property.
        pub fn caret_index_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<TextBox, _>(
                "CaretIndex",
                StyledPropertyOptions::new(0).coerce(Self::coerce_caret_index),
            )
        }
    );

    ferro_property!(
        /// Defines the `IsReadOnly` property.
        pub fn is_read_only_property() -> StyledProperty<bool> {
            FerroProperty::register::<TextBox, _>("IsReadOnly", false)
        }
    );

    ferro_property!(
        /// Defines the `PasswordChar` property (`'\0'`: none).
        pub fn password_char_property() -> StyledProperty<char> {
            FerroProperty::register::<TextBox, _>("PasswordChar", '\0')
        }
    );

    ferro_property!(
        /// Defines the `SelectionBrush` property.
        pub fn selection_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TextBox, _>("SelectionBrush", None)
        }
    );

    ferro_property!(
        /// Defines the `SelectionForegroundBrush` property.
        pub fn selection_foreground_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TextBox, _>("SelectionForegroundBrush", None)
        }
    );

    ferro_property!(
        /// Defines the `CaretBrush` property.
        pub fn caret_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TextBox, _>("CaretBrush", None)
        }
    );

    ferro_property!(
        /// Defines the `CaretBlinkInterval` property.
        pub fn caret_blink_interval_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<TextBox, _>("CaretBlinkInterval", TimeSpan::from_milliseconds(500.0))
        }
    );

    ferro_property!(
        /// Defines the `SelectionStart` property.
        pub fn selection_start_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<TextBox, _>(
                "SelectionStart",
                StyledPropertyOptions::new(0).coerce(Self::coerce_caret_index),
            )
        }
    );

    ferro_property!(
        /// Defines the `SelectionEnd` property.
        pub fn selection_end_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<TextBox, _>(
                "SelectionEnd",
                StyledPropertyOptions::new(0).coerce(Self::coerce_caret_index),
            )
        }
    );

    ferro_property!(
        /// Defines the `MaxLength` property.
        pub fn max_length_property() -> StyledProperty<i32> {
            FerroProperty::register::<TextBox, _>("MaxLength", 0)
        }
    );

    ferro_property!(
        /// Defines the `MaxLines` property.
        pub fn max_lines_property() -> StyledProperty<i32> {
            FerroProperty::register::<TextBox, _>("MaxLines", 0)
        }
    );

    ferro_property!(
        /// Defines the `MinLines` property.
        pub fn min_lines_property() -> StyledProperty<i32> {
            FerroProperty::register::<TextBox, _>("MinLines", 0)
        }
    );

    ferro_property!(
        /// Defines the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            TextBlock::text_property().add_owner_with::<TextBox>(
                StyledPropertyMetadata::new(None)
                    .with_coerce(Self::coerce_text_callback)
                    .with_default_binding_mode(BindingMode::TwoWay)
                    .with_enable_data_validation(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `TextAlignment` property.
        pub fn text_alignment_property() -> AttachedProperty<TextAlignment> {
            TextBlock::text_alignment_property().add_owner::<TextBox>()
        }
    );

    ferro_property!(
        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            ContentControl::horizontal_content_alignment_property().add_owner::<TextBox>()
        }
    );

    ferro_property!(
        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<TextBox>()
        }
    );

    ferro_property!(
        /// Defines the `TextWrapping` property.
        pub fn text_wrapping_property() -> AttachedProperty<TextWrapping> {
            TextBlock::text_wrapping_property().add_owner::<TextBox>()
        }
    );

    ferro_property!(
        /// Defines the `LineHeight` property.
        pub fn line_height_property() -> AttachedProperty<f64> {
            TextBlock::line_height_property().add_owner_with::<TextBox>(StyledPropertyMetadata::new(Some(f64::NAN)))
        }
    );

    ferro_property!(
        /// Defines the `PlaceholderText` property.
        pub fn placeholder_text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TextBox, _>("PlaceholderText", None)
        }
    );
} }

impl TextBox {
    /// Defines the `Watermark` property.
    #[deprecated(note = "Use placeholder_text_property instead.")]
    pub fn watermark_property() -> &'static StyledProperty<Option<String>> {
        Self::placeholder_text_property()
    }

    ferro_property!(for TextBox;
        /// Defines the `UseFloatingPlaceholder` property.
        pub fn use_floating_placeholder_property() -> StyledProperty<bool> {
            FerroProperty::register::<TextBox, _>("UseFloatingPlaceholder", false)
        }
    );

    /// Defines the `UseFloatingWatermark` property.
    #[deprecated(note = "Use use_floating_placeholder_property instead.")]
    pub fn use_floating_watermark_property() -> &'static StyledProperty<bool> {
        Self::use_floating_placeholder_property()
    }

    ferro_property!(for TextBox;
        /// Defines the `PlaceholderForeground` property.
        pub fn placeholder_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TextBox, _>("PlaceholderForeground", None)
        }
    );

    /// Defines the `WatermarkForeground` property.
    #[deprecated(note = "Use placeholder_foreground_property instead.")]
    pub fn watermark_foreground_property() -> &'static StyledProperty<Option<Rc<dyn IBrush>>> {
        Self::placeholder_foreground_property()
    }

    ferro_property!(for TextBox;
        /// Defines the `NewLine` property.
        pub fn new_line_property() -> StyledProperty<String> {
            FerroProperty::register::<TextBox, _>("NewLine", environment_new_line().to_owned())
        }
    );

    ferro_property!(for TextBox;
        /// Defines the `InnerLeftContent` property.
        pub fn inner_left_content_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TextBox, _>("InnerLeftContent", None)
        }
    );

    ferro_property!(for TextBox;
        /// Defines the `InnerRightContent` property.
        pub fn inner_right_content_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TextBox, _>("InnerRightContent", None)
        }
    );

    ferro_property!(for TextBox;
        /// Defines the `RevealPassword` property.
        pub fn reveal_password_property() -> StyledProperty<bool> {
            FerroProperty::register::<TextBox, _>("RevealPassword", false)
        }
    );

    ferro_property!(for TextBox;
        /// Defines the `CanCut` property.
        pub fn can_cut_property() -> DirectProperty<TextBox, bool> {
            FerroProperty::register_direct::<TextBox, _>("CanCut", |o| o.can_cut(), None, false)
        }
    );

    ferro_property!(for TextBox;
        /// Defines the `CanCopy` property.
        pub fn can_copy_property() -> DirectProperty<TextBox, bool> {
            FerroProperty::register_direct::<TextBox, _>("CanCopy", |o| o.can_copy(), None, false)
        }
    );

    ferro_property!(for TextBox;
        /// Defines the `CanPaste` property.
        pub fn can_paste_property() -> DirectProperty<TextBox, bool> {
            FerroProperty::register_direct::<TextBox, _>("CanPaste", |o| o.can_paste(), None, false)
        }
    );

    ferro_property!(for TextBox;
        /// Defines the `IsUndoEnabled` property.
        pub fn is_undo_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<TextBox, _>("IsUndoEnabled", true)
        }
    );

    ferro_property!(for TextBox;
        /// Defines the `UndoLimit` property.
        pub fn undo_limit_property() -> StyledProperty<i32> {
            FerroProperty::register::<TextBox, _>("UndoLimit", UndoRedoHelper::<UndoRedoState>::DEFAULT_UNDO_LIMIT)
        }
    );

    ferro_property!(for TextBox;
        /// Defines the `CanUndo` property.
        pub fn can_undo_property() -> DirectProperty<TextBox, bool> {
            FerroProperty::register_direct::<TextBox, _>("CanUndo", |o| o.can_undo(), None, false)
        }
    );

    ferro_property!(for TextBox;
        /// Defines the `CanRedo` property.
        pub fn can_redo_property() -> DirectProperty<TextBox, bool> {
            FerroProperty::register_direct::<TextBox, _>("CanRedo", |o| o.can_redo(), None, false)
        }
    );

    ferro_routed_event!(
        /// Defines the `CopyingToClipboard` event.
        pub fn copying_to_clipboard_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TextBox, _>("CopyingToClipboard", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `CuttingToClipboard` event.
        pub fn cutting_to_clipboard_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TextBox, _>("CuttingToClipboard", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PastingFromClipboard` event.
        ///
        /// The event is raised with [`PastingFromClipboardEventArgs`].
        pub fn pasting_from_clipboard_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TextBox, _>("PastingFromClipboard", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `TextChanged` event.
        pub fn text_changed_event() -> RoutedEvent<TextChangedEventArgs> {
            RoutedEvent::register::<TextBox, _>("TextChanged", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `TextChanging` event.
        pub fn text_changing_event() -> RoutedEvent<TextChangingEventArgs> {
            RoutedEvent::register::<TextBox, _>("TextChanging", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<TextBox>(true);
        // Not ported: the `Auto` default of `PlatformFeedback.FeedbackType` (waits for `PlatformFeedback`).
        InputElement::text_input_method_client_requested_event().add_class_handler::<TextBox>(|tb, e| {
            if !tb.is_read_only() {
                e.set_client(Some(tb.im_client.clone()));
            }
        });
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            touch_radius: OnceCell::new(),
            presenter: RefCell::new(None),
            scroll_viewer: RefCell::new(None),
            im_client: TextBoxTextInputMethodClient::new(),
            undo_redo_helper: UndoRedoHelper::unattached(),
            undo_redo_host: RefCell::new(None),
            is_undoing_redoing: Cell::new(false),
            text_mutation_kind: Cell::new(TextMutationKind::ExternalReplacement),
            needs_undo_redo_snapshot_after_text_change: Cell::new(false),
            can_cut: Cell::new(false),
            can_copy: Cell::new(false),
            can_paste: Cell::new(false),
            can_undo: Cell::new(false),
            can_redo: Cell::new(false),
            word_selection_start: Cell::new(-1),
            selection_at_pointer_press: Cell::new((0, 0)),
            selected_text_changes_made_since_last_undo_snapshot: Cell::new(0),
            has_done_snapshot_once: Cell::new(false),
            current_click_count: Cell::new(0),
            is_double_tapped: Cell::new(false),
            is_in_touch_mode: Cell::new(false),
            last_point: Cell::new(Point::default()),
            is_in_touch_selection_mode: Cell::new(false),
            is_in_touch_caret_mode: Cell::new(false),
            has_touch_selection: Cell::new(false),
            presenter_property_changed: RefCell::new(None),
        }
    }

    /// Initializes a new instance of the `TextBox` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the text box shows a selection highlight when it is not
    /// focused.
    pub fn is_inactive_selection_highlight_enabled(&self) -> bool {
        self.get_value(Self::is_inactive_selection_highlight_enabled_property())
    }

    pub fn set_is_inactive_selection_highlight_enabled(&self, value: bool) {
        self.set_value(Self::is_inactive_selection_highlight_enabled_property(), value)
    }

    /// Whether the text box clears its selection after it loses focus.
    pub fn clear_selection_on_lost_focus(&self) -> bool {
        self.get_value(Self::clear_selection_on_lost_focus_property())
    }

    pub fn set_clear_selection_on_lost_focus(&self, value: bool) {
        self.set_value(Self::clear_selection_on_lost_focus_property(), value)
    }

    /// Whether the text box allows and displays newline or return
    /// characters.
    pub fn accepts_return(&self) -> bool {
        self.get_value(Self::accepts_return_property())
    }

    pub fn set_accepts_return(&self, value: bool) {
        self.set_value(Self::accepts_return_property(), value)
    }

    /// Whether the text box allows and displays tabs.
    pub fn accepts_tab(&self) -> bool {
        self.get_value(Self::accepts_tab_property())
    }

    pub fn set_accepts_tab(&self, value: bool) {
        self.set_value(Self::accepts_tab_property(), value)
    }

    /// The index of the text caret.
    pub fn caret_index(&self) -> i32 {
        self.get_value(Self::caret_index_property())
    }

    pub fn set_caret_index(&self, value: i32) {
        self.set_value(Self::caret_index_property(), value)
    }

    fn on_caret_index_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if self.is_undo_enabled() {
            if let Some(state) = self.undo_redo_helper.try_get_last_state() {
                if state.text == self.text() {
                    self.undo_redo_helper.update_last_state();
                }
            }
        }

        let _change = self.im_client.begin_change();

        let new_value = e.get_new_value::<i32>();
        self.set_current_value(Self::selection_start_property(), new_value);
        self.set_current_value(Self::selection_end_property(), new_value);

        if let Some(presenter) = self.presenter() {
            presenter.set_current_value(TextPresenter::caret_index_property(), new_value);
        }
    }

    /// Whether this text box is read-only.
    pub fn is_read_only(&self) -> bool {
        self.get_value(Self::is_read_only_property())
    }

    pub fn set_is_read_only(&self, value: bool) {
        self.set_value(Self::is_read_only_property(), value)
    }

    /// The character that should be used for password masking (`'\0'`:
    /// none).
    pub fn password_char(&self) -> char {
        self.get_value(Self::password_char_property())
    }

    pub fn set_password_char(&self, value: char) {
        self.set_value(Self::password_char_property(), value)
    }

    /// A brush that is used to highlight selected text.
    pub fn selection_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_brush_property())
    }

    pub fn set_selection_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_brush_property(), value)
    }

    /// A brush that is used for the foreground of selected text.
    pub fn selection_foreground_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_foreground_brush_property())
    }

    pub fn set_selection_foreground_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_foreground_brush_property(), value)
    }

    /// A brush that is used for the text caret.
    pub fn caret_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::caret_brush_property())
    }

    pub fn set_caret_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::caret_brush_property(), value)
    }

    /// The caret blink rate.
    pub fn caret_blink_interval(&self) -> TimeSpan {
        self.get_value(Self::caret_blink_interval_property())
    }

    pub fn set_caret_blink_interval(&self, value: TimeSpan) {
        self.set_value(Self::caret_blink_interval_property(), value)
    }

    /// The starting position of the text selected in the text box.
    pub fn selection_start(&self) -> i32 {
        self.get_value(Self::selection_start_property())
    }

    pub fn set_selection_start(&self, value: i32) {
        self.set_value(Self::selection_start_property(), value)
    }

    fn on_selection_start_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        self.update_command_states();

        let value = e.get_new_value::<i32>();
        if self.selection_end() == value && self.caret_index() != value {
            self.set_current_value(Self::caret_index_property(), value);
        }
    }

    /// The end position of the text selected in the text box.
    ///
    /// When the `SelectionEnd` is equal to `SelectionStart`, there is no
    /// selected text and it marks the caret position.
    pub fn selection_end(&self) -> i32 {
        self.get_value(Self::selection_end_property())
    }

    pub fn set_selection_end(&self, value: i32) {
        self.set_value(Self::selection_end_property(), value)
    }

    fn on_selection_end_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        self.update_command_states();

        let value = e.get_new_value::<i32>();
        if self.selection_start() == value && self.caret_index() != value {
            self.set_current_value(Self::caret_index_property(), value);
        }
    }

    /// The maximum number of characters that the text box can accept. This
    /// constraint only applies for manually entered (user-inputted) text.
    pub fn max_length(&self) -> i32 {
        self.get_value(Self::max_length_property())
    }

    pub fn set_max_length(&self, value: i32) {
        self.set_value(Self::max_length_property(), value)
    }

    /// The maximum number of visible lines to size to.
    pub fn max_lines(&self) -> i32 {
        self.get_value(Self::max_lines_property())
    }

    pub fn set_max_lines(&self, value: i32) {
        self.set_value(Self::max_lines_property(), value)
    }

    /// The minimum number of visible lines to size to.
    pub fn min_lines(&self) -> i32 {
        self.get_value(Self::min_lines_property())
    }

    pub fn set_min_lines(&self, value: i32) {
        self.set_value(Self::min_lines_property(), value)
    }

    /// The line height.
    pub fn line_height(&self) -> f64 {
        self.get_value(Self::line_height_property())
    }

    pub fn set_line_height(&self, value: f64) {
        self.set_value(Self::line_height_property(), value)
    }

    /// The text content of the text box.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_owned))
    }

    fn coerce_text_callback(sender: &FerroObject, value: Option<String>) -> Option<String> {
        match sender.downcast_ref::<TextBox>() {
            Some(text_box) => text_box.coerce_text(value),
            None => value,
        }
    }

    /// The text selected in the text box.
    pub fn selected_text(&self) -> String {
        self.get_selection()
    }

    /// Replaces the selected text; a null or empty value deletes it.
    pub fn set_selected_text(&self, value: Option<&str>) {
        match value {
            Some(value) if !value.is_empty() => self.handle_text_input(Some(value)),
            _ => {
                self.selected_text_changes_made_since_last_undo_snapshot
                    .set(self.selected_text_changes_made_since_last_undo_snapshot.get() + 1);
                self.snapshot_undo_redo(false);
                self.delete_selection();
            }
        }
    }

    /// The horizontal alignment of the content within the control.
    pub fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_content_alignment_property())
    }

    pub fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_content_alignment_property(), value)
    }

    /// The vertical alignment of the content within the control.
    pub fn vertical_content_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_content_alignment_property())
    }

    pub fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_content_alignment_property(), value)
    }

    /// The text alignment of the text box.
    pub fn text_alignment(&self) -> TextAlignment {
        self.get_value(Self::text_alignment_property())
    }

    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.set_value(Self::text_alignment_property(), value)
    }

    /// The placeholder or descriptive text that is displayed even if the
    /// `Text` property is not yet set.
    pub fn placeholder_text(&self) -> Option<String> {
        self.get_value(Self::placeholder_text_property())
    }

    pub fn set_placeholder_text(&self, value: Option<&str>) {
        self.set_value(Self::placeholder_text_property(), value.map(str::to_owned))
    }

    /// The placeholder or descriptive text that is displayed even if the
    /// `Text` property is not yet set.
    #[deprecated(note = "Use placeholder_text instead.")]
    pub fn watermark(&self) -> Option<String> {
        self.placeholder_text()
    }

    #[deprecated(note = "Use set_placeholder_text instead.")]
    pub fn set_watermark(&self, value: Option<&str>) {
        self.set_placeholder_text(value)
    }

    /// Whether the `PlaceholderText` will still be shown above the `Text`
    /// even after a text value is set.
    pub fn use_floating_placeholder(&self) -> bool {
        self.get_value(Self::use_floating_placeholder_property())
    }

    pub fn set_use_floating_placeholder(&self, value: bool) {
        self.set_value(Self::use_floating_placeholder_property(), value)
    }

    /// Whether the `PlaceholderText` will still be shown above the `Text`
    /// even after a text value is set.
    #[deprecated(note = "Use use_floating_placeholder instead.")]
    pub fn use_floating_watermark(&self) -> bool {
        self.use_floating_placeholder()
    }

    #[deprecated(note = "Use set_use_floating_placeholder instead.")]
    pub fn set_use_floating_watermark(&self, value: bool) {
        self.set_use_floating_placeholder(value)
    }

    /// The brush used for the foreground color of the placeholder text.
    pub fn placeholder_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::placeholder_foreground_property())
    }

    pub fn set_placeholder_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::placeholder_foreground_property(), value)
    }

    /// The brush used for the foreground color of the placeholder text.
    #[deprecated(note = "Use placeholder_foreground instead.")]
    pub fn watermark_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.placeholder_foreground()
    }

    #[deprecated(note = "Use set_placeholder_foreground instead.")]
    pub fn set_watermark_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_placeholder_foreground(value)
    }

    /// Custom content that is positioned on the left side of the text
    /// layout box.
    pub fn inner_left_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::inner_left_content_property())
    }

    pub fn set_inner_left_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::inner_left_content_property(), value)
    }

    /// Custom content that is positioned on the right side of the text
    /// layout box.
    pub fn inner_right_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::inner_right_content_property())
    }

    pub fn set_inner_right_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::inner_right_content_property(), value)
    }

    /// Whether text masked by `PasswordChar` should be revealed.
    pub fn reveal_password(&self) -> bool {
        self.get_value(Self::reveal_password_property())
    }

    pub fn set_reveal_password(&self, value: bool) {
        self.set_value(Self::reveal_password_property(), value)
    }

    /// The text wrapping of the text box.
    pub fn text_wrapping(&self) -> TextWrapping {
        self.get_value(Self::text_wrapping_property())
    }

    pub fn set_text_wrapping(&self, value: TextWrapping) {
        self.set_value(Self::text_wrapping_property(), value)
    }

    /// Which characters are inserted when Enter is pressed. Default: the
    /// line break of the platform.
    pub fn new_line(&self) -> String {
        self.get_value(Self::new_line_property())
    }

    pub fn set_new_line(&self, value: &str) {
        self.set_value(Self::new_line_property(), value.to_owned())
    }

    /// Clears the current selection, maintaining the `CaretIndex`.
    pub fn clear_selection(&self) {
        self.set_current_value(Self::caret_index_property(), self.selection_start());
        self.set_current_value(Self::selection_end_property(), self.selection_start());
    }

    /// Whether the cut command can be executed.
    pub fn can_cut(&self) -> bool {
        self.can_cut.get()
    }

    fn set_can_cut(&self, value: bool) {
        self.set_and_raise_cell(Self::can_cut_property(), &self.can_cut, value);
    }

    /// Whether the copy command can be executed.
    pub fn can_copy(&self) -> bool {
        self.can_copy.get()
    }

    fn set_can_copy(&self, value: bool) {
        self.set_and_raise_cell(Self::can_copy_property(), &self.can_copy, value);
    }

    /// Whether the paste command can be executed.
    pub fn can_paste(&self) -> bool {
        self.can_paste.get()
    }

    fn set_can_paste(&self, value: bool) {
        self.set_and_raise_cell(Self::can_paste_property(), &self.can_paste, value);
    }

    /// Whether undo/redo is enabled.
    pub fn is_undo_enabled(&self) -> bool {
        self.get_value(Self::is_undo_enabled_property())
    }

    pub fn set_is_undo_enabled(&self, value: bool) {
        self.set_value(Self::is_undo_enabled_property(), value)
    }

    /// The maximum number of items that can reside in the undo stack.
    pub fn undo_limit(&self) -> i32 {
        self.get_value(Self::undo_limit_property())
    }

    pub fn set_undo_limit(&self, value: i32) {
        self.set_value(Self::undo_limit_property(), value)
    }

    fn on_undo_limit_changed(&self, new_value: i32) {
        self.undo_redo_helper.set_limit(new_value);

        // "Setting UndoLimit clears the undo queue."
        self.clear_undo_redo();
    }

    /// Whether the undo stack has an action that can be undone.
    pub fn can_undo(&self) -> bool {
        self.can_undo.get()
    }

    fn set_can_undo(&self, value: bool) {
        self.set_and_raise_cell(Self::can_undo_property(), &self.can_undo, value);
    }

    /// Whether the redo stack has an action that can be redone.
    pub fn can_redo(&self) -> bool {
        self.can_redo.get()
    }

    fn set_can_redo(&self, value: bool) {
        self.set_and_raise_cell(Self::can_redo_property(), &self.can_redo, value);
    }

    /// The number of lines in the text box, or -1 if no layout information
    /// is available.
    ///
    /// If the text wraps, changing the width of the text box may change this
    /// value. The value returned is the number of lines in the entire text
    /// box, regardless of how many are currently in view.
    pub fn get_line_count(&self) -> i32 {
        match self.presenter() {
            Some(presenter) => presenter.text_layout().text_lines().len() as i32,
            None => -1,
        }
    }

    /// Raised when content is being copied to the clipboard.
    pub fn copying_to_clipboard(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::copying_to_clipboard_event(), handler)
    }

    /// Raised when content is being cut to the clipboard.
    pub fn cutting_to_clipboard(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::cutting_to_clipboard_event(), handler)
    }

    /// Raised when content is being pasted from the clipboard.
    ///
    /// The event is raised with [`PastingFromClipboardEventArgs`]: the
    /// handler receives the args as raised, so that it can reach them with
    /// `e.downcast_ref::<PastingFromClipboardEventArgs>()` (the event itself
    /// is declared with the base args, as in the reference).
    pub fn pasting_from_clipboard(
        &self,
        handler: impl Fn(&Interactive, &dyn IRoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_route_handler(
            Self::pasting_from_clipboard_event(),
            Rc::new(handler),
            Interactive::DEFAULT_ROUTES,
            false,
        )
    }

    /// Occurs asynchronously after text changes and the new text is
    /// rendered.
    pub fn text_changed(
        &self,
        handler: impl Fn(&Interactive, &TextChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::text_changed_event(), handler)
    }

    /// Occurs synchronously when text starts to change but before it is
    /// rendered.
    ///
    /// This event occurs just after the `Text` property value has been
    /// updated.
    pub fn text_changing(
        &self,
        handler: impl Fn(&Interactive, &TextChangingEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::text_changing_event(), handler)
    }

    fn presenter(&self) -> Option<Ref<TextPresenter>> {
        self.presenter.borrow().clone()
    }

    fn scroll_viewer(&self) -> Option<Ref<ScrollViewer>> {
        self.scroll_viewer.borrow().clone()
    }

    /// The UTF-16 form of the text (empty when the text is null).
    fn text_utf16(&self) -> Vec<u16> {
        self.text().map(|text| text.encode_utf16().collect()).unwrap_or_default()
    }

    /// The length of the text in UTF-16 code units (0 when the text is
    /// null).
    fn text_length(&self) -> i32 {
        self.text().map_or(0, |text| utf16_length(&text))
    }

    fn touch_radius(&self) -> i32 {
        *self.touch_radius.get_or_init(|| {
            let height = FerroLocator::current()
                .get_service::<dyn IPlatformSettings>()
                .map_or(10.0, |settings| settings.get_tap_size(PointerType::Touch).height);

            (height / 2.0) as i32 + 5
        })
    }

    fn presenter_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == TextPresenter::preedit_text_property().as_property() {
            let (old_value, new_value) = e.get_old_and_new_value::<Option<String>>();

            if old_value.is_none_or(|old_value| old_value.is_empty())
                && new_value.is_some_and(|new_value| !new_value.is_empty())
            {
                self.pseudo_classes().set(":empty", false);

                self.delete_selection();
            }
        }
    }

    fn update_command_states(&self) {
        let has_selection = self.has_selection();
        self.set_can_copy(!self.is_password_box() && has_selection);
        self.set_can_cut(!self.is_password_box() && has_selection && !self.is_read_only());
        self.set_can_paste(!self.is_read_only());
    }

    /// Inserts typed or pasted text at the caret, replacing the selection.
    pub(crate) fn handle_text_input(&self, input: Option<&str>) {
        if self.is_read_only() {
            return;
        }

        let Some(mut input) = self.sanitize_input_text(input).filter(|input| !input.is_empty()) else {
            return;
        };

        self.selected_text_changes_made_since_last_undo_snapshot
            .set(self.selected_text_changes_made_since_last_undo_snapshot.get() + 1);
        self.snapshot_undo_redo(false);

        let mut text = self.text_utf16();
        let selection_length = (self.selection_start() - self.selection_end()).abs();
        let new_length = input.len() as i32 + text.len() as i32 - selection_length;

        let max_length = self.max_length();

        if max_length > 0 && new_length > max_length {
            let length = (input.len() as i32 - (new_length - max_length)).max(0);
            input.truncate(length as usize);
        }

        if !input.is_empty() {
            let mut caret_index = self.caret_index();

            if selection_length != 0 {
                let (start, _) = self.get_selection_range();

                remove_range(&mut text, start, selection_length);

                caret_index = start;
            }

            let insert_at = (caret_index.max(0) as usize).min(text.len());
            let input_length = input.len() as i32;

            text.splice(insert_at..insert_at, input);

            let text = String::from_utf16_lossy(&text);

            self.set_text_from_edit(Some(text.clone()));

            self.clear_selection();

            if self.is_undo_enabled() {
                self.undo_redo_helper.discard_redo();
            }

            let presenter = self.presenter();

            // Make sure the updated text is in sync.
            if let Some(presenter) = &presenter {
                presenter.set_current_value(TextPresenter::text_property(), Some(text));
            }

            caret_index += input_length;

            // Make sure the caret is in sync.
            if let Some(presenter) = &presenter {
                presenter.move_caret_to_text_position(caret_index, false);
            }

            self.set_current_value(Self::caret_index_property(), caret_index);
        }
    }

    /// The UTF-16 form of an input without the characters the text box does
    /// not accept: everything from the first line break on when the text box
    /// does not accept returns, and the invalid characters.
    fn sanitize_input_text(&self, text: Option<&str>) -> Option<Vec<u16>> {
        let mut text: Vec<u16> = text?.encode_utf16().collect();

        if !self.accepts_return() {
            let mut line_break_start = 0;
            let mut grapheme_enumerator = GraphemeEnumerator::new(&text);

            while let Some(grapheme) = grapheme_enumerator.move_next() {
                if grapheme.first_codepoint().is_break_char() {
                    break;
                }

                line_break_start += grapheme.length();
            }

            // All lines except the first one are discarded when the text box does not accept the Return key.
            text.truncate(line_break_start);
        }

        text.retain(|unit| !INVALID_CHARACTERS.contains(unit));

        Some(text)
    }

    /// Cuts the current text onto the clipboard.
    ///
    /// Like the asynchronous method of the reference, everything up to the
    /// clipboard call runs before this returns; the selection is deleted
    /// once the clipboard has taken the text, from a dispatcher job unless
    /// the clipboard completed at once. A failure platform clipboards are
    /// known for (see `ClipboardHelper::is_expected_clipboard_exception`)
    /// is logged and leaves the selection in place; any other failure, and
    /// a panic of the clipboard, is raised on the dispatcher.
    pub fn cut(&self) {
        let text = self.get_selection();

        if text.is_empty() {
            return;
        }

        let event_args = RoutedEventArgs::with_event(Self::cutting_to_clipboard_event());
        self.raise_event(&event_args);
        if !event_args.handled() {
            self.snapshot_undo_redo(true);

            let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top_level| top_level.clipboard())
            else {
                return;
            };

            let this = self.to_ref();

            ClipboardHelper::start(async move {
                match clipboard.set_text_async(Some(&text)).await {
                    Ok(()) => {}
                    Err(ex) if ClipboardHelper::is_expected_clipboard_exception(&ex) => {
                        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                            let source: &dyn Any = &this;
                            logger.log_with_values(Some(source), "Failed to write text to clipboard: {Error}", &[&ex]);
                        }
                        return Ok(());
                    }
                    Err(ex) => return Err(ex),
                }

                this.delete_selection();
                Ok(())
            });
        }
    }

    /// Copies the current text onto the clipboard.
    ///
    /// See [`cut`](Self::cut) for how the clipboard operation runs.
    pub fn copy(&self) {
        let text = self.get_selection();

        if text.is_empty() {
            return;
        }

        let event_args = RoutedEventArgs::with_event(Self::copying_to_clipboard_event());
        self.raise_event(&event_args);
        if !event_args.handled() {
            let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top_level| top_level.clipboard())
            else {
                return;
            };

            let this = self.to_ref();

            ClipboardHelper::start(async move {
                match clipboard.set_text_async(Some(&text)).await {
                    Ok(()) => {}
                    Err(ex) if ClipboardHelper::is_expected_clipboard_exception(&ex) => {
                        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                            let source: &dyn Any = &this;
                            logger.log_with_values(Some(source), "Failed to write text to clipboard: {Error}", &[&ex]);
                        }
                    }
                    Err(ex) => return Err(ex),
                }

                Ok(())
            });
        }
    }

    /// Pastes the current clipboard text content into the text box.
    ///
    /// See [`cut`](Self::cut) for how the clipboard operation runs.
    pub fn paste(&self) {
        let pasted = self.paste_core_async(TopLevel::get_top_level(Some(self)).and_then(|top_level| top_level.clipboard()));

        ClipboardHelper::start(async move {
            match pasted.await {
                Ok(result) => result,
                // A paste that was dropped with the dispatcher has nothing left to do.
                Err(_) => Ok(()),
            }
        });
    }

    /// Pastes the text of a clipboard into the text box, once the clipboard
    /// has delivered it. The pasting event is raised before this returns.
    ///
    /// A failure platform clipboards are known for is logged and pastes
    /// nothing; any other failure is the result of the task, for whoever
    /// awaits it.
    fn paste_core_async(&self, clipboard: Option<Rc<dyn IClipboard>>) -> DispatcherTask<Result<(), ClipboardError>> {
        let this = self.to_ref();

        Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            let event_args =
                PastingFromClipboardEventArgs::new(Self::pasting_from_clipboard_event(), clipboard.clone());
            this.raise_event(&event_args);

            let Some(clipboard) = clipboard.filter(|_| !event_args.handled()) else {
                return Ok(());
            };

            let mut text: Option<String> = None;

            match clipboard.try_get_text_async().await {
                Ok(value) => text = value,
                Err(ex) if ClipboardHelper::is_expected_clipboard_exception(&ex) => {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                        let source: &dyn Any = &this;
                        logger.log_with_values(Some(source), "Failed to read text from clipboard: {Error}", &[&ex]);
                    }
                }
                Err(ex) => return Err(ex),
            }

            let Some(text) = text.filter(|text| !text.is_empty()) else {
                return Ok(());
            };

            this.snapshot_undo_redo(true);
            this.handle_text_input(Some(&text));
            Ok(())
        })
    }

    fn select_word(&self, text: &[u16], caret_index: i32, mut selection_start: i32, mut selection_end: i32) {
        if self.is_password_box() {
            // Double-clicking in a cloaked single-line password box selects all text.
            self.word_selection_start.set(-1);

            self.select_all();
        }

        if !StringUtils::is_start_of_word(text, caret_index) {
            selection_start = StringUtils::previous_word(text, caret_index);
        }

        if !StringUtils::is_end_of_word(text, caret_index) {
            selection_end = StringUtils::next_word(text, caret_index);
        }

        if selection_start != selection_end {
            self.word_selection_start.set(selection_start);
        }

        self.set_current_value(Self::selection_start_property(), selection_start);
        self.set_current_value(Self::selection_end_property(), selection_end);
    }

    fn update_word_selection_range(&self, caret_index: i32, selection_start: &mut i32, selection_end: &mut i32) {
        let text = self.text_utf16();

        if text.is_empty() {
            return;
        }

        let word_selection_start = self.word_selection_start.get();

        if caret_index > word_selection_start {
            let next_word = StringUtils::next_word(&text, caret_index);

            *selection_end = next_word;

            *selection_start = word_selection_start;
        } else {
            let previous_word = StringUtils::previous_word(&text, caret_index);
            *selection_start = previous_word;

            *selection_end = StringUtils::next_word(&text, word_selection_start);
        }
    }

    /// Clamps a text position to the text of `sender` and moves it out of
    /// the middle of a `\r\n` pair.
    ///
    /// Also used by the text presenter and the selectable text block.
    pub(crate) fn coerce_caret_index(sender: &FerroObject, value: i32) -> i32 {
        let text = match sender.downcast_ref::<SelectableTextBlock>() {
            Some(text_block) if text_block.has_complex_content() => {
                text_block.inlines().and_then(|inlines| inlines.text())
            }
            _ => sender.get_value(Self::text_property()),
        };

        let Some(text) = text else {
            return 0;
        };

        if value < 0 {
            return 0;
        }

        let mut length = 0;
        let mut previous = None;
        let mut current = None;

        for unit in text.encode_utf16() {
            if length == value - 1 {
                previous = Some(unit);
            } else if length == value {
                current = Some(unit);
            }

            length += 1;
        }

        if value > length {
            length
        } else if previous == Some(CARRIAGE_RETURN) && current == Some(LINE_FEED) {
            value + 1
        } else {
            value
        }
    }

    /// Clears the text in the text box.
    pub fn clear(&self) {
        self.set_text_from_edit(Some(String::new()))
    }

    fn move_horizontal(&self, direction: i32, whole_word: bool, is_selecting: bool, move_caret_position: bool) {
        let Some(presenter) = self.presenter() else {
            return;
        };

        let _change = self.im_client.begin_change();

        let selection_start = self.selection_start();
        let selection_end = self.selection_end();

        let logical_direction = if direction > 0 { LogicalDirection::Forward } else { LogicalDirection::Backward };

        if !whole_word {
            if is_selecting {
                presenter.move_caret_to_text_position(selection_end, false);

                presenter.move_caret_horizontal(logical_direction);

                self.set_current_value(Self::selection_end_property(), presenter.caret_index());
            } else {
                if selection_start != selection_end {
                    self.clear_selection_and_move_caret_to_text_position(logical_direction);
                } else {
                    presenter.move_caret_horizontal(logical_direction);
                }

                self.set_current_value(Self::caret_index_property(), presenter.caret_index());
            }
        } else {
            let text = self.text_utf16();

            let offset = if direction > 0 {
                StringUtils::next_word(&text, selection_end) - selection_end
            } else {
                StringUtils::previous_word(&text, selection_end) - selection_end
            };

            self.set_current_value(Self::selection_end_property(), self.selection_end() + offset);

            if move_caret_position {
                presenter.move_caret_to_text_position(self.selection_end(), false);
            }

            if !is_selecting && move_caret_position {
                self.set_current_value(Self::caret_index_property(), self.selection_end());
            } else {
                self.set_current_value(Self::selection_start_property(), selection_start);
            }
        }
    }

    fn move_vertical(&self, direction: LogicalDirection, is_selecting: bool) {
        let Some(presenter) = self.presenter() else {
            return;
        };

        if is_selecting {
            let old_caret_index = presenter.caret_index();
            presenter.move_caret_vertical(direction);
            let new_caret_index = presenter.caret_index();

            if old_caret_index == new_caret_index {
                let text_length = self.text_length();

                // The caret did not move while we are selecting so we could not move to the previous/next
                // line, but check if we are already at the 'boundary' of the text.
                if direction == LogicalDirection::Forward && new_caret_index < text_length {
                    presenter.move_caret_to_text_position(text_length, false);
                } else if direction == LogicalDirection::Backward && new_caret_index > 0 {
                    presenter.move_caret_to_text_position(0, false);
                }
            }

            self.set_current_value(Self::selection_end_property(), presenter.caret_index());
        } else {
            if self.selection_start() != self.selection_end() {
                self.clear_selection_and_move_caret_to_text_position(direction);
            }

            presenter.move_caret_vertical(direction);

            self.set_current_value(Self::caret_index_property(), presenter.caret_index());
        }
    }

    fn move_home(&self, document: bool) {
        let Some(presenter) = self.presenter() else {
            return;
        };

        let caret_index = self.caret_index();

        if document {
            presenter.move_caret_to_text_position(0, false);
        } else {
            let text_layout = presenter.text_layout();
            let line_index = text_layout.get_line_index_from_character_index(caret_index, false);
            let text_position = text_layout.text_lines()[line_index as usize].first_text_source_index();

            presenter.move_caret_to_text_position(text_position, false);
        }
    }

    fn move_end(&self, document: bool) {
        let Some(presenter) = self.presenter() else {
            return;
        };

        let caret_index = self.caret_index();

        if document {
            presenter.move_caret_to_text_position(self.text_length(), true);
        } else {
            let text_layout = presenter.text_layout();
            let line_index = text_layout.get_line_index_from_character_index(caret_index, false);
            let text_line = &text_layout.text_lines()[line_index as usize];

            let text_position = text_line.first_text_source_index() + text_line.length() - text_line.new_line_length();

            presenter.move_caret_to_text_position(text_position, true);
        }
    }

    fn move_page_right(&self) {
        if let Some(scroll_viewer) = self.scroll_viewer() {
            scroll_viewer.page_right();
        }
    }

    fn move_page_left(&self) {
        if let Some(scroll_viewer) = self.scroll_viewer() {
            scroll_viewer.page_left();
        }
    }

    fn move_page_up(&self) {
        if let Some(scroll_viewer) = self.scroll_viewer() {
            scroll_viewer.page_up();
        }
    }

    fn move_page_down(&self) {
        if let Some(scroll_viewer) = self.scroll_viewer() {
            scroll_viewer.page_down();
        }
    }

    fn clear_selection_and_move_caret_to_text_position(&self, direction: LogicalDirection) {
        let new_position = if direction == LogicalDirection::Forward {
            self.selection_start().max(self.selection_end())
        } else {
            self.selection_start().min(self.selection_end())
        };
        self.set_current_value(Self::selection_start_property(), new_position);
        self.set_current_value(Self::selection_end_property(), new_position);
        // Move the caret to the appropriate side of the previous selection.
        if let Some(presenter) = self.presenter() {
            presenter.move_caret_to_text_position(new_position, false);
        }
    }

    /// Scrolls the text box to the specified line index.
    ///
    /// # Panics
    ///
    /// When `line_index` is less than zero, or larger than or equal to the
    /// line count.
    pub fn scroll_to_line(&self, line_index: i32) {
        let Some(presenter) = self.presenter() else {
            return;
        };

        let text_layout = presenter.text_layout();

        if line_index < 0 || line_index >= text_layout.text_lines().len() as i32 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'lineIndex')");
        }

        let text_position = text_layout.text_lines()[line_index as usize].first_text_source_index();
        presenter.move_caret_to_text_position(text_position, false);
    }

    /// Selects all text in the text box.
    pub fn select_all(&self) {
        let _change = self.im_client.begin_change();

        self.set_current_value(Self::selection_start_property(), 0);
        self.set_current_value(Self::selection_end_property(), self.text_length());
    }

    fn get_selection_range(&self) -> (i32, i32) {
        let selection_start = self.selection_start();
        let selection_end = self.selection_end();

        (selection_start.min(selection_end), selection_start.max(selection_end))
    }

    /// Deletes the selected text. Returns whether there was a selection (or
    /// the text box is read-only).
    pub(crate) fn delete_selection(&self) -> bool {
        if self.is_read_only() {
            return true;
        }

        let _change = self.im_client.begin_change();

        let (start, end) = self.get_selection_range();

        if start != end {
            let mut text = self.text_utf16();

            remove_range(&mut text, start, end - start);

            self.set_text_from_edit(Some(String::from_utf16_lossy(&text)));

            if let Some(presenter) = self.presenter() {
                presenter.move_caret_to_text_position(start, false);
            }

            self.set_current_value(Self::selection_start_property(), start);

            self.clear_selection();

            return true;
        }

        self.set_current_value(Self::caret_index_property(), self.selection_start());

        false
    }

    /// Reports the same emptiness conditions as `get_selection`, without
    /// building the selected string.
    fn has_selection(&self) -> bool {
        let (start, end) = self.get_selection_range();

        if start == end {
            return false;
        }

        let text_length = self.text_length();

        text_length > 0 && end <= text_length
    }

    fn get_selection(&self) -> String {
        let Some(text) = self.text().filter(|text| !text.is_empty()) else {
            return String::new();
        };

        let selection_start = self.selection_start();
        let selection_end = self.selection_end();
        let start = selection_start.min(selection_end);
        let end = selection_start.max(selection_end);

        if start == end || utf16_length(&text) < end {
            return String::new();
        }

        // The indices are UTF-16 code units; a selection that splits a
        // surrogate pair yields a replacement character for the lone half.
        let selected_text: Vec<u16> =
            text.encode_utf16().skip(start.max(0) as usize).take((end - start) as usize).collect();

        String::from_utf16_lossy(&selected_text)
    }

    /// Sets the text as an edit of the user: the undo history is kept.
    pub(crate) fn set_text_from_edit(&self, value: Option<String>) {
        self.set_text_core(value, TextMutationKind::Edit)
    }

    /// Sets the text as a synchronization of the control with itself: the
    /// undo history is neither extended nor cleared.
    pub(crate) fn set_text_from_internal_synchronization(&self, value: Option<String>) {
        self.set_text_core(value, TextMutationKind::InternalSynchronization)
    }

    fn set_text_core(&self, value: Option<String>, mutation_kind: TextMutationKind) {
        // Stays set through the synchronous TwoWay source echo so it isn't mistaken for an external replacement.
        let previous_mutation_kind = self.text_mutation_kind.replace(mutation_kind);

        // Restores the mutation kind when the scope ends, also by a panic of a handler.
        struct Restore<'a>(&'a Cell<TextMutationKind>, TextMutationKind);

        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }

        let _restore = Restore(&self.text_mutation_kind, previous_mutation_kind);

        self.set_current_value(Self::text_property(), value);
    }

    /// Returns the sum of any vertical whitespace added between the
    /// [`ScrollViewer`] and the [`TextPresenter`] in the control template.
    fn get_vertical_space_between_scroll_viewer_and_presenter(&self) -> f64 {
        let mut vertical_space = 0.0;

        if let Some(presenter) = self.presenter() {
            let scroll_viewer: Option<Ref<Visual>> = self.scroll_viewer().map(Ref::upcast);
            let this: Ref<Visual> = self.to_ref().upcast();
            let mut visual: Option<Ref<Visual>> = Some(presenter.upcast());

            while let Some(current) = visual.filter(|visual| *visual != this) {
                if Some(&current) == scroll_viewer.as_ref() {
                    // The scroll viewer is a stopping point and should only include the padding.
                    let padding: Thickness = current.get_value(Decorator::padding_property());
                    vertical_space += padding.top + padding.bottom;
                    break;
                }

                let margin: Thickness = current.get_value(Layoutable::margin_property());
                let padding: Thickness = current.get_value(Decorator::padding_property());

                vertical_space += margin.top + padding.top + padding.bottom + margin.bottom;

                visual = current.visual_parent();
            }
        }

        vertical_space
    }

    /// The layout of `lines` empty lines in the font of the text box.
    fn create_lines_text_layout(&self, lines: i32) -> TextLayout {
        let font_size = self.font_size();
        let typeface =
            Typeface::with_style(self.font_family(), self.font_style(), self.font_weight(), self.font_stretch());

        let text_run_style: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
            typeface,
            font_size,
            None,
            None,
            None,
            BaselineAlignment::Baseline,
            None,
            self.font_features(),
        ));

        let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_all(
            FlowDirection::LeftToRight,
            TextAlignment::Left,
            true,
            false,
            text_run_style,
            TextWrapping::NoWrap,
            self.line_height(),
            0.0,
            0.0,
        ));

        TextLayout::from_text_source(
            Rc::new(LineTextSource { lines }),
            paragraph_properties,
            None,
            f64::INFINITY,
            f64::INFINITY,
            0,
            None,
        )
    }

    /// Raises both the `TextChanging` and `TextChanged` events.
    ///
    /// This must be called after the `Text` property is set.
    fn raise_text_change_events(&self) {
        // Note the following sequence of these events:
        // 1. TextChanging occurs synchronously when text starts to change but before it is rendered.
        //    This occurs after the Text property is set.
        // 2. TextChanged occurs asynchronously after text changes and the new text is rendered.

        let text_changing_event_args = TextChangingEventArgs::with_event(Self::text_changing_event());
        self.raise_event(&text_changing_event_args);

        let this = self.to_ref();

        Dispatcher::ui_thread().post_local(
            move || {
                let text_changed_event_args = TextChangedEventArgs::with_event(Self::text_changed_event());
                this.raise_event(&text_changed_event_args);
            },
            DispatcherPriority::NORMAL,
        );
    }

    fn set_selection_for_control_backspace(&self) {
        let text = self.text_utf16();
        let selection_start = self.caret_index();

        let _change = self.im_client.begin_change();

        self.move_horizontal(-1, true, false, false);

        if self.selection_end() > 0
            && selection_start < text.len() as i32
            && text.get(selection_start.max(0) as usize) == Some(&SPACE)
        {
            self.set_current_value(Self::selection_end_property(), self.selection_end() - 1);
        }

        self.set_current_value(Self::selection_start_property(), selection_start);
    }

    fn set_selection_for_control_delete(&self) {
        let text = self.text_utf16();
        let text_length = text.len() as i32;
        if self.presenter().is_none() || text_length == 0 {
            return;
        }

        let _change = self.im_client.begin_change();

        self.set_current_value(Self::selection_start_property(), self.caret_index());

        self.move_horizontal(1, true, true, false);

        let selection_end = self.selection_end();

        if selection_end < text_length && text.get(selection_end.max(0) as usize) == Some(&SPACE) {
            self.set_current_value(Self::selection_end_property(), selection_end + 1);
        }
    }

    fn update_pseudoclasses(&self) {
        self.pseudo_classes().set(":empty", self.text().is_none_or(|text| text.is_empty()));
        self.pseudo_classes().set(":touch-mode", self.is_in_touch_mode.get());
    }

    fn is_password_box(&self) -> bool {
        self.password_char() != '\0' && !self.reveal_password()
    }

    fn snapshot_undo_redo(&self, ignore_change_count: bool) {
        if self.is_undo_enabled()
            && (ignore_change_count
                || !self.has_done_snapshot_once.get()
                || self.selected_text_changes_made_since_last_undo_snapshot.get() >= MAX_CHARS_BEFORE_UNDO_SNAPSHOT)
        {
            self.undo_redo_helper.snapshot();
            self.selected_text_changes_made_since_last_undo_snapshot.set(0);
            self.has_done_snapshot_once.set(true);
        }
    }

    fn clear_undo_redo(&self) {
        self.undo_redo_helper.clear();
        self.selected_text_changes_made_since_last_undo_snapshot.set(0);
        self.has_done_snapshot_once.set(false);
    }

    /// Undoes the first action in the undo stack.
    pub fn undo(&self) {
        if self.is_undo_enabled() && self.can_undo() {
            // Snapshot the current Text state - this will get popped on to the redo stack
            // when we call undo below.
            self.snapshot_undo_redo(true);

            let _undoing_redoing = UndoingRedoing::enter(&self.is_undoing_redoing);
            self.undo_redo_helper.undo();
        }
    }

    /// Reapplies the first item on the redo stack.
    pub fn redo(&self) {
        if self.is_undo_enabled() && self.can_redo() {
            let _undoing_redoing = UndoingRedoing::enter(&self.is_undoing_redoing);
            self.undo_redo_helper.redo();
        }
    }
}

/// The scope of an undo or a redo: the flag is cleared when it ends.
struct UndoingRedoing<'a>(&'a Cell<bool>);

impl<'a> UndoingRedoing<'a> {
    fn enter(flag: &'a Cell<bool>) -> Self {
        flag.set(true);
        Self(flag)
    }
}

impl Drop for UndoingRedoing<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// The line break of the platform.
fn environment_new_line() -> &'static str {
    if cfg!(windows) {
        "\r\n"
    } else {
        "\n"
    }
}

/// The length of `text` in UTF-16 code units.
fn utf16_length(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// Removes `length` code units at `start`, clamped to the text.
fn remove_range(text: &mut Vec<u16>, start: i32, length: i32) {
    let start = (start.max(0) as usize).min(text.len());
    let end = (start + length.max(0) as usize).min(text.len());

    text.drain(start..end);
}

/// Whether the pointer of a pointer event is captured by the presenter.
fn is_captured_by(e: &PointerEventArgs, presenter: &Ref<TextPresenter>) -> bool {
    e.pointer().captured().is_some_and(|captured| captured == *presenter)
}

/// Clamps a point to the bounds of the presenter.
fn clamp_to_bounds(point: Point, presenter: &TextPresenter) -> Point {
    let bounds = presenter.bounds();

    Point::new(
        MathUtilities::clamp(point.x, 0.0, (bounds.width - 1.0).max(0.0)),
        MathUtilities::clamp(point.y, 0.0, (bounds.height - 1.0).max(0.0)),
    )
}
