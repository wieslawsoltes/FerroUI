use crate::{
    CompletionAcceptAction, CompletionList, CopyingToClipboardEventArgs, DottedLineMargin,
    PastingFromClipboardEventArgs, SyntaxHighlighting, TextArea, TextCopiedEventArgs,
    TextMateInstallation, TextPastedEventArgs,
};
use ferroui_base::data::model::Event;
use ferroui_base::input::{IScrollable, InputElementImpl, InputElementImplExt};
use ferroui_base::interactivity::{
    InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::text_formatting::GenericTextRunProperties;
use ferroui_base::media::{BaselineAlignment, FontFamily, FontStretch, FontStyle, FontWeight, IBrush, Typeface};
use ferroui_base::platform::storage::IStorageFile;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::current_locale;
use ferroui_base::{
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    Thickness, Vector, Visual, VisualImpl, WeakRef, ferro_class, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate,
};
use ferroui_controls::automation::provider::TextEditDelta;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{ControlImpl, Decorator, ScrollViewer, TextBlock};
use ferroui_edit_core::document::{
    DocumentEdit, DocumentError, DocumentSubscription, DocumentVersion, SimpleSegment, TextDocument, TextSnapshot,
};
use ferroui_edit_core::editing::{EditError, EditorOptions, EditorSession, Selection, TextTransformation};
use ferroui_edit_core::folding::{FoldingManager, FoldingSubscription};
use ferroui_edit_core::highlighting::{
    DocumentHighlighter, HighlightingDefinition, HighlightingEngine,
};
use ferroui_edit_core::snippets::{Snippet, SnippetError};
use ferroui_edit_core::utils::{FileReader, TextEncoding};
use ferroui_textmate::{Grammar, Theme};
use std::cell::{Cell, OnceCell, RefCell, RefMut};
use std::collections::{HashMap, VecDeque};
use std::io::{self, Read, Write};
use std::rc::Rc;

/// A fixed edit belongs to one revision of the document displayed by an
/// editor. Clipboard waits and host callbacks can change that binding.
pub(crate) struct EditTarget {
    document: TextDocument,
    version: DocumentVersion,
    epoch: u64,
    root: Option<WeakRef<Visual>>,
}

impl EditTarget {
    pub(crate) fn new(editor: &TextEditor) -> Self {
        let document = editor.document();
        let version = document.version();
        Self {
            document,
            version,
            epoch: editor.document_epoch(),
            root: editor.visual_root().map(|root| root.downgrade()),
        }
    }

    pub(crate) fn is_current(&self, editor: &TextEditor) -> bool {
        self.is_current_after(editor, 0)
    }

    pub(crate) fn is_current_after(&self, editor: &TextEditor, document_changes: usize) -> bool {
        editor.is_effectively_enabled()
            && self.epoch == editor.document_epoch()
            && self.version.revision().checked_add(document_changes as u64)
                == Some(self.document.version().revision())
            && match &self.root {
                Some(root) => root.upgrade().is_some_and(|root| editor.visual_root() == Some(root)),
                None => editor.visual_root().is_none(),
            }
    }

    pub(crate) fn apply(&self, editor: &TextEditor, edits: &[DocumentEdit]) -> Result<(), EditError> {
        editor.apply_checked_document_edits(&self.document, edits, || self.is_current(editor))
    }
}

/// A code editor with a shared document, independent editing session and a
/// virtualized text area. All public positions use UTF-16 code units.
#[repr(C)]
pub struct TextEditor {
    base: Decorator,
    session: RefCell<EditorSession>,
    document_epoch: Cell<u64>,
    area: OnceCell<Ref<TextArea>>,
    scroll_viewer: OnceCell<Ref<ScrollViewer>>,
    document_subscriptions: RefCell<Vec<DocumentSubscription>>,
    observed_modified: Cell<bool>,
    write_bom: Cell<bool>,
    file_load_epoch: Cell<u64>,
    file_io: tokio::sync::Mutex<()>,
    textmate: RefCell<Option<Rc<TextMateInstallation>>>,
    last_error: RefCell<Option<String>>,
    previous_snapshot: RefCell<TextSnapshot>,
    pub(crate) automation_text_edits: Event<TextEditDelta>,
    pending_text_edits: RefCell<Option<VecDeque<TextEditDelta>>>,
    search_panel: RefCell<Option<Ref<crate::SearchPanel>>>,
    folding: RefCell<Option<Rc<FoldingManager>>>,
    folding_subscription: RefCell<Option<FoldingSubscription>>,
    language_folding: Cell<bool>,
    language_folding_pending: Cell<bool>,
    highlighter: RefCell<Option<Rc<DocumentHighlighter>>>,
    snippet: Cell<Option<Rc<crate::SnippetController>>>,
    snippet_insertion: RefCell<()>,
    textmate_subscription: RefCell<Option<Subscription>>,
    previous_selection: RefCell<Selection>,
    caret_position_changed: Event<()>,
    observed_caret: Cell<Option<crate::text_area::CaretState>>,
    caret_change_pending: Cell<bool>,
    commands: RefCell<HashMap<crate::EditorCommandKind, Rc<crate::EditorCommand>>>,
    style_subscriptions: RefCell<Vec<Subscription>>,
}

ferro_class!(TextEditor: Decorator);
ferroui_base::ferro_class_info!(TextEditor {
    new: TextEditor::new,
    markup: {
        properties: [
            CaretOffset: usize { get: TextEditor::caret_offset, set: TextEditor::set_caret_offset_from_markup },
            LineNumbersMinWidthInDigits: u32 { get: TextEditor::line_numbers_min_width_in_digits, set: TextEditor::set_line_numbers_min_width_in_digits },
            UndoCommand: Rc<dyn ferroui_base::input::ICommand> { get: TextEditor::undo_command },
            RedoCommand: Rc<dyn ferroui_base::input::ICommand> { get: TextEditor::redo_command },
            CutCommand: Rc<dyn ferroui_base::input::ICommand> { get: TextEditor::cut_command },
            CopyCommand: Rc<dyn ferroui_base::input::ICommand> { get: TextEditor::copy_command },
            PasteCommand: Rc<dyn ferroui_base::input::ICommand> { get: TextEditor::paste_command },
        ],
    }
});
ferro_impl_classes!(TextEditor: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl InputElementImpl for TextEditor {
    fn on_got_focus(this: &Self, e: &ferroui_base::input::FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        if e.source()
            .is_some_and(|source| source.ptr_eq(&this.to_ref()))
        {
            if let Some(area) = this.area.get() {
                area.focus();
            }
        }
    }
}

impl ControlImpl for TextEditor {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        crate::automation::TextEditorAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for TextEditor {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let area = TextArea::new();
        area.set_editor(this.to_ref().downgrade());
        let viewer = ScrollViewer::new();
        viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
        viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        viewer.set_content(Some(Rc::new(area.clone())));
        this.area.set(area).ok();
        this.scroll_viewer.set(viewer.clone()).ok();
        this.set_child(viewer);
        this.set_current_value(
            Self::document_property(),
            Some(this.session().document().clone()),
        );
        let weak = this.to_ref().downgrade();
        let resources = this.resources_changed(move |_| {
            if let Some(editor) = weak.upgrade() {
                editor.text_area().invalidate_layout_cache();
            }
        });
        let weak = this.to_ref().downgrade();
        let theme = this.actual_theme_variant_changed(move || {
            if let Some(editor) = weak.upgrade() {
                editor.text_area().invalidate_layout_cache();
            }
        });
        this.style_subscriptions
            .borrow_mut()
            .extend([resources, theme]);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        let font_changed = [
            Self::font_family_property().as_property(),
            Self::font_size_property().as_property(),
            Self::font_style_property().as_property(),
            Self::font_weight_property().as_property(),
            Self::font_stretch_property().as_property(),
        ].contains(&property);
        if property == Self::document_property().as_property() {
            this.attach_document(
                change
                    .get_new_value::<Option<TextDocument>>()
                    .unwrap_or_default(),
            );
        } else if property == Self::is_read_only_property().as_property() {
            this.session().set_is_read_only(this.is_read_only());
            this.raise_event(&RoutedEventArgs::with_event(Self::selection_changed_event()));
        } else if property == Self::options_property().as_property() {
            this.session().set_options(this.options());
            this.schedule_language_foldings();
            if let Some(area) = this.area.get() {
                area.update_text_drag_option();
            }
        } else if property == Self::syntax_highlighting_property().as_property() {
            this.rebuild_highlighter(this.document());
        }
        if let Some(area) = this.area.get() {
            if font_changed || [
                Self::show_line_numbers_property().as_property(),
                Self::line_numbers_foreground_property().as_property(),
                Self::line_numbers_margin_property().as_property(),
                Self::line_numbers_min_width_in_digits_property().as_property(),
            ].contains(&property) {
                area.update_line_number_gutter(this);
            }
            if property == Self::font_size_property().as_property() {
                area.update_folding_gutter(this);
            }
            // Resetting measured line heights can temporarily shrink the scroll
            // extent, so only reset them when an input to formatting changes.
            if font_changed || [
                Self::foreground_property().as_property(),
                Self::word_wrap_property().as_property(),
                Self::options_property().as_property(),
                Self::syntax_highlighting_property().as_property(),
                Self::show_line_numbers_property().as_property(),
                Self::line_numbers_margin_property().as_property(),
                Self::line_numbers_min_width_in_digits_property().as_property(),
            ].contains(&property) {
                area.invalidate_layout_cache();
            } else if [
                Self::background_property().as_property(),
                Self::folding_marker_brush_property().as_property(),
                Self::folding_marker_background_brush_property().as_property(),
                Self::selected_folding_marker_brush_property().as_property(),
                Self::selected_folding_marker_background_brush_property().as_property(),
                Self::line_numbers_foreground_property().as_property(),
            ].contains(&property) {
                area.invalidate_margin_viewport();
                area.request_redraw();
            } else if property == Self::search_results_brush_property().as_property() {
                area.request_redraw();
            }
        }
        if property == Self::is_read_only_property().as_property()
            || font_changed
            || property == Self::foreground_property().as_property()
            || property == Self::background_property().as_property()
            || property == Self::syntax_highlighting_property().as_property()
        {
            this.raise_automation_format_changed();
        }
        if property == Self::options_property().as_property()
            && this.caret_position_changed.has_handlers()
        {
            this.record_caret_change(&this.selection());
            this.publish_caret_change();
        }
    }
}

ferro_properties! { impl TextEditor {
    pub fn text_property() -> DirectProperty<TextEditor, String> {
        FerroProperty::register_direct::<TextEditor, _>("Text", TextEditor::text, Some(TextEditor::set_text_from_markup), String::new())
    }
    pub fn watermark_property() -> DirectProperty<TextEditor, Option<String>> {
        FerroProperty::register_direct::<TextEditor, _>("Watermark", TextEditor::watermark, Some(TextEditor::set_watermark), None)
    }
    pub fn document_property() -> StyledProperty<Option<TextDocument>> {
        FerroProperty::register::<TextEditor, _>("Document", None)
    }
    pub fn syntax_highlighting_property() -> StyledProperty<Option<SyntaxHighlighting>> {
        FerroProperty::register::<TextEditor, _>("SyntaxHighlighting", None)
    }
    pub fn is_read_only_property() -> StyledProperty<bool> {
        FerroProperty::register::<TextEditor, _>("IsReadOnly", false)
    }
    pub fn is_modified_property() -> DirectProperty<TextEditor, bool> {
        FerroProperty::register_direct::<TextEditor, _>("IsModified", TextEditor::is_modified, Some(TextEditor::set_is_modified), false)
    }
    pub fn encoding_property() -> StyledProperty<TextEncoding> {
        FerroProperty::register::<TextEditor, _>("Encoding", TextEncoding::Utf8)
    }
    pub fn show_line_numbers_property() -> StyledProperty<bool> {
        FerroProperty::register::<TextEditor, _>("ShowLineNumbers", false)
    }
    pub fn line_numbers_min_width_in_digits_property() -> StyledProperty<u32> {
        FerroProperty::register::<TextEditor, _>("LineNumbersMinWidthInDigits", 2)
    }
    pub fn word_wrap_property() -> StyledProperty<bool> {
        FerroProperty::register::<TextEditor, _>("WordWrap", false)
    }
    pub fn font_family_property() -> StyledProperty<FontFamily> {
        FerroProperty::register_with::<TextEditor, _>("FontFamily", StyledPropertyOptions::new(FontFamily::new("Cascadia Code,Consolas,Menlo,monospace")).inherits(true))
    }
    pub fn font_size_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<TextEditor, _>("FontSize", StyledPropertyOptions::new(14.0_f64).inherits(true).validate(|v| v.is_finite() && *v > 0.0))
    }
    pub fn font_style_property() -> StyledProperty<FontStyle> {
        TextBlock::font_style_property().add_owner::<TextEditor>()
    }
    pub fn font_weight_property() -> StyledProperty<FontWeight> {
        TextBlock::font_weight_property().add_owner::<TextEditor>()
    }
    pub fn font_stretch_property() -> StyledProperty<FontStretch> {
        TextBlock::font_stretch_property().add_owner::<TextEditor>()
    }
    pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        TextBlock::foreground_property().add_owner::<TextEditor>()
    }
    pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextEditor, _>("Background", None)
    }
    pub fn selection_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register_with::<TextEditor, _>("SelectionBrush", StyledPropertyOptions::new(Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0x604d90fe)) as Rc<dyn IBrush>)).inherits(true))
    }
    pub fn line_numbers_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextEditor, _>("LineNumbersForeground", Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0xff858585)) as Rc<dyn IBrush>))
    }
    pub fn line_numbers_margin_property() -> StyledProperty<Thickness> {
        FerroProperty::register_with::<TextEditor, _>("LineNumbersMargin", StyledPropertyOptions::new(DottedLineMargin::DEFAULT_MARGIN).validate(|margin| {
            [margin.left, margin.top, margin.right, margin.bottom].iter().all(|side| side.is_finite())
        }))
    }
    pub fn folding_marker_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextEditor, _>("FoldingMarkerBrush", None)
    }
    pub fn folding_marker_background_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextEditor, _>("FoldingMarkerBackgroundBrush", None)
    }
    pub fn selected_folding_marker_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextEditor, _>("SelectedFoldingMarkerBrush", None)
    }
    pub fn selected_folding_marker_background_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextEditor, _>("SelectedFoldingMarkerBackgroundBrush", None)
    }
    pub fn options_property() -> StyledProperty<EditorOptions> {
        FerroProperty::register::<TextEditor, _>("Options", EditorOptions::default())
    }
    pub fn completion_accept_action_property() -> StyledProperty<CompletionAcceptAction> {
        CompletionList::completion_accept_action_property().add_owner::<TextEditor>()
    }
    pub fn search_results_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextEditor, _>(
            "SearchResultsBrush",
            Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0x70e7b83a)) as Rc<dyn IBrush>),
        )
    }
} }

impl TextEditor {
    fn static_constructor() {
        Self::text_changed_event();
        Self::selection_changed_event();
        Self::copying_to_clipboard_event();
        Self::text_copied_event();
        Self::pasting_from_clipboard_event();
        Self::text_pasted_event();
    }

    pub fn construct() -> Self {
        Self {
            base: Decorator::construct(),
            session: RefCell::new(EditorSession::new(TextDocument::new())),
            document_epoch: Cell::new(0),
            area: OnceCell::new(),
            scroll_viewer: OnceCell::new(),
            document_subscriptions: RefCell::new(Vec::new()),
            observed_modified: Cell::new(false),
            write_bom: Cell::new(false),
            file_load_epoch: Cell::new(0),
            file_io: tokio::sync::Mutex::new(()),
            textmate: RefCell::new(None),
            last_error: RefCell::new(None),
            previous_snapshot: RefCell::new(TextDocument::new().create_snapshot()),
            automation_text_edits: Event::new(),
            pending_text_edits: RefCell::new(None),
            search_panel: RefCell::new(None),
            folding: RefCell::new(None),
            folding_subscription: RefCell::new(None),
            language_folding: Cell::new(false),
            language_folding_pending: Cell::new(false),
            highlighter: RefCell::new(None),
            snippet: Cell::new(None),
            snippet_insertion: RefCell::new(()),
            textmate_subscription: RefCell::new(None),
            previous_selection: RefCell::new(Selection::empty(0)),
            caret_position_changed: Event::new(),
            observed_caret: Cell::new(None),
            caret_change_pending: Cell::new(false),
            commands: RefCell::new(HashMap::new()),
            style_subscriptions: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    ferro_routed_event!(
        pub fn text_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TextEditor, _>("TextChanged", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub(crate) fn automation_format_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TextEditor, _>("AutomationFormatChanged", RoutingStrategies::DIRECT)
        }
    );
    pub(crate) fn raise_automation_format_changed(&self) {
        self.raise_event(&RoutedEventArgs::with_event(Self::automation_format_changed_event()));
    }
    ferro_routed_event!(
        pub(crate) fn search_state_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TextEditor, _>("SearchStateChanged", RoutingStrategies::DIRECT)
        }
    );
    pub(crate) fn notify_search_state_changed(&self) {
        self.raise_event(&RoutedEventArgs::with_event(Self::search_state_changed_event()));
    }
    ferro_routed_event!(
        pub fn selection_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TextEditor, _>("SelectionChanged", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn copying_to_clipboard_event() -> RoutedEvent<CopyingToClipboardEventArgs> {
            RoutedEvent::register::<TextEditor, _>("CopyingToClipboard", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn text_copied_event() -> RoutedEvent<TextCopiedEventArgs> {
            RoutedEvent::register::<TextEditor, _>("TextCopied", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn pasting_from_clipboard_event() -> RoutedEvent<PastingFromClipboardEventArgs> {
            RoutedEvent::register::<TextEditor, _>(
                "PastingFromClipboard",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    ferro_routed_event!(
        pub fn text_pasted_event() -> RoutedEvent<TextPastedEventArgs> {
            RoutedEvent::register::<TextEditor, _>("TextPasted", RoutingStrategies::BUBBLE)
        }
    );

    pub fn session(&self) -> EditorSession {
        self.session.borrow().clone()
    }
    pub fn command(&self, kind: crate::EditorCommandKind) -> Rc<crate::EditorCommand> {
        if let Some(command) = self.commands.borrow().get(&kind).cloned() {
            return command;
        }
        let command = crate::EditorCommand::new(&self.to_ref(), kind);
        self.commands.borrow_mut().insert(kind, command.clone());
        command
    }
    pub fn undo_command(&self) -> Rc<dyn ferroui_base::input::ICommand> {
        self.command(crate::EditorCommandKind::Undo).as_command()
    }
    pub fn redo_command(&self) -> Rc<dyn ferroui_base::input::ICommand> {
        self.command(crate::EditorCommandKind::Redo).as_command()
    }
    pub fn cut_command(&self) -> Rc<dyn ferroui_base::input::ICommand> {
        self.command(crate::EditorCommandKind::Cut).as_command()
    }
    pub fn copy_command(&self) -> Rc<dyn ferroui_base::input::ICommand> {
        self.command(crate::EditorCommandKind::Copy).as_command()
    }
    pub fn paste_command(&self) -> Rc<dyn ferroui_base::input::ICommand> {
        self.command(crate::EditorCommandKind::Paste).as_command()
    }
    pub(crate) fn document_epoch(&self) -> u64 {
        self.document_epoch.get()
    }
    pub fn document(&self) -> TextDocument {
        self.session().document().clone()
    }
    pub fn set_document(&self, document: TextDocument) {
        self.set_value(Self::document_property(), Some(document));
    }
    pub fn text(&self) -> String {
        self.document().text()
    }
    pub fn set_text(&self, text: &str) -> Result<(), DocumentError> {
        let document = self.document();
        document.replace(0, document.text_length(), text)
    }

    /// The hint displayed by the text area when its document is empty.
    pub fn watermark(&self) -> Option<String> {
        self.text_area().watermark()
    }
    pub fn set_watermark(&self, value: Option<String>) {
        self.text_area().set_watermark(value);
    }

    /// Reindents selected lines, or the document when no text is selected.
    /// Installed language rules are used when the session strategy allows them.
    /// All changed ranges are checked and applied together.
    pub fn reindent_selection(&self) -> Result<(), EditError> {
        crate::language_reindent::reindent(self)
    }

    /// Deletes the writable portions of the current or selected lines.
    pub fn delete_lines(&self) -> Result<(), EditError> {
        self.text_area().delete_lines(self)
    }

    /// Applies a transformation to the writable parts of the selected text or
    /// lines, with one undo group for the command.
    pub fn transform_text(&self, transformation: TextTransformation) -> Result<(), EditError> {
        if transformation == TextTransformation::DeleteLines {
            return self.delete_lines();
        }
        let target = EditTarget::new(self);
        let epoch = self.document_epoch();
        let locale = current_locale();
        self.session().transform_text_in_language_checked(
            transformation,
            locale.id.language.as_str(),
            || target.is_current(self),
        )?;
        if self.document_epoch() == epoch && !target.is_current(self) {
            self.selection_changed();
        }
        Ok(())
    }

    pub(crate) fn indent(&self, unindent: bool, from_tab: bool) -> Result<(), EditError> {
        let target = EditTarget::new(self);
        let epoch = self.document_epoch();
        let session = self.session();
        let selection = session.selection();
        if from_tab && !unindent {
            session.insert_tab_checked(self.caret_offset(), || target.is_current(self))?;
        } else {
            session.indent_selection_checked(unindent, || target.is_current(self))?;
        }
        if self.document_epoch() == epoch
            && (session.selection() != selection || !target.is_current(self))
        {
            self.selection_changed();
        }
        Ok(())
    }

    pub(crate) fn apply_checked_document_edits(
        &self,
        document: &TextDocument,
        edits: &[DocumentEdit],
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        let session = self.session();
        let validate = || {
            if !is_current() {
                return Err(EditError::StaleEditPlan);
            }
            let policy = session.validate_document_edits(edits);
            if !is_current() {
                return Err(EditError::StaleEditPlan);
            }
            policy?;
            if session.is_read_only() || document.is_read_only() {
                return Err(EditError::ReadOnly);
            }
            Ok(())
        };
        validate()?;
        let guard = document.begin_update()?;
        // Host policy and update-start callbacks run outside document borrows
        // and can invalidate the fixed coordinates before an edit is applied.
        validate()?;
        document.apply_edits(edits)?;
        guard.finish()?;
        self.selection_changed();
        Ok(())
    }
    pub fn is_modified(&self) -> bool {
        self.document().is_modified()
    }
    pub fn set_is_modified(&self, modified: bool) {
        let document = self.document();
        if document.is_modified() == modified {
            return;
        }
        if modified {
            document.discard_original_file_marker();
        } else {
            document.mark_as_original_file();
        }
    }
    pub fn encoding(&self) -> TextEncoding {
        self.get_value(Self::encoding_property())
    }
    pub fn set_encoding(&self, encoding: TextEncoding) {
        self.set_value(Self::encoding_property(), encoding);
    }
    pub fn write_bom(&self) -> bool {
        self.write_bom.get()
    }
    pub fn set_write_bom(&self, write_bom: bool) {
        self.write_bom.set(write_bom);
    }

    fn file_context(&self) -> (u64, u64) {
        (self.document_epoch(), self.file_load_epoch.get())
    }

    /// Loads a byte stream into the current document, detecting its encoding.
    /// Decoding finishes before the document is changed.
    pub fn load(&self, source: impl Read) -> io::Result<()> {
        let document = self.document();
        let version = document.version();
        let context = self.file_context();
        let mut reader = FileReader::open_stream(source, self.encoding())?;
        let text = reader.read_to_end_utf16()?;
        if self.file_context() != context || document.version() != version {
            return Err(io::Error::other("The document changed while loading"));
        }
        if document.is_in_update() {
            return Err(io::Error::other(DocumentError::UpdateInProgress));
        }
        let changed = !text.is_empty() || document.text_length() != 0;
        document
            .replace_utf16(0, document.text_length(), &text)
            .map_err(io::Error::other)?;
        let loaded_version = document.version();
        if loaded_version.revision() - version.revision() > u64::from(changed)
            || self.file_context() != context
        {
            return Err(io::Error::other("The document changed while loading"));
        }
        self.file_load_epoch
            .set(self.file_load_epoch.get().wrapping_add(1));
        let loaded_context = self.file_context();
        let check_current = || {
            (self.file_context() == loaded_context && document.version() == loaded_version)
                .then_some(())
                .ok_or_else(|| io::Error::other("The document changed while loading"))
        };
        document.clear_undo_history().map_err(io::Error::other)?;
        check_current()?;
        self.set_current_value(Self::encoding_property(), reader.encoding());
        check_current()?;
        self.set_write_bom(reader.byte_order_mark_length() != 0);
        document.mark_as_original_file();
        check_current()?;
        self.set_caret_offset(0).map_err(io::Error::other)?;
        check_current()?;
        self.scroll_viewer().scroll_to_home();
        check_current()
    }

    /// Opens a file through the platform storage service.
    pub async fn load_file(&self, file: &dyn IStorageFile) -> io::Result<()> {
        let document = self.document();
        let version = document.version();
        let context = self.file_context();
        let name = file.name();
        let _permit = self.file_io.lock().await;
        if self.file_context() != context || document.version() != version {
            return Err(io::Error::other("The document changed before opening the file"));
        }
        let source = file.open_read_async().await?;
        if self.file_context() != context || document.version() != version {
            return Err(io::Error::other("The document changed while opening the file"));
        }
        self.load(source)?;
        document.set_file_name(Some(name));
        Ok(())
    }

    /// Saves the captured snapshot using the configured encoding and BOM.
    /// Successful output marks that revision clean when it is still current.
    pub fn save(&self, target: impl Write) -> io::Result<()> {
        let document = self.document();
        let snapshot = document.create_snapshot();
        let context = self.file_context();
        let encoding = self.encoding();
        let write_bom = self.write_bom();
        encoding.write_snapshot(&snapshot, target, write_bom)?;
        if self.file_context() == context
            && snapshot.version() == Some(document.version())
            && self.encoding() == encoding
            && self.write_bom() == write_bom
        {
            document.mark_as_original_file();
        }
        Ok(())
    }

    /// Saves to the platform storage stream. Validate encoding before opening
    /// the destination because some providers truncate it on open.
    pub async fn save_file(&self, file: &dyn IStorageFile) -> io::Result<()> {
        let document = self.document();
        let snapshot = document.create_snapshot();
        let version = snapshot.version();
        let context = self.file_context();
        let encoding = self.encoding();
        let write_bom = self.write_bom();
        encoding.write_snapshot(&snapshot, io::sink(), write_bom)?;
        let name = file.name();
        let _permit = self.file_io.lock().await;
        if self.file_context() != context {
            return Err(io::Error::other("The document changed before saving the file"));
        }
        file.write_with_async(Box::new(move |target| {
            encoding.write_snapshot(&snapshot, target, write_bom)
        }))
        .await?;
        if self.file_context() == context
            && version == Some(document.version())
            && self.encoding() == encoding
            && self.write_bom() == write_bom
        {
            document.mark_as_original_file();
        }
        if self.file_context() == context {
            document.set_file_name(Some(name));
        }
        Ok(())
    }
    fn set_text_from_markup(&self, text: String) {
        if let Err(error) = self.set_text(&text) {
            self.report_error(error.to_string());
        }
    }
    pub fn text_area(&self) -> Ref<TextArea> {
        self.area.get().expect("constructed editor").clone()
    }
    pub fn search_panel(&self) -> Ref<crate::SearchPanel> {
        crate::SearchPanel::install(&self.to_ref())
    }
    pub fn installed_search_panel(&self) -> Option<Ref<crate::SearchPanel>> {
        self.search_panel.borrow().clone()
    }
    pub fn search_results_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::search_results_brush_property())
    }
    pub fn set_search_results_brush(&self, brush: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::search_results_brush_property(), brush);
    }
    pub(crate) fn attach_search_panel(&self, panel: Ref<crate::SearchPanel>) {
        self.search_panel.replace(Some(panel));
    }
    pub(crate) fn detach_search_panel(&self, panel: &Ref<crate::SearchPanel>) {
        if self.search_panel.borrow().as_ref() == Some(panel) {
            self.search_panel.borrow_mut().take();
            self.notify_search_state_changed();
        }
    }
    pub fn scroll_viewer(&self) -> Ref<ScrollViewer> {
        self.scroll_viewer
            .get()
            .expect("constructed editor")
            .clone()
    }
    pub fn caret_offset(&self) -> usize {
        self.area.get().and_then(|area| area.projected_caret_offset())
            .unwrap_or_else(|| self.session().caret_offset())
    }
    pub fn caret_position(&self) -> crate::TextViewPosition {
        self.text_area().caret_position().unwrap_or_else(|| {
            crate::TextViewPosition::from_location(
                self.document()
                    .get_location(self.caret_offset())
                    .unwrap_or_default(),
            )
        })
    }

    /// Observes movement of the primary caret, including soft-wrap affinity and
    /// virtual cells. Document updates deliver one callback after the outermost
    /// update finishes. Dropping the subscription removes the handler.
    pub fn caret_position_changed(&self, handler: impl Fn() + 'static) -> Subscription {
        if !self.caret_position_changed.has_handlers() {
            self.observed_caret
                .set(self.text_area().caret_state(&self.document(), &self.selection()));
            self.caret_change_pending.set(false);
        }
        let token = self.caret_position_changed.add(Rc::new(move |_| handler()));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(editor) = weak.upgrade() {
                editor.caret_position_changed.remove(token);
            }
        })
        .into()
    }

    pub fn set_caret_position(&self, position: crate::TextViewPosition) -> Result<(), EditError> {
        let document = self.document();
        let offset = document.get_offset(position.location())?;
        if let Some(manager) = self.folding_manager() {
            manager.unfold_at(offset);
        }
        let extra = if let Some(visual) = position.visual_column {
            let logical = document.get_line_by_offset(offset)?;
            if offset == logical.end_offset() {
                visual.saturating_sub(
                    self.text_area()
                        .visual_column_for_offset(offset)
                        .unwrap_or(visual),
                )
            } else {
                0
            }
        } else {
            0
        };
        self.session().set_caret_position(offset, extra)?;
        self.selection_changed_with_affinity(position.is_at_end_of_line);
        Ok(())
    }
    pub fn set_caret_offset(&self, offset: usize) -> Result<(), EditError> {
        self.session().set_caret_offset(offset)?;
        self.selection_changed();
        Ok(())
    }
    pub fn set_caret_offset_from_markup(&self, offset: usize) {
        if let Err(error) = self.set_caret_offset(offset) {
            self.report_error(error.to_string());
        }
    }
    pub fn selection(&self) -> Selection {
        self.session().selection()
    }
    pub fn set_selection(&self, selection: Selection) -> Result<(), EditError> {
        self.session().set_selection(selection)?;
        self.selection_changed();
        Ok(())
    }
    pub fn select(&self, start: usize, length: usize) -> Result<(), EditError> {
        self.session().select(start, length)?;
        self.selection_changed();
        Ok(())
    }
    pub fn select_all(&self) {
        self.session().select_all();
        self.selection_changed();
    }
    pub fn selected_text(&self) -> Result<String, EditError> {
        self.session().selected_text()
    }
    pub fn replace_selection_with_text(&self, text: &str) -> Result<(), EditError> {
        self.session().replace_selection_with_text(text)?;
        self.selection_changed();
        Ok(())
    }
    pub fn undo(&self) -> Result<bool, EditError> {
        self.session().undo()
    }
    pub fn redo(&self) -> Result<bool, EditError> {
        self.session().redo()
    }
    pub fn is_read_only(&self) -> bool {
        self.get_value(Self::is_read_only_property())
    }
    pub fn set_is_read_only(&self, value: bool) {
        self.set_value(Self::is_read_only_property(), value);
    }
    pub fn show_line_numbers(&self) -> bool {
        self.get_value(Self::show_line_numbers_property())
    }
    pub fn set_show_line_numbers(&self, value: bool) {
        self.set_value(Self::show_line_numbers_property(), value);
    }
    /// Minimum digit advances reserved for the visible line-number gutter.
    pub fn line_numbers_min_width_in_digits(&self) -> u32 {
        self.get_value(Self::line_numbers_min_width_in_digits_property())
    }
    pub fn set_line_numbers_min_width_in_digits(&self, value: u32) {
        self.set_value(Self::line_numbers_min_width_in_digits_property(), value);
    }
    pub fn word_wrap(&self) -> bool {
        self.get_value(Self::word_wrap_property())
    }
    pub fn set_word_wrap(&self, value: bool) {
        self.set_value(Self::word_wrap_property(), value);
    }
    pub fn font_family(&self) -> FontFamily {
        self.get_value(Self::font_family_property())
    }

    pub(crate) fn typeface(&self) -> Typeface {
        Typeface::with_style(self.font_family(), self.font_style(), self.font_weight(), self.font_stretch())
    }

    pub(crate) fn default_text_run_properties(&self) -> GenericTextRunProperties {
        GenericTextRunProperties::with_all(
            self.typeface(),
            self.font_size(),
            None,
            self.foreground(),
            None,
            BaselineAlignment::Baseline,
            Some(current_locale()),
            None,
        )
    }
    pub fn set_font_family(&self, value: FontFamily) {
        self.set_value(Self::font_family_property(), value);
    }
    pub fn font_size(&self) -> f64 {
        self.get_value(Self::font_size_property())
    }
    pub fn set_font_size(&self, value: f64) {
        self.set_value(Self::font_size_property(), value);
    }
    pub fn font_style(&self) -> FontStyle {
        self.get_value(Self::font_style_property())
    }
    pub fn set_font_style(&self, value: FontStyle) {
        self.set_value(Self::font_style_property(), value);
    }
    pub fn font_weight(&self) -> FontWeight {
        self.get_value(Self::font_weight_property())
    }
    pub fn set_font_weight(&self, value: FontWeight) {
        self.set_value(Self::font_weight_property(), value);
    }
    pub fn font_stretch(&self) -> FontStretch {
        self.get_value(Self::font_stretch_property())
    }
    pub fn set_font_stretch(&self, value: FontStretch) {
        self.set_value(Self::font_stretch_property(), value);
    }
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }
    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value);
    }
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }
    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value);
    }
    pub fn selection_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_brush_property())
    }
    pub fn set_selection_brush(&self, brush: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_brush_property(), brush);
    }
    pub fn line_numbers_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::line_numbers_foreground_property())
    }
    pub fn set_line_numbers_foreground(&self, brush: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::line_numbers_foreground_property(), brush);
    }
    /// The spacing around the separator to the right of the line numbers.
    pub fn line_numbers_margin(&self) -> Thickness {
        self.get_value(Self::line_numbers_margin_property())
    }
    pub fn set_line_numbers_margin(&self, margin: Thickness) {
        self.set_value(Self::line_numbers_margin_property(), margin);
    }
    pub fn options(&self) -> EditorOptions {
        self.get_value(Self::options_property())
    }
    pub fn set_options(&self, value: EditorOptions) {
        self.set_value(Self::options_property(), value);
    }
    /// The default pointer gesture for completion lists opened by this editor.
    pub fn completion_accept_action(&self) -> CompletionAcceptAction {
        self.get_value(Self::completion_accept_action_property())
    }
    pub fn set_completion_accept_action(&self, action: CompletionAcceptAction) {
        self.set_value(Self::completion_accept_action_property(), action);
    }
    pub fn last_error(&self) -> Option<String> {
        self.last_error.borrow().clone()
    }
    pub(crate) fn report_error(&self, error: String) {
        *self.last_error.borrow_mut() = Some(error);
    }

    pub fn install_textmate(&self, grammar: Grammar, theme: Theme) -> Rc<TextMateInstallation> {
        if let Some(subscription) = self.textmate_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        let installation = Rc::new(TextMateInstallation::new(grammar, theme));
        let weak = self.to_ref().downgrade();
        *self.textmate_subscription.borrow_mut() = Some(installation.changed(move || {
            if let Some(editor) = weak.upgrade() {
                editor.schedule_language_foldings();
                editor.text_area().invalidate_layout_cache();
                editor.raise_automation_format_changed();
            }
        }));
        *self.textmate.borrow_mut() = Some(installation.clone());
        self.schedule_language_foldings();
        self.text_area().invalidate_layout_cache();
        self.raise_automation_format_changed();
        installation
    }
    pub fn uninstall_textmate(&self) {
        if let Some(subscription) = self.textmate_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.textmate.borrow_mut().take();
        self.schedule_language_foldings();
        self.text_area().invalidate_layout_cache();
        self.raise_automation_format_changed();
    }
    pub fn textmate_installation(&self) -> Option<Rc<TextMateInstallation>> {
        self.textmate.borrow().clone()
    }

    /// Installs folding for the current document. Section changes automatically
    /// invalidate the view; replacing Document removes the old installation.
    pub fn install_folding(&self) -> Rc<FoldingManager> {
        if let Some(manager) = self.folding_manager() {
            return manager;
        }
        let manager = Rc::new(FoldingManager::new(self.document()));
        let weak = self.to_ref().downgrade();
        let subscription = manager.changed(move || {
            if let Some(editor) = weak.upgrade() {
                editor.text_area().invalidate_layout_cache();
                editor.raise_automation_format_changed();
            }
        });
        *self.folding_subscription.borrow_mut() = Some(subscription);
        *self.folding.borrow_mut() = Some(manager.clone());
        self.text_area().update_folding_gutter(self);
        self.text_area().invalidate_layout_cache();
        manager
    }
    pub fn folding_manager(&self) -> Option<Rc<FoldingManager>> {
        self.folding.borrow().clone()
    }

    /// Installs automatic folding and gives the editor's manager to the
    /// language provider. The provider updates its sections after document,
    /// language or editor-option changes. Use `install_folding` for host-owned
    /// ranges.
    pub fn install_language_folding(&self) -> Rc<FoldingManager> {
        self.language_folding.set(true);
        let manager = self.install_folding();
        self.refresh_language_foldings();
        manager
    }

    fn refresh_language_foldings(&self) {
        let Some(manager) = self.folding_manager() else {
            return;
        };
        let input = self.textmate_installation().map(|installation| installation.input_languages());
        let fallback = ferroui_textmate::FoldingRules::default();
        let rules = input.as_ref().map_or(&fallback, |input| &input.folding);
        let snapshot = manager.document().create_snapshot();
        let result = crate::language_folding::compute_foldings(
            &snapshot,
            rules,
            self.session().options().indentation_size,
        )
        .and_then(|ranges| Ok(manager.update_foldings(&ranges, None)?));
        if let Err(error) = result {
            self.report_error(error.to_string());
        }
    }

    fn schedule_language_foldings(&self) {
        if !self.language_folding.get() || self.language_folding_pending.replace(true) {
            return;
        }
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(editor) = weak.upgrade() {
                    editor.language_folding_pending.set(false);
                    if editor.language_folding.get() {
                        editor.refresh_language_foldings();
                    }
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    pub fn uninstall_folding(&self) {
        self.language_folding.set(false);
        self.folding_subscription.borrow_mut().take();
        self.folding.borrow_mut().take();
        self.text_area().update_folding_gutter(self);
        self.text_area().invalidate_layout_cache();
        self.raise_automation_format_changed();
    }
    pub fn set_syntax_highlighting(&self, definition: Option<Rc<HighlightingDefinition>>) {
        self.set_value(Self::syntax_highlighting_property(), definition.map(SyntaxHighlighting));
    }
    pub fn syntax_highlighting(&self) -> Option<Rc<HighlightingDefinition>> {
        self.get_value(Self::syntax_highlighting_property()).map(|value| value.0)
    }
    pub fn document_highlighter(&self) -> Option<Rc<DocumentHighlighter>> {
        self.highlighter.borrow().clone()
    }

    fn rebuild_highlighter(&self, document: TextDocument) {
        *self.highlighter.borrow_mut() = self.syntax_highlighting().map(|definition| {
            Rc::new(DocumentHighlighter::new(document, HighlightingEngine::new(definition)))
        });
    }
    pub fn insert_snippet(
        &self,
        snippet: &Snippet,
    ) -> Result<Rc<crate::SnippetController>, SnippetError> {
        self.replace_snippet(|| {
            crate::SnippetController::insert_over_selection(&self.to_ref(), snippet)
        })
    }

    pub(crate) fn insert_snippet_at(
        &self,
        snippet: &Snippet,
        range: SimpleSegment,
        selected_text: &str,
    ) -> Result<Rc<crate::SnippetController>, SnippetError> {
        let target = EditTarget::new(self);
        self.replace_snippet(|| {
            if !target.is_current(self) {
                return Err(DocumentError::ReentrantChange.into());
            }
            crate::SnippetController::insert_at(&self.to_ref(), snippet, range, selected_text)
        })
    }

    // The guard stays alive through all extension callbacks and controller publication.
    pub(crate) fn begin_snippet_insertion(&self) -> Result<RefMut<'_, ()>, SnippetError> {
        self.snippet_insertion
            .try_borrow_mut()
            .map_err(|_| DocumentError::ReentrantChange.into())
    }

    fn replace_snippet(
        &self,
        insert: impl FnOnce() -> Result<Rc<crate::SnippetController>, SnippetError>,
    ) -> Result<Rc<crate::SnippetController>, SnippetError> {
        let _insertion = self.begin_snippet_insertion()?;
        if let Some(previous) = self.snippet.take() {
            let epoch = self.document_epoch();
            let version = self.document().version();
            let selection = self.selection();
            previous.finish();
            if self.document_epoch() != epoch
                || self.document().version() != version
                || self.selection() != selection
            {
                return Err(DocumentError::ReentrantChange.into());
            }
        }
        let controller = insert()?;
        self.snippet.set(Some(controller.clone()));
        Ok(controller)
    }

    pub(crate) fn selection_changed(&self) {
        self.selection_changed_with_affinity(false);
    }

    pub(crate) fn selection_changed_with_affinity(&self, at_end_of_line: bool) {
        let previous_projected = self.area.get().and_then(|area| area.projected_caret_offset());
        if let Some(area) = self.area.get() {
            area.selection_changed_with_affinity(at_end_of_line);
        }
        self.notify_selection_changed(previous_projected.is_some_and(|offset| offset != self.caret_offset()));
    }

    pub(crate) fn notify_selection_changed(&self, caret_moved: bool) {
        let selection = self.selection();
        self.record_caret_change(&selection);
        let changed = caret_moved || *self.previous_selection.borrow() != selection;
        if changed {
            if let Some(installation) = self.textmate_installation() {
                crate::language_pairs::retain_active_pairs(&installation, &self.document(), &selection);
            }
        }
        *self.previous_selection.borrow_mut() = selection;
        if changed {
            self.raise_event(&RoutedEventArgs::with_event(Self::selection_changed_event()));
        }
        self.publish_caret_change();
    }

    fn record_caret_change(&self, selection: &Selection) {
        if self.caret_position_changed.has_handlers() {
            let current = self
                .area
                .get()
                .and_then(|area| area.caret_state(&self.document(), selection));
            if self.observed_caret.replace(current) != current {
                self.caret_change_pending.set(true);
            }
        }
    }

    fn publish_caret_change(&self) {
        if self.caret_change_pending.get() && !self.document().is_in_update() {
            self.caret_change_pending.set(false);
            self.caret_position_changed.raise(&());
        }
    }

    fn synchronize_history_selection(&self) {
        let restored = self.area.get().is_some_and(|area| area.restore_undo_caret(self));
        if !restored && self.previous_selection.borrow().ne(&self.selection()) {
            self.selection_changed();
        }
    }

    fn attach_document(&self, document: TextDocument) {
        let previous_epoch = self.document_epoch();
        let snippet = self.snippet.take();
        if let Some(snippet) = snippet {
            // Prevent a teardown callback from starting a snippet on the outgoing binding.
            let _insertion = self.snippet_insertion.try_borrow_mut().ok();
            snippet.finish();
        }
        // Deactivation is an extension callback. It can attach a different
        // document while the old session is still active.
        if self.document_epoch() != previous_epoch {
            return;
        }
        if let Some(installation) = self.textmate_installation() {
            installation.generated_pairs.borrow_mut().clear();
        }
        self.document_subscriptions.take();
        self.folding_subscription.take();
        self.folding.take();
        let previous_text = self
            .automation_text_edits
            .has_handlers()
            .then(|| self.document().create_snapshot());
        let session = EditorSession::new(document.clone());
        session.set_options(self.options());
        session.set_is_read_only(self.is_read_only());
        let epoch = previous_epoch.wrapping_add(1);
        self.document_epoch.set(epoch);
        self.session.replace(session);
        if self.document_epoch() != epoch {
            return;
        }
        if let Some(area) = self.area.get() {
            area.detach_collapses_from_other_document(Some(&document));
        }
        self.rebuild_highlighter(document.clone());
        if let Some(installation) = self.textmate_installation() {
            installation.invalidate_from(1);
        }
        self.connect_document(&document, epoch);
        if let Some(previous_text) = previous_text {
            self.publish_text_edit(0, &previous_text, &document.create_snapshot());
            if self.document_epoch() != epoch {
                return;
            }
        }
        self.notify_modified_property();
        if self.document_epoch() != epoch {
            return;
        }
        self.notify_text_property();
        if self.document_epoch() != epoch {
            return;
        }
        if self.language_folding.get() {
            self.install_folding();
            self.schedule_language_foldings();
        }
        if let Some(area) = self.area.get().filter(|_| self.document_epoch() == epoch) {
            area.update_folding_gutter(self);
            if self.document_epoch() != epoch {
                return;
            }
            area.document_changed();
            if self.document_epoch() == epoch {
                area.set_offset(Vector::default());
            }
        }
        if self.document_epoch() != epoch {
            return;
        }
        self.selection_changed();
        if self.document_epoch() != epoch {
            return;
        }
        self.raise_event(&RoutedEventArgs::with_event(Self::text_changed_event()));
        if let Some(area) = self.area.get().filter(|_| self.document_epoch() == epoch) {
            area.notify_margin_document_changed();
        }
    }

    fn connect_document(&self, document: &TextDocument, epoch: u64) {
        let weak = self.to_ref().downgrade();
        let changed = document.changed(move |change| {
            if let Some(editor) = weak.upgrade().filter(|editor| editor.document_epoch() == epoch) {
                editor.publish_text_edit(change.offset, &change.removed_text, &change.inserted_text);
                if editor.document_epoch() != epoch {
                    return;
                }
                if let Some(installation) = editor.textmate_installation() {
                    let line = editor
                        .document()
                        .get_line_by_offset(change.offset.min(editor.document().text_length()))
                        .map(|line| line.line_number())
                        .unwrap_or(1);
                    installation.invalidate_from(line);
                }
                let rebased = editor.area.get().is_some_and(|area| {
                    area.document_edited(change);
                    editor.document_epoch() == epoch && area.rebase_context_caret(&editor, change)
                });
                if editor.document_epoch() != epoch {
                    return;
                }
                if !rebased {
                    editor.selection_changed();
                }
            }
        });
        // Layout and accessibility deltas above track individual edits. The
        // public text event and binding publish the completed document update.
        let weak = self.to_ref().downgrade();
        let text_changed = document.text_changed(move || {
            if let Some(editor) = weak.upgrade().filter(|editor| editor.document_epoch() == epoch) {
                editor.notify_text_property();
                if editor.document_epoch() == epoch {
                    editor.raise_event(&RoutedEventArgs::with_event(Self::text_changed_event()));
                }
            }
        });
        let weak = self.to_ref().downgrade();
        let update_finished = document.update_finished(move || {
            if let Some(editor) = weak.upgrade().filter(|editor| editor.document_epoch() == epoch) {
                editor.synchronize_history_selection();
                if editor.document_epoch() == epoch {
                    editor.schedule_language_foldings();
                    editor.publish_caret_change();
                }
            }
        });
        let weak = self.to_ref().downgrade();
        let undo_playback_finished = document.undo_playback_finished(move || {
            if let Some(editor) = weak.upgrade().filter(|editor| editor.document_epoch() == epoch) {
                editor.synchronize_history_selection();
            }
        });
        let weak = self.to_ref().downgrade();
        let property_changed = document.property_changed(move |property| {
            if property == "IsModified" {
                if let Some(editor) = weak.upgrade().filter(|editor| editor.document_epoch() == epoch) {
                    editor.notify_modified_property();
                }
            }
        });
        let mut subscriptions = vec![
            changed,
            text_changed,
            update_finished,
            undo_playback_finished,
            property_changed,
        ];
        if let Some(area) = self.area.get() {
            subscriptions.push(area.track_caret_undo(document, epoch));
        }
        self.document_subscriptions.replace(subscriptions);
    }

    fn publish_text_edit(&self, offset: usize, removed: &TextSnapshot, inserted: &TextSnapshot) {
        if !self.automation_text_edits.has_handlers() {
            return;
        }
        let updated = self.document().create_snapshot();
        let Some(edit) = projected_edit(&updated, offset, removed, inserted) else { return };
        {
            let mut pending = self.pending_text_edits.borrow_mut();
            if let Some(queue) = pending.as_mut() {
                queue.push_back(edit);
                return;
            }
            *pending = Some(VecDeque::new());
        }
        // A handler may replace the document. Everyone sees the current edit
        // before the resulting replacement or edits to the new document.
        struct Reset<'a>(&'a RefCell<Option<VecDeque<TextEditDelta>>>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.take();
            }
        }
        let _reset = Reset(&self.pending_text_edits);
        let mut next = Some(edit);
        while let Some(edit) = next {
            self.automation_text_edits.raise(&edit);
            next = self.pending_text_edits.borrow_mut().as_mut().and_then(VecDeque::pop_front);
        }
    }

    fn notify_text_property(&self) {
        let snapshot = self.document().create_snapshot();
        let previous = self.previous_snapshot.replace(snapshot.clone());
        // A full string binding necessarily materializes text. Document users
        // and routed TextChanged observers keep logarithmic editing without it.
        if self.property_changed_subscriber_count() != 0
            || Self::text_property().changed().has_observers()
        {
            self.raise_direct_property_changed(
                Self::text_property(),
                &previous.text(),
                &snapshot.text(),
            );
        }
    }

    fn notify_modified_property(&self) {
        let modified = self.is_modified();
        let previous = self.observed_modified.replace(modified);
        if modified != previous {
            self.raise_direct_property_changed(Self::is_modified_property(), &previous, &modified);
        }
    }

    /// Clipboard operations are dispatched through FerroUI's platform service.
    /// Cut deletes only after the write succeeds and only if the document and
    /// selection still match the captured state.
    pub fn copy(&self) {
        self.copy_or_cut(false);
    }
    pub fn cut(&self) {
        self.copy_or_cut(true);
    }
    fn copy_or_cut(&self, cut: bool) {
        crate::EditorClipboard::copy_or_cut(self, cut);
    }
    pub fn paste(&self) {
        crate::EditorClipboard::paste(self);
    }
}

#[cfg(test)]
#[path = "text_editor_file_tests.rs"]
mod file_tests;

fn projected_edit(
    updated: &TextSnapshot,
    offset: usize,
    removed: &TextSnapshot,
    inserted: &TextSnapshot,
) -> Option<TextEditDelta> {
    let preceding = offset.checked_sub(1).and_then(|index| updated.get_char_at(index).ok());
    let following = updated.get_char_at(offset + inserted.text_length()).ok();
    let is_high = |unit| (0xd800..=0xdbff).contains(&unit);
    let is_low = |unit| (0xdc00..=0xdfff).contains(&unit);
    let first = |text: &TextSnapshot| text.get_char_at(0).ok().or(following);
    let last = |text: &TextSnapshot| {
        text.text_length().checked_sub(1).and_then(|end| text.get_char_at(end).ok()).or(preceding)
    };
    // The raw edit can split or join a surrogate pair at either seam.
    // Include its neighbor in both fragments so the UTF-8 projection composes.
    let left = preceding.filter(|&unit| {
        is_high(unit) && (first(removed).is_some_and(is_low) || first(inserted).is_some_and(is_low))
    });
    let right = following.filter(|&unit| {
        is_low(unit) && (last(removed).is_some_and(is_high) || last(inserted).is_some_and(is_high))
    });
    let project = |text: &TextSnapshot| {
        if left.is_none() && right.is_none() {
            text.text()
        } else {
            let units: Vec<_> = left.into_iter().chain(text.to_utf16()).chain(right).collect();
            String::from_utf16_lossy(&units)
        }
    };
    let removed = project(removed);
    let inserted = project(inserted);
    if removed == inserted {
        return None;
    }
    let offset = offset - usize::from(left.is_some());
    let character_offset = updated.get_rope().character_offset_from_utf16(offset)?;
    Some(TextEditDelta { offset, character_offset, removed, inserted })
}

impl Drop for TextEditor {
    fn drop(&mut self) {
        self.document_subscriptions.get_mut().clear();
        if let Some(area) = self.area.get() {
            area.invalidate_margin_viewport();
        }
        for subscription in self.style_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        if let Some(subscription) = self.textmate_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
