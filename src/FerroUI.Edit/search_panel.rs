use crate::TextEditor;
use ferroui_base::input::{InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, Thickness, VisualImpl, WeakRef, ferro_class,
    ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::popup_positioning::{PopupAnchor, PopupGravity};
use ferroui_controls::{
    Border, Button, CheckBox, ControlImpl, PlacementMode, StackPanel, TextBlock, TextBox,
};
use ferroui_edit_core::document::{DocumentEdit, SimpleSegment};
use ferroui_edit_core::search::{SearchMode, SearchResult, SearchStrategyFactory};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

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
    results: RefCell<Vec<SearchResult>>,
    current: Cell<Option<usize>>,
    error: RefCell<Option<String>>,
    synchronizing: Cell<bool>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
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
            this.refresh();
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
            this.refresh();
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
            results: RefCell::new(Vec::new()),
            current: Cell::new(None),
            error: RefCell::new(None),
            synchronizing: Cell::new(false),
            subscriptions: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn install(editor: &Ref<TextEditor>) -> Ref<Self> {
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
                    panel.refresh();
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
    pub fn result_count(&self) -> usize {
        self.results.borrow().len()
    }
    /// Snapshot match ranges intersecting a visible document interval. Rendering
    /// hosts can highlight results without copying all captured match text.
    pub fn results_in_range(&self, offset: usize, length: usize) -> Vec<SimpleSegment> {
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
    pub fn show(&self, replace: bool) {
        if let Some(row) = self.replacement_row.get() {
            row.set_is_visible(replace);
        }
        if let Some(editor) = self.editor() {
            self.set_placement_rect(Some(ferroui_base::Rect::new(
                0.0,
                0.0,
                editor.bounds().width,
                0.0,
            )));
            if let Ok(selected) = editor.selected_text() {
                if !selected.is_empty() && !selected.contains(['\n', '\r']) {
                    self.set_search_pattern(&selected);
                }
            }
        }
        self.update_surface();
        self.refresh();
        self.open();
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
        self.close();
        for subscription in self.subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        self.editor.borrow_mut().take();
        self.set_popup_parent(None);
    }
    pub fn refresh(&self) {
        let Some(editor) = self.editor() else {
            return;
        };
        let pattern = self.search_pattern();
        if pattern.is_empty() {
            self.results.borrow_mut().clear();
            self.current.set(None);
            *self.error.borrow_mut() = None;
            self.update_status();
            editor.text_area().invalidate_visual();
            return;
        }
        let mode = if self.use_regex() {
            SearchMode::RegEx
        } else {
            SearchMode::Normal
        };
        let result =
            SearchStrategyFactory::create(&pattern, !self.match_case(), self.whole_words(), mode)
                .and_then(|strategy| {
                    let text = editor.text();
                    strategy.find_all(&text, 0, editor.document().text_length())
                });
        match result {
            Ok(results) => {
                *self.results.borrow_mut() = results;
                *self.error.borrow_mut() = None;
            }
            Err(error) => {
                self.results.borrow_mut().clear();
                *self.error.borrow_mut() = Some(error.to_string());
            }
        }
        self.current.set(None);
        self.update_status();
        editor.text_area().invalidate_visual();
    }
    fn update_status(&self) {
        let text = if let Some(error) = self.error() {
            error
        } else if let Some(current) = self.current.get() {
            format!("{} of {}", current + 1, self.result_count())
        } else {
            format!("{} matches", self.result_count())
        };
        if let Some(status) = self.status.get() {
            status.set_text(Some(&text));
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
        let results = self.results.borrow();
        if results.is_empty() {
            return false;
        }
        let selected = editor.selection().primary();
        let exact = results.iter().position(|r| {
            r.offset == selected.start_offset()
                && r.length == selected.length()
                && (!selected.is_empty() || self.current.get().is_some())
        });
        let index = if let Some(current) = exact {
            if backwards {
                (current + results.len() - 1) % results.len()
            } else {
                (current + 1) % results.len()
            }
        } else if backwards {
            results
                .iter()
                .rposition(|r| r.offset < editor.caret_offset())
                .unwrap_or(results.len() - 1)
        } else {
            results
                .iter()
                .position(|r| r.offset >= editor.caret_offset())
                .unwrap_or(0)
        };
        let (offset, length) = (results[index].offset, results[index].length);
        drop(results);
        if editor.select(offset, length).is_err() {
            return false;
        }
        self.current.set(Some(index));
        self.update_status();
        true
    }
    pub fn replace_next(&self) -> Result<bool, String> {
        let editor = self
            .editor()
            .ok_or_else(|| "Search panel is not installed".to_owned())?;
        self.refresh();
        let selected = editor.selection().primary();
        let result = self
            .results
            .borrow()
            .iter()
            .find(|r| r.offset == selected.start_offset() && r.length == selected.length())
            .cloned();
        let Some(result) = result else {
            self.find_next();
            return Ok(false);
        };
        let replacement = if self.use_regex() {
            result.replace_with(&self.replacement_pattern())
        } else {
            self.replacement_pattern()
        };
        editor
            .replace_selection_with_text(&replacement)
            .map_err(|e| e.to_string())?;
        self.refresh();
        self.find_next();
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
                DocumentEdit::new(
                    r.offset,
                    r.length,
                    if regex {
                        r.replace_with(&replacement)
                    } else {
                        replacement.clone()
                    },
                )
            })
            .collect();
        editor
            .session()
            .apply_document_edits(&edits)
            .map_err(|e| e.to_string())?;
        editor.selection_changed();
        self.refresh();
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
        query.set_placeholder_text(Some("Find"));
        query_row
            .children()
            .add(query.clone().upcast::<ferroui_controls::Control>());
        let previous = self.button("Previous", |p| {
            p.find_previous();
        });
        let next = self.button("Next", |p| {
            p.find_next();
        });
        let close = self.button("Close", |p| p.close_panel());
        query_row
            .children()
            .add(previous.upcast::<ferroui_controls::Control>());
        query_row
            .children()
            .add(next.upcast::<ferroui_controls::Control>());
        query_row
            .children()
            .add(close.upcast::<ferroui_controls::Control>());
        content
            .children()
            .add(query_row.upcast::<ferroui_controls::Control>());
        let flags = StackPanel::new();
        flags.set_orientation(Orientation::Horizontal);
        flags.set_spacing(10.0);
        let mut checkboxes = Vec::new();
        for (label, kind) in [("Match case", 0), ("Whole words", 1), ("Regex", 2)] {
            let checkbox = CheckBox::new();
            checkbox.set_content(Some(Rc::new(label.to_owned())));
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
        let replacement = TextBox::new();
        replacement.set_width(260.0);
        replacement.set_placeholder_text(Some("Replace"));
        row.children()
            .add(replacement.clone().upcast::<ferroui_controls::Control>());
        row.children().add(
            self.button("Replace", |p| {
                if let Err(error) = p.replace_next() {
                    *p.error.borrow_mut() = Some(error);
                    p.update_status();
                }
            })
            .upcast::<ferroui_controls::Control>(),
        );
        row.children().add(
            self.button("Replace all", |p| {
                if let Err(error) = p.replace_all() {
                    *p.error.borrow_mut() = Some(error);
                    p.update_status();
                }
            })
            .upcast::<ferroui_controls::Control>(),
        );
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
                    if let Some(panel) = weak.upgrade() {
                        match e.key {
                            Key::Escape => {
                                panel.close_panel();
                                e.set_handled(true);
                            }
                            Key::Return => {
                                if e.key_modifiers.contains(KeyModifiers::SHIFT) {
                                    panel.find_previous();
                                } else {
                                    panel.find_next();
                                }
                                e.set_handled(true);
                            }
                            _ => {}
                        }
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            ));
    }
    fn button(&self, label: &str, action: impl Fn(&SearchPanel) + 'static) -> Ref<Button> {
        let button = Button::new();
        button.set_content(Some(Rc::new(label.to_owned())));
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(
                button.add_disposable_handler(Button::click_event(), move |_, _| {
                    if let Some(panel) = weak.upgrade() {
                        action(&panel);
                    }
                }),
            );
        button
    }
}
impl Drop for SearchPanel {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
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
}
