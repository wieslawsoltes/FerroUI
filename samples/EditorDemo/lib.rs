//! A shared editor sample exercising the editor through its public API.

extern crate self as editor_demo;

ferroui_markup_xaml::include_compiled_xaml!();

mod app;
mod smoke;
pub use app::App;

use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::IBrush;
use ferroui_base::metadata::{from_markup_value, MarkupAssembly};
use ferroui_base::reactive::Subscription;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::Ref;
use ferroui_controls::{Button, TextBlock, UserControl};
use ferroui_edit::core::completion::CompletionItem;
use ferroui_edit::core::document::TextDocument;
use ferroui_edit::core::editing::{CSharpIndentationStrategy, DefaultIndentationStrategy};
use ferroui_edit::core::snippets::TextMateSnippet;
use ferroui_textmate::{BundledCatalog, Registry, ThemeName};
use ferroui_edit::{CompletionWindow, EditorCommandKind, ICompletionData, TextEditor};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const RUST_SOURCE: &str = include_str!("examples/hello.rs.txt");
const CSHARP_SOURCE: &str = include_str!("examples/hello.cs.txt");
const JAVASCRIPT_SOURCE: &str = "/**\n * Greets the editor.\n */\nfunction greet(name) {\n\treturn `Hello, ${name}!`;\n}\n\nconsole.log(greet(\"FerroUI\"));\n";
const HTML_SOURCE: &str = "<!DOCTYPE html>\n<title>FerroUI</title>\n<style>\n  body { color: teal; }\n</style>\n<script>\n  /**\n   * Greets a visitor.\n   */\n  function greet(name) {\n    return `Hello, ${name}!`;\n  }\n</script>\n<p>Hello</p>\n";

pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "EditorDemo",
    crate_name: "editor_demo",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageMode {
    Rust,
    CSharp,
    JavaScript,
    Html,
}
impl LanguageMode {
    fn id(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::CSharp => "csharp",
            Self::JavaScript => "javascript",
            Self::Html => "html",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::CSharp => "C#",
            Self::JavaScript => "JavaScript",
            Self::Html => "HTML",
        }
    }
}

pub struct EditorDemo {
    view: Ref<UserControl>,
    editor: Ref<TextEditor>,
    status: Ref<TextBlock>,
    catalog: BundledCatalog,
    registry: Registry,
    language: Cell<LanguageMode>,
    dark: Cell<bool>,
    completion: RefCell<Option<Ref<CompletionWindow>>>,
    subscriptions: RefCell<Vec<Subscription>>,
}
impl EditorDemo {
    pub fn new(dark: bool) -> Result<Rc<Self>, String> {
        ferroui_markup_xaml::register_types();
        ferroui_edit::register_types();
        let value = compiled_markup::try_load(None, "ferres://EditorDemo/MainView.xaml")
            .map_err(|error| error.to_string())?;
        let view: Ref<UserControl> =
            from_markup_value(&value).ok_or("Compiled editor view is not a UserControl")?;
        let editor = view
            .find_control::<TextEditor>("Editor")
            .ok_or("Editor name is missing")?;
        let mut options = editor.options();
        options.auto_indent_on_paste = true;
        options.auto_indent_on_paste_within_string = false;
        editor.set_options(options);
        let status = view
            .find_control::<TextBlock>("Status")
            .ok_or("Status name is missing")?;
        let catalog = BundledCatalog::new().map_err(|error| error.to_string())?;
        let registry = catalog.registry().map_err(|error| error.to_string())?;
        let demo = Rc::new(Self {
            view,
            editor,
            status,
            catalog,
            registry,
            language: Cell::new(LanguageMode::Rust),
            dark: Cell::new(dark),
            completion: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
        });
        demo.connect_actions()?;
        demo.set_language(LanguageMode::Rust)?;
        Ok(demo)
    }
    pub fn view(&self) -> Ref<UserControl> {
        self.view.clone()
    }
    pub fn editor(&self) -> Ref<TextEditor> {
        self.editor.clone()
    }
    pub fn language(&self) -> LanguageMode {
        self.language.get()
    }
    pub fn set_language(&self, language: LanguageMode) -> Result<(), String> {
        self.language.set(language);
        let previous = self.completion.borrow_mut().take();
        if let Some(previous) = previous {
            previous.close_window();
        }
        self.editor
            .set_document(TextDocument::from_text(match language {
                LanguageMode::Rust => RUST_SOURCE,
                LanguageMode::CSharp => CSHARP_SOURCE,
                LanguageMode::JavaScript => JAVASCRIPT_SOURCE,
                LanguageMode::Html => HTML_SOURCE,
            }));
        if language == LanguageMode::CSharp {
            self.editor.session().set_indentation_strategy(Some(Rc::new(
                CSharpIndentationStrategy::from_options(&self.editor.options()),
            )));
        } else {
            self.editor.session().set_indentation_strategy(Some(Rc::new(DefaultIndentationStrategy)));
        }
        self.apply_theme()?;
        self.editor.install_language_folding();
        self.update_status();
        Ok(())
    }
    pub fn toggle_theme(&self) -> Result<(), String> {
        self.dark.set(!self.dark.get());
        self.apply_theme()
    }
    fn apply_theme(&self) -> Result<(), String> {
        self.view.set_value(
            ThemeVariant::requested_theme_variant_property(),
            Some(if self.dark.get() {
                ThemeVariant::dark()
            } else {
                ThemeVariant::light()
            }),
        );
        let language = self
            .catalog
            .language_by_id(self.language.get().id())
            .ok_or("Bundled language is missing")?;
        let scope = language.scope_name.as_deref()
            .ok_or("Bundled language has no grammar")?;
        let grammar = self
            .registry
            .load_grammar(scope)
            .map_err(|error| error.to_string())?;
        let theme = self
            .catalog
            .theme(if self.dark.get() {
                ThemeName::DarkPlus
            } else {
                ThemeName::LightPlus
            })
            .map_err(|error| error.to_string())?;
        let defaults = theme.defaults();
        let background = theme.colors.get("editor.background").copied()
            .or(defaults.background).map(argb).unwrap_or(if self.dark.get() {
            0xff1e1e1e
        } else {
            0xffffffff
        });
        let foreground = theme.colors.get("editor.foreground").copied()
            .or(defaults.foreground).map(argb).unwrap_or(if self.dark.get() {
            0xffd4d4d4
        } else {
            0xff202020
        });
        self.view.set_background(Some(
            Rc::new(ImmutableSolidColorBrush::from_uint32(background)) as Rc<dyn IBrush>,
        ));
        self.view.set_value(
            TextBlock::foreground_property(),
            Some(Rc::new(ImmutableSolidColorBrush::from_uint32(foreground)) as Rc<dyn IBrush>),
        );
        self.editor.set_background(Some(
            Rc::new(ImmutableSolidColorBrush::from_uint32(background)) as Rc<dyn IBrush>,
        ));
        self.editor.set_foreground(Some(
            Rc::new(ImmutableSolidColorBrush::from_uint32(foreground)) as Rc<dyn IBrush>,
        ));
        let ids: HashMap<_, _> = self
            .catalog
            .languages()
            .iter()
            .enumerate()
            .filter_map(|(index, language)| Some((language.id.as_str(), u8::try_from(index + 1).ok()?)))
            .collect();
        let mut configuration = self
            .catalog
            .grammar_configuration(scope, |id| ids.get(id).copied())
            .map_err(|error| error.to_string())?;
        configuration.initial_language = ids
            .get(language.id.as_str())
            .copied()
            .ok_or("Missing numeric ID for the sample language")?;
        let languages = self
            .catalog
            .languages()
            .iter()
            .filter_map(|language| {
                let id = *ids.get(language.id.as_str())?;
                (id == configuration.initial_language
                    || configuration.embedded_languages.values().any(|value| *value == id))
                    .then_some((id, language))
            })
            .collect::<Vec<_>>();
        let root = configuration.initial_language;
        let embedded = languages.iter().filter_map(|&(id, language)| {
            (id != root).then_some((id, language.configuration.as_ref()?))
        });
        let installation = self.editor.install_textmate(grammar, theme);
        installation
            .set_language_configurations(configuration, language.configuration.as_ref(), embedded)
            .map_err(|error| error.to_string())?;
        installation.set_package_snippets(languages).map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn show_completion(&self) -> Result<(), String> {
        let document = self.editor.document();
        let caret = self.editor.caret_offset();
        let line = document
            .get_line_by_offset(caret)
            .map_err(|e| e.to_string())?;
        let prefix = document
            .get_text(line.offset, caret - line.offset)
            .map_err(|e| e.to_string())?;
        let word_length = prefix
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .map(char::len_utf16)
            .sum::<usize>();
        let words = match self.language.get() {
            LanguageMode::Rust => vec![
                (
                    "println!",
                    "println!(\"Hello, FerroUI!\");",
                    "Print a line to standard output.",
                ),
                ("String", "String", "An owned UTF-8 string."),
                ("Vec", "Vec", "A growable vector."),
                ("Result", "Result", "A success value or an error."),
                ("Option", "Option", "An optional value."),
            ],
            LanguageMode::CSharp => vec![
                (
                    "Console.WriteLine",
                    "Console.WriteLine(\"Hello, FerroUI!\");",
                    "Write a line to standard output.",
                ),
                ("String", "String", "A UTF-16 string."),
                ("List", "List", "A typed list."),
                ("Task", "Task", "An asynchronous operation."),
                (
                    "CancellationToken",
                    "CancellationToken",
                    "Cooperative operation cancellation.",
                ),
            ],
            LanguageMode::JavaScript => vec![
                ("console.log", "console.log(\"Hello, FerroUI!\");", "Write a line to the console."),
                ("Array", "Array", "An ordered collection."),
                ("Promise", "Promise", "An asynchronous result."),
            ],
            LanguageMode::Html => vec![
                ("section", "<section></section>", "Group related content."),
                ("button", "<button></button>", "Add a button."),
                ("script", "<script></script>", "Embed JavaScript."),
            ],
        };
        let packages = self.editor.package_snippet_suggestions().map_err(|error| error.to_string())?;
        let (start, mut items) = packages.map_or((caret - word_length, Vec::new()), |suggestions| {
            (suggestions.range.offset, suggestions.items)
        });
        items.extend(words
            .into_iter()
            .map(|(text, insert, description)| Rc::new(CompletionItem {
                text: text.into(),
                insertion_text: Some(insert.into()),
                description: description.into(),
                priority: 0.0,
            }) as Rc<dyn ICompletionData>));
        let previous = self.completion.borrow_mut().take();
        if let Some(previous) = previous {
            previous.close_window();
        }
        self.editor.text_area().focus();
        *self.completion.borrow_mut() = Some(
            CompletionWindow::show_with_data(&self.editor, start, caret, items)
                .map_err(|error| error.to_string())?,
        );
        Ok(())
    }
    pub fn insert_snippet(&self) -> Result<(), String> {
        let source = match self.language.get() {
            LanguageMode::Rust => "let ${1:name} = ${2:value};\nprintln!(\"{:?}\", $1);$0",
            LanguageMode::CSharp => "var ${1:name} = ${2:value};\nConsole.WriteLine($1);$0",
            LanguageMode::JavaScript => "const ${1|name,entry,result|} = ${2:value};\nconsole.log($1);\n// ${2/(.*)/${1:/upcase}/}$0",
            LanguageMode::Html => "<${1:section}>${2:Content}</$1>$0",
        };
        let snippet = TextMateSnippet::parse(source)
            .and_then(|snippet| snippet.instantiate(""))
            .map_err(|error| error.to_string())?;
        self.editor
            .insert_snippet(&snippet)
            .map_err(|error| error.to_string())?;
        self.editor.text_area().focus();
        Ok(())
    }
    fn update_status(&self) {
        let document = self.editor.document();
        let location = document
            .get_location(self.editor.caret_offset())
            .unwrap_or_default();
        self.status.set_text(Some(&format!(
            "{}  |  Ln {}, Col {}  |  {} lines  |  {} UTF-16 units  |  {}",
            self.language.get().title(),
            location.line,
            location.column,
            document.line_count(),
            document.text_length(),
            if document.can_undo() {
                "Undo available"
            } else {
                "Saved example"
            }
        )));
    }
    fn command(
        self: &Rc<Self>,
        name: &str,
        action: impl Fn(&EditorDemo) -> Result<(), String> + 'static,
    ) -> Result<(), String> {
        let button = self
            .view
            .find_control::<Button>(name)
            .ok_or_else(|| format!("Missing toolbar control {name}"))?;
        let weak = Rc::downgrade(self);
        self.subscriptions
            .borrow_mut()
            .push(
                button.add_disposable_handler(Button::click_event(), move |_, _| {
                    if let Some(demo) = weak.upgrade() {
                        if let Err(error) = action(&demo) {
                            demo.status.set_text(Some(&error));
                        }
                    }
                }),
            );
        Ok(())
    }
    fn connect_actions(self: &Rc<Self>) -> Result<(), String> {
        self.command("RustLanguage", |demo| demo.set_language(LanguageMode::Rust))?;
        self.command("CSharpLanguage", |demo| {
            demo.set_language(LanguageMode::CSharp)
        })?;
        self.command("JavaScriptLanguage", |demo| demo.set_language(LanguageMode::JavaScript))?;
        self.command("HtmlLanguage", |demo| demo.set_language(LanguageMode::Html))?;
        self.command("Find", |demo| {
            demo.editor.search_panel().show(false);
            Ok(())
        })?;
        self.command("Replace", |demo| {
            demo.editor.search_panel().show(true);
            Ok(())
        })?;
        self.command("Complete", Self::show_completion)?;
        self.command("Snippet", Self::insert_snippet)?;
        let reindent = self
            .view
            .find_control::<Button>("Reindent")
            .ok_or("Missing toolbar control Reindent")?;
        reindent.set_command(Some(
            self.editor
                .command(EditorCommandKind::Reindent)
                .as_command(),
        ));
        self.command("Theme", Self::toggle_theme)?;
        self.command("Wrap", |demo| {
            demo.editor.set_word_wrap(!demo.editor.word_wrap());
            Ok(())
        })?;
        self.command("Fold", |demo| {
            if let Some(manager) = demo.editor.folding_manager() {
                manager.set_all_folded(true, false);
            }
            Ok(())
        })?;
        self.command("Unfold", |demo| {
            if let Some(manager) = demo.editor.folding_manager() {
                manager.set_all_folded(false, false);
            }
            Ok(())
        })?;
        let weak = Rc::downgrade(self);
        self.subscriptions
            .borrow_mut()
            .push(self.editor.add_disposable_handler(
                TextEditor::text_changed_event(),
                move |_, _| {
                    if let Some(demo) = weak.upgrade() {
                        demo.update_status();
                    }
                },
            ));
        let weak = Rc::downgrade(self);
        self.subscriptions
            .borrow_mut()
            .push(self.editor.add_disposable_handler_with(
                InputElement::key_down_event(),
                move |_, event: &KeyEventArgs| {
                    if event.key == Key::Space
                        && event
                            .key_modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
                    {
                        if let Some(demo) = weak.upgrade() {
                            if let Err(error) = demo.show_completion() {
                                demo.status.set_text(Some(&error));
                            }
                            event.set_handled(true);
                        }
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            ));
        let weak = Rc::downgrade(self);
        self.subscriptions
            .borrow_mut()
            .push(self.editor.add_disposable_handler_with(
                InputElement::key_up_event(),
                move |_, _| {
                    if let Some(demo) = weak.upgrade() {
                        demo.update_status();
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            ));
        let weak = Rc::downgrade(self);
        self.subscriptions
            .borrow_mut()
            .push(self.editor.add_disposable_handler_with(
                InputElement::pointer_released_event(),
                move |_, _| {
                    if let Some(demo) = weak.upgrade() {
                        demo.update_status();
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            ));
        Ok(())
    }
}
impl Drop for EditorDemo {
    fn drop(&mut self) {
        if let Some(window) = self.completion.get_mut().take() {
            window.close_window();
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}
fn argb(color: ferroui_edit::textmate::Color) -> u32 {
    u32::from_be_bytes([color.a, color.r, color.g, color.b])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_bundled_snippet_body_compiles_and_prepares_with_or_without_selection() {
        let catalog = BundledCatalog::new().unwrap();
        let mut count = 0;
        for language in catalog.languages() {
            for (name, definition) in language.typed_snippets().unwrap().snippets {
                if definition.body.is_empty() { continue; }
                let body = TextMateSnippet::parse(&definition.body.join("\n"))
                    .unwrap_or_else(|error| panic!("{} {name:?}: {error}", language.id));
                for selection in ["", "selected😀"] {
                    body.instantiate(selection).and_then(|snippet| snippet.prepare(selection))
                        .unwrap_or_else(|error| panic!("{} {name:?}: {error}", language.id));
                }
                count += 1;
            }
        }
        assert!(count > 150, "expected the bundled snippet corpus, got {count}");
    }
    #[test]
    fn compiled_markup_loads_editor_and_commands_use_real_services() {
        use ferroui_base::media::text_formatting::testing::TextTestScope;
        use ferroui_base::platform::IPlatformRenderInterface;
        use ferroui_base::{FerroLocator, LocatorExtensions};
        use ferroui_controls::testing::{TestServices, UnitTestApplication};
        let _text = TextTestScope::new();
        let renderer = FerroLocator::current()
            .get_service::<dyn IPlatformRenderInterface>()
            .unwrap();
        let _app = UnitTestApplication::start(
            TestServices::styled_window().with_render_interface(renderer),
        );
        let demo = EditorDemo::new(false).unwrap();
        assert!(demo.editor.text().contains("fn main"));
        assert!(demo.editor.show_line_numbers());
        demo.editor.select(0, 0).unwrap();
        demo.insert_snippet().unwrap();
        assert_eq!(demo.editor.selected_text().unwrap(), "name");
        demo.editor
            .replace_selection_with_text("value_name")
            .unwrap();
        assert_eq!(demo.editor.text().matches("value_name").count(), 2);
        demo.set_language(LanguageMode::CSharp).unwrap();
        assert!(demo.editor.text().contains("class Program"));
        assert!(demo.editor.folding_manager().unwrap().all_foldings().len() >= 2);
        demo.toggle_theme().unwrap();
        assert_eq!(demo.view.actual_theme_variant(), Some(ThemeVariant::dark()));

        demo.set_language(LanguageMode::Html).unwrap();
        let source = "<script>\n\twork😀();\n</script>";
        let start = "<script>\n\t".len();
        let end = start + "work😀();".encode_utf16().count();
        demo.editor.set_text(source).unwrap();
        demo.editor.select(start, end - start).unwrap();
        assert!(demo.editor.insert_package_snippet("For Loop").unwrap().is_some());
        assert_eq!(demo.editor.selected_text().unwrap(), "index");
        assert!(demo.editor.text().contains("\n\t\twork😀();"));
        assert!(demo.editor.undo().unwrap());
        assert_eq!(demo.editor.text(), source);
        assert_eq!(demo.editor.selected_text().unwrap(), "work😀();");
    }
}
