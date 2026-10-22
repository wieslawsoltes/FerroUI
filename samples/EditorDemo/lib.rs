//! A runtime XAML application exercising the editor through its public API.

mod app;
pub use app::App;

use ferroui_base::Ref;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::IBrush;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::{Button, TextBlock, UserControl};
use ferroui_edit::core::completion::CompletionItem;
use ferroui_edit::core::document::TextDocument;
use ferroui_edit::core::folding::NewFolding;
use ferroui_edit::core::snippets::Snippet;
use ferroui_edit::textmate::{BundledCatalog, Registry, ThemeName};
use ferroui_edit::{CompletionWindow, TextEditor};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const RUST_SOURCE: &str = include_str!("examples/hello.rs.txt");
const CSHARP_SOURCE: &str = include_str!("examples/hello.cs.txt");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageMode {
    Rust,
    CSharp,
}
impl LanguageMode {
    fn id(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::CSharp => "csharp",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::CSharp => "C#",
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
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}
impl EditorDemo {
    pub fn new(dark: bool) -> Result<Rc<Self>, String> {
        ferroui_markup_xaml::register_types();
        ferroui_edit::register_types();
        let view = FerroRuntimeXamlLoader::parse_as::<Ref<UserControl>>(
            include_str!("MainView.xaml"),
            None,
        )
        .map_err(|error| error.to_string())?;
        let editor = view
            .find_control::<TextEditor>("Editor")
            .ok_or("Editor name is missing")?;
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
            }));
        self.apply_theme()?;
        self.editor.install_folding();
        self.update_foldings()?;
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
        let scope = self
            .catalog
            .language_by_id(self.language.get().id())
            .and_then(|language| language.scope_name.as_deref())
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
        let background = defaults.background.map(argb).unwrap_or(if self.dark.get() {
            0xff1e1e1e
        } else {
            0xffffffff
        });
        let foreground = defaults.foreground.map(argb).unwrap_or(if self.dark.get() {
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
        self.editor.install_textmate(grammar, theme);
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
        };
        let items = words
            .into_iter()
            .map(|(text, insert, description)| CompletionItem {
                text: text.into(),
                insertion_text: Some(insert.into()),
                description: description.into(),
                priority: 0.0,
            })
            .collect();
        let previous = self.completion.borrow_mut().take();
        if let Some(previous) = previous {
            previous.close_window();
        }
        self.editor.text_area().focus();
        *self.completion.borrow_mut() = Some(
            CompletionWindow::show_for(&self.editor, caret - word_length, caret, items)
                .map_err(|error| error.to_string())?,
        );
        Ok(())
    }
    pub fn insert_snippet(&self) -> Result<(), String> {
        let source = match self.language.get() {
            LanguageMode::Rust => "let ${1:name} = ${2:value};\nprintln!(\"{:?}\", $1);$0",
            LanguageMode::CSharp => "var ${1:name} = ${2:value};\nConsole.WriteLine($1);$0",
        };
        let snippet = Snippet::parse(source).map_err(|error| error.to_string())?;
        self.editor
            .insert_snippet(&snippet)
            .map_err(|error| error.to_string())?;
        self.editor.text_area().focus();
        Ok(())
    }
    fn update_foldings(&self) -> Result<(), String> {
        if let Some(manager) = self.editor.folding_manager() {
            let (regions, error) = region_foldings(&self.editor.document())?;
            manager
                .update_foldings(&regions, error)
                .map_err(|error| error.to_string())?;
        }
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
                        if let Err(error) = demo.update_foldings() {
                            demo.status.set_text(Some(&error));
                        }
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

/// The demo intentionally folds explicit region markers. Language parsers can
/// supply their own ranges through the same FoldingManager API.
fn region_foldings(document: &TextDocument) -> Result<(Vec<NewFolding>, Option<usize>), String> {
    let mut stack = Vec::new();
    let mut foldings = Vec::new();
    for number in 1..=document.line_count() {
        let line = document
            .get_line_by_number(number)
            .map_err(|e| e.to_string())?;
        let text = document
            .get_text(line.offset, line.length)
            .map_err(|e| e.to_string())?;
        let trimmed = text.trim_start();
        if let Some(title) = trimmed
            .strip_prefix("// region ")
            .or_else(|| trimmed.strip_prefix("#region "))
        {
            stack.push((line.end_offset(), title.to_owned()));
        } else if trimmed.starts_with("// endregion") || trimmed.starts_with("#endregion") {
            if let Some((start, title)) = stack.pop() {
                let mut region = NewFolding::new(start, line.end_offset());
                region.name = Some(title);
                region.is_definition = true;
                foldings.push(region);
            }
        }
    }
    foldings.sort_by_key(|region| region.start_offset);
    Ok((foldings, stack.first().map(|(offset, _)| *offset)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_ranges_use_utf16_and_preserve_incomplete_suffix() {
        let doc =
            TextDocument::from_text("// region 😀\nlet x = 1;\n// endregion\n// region pending\n");
        let (regions, error) = region_foldings(&doc).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start_offset, 12);
        assert!(error.is_some());
    }
    #[test]
    fn runtime_markup_loads_editor_and_commands_use_real_services() {
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
    }
}
