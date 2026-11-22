use crate::{search_strings::SearchLanguage, text_editor::EditTarget, EditorCommandKind, TextEditor};
use ferroui_base::input::{
    ICommand, IScrollable, InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::{HandlerList, current_locale};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    StyledElementImpl, StyledProperty, Thickness, VisualImpl, WeakRef, ferro_class,
    ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::popup_positioning::{PopupAnchor, PopupGravity};
use ferroui_controls::{
    Application, Border, Button, CheckBox, Control, ControlImpl, PlacementMode, StackPanel, TextBlock,
    TextBox, ToolTip,
};
use ferroui_edit_core::document::{DocumentEdit, SimpleSegment};
use ferroui_edit_core::editing::{Selection, SelectionSegment};
use ferroui_edit_core::search::{SearchMode, SearchOptions, SearchResult, SearchStrategyFactory};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// The accepted pattern and flags of the search panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchOptionsChangedEventArgs {
    pub search_pattern: String,
    pub match_case: bool,
    pub use_regex: bool,
    pub whole_words: bool,
}

/// An editor search/replace surface made from framework controls and hosted by
/// an overlay popup. Search results retain the snapshot used for replacement.
#[repr(C)]
pub struct SearchPanel {
    base: Popup,
    editor: RefCell<Option<WeakRef<TextEditor>>>,
    query: OnceCell<Ref<TextBox>>,
    replacement: OnceCell<Ref<TextBox>>,
    replacement_row: OnceCell<Ref<StackPanel>>,
    status: OnceCell<Ref<TextBlock>>,
    surface: OnceCell<Ref<Border>>,
    flags: OnceCell<[Ref<CheckBox>; 3]>,
    action_buttons: OnceCell<[(EditorCommandKind, Ref<Button>); 5]>,
    language: Cell<SearchLanguage>,
    results: RefCell<Vec<SearchResult>>,
    results_stale: Cell<bool>,
    current: Cell<Option<usize>>,
    error: RefCell<Option<String>>,
    synchronizing: Cell<bool>,
    search_options_changed: HandlerList<dyn Fn(&Self, &SearchOptionsChangedEventArgs)>,
    subscriptions: RefCell<Vec<Subscription>>,
}
ferro_class!(SearchPanel:Popup);
ferroui_base::ferro_class_info!(SearchPanel {
    new: SearchPanel::new
});
ferro_impl_classes!(SearchPanel:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl FerroObjectImpl for SearchPanel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.build_content();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::search_pattern_property().as_property() {
            if let Some(query) = this.query.get() {
                this.synchronizing.set(true);
                query.set_text(Some(&this.search_pattern()));
                this.synchronizing.set(false);
            }
            this.refresh_results(true);
        } else if change.property() == Self::replacement_pattern_property().as_property() {
            if let Some(replacement) = this.replacement.get() {
                this.synchronizing.set(true);
                replacement.set_text(Some(&this.replacement_pattern()));
                this.synchronizing.set(false);
            }
        } else if change.property() == Self::match_case_property().as_property()
            || change.property() == Self::whole_words_property().as_property()
            || change.property() == Self::use_regex_property().as_property()
        {
            this.synchronize_flags();
            this.refresh_results(true);
        } else if change.property() == Popup::is_open_property().as_property() {
            this.notify_command_state();
        }
    }
}
ferro_properties! {impl SearchPanel{
    pub fn search_pattern_property()->StyledProperty<String>{FerroProperty::register::<SearchPanel,_>("SearchPattern",String::new())}
    pub fn replacement_pattern_property()->StyledProperty<String>{FerroProperty::register::<SearchPanel,_>("ReplacementPattern",String::new())}
    pub fn match_case_property()->StyledProperty<bool>{FerroProperty::register::<SearchPanel,_>("MatchCase",false)}
    pub fn whole_words_property()->StyledProperty<bool>{FerroProperty::register::<SearchPanel,_>("WholeWords",false)}
    pub fn use_regex_property()->StyledProperty<bool>{FerroProperty::register::<SearchPanel,_>("UseRegex",false)}
}}
impl SearchPanel {
    pub fn construct() -> Self {
        Self {
            base: Popup::construct(),
            editor: RefCell::new(None),
            query: OnceCell::new(),
            replacement: OnceCell::new(),
            replacement_row: OnceCell::new(),
            status: OnceCell::new(),
            surface: OnceCell::new(),
            flags: OnceCell::new(),
            action_buttons: OnceCell::new(),
            language: Cell::new(SearchLanguage::default()),
            results: RefCell::new(Vec::new()),
            results_stale: Cell::new(false),
            current: Cell::new(None),
            error: RefCell::new(None),
            synchronizing: Cell::new(false),
            search_options_changed: HandlerList::new(),
            subscriptions: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn install(editor: &Ref<TextEditor>) -> Ref<Self> {
        if let Some(panel) = editor.installed_search_panel() {
            return panel;
        }
        let panel = Self::new();
        *panel.editor.borrow_mut() = Some(editor.downgrade());
        panel.set_placement_target(editor.clone());
        panel.set_popup_parent(Some(&editor.clone().upcast::<ferroui_controls::Control>()));
        panel.set_placement(PlacementMode::AnchorAndGravity);
        panel.set_placement_anchor(PopupAnchor::TOP_RIGHT);
        panel.set_placement_gravity(PopupGravity::BOTTOM_LEFT);
        panel.set_should_use_overlay_layer(true);
        panel.set_is_light_dismiss_enabled(false);
        let weak = panel.downgrade();
        let subscription =
            editor.add_disposable_handler(TextEditor::text_changed_event(), move |_, _| {
                if let Some(panel) = weak.upgrade() {
                    // A search copies the document: a closed panel searches when it is used.
                    if panel.is_open() {
                        panel.refresh();
                    } else {
                        panel.results_stale.set(true);
                    }
                }
            });
        panel.subscriptions.borrow_mut().push(subscription);
        let weak = panel.downgrade();
        let subscription =
            editor.add_disposable_handler(TextEditor::selection_changed_event(), move |_, _| {
                if let Some(panel) = weak.upgrade() {
                    if panel.current.take().is_some() {
                        panel.update_status();
                    }
                }
            });
        panel.subscriptions.borrow_mut().push(subscription);
        let weak = panel.downgrade();
        panel
            .subscriptions
            .borrow_mut()
            .push(editor.actual_theme_variant_changed(move || {
                if let Some(panel) = weak.upgrade() {
                    panel.update_surface();
                }
            }));
        panel.update_surface();
        if let Some(settings) = Application::current().and_then(|app| app.platform_settings()) {
            panel.set_platform_language(&settings.preferred_application_language());
            let weak_panel = panel.downgrade();
            let weak_settings = Rc::downgrade(&settings);
            let subscription = settings.preferred_application_language_changed(Rc::new(move || {
                if let (Some(panel), Some(settings)) = (weak_panel.upgrade(), weak_settings.upgrade()) {
                    panel.set_platform_language(&settings.preferred_application_language());
                }
            }));
            panel.subscriptions.borrow_mut().push(subscription.into());
        }
        editor.attach_search_panel(panel.clone());
        if let Some(buttons) = panel.action_buttons.get() {
            for (kind, button) in buttons {
                button.set_command(Some(editor.command(*kind).as_command()));
            }
        }
        panel
    }
    pub fn search_pattern(&self) -> String {
        self.get_value(Self::search_pattern_property())
    }
    pub fn set_search_pattern(&self, value: &str) {
        self.set_value(Self::search_pattern_property(), value.to_owned());
    }
    pub fn replacement_pattern(&self) -> String {
        self.get_value(Self::replacement_pattern_property())
    }
    pub fn set_replacement_pattern(&self, value: &str) {
        self.set_value(Self::replacement_pattern_property(), value.to_owned());
    }
    pub fn match_case(&self) -> bool {
        self.get_value(Self::match_case_property())
    }
    pub fn set_match_case(&self, value: bool) {
        self.set_value(Self::match_case_property(), value);
    }
    pub fn whole_words(&self) -> bool {
        self.get_value(Self::whole_words_property())
    }
    pub fn set_whole_words(&self, value: bool) {
        self.set_value(Self::whole_words_property(), value);
    }
    pub fn use_regex(&self) -> bool {
        self.get_value(Self::use_regex_property())
    }
    pub fn set_use_regex(&self, value: bool) {
        self.set_value(Self::use_regex_property(), value);
    }
    /// Subscribes to accepted search options before the results are refreshed.
    pub fn search_options_changed(
        &self,
        handler: impl Fn(&Self, &SearchOptionsChangedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.search_options_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(panel) = weak.upgrade() {
                panel.search_options_changed.remove(token);
            }
        })
        .into()
    }
    pub fn result_count(&self) -> usize {
        self.refresh_stale_results();
        self.results.borrow().len()
    }
    /// Snapshot match ranges intersecting a visible document interval. Rendering
    /// hosts can highlight results without copying all captured match text.
    pub fn results_in_range(&self, offset: usize, length: usize) -> Vec<SimpleSegment> {
        // Rendering calls this: a closed panel whose results are out of date draws nothing.
        if self.results_stale.get() {
            return Vec::new();
        }
        let results = self.results.borrow();
        let end = offset.saturating_add(length);
        let first =
            results.partition_point(|result| result.offset.saturating_add(result.length) < offset);
        results[first..]
            .iter()
            .take_while(|result| result.offset <= end)
            .map(|result| SimpleSegment::new(result.offset, result.length))
            .collect()
    }
    pub fn error(&self) -> Option<String> {
        self.error.borrow().clone()
    }
    fn editor(&self) -> Option<Ref<TextEditor>> {
        self.editor.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    fn notify_command_state(&self) {
        if let Some(editor) = self.editor() {
            editor.notify_search_state_changed();
        }
    }
    pub fn is_replace_mode(&self) -> bool {
        self.replacement_row.get().is_some_and(|row| row.is_visible())
    }
    pub fn show(&self, replace: bool) {
        if let Some(row) = self.replacement_row.get() {
            let replace = replace && self.editor().is_some_and(|editor| {
                !editor.is_read_only() && !editor.document().is_read_only()
            });
            if replace != row.is_visible() {
                row.set_is_visible(replace);
                self.notify_command_state();
            }
        }
        let focused = self.surface.get().is_some_and(|surface| surface.is_keyboard_focus_within());
        if !focused {
            let single_line = self
                .editor()
                .and_then(|editor| editor.selected_text().ok())
                .filter(|text| !text.is_empty() && !text.contains(['\n', '\r']));
            if let Some(selected) = single_line {
                self.set_search_pattern(&selected);
            }
        }
        self.update_surface();
        self.refresh();
        self.open();
        self.reactivate();
    }
    /// Focuses the query and selects its text so typing can replace it.
    pub fn reactivate(&self) {
        if let Some(query) = self.query.get() {
            query.focus();
            query.select_all();
        }
    }
    pub fn close_panel(&self) {
        self.close();
        if let Some(editor) = self.editor() {
            editor.text_area().invalidate_visual();
            editor.text_area().focus();
        }
    }
    pub fn uninstall(&self) {
        let editor = self.editor.borrow_mut().take().and_then(|editor| editor.upgrade());
        if let Some(editor) = editor {
            editor.detach_search_panel(&self.to_ref());
            editor.text_area().invalidate_visual();
        }
        self.close();
        if let Some(buttons) = self.action_buttons.get() {
            for (_, button) in buttons {
                button.set_command(None);
            }
        }
        for subscription in self.subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        self.results.borrow_mut().clear();
        self.current.set(None);
        self.set_placement_target(None::<Ref<ferroui_controls::Control>>);
        self.set_popup_parent(None);
    }

    pub(crate) fn run_search_command(&self, kind: EditorCommandKind) {
        use EditorCommandKind::{CloseSearchPanel, FindNext, FindPrevious, ReplaceAll, ReplaceNext};
        let result = match kind {
            FindNext => {
                self.find_next();
                Ok(())
            }
            FindPrevious => {
                self.find_previous();
                Ok(())
            }
            CloseSearchPanel => {
                self.close_panel();
                Ok(())
            }
            ReplaceNext => self.replace_next().map(|_| ()),
            ReplaceAll => self.replace_all().map(|_| ()),
            _ => return,
        };
        if let Err(error) = result {
            *self.error.borrow_mut() = Some(error);
            self.update_status();
        }
    }
    pub fn refresh(&self) {
        self.refresh_results(false);
    }
    fn search_options(&self) -> SearchOptionsChangedEventArgs {
        SearchOptionsChangedEventArgs {
            search_pattern: self.search_pattern(),
            match_case: self.match_case(),
            use_regex: self.use_regex(),
            whole_words: self.whole_words(),
        }
    }
    fn refresh_stale_results(&self) {
        if self.results_stale.get() {
            self.refresh();
        }
    }

    fn refresh_results(&self, notify_options: bool) {
        self.results_stale.set(false);
        let options = self.search_options();
        let strategy = (!options.search_pattern.is_empty())
            .then(|| {
                let mode = if options.use_regex {
                    SearchMode::RegEx
                } else {
                    SearchMode::Normal
                };
                SearchStrategyFactory::with_options_for_language(
                    &options.search_pattern,
                    SearchOptions {
                        ignore_case: !options.match_case,
                        match_whole_words: options.whole_words,
                        mode,
                    },
                    current_locale().id.language.as_str(),
                )
            })
            .transpose();
        if notify_options && strategy.is_ok() && !self.search_options_changed.is_empty() {
            for (_, handler) in self.search_options_changed.snapshot().iter() {
                handler(self, &options);
            }
            // A handler can update the options and publish a newer search.
            if self.search_options() != options {
                return;
            }
        }
        let Some(editor) = self.editor() else {
            return;
        };
        let result = strategy.and_then(|strategy| {
            let Some(strategy) = strategy else {
                return Ok(Vec::new());
            };
            let text = editor.text();
            strategy.find_all(&text, 0, editor.document().text_length())
        });
        self.current.set(None);
        match result {
            Ok(results) => {
                *self.results.borrow_mut() = results;
                *self.error.borrow_mut() = None;
                if notify_options && self.is_open() {
                    self.select_match_for_updated_options(&editor);
                }
            }
            Err(error) => {
                self.results.borrow_mut().clear();
                *self.error.borrow_mut() = Some(error.to_string());
            }
        }
        self.update_status();
        editor.text_area().invalidate_visual();
    }
    fn select_match_for_updated_options(&self, editor: &TextEditor) {
        let selection = editor.selection();
        let caret = selection.primary();
        let start = caret.caret.saturating_sub(selection.length());
        let (index, updated) = {
            let results = self.results.borrow();
            let index = results.partition_point(|result| result.offset < start);
            let index = if index == results.len() { 0 } else { index };
            match results.get(index) {
                Some(result) => (
                    Some(index),
                    Selection::simple(result.offset, result.offset + result.length),
                ),
                None => (
                    None,
                    Selection::from_segments(
                        [SelectionSegment {
                            anchor: caret.caret,
                            anchor_virtual_space: caret.caret_virtual_space,
                            ..caret
                        }],
                        0,
                    ),
                ),
            }
        };
        let index = if selection != updated && editor.set_selection(updated).is_err() {
            None
        } else {
            index
        };
        self.current.set(index);
        if index.is_some() {
            self.reveal_search_result(editor);
        }
    }

    fn reveal_search_result(&self, editor: &TextEditor) {
        let Some(surface) = self.surface.get().filter(|_| self.is_open()) else {
            return;
        };
        let area = editor.text_area();
        let Some(transform) = surface.transform_to_visual(&area) else {
            return;
        };
        let overlay = Rect::from_size(surface.bounds().size()).transform_to_aabb(transform);
        let caret = area.cursor_rectangle();
        if !caret.intersects(overlay) {
            return;
        }

        let clearance = area.viewport().height - overlay.bottom() - caret.height;
        if clearance >= 0.0 {
            let mut offset = area.offset();
            let gap = clearance.min(area.line_height());
            offset.y += caret.y - overlay.bottom() - gap;
            area.set_offset(offset);
        }
    }
    fn update_status(&self) {
        if let Some(status) = self.status.get() {
            let text = self.language.get().status(
                self.result_count(),
                self.current.get(),
                self.error().as_deref(),
            );
            status.set_text(Some(&text));
        }
    }

    fn set_platform_language(&self, tag: &str) {
        let language = SearchLanguage::from_tag(tag);
        if self.language.replace(language) != language {
            self.update_labels();
            self.update_status();
        }
    }

    fn update_labels(&self) {
        let strings = self.language.get().strings();
        if let Some(query) = self.query.get() {
            query.set_placeholder_text(Some(strings.find));
        }
        if let Some(replacement) = self.replacement.get() {
            replacement.set_placeholder_text(Some(strings.replace));
        }
        if let Some(flags) = self.flags.get() {
            for (flag, label) in flags.iter().zip(strings.flags) {
                flag.set_content(Some(Rc::new(label.to_owned())));
            }
        }
        if let Some(buttons) = self.action_buttons.get() {
            for ((_, button), (label, tip)) in buttons.iter().zip(strings.buttons) {
                button.set_content(Some(Rc::new(label.to_owned())));
                ToolTip::set_tip(button.upcast_ref::<Control>(), Some(Rc::new(tip.to_owned())));
            }
        }
    }
    pub fn find_next(&self) -> bool {
        self.navigate(false)
    }
    pub fn find_previous(&self) -> bool {
        self.navigate(true)
    }
    fn navigate(&self, backwards: bool) -> bool {
        let Some(editor) = self.editor() else {
            return false;
        };
        self.refresh_stale_results();
        let results = self.results.borrow();
        if results.is_empty() {
            return false;
        }
        let selected = editor.selection().primary();
        let caret = editor.caret_offset();
        let start = if selected.is_empty() { caret } else { selected.start_offset() };
        let first_at_start = results.partition_point(|result| result.offset < start);
        let index = if backwards {
            first_at_start.checked_sub(1).unwrap_or(results.len() - 1)
        } else if results.get(first_at_start).is_some_and(|result| {
            result.offset == start && result.length == selected.length()
                && (!selected.is_empty() || self.current.get() == Some(first_at_start))
        }) {
            (first_at_start + 1) % results.len()
        } else {
            results.partition_point(|result| result.offset < caret) % results.len()
        };
        drop(results);
        self.select_result(&editor, index)
    }
    fn select_result(&self, editor: &TextEditor, index: usize) -> bool {
        let (offset, length) = {
            let results = self.results.borrow();
            (results[index].offset, results[index].length)
        };
        if editor.select(offset, length).is_err() {
            return false;
        }
        self.reveal_search_result(editor);
        self.current.set(Some(index));
        self.update_status();
        true
    }
    pub fn replace_next(&self) -> Result<bool, String> {
        let editor = self
            .editor()
            .ok_or_else(|| "Search panel is not installed".to_owned())?;
        self.refresh();
        if let Some(error) = self.error() {
            return Err(error);
        }
        let start = editor.selection().primary().start_offset();
        let result = {
            let results = self.results.borrow();
            let index = results.partition_point(|result| result.offset < start);
            results.get(index).or_else(|| results.first()).cloned()
        };
        let Some(result) = result else {
            return Ok(false);
        };
        let replacement = if self.use_regex() {
            result.try_replace_with(&self.replacement_pattern()).map_err(|error| error.to_string())?
        } else {
            self.replacement_pattern()
        };
        let replacement_end = result.offset + replacement.encode_utf16().count();
        let changes = usize::from(result.length != 0 || !replacement.is_empty());
        let target = EditTarget::new(&editor);
        let edit = DocumentEdit::new(result.offset, result.length, replacement);
        target
            .apply(&editor, &[edit])
            .map_err(|e| e.to_string())?;
        if target.is_current_after(&editor, changes)
            && self.editor().is_some_and(|owner| owner == editor)
        {
            self.refresh_stale_results();
            let results = self.results.borrow();
            let count = results.len();
            // An empty match can survive at the boundary after insertion.
            let next = results.partition_point(|next| {
                next.offset < replacement_end
                    || (next.offset == replacement_end && result.length == 0)
            });
            drop(results);
            if count == 0 {
                editor.set_caret_offset(replacement_end).map_err(|error| error.to_string())?;
            } else {
                self.select_result(&editor, next % count);
            }
        }
        Ok(true)
    }
    pub fn replace_all(&self) -> Result<usize, String> {
        let editor = self
            .editor()
            .ok_or_else(|| "Search panel is not installed".to_owned())?;
        self.refresh();
        if let Some(error) = self.error() {
            return Err(error);
        }
        let replacement = self.replacement_pattern();
        let regex = self.use_regex();
        let edits: Vec<_> = self
            .results
            .borrow()
            .iter()
            .map(|r| {
                Ok(DocumentEdit::new(
                    r.offset,
                    r.length,
                    if regex {
                        r.try_replace_with(&replacement)?
                    } else {
                        replacement.clone()
                    },
                ))
            })
            .collect::<Result<_, ferroui_edit_core::search::SearchError>>()
            .map_err(|error| error.to_string())?;
        if !edits.is_empty() {
            EditTarget::new(&editor)
                .apply(&editor, &edits)
                .map_err(|e| e.to_string())?;
        }
        Ok(edits.len())
    }
    fn synchronize_flags(&self) {
        if let Some(flags) = self.flags.get() {
            for (control, value) in
                flags
                    .iter()
                    .zip([self.match_case(), self.whole_words(), self.use_regex()])
            {
                control.set_is_checked(Some(value));
            }
        }
    }
    fn update_surface(&self) {
        if let (Some(editor), Some(border)) = (self.editor(), self.surface.get()) {
            style_overlay(border, &editor);
        }
    }
    fn build_content(&self) {
        let content = StackPanel::new();
        content.set_spacing(6.0);
        let query_row = StackPanel::new();
        query_row.set_orientation(Orientation::Horizontal);
        query_row.set_spacing(4.0);
        let query = TextBox::new();
        query.set_width(260.0);
        query_row
            .children()
            .add(query.clone().upcast::<ferroui_controls::Control>());
        let previous = Button::new();
        let next = Button::new();
        let close = Button::new();
        query_row
            .children()
            .add(previous.clone().upcast::<ferroui_controls::Control>());
        query_row
            .children()
            .add(next.clone().upcast::<ferroui_controls::Control>());
        query_row
            .children()
            .add(close.clone().upcast::<ferroui_controls::Control>());
        content
            .children()
            .add(query_row.upcast::<ferroui_controls::Control>());
        let flags = StackPanel::new();
        flags.set_orientation(Orientation::Horizontal);
        flags.set_spacing(10.0);
        let mut checkboxes = Vec::new();
        for kind in 0..3 {
            let checkbox = CheckBox::new();
            let weak = self.to_ref().downgrade();
            let check = checkbox.downgrade();
            let subscription =
                checkbox.add_disposable_handler(Button::click_event(), move |_, _| {
                    if let (Some(panel), Some(check)) = (weak.upgrade(), check.upgrade()) {
                        let value = check.is_checked() == Some(true);
                        match kind {
                            0 => panel.set_match_case(value),
                            1 => panel.set_whole_words(value),
                            _ => panel.set_use_regex(value),
                        }
                    }
                });
            self.subscriptions.borrow_mut().push(subscription);
            checkboxes.push(checkbox.clone());
            flags
                .children()
                .add(checkbox.upcast::<ferroui_controls::Control>());
        }
        if let Ok(checkboxes) = checkboxes.try_into() {
            self.flags.set(checkboxes).ok();
        }
        self.synchronize_flags();
        content
            .children()
            .add(flags.upcast::<ferroui_controls::Control>());
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(4.0);
        row.set_is_visible(false);
        let replacement = TextBox::new();
        replacement.set_width(260.0);
        row.children()
            .add(replacement.clone().upcast::<ferroui_controls::Control>());
        let replace_next = Button::new();
        let replace_all = Button::new();
        row.children().add(replace_next.clone().upcast::<ferroui_controls::Control>());
        row.children().add(replace_all.clone().upcast::<ferroui_controls::Control>());
        self.action_buttons.set([
            (EditorCommandKind::FindPrevious, previous),
            (EditorCommandKind::FindNext, next),
            (EditorCommandKind::CloseSearchPanel, close),
            (EditorCommandKind::ReplaceNext, replace_next),
            (EditorCommandKind::ReplaceAll, replace_all),
        ]).ok();
        content
            .children()
            .add(row.clone().upcast::<ferroui_controls::Control>());
        let status = TextBlock::new();
        content
            .children()
            .add(status.clone().upcast::<ferroui_controls::Control>());
        let border = Border::new();
        border.set_padding(Thickness::uniform(10.0));
        border.set_child(content);
        self.surface.set(border.clone()).ok();
        self.set_child(border);
        self.query.set(query.clone()).ok();
        self.replacement.set(replacement.clone()).ok();
        self.replacement_row.set(row).ok();
        self.status.set(status).ok();
        self.update_labels();
        self.update_status();
        let weak = self.to_ref().downgrade();
        let input = query.downgrade();
        self.subscriptions
            .borrow_mut()
            .push(
                query.add_disposable_handler(TextBox::text_changing_event(), move |_, _| {
                    if let (Some(panel), Some(input)) = (weak.upgrade(), input.upgrade()) {
                        if !panel.synchronizing.get() {
                            panel.set_search_pattern(&input.text().unwrap_or_default());
                        }
                    }
                }),
            );
        let weak = self.to_ref().downgrade();
        let input = replacement.downgrade();
        self.subscriptions
            .borrow_mut()
            .push(replacement.add_disposable_handler(
                TextBox::text_changing_event(),
                move |_, _| {
                    if let (Some(panel), Some(input)) = (weak.upgrade(), input.upgrade()) {
                        if !panel.synchronizing.get() {
                            panel.set_replacement_pattern(&input.text().unwrap_or_default());
                        }
                    }
                },
            ));
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(self.add_disposable_handler_with(
                InputElement::key_down_event(),
                move |_, e: &KeyEventArgs| {
                    if let Some(editor) = weak.upgrade().and_then(|panel| panel.editor()) {
                        let keymap = crate::text_area::platform_keymap();
                        dispatch_search_key(&editor, e, keymap.command_modifiers, true);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            ));
    }
}
impl Drop for SearchPanel {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

pub(crate) fn dispatch_search_key(
    editor: &TextEditor,
    event: &KeyEventArgs,
    command_modifiers: KeyModifiers,
    from_panel: bool,
) -> bool {
    use EditorCommandKind::*;
    let replace_gesture = if command_modifiers == KeyModifiers::META {
        (Key::F, command_modifiers | KeyModifiers::ALT)
    } else {
        (Key::H, command_modifiers)
    };
    let kind = match (event.key, event.key_modifiers) {
        (Key::F, mods) if mods == command_modifiers => Find,
        gesture if gesture == replace_gesture => Replace,
        (Key::F3, KeyModifiers::NONE) => FindNext,
        (Key::F3, KeyModifiers::SHIFT) => FindPrevious,
        (Key::Escape, KeyModifiers::NONE) => CloseSearchPanel,
        (Key::R, KeyModifiers::ALT) => ReplaceNext,
        (Key::A, KeyModifiers::ALT) => ReplaceAll,
        (Key::Return, KeyModifiers::NONE) if from_panel => FindNext,
        (Key::Return, KeyModifiers::SHIFT) if from_panel => FindPrevious,
        _ => return false,
    };
    if !matches!(kind, Find | Replace) && editor.installed_search_panel().is_none() {
        return false;
    }
    let command = editor.command(kind);
    if !command.can_execute(None) {
        return false;
    }
    command.execute(None);
    event.set_handled(true);
    true
}

pub(crate) fn style_overlay(border: &Ref<Border>, editor: &Ref<TextEditor>) {
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::{IBrush, SolidColorBrush};
    use ferroui_base::styling::ThemeVariant;
    let theme = editor.actual_theme_variant();
    let background = [
        "FlyoutPresenterBackground",
        "SystemControlBackgroundChromeMediumLowBrush",
    ]
    .into_iter()
    .find_map(|key| {
        let resource = editor
            .try_find_resource(&key.into(), theme.as_ref())
            .flatten()?;
        if let Some(brush) = resource.downcast_ref::<Rc<dyn IBrush>>() {
            Some(brush.clone())
        } else if let Some(brush) = resource.downcast_ref::<Option<Rc<dyn IBrush>>>() {
            brush.clone()
        } else if let Some(brush) = resource.downcast_ref::<Ref<SolidColorBrush>>() {
            Some(brush.clone().into())
        } else if let Some(brush) = resource.downcast_ref::<Rc<ImmutableSolidColorBrush>>() {
            Some(brush.clone() as Rc<dyn IBrush>)
        } else {
            None
        }
    })
    .unwrap_or_else(|| {
        Rc::new(ImmutableSolidColorBrush::from_uint32(
            if theme == Some(ThemeVariant::dark()) {
                0xff252526
            } else {
                0xfff3f3f3
            },
        )) as Rc<dyn IBrush>
    });
    border.set_background(Some(background));
    border.set_value(
        TextBlock::foreground_property(),
        Some(editor.foreground().unwrap_or_else(|| {
            Rc::new(ImmutableSolidColorBrush::from_uint32(
                if theme == Some(ThemeVariant::dark()) {
                    0xfff1f1f1
                } else {
                    0xff1f1f1f
                },
            )) as Rc<dyn IBrush>
        })),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_controls::Window;
    use ferroui_edit_core::document::TextDocument;

    #[test]
    fn native_language_changes_update_the_open_panel_and_survive_reinstallation() {
        use ferroui_base::input::{platform::PlatformHotkeyConfiguration, PointerType};
        use ferroui_base::platform::{DefaultPlatformSettings, IPlatformSettings, PlatformColorValues};
        use ferroui_base::reactive::IDisposable;
        use ferroui_base::Size;
        use ferroui_controls::testing::TestServices;
        use std::time::Duration;

        struct Settings {
            base: DefaultPlatformSettings,
            language: Cell<&'static str>,
        }
        impl IPlatformSettings for Settings {
            fn get_tap_size(&self, pointer: PointerType) -> Size {
                self.base.get_tap_size(pointer)
            }
            fn get_double_tap_size(&self, pointer: PointerType) -> Size {
                self.base.get_double_tap_size(pointer)
            }
            fn get_double_tap_time(&self, pointer: PointerType) -> Duration {
                self.base.get_double_tap_time(pointer)
            }
            fn hold_wait_duration(&self) -> Duration {
                self.base.hold_wait_duration()
            }
            fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration> {
                self.base.hotkey_configuration()
            }
            fn preferred_application_language(&self) -> String {
                self.language.get().to_owned()
            }
            fn get_color_values(&self) -> PlatformColorValues {
                self.base.get_color_values()
            }
            fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
                self.base.color_values_changed(handler)
            }
            fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
                self.base.preferred_application_language_changed(handler)
            }
        }

        let settings = Rc::new(Settings {
            base: DefaultPlatformSettings::new(),
            language: Cell::new("en-US"),
        });
        let _scope = crate::tests::test_scope_with_services(
            TestServices::styled_window().with_platform_settings(settings.clone()),
        );
        let editor = crate::tests::laid_out_editor("one one two");
        let root = Window::new();
        root.set_content(Some(Rc::new(editor.clone())));
        root.show();
        let panel = SearchPanel::install(&editor);
        panel.show(true);
        let status = panel.status.get().unwrap();
        assert_eq!(status.text().as_deref(), Some("No matches found"));
        panel.set_search_pattern("one");
        panel.set_replacement_pattern("two");
        assert_eq!(status.text().as_deref(), Some("1 of 2"));
        let selection = editor.selection();

        let set_language = |language| {
            settings.language.set(language);
            settings.base.on_preferred_application_language_changed();
        };
        for tag in ["zh-CN", "zh-Hans-CN", "zh-SG"] {
            set_language(tag);
            assert_eq!(panel.query.get().unwrap().placeholder_text().as_deref(), Some("查找..."));
            assert_eq!(status.text().as_deref(), Some("第1项，共2项"));
        }
        let replace_all = &panel.action_buttons.get().unwrap()[4].1;
        let tip = ToolTip::get_tip(replace_all.upcast_ref::<Control>()).unwrap();
        assert_eq!(tip.downcast_ref::<String>().map(String::as_str), Some("全部替换 (Alt+A)"));
        assert_eq!(panel.replacement_pattern(), "two");
        assert_eq!(editor.selection(), selection);
        assert!(panel.is_open() && panel.is_replace_mode());
        for tag in ["zh", "zh-TW", "zh-MY", "fr-FR"] {
            set_language(tag);
            assert_eq!(status.text().as_deref(), Some("1 of 2"));
        }
        set_language("zh-Hans");
        panel.set_search_pattern("two");
        assert_eq!(status.text().as_deref(), Some("第1项，共1项"));
        panel.set_search_pattern("missing");
        assert_eq!(status.text().as_deref(), Some("无结果"));
        panel.set_use_regex(true);
        panel.set_search_pattern("(");
        assert!(status.text().unwrap().starts_with("错误："));

        panel.uninstall();
        set_language("en-US");
        assert!(status.text().unwrap().starts_with("错误："));
        let replacement = SearchPanel::install(&editor);
        assert_eq!(replacement.query.get().unwrap().placeholder_text().as_deref(), Some("Find..."));
        assert!(replacement != panel);
        root.close();
    }

    #[test]
    fn literal_find_uses_the_formatting_locale_for_utf16_ranges() {
        use ferroui_base::utilities::{CultureInfo, locale, set_current_locale};

        struct RestoreLocale(CultureInfo);
        impl Drop for RestoreLocale {
            fn drop(&mut self) {
                CultureInfo::set_current_formatting(self.0.clone());
            }
        }

        let _scope = crate::tests::test_scope();
        let _restore = RestoreLocale(CultureInfo::current_formatting());
        let editor = crate::tests::laid_out_editor("😀 I i İ ı");
        let panel = editor.search_panel();
        panel.set_search_pattern("I");
        for (language, offsets) in [
            (locale!("tr-TR"), &[3, 9][..]),
            (locale!("az-Latn-AZ"), &[3, 9][..]),
            (locale!("en-US"), &[3, 5, 7][..]),
            (locale!("und"), &[3, 5][..]),
        ] {
            set_current_locale(language);
            panel.refresh();
            assert_eq!(
                panel.results_in_range(0, editor.document().text_length()),
                offsets.iter().map(|&offset| SimpleSegment::new(offset, 1)).collect::<Vec<_>>(),
            );
        }
        set_current_locale(locale!("tr-TR"));
        panel.set_match_case(true);
        assert_eq!(panel.results_in_range(0, 10), [SimpleSegment::new(3, 1)]);
    }

    #[test]
    fn search_options_publish_valid_snapshots_before_refresh() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("one ONE ones two").unwrap();
        let panel = editor.search_panel();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let subscription = panel.search_options_changed({
            let changes = changes.clone();
            move |panel, options| {
                changes.borrow_mut().push((
                    options.search_pattern.clone(),
                    [options.match_case, options.use_regex, options.whole_words],
                    panel.result_count(),
                ));
            }
        });

        panel.set_search_pattern("one");
        panel.set_match_case(true);
        panel.set_whole_words(true);
        panel.set_use_regex(true);
        panel.set_search_pattern("(");
        assert!(panel.error().is_some());
        assert_eq!(panel.result_count(), 0);
        assert_eq!(
            changes.borrow().as_slice(),
            &[
                ("one".into(), [false, false, false], 0),
                ("one".into(), [true, false, false], 3),
                ("one".into(), [true, false, true], 2),
                ("one".into(), [true, true, true], 1),
            ]
        );

        panel.set_search_pattern("");
        panel.set_search_pattern("");
        panel.set_replacement_pattern("replacement");
        editor.set_text("updated document").unwrap();
        assert_eq!(changes.borrow().len(), 5);
        assert_eq!(changes.borrow()[4], ("".into(), [true, true, true], 0));
        subscription.dispose();
        panel.set_search_pattern("document");
        assert_eq!(panel.result_count(), 1);
        assert_eq!(changes.borrow().len(), 5);
    }

    #[test]
    fn open_search_selects_the_next_match_when_the_query_or_flags_change() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let root = Window::new();
        root.set_content(Some(Rc::new(editor.clone())));
        root.show();
        editor.set_text("one ONE one two").unwrap();
        editor.set_caret_offset(4).unwrap();
        let panel = editor.search_panel();
        panel.set_search_pattern("one");
        assert_eq!(editor.selection(), Selection::empty(4));
        panel.show(false);
        assert_eq!(editor.selection(), Selection::empty(4));

        panel.set_match_case(true);
        assert_eq!(editor.selection(), Selection::simple(8, 11));
        panel.set_search_pattern("two");
        assert_eq!(editor.selection(), Selection::simple(12, 15));
        panel.set_search_pattern("one");
        assert_eq!(editor.selection(), Selection::simple(0, 3));
        assert!(panel.find_next());
        assert_eq!(editor.selection(), Selection::simple(8, 11));

        panel.set_use_regex(true);
        assert_eq!(editor.selection(), Selection::simple(8, 11));
        panel.set_search_pattern("(");
        assert!(panel.error().is_some());
        assert_eq!(editor.selection(), Selection::simple(8, 11));
        panel.set_search_pattern("missing");
        assert_eq!(editor.selection(), Selection::empty(11));
        panel.set_search_pattern("one");
        assert_eq!(editor.selection(), Selection::simple(0, 3));
        panel.set_search_pattern("");
        assert_eq!(editor.selection(), Selection::empty(3));
        assert_eq!(editor.text(), "one ONE one two");
        root.close();
    }

    #[test]
    fn search_docks_at_the_upper_right_and_reveals_the_match_below_the_overlay() {
        use ferroui_base::layout::ILayoutManager;
        use ferroui_base::threading::Dispatcher;
        use ferroui_base::Vector;

        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let root = Window::new();
        root.set_width(640.0);
        root.set_height(480.0);
        root.set_content(Some(Control::boxed(editor.clone())));
        root.show();

        let line = format!("{}needle\n", "x".repeat(34));
        let text = format!("{}{}{}", "filler\n".repeat(28), line, "filler\n".repeat(56));
        let start = text.find("needle").unwrap();
        let end = start + "needle".len();
        editor.set_text(&text).unwrap();
        let area = editor.text_area();
        let panel = editor.search_panel();
        panel.set_search_pattern("needle");
        panel.show(false);
        for _ in 0..3 {
            root.layout_manager().execute_layout_pass();
            Dispatcher::ui_thread().run_jobs(None);
        }

        let surface = panel.surface.get().unwrap();
        let overlay = Rect::from_size(surface.bounds().size())
            .transform_to_aabb(surface.transform_to_visual(&area).unwrap());
        let editor_bounds = Rect::from_size(editor.bounds().size())
            .transform_to_aabb(editor.transform_to_visual(&area).unwrap());
        assert!(
            (overlay.right() - editor_bounds.right()).abs() <= 1.0
                && (overlay.y - editor_bounds.y).abs() <= 1.0,
            "search overlay {overlay:?}, editor {editor_bounds:?}"
        );
        assert!(overlay.height > 8.0);
        area.set_offset(Vector::new(0.0, 28.0 * area.line_height()));
        let result = area.rectangle_for_offset(end).unwrap();
        let overlap = Vector::new(0.0, area.offset().y + result.y - overlay.y - 4.0);
        area.set_offset(overlap);
        let result = area.rectangle_for_offset(end).unwrap();
        assert!(result.intersects(overlay), "caret {result:?}, search overlay {overlay:?}");

        assert!(panel.find_next());
        assert_eq!(editor.selection(), Selection::simple(start, end));
        let caret = area.cursor_rectangle();
        assert!(caret.y >= overlay.bottom(), "caret {caret:?}, search overlay {overlay:?}");

        area.set_offset(overlap);
        assert!(area.cursor_rectangle().intersects(overlay));
        panel.set_search_pattern("needl");
        assert_eq!(editor.selection(), Selection::simple(start, end - 1));
        let caret = area.cursor_rectangle();
        assert!(caret.y >= overlay.bottom(), "caret {caret:?}, search overlay {overlay:?}");
        root.close();
    }

    #[test]
    fn search_options_callbacks_can_replace_the_query_during_notification() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("one two two").unwrap();
        let panel = editor.search_panel();
        let _subscription = panel.search_options_changed(|panel, options| {
            if options.search_pattern == "one" {
                panel.set_search_pattern("two");
            }
        });

        panel.set_search_pattern("one");
        assert_eq!(panel.search_pattern(), "two");
        assert_eq!(panel.result_count(), 2);
    }

    #[test]
    fn find_replace_ui_service_tracks_utf16_and_one_undo() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("😀 one one").unwrap();
        let panel = SearchPanel::install(&editor);
        panel.set_search_pattern("one");
        assert_eq!(panel.result_count(), 2);
        assert!(panel.find_next());
        assert_eq!(editor.selection().primary().start_offset(), 3);
        panel.set_replacement_pattern("two");
        assert_eq!(panel.replace_all().unwrap(), 2);
        assert_eq!(editor.text(), "😀 two two");
        editor.undo().unwrap();
        assert_eq!(editor.text(), "😀 one one");
        panel.set_use_regex(true);
        panel.set_search_pattern("(");
        assert!(panel.error().is_some());
    }

    #[test]
    fn replace_gesture_follows_the_platform_in_the_editor_and_search_popup() {
        use ferroui_base::input::platform::PlatformHotkeyConfiguration;
        use ferroui_base::FerroLocator;

        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let root = Window::new();
        root.set_content(Some(Rc::new(editor.clone())));
        root.show();
        let area = editor.text_area().upcast::<InputElement>();
        let key = |target: &Ref<InputElement>, key, modifiers| {
            let mut event = KeyEventArgs::new();
            event.key = key;
            event.key_modifiers = modifiers;
            event.set_routed_event(Some(InputElement::key_down_event()));
            target.raise_event(&event);
            event.handled()
        };

        let ctrl = KeyModifiers::CONTROL;
        let meta = KeyModifiers::META;
        let alt = KeyModifiers::ALT;
        for (command, replace_key, replace_modifiers, other_key, other_modifiers) in [
            (ctrl, Key::H, ctrl, Key::F, meta | alt),
            (meta, Key::F, meta | alt, Key::H, meta),
        ] {
            FerroLocator::current_mutable()
                .bind_to_self(Rc::new(PlatformHotkeyConfiguration::new(command)));
            assert!(!key(&area, other_key, other_modifiers));
            assert!(key(&area, replace_key, replace_modifiers));
            let panel = editor.search_panel();
            assert!(panel.is_open() && panel.is_replace_mode());

            assert!(key(&area, Key::F, command));
            assert!(panel.is_open() && !panel.is_replace_mode());
            let query = panel.query.get().unwrap().clone().upcast::<InputElement>();
            assert!(!key(&query, other_key, other_modifiers));
            assert!(key(&query, replace_key, replace_modifiers));
            assert!(panel.is_open() && panel.is_replace_mode());
            panel.uninstall();
        }
        root.close();
    }

    #[test]
    fn search_commands_route_from_editor_and_popup_and_survive_reinstall() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let root = Window::new();
        root.set_content(Some(Rc::new(editor.clone())));
        root.show();
        editor.set_text("one one one").unwrap();
        let next = editor.command(EditorCommandKind::FindNext);
        let replace_all = editor.command(EditorCommandKind::ReplaceAll);
        let state_changes = Rc::new(Cell::new(0));
        let _subscription = next.can_execute_changed({
            let state_changes = state_changes.clone();
            Rc::new(move || state_changes.set(state_changes.get() + 1))
        });
        assert!(!next.can_execute(None));
        let key = |target: &Ref<InputElement>, key, modifiers| {
            let mut event = KeyEventArgs::new();
            event.key = key;
            event.key_modifiers = modifiers;
            event.set_routed_event(Some(InputElement::key_down_event()));
            target.raise_event(&event);
            event.handled()
        };
        let area = editor.text_area().upcast::<InputElement>();
        assert!(!key(&area, Key::F3, KeyModifiers::NONE));
        assert!(editor.installed_search_panel().is_none());

        editor.command(EditorCommandKind::Find).execute(None);
        let panel = editor.search_panel();
        assert!(SearchPanel::install(&editor) == panel);
        assert!(next.can_execute(None));
        assert!(!replace_all.can_execute(None));
        assert_eq!(state_changes.get(), 1);
        panel.set_search_pattern("one");
        assert_eq!(editor.selection().primary().start_offset(), 0);
        let query = panel.query.get().unwrap().clone().upcast::<InputElement>();
        assert!(key(&query, Key::F3, KeyModifiers::NONE));
        assert_eq!(editor.selection().primary().start_offset(), 4);
        assert!(key(&area, Key::F3, KeyModifiers::SHIFT));
        assert_eq!(editor.selection().primary().start_offset(), 0);
        assert!(!key(&query, Key::A, KeyModifiers::ALT));

        panel.show(true);
        panel.set_replacement_pattern("two");
        editor.set_is_read_only(true);
        assert!(!replace_all.can_execute(None));
        editor.set_is_read_only(false);
        assert!(replace_all.can_execute(None));
        assert!(key(&query, Key::A, KeyModifiers::ALT));
        assert_eq!(editor.text(), "two two two");
        assert!(key(&area, Key::Escape, KeyModifiers::NONE));
        assert!(!next.can_execute(None));
        assert_eq!(state_changes.get(), 2);

        panel.uninstall();
        assert!(editor.installed_search_panel().is_none());
        editor.command(EditorCommandKind::Find).execute(None);
        let reinstalled = editor.search_panel();
        assert!(panel != reinstalled);
        assert!(next.can_execute(None));
        assert_eq!(state_changes.get(), 3);
        root.close();
    }

    #[test]
    fn replace_next_changes_only_the_primary_match_and_keeps_options_quiet() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("😀 one one").unwrap();
        let panel = editor.search_panel();
        panel.set_search_pattern("one");
        panel.set_replacement_pattern("two");
        let _options = panel.search_options_changed(|_, _| panic!("search options are unchanged"));
        editor.session().set_selection(Selection::from_segments([
            SelectionSegment::new(3, 6),
            SelectionSegment::new(7, 10),
        ], 0)).unwrap();
        assert!(panel.replace_next().unwrap());
        assert_eq!(editor.text(), "😀 two one");
        assert_eq!(editor.selected_text().unwrap(), "one");
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), "😀 one one");
    }

    #[test]
    fn replace_next_edits_the_next_match_without_a_prior_selection() {
        let _scope = crate::tests::test_scope();
        for (caret, edited, next) in [
            (0, "😀 two / one", (9, 12)),
            (7, "😀 one / two", (3, 6)),
            (12, "😀 two / one", (9, 12)),
        ] {
            let editor = TextEditor::new();
            editor.set_text("😀 one / one").unwrap();
            editor.document().clear_undo_history().unwrap();
            let panel = editor.search_panel();
            panel.set_search_pattern("one");
            panel.set_replacement_pattern("two");
            editor.set_caret_offset(caret).unwrap();

            assert!(panel.replace_next().unwrap(), "caret at {caret}");
            assert_eq!(editor.text(), edited);
            assert_eq!(editor.selection(), Selection::simple(next.0, next.1));
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), "😀 one / one");
            assert!(!editor.document().can_undo());
        }
    }

    #[test]
    fn replace_next_keeps_protected_text_and_leaves_the_caret_after_the_last_replacement() {
        use ferroui_edit_core::editing::TextSegmentReadOnlySectionProvider;

        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("😀 one").unwrap();
        let document = editor.document();
        document.clear_undo_history().unwrap();
        let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        policy.add(4, 1).unwrap();
        editor.session().set_read_only_section_provider(policy);
        editor.set_caret_offset(0).unwrap();
        let panel = editor.search_panel();
        panel.set_search_pattern("one");
        panel.set_replacement_pattern("🧭");
        assert!(panel.replace_next().is_err());
        assert_eq!(editor.text(), "😀 one");
        assert_eq!(editor.selection(), Selection::empty(0));
        assert!(!document.can_undo());

        editor.session().set_read_only_section_provider(Rc::new(
            TextSegmentReadOnlySectionProvider::new(document.clone()),
        ));
        assert!(panel.replace_next().unwrap());
        assert_eq!(editor.text(), "😀 🧭");
        assert_eq!(editor.selection(), Selection::empty(5));
        editor.select(0, 2).unwrap();
        assert!(!panel.replace_next().unwrap());
        assert_eq!(editor.selection(), Selection::simple(0, 2));
        assert_eq!(editor.text(), "😀 🧭");
    }

    #[test]
    fn a_closed_panel_searches_when_it_is_used_and_not_on_each_edit() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("a a").unwrap();
        let panel = editor.search_panel();
        panel.set_search_pattern("a");
        assert!(!panel.is_open());
        assert_eq!(panel.result_count(), 2);

        editor.document().insert(0, "a ").unwrap();
        assert!(panel.results_stale.get());
        assert!(panel.results_in_range(0, 5).is_empty());
        assert_eq!(panel.result_count(), 3);
        assert!(!panel.results_stale.get());
        assert_eq!(panel.results_in_range(0, 5).len(), 3);
    }

    #[test]
    fn replace_next_advances_past_a_zero_width_match_after_insertion() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("ab").unwrap();
        editor.set_caret_offset(0).unwrap();
        let panel = editor.search_panel();
        panel.set_use_regex(true);
        panel.set_search_pattern("(?=.)");
        panel.set_replacement_pattern("x");
        assert!(panel.replace_next().unwrap());
        assert_eq!(editor.text(), "xab");
        assert_eq!(editor.caret_offset(), 2);
    }

    #[test]
    fn replacement_rejects_a_document_swap_during_update_start() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("one one").unwrap();
        let previous = editor.document();
        let panel = editor.search_panel();
        panel.set_search_pattern("one");
        panel.set_replacement_pattern("two");
        let weak = editor.downgrade();
        let _change = previous.update_started(move || {
            if let Some(editor) = weak.upgrade() {
                editor.set_document(TextDocument::from_text("one replacement"));
            }
        });
        assert!(panel.replace_all().is_err());
        assert_eq!(previous.text(), "one one");
        assert_eq!(editor.text(), "one replacement");
        assert_eq!(panel.result_count(), 1);
    }

    #[test]
    fn regex_replacement_uses_source_numbers_and_rejects_an_overflowing_template() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("😀 abba").unwrap();
        editor.document().clear_undo_history().unwrap();
        let panel = editor.search_panel();
        panel.set_use_regex(true);
        panel.set_search_pattern(r"(?<word>a)(b)\1\2");
        assert!(panel.find_next());
        assert_eq!(editor.selection().primary().start_offset(), 3);
        panel.set_replacement_pattern("$2147483648");
        assert!(panel.replace_next().unwrap_err().contains("Capture group number"));
        assert!(panel.replace_all().is_err());
        assert_eq!(editor.text(), "😀 abba");
        assert!(!editor.document().can_undo());
        panel.set_replacement_pattern("$1/$2/${word}/$+");
        assert_eq!(panel.replace_all().unwrap(), 1);
        assert_eq!(editor.text(), "😀 b/a/a/a");
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), "😀 abba");
    }
    #[test]
    fn find_previous_searches_before_the_primary_range_and_wraps() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let root = Window::new();
        root.set_content(Some(Control::boxed(editor.clone())));
        root.show();
        editor.set_text("😀 one / one / one").unwrap();
        let panel = editor.search_panel();
        panel.set_search_pattern("one");
        panel.show(false);
        let area = editor.text_area();

        for (anchor, caret, expected) in [
            (7, 18, 3), (0, 13, 15), (9, 12, 3), (16, 16, 15), (18, 7, 3),
        ] {
            editor.set_selection(Selection::simple(anchor, caret)).unwrap();
            let mut event = KeyEventArgs::new();
            event.key = Key::F3;
            event.key_modifiers = KeyModifiers::SHIFT;
            event.set_routed_event(Some(InputElement::key_down_event()));
            area.raise_event(&event);
            assert!(event.handled());
            assert_eq!(editor.selection(), Selection::simple(expected, expected + 3),
                "selection from {anchor} to {caret}");
        }
        root.close();
    }

    #[test]
    fn zero_width_navigation_does_not_skip_first_result() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("ab").unwrap();
        editor.set_caret_offset(0).unwrap();
        let panel = SearchPanel::install(&editor);
        panel.set_use_regex(true);
        panel.set_search_pattern("(?=.)");
        assert!(panel.find_next());
        assert_eq!(editor.caret_offset(), 0);
        assert!(panel.find_next());
        assert_eq!(editor.caret_offset(), 1);
        assert_eq!(panel.results_in_range(0, 0), vec![SimpleSegment::new(0, 0)]);
    }

    #[test]
    fn find_next_after_a_manual_caret_move_includes_a_zero_width_match_at_that_caret() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("abc").unwrap();
        editor.set_caret_offset(0).unwrap();
        let panel = editor.search_panel();
        panel.set_use_regex(true);
        panel.set_search_pattern("(?=.)");
        assert!(panel.find_next());
        assert_eq!(editor.caret_offset(), 0);

        editor.set_caret_offset(1).unwrap();
        assert_eq!(panel.current.get(), None);
        assert!(panel.find_next());
        assert_eq!(editor.caret_offset(), 1);
        assert!(panel.find_next());
        assert_eq!(editor.caret_offset(), 2);
        assert!(panel.find_next());
        assert_eq!(editor.caret_offset(), 0);

        editor.set_caret_offset(1).unwrap();
        editor.set_caret_offset(0).unwrap();
        assert!(panel.find_next());
        assert_eq!(editor.caret_offset(), 0);
    }
}
