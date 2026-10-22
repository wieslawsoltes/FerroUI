use crate::{TextArea, TextMateInstallation};
use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::input::{InputElementImpl, InputElementImplExt};
use ferroui_base::interactivity::{
    InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{FontFamily, IBrush};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    VisualImpl, ferro_class, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate,
};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{ControlImpl, Decorator, ScrollViewer, TextBlock, TopLevel};
use ferroui_edit_core::document::{
    DocumentError, DocumentSubscription, TextDocument, TextSnapshot,
};
use ferroui_edit_core::editing::{EditError, EditorOptions, EditorSession, Selection};
use ferroui_edit_core::folding::{FoldingManager, FoldingSubscription};
use ferroui_edit_core::highlighting::{
    DocumentHighlighter, HighlightingDefinition, HighlightingEngine,
};
use ferroui_edit_core::snippets::{Snippet, SnippetError};
use ferroui_textmate::{Grammar, Theme};
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// A code editor with a shared document, independent editing session and a
/// virtualized text area. All public positions use UTF-16 code units.
#[repr(C)]
pub struct TextEditor {
    base: Decorator,
    session: RefCell<EditorSession>,
    area: OnceCell<Ref<TextArea>>,
    scroll_viewer: OnceCell<Ref<ScrollViewer>>,
    subscription: RefCell<Option<DocumentSubscription>>,
    textmate: RefCell<Option<Rc<TextMateInstallation>>>,
    last_error: RefCell<Option<String>>,
    previous_snapshot: RefCell<TextSnapshot>,
    search_panel: OnceCell<Ref<crate::SearchPanel>>,
    folding: RefCell<Option<Rc<FoldingManager>>>,
    folding_subscription: RefCell<Option<FoldingSubscription>>,
    highlighter: RefCell<Option<Rc<DocumentHighlighter>>>,
    highlighting_definition: RefCell<Option<Rc<HighlightingDefinition>>>,
    snippet: RefCell<Option<Rc<crate::SnippetController>>>,
    textmate_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    previous_selection: RefCell<Selection>,
    commands: RefCell<HashMap<crate::EditorCommandKind, Rc<crate::EditorCommand>>>,
}

ferro_class!(TextEditor: Decorator);
ferroui_base::ferro_class_info!(TextEditor {
    new: TextEditor::new,
    markup: {
        properties: [
            CaretOffset: usize { get: TextEditor::caret_offset, set: TextEditor::set_caret_offset_from_markup },
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
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::document_property().as_property() {
            this.attach_document(
                change
                    .get_new_value::<Option<TextDocument>>()
                    .unwrap_or_default(),
            );
        } else if change.property() == Self::is_read_only_property().as_property() {
            this.session().set_is_read_only(this.is_read_only());
            this.raise_event(&RoutedEventArgs::with_event(Self::selection_changed_event()));
        } else if change.property() == Self::options_property().as_property() {
            this.session().set_options(this.options());
        }
        if let Some(area) = this.area.get() {
            area.invalidate_layout_cache();
        }
    }
}

ferro_properties! { impl TextEditor {
    pub fn text_property() -> DirectProperty<TextEditor, String> {
        FerroProperty::register_direct::<TextEditor, _>("Text", TextEditor::text, Some(TextEditor::set_text_from_markup), String::new())
    }
    pub fn document_property() -> StyledProperty<Option<TextDocument>> {
        FerroProperty::register::<TextEditor, _>("Document", None)
    }
    pub fn is_read_only_property() -> StyledProperty<bool> {
        FerroProperty::register::<TextEditor, _>("IsReadOnly", false)
    }
    pub fn show_line_numbers_property() -> StyledProperty<bool> {
        FerroProperty::register::<TextEditor, _>("ShowLineNumbers", false)
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
    pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        TextBlock::foreground_property().add_owner::<TextEditor>()
    }
    pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextEditor, _>("Background", None)
    }
    pub fn selection_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextEditor, _>("SelectionBrush", Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0x604d90fe)) as Rc<dyn IBrush>))
    }
    pub fn line_numbers_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<TextEditor, _>("LineNumbersForeground", Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0xff858585)) as Rc<dyn IBrush>))
    }
    pub fn options_property() -> StyledProperty<EditorOptions> {
        FerroProperty::register::<TextEditor, _>("Options", EditorOptions::default())
    }
} }

impl TextEditor {
    pub fn construct() -> Self {
        Self {
            base: Decorator::construct(),
            session: RefCell::new(EditorSession::new(TextDocument::new())),
            area: OnceCell::new(),
            scroll_viewer: OnceCell::new(),
            subscription: RefCell::new(None),
            textmate: RefCell::new(None),
            last_error: RefCell::new(None),
            previous_snapshot: RefCell::new(TextDocument::new().create_snapshot()),
            search_panel: OnceCell::new(),
            folding: RefCell::new(None),
            folding_subscription: RefCell::new(None),
            highlighter: RefCell::new(None),
            highlighting_definition: RefCell::new(None),
            snippet: RefCell::new(None),
            textmate_subscription: RefCell::new(None),
            previous_selection: RefCell::new(Selection::empty(0)),
            commands: RefCell::new(HashMap::new()),
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
        pub fn selection_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TextEditor, _>("SelectionChanged", RoutingStrategies::BUBBLE)
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
    fn set_text_from_markup(&self, text: String) {
        if let Err(error) = self.set_text(&text) {
            self.report_error(error.to_string());
        }
    }
    pub fn text_area(&self) -> Ref<TextArea> {
        self.area.get().expect("constructed editor").clone()
    }
    pub fn search_panel(&self) -> Ref<crate::SearchPanel> {
        self.search_panel
            .get_or_init(|| crate::SearchPanel::install(&self.to_ref()))
            .clone()
    }
    pub fn installed_search_panel(&self) -> Option<Ref<crate::SearchPanel>> {
        self.search_panel.get().cloned()
    }
    pub fn scroll_viewer(&self) -> Ref<ScrollViewer> {
        self.scroll_viewer
            .get()
            .expect("constructed editor")
            .clone()
    }
    pub fn caret_offset(&self) -> usize {
        self.session().caret_offset()
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
        let changed = self.session().undo()?;
        self.selection_changed();
        Ok(changed)
    }
    pub fn redo(&self) -> Result<bool, EditError> {
        let changed = self.session().redo()?;
        self.selection_changed();
        Ok(changed)
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
    pub fn word_wrap(&self) -> bool {
        self.get_value(Self::word_wrap_property())
    }
    pub fn set_word_wrap(&self, value: bool) {
        self.set_value(Self::word_wrap_property(), value);
    }
    pub fn font_family(&self) -> FontFamily {
        self.get_value(Self::font_family_property())
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
    pub fn line_numbers_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::line_numbers_foreground_property())
    }
    pub fn options(&self) -> EditorOptions {
        self.get_value(Self::options_property())
    }
    pub fn set_options(&self, value: EditorOptions) {
        self.set_value(Self::options_property(), value);
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
                editor.text_area().invalidate_layout_cache();
            }
        }));
        *self.textmate.borrow_mut() = Some(installation.clone());
        self.text_area().invalidate_layout_cache();
        installation
    }
    pub fn uninstall_textmate(&self) {
        if let Some(subscription) = self.textmate_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.textmate.borrow_mut().take();
        self.text_area().invalidate_layout_cache();
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
            }
        });
        *self.folding_subscription.borrow_mut() = Some(subscription);
        *self.folding.borrow_mut() = Some(manager.clone());
        self.text_area().invalidate_layout_cache();
        manager
    }
    pub fn folding_manager(&self) -> Option<Rc<FoldingManager>> {
        self.folding.borrow().clone()
    }
    pub fn uninstall_folding(&self) {
        self.folding_subscription.borrow_mut().take();
        self.folding.borrow_mut().take();
        self.text_area().invalidate_layout_cache();
    }
    pub fn set_syntax_highlighting(&self, definition: Option<Rc<HighlightingDefinition>>) {
        *self.highlighter.borrow_mut() = definition.as_ref().map(|definition| {
            Rc::new(DocumentHighlighter::new(
                self.document(),
                HighlightingEngine::new(definition.clone()),
            ))
        });
        *self.highlighting_definition.borrow_mut() = definition;
        self.text_area().invalidate_layout_cache();
    }
    pub fn syntax_highlighting(&self) -> Option<Rc<HighlightingDefinition>> {
        self.highlighting_definition.borrow().clone()
    }
    pub fn document_highlighter(&self) -> Option<Rc<DocumentHighlighter>> {
        self.highlighter.borrow().clone()
    }
    pub fn insert_snippet(
        &self,
        snippet: &Snippet,
    ) -> Result<Rc<crate::SnippetController>, SnippetError> {
        if let Some(previous) = self.snippet.borrow_mut().take() {
            previous.finish();
        }
        let controller = crate::SnippetController::insert(&self.to_ref(), snippet)?;
        *self.snippet.borrow_mut() = Some(controller.clone());
        Ok(controller)
    }

    pub(crate) fn selection_changed(&self) {
        if let Some(area) = self.area.get() {
            area.selection_changed();
        }
        let selection = self.selection();
        let changed = *self.previous_selection.borrow() != selection;
        *self.previous_selection.borrow_mut() = selection;
        if changed {
            self.raise_event(&RoutedEventArgs::with_event(Self::selection_changed_event()));
        }
    }

    fn attach_document(&self, document: TextDocument) {
        if let Some(subscription) = self.subscription.borrow_mut().take() {
            subscription.dispose();
        }
        if let Some(snippet) = self.snippet.borrow_mut().take() {
            snippet.finish();
        }
        self.folding_subscription.borrow_mut().take();
        self.folding.borrow_mut().take();
        let session = EditorSession::new(document.clone());
        session.set_options(self.options());
        session.set_is_read_only(self.is_read_only());
        *self.session.borrow_mut() = session;
        let definition = self.syntax_highlighting();
        *self.highlighter.borrow_mut() = definition.map(|definition| {
            Rc::new(DocumentHighlighter::new(
                document.clone(),
                HighlightingEngine::new(definition),
            ))
        });
        if let Some(installation) = self.textmate_installation() {
            installation.invalidate_from(1);
        }
        let weak = self.to_ref().downgrade();
        let subscription = document.changed(move |change| {
            if let Some(editor) = weak.upgrade() {
                if let Some(installation) = editor.textmate_installation() {
                    let line = editor
                        .document()
                        .get_line_by_offset(change.offset.min(editor.document().text_length()))
                        .map(|line| line.line_number())
                        .unwrap_or(1);
                    installation.invalidate_from(line);
                }
                if let Some(area) = editor.area.get() {
                    area.document_changed();
                }
                editor.selection_changed();
                editor.notify_text_property();
                editor.raise_event(&RoutedEventArgs::with_event(Self::text_changed_event()));
            }
        });
        *self.subscription.borrow_mut() = Some(subscription);
        self.notify_text_property();
        if let Some(area) = self.area.get() {
            area.document_changed();
        }
        self.selection_changed();
        self.raise_event(&RoutedEventArgs::with_event(Self::text_changed_event()));
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
        let session = self.session();
        let Ok(text) = session.selected_text() else {
            return;
        };
        if text.is_empty() || cut && self.is_read_only() {
            return;
        }
        let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top| top.clipboard())
        else {
            return;
        };
        let selection = session.selection();
        let version = session.document().version();
        let weak = self.to_ref().downgrade();
        drop(
            Dispatcher::ui_thread()
                .to_task_scheduler()
                .start_local(async move {
                    let result = clipboard.set_text_async(Some(&text)).await;
                    if let Some(editor) = weak.upgrade() {
                        match result {
                            Ok(())
                                if cut
                                    && editor.document() == *session.document()
                                    && editor.document().version() == version
                                    && editor.selection() == selection =>
                            {
                                if let Err(error) = editor.replace_selection_with_text("") {
                                    editor.report_error(error.to_string());
                                }
                            }
                            Err(error) => editor.report_error(error.to_string()),
                            _ => {}
                        }
                    }
                }),
        );
    }
    pub fn paste(&self) {
        if self.is_read_only() {
            return;
        }
        let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top| top.clipboard())
        else {
            return;
        };
        let document = self.document();
        let weak = self.to_ref().downgrade();
        drop(
            Dispatcher::ui_thread()
                .to_task_scheduler()
                .start_local(async move {
                    let result = clipboard.try_get_text_async().await;
                    if let Some(editor) = weak.upgrade() {
                        match result {
                            Ok(Some(text)) if editor.document() == document => {
                                if let Err(error) = editor.replace_selection_with_text(&text) {
                                    editor.report_error(error.to_string());
                                }
                            }
                            Err(error) => editor.report_error(error.to_string()),
                            _ => {}
                        }
                    }
                }),
        );
    }
}

impl Drop for TextEditor {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.textmate_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
