use crate::collapsed_line_section::CollapsedLineSections;
use crate::rendering::HeightTree;
use crate::text_input_method_client::EditorTextInputMethodClient;
use crate::{CollapsedLineSection, FormattedTextElement, TextEditor, VisualLineError};
#[path = "text_area_automation.rs"]
mod automation;
pub(crate) use automation::AutomationVisibility;
#[cfg(test)]
#[path = "text_area_background_tests.rs"]
mod background_tests;
#[path = "text_area_backgrounds.rs"]
mod backgrounds;
#[path = "text_area_inline_controls.rs"]
mod inline_controls;
#[path = "text_area_history.rs"]
mod history;
#[path = "text_area_input.rs"]
mod input;
pub use input::{
    EditorInputBindings, EditorInputHandler, EditorInputHandlerError, EditorInputLayer,
    EditorInputMode,
};
#[path = "text_area_layers.rs"]
mod layers;
pub use layers::{EditorLayer, LayerInsertionPosition, LayerPosition};
#[cfg(test)]
#[path = "text_area_adornment_tests.rs"]
mod adornment_tests;
#[path = "text_area_margins.rs"]
mod margins;
#[path = "text_area_navigation.rs"]
mod navigation;
pub(crate) use navigation::NavigationCaret;
#[cfg(test)]
#[path = "text_area_visual_line_tests.rs"]
mod visual_line_tests;
#[path = "text_area_visual_lines.rs"]
mod visual_lines;
#[path = "text_area_visual_positions.rs"]
mod visual_positions;
#[path = "text_area_viewport_events.rs"]
mod viewport_events;
use backgrounds::BackgroundRendererHost;
use inline_controls::InlineControlLayer;
use layers::LayerHost;
use margins::MarginHost;
use navigation::{BoxCaret, ContextCaret};
#[path = "text_area_pointer_selection.rs"]
mod pointer_selection;
use pointer_selection::{PointerSelection, TouchSelection};
#[path = "text_area_text_drag.rs"]
mod text_drag;
#[path = "text_area_typing.rs"]
mod typing;
use ferroui_base::data::BindingPriority;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::text_input::TextInputMethodClient;
use ferroui_base::input::{
    Cursor, DragDrop, FocusChangedEventArgs, HoldingRoutedEventArgs, HoldingState, ICommand,
    IScrollable, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    KeyModifiers, NavigationDirection, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind,
    StandardCursorType, TappedEventArgs, TextInputEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::text_formatting::TextLayout;
use ferroui_base::media::{DrawingContext, IBrush};
use ferroui_base::platform::ICursorFactory;
use ferroui_base::reactive::{Disposable, IDisposable, Subscription};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    FerroLocator, FerroObjectImpl, FerroObjectImplExt, LocatorExtensions, Point, Rect, Ref, Size,
    StyledElementImpl, Vector, Visual, VisualImpl, WeakRef, ferro_class, ferro_impl_classes,
    instantiate,
};
use ferroui_controls::primitives::{ILogicalScrollable, register_logical_scrollable};
use ferroui_controls::{Application, Control, ControlImpl, TopLevel};
use ferroui_edit_core::document::{DocumentError, LiveDocumentLine, TextDocument};
use ferroui_edit_core::editing::{CaretMovement, Selection, SelectionSegment, TextTransformation};
use ferroui_edit_core::folding::FoldingSection;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;
use std::time::Duration;
use visual_lines::VisualLineHost;

struct CachedVisualLine {
    offset: usize,
    length: usize,
    display: Rc<crate::VisualLine>,
    layout: TextLayout,
    preedit: Option<(usize, usize)>,
    has_inline_controls: bool,
    selection_foreground: Option<visual_lines::SelectionForegroundStyle>,
}

#[derive(Clone, Copy)]
struct DefaultTextMetrics {
    height: f64,
    baseline: f64,
    minimum_line_height: f64,
    wide_space_width: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct CaretState {
    location: crate::TextLocation,
    navigation: NavigationCaret,
}

impl CachedVisualLine {
    fn starts_fold_at(&self, offset: usize) -> bool {
        self.display.elements().iter().any(|element| {
            element.as_any().is::<crate::FoldingLineElement>()
                && self.offset + element.data().relative_text_offset() == offset
        })
    }

    fn document_range_for_row(&self, index: usize) -> Range<usize> {
        let rows = self.layout.text_lines();
        let offset = |row: &dyn ferroui_base::media::text_formatting::TextLine| {
            self.offset
                + self
                    .display
                    .get_relative_offset(row.first_text_source_index().max(0) as usize)
        };
        let start = rows.get(index).map_or(self.offset, |row| offset(&**row));
        let end = rows.get(index + 1).map_or_else(
            || {
                let last = self.display.last_document_line();
                last.end_offset() + last.delimiter_length()
            },
            |row| offset(&**row),
        );
        start..end
    }
}

/// The key gestures of the application: those of its platform settings, or the defaults when
/// the platform registered none.
pub(crate) fn platform_keymap() -> Rc<PlatformHotkeyConfiguration> {
    let Some(registered) = FerroLocator::current().get_service::<PlatformHotkeyConfiguration>() else {
        return Rc::default();
    };
    Application::current()
        .and_then(|app| app.platform_settings())
        .map_or(registered, |settings| settings.hotkey_configuration())
}

/// The modifier of rectangular selection: Alt, unless the keymap moves by word with it, as the
/// Apple keymaps do with Option. Those use Control, which they leave free.
pub(crate) fn box_selection_modifier() -> KeyModifiers {
    if platform_keymap().whole_word_text_action_modifiers == KeyModifiers::ALT {
        KeyModifiers::CONTROL
    } else {
        KeyModifiers::ALT
    }
}

/// The modifier that makes a text drag a copy: Option on the Apple keymaps, Control on the others.
pub(crate) fn text_drag_copy_modifier() -> KeyModifiers {
    if platform_keymap().whole_word_text_action_modifiers == KeyModifiers::ALT {
        KeyModifiers::ALT
    } else {
        KeyModifiers::CONTROL
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingRedraw {
    None,
    Requested,
    InvalidatedFrame,
}

/// The editor's focus, input and rendering surface. A `ScrollViewer` discovers
/// its logical scrolling interface, so only visible logical lines are shaped.
#[repr(C)]
pub struct TextArea {
    base: Control,
    editor: RefCell<Option<WeakRef<TextEditor>>>,
    services: OnceCell<Rc<FerroLocator>>,
    viewport: Cell<Size>,
    offset: Cell<Vector>,
    extent: Cell<Size>,
    horizontal: Cell<bool>,
    vertical: Cell<bool>,
    cache: RefCell<BTreeMap<usize, Rc<CachedVisualLine>>>,
    watermark_layout: RefCell<Option<Rc<TextLayout>>>,
    visual_lines: VisualLineHost,
    viewport_events: viewport_events::ViewportEvents,
    automation_format_changes: automation::AutomationFormatChanges,
    explicit_line_collapses: RefCell<Option<CollapsedLineSections>>,
    explicit_collapse_revision: Cell<u64>,
    background_layers: BackgroundRendererHost,
    layers: LayerHost,
    margins: MarginHost,
    inline_controls: OnceCell<Ref<InlineControlLayer>>,
    preparing_inline_controls: Cell<bool>,
    heights: RefCell<HeightTree>,
    scroll_invalidated: Rc<HandlerList<dyn Fn()>>,
    rendering: Cell<bool>,
    redraw_pending: Cell<PendingRedraw>,
    scroll_notification_pending: Cell<bool>,
    pointer_selection: RefCell<Option<PointerSelection>>,
    touch_press: Cell<Option<i32>>,
    pointer_hover: OnceCell<crate::PointerHoverLogic>,
    text_cursor: OnceCell<Rc<Cursor>>,
    hover_cursor: RefCell<Option<(Rc<Cursor>, Subscription)>>,
    mouse_hover_modifiers: Cell<Option<KeyModifiers>>,
    hover_cursor_refresh_pending: Cell<bool>,
    input_handlers: input::InputHandlers,
    updating_pointer_selection: Cell<bool>,
    pointer_selection_refresh_pending: Cell<bool>,
    drop_caret: Cell<Option<NavigationCaret>>,
    box_caret: Cell<Option<BoxCaret>>,
    context_caret: Cell<Option<ContextCaret>>,
    default_text_metrics: Cell<Option<DefaultTextMetrics>>,
    caret_visible: Cell<bool>,
    caret_at_end_of_line: Cell<bool>,
    pending_undo_caret: Cell<Option<history::PendingUndoCaret>>,
    caret_geometry_pending: Cell<bool>,
    blink_timer: OnceCell<Rc<DispatcherTimer>>,
    im_client: OnceCell<Rc<EditorTextInputMethodClient>>,
    preedit: RefCell<Option<(String, Option<i32>)>>,
    highlight_pending: Cell<Option<usize>>,
    folds: RefCell<Vec<FoldingSection>>,
    desired_x: Cell<Option<f64>>,
    moving_vertically: Cell<bool>,
}

ferro_class!(TextArea: Control);
ferroui_base::ferro_class_info!(TextArea { new: TextArea::new });
ferro_impl_classes!(TextArea: StyledElementImpl, InteractiveImpl);

impl ControlImpl for TextArea {
    fn automation_target(this: &Self) -> Ref<Control> {
        this.editor().map_or_else(|| this.to_ref().upcast(), Ref::upcast)
    }
}

impl FerroObjectImpl for TextArea {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.im_client
            .set(EditorTextInputMethodClient::new(this.to_ref().downgrade()))
            .ok();
        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_millis(530));
        let weak = this.to_ref().downgrade();
        timer.tick(move |_| {
            if let Some(area) = weak.upgrade() {
                area.caret_visible.set(!area.caret_visible.get());
                area.invalidate_layer(crate::KnownLayer::Caret);
            }
        });
        this.blink_timer.set(timer).ok();
        this.pointer_hover.set(this.attach_pointer_hover()).ok();
        this.connect_visual_line_extensions();
        this.connect_background_renderers();
    }

    fn on_property_changed(this: &Self, change: &ferroui_base::FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.on_background_property_changed(change);
        if change.property() == Self::non_printable_character_brush_property().as_property()
            && change.is_effective_value_change()
        {
            this.invalidate_layout_cache();
        }
        if change.property() == Self::watermark_property().as_property() && change.is_effective_value_change() {
            this.watermark_layout.take();
            this.request_redraw();
            if let Some(editor) = this.editor() {
                let (old, new) = change.get_old_and_new_value::<Option<String>>();
                editor.raise_direct_property_changed(TextEditor::watermark_property(), &old, &new);
            }
        }
        if change.property() == Self::overstrike_mode_property().as_property() {
            this.update_caret_geometry();
            this.invalidate_layer(crate::KnownLayer::Caret);
        }
    }
}

impl LayoutableImpl for TextArea {
    fn measure_override(this: &Self, available: Size) -> Size {
        this.measure_margins(available);
        Size::new(available.width.min(640.0), available.height.min(400.0))
    }

    fn arrange_override(this: &Self, size: Size) -> Size {
        let old = this.viewport.replace(size);
        let gutter_changed = this.arrange_margins(size);
        if old.width != size.width || gutter_changed {
            this.reset_line_metrics();
            // Recompute the extent when its width comes from the old viewport.
            if gutter_changed || this.extent.get().width <= old.width {
                this.extent.set(Size::default());
            }
        }
        this.arrange_native_layers();
        this.arrange_inline_controls();
        this.update_extent();
        if old != size {
            this.raise_scroll_invalidated();
        }
        this.refresh_margin_viewport();
        this.refresh_arranged_viewport();
        this.invalidate_hover_cursor();
        size
    }
}

impl VisualImpl for TextArea {
    fn render(this: &Self, context: &mut DrawingContext) {
        this.render_text_view(context);
    }
}

impl InputElementImpl for TextArea {
    fn as_scrollable(this: &Self) -> Option<&dyn IScrollable> {
        Some(this)
    }

    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        if this.is_hosted_child_event(e) {
            return;
        }
        this.caret_visible.set(true);
        if let Some(timer) = this.blink_timer.get() {
            timer.start();
        }
        this.request_redraw();
    }
    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);
        if this.is_hosted_child_event(e) {
            return;
        }
        if let Some(timer) = this.blink_timer.get() {
            timer.stop();
        }
        this.set_preedit(None, None);
        this.request_redraw();
    }
    fn on_text_input(this: &Self, e: &TextInputEventArgs) {
        if e.handled() || this.is_hosted_child_event(e) {
            return;
        }
        if let Some(commit) = e.input_method_commit() {
            use ferroui_base::input::text_input::TextInputMethodError;
            use ferroui_edit_core::editing::EditError;
            let result = this.editor().ok_or(TextInputMethodError::Unavailable).and_then(|editor| {
                let owns_input = || commit.is_current();
                let text = e.text.as_deref().unwrap_or_default();
                let result = if let Some(edit) = commit.surrounding_edit() {
                    this.perform_local_input_edit(&editor, edit, owns_input)
                } else if let Some((before, after)) = commit.surrounding_deletion() {
                    this.perform_surrounding_edit(&editor, before, after, text, owns_input)
                } else if commit.edit().is_some() || commit.completes_composition() {
                    return Err(TextInputMethodError::Unsupported);
                } else {
                    this.perform_composition_commit(&editor, text, owns_input)
                };
                result.map_err(|error| match error {
                    EditError::ReadOnly => TextInputMethodError::ReadOnly,
                    EditError::InvalidSelection => TextInputMethodError::InvalidRange,
                    EditError::StaleEditPlan => TextInputMethodError::StateChanged,
                    _ => TextInputMethodError::Rejected,
                })
            });
            commit.complete(result);
            e.set_handled(true);
            return;
        }
        let text = e.text.as_deref().filter(|text| {
            !matches!(*text, "" | "\u{1b}" | "\u{8}" | "\u{7f}")
        });
        if let (Some(editor), Some(text)) = (this.editor(), text) {
            let from_input_method = e.is_from_input_method() || this.preedit.borrow().is_some();
            this.set_preedit(None, None);
            if let Err(error) = this.perform_committed_input(text, from_input_method) {
                editor.report_error(error.to_string());
            }
            e.set_handled(true);
        }
    }
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        this.update_hover_modifiers(e.key_modifiers);
        if e.handled() || this.is_hosted_child_event(e) || !this.dispatch_primary_key(e, true) {
            return;
        }
        let Some(editor) = this.editor() else {
            return;
        };
        if this.has_preedit() {
            // Prevent the window's focus navigation from taking the editor
            // off the current input-method client during composition.
            if matches!(e.key, Key::Tab | Key::Left | Key::Right | Key::Up | Key::Down) {
                e.set_handled(true);
            }
            return;
        }
        let keymap = platform_keymap();
        let modifiers = e.key_modifiers;
        let matches = |gestures: &[ferroui_base::input::KeyGesture]| {
            gestures.iter().any(|g| g.matches(Some(e)))
        };
        use crate::{EditorCommandKind, EditorNavigation};
        if e.key == Key::Insert && modifiers.is_empty() {
            let toggle = editor.command(EditorCommandKind::ToggleOverstrike);
            if toggle.can_execute(None) {
                toggle.execute(None);
                e.set_handled(true);
                return;
            }
        }
        if crate::search_panel::dispatch_search_key(&editor, e, keymap.command_modifiers, false) {
            return;
        }
        for (gestures, kind) in [
            (&keymap.copy, EditorCommandKind::Copy),
            (&keymap.cut, EditorCommandKind::Cut),
            (&keymap.paste, EditorCommandKind::Paste),
            (&keymap.select_all, EditorCommandKind::SelectAll),
            (&keymap.undo, EditorCommandKind::Undo),
            (&keymap.redo, EditorCommandKind::Redo),
        ] {
            if matches(gestures) {
                editor.command(kind).execute(None);
                e.set_handled(true);
                return;
            }
        }
        if modifiers == keymap.command_modifiers {
            let line_command = match e.key {
                Key::D => Some(EditorCommandKind::Transform(TextTransformation::DeleteLines)),
                Key::I => Some(EditorCommandKind::Reindent),
                _ => None,
            };
            if let Some(kind) = line_command {
                editor.command(kind).execute(None);
                e.set_handled(true);
                return;
            }
        }
        let selection_modifiers = keymap.selection_modifiers;
        let selecting = !selection_modifiers.is_empty() && modifiers.contains(selection_modifiers);
        let movement_modifiers = if selecting {
            modifiers.difference(selection_modifiers)
        } else {
            modifiers
        };
        let word_modifiers = keymap.whole_word_text_action_modifiers;
        let box_modifiers = box_selection_modifier();
        let arrow = match e.key {
            Key::Left => Some(CaretMovement::CharacterLeft),
            Key::Right => Some(CaretMovement::CharacterRight),
            Key::Up => Some(CaretMovement::LineUp),
            Key::Down => Some(CaretMovement::LineDown),
            _ => None,
        };
        let word_arrow = match e.key {
            Key::Left => Some(CaretMovement::WordLeft),
            Key::Right => Some(CaretMovement::WordRight),
            _ => None,
        };
        let box_movement = match e.key {
            Key::Home => Some(CaretMovement::LineStart),
            Key::End => Some(CaretMovement::LineEnd),
            _ => arrow,
        };
        let explicit_box = match movement_modifiers {
            m if selecting && m == box_modifiers => box_movement,
            m if selecting && m == (box_modifiers | word_modifiers) => word_arrow,
            _ => None,
        };
        let edge = [
            (
                &keymap.move_cursor_to_the_start_of_line,
                CaretMovement::LineStart,
                false,
            ),
            (
                &keymap.move_cursor_to_the_end_of_line,
                CaretMovement::LineEnd,
                false,
            ),
            (
                &keymap.move_cursor_to_the_start_of_document,
                CaretMovement::DocumentStart,
                false,
            ),
            (
                &keymap.move_cursor_to_the_end_of_document,
                CaretMovement::DocumentEnd,
                false,
            ),
            (
                &keymap.move_cursor_to_the_start_of_line_with_selection,
                CaretMovement::LineStart,
                true,
            ),
            (
                &keymap.move_cursor_to_the_end_of_line_with_selection,
                CaretMovement::LineEnd,
                true,
            ),
            (
                &keymap.move_cursor_to_the_start_of_document_with_selection,
                CaretMovement::DocumentStart,
                true,
            ),
            (
                &keymap.move_cursor_to_the_end_of_document_with_selection,
                CaretMovement::DocumentEnd,
                true,
            ),
        ]
        .into_iter()
        .find(|(gestures, _, _)| matches(gestures));
        let arrow = match movement_modifiers {
            KeyModifiers::NONE => arrow,
            m if m == word_modifiers => word_arrow,
            _ => None,
        };
        let caret = explicit_box
            .map(|movement| (movement, true))
            .or_else(|| edge.map(|(_, movement, extend_selection)| (movement, extend_selection)))
            .or_else(|| arrow.map(|movement| (movement, selecting)));
        let navigation = caret
            .map(|(movement, extend_selection)| {
                if explicit_box.is_some() && editor.options().enable_rectangular_selection {
                    EditorNavigation::Rectangle(movement)
                } else {
                    EditorNavigation::Caret { movement, extend_selection }
                }
            })
            .or_else(|| {
                (movement_modifiers.is_empty() && matches!(e.key, Key::PageUp | Key::PageDown))
                    .then_some(EditorNavigation::Page {
                        upward: e.key == Key::PageUp,
                        extend_selection: selecting,
                    })
            });
        if let Some(navigation) = navigation {
            if let Err(error) = this.navigate(&editor, navigation) {
                editor.report_error(error.to_string());
            }
            e.set_handled(true);
            return;
        }
        let word_deletion = modifiers == KeyModifiers::CONTROL
            || (!word_modifiers.is_empty() && modifiers == word_modifiers);
        let result = match e.key {
            Key::Back | Key::Delete if modifiers.is_empty() || word_deletion
                || (e.key == Key::Back && modifiers == KeyModifiers::SHIFT) =>
            {
                this.delete_directionally(&editor, e.key == Key::Back, word_deletion)
            }
            Key::Return if matches!(modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                this.insert_line_break()
            }
            Key::Tab if editor.options().accepts_tab => {
                editor.indent(modifiers == KeyModifiers::SHIFT, true)
            }
            // Search and stacked handlers can act on Escape before this fallback.
            Key::Escape if modifiers.is_empty() => Ok(()),
            _ => {
                Self::parent_on_key_down(this, e);
                return;
            }
        };
        if let Err(error) = result {
            editor.report_error(error.to_string());
        }
        e.set_handled(true);
    }
    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        this.update_hover_modifiers(e.key_modifiers);
        if !e.handled() && !this.is_hosted_child_event(e) {
            this.dispatch_primary_key(e, false);
        }
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        if e.handled()
            || this.is_hosted_child_event(e)
            || this.pointer_selection.borrow().is_some()
            || !this.uses_default_editor_input()
        {
            return;
        }
        let point = e.get_current_point(Some(this));
        if point.position.x < this.custom_margin_width()
            || this.line_number_separator_contains(point.position.x)
        {
            return;
        }
        if !point.properties.is_left_button_pressed {
            if point.properties.pointer_update_kind == PointerUpdateKind::RightButtonPressed
                && this.right_click_moves_caret()
            {
                if let Some(editor) = this.editor() {
                    if let Err(error) = this.move_context_caret(&editor, point.position) {
                        editor.report_error(error.to_string());
                    }
                }
            }
            return;
        }
        if let Some((line, index)) = this.element_at_point(point.position) {
            line.display.elements()[index].on_pointer_pressed(this, e);
            if e.handled() {
                return;
            }
        }
        let Some(editor) = this.editor() else {
            return;
        };
        if this.folding_gutter_contains(point.position) {
            this.press_builtin_folding_margin(e, point.position);
            return;
        }
        if e.pointer().type_() == PointerType::Touch
            && !this.line_number_gutter_contains(&editor, point.position.x)
        {
            if e.pointer().is_primary() && this.text_viewport_contains(point.position) {
                this.touch_press.set(Some(e.pointer().id()));
            }
            return;
        }
        this.focus();
        let line_number = this.line_number_gutter_contains(&editor, point.position.x);
        if let Err(error) = this.begin_pointer_selection(&editor, e, point.position, line_number) {
            editor.report_error(error.to_string());
        }
        e.set_handled(true);
    }

    fn on_tapped(this: &Self, e: &TappedEventArgs) {
        if !e.handled()
            && this.select_touch(e.pointer(), e.get_position(Some(this)), TouchSelection::Caret)
        {
            e.set_handled(true);
        } else {
            Self::parent_on_tapped(this, e);
        }
    }

    fn on_double_tapped(this: &Self, e: &TappedEventArgs) {
        if !e.handled()
            && this.select_touch(e.pointer(), e.get_position(Some(this)), TouchSelection::Word)
        {
            e.set_handled(true);
        } else {
            Self::parent_on_double_tapped(this, e);
        }
    }

    fn on_holding(this: &Self, e: &HoldingRoutedEventArgs) {
        let pointer = e.pointer_event_args();
        if !e.handled()
            && e.holding_state() == HoldingState::Started
            && this.select_touch(
                pointer.pointer(),
                pointer.get_position(Some(this)),
                TouchSelection::WordDrag,
            )
        {
            e.set_handled(true);
        } else {
            Self::parent_on_holding(this, e);
        }
    }

    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        let query_elements = this.pointer_selection.borrow().is_none() && this.uses_default_editor_input();
        this.update_hover_cursor(e, e.get_position(Some(this)), query_elements);
        Self::parent_on_pointer_entered(this, e);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        let default_input = this.uses_default_editor_input();
        let drag_id = this.pointer_selection.borrow().as_ref().map(|drag| drag.id);
        let point = e.get_position(Some(this));
        let hover = default_input && drag_id.is_none() && !this.is_hosted_child_event(e);
        this.update_folding_pointer(hover.then_some(point));
        if default_input && drag_id == Some(e.pointer().id()) {
            if e.pointer().type_() == PointerType::Touch {
                e.prevent_gesture_recognition();
            }
            this.update_hover_cursor(e, point, false);
            this.move_pointer_selection(e, point);
            e.set_handled(true);
        } else if drag_id.is_none() {
            this.update_hover_cursor(e, point, default_input);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if !this.uses_default_editor_input() {
            return;
        }
        let active = this.pointer_selection.borrow().as_ref().map(|drag| drag.id);
        let point = e.get_position(Some(this));
        match active {
            Some(id) if id == e.pointer().id() => {
                if e.get_current_point(Some(this)).properties.is_left_button_pressed {
                    return;
                }
                this.finish_pointer_selection(point);
                this.pointer_selection.take();
                e.pointer().capture(None);
                this.update_hover_cursor(e, point, true);
                e.set_handled(true);
            }
            None if !this.is_hosted_child_event(e) => {
                if let Some((line, index)) = this.element_at_point(point) {
                    line.display.elements()[index].on_pointer_released(this, e);
                }
                if e.handled() && this.touch_press.get() == Some(e.pointer().id()) {
                    this.touch_press.set(None);
                }
            }
            _ => {}
        }
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        if this.touch_press.get() == Some(e.pointer().id()) {
            this.touch_press.set(None);
        }
        let drag_id = this.pointer_selection.borrow().as_ref().map(|drag| drag.id);
        if drag_id == Some(e.pointer().id()) {
            this.pointer_selection.take();
        }
        Self::parent_on_pointer_capture_lost(this, e);
    }
    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        this.update_folding_pointer(None);
        if e.pointer().type_() == PointerType::Mouse {
            this.mouse_hover_modifiers.set(None);
            this.set_hover_cursor(None);
        }
        Self::parent_on_pointer_exited(this, e);
    }
}

impl TextArea {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<TextArea>(true);
        Visual::clip_to_bounds_property().override_default_value::<TextArea>(true);
        register_logical_scrollable::<TextArea>();
        Self::register_input_handler_events();
        Self::register_text_drag_events();
        InputElement::text_input_method_client_requested_event().add_class_handler::<TextArea>(
            |area, e| {
                if !area.is_hosted_child_event(e) && area.editor().is_some_and(|editor| !editor.is_read_only()) {
                    e.set_client(
                        area.im_client
                            .get()
                            .map(|client| client.clone() as Rc<dyn TextInputMethodClient>),
                    );
                }
            },
        ).detach();
    }
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            editor: RefCell::new(None),
            services: OnceCell::new(),
            viewport: Cell::new(Size::default()),
            offset: Cell::new(Vector::default()),
            extent: Cell::new(Size::default()),
            horizontal: Cell::new(true),
            vertical: Cell::new(true),
            cache: RefCell::new(BTreeMap::new()),
            watermark_layout: RefCell::new(None),
            visual_lines: VisualLineHost::new(),
            viewport_events: viewport_events::ViewportEvents::default(),
            automation_format_changes: automation::AutomationFormatChanges::default(),
            background_layers: BackgroundRendererHost::new(),
            layers: LayerHost::default(),
            margins: MarginHost::default(),
            inline_controls: OnceCell::new(),
            preparing_inline_controls: Cell::new(false),
            heights: RefCell::new(HeightTree::new(1, 20.0)),
            explicit_line_collapses: RefCell::new(None),
            explicit_collapse_revision: Cell::new(0),
            scroll_invalidated: Rc::new(HandlerList::new()),
            rendering: Cell::new(false),
            redraw_pending: Cell::new(PendingRedraw::None),
            scroll_notification_pending: Cell::new(false),
            pointer_selection: RefCell::new(None),
            touch_press: Cell::new(None),
            pointer_hover: OnceCell::new(),
            text_cursor: OnceCell::new(),
            hover_cursor: RefCell::new(None),
            mouse_hover_modifiers: Cell::new(None),
            hover_cursor_refresh_pending: Cell::new(false),
            input_handlers: input::InputHandlers::default(),
            updating_pointer_selection: Cell::new(false),
            pointer_selection_refresh_pending: Cell::new(false),
            drop_caret: Cell::new(None),
            box_caret: Cell::new(None),
            context_caret: Cell::new(None),
            default_text_metrics: Cell::new(None),
            caret_visible: Cell::new(true),
            caret_at_end_of_line: Cell::new(false),
            pending_undo_caret: Cell::new(None),
            caret_geometry_pending: Cell::new(false),
            blink_timer: OnceCell::new(),
            im_client: OnceCell::new(),
            preedit: RefCell::new(None),
            highlight_pending: Cell::new(None),
            folds: RefCell::new(Vec::new()),
            desired_x: Cell::new(None),
            moving_vertically: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn set_editor(&self, editor: WeakRef<TextEditor>) {
        *self.editor.borrow_mut() = Some(editor);
        if let Some(editor) = self.editor() {
            self.update_line_number_gutter(&editor);
        }
        self.update_text_drag_option();
        self.invalidate_margin_viewport();
    }
    pub(crate) fn update_text_drag_option(&self) {
        let enabled = self.uses_default_editor_input()
            && self.editor().is_some_and(|editor| editor.options().enable_text_drag_drop);
        DragDrop::set_allow_drop(self, enabled);
        if !enabled {
            self.set_drop_caret(None);
        }
    }
    pub fn editor(&self) -> Option<Ref<TextEditor>> {
        self.editor.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Hides the inclusive range of live document lines for a multiline visual
    /// element. Keep its owning line visible and collapse the following rows.
    /// The section remains active until restored, removed, or detached from the document.
    pub fn collapse_lines(
        &self,
        first: &LiveDocumentLine,
        last: &LiveDocumentLine,
    ) -> Result<CollapsedLineSection, VisualLineError> {
        let editor = self
            .editor()
            .ok_or_else(|| VisualLineError::Extension("The text area has no document".into()))?;
        let document = editor.document();
        let start = first.line_number()?;
        let end = last.line_number()?;
        for (number, line) in [(start, first), (end, last)] {
            if !document
                .get_live_line_by_number(number)
                .is_ok_and(|live| live == *line)
            {
                return Err(DocumentError::DifferentDocument.into());
            }
        }
        if start > end {
            return Err(VisualLineError::InvalidCollapseRange { start, end });
        }
        self.detach_collapses_from_other_document(Some(&document));
        let section = self
            .explicit_line_collapses
            .borrow_mut()
            .get_or_insert_with(|| CollapsedLineSections::new(document))
            .collapse(self.to_ref().downgrade(), (first.clone(), last.clone()));
        self.refresh_explicit_line_collapses();
        Ok(section)
    }

    /// Total content height in device-independent pixels, excluding blank
    /// space added to allow scrolling below the document.
    pub fn document_height(&self) -> f64 {
        self.editor().map_or(0.0, |_| self.heights.borrow().total())
    }

    /// Resolves a document-space vertical position to a visible logical row.
    pub fn get_document_line_by_visual_top(&self, top: f64) -> Option<LiveDocumentLine> {
        let editor = self.editor()?;
        let index = self.heights.borrow().line_at(top.max(0.0));
        editor.document().get_live_line_by_number(index + 1).ok()
    }

    pub(crate) fn detach_collapses_from_other_document(&self, document: Option<&TextDocument>) {
        let stale = self
            .explicit_line_collapses
            .borrow()
            .as_ref()
            .is_some_and(|binding| Some(&binding.document) != document);
        if stale {
            self.explicit_line_collapses.take();
        }
    }

    pub(crate) fn refresh_explicit_line_collapses(&self) {
        let Some(editor) = self.editor() else {
            self.detach_collapses_from_other_document(None);
            return;
        };
        let document = editor.document();
        self.explicit_collapse_revision
            .set(self.explicit_collapse_revision.get().wrapping_add(1));
        self.sync_collapsed_line_heights(&document);
        let evicted = {
            let mut cache = self.cache.borrow_mut();
            let heights = self.heights.borrow();
            let keys: Vec<_> = cache
                .keys()
                .copied()
                .filter(|&index| heights.is_hidden(index))
                .collect();
            keys.into_iter()
                .filter_map(|key| cache.remove(&key))
                .collect::<Vec<_>>()
        };
        drop(evicted);
        self.invalidate_line_measure();
        self.request_redraw();
        self.update_extent();
        self.notify_automation_format_change(document);
    }

    pub(crate) fn component_services(&self) -> Rc<FerroLocator> {
        self.services.get_or_init(|| crate::text_editor_component::view_services(self)).clone()
    }
    pub fn overstrike_mode(&self) -> bool {
        self.get_value(Self::overstrike_mode_property())
    }

    pub fn set_overstrike_mode(&self, enabled: bool) {
        self.set_current_value(Self::overstrike_mode_property(), enabled);
    }

    /// Whether a secondary mouse click places the text caret.
    pub fn right_click_moves_caret(&self) -> bool {
        self.get_value(Self::right_click_moves_caret_property())
    }

    pub fn set_right_click_moves_caret(&self, enabled: bool) {
        self.set_current_value(Self::right_click_moves_caret_property(), enabled);
    }
    pub fn cached_visual_line_count(&self) -> usize {
        self.cache.borrow().len()
    }
    /// The hint displayed over an empty document.
    pub fn watermark(&self) -> Option<String> {
        self.get_value(Self::watermark_property())
    }
    pub fn set_watermark(&self, value: Option<String>) {
        self.set_value(Self::watermark_property(), value);
    }
    pub fn line_height(&self) -> f64 {
        self.default_text_metrics().minimum_line_height
    }

    /// Width of the shaped `x` used for a virtual editor column.
    pub fn wide_space_width(&self) -> f64 {
        self.default_text_metrics().wide_space_width
    }

    fn default_text_metrics(&self) -> DefaultTextMetrics {
        if let Some(metrics) = self.default_text_metrics.get() {
            return metrics;
        }
        let Some(editor) = self.editor() else {
            return DefaultTextMetrics {
                height: 20.0,
                baseline: 16.0,
                minimum_line_height: 20.0,
                wide_space_width: 7.0,
            };
        };
        let layout = FormattedTextElement::prepare_text("x", Rc::new(editor.default_text_run_properties()));
        let height = layout.height().max(1.0);
        let factor = editor.options().line_height_factor.get();
        let metrics = DefaultTextMetrics {
            height,
            baseline: layout.baseline().max(1.0),
            wide_space_width: layout.width_including_trailing_whitespace().max(1.0),
            // Keep the minimum representable by drawing backends with f32 coordinates.
            minimum_line_height: (height * factor).min(f64::from(f32::MAX)),
        };
        self.default_text_metrics.set(Some(metrics));
        metrics
    }
    fn gutter_width(&self) -> f64 {
        self.custom_margin_width() + self.editor().map_or(6.0, |editor| {
            (if editor.show_line_numbers() {
                self.line_number_strip_width(&editor) + Self::line_number_separator_width(&editor)
            } else {
                6.0
            }) + self.folding_gutter_width()
        })
    }

    fn text_viewport_contains(&self, point: Point) -> bool {
        let viewport = self.viewport.get();
        point.x >= self.gutter_width()
            && point.x < viewport.width
            && point.y >= 0.0
            && point.y < viewport.height
    }

    fn update_hover_cursor(&self, event: &PointerEventArgs, point: Point, query_elements: bool) {
        if event.pointer().type_() != PointerType::Mouse {
            return;
        }
        let modifiers = event.key_modifiers();
        self.mouse_hover_modifiers.set(Some(modifiers));
        if self.is_hosted_child_event(event) || !self.text_viewport_contains(point) {
            self.set_hover_cursor(None);
            return;
        }
        self.apply_text_hover_cursor(point, modifiers, query_elements);
    }

    fn update_hover_modifiers(&self, modifiers: KeyModifiers) {
        if self.mouse_hover_modifiers.get().is_some_and(|old| old != modifiers) {
            self.mouse_hover_modifiers.set(Some(modifiers));
            self.invalidate_hover_cursor();
        }
    }

    fn invalidate_hover_cursor(&self) {
        if self.mouse_hover_modifiers.get().is_none()
            || !self.is_pointer_over()
            || self.pointer_selection.borrow().is_some()
            || self.hover_cursor_refresh_pending.replace(true)
        {
            return;
        }
        let weak = self.to_ref().downgrade();
        // Layout and scrolling can move links or hosted controls under a resting mouse.
        // Resolve the current hit after layout has committed.
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(area) = weak.upgrade() {
                    area.hover_cursor_refresh_pending.set(false);
                    area.refresh_hover_cursor();
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    fn refresh_hover_cursor(&self) {
        let Some(modifiers) = self.mouse_hover_modifiers.get() else { return };
        if !self.is_pointer_over() || self.pointer_selection.borrow().is_some() {
            return;
        }
        let Some(root) = TopLevel::get_top_level(Some(self)) else { return };
        let Some(position) = root.last_pointer_position() else { return };
        let position = root.point_to_client(position);
        let point = root.translate_point(position, self)
            .filter(|&point| self.text_viewport_contains(point))
            .filter(|_| {
                root.input_hit_test(position).is_some_and(|hit| {
                    (self.to_ref().ptr_eq(&hit) || self.is_visual_ancestor_of(&hit))
                        && !self.is_hosted_child_visual(&hit.upcast())
                })
            });
        if let Some(point) = point {
            self.apply_text_hover_cursor(point, modifiers, self.uses_default_editor_input());
        } else {
            self.set_hover_cursor(None);
        }
    }

    fn apply_text_hover_cursor(&self, point: Point, modifiers: KeyModifiers, query_elements: bool) {
        if FerroLocator::current().get_service::<dyn ICursorFactory>().is_none() {
            return;
        }
        let cursor = if query_elements {
            self.element_at_point(point)
                .and_then(|(line, index)| line.display.elements()[index].query_cursor(modifiers))
        } else {
            None
        };
        let cursor = cursor.unwrap_or_else(|| {
            self.text_cursor.get_or_init(|| Cursor::new(StandardCursorType::Ibeam)).clone()
        });
        self.set_hover_cursor(Some(cursor));
    }

    fn set_hover_cursor(&self, cursor: Option<Rc<Cursor>>) {
        if self.hover_cursor.borrow().as_ref().map(|(cursor, _)| cursor) == cursor.as_ref() {
            return;
        }
        let next = cursor.and_then(|cursor| {
            let value = self.set_value_with_priority(
                InputElement::cursor_property(),
                Some(cursor.clone()),
                BindingPriority::Template,
            )?;
            Some((cursor, Subscription::new(value)))
        });
        // Install the new value first so switching cursors never reveals the inherited cursor.
        let previous = self.hover_cursor.replace(next);
        drop(previous);
    }

    pub(crate) fn invalidate_layout_cache(&self) {
        self.watermark_layout.take();
        self.visual_lines.end_of_line_layouts.take();
        self.reset_line_metrics();
        self.extent.set(Size::default());
        self.invalidate_line_measure();
        self.request_redraw();
        self.update_extent();
    }

    pub(crate) fn request_redraw(&self) {
        self.schedule_redraw(PendingRedraw::Requested);
    }

    fn retry_invalidated_frame(&self) {
        self.schedule_redraw(PendingRedraw::InvalidatedFrame);
    }

    fn schedule_redraw(&self, requested: PendingRedraw) {
        self.refresh_margin_viewport();
        if !self.rendering.get() {
            self.invalidate_visual();
            self.invalidate_native_layer_planes();
            self.invalidate_builtin_margins();
            return;
        }
        let pending = self.redraw_pending.get();
        if pending == PendingRedraw::None || requested == PendingRedraw::InvalidatedFrame {
            self.redraw_pending.set(requested);
        }
        if pending != PendingRedraw::None {
            return;
        }
        let weak = self.to_ref().downgrade();
        // Renderer and extension callbacks may request a new frame while the
        // compositor is recording this one. Dirty the visual after recording.
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(area) = weak.upgrade() {
                    area.redraw_pending.set(PendingRedraw::None);
                    area.invalidate_visual();
                    area.invalidate_native_layer_planes();
                    area.invalidate_builtin_margins();
                }
            },
            DispatcherPriority::NORMAL,
        );
    }
    fn reset_line_metrics(&self) {
        self.clear_visual_line_cache();
        self.visual_lines.collapsed.borrow_mut().clear();
        self.default_text_metrics.set(None);
        let mut active = Vec::<FoldingSection>::new();
        let document = self.editor().map(|editor| editor.document());
        self.detach_collapses_from_other_document(document.as_ref());
        if let Some(document) = &document {
            let line_height = self.line_height();
            self.heights
                .borrow_mut()
                .reset(document.line_count(), line_height);
            if let Some(manager) = self.editor().and_then(|editor| editor.folding_manager()) {
                let mut sections = manager.all_foldings();
                sections.sort_by_key(|s| (s.start_offset(), std::cmp::Reverse(s.end_offset())));
                active.extend(sections.into_iter().filter(|s| s.is_folded()));
            }
        }
        *self.folds.borrow_mut() = active;
        if let Some(document) = &document {
            self.sync_collapsed_line_heights(document);
        }
    }

    fn sync_collapsed_line_heights(&self, document: &TextDocument) {
        let mut collapsed: Vec<_> = self
            .visual_lines
            .collapsed
            .borrow()
            .iter()
            .map(|(&first, &last)| (first, last))
            .collect();
        let mut add_range = |start, end| {
            if let (Ok(first), Ok(last)) = (
                document.get_line_by_offset(start),
                document.get_line_by_offset(end),
            ) {
                collapsed.push((first.line_number(), last.line_number()));
            }
        };
        for fold in self.folds.borrow().iter() {
            add_range(fold.start_offset(), fold.end_offset());
        }
        if self.preedit.borrow().is_some() {
            if let Some(editor) = self.editor() {
                let selection = editor.selection().primary();
                add_range(selection.start_offset(), selection.end_offset());
            }
        }
        if let Some(sections) = self.explicit_line_collapses.borrow().as_ref() {
            sections.extend_intervals(&mut collapsed);
        }
        self.heights.borrow_mut().set_collapsed_preserving_heights(&collapsed);
    }
    pub(crate) fn document_changed(&self) {
        self.set_drop_caret(None);
        if let Some(client) = self.im_client.get() {
            client.discard_stale_preview(self);
        }
        self.invalidate_layout_cache();
        if let Some(client) = self.im_client.get() {
            client.raise_surrounding_text_changed();
            client.raise_selection_changed();
        }
    }
    pub(crate) fn selection_changed_with_affinity(&self, at_end_of_line: bool) {
        if let Some(client) = self.im_client.get() {
            client.discard_stale_preview(self);
        }
        if self.has_preedit() {
            self.reset_line_metrics();
        }
        self.box_caret.set(None);
        self.context_caret.set(None);
        self.apply_caret_affinity(at_end_of_line);
    }

    fn apply_caret_affinity(&self, at_end_of_line: bool) {
        self.pending_undo_caret.set(None);
        self.caret_at_end_of_line.set(at_end_of_line);
        if !self.moving_vertically.get() {
            self.desired_x.set(None);
        }
        self.caret_visible.set(true);
        self.update_caret_geometry();
        self.request_redraw();
    }
    fn update_caret_geometry(&self) {
        if self.rendering.get() || self.visual_line_construction_in_progress() {
            self.defer_caret_geometry();
            return;
        }
        self.bring_caret_into_view();
        if let Some(client) = self.im_client.get() {
            // Moving the selection may change which logical lines the IME sees.
            client.raise_surrounding_text_changed();
            client.raise_selection_changed();
            client.raise_cursor_rectangle_changed();
        }
    }
    fn defer_caret_geometry(&self) {
        if self.caret_geometry_pending.replace(true) {
            return;
        }
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(area) = weak.upgrade() {
                    area.caret_geometry_pending.set(false);
                    area.update_caret_geometry();
                }
            },
            DispatcherPriority::NORMAL,
        );
    }
    fn update_extent(&self) {
        self.update_extent_with_width(self.extent.get().width);
    }

    fn update_extent_with_width(&self, retained_width: f64) {
        let viewport = self.viewport.get();
        let below = self
            .editor()
            .is_some_and(|e| e.options().allow_scroll_below_document);
        let height = self.heights.borrow().total()
            + if below {
                (viewport.height - self.line_height()).max(0.0)
            } else {
                0.0
            };
        let width = if self.editor().is_some_and(|e| e.word_wrap()) {
            viewport.width
        } else {
            self.cache
                .borrow()
                .values()
                .map(|l| l.layout.width_including_trailing_whitespace() + self.gutter_width() + 8.0)
                .fold(retained_width, f64::max)
                .max(viewport.width)
        };
        let virtual_width = self.editor().map_or(0.0, |editor| {
            let primary = editor.selection().primary();
            if primary.caret_virtual_space == 0 {
                return 0.0;
            }
            let Ok(document_line) = editor.document().get_line_by_offset(primary.caret) else {
                return 0.0;
            };
            let index = self
                .heights
                .borrow()
                .visible_owner(document_line.line_number() - 1);
            let line = self.cache.borrow().get(&index).cloned();
            line.map_or(0.0, |line| {
                let edge = Self::virtual_space_bounds(&line.layout);
                edge.x
                    + primary.caret_virtual_space as f64 * self.wide_space_width()
                    + self.gutter_width()
                    + 8.0
            })
        });
        let width = width.max(virtual_width);
        let extent = Size::new(width, height.max(viewport.height));
        if self.extent.replace(extent) != extent {
            self.raise_scroll_invalidated();
        }
    }
    fn line_layout(&self, index: usize) -> Option<Rc<CachedVisualLine>> {
        if self.heights.borrow().is_hidden(index) {
            return None;
        }
        match self.try_line_layout(index) {
            Ok(line) => Some(line),
            Err(error) => {
                if let Some(editor) = self.editor() {
                    editor.report_error(error.to_string());
                }
                None
            }
        }
    }
    fn schedule_highlight(&self, target: usize) {
        let pending = self.highlight_pending.get();
        self.highlight_pending.set(Some(pending.map_or(target, |pending| pending.max(target))));
        if pending.is_some() {
            return;
        }
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(area) = weak.upgrade() else { return };
                let Some(target) = area.highlight_pending.take() else { return };
                let Some(editor) = area.editor() else { return };
                let Some(installation) = editor.textmate_installation() else { return };
                let document = editor.document();
                let visible_was_ready = installation.cached_through(&document, target);
                let background = installation.background_target(&document, target);
                let done = installation.advance_to(&document, background, 256);
                if !visible_was_ready {
                    area.clear_visual_line_cache();
                    area.request_redraw();
                }
                if !done && installation.last_error().is_none() {
                    area.schedule_highlight(target);
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    pub fn offset_at_point(&self, point: Point) -> usize {
        self.selection_position_at_point(point, false).0
    }

    fn selection_position_on_line(
        &self,
        line: &CachedVisualLine,
        point: Point,
        allow_virtual_space: bool,
    ) -> (usize, usize) {
        if allow_virtual_space {
            let edge = Self::virtual_space_bounds(&line.layout);
            if point.x.is_finite() && point.x > edge.x && point.y >= edge.y {
                let extra = ((point.x - edge.x) / self.wide_space_width()).round().max(0.0) as usize;
                return (
                    line.offset + line.display.document_offset(line.display.visual_length()),
                    extra,
                );
            }
        }
        let hit = line.layout.hit_test_point(point);
        let visual = (hit.text_position().max(0) as usize).min(line.display.visual_length());
        let offset = line.offset + line.display.document_offset(visual);
        (offset, 0)
    }

    fn selection_position_at_point(
        &self,
        point: Point,
        allow_virtual_space: bool,
    ) -> (usize, usize) {
        let offset = self.offset.get();
        let y = (point.y + offset.y).max(0.0);
        let index = self.heights.borrow().line_at(y);
        let Some(line) = self.line_layout(index) else {
            return (0, 0);
        };
        let top = self.heights.borrow().top(index);
        self.selection_position_on_line(
            &line,
            Point::new(point.x + offset.x - self.gutter_width(), y - top),
            allow_virtual_space,
        )
    }

    /// Resolves a viewport point through the actual formatter, including tabs,
    /// folded text and optional cells beyond the final glyph of a logical line.
    pub fn position_at_point(
        &self,
        point: Point,
        allow_virtual_space: bool,
    ) -> Option<crate::TextViewPosition> {
        let editor = self.editor()?;
        let (offset, extra) = self.selection_position_at_point(point, allow_virtual_space);
        let location = editor.document().get_location(offset).ok()?;
        let index = self.heights.borrow().visible_owner(location.line - 1);
        let line = self.line_layout(index)?;
        if !line
            .display
            .is_document_offset_visible(offset - line.offset)
        {
            return None;
        }
        let visual = line.display.visual_offset(offset - line.offset);
        let mut result =
            crate::TextViewPosition::with_visual_column(location, visual.saturating_add(extra));
        let normal = line.layout.hit_test_text_position(visual as i32);
        let local_y = point.y + self.offset.get().y - self.heights.borrow().top(index);
        result.is_at_end_of_line = extra == 0 && local_y < normal.y;
        Some(result)
    }

    pub(crate) fn caret_state(
        &self,
        document: &TextDocument,
        selection: &Selection,
    ) -> Option<CaretState> {
        let navigation = self.navigation_caret(selection);
        let location = document.get_location(navigation.offset).ok()?;
        Some(CaretState { location, navigation })
    }

    pub(crate) fn caret_position(&self) -> Option<crate::TextViewPosition> {
        let editor = self.editor()?;
        let CaretState {
            location,
            navigation: caret,
        } = self.caret_state(&editor.document(), &editor.selection())?;
        let mut result = crate::TextViewPosition::with_visual_column(
            location,
            self.visual_column_for_offset(caret.offset)?.saturating_add(caret.virtual_space),
        );
        result.is_at_end_of_line = caret.at_end_of_row;
        Some(result)
    }

    pub(crate) fn visual_column_for_offset(&self, offset: usize) -> Option<usize> {
        let editor = self.editor()?;
        let logical = editor.document().get_line_by_offset(offset).ok()?;
        if self.viewport.get().width > 0.0 {
            let index = self
                .heights
                .borrow()
                .visible_owner(logical.line_number() - 1);
            let line = self.line_layout(index)?;
            return Some(line.display.visual_offset(offset - line.offset));
        }
        let source = editor
            .document()
            .get_text_utf16(logical.offset(), logical.length())
            .ok()?;
        let tab_size = editor.options().indentation_size.clamp(1, 256);
        Some(
            source
                .iter()
                .take(offset - logical.offset())
                .fold(0, |column, unit| {
                    column
                        + if *unit == 9 {
                            tab_size - column % tab_size
                        } else {
                            1
                        }
                }),
        )
    }

    fn caret_bounds(layout: &TextLayout, visual: usize, at_end: bool) -> Rect {
        if !at_end {
            return layout.hit_test_text_position(visual as i32);
        }
        let index = layout
            .get_line_index_from_character_index(visual as i32, true)
            .max(0) as usize;
        let Some(row) = layout.text_lines().get(index) else {
            return Rect::default();
        };
        Rect::new(
            row.get_distance_from_character_hit(ferroui_base::media::CharacterHit::new(
                visual as i32,
            )),
            layout
                .text_lines()
                .iter()
                .take(index)
                .map(|line| line.height())
                .sum(),
            1.0,
            row.height(),
        )
    }

    /// Inserts rectangle clipboard rows at the same shaped horizontal
    /// position without temporarily changing the live selection. Folded rows
    /// are omitted. A view that has not been arranged uses core cell metrics.
    pub(crate) fn paste_rectangle(
        &self,
        text: &str,
        select_inserted_text: bool,
    ) -> Result<bool, ferroui_edit_core::editing::EditError> {
        let Some(editor) = self.editor() else {
            return Ok(false);
        };
        let target = crate::text_editor::EditTarget::new(&editor);
        let session = editor.session();
        let selection = session.selection();
        let is_current = || target.is_current(&editor) && editor.selection() == selection;
        if self.viewport.get().width <= 0.0 {
            return session.paste_rectangle_checked(text, select_inserted_text, is_current);
        }
        if !selection.is_empty() || selection.segments().len() != 1 {
            return Ok(false);
        }
        let positions = self.rectangle_paste_positions(
            &editor,
            self.navigation_caret(&selection),
            text,
            selection.is_rectangular(),
        )?;
        if !is_current() {
            return Err(ferroui_edit_core::editing::EditError::StaleEditPlan);
        }
        let Some(positions) = positions else {
            return Ok(false);
        };
        session.paste_rectangle_at_checked(&positions, text, select_inserted_text, is_current)?;
        Ok(true)
    }

    fn rectangle_paste_positions(
        &self,
        editor: &TextEditor,
        caret: NavigationCaret,
        text: &str,
        rectangle: bool,
    ) -> Result<Option<Selection>, ferroui_edit_core::editing::EditError> {
        use ferroui_edit_core::editing::EditError;
        let document = editor.document();
        let version = document.version();
        let revision = self.visual_lines.revision.get();
        let selection = editor.selection();
        let first = document.get_line_by_offset(caret.offset)?;
        let mut row_count = 1_usize;
        let mut bytes = text.bytes().peekable();
        while let Some(byte) = bytes.next() {
            if byte == b'\r' || byte == b'\n' {
                if byte == b'\r' && bytes.peek() == Some(&b'\n') {
                    bytes.next();
                }
                row_count += 1;
                if row_count > document.line_count() {
                    return Ok(None);
                }
            }
        }
        let positions = (|| {
            let mut index = self.heights.borrow().visible_owner(first.line_number() - 1);
            let mut line = self
                .try_line_layout(index)
                .map_err(|_| EditError::InvalidSelection)?;
            let x = self.caret_x_in_line(&line, caret);
            let mut positions = Vec::new();
            positions
                .try_reserve_exact(row_count)
                .map_err(|_| EditError::VirtualSpaceTooLarge)?;
            for row in 0..row_count {
                if row != 0 {
                    index = self.heights.borrow().next_visible(index + 1);
                    if index >= document.line_count() {
                        return Ok(None);
                    }
                    line = self
                        .try_line_layout(index)
                        .map_err(|_| EditError::InvalidSelection)?;
                }
                let position = self.rectangle_position_on_line(&line, x)?;
                positions.push(SelectionSegment::with_virtual_space(
                    position.offset,
                    position.virtual_space,
                    position.offset,
                    position.virtual_space,
                ));
            }
            if !editor.options().enable_virtual_space && !rectangle {
                let last = document.get_line_by_offset(positions.last().unwrap().caret)?;
                if last.length() < caret.offset - first.offset() {
                    return Ok(None);
                }
            }
            Ok(Some(Selection::from_rectangle_segments(positions, row_count - 1)))
        })();
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
            || self.visual_lines.revision.get() != revision
        {
            return Err(EditError::StaleEditPlan);
        }
        positions
    }

    fn rectangle_position_on_line(
        &self,
        line: &CachedVisualLine,
        x: f64,
    ) -> Result<NavigationCaret, ferroui_edit_core::editing::EditError> {
        self.caret_at_row_x(line, 0, x, true, false)
            .map_err(|_| ferroui_edit_core::editing::EditError::InvalidSelection)
    }

    fn rectangle_endpoint(
        &self,
        point: Point,
    ) -> Result<((usize, f64), NavigationCaret), ferroui_edit_core::editing::EditError> {
        let scroll = self.offset.get();
        let index = self.heights.borrow().line_at((point.y + scroll.y).max(0.0));
        let line = self
            .try_line_layout(index)
            .map_err(|_| ferroui_edit_core::editing::EditError::InvalidSelection)?;
        let x = point.x + scroll.x - self.gutter_width();
        let mut caret = self.rectangle_position_on_line(&line, x)?;
        caret.at_end_of_row = true;
        Ok(((index, self.caret_x_in_line(&line, caret)), caret))
    }

    /// Virtual cells follow the final formatted row's physical right edge.
    /// The logical EOL caret may be inside that row for a trailing RTL run.
    fn virtual_space_bounds(layout: &TextLayout) -> Rect {
        let rows = layout.text_lines();
        let Some(last) = rows.last() else {
            return layout.hit_test_text_position(0);
        };
        Rect::new(
            last.start() + last.width_including_trailing_whitespace(),
            rows.iter()
                .take(rows.len() - 1)
                .map(|row| row.height())
                .sum(),
            1.0,
            last.height(),
        )
    }

    fn rectangle_anchor_for_selection(&self, selection: &Selection) -> (usize, f64) {
        let primary = selection.primary();
        let segment = if selection.is_rectangular() {
            if selection.primary_index() == 0 {
                selection.segments().last().copied().unwrap_or(primary)
            } else {
                selection.segments().first().copied().unwrap_or(primary)
            }
        } else {
            primary
        };
        let index = self
            .editor()
            .and_then(|editor| editor.document().get_line_by_offset(segment.anchor).ok())
            .map_or(0, |line| {
                self.heights.borrow().visible_owner(line.line_number() - 1)
            });
        (
            index,
            self.x_for_position(segment.anchor, segment.anchor_virtual_space),
        )
    }

    fn x_for_position(&self, offset: usize, extra: usize) -> f64 {
        self.caret_x(NavigationCaret {
            offset,
            virtual_space: extra,
            at_end_of_row: false,
        })
    }

    fn is_inside_fold(&self, offset: usize) -> bool {
        self.folds
            .borrow()
            .iter()
            .any(|fold| fold.start_offset() < offset && offset < fold.end_offset())
    }

    fn can_resume_folds(&self) -> bool {
        self.has_custom_visual_line_construction() || self.preedit.borrow().is_some()
    }

    /// Gets a visible insertion rectangle in this control's coordinates.
    /// Returns `None` for invalid offsets, hidden fold interiors, or positions
    /// outside the arranged viewport. Popup hosts can anchor to a tracked
    /// document offset without moving the editor's caret.
    pub fn rectangle_for_offset(&self, offset: usize) -> Option<Rect> {
        self.rectangle_for_caret(NavigationCaret::at_offset(offset))
    }

    pub(crate) fn rectangle_for_caret(&self, mut caret: NavigationCaret) -> Option<Rect> {
        let viewport = self.viewport.get();
        if viewport.width <= 0.0 || viewport.height <= 0.0 {
            return None;
        }
        let editor = self.editor()?;
        let offset = caret.offset;
        let document_line = editor.document().get_line_by_offset(offset).ok()?;
        let inside_fold = self.is_inside_fold(offset);
        if inside_fold && !self.can_resume_folds() {
            return None;
        }
        let index = self
            .heights
            .borrow()
            .visible_owner(document_line.line_number() - 1);
        let scroll = self.offset.get();
        let top = self.heights.borrow().top(index) - scroll.y;
        if top >= viewport.height {
            return None;
        }
        let line = self.line_layout(index)?;
        if inside_fold && !line.starts_fold_at(offset) {
            return None;
        }
        if !line
            .display
            .is_document_offset_visible(offset - line.offset)
        {
            return None;
        }
        if offset != document_line.end_offset() {
            caret.virtual_space = 0;
        }
        let rect = self.line_caret_bounds(&line, caret, false, false);
        let result = Rect::new(
            rect.x + self.gutter_width() - scroll.x,
            rect.y + top,
            1.0,
            rect.height.max(self.line_height()),
        );
        (result.y + result.height > 0.0
            && result.y < viewport.height
            && result.x + result.width > self.gutter_width()
            && result.x < viewport.width)
            .then_some(result)
    }

    fn select_rectangle_to(
        &self,
        anchor: (usize, f64),
        caret: (usize, f64),
        position: NavigationCaret,
        desired_x: Option<f64>,
    ) -> Result<(), ferroui_edit_core::editing::EditError> {
        let editor = self
            .editor()
            .ok_or(ferroui_edit_core::editing::EditError::InvalidSelection)?;
        let document = editor.document();
        let version = document.version();
        let revision = self.visual_lines.revision.get();
        let previous = editor.selection();
        let count = document.line_count();
        let first = anchor.0.min(caret.0).min(count - 1);
        let last = anchor.0.max(caret.0).min(count - 1);
        let mut segments = Vec::new();
        let mut index = self.heights.borrow().visible_owner(first);
        while index <= last {
            let line = self
                .line_layout(index)
                .ok_or(ferroui_edit_core::editing::EditError::InvalidSelection)?;
            let a = self.rectangle_position_on_line(&line, anchor.1)?;
            let c = self.rectangle_position_on_line(&line, caret.1)?;
            segments.push(SelectionSegment::with_virtual_space(
                a.offset,
                a.virtual_space,
                c.offset,
                c.virtual_space,
            ));
            index = self.heights.borrow().next_visible(index + 1);
        }
        let primary = if caret.0 < anchor.0 {
            0
        } else {
            segments.len().saturating_sub(1)
        };
        use ferroui_edit_core::editing::EditError;
        if editor.document() != document
            || document.version() != version
            || editor.selection() != previous
            || self.visual_lines.revision.get() != revision
        {
            return Err(EditError::StaleEditPlan);
        }
        let old_caret = self.navigation_caret(&previous);
        let selection = Selection::from_rectangle_segments(segments, primary);
        let projected_primary = selection.primary();
        editor.session().set_selection(selection)?;
        self.context_caret.set(None);
        self.box_caret.set(Some(BoxCaret { anchor, position, projected_primary }));
        self.desired_x.set(desired_x);
        self.moving_vertically.set(desired_x.is_some());
        self.apply_caret_affinity(position.at_end_of_row);
        self.moving_vertically.set(false);
        editor.notify_selection_changed(old_caret != position);
        Ok(())
    }

    /// Selects a rectangle using viewport points snapped to caret positions.
    /// The active edge and selected segments use the first formatted row of
    /// each visible logical line. Collapsed continuations are skipped.
    pub fn set_rectangular_selection(
        &self,
        anchor: Point,
        caret: Point,
    ) -> Result<(), ferroui_edit_core::editing::EditError> {
        use ferroui_edit_core::editing::EditError;
        let editor = self.editor().ok_or(EditError::InvalidSelection)?;
        let document = editor.document();
        let version = document.version();
        let revision = self.visual_lines.revision.get();
        let selection = editor.selection();
        let anchor = self.navigation_caret_at_point(anchor, true);
        let index = self
            .line_owner(&document, anchor.offset)
            .map_err(|_| EditError::InvalidSelection)?;
        let anchor = (index, self.caret_x(anchor));
        let (target, position) = self.rectangle_endpoint(caret)?;
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
            || self.visual_lines.revision.get() != revision
        {
            return Err(EditError::StaleEditPlan);
        }
        self.select_rectangle_to(anchor, target, position, None)
    }
    pub fn cursor_rectangle(&self) -> Rect {
        if self.visual_line_construction_in_progress() {
            self.defer_caret_geometry();
            return Rect::default();
        }
        if self.viewport.get().width <= 0.0 || self.viewport.get().height <= 0.0 {
            return Rect::default();
        }
        let Some(editor) = self.editor() else {
            return Rect::default();
        };
        let selection = editor.selection();
        let mut caret = self.navigation_caret(&selection);
        let composing = self.preedit.borrow().is_some();
        if composing {
            caret.offset = selection.primary().start_offset();
        }
        self.navigation_rectangle(
            caret,
            self.overstrike_mode() && selection.is_empty(),
            composing,
        )
    }

    fn display_caret_bounds(
        &self,
        line: &CachedVisualLine,
        visible: usize,
        virtual_cells: usize,
        trailing: bool,
        overstrike: bool,
    ) -> Rect {
        let layout = &line.layout;
        if virtual_cells > 0 {
            let width = if overstrike { self.wide_space_width() } else { 1.0 };
            return Self::virtual_space_bounds(layout).with_width(width.max(1.0));
        }
        if !overstrike {
            return Self::caret_bounds(layout, visible, trailing).with_width(1.0);
        }
        if visible < line.display.visual_length() {
            let column = visible as i32;
            let row = layout.get_line_index_from_character_index(column, false);
            if let Some(row) = layout.text_lines().get(row.max(0) as usize) {
                let hit = row.get_next_caret_character_hit(
                    ferroui_base::media::CharacterHit::new(column),
                );
                let end = (hit.first_character_index() + hit.trailing_length())
                    .min(line.display.visual_length() as i32);
                if end > column {
                    let bounds = layout.hit_test_text_range(column, end - column);
                    if let Some(rectangle) = bounds.first() {
                        return rectangle.with_width(rectangle.width.max(1.0));
                    }
                }
            }
            return Self::caret_bounds(layout, visible, false).with_width(1.0);
        }
        Self::caret_bounds(layout, visible, trailing).with_width(self.wide_space_width())
    }

    fn line_caret_bounds(
        &self,
        line: &CachedVisualLine,
        caret: NavigationCaret,
        overstrike: bool,
        composing: bool,
    ) -> Rect {
        if let (true, Some((start, length))) = (composing, line.preedit) {
            let cursor = self
                .preedit
                .borrow()
                .as_ref()
                .and_then(|(_, cursor)| *cursor)
                .map_or(length, |cursor| (cursor.max(0) as usize).min(length));
            return Self::caret_bounds(&line.layout, start + cursor, false).with_width(1.0);
        }
        let visible = line.display.visual_offset(caret.offset - line.offset);
        self.display_caret_bounds(
            line,
            visible,
            caret.virtual_space,
            caret.at_end_of_row,
            overstrike,
        )
        .translate(Vector::new(caret.virtual_space as f64 * self.wide_space_width(), 0.0))
    }

    fn navigation_rectangle(&self, caret: NavigationCaret, overstrike: bool, composing: bool) -> Rect {
        let Some(editor) = self.editor() else {
            return Rect::default();
        };
        let Ok(document_line) = editor.document().get_line_by_offset(caret.offset) else {
            return Rect::default();
        };
        let index = self
            .heights
            .borrow()
            .visible_owner(document_line.line_number() - 1);
        let Some(line) = self.line_layout(index) else {
            return Rect::default();
        };
        let r = self.line_caret_bounds(&line, caret, overstrike, composing);
        let scroll = self.offset.get();
        Rect::new(
            r.x + self.gutter_width() - scroll.x,
            r.y + self.heights.borrow().top(index) - scroll.y,
            r.width,
            r.height.max(self.line_height()),
        )
    }
    pub fn bring_caret_into_view(&self) {
        if self.visual_line_construction_in_progress() {
            self.defer_caret_geometry();
            return;
        }
        if let Some(editor) = self.editor() {
            let offset = editor.caret_offset();
            if self.is_inside_fold(offset) {
                let document = editor.document();
                let version = document.version();
                let revision = self.visual_lines.revision.get();
                let viewport = self.viewport.get();
                let at_marker = viewport.width > 0.0
                    && viewport.height > 0.0
                    && self.can_resume_folds()
                    && document.get_line_by_offset(offset).ok().is_some_and(|logical| {
                        let index = self.heights.borrow().visible_owner(logical.line_number() - 1);
                        self.line_layout(index).is_some_and(|line| line.starts_fold_at(offset))
                    });
                if document != editor.document()
                    || document.version() != version
                    || self.visual_lines.revision.get() != revision
                    || editor.caret_offset() != offset
                {
                    return;
                }
                if let Some(manager) = editor.folding_manager().filter(|_| !at_marker) {
                    manager.unfold_at(offset);
                }
            }
        }
        if self.viewport.get().width <= 0.0 || self.viewport.get().height <= 0.0 {
            return;
        }
        let r = self.cursor_rectangle();
        let size = self.viewport.get();
        let mut offset = self.offset.get();
        if r.y < 0.0 {
            offset.y += r.y;
        } else if r.y + r.height > size.height {
            offset.y += r.y + r.height - size.height;
        }
        if r.x < self.gutter_width() {
            offset.x += r.x - self.gutter_width();
        } else if r.x + 8.0 > size.width {
            offset.x += r.x + 8.0 - size.width;
        }
        self.update_extent();
        self.set_offset(offset);
    }
    pub(crate) fn has_preedit(&self) -> bool {
        self.preedit.borrow().is_some()
    }

    pub(crate) fn set_preedit(&self, text: Option<&str>, cursor: Option<i32>) {
        let text = text.filter(|text| !text.is_empty());
        let content_changed = {
            let mut preedit = self.preedit.borrow_mut();
            if preedit.as_ref().map(|(current, _)| current.as_str()) == text {
                let Some((_, position)) = preedit.as_mut() else { return };
                if *position == cursor {
                    return;
                }
                *position = cursor;
                false
            } else {
                *preedit = text.map(|text| (text.to_owned(), cursor));
                true
            }
        };
        if let Some(client) = self.im_client.get() {
            client.preview_changed();
        }
        if content_changed {
            self.reset_line_metrics();
        }
        self.request_redraw();
        if let Some(client) = self.im_client.get() {
            client.raise_cursor_rectangle_changed();
        }
    }
}

impl IScrollable for TextArea {
    fn extent(&self) -> Size {
        self.extent.get()
    }
    fn viewport(&self) -> Size {
        self.viewport.get()
    }
    fn offset(&self) -> Vector {
        self.offset.get()
    }
    fn set_offset(&self, value: Vector) {
        let extent = self.extent.get();
        let viewport = self.viewport.get();
        let value = Vector::new(
            if self.horizontal.get() && value.x.is_finite() {
                value.x.clamp(0.0, (extent.width - viewport.width).max(0.0))
            } else {
                0.0
            },
            if self.vertical.get() && value.y.is_finite() {
                value
                    .y
                    .clamp(0.0, (extent.height - viewport.height).max(0.0))
            } else {
                0.0
            },
        );
        let previous = self.offset.replace(value);
        if previous != value {
            self.raise_scroll_offset_changed(previous, value);
            if self.inline_controls.get().is_some() {
                self.invalidate_line_measure();
            }
            self.request_redraw();
            self.raise_scroll_invalidated();
            self.refresh_pointer_selection_after_scroll();
            self.invalidate_hover_cursor();
            if let Some(client) = self.im_client.get() {
                client.raise_cursor_rectangle_changed();
            }
        }
    }
    fn can_horizontally_scroll(&self) -> bool {
        self.horizontal.get()
    }
    fn can_vertically_scroll(&self) -> bool {
        self.vertical.get()
    }
}
impl ILogicalScrollable for TextArea {
    fn set_can_horizontally_scroll(&self, value: bool) {
        self.horizontal.set(value);
        self.set_offset(self.offset.get());
    }
    fn set_can_vertically_scroll(&self, value: bool) {
        self.vertical.set(value);
        self.set_offset(self.offset.get());
    }
    fn is_logical_scroll_enabled(&self) -> bool {
        true
    }
    fn scroll_size(&self) -> Size {
        Size::new(self.line_height(), self.line_height() * 3.0)
    }
    fn page_scroll_size(&self) -> Size {
        self.viewport.get()
    }
    fn scroll_invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.scroll_invalidated.add(handler);
        let handlers = self.scroll_invalidated.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
    fn raise_scroll_invalidated(&self) {
        if self.rendering.get() {
            // Measured line heights/widths become available during drawing.
            // Scrollbar visibility can invalidate layout, which the compositor
            // forbids during the render pass. Publish once after it completes.
            if !self.scroll_notification_pending.replace(true) {
                let weak = self.to_ref().downgrade();
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(area) = weak.upgrade() {
                            area.scroll_notification_pending.set(false);
                            area.raise_scroll_invalidated();
                        }
                    },
                    DispatcherPriority::NORMAL,
                );
            }
            return;
        }
        for (_, handler) in self.scroll_invalidated.snapshot().iter() {
            handler();
        }
    }
    fn bring_into_view(&self, target: &Ref<Control>, rect: Rect) -> bool {
        if !target.ptr_eq(&self.to_ref()) {
            return false;
        }
        let old = self.offset.get();
        let mut next = old;
        let viewport = self.viewport.get();
        if rect.x < self.gutter_width() {
            next.x += rect.x - self.gutter_width();
        } else if rect.x + rect.width > viewport.width {
            next.x += rect.x + rect.width - viewport.width;
        }
        if rect.y < 0.0 {
            next.y += rect.y;
        } else if rect.y + rect.height > viewport.height {
            next.y += rect.y + rect.height - viewport.height;
        }
        self.set_offset(next);
        self.offset.get() != old
    }
    fn get_control_in_direction(
        &self,
        _direction: NavigationDirection,
        _from: Option<&Ref<Control>>,
    ) -> Option<Ref<Control>> {
        None
    }
}

impl Drop for TextArea {
    fn drop(&mut self) {
        self.explicit_line_collapses.get_mut().take();
        self.disconnect_input_handlers();
        self.disconnect_margins();
        self.disconnect_background_renderers();
        self.disconnect_visual_line_extensions();
        if let Some(timer) = self.blink_timer.get() {
            timer.stop();
        }
    }
}
