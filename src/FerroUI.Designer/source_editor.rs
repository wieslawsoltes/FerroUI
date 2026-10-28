//! Native text interaction backed by the shared XAML language service.
use super::*;
use ferroui_designer_core::{Edit, SourceSpan};
use ferroui_designer_language::{
    FormattingOptions, LanguageService, LineIndex, Location, Schema, TextEdit, WorkspaceEdit,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct SourceTools {
    pub service: LanguageService,
    versions: BTreeMap<String, i64>,
    diagnostics: bool,
}
impl Default for SourceTools {
    fn default() -> Self {
        Self {
            service: LanguageService::new(Schema::ferroui()),
            versions: BTreeMap::new(),
            diagnostics: true,
        }
    }
}

pub(crate) fn byte_from_utf16(source: &str, index: i32) -> usize {
    let mut count = 0;
    let target = index.max(0) as usize;
    for (byte, ch) in source.char_indices() {
        if count + ch.len_utf16() > target {
            return byte;
        }
        count += ch.len_utf16();
    }
    source.len()
}
pub(crate) fn utf16_from_byte(source: &str, byte: usize) -> i32 {
    let mut byte = byte.min(source.len());
    while !source.is_char_boundary(byte) {
        byte -= 1;
    }
    source[..byte].encode_utf16().count() as i32
}
impl DesignerWorkspace {
    pub fn language_service(&self) -> std::cell::RefMut<'_, LanguageService> {
        std::cell::RefMut::map(self.source_tools.borrow_mut(), |tools| &mut tools.service)
    }
    pub fn try_language_service(&self) -> Result<std::cell::RefMut<'_, LanguageService>, String> {
        let tools = self
            .source_tools
            .try_borrow_mut()
            .map_err(|_| "The language service is busy")?;
        Ok(std::cell::RefMut::map(tools, |tools| &mut tools.service))
    }
    pub fn synchronize_language_service(self: &Rc<Self>) -> Result<(), String> {
        drop(self.try_language_service()?);
        drop(
            self.session
                .try_borrow()
                .map_err(|_| "The designer session is busy")?,
        );
        self.refresh_source_tools();
        self.refresh_components();
        Ok(())
    }
    pub(super) fn connect_source_tools(self: &Rc<Self>) {
        for name in [
            "CompleteSource",
            "SourceDefinition",
            "SourceReferences",
            "SourceDiagnostics",
            "FormatSource",
            "RenameSource",
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<Button>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let result = this.source_command(name);
                    this.report(result);
                }
            });
        }
        let weak = Rc::downgrade(self);
        self.source
            .add_handler(InputElement::key_down_event(), move |_, event| {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                let command = match (
                    event.key,
                    event
                        .key_modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::META),
                    event.key_modifiers.contains(KeyModifiers::SHIFT),
                ) {
                    (Key::Space, true, _) => Some("CompleteSource"),
                    (Key::F12, _, false) => Some("SourceDefinition"),
                    (Key::F12, _, true) => Some("SourceReferences"),
                    (Key::F, true, true) => Some("FormatSource"),
                    _ => None,
                };
                if let Some(command) = command {
                    let result = this.source_command(command);
                    this.report(result);
                    event.set_handled(true);
                }
            });
    }
    pub(super) fn refresh_source_tools(self: &Rc<Self>) {
        let (uri, source, revision) = {
            let session = self.session.borrow();
            (
                super::project::document_identity(session.document_uri())
                    .unwrap_or_else(|_| session.document_uri().to_owned()),
                session.document().source().to_owned(),
                session.document().revision(),
            )
        };
        let mut tools = self.source_tools.borrow_mut();
        if tools.versions.get(&uri).copied() != Some(revision as i64) {
            let version = tools.service.version(&uri).map_or(revision as i64, |old| {
                (old.saturating_add(1)).max(revision as i64)
            });
            let result = if tools.service.version(&uri).is_some() {
                tools.service.replace(&uri, &source, version)
            } else {
                tools.service.open(&uri, &source, version)
            };
            if result.is_ok() {
                tools.versions.insert(uri.clone(), revision as i64);
            }
        }
        let diagnostics = if tools.diagnostics {
            tools.service.diagnostics(&uri).ok()
        } else {
            None
        };
        drop(tools);
        let offset = byte_from_utf16(&source, self.source.caret_index());
        if let Ok(position) = LineIndex::new(&source).position(offset) {
            self.root
                .get_control::<TextBlock>("SourcePosition")
                .set_text(Some(&format!(
                    "XAML  {}:{}",
                    position.line + 1,
                    position.character + 1
                )));
        }
        if let Some(diagnostics) = diagnostics {
            let panel = self.root.get_control::<StackPanel>("LanguageResults");
            panel.children().clear();
            for diagnostic in diagnostics.into_iter().take(100) {
                let row = button(&format!(
                    "{}:{}  {}",
                    diagnostic.range.start.line + 1,
                    diagnostic.range.start.character + 1,
                    diagnostic.message
                ));
                row.set_horizontal_content_alignment(HorizontalAlignment::Left);
                let span = diagnostic.span;
                let weak = Rc::downgrade(self);
                row.click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.select_source_span(span);
                    }
                });
                panel.children().add(row.upcast::<Control>());
            }
        }
    }
    pub fn select_source_span(&self, span: SourceSpan) {
        if self.view_mode() == super::shell::WorkspaceViewMode::Design {
            self.set_view_mode(super::shell::WorkspaceViewMode::Split);
        }
        let source = self.source.text().unwrap_or_default();
        // Moving the caret collapses TextBox's selection. Position it first, then
        // select the original source span so definition/property navigation highlights it.
        self.source
            .set_caret_index(utf16_from_byte(&source, span.end));
        self.source
            .set_selection_start(utf16_from_byte(&source, span.start));
        self.source
            .set_selection_end(utf16_from_byte(&source, span.end));
        self.source.focus();
    }
    fn source_command(self: &Rc<Self>, command: &str) -> Result<(), String> {
        self.refresh_source_tools();
        let (uri, source, revision) = {
            let session = self.session.borrow();
            (
                super::project::document_identity(session.document_uri())?,
                session.document().source().to_owned(),
                session.document().revision(),
            )
        };
        let offset = byte_from_utf16(&source, self.source.caret_index());
        let panel = self.root.get_control::<StackPanel>("LanguageResults");
        match command {
            "CompleteSource" => {
                let completions = self
                    .source_tools
                    .borrow()
                    .service
                    .completions(&uri, offset)
                    .map_err(|e| e.to_string())?;
                self.source_tools.borrow_mut().diagnostics = false;
                panel.children().clear();
                for item in completions.into_iter().take(100) {
                    let row = button(&format!("{}  {}", item.label, item.detail));
                    row.set_horizontal_content_alignment(HorizontalAlignment::Left);
                    let weak = Rc::downgrade(self);
                    row.click(move |_, _| {
                        if let Some(this) = weak.upgrade() {
                            let end = item.span.start + item.insert_text.len();
                            let result = this.apply_source_completion(
                                revision,
                                item.span,
                                &item.insert_text,
                            );
                            if result.is_ok() {
                                this.pending_source.set(false);
                                this.source_tools.borrow_mut().diagnostics = true;
                                this.refresh(true);
                                this.select_source_span(SourceSpan::new(end, end));
                            }
                            this.report(result);
                        }
                    });
                    panel.children().add(row.upcast::<Control>());
                }
            }
            "SourceDefinition" | "SourceReferences" => {
                let locations = {
                    let tools = self.source_tools.borrow();
                    if command == "SourceDefinition" {
                        tools.service.definitions(&uri, offset)
                    } else {
                        tools.service.references(&uri, offset)
                    }
                }
                .map_err(|e| e.to_string())?;
                self.source_tools.borrow_mut().diagnostics = false;
                panel.children().clear();
                for location in locations {
                    let row = button(&format!(
                        "{}  {}:{}",
                        location.uri.rsplit('/').next().unwrap_or(&location.uri),
                        location.range.start.line + 1,
                        location.range.start.character + 1
                    ));
                    let weak = Rc::downgrade(self);
                    row.click(move |_, _| {
                        if let Some(this) = weak.upgrade() {
                            let result = this.navigate_to(&location);
                            this.report(result);
                        }
                    });
                    panel.children().add(row.upcast::<Control>());
                }
            }
            "SourceDiagnostics" => {
                self.source_tools.borrow_mut().diagnostics = true;
                self.refresh_source_tools();
            }
            "FormatSource" => {
                self.session
                    .borrow()
                    .ensure_writable()
                    .map_err(|error| error.to_string())?;
                let edits = self
                    .source_tools
                    .borrow()
                    .service
                    .format(&uri, FormattingOptions::default())
                    .map_err(|e| e.to_string())?;
                let source = apply_text_edits(&source, &edits)?;
                {
                    let mut session = self.session.borrow_mut();
                    session.stage_source(source);
                    session.apply_staged_preview()?;
                }
                self.pending_source.set(false);
                self.refresh(true);
            }
            "RenameSource" => {
                let name = self
                    .root
                    .get_control::<TextBox>("RenameSymbol")
                    .text()
                    .unwrap_or_default();
                let edits = self
                    .source_tools
                    .borrow()
                    .service
                    .rename(&uri, offset, &name)
                    .map_err(|e| e.to_string())?;
                self.apply_language_edits(edits)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn navigate_to(self: &Rc<Self>, location: &Location) -> Result<(), String> {
        self.activate_document(&location.uri)?;
        let source = self.session.borrow().document().source().to_owned();
        let span = LineIndex::new(&source)
            .span(location.range)
            .map_err(|e| e.to_string())?;
        self.select_source_span(span);
        Ok(())
    }
    /// Apply a versioned completion even while its surrounding XAML is an incomplete draft.
    pub fn apply_source_completion(
        &self,
        expected_revision: u64,
        span: SourceSpan,
        text: &str,
    ) -> Result<(), String> {
        let mut session = self.session.borrow_mut();
        session
            .ensure_writable()
            .map_err(|error| error.to_string())?;
        if session.document().revision() != expected_revision {
            return Err("Source changed since the completion was prepared".into());
        }
        let mut source = session.document().source().to_owned();
        if span.start > span.end
            || span.end > source.len()
            || !source.is_char_boundary(span.start)
            || !source.is_char_boundary(span.end)
        {
            return Err("The completion does not match the current source text".into());
        }
        source.replace_range(span.start..span.end, text);
        session.stage_source(source);
        session.apply_staged_preview()
    }
    pub fn apply_language_edits(self: &Rc<Self>, edit: WorkspaceEdit) -> Result<(), String> {
        self.session
            .borrow()
            .ensure_writable()
            .map_err(|error| error.to_string())?;
        let mut changes = edit.document_changes;
        let mut documents = BTreeSet::new();
        for change in &mut changes {
            change.uri = super::project::document_identity(&change.uri)?;
            if !documents.insert(change.uri.clone()) {
                return Err("A workspace edit contains duplicate document identities".into());
            }
        }
        let (active, expected_revision, active_document) = {
            let session = self.session.borrow();
            (
                super::project::document_identity(session.document_uri())?,
                session.document().revision(),
                session.document().clone(),
            )
        };
        let mut candidates = Vec::new();
        let mut language = self.source_tools.borrow().service.clone();
        let (mut tabs, mut project) = {
            let state = self.project.borrow();
            (state.tabs.clone(), state.workspace.clone())
        };
        tabs.insert(
            active.clone(),
            super::project::Tab {
                preview_uri: self.session.borrow().document_uri().into(),
                document: active_document,
                saved: self.saved_source.borrow().clone(),
                file: self.file.borrow().clone(),
                layers: Some(self.session.borrow().layer_view_snapshot()),
                viewport: self.document_canvas_viewport(),
                guides: Some(self.canvas_guide_view_snapshot()),
            },
        );
        for change in changes {
            if language.version(&change.uri) != Some(change.version) {
                return Err(format!(
                    "{} changed since the refactoring was prepared",
                    change.uri
                ));
            }
            if !tabs.contains_key(&change.uri) {
                let workspace = project.as_mut().ok_or(
                    "Open the document's solution before applying a workspace refactoring",
                )?;
                let buffer = workspace.open(&change.uri).map_err(|e| e.to_string())?;
                let tab = super::project::Tab {
                    preview_uri: change.uri.clone(),
                    document: buffer.document().clone(),
                    saved: buffer.saved_source().unwrap_or("").into(),
                    file: None,
                    layers: None,
                    viewport: None,
                    guides: None,
                };
                tabs.insert(change.uri.clone(), tab);
            }
            let tab = &tabs[&change.uri];
            if language
                .document(&change.uri)
                .is_none_or(|indexed| indexed.source() != tab.document.source())
            {
                return Err(format!(
                    "{} changed since it was indexed; request the refactoring again",
                    change.uri
                ));
            }
            let source = apply_text_edits(tab.document.source(), &change.edits)?;
            let mut document = tab.document.clone();
            document.update_source("Refactor XAML", source.clone());
            let version = change
                .version
                .checked_add(1)
                .ok_or("Language document version exhausted")?
                .max(document.revision() as i64);
            language
                .replace(&change.uri, &source, version)
                .map_err(|e| e.to_string())?;
            candidates.push((change.uri, document, source, version));
        }
        for (uri, document, _, _) in &candidates {
            let document = document.clone();
            tabs.get_mut(uri).unwrap().document = document.clone();
            if let Some(workspace) = project.as_mut() {
                if let Some(buffer) = workspace.buffer(uri) {
                    let version = buffer.version();
                    workspace
                        .store_document(uri, version, document)
                        .map_err(|error| error.to_string())?;
                }
            }
        }
        if let Some(project) = project.as_mut() {
            if project.buffer(&active).is_some() {
                project
                    .activate(&active)
                    .map_err(|error| error.to_string())?;
            }
        }
        let active_changed = candidates.iter().any(|(uri, _, _, _)| uri == &active);
        if let Some((_, _, source, _)) = candidates.iter().find(|(uri, _, _, _)| uri == &active) {
            self.session
                .borrow_mut()
                .apply_at(
                    expected_revision,
                    "Refactor XAML",
                    vec![Edit::ReplaceSource {
                        source: source.clone(),
                    }],
                )
                .map_err(|error| error.to_string())?;
            tabs.get_mut(&active).unwrap().document = self.session.borrow().document().clone();
        } else if self.session.borrow().document().revision() != expected_revision {
            return Err("The active document changed while the refactoring was prepared".into());
        }
        {
            let mut state = self.project.borrow_mut();
            state.tabs = tabs;
            state.workspace = project;
        }
        let mut tools = self.source_tools.borrow_mut();
        tools.service = language;
        for (uri, document, _, _) in candidates {
            let revision = if uri == active {
                self.session.borrow().document().revision()
            } else {
                document.revision()
            };
            tools.versions.insert(uri, revision as i64);
        }
        drop(tools);
        if active_changed {
            self.pending_source.set(false);
        }
        self.refresh(true);
        self.rebuild_document_tabs();
        Ok(())
    }
    pub(super) fn close_language_document(&self, uri: &str) -> Result<(), String> {
        let uri = super::project::document_identity(uri)?;
        let uri = uri.as_str();
        let mut tools = self.source_tools.borrow_mut();
        if tools.service.document(uri).is_some() {
            tools
                .service
                .close(uri)
                .map_err(|error| error.to_string())?;
        }
        tools.versions.remove(uri);
        Ok(())
    }
}
fn apply_text_edits(source: &str, edits: &[TextEdit]) -> Result<String, String> {
    ferroui_designer_language::apply_text_edits(source, edits).map_err(|error| error.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_utf16_offsets_preserve_supplementary_characters() {
        let s = "a😀ż";
        assert_eq!(byte_from_utf16(s, 1), 1);
        assert_eq!(byte_from_utf16(s, 2), 1);
        assert_eq!(byte_from_utf16(s, 3), 5);
        assert_eq!(utf16_from_byte(s, 5), 3);
    }
}
