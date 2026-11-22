use crate::{ICompletionData, SnippetController, TextEditor, TextMateInstallation};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::BoxedValue;
use ferroui_edit_core::document::{DocumentError, SimpleSegment, TextSnapshot, TextUtilities};
use ferroui_edit_core::editing::Selection;
use ferroui_edit_core::snippets::{SnippetError, TextMateSnippet};
use ferroui_textmate::{Language, LanguageConfiguration};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

/// Tracked replacement and native completion items for the language at the
/// caret. The host can append its other completions before opening the popup.
pub struct PackageSnippetSuggestions {
    pub range: SimpleSegment,
    pub items: Vec<Rc<dyn ICompletionData>>,
}

/// Resolves a snippet variable from host data, such as workspace paths,
/// a clock, or clipboard text already obtained by the host.
pub type SnippetVariableProvider = dyn Fn(&str) -> Result<Option<String>, SnippetError>;

#[derive(Default)]
pub(crate) struct PackageSnippets {
    languages: BTreeMap<u8, Vec<Rc<PackageSnippet>>>,
}

struct PackageSnippet {
    name: String,
    prefixes: Vec<String>,
    description: String,
    body: TextMateSnippet,
}

impl TextMateInstallation {
    /// Supplies contextual values for named and completion-based expansion.
    /// A host value takes precedence over document metadata. The selection
    /// variables always use the editor selection.
    pub fn set_snippet_variable_provider(&self, provider: Option<Rc<SnippetVariableProvider>>) {
        self.snippet_variables.replace(provider);
    }

    /// Compiles snippet definitions for the numeric language IDs used by this
    /// installation. It uses each package's language scope and prefix aliases.
    /// A parse error includes the definition name and leaves the prior set intact.
    pub fn set_package_snippets<'a>(
        &self,
        languages: impl IntoIterator<Item = (u8, &'a Language)>,
    ) -> Result<(), SnippetError> {
        let mut result = PackageSnippets::default();
        for (id, language) in languages {
            let definitions = language
                .typed_snippets()
                .map_err(|error| SnippetError::Extension(error.to_string()))?;
            let mut entries = Vec::new();
            for (name, definition) in definitions.snippets {
                if definition.body.is_empty()
                    || (!definition.scopes.is_empty() && !definition.scopes.contains(&language.id))
                {
                    continue;
                }
                let body =
                    TextMateSnippet::parse(&definition.body.join("\n")).map_err(|error| {
                        SnippetError::Extension(format!(
                            "{} snippet {name:?}: {error}",
                            language.id
                        ))
                    })?;
                entries.push(Rc::new(PackageSnippet {
                    description: definition.description.unwrap_or_else(|| name.clone()),
                    name,
                    prefixes: definition.prefixes,
                    body,
                }));
            }
            if result.languages.insert(id, entries).is_some() {
                return Err(SnippetError::Extension(format!(
                    "duplicate snippet language ID {id}"
                )));
            }
        }
        self.package_snippets.replace(Rc::new(result));
        Ok(())
    }
}

impl TextEditor {
    /// Gets package snippet suggestions at the current caret. A host can use
    /// `CompletionWindow::show_with_data` to accept one through ordinary input.
    pub fn package_snippet_suggestions(
        &self,
    ) -> Result<Option<PackageSnippetSuggestions>, DocumentError> {
        let selection = self.selection();
        if !physical(&selection) || !selection.is_empty() {
            return Ok(None);
        }
        let caret = selection.primary().caret;
        let Some((installation, scope)) = language_at(self, caret)? else {
            return Ok(None);
        };
        let packages = installation.package_snippets.borrow().clone();
        let Some(entries) = packages
            .languages
            .get(&scope.id)
            .filter(|entries| !entries.is_empty())
        else {
            return Ok(None);
        };
        let document = self.document();
        let line = document.get_line_by_offset(caret)?;
        let before =
            document.get_text(line.offset + scope.start, caret - line.offset - scope.start)?;
        let entries_and_prefixes = || {
            entries.iter().flat_map(|entry| {
                entry
                    .prefixes
                    .iter()
                    .filter(|prefix| !prefix.is_empty())
                    .map(move |prefix| (entry, prefix))
            })
        };
        let start = prefix_start(
            &before,
            entries_and_prefixes().map(|(_, prefix)| prefix.as_str()),
        );
        let length = before[start..].encode_utf16().count();
        let items = entries_and_prefixes()
            .map(|(entry, prefix)| {
                Rc::new(PackageSnippetCompletion {
                    prefix: prefix.clone(),
                    snippet: entry.clone(),
                    installation: Rc::downgrade(&installation),
                    packages: Rc::downgrade(&packages),
                    language: scope.id,
                }) as Rc<dyn ICompletionData>
            })
            .collect::<Vec<_>>();
        Ok((!items.is_empty()).then_some(PackageSnippetSuggestions {
            range: SimpleSegment::new(caret - length, length),
            items,
        }))
    }

    /// Inserts a named package snippet in the language at the start of the
    /// selection. Its selected-text variable uses the actual editor selection.
    pub fn insert_package_snippet(
        &self,
        name: &str,
    ) -> Result<Option<Rc<SnippetController>>, SnippetError> {
        let selection = self.selection();
        if selection.is_rectangular() || selection.segments().len() != 1 {
            return Err(SnippetError::MultipleSelections);
        }
        let range = selection.primary().segment();
        let Some((installation, scope)) = language_at(self, range.offset)? else {
            return Ok(None);
        };
        let packages = installation.package_snippets.borrow().clone();
        let Some(entry) = packages
            .languages
            .get(&scope.id)
            .and_then(|entries| entries.iter().find(|entry| entry.name == name))
        else {
            return Ok(None);
        };
        entry
            .accept(self, range, scope.id, &installation, &packages, true)
            .map(Some)
    }
}

fn physical(selection: &Selection) -> bool {
    !selection.is_rectangular()
        && selection.segments().len() == 1
        && selection.primary().start_virtual_space() == 0
        && selection.primary().end_virtual_space() == 0
}

fn language_at(
    editor: &TextEditor,
    offset: usize,
) -> Result<
    Option<(
        Rc<TextMateInstallation>,
        crate::input_languages::LanguageScope,
    )>,
    DocumentError,
> {
    let Some(installation) = editor.textmate_installation() else {
        return Ok(None);
    };
    let input = installation.input_languages();
    let document = editor.document();
    let line = document.get_line_by_offset(offset)?;
    let column = offset - line.offset;
    if column > line.length {
        return Ok(None);
    }
    if !input.grammar.embedded_languages.is_empty()
        && !installation.advance_to(&document, line.line_number, 128)
    {
        return Ok(None);
    }
    let scope = installation.input_scope(&document, &input, line.line_number, column, line.length);
    Ok(scope.map(|scope| (installation, scope)))
}

fn prefix_start<'a>(before: &str, prefixes: impl Iterator<Item = &'a str>) -> usize {
    let word = |ch: char| ch.is_alphanumeric() || ch == '_';
    let mut start = before.trim_end_matches(word).len();
    for prefix in prefixes {
        let limit = prefix.chars().count();
        for (index, _) in before.char_indices().rev().take(limit) {
            if index < start
                && !before[..index].ends_with(word)
                && prefix
                    .to_lowercase()
                    .starts_with(&before[index..].to_lowercase())
            {
                start = index;
            }
        }
    }
    start
}

struct PackageSnippetCompletion {
    prefix: String,
    snippet: Rc<PackageSnippet>,
    installation: Weak<TextMateInstallation>,
    packages: Weak<PackageSnippets>,
    language: u8,
}

impl ICompletionData for PackageSnippetCompletion {
    fn text(&self) -> &str {
        &self.prefix
    }
    fn description(&self) -> Option<BoxedValue> {
        Some(Rc::new(self.snippet.description.clone()))
    }

    fn complete(
        &self,
        editor: &TextEditor,
        range: SimpleSegment,
        _: Option<&dyn IRoutedEventArgs>,
    ) -> Result<bool, DocumentError> {
        let selection = editor.selection();
        let Some((installation, scope)) = language_at(editor, range.end_offset())? else {
            return Ok(false);
        };
        let Some(packages) = self.packages.upgrade() else {
            return Ok(false);
        };
        if !physical(&selection)
            || !selection.is_empty()
            || scope.id != self.language
            || !self
                .installation
                .upgrade()
                .is_some_and(|current| Rc::ptr_eq(&current, &installation))
            || !Rc::ptr_eq(&packages, &installation.package_snippets.borrow())
        {
            return Ok(false);
        }
        match self
            .snippet
            .accept(editor, range, scope.id, &installation, &packages, false)
        {
            Ok(_) => Ok(true),
            Err(SnippetError::Document(error)) => Err(error),
            Err(error) => {
                editor.report_error(error.to_string());
                Ok(false)
            }
        }
    }
}

impl PackageSnippet {
    fn accept(
        &self,
        editor: &TextEditor,
        range: SimpleSegment,
        language: u8,
        installation: &Rc<TextMateInstallation>,
        packages: &Rc<PackageSnippets>,
        use_selection: bool,
    ) -> Result<Rc<SnippetController>, SnippetError> {
        let document = editor.document();
        let snapshot = document.create_snapshot();
        let epoch = editor.document_epoch();
        let selection = editor.selection();
        let selected = if use_selection {
            snapshot.get_text(range.offset, range.length)?
        } else {
            String::new()
        };
        let provider = installation.snippet_variables.borrow().clone();
        let input = installation.input_languages();
        let variables = EditorSnippetVariables {
            snapshot: &snapshot,
            caret: selection.primary().caret,
            file_name: document.file_name(),
            configuration: input.get(language).map(|rules| &rules.configuration),
        };
        let body = {
            let resolving = &installation.resolving_snippet_variables;
            if resolving.replace(true) {
                return Err(DocumentError::ReentrantChange.into());
            }
            struct Restore<'a>(&'a Cell<bool>);
            impl Drop for Restore<'_> {
                fn drop(&mut self) {
                    self.0.set(false);
                }
            }
            let _restore = Restore(resolving);
            self.body.instantiate_with_variables(&selected, |name| {
                if let Some(provider) = &provider {
                    if let Some(value) = provider(name)? {
                        return Ok(Some(value));
                    }
                }
                variables.resolve(name)
            })?
        };
        if epoch != editor.document_epoch()
            || snapshot.version().as_ref() != Some(&document.version())
            || selection != editor.selection()
            || !editor
                .textmate_installation()
                .is_some_and(|current| Rc::ptr_eq(&current, installation))
            || !Rc::ptr_eq(packages, &installation.package_snippets.borrow())
            || !Rc::ptr_eq(&input, &installation.input_languages())
        {
            return Err(DocumentError::ReentrantChange.into());
        }
        editor.insert_snippet_at(&body, range, &selected)
    }
}

struct EditorSnippetVariables<'a> {
    snapshot: &'a TextSnapshot,
    caret: usize,
    file_name: Option<String>,
    configuration: Option<&'a LanguageConfiguration>,
}

impl EditorSnippetVariables<'_> {
    fn resolve(&self, name: &str) -> Result<Option<String>, SnippetError> {
        let value = match name {
            "TM_CURRENT_LINE" | "TM_CURRENT_WORD" | "TM_LINE_INDEX" | "TM_LINE_NUMBER" => {
                let line = self.snapshot.get_line_by_offset(self.caret)?;
                return match name {
                    "TM_LINE_INDEX" => Ok(Some((line.line_number - 1).to_string())),
                    "TM_LINE_NUMBER" => Ok(Some(line.line_number.to_string())),
                    "TM_CURRENT_LINE" => {
                        Ok(Some(self.snapshot.get_text(line.offset, line.length)?))
                    }
                    _ => {
                        let column = (self.caret - line.offset).min(line.length);
                        self.word(line.offset, line.length, column)
                    }
                };
            }
            "CURSOR_INDEX" => Some("0".to_owned()),
            "CURSOR_NUMBER" => Some("1".to_owned()),
            "LINE_COMMENT" => self
                .configuration
                .and_then(LanguageConfiguration::line_comment)
                .map(str::to_owned),
            "BLOCK_COMMENT_START" | "BLOCK_COMMENT_END" => self
                .configuration
                .and_then(LanguageConfiguration::block_comment)
                .map(|(open, close)| {
                    if name == "BLOCK_COMMENT_START" {
                        open
                    } else {
                        close
                    }
                })
                .map(str::to_owned),
            _ => self
                .file_name
                .as_deref()
                .and_then(|path| file_variable(name, path))
                .map(str::to_owned),
        };
        Ok(value)
    }

    fn word(
        &self,
        start: usize,
        length: usize,
        column: usize,
    ) -> Result<Option<String>, SnippetError> {
        let text = self.snapshot.get_text(start, length)?;
        let regex_error =
            |error: ferroui_textmate::Error| SnippetError::Extension(error.to_string());
        if let Some(configuration) = self.configuration {
            if configuration.word_regex().map_err(regex_error)?.is_some() {
                let word = configuration
                    .word_at_offset(&text, column)
                    .map_err(regex_error)?;
                return word
                    .map(|word| {
                        self.snapshot
                            .get_text(start + word.start_index, word.end_index - word.start_index)
                    })
                    .transpose()
                    .map_err(Into::into);
            }
        }
        Ok(TextUtilities::identifier_at(&text, column).map(str::to_owned))
    }
}

fn file_variable<'a>(name: &str, path: &'a str) -> Option<&'a str> {
    let separator = ['/', '\\'];
    let filename = path.rsplit(separator).next()?;
    let directory = || {
        let (parent, _) = path.rsplit_once(separator)?;
        Some(if parent.is_empty() || parent.ends_with(':') {
            &path[..parent.len() + 1]
        } else {
            parent
        })
    };
    match name {
        "TM_FILENAME" => Some(filename),
        "TM_FILENAME_BASE" => Some(
            filename
                .rsplit_once('.')
                .filter(|(stem, _)| !stem.is_empty())
                .map_or(filename, |(stem, _)| stem),
        ),
        "TM_DIRECTORY" => directory(),
        "TM_DIRECTORY_BASE" => directory()?.rsplit(separator).next(),
        "TM_FILEPATH" => directory().map(|_| path),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
    use ferroui_base::Ref;
    use ferroui_controls::{Control, Window};
    use ferroui_textmate::{read_json, Registry, Theme};

    fn install(editor: &Ref<TextEditor>, definitions: &str) -> Rc<TextMateInstallation> {
        let mut registry = Registry::new();
        registry
            .add_grammar_json(r#"{"scopeName":"source.test","patterns":[]}"#)
            .unwrap();
        let installation = editor.install_textmate(
            registry.load_grammar("source.test").unwrap(),
            Theme::from_json(r#"{"tokenColors":[]}"#).unwrap(),
        );
        let configuration = LanguageConfiguration::from_json(
            r#"{"wordPattern":{"pattern":"[\\p{L}\\p{N}_-]+","flags":"u"},"comments":{"lineComment":"//"}}"#,
        )
        .unwrap();
        installation
            .set_language_configuration(Some(&configuration))
            .unwrap();
        let definitions = read_json(definitions).unwrap();
        let language = Language {
            id: "test".to_owned(),
            aliases: Vec::new(),
            extensions: Vec::new(),
            filenames: Vec::new(),
            filename_patterns: Vec::new(),
            first_line: None,
            mime_types: Vec::new(),
            configuration_file: None,
            scope_name: Some("source.test".to_owned()),
            configuration: Some(configuration),
            snippets: definitions
                .as_object()
                .unwrap()
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
        };
        installation.set_package_snippets([(0, &language)]).unwrap();
        installation
    }

    #[test]
    fn virtual_space_snippet_named_insertion_materializes_padding() {
        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("ab");
        install(
            &editor,
            r#"{"Named":{"prefix":"append","body":"${1:😀}$0"}}"#,
        );
        editor.session().set_caret_position(2, 3).unwrap();
        assert!(editor.package_snippet_suggestions().unwrap().is_none());
        let controller = editor.insert_package_snippet("Named").unwrap().unwrap();
        assert_eq!(editor.text(), "ab   😀");
        assert_eq!(editor.selection().primary().segment(), SimpleSegment::new(5, 2));
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), "ab");
        assert!(!controller.is_active());
    }

    #[test]
    fn package_transform_uses_filename_and_commits_on_return() {
        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("");
        install(
            &editor,
            r#"{"Named":{"prefix":"format","body":"${TM_FILENAME/(.*)\\..+$/${1:/pascalcase}/}: ${1:value} = ${1/(.*)/${1:/upcase}/}$0"}}"#,
        );
        editor
            .document()
            .set_file_name(Some("main.window.rs".to_owned()));
        let controller = editor.insert_package_snippet("Named").unwrap().unwrap();
        assert_eq!(editor.text(), "MainWindow: value = value");
        editor.replace_selection_with_text("model").unwrap();
        assert_eq!(editor.text(), "MainWindow: model = model");
        let mut key = KeyEventArgs::new();
        key.key = Key::Return;
        key.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&key);
        assert!(key.handled());
        assert!(!controller.is_active());
        assert_eq!(editor.text(), "MainWindow: model = MODEL");
    }

    #[test]
    fn package_choice_popup_uses_editor_keys_and_reopens_on_return_to_field() {
        let _scope = crate::tests::test_scope();
        let editor = crate::tests::laid_out_editor("");
        let root = Window::new();
        root.set_width(600.0);
        root.set_height(360.0);
        root.set_content(Some(Control::boxed(editor.clone())));
        root.show();
        let area = editor.text_area();
        area.focus();
        install(
            &editor,
            r#"{"Named":{"prefix":"choice","body":"${1:before} ${2|one,two,😀|} / $2 ${3:after}$0"}}"#,
        );
        let controller = editor.insert_package_snippet("Named").unwrap().unwrap();
        let press = |key, key_modifiers| {
            let mut event = KeyEventArgs::new();
            event.key = key;
            event.key_modifiers = key_modifiers;
            event.set_routed_event(Some(InputElement::key_down_event()));
            area.raise_event(&event);
            assert!(event.handled(), "{key:?}");
        };
        press(Key::Tab, KeyModifiers::NONE);
        let popup = crate::CompletionWindow::active_for(&editor).unwrap();
        let list = popup.completion_list().current_list();
        assert_eq!(
            list.iter().map(|item| item.text()).collect::<Vec<_>>(),
            ["one", "two", "😀"]
        );
        press(Key::Tab, KeyModifiers::SHIFT);
        assert!(popup.is_closed());
        assert_eq!(editor.selected_text().unwrap(), "before");
        press(Key::Tab, KeyModifiers::NONE);
        press(Key::Up, KeyModifiers::NONE);
        press(Key::Return, KeyModifiers::NONE);
        assert_eq!(editor.text(), "before 😀 / 😀 after");
        assert_eq!(editor.selected_text().unwrap(), "after");
        press(Key::Tab, KeyModifiers::SHIFT);
        let popup = crate::CompletionWindow::active_for(&editor).unwrap();
        assert_eq!(popup.selected_completion_data().unwrap().text(), "😀");
        press(Key::Escape, KeyModifiers::NONE);
        assert!(popup.is_closed());
        assert!(controller.is_active());
        assert_eq!(editor.selected_text().unwrap(), "😀");
        root.close();
    }

    #[test]
    fn named_and_completion_snippets_expand_document_and_host_values() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let installation = install(
            &editor,
            r#"{
            "Named":{"prefix":"named","body":"${TM_SELECTED_TEXT}:${TM_CURRENT_WORD}:${TM_LINE_INDEX}/${TM_LINE_NUMBER}:${TM_FILENAME_BASE}:${LINE_COMMENT}:${WORKSPACE_NAME}:${CLIPBOARD:missing}$0"},
            "Complete":{"prefix":"cmp","body":"${TM_CURRENT_LINE}:${TM_CURRENT_WORD}:${CURRENT_YEAR}:${TM_FILEPATH}:${TM_SELECTED_TEXT}"}
        }"#,
        );
        installation.set_snippet_variable_provider(Some(Rc::new(|name| {
            Ok(match name {
                "WORKSPACE_NAME" => Some("workspace$1".to_owned()),
                "CURRENT_YEAR" => Some("2026".to_owned()),
                "TM_SELECTED_TEXT" => panic!("the selection belongs to the editor"),
                _ => None,
            })
        })));
        let before = "head\r\n😀 café-42 tail";
        editor.set_text(before).unwrap();
        editor
            .document()
            .set_file_name(Some(r"C:\workspace\file.test.rs".to_owned()));
        editor.select(9, 2).unwrap();
        editor.insert_package_snippet("Named").unwrap().unwrap();
        assert_eq!(
            editor.text(),
            "head\r\n😀 ca:café-42:1/2:file.test://:workspace$1:missingfé-42 tail"
        );
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), before);

        editor.set_text("😀 cmp").unwrap();
        editor.set_caret_offset(6).unwrap();
        let suggestions = editor.package_snippet_suggestions().unwrap().unwrap();
        let item = suggestions
            .items
            .iter()
            .find(|item| item.text() == "cmp")
            .unwrap();
        assert!(item.complete(&editor, suggestions.range, None).unwrap());
        assert_eq!(
            editor.text(),
            r"😀 😀 cmp:cmp:2026:C:\workspace\file.test.rs:"
        );
    }

    #[test]
    fn variable_provider_that_replaces_the_document_cannot_apply_a_stale_snippet() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("original").unwrap();
        let original = editor.document();
        let installation = install(
            &editor,
            r#"{"Named":{"prefix":"n","body":"${WORKSPACE_NAME}"}}"#,
        );
        let weak = editor.downgrade();
        installation.set_snippet_variable_provider(Some(Rc::new(move |_| {
            weak.upgrade()
                .unwrap()
                .set_document(crate::TextDocument::from_text("replacement"));
            Ok(Some("workspace".to_owned()))
        })));
        assert!(matches!(
            editor.insert_package_snippet("Named"),
            Err(SnippetError::Document(DocumentError::ReentrantChange))
        ));
        assert_eq!(original.text(), "original");
        assert_eq!(editor.text(), "replacement");
    }

    #[test]
    fn recursive_variable_provider_is_rejected_and_can_retry() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let installation = install(
            &editor,
            r#"{"Named":{"prefix":"n","body":"${WORKSPACE_NAME:workspace}"}}"#,
        );
        let weak = editor.downgrade();
        installation.set_snippet_variable_provider(Some(Rc::new(move |_| {
            weak.upgrade().unwrap().insert_package_snippet("Named")?;
            Ok(None)
        })));
        assert!(matches!(
            editor.insert_package_snippet("Named"),
            Err(SnippetError::Document(DocumentError::ReentrantChange))
        ));
        assert_eq!(editor.text(), "");
        installation.set_snippet_variable_provider(None);
        editor.insert_package_snippet("Named").unwrap().unwrap();
        assert_eq!(editor.text(), "workspace");
    }
}
