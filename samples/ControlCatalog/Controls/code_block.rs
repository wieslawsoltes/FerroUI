//! Port of `Controls/CodeBlock.cs`.

use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_enum, ferro_properties, instantiate, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::documents::{Run, TextElement};
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::{Button, ControlImpl, SelectableTextBlock, TopLevel};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use mini_mvvm::start_async;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The language a [`CodeBlock`] highlights.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CodeLanguage {
    #[default]
    Xaml = 0,
    CSharp = 1,
}

ferro_markup_enum!(CodeLanguage { Xaml, CSharp });

const PART_TEXT: &str = "PART_Text";
const PART_COPY_BUTTON: &str = "PART_CopyButton";

/// Snippets longer than this start collapsed. Catalog snippets are short by
/// design (the median is four lines), so showing them costs less than a row
/// that says nothing; only the rare long one is worth hiding.
const COLLAPSE_THRESHOLD: usize = 12;

/// A read-only, syntax-highlighted snippet with a copy button, collapsed
/// until the reader asks for it. Highlighting is a small hand-rolled
/// tokenizer rather than a dependency, and every colour comes from the
/// catalog theme dictionaries so it reads in both variants.
#[repr(C)]
pub struct CodeBlock {
    base: TemplatedControl,
    text: RefCell<Option<Ref<SelectableTextBlock>>>,
    copy_button: RefCell<Option<Ref<Button>>>,
    copy_click: RefCell<Option<RoutedEventHandlerToken>>,
    copy_reset: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(CodeBlock: TemplatedControl);
ferro_class_info!(CodeBlock {
    new: CodeBlock::new,
    markup: {
        attributes: [
            TemplatePart("PART_Text", type(Ref<SelectableTextBlock>)),
            TemplatePart("PART_CopyButton", type(Ref<Button>)),
        ],
    },
});
ferro_impl_classes!(CodeBlock: StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for CodeBlock {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::code_property().as_property() {
            let code = change.get_new_value::<Option<String>>();
            if let Some(code) = code.filter(|code| !code.trim().is_empty()) {
                let lines = Self::dedent(&code).split('\n').count();
                this.set_current_value(Self::is_expanded_property(), lines <= COLLAPSE_THRESHOLD);
            }

            this.update_inlines();
        } else if change.property() == Self::language_property().as_property() {
            this.update_inlines();
        }
    }
}

impl VisualImpl for CodeBlock {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        let copy_reset = this.copy_reset.take();
        if let Some(copy_reset) = copy_reset {
            copy_reset.dispose();
        }

        let copy_button = this.copy_button.borrow().clone();
        if let Some(copy_button) = copy_button {
            copy_button.set_content(Some(text_content("Copy")));
        }
    }
}

impl TemplatedControlImpl for CodeBlock {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let previous = this.copy_button.borrow().clone();
        if let (Some(previous), Some(token)) = (previous, this.copy_click.take()) {
            previous.remove_handler(Button::click_event(), token);
        }

        *this.text.borrow_mut() = e.name_scope().find_as::<SelectableTextBlock>(PART_TEXT);
        let copy_button = e.name_scope().find_as::<Button>(PART_COPY_BUTTON);
        *this.copy_button.borrow_mut() = copy_button.clone();

        if let Some(copy_button) = copy_button {
            // The button is a child of this control: its handler holds the control weakly.
            let weak = this.to_ref().downgrade();
            let token = copy_button.click(move |sender: &Interactive, e: &RoutedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_copy_click(sender, e);
                }
            });
            *this.copy_click.borrow_mut() = Some(token);
        }

        this.update_inlines();
    }
}

ferro_properties! {
    impl CodeBlock {
        pub fn code_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<CodeBlock, _>("Code", None)
        }

        pub fn language_property() -> StyledProperty<CodeLanguage> {
            FerroProperty::register::<CodeBlock, _>("Language", CodeLanguage::Xaml)
        }

        pub fn is_expanded_property() -> StyledProperty<bool> {
            FerroProperty::register::<CodeBlock, _>("IsExpanded", true)
        }
    }
}

fn text_content(text: &str) -> BoxedValue {
    Rc::new(text.to_string())
}

/// `char.IsLetter`.
fn is_letter(c: char) -> bool {
    c.is_alphabetic()
}

/// `char.IsLetterOrDigit`.
fn is_letter_or_digit(c: char) -> bool {
    c.is_alphanumeric()
}

/// The class `\w` of a regular expression.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The match of the expression `^(\s*)<[\w:.]+\s+\S` at the start of
/// `line`: the length of its first group.
fn match_opening_tag(line: &[char]) -> Option<usize> {
    let mut i = 0;
    while i < line.len() && line[i].is_whitespace() {
        i += 1;
    }
    let lead = i;
    if i >= line.len() || line[i] != '<' {
        return None;
    }
    i += 1;
    let name_start = i;
    while i < line.len() && (is_word(line[i]) || line[i] == ':' || line[i] == '.') {
        i += 1;
    }
    if i == name_start {
        return None;
    }
    let space_start = i;
    while i < line.len() && line[i].is_whitespace() {
        i += 1;
    }
    // `\s+\S`: at least one white space character, then a character that is not one.
    if i == space_start || i >= line.len() {
        return None;
    }
    Some(lead)
}

/// Whether the expression `^\s*([\w.:]+\s*=|/?>)` matches at the start of
/// `line`.
fn is_attribute_line(line: &[char]) -> bool {
    let mut i = 0;
    while i < line.len() && line[i].is_whitespace() {
        i += 1;
    }

    // The white space before the alternatives may also be shorter than the run of white
    // space, but neither alternative starts with white space, so only the full run can match.
    let name_start = i;
    let mut j = i;
    while j < line.len() && (is_word(line[j]) || line[j] == '.' || line[j] == ':') {
        j += 1;
    }
    if j > name_start {
        // `[\w.:]+\s*=`: a name of any shorter length is followed by a name character,
        // which is neither white space nor `=`.
        let mut k = j;
        while k < line.len() && line[k].is_whitespace() {
            k += 1;
        }
        if k < line.len() && line[k] == '=' {
            return true;
        }
    }

    if i < line.len() && line[i] == '/' {
        i += 1;
    }
    i < line.len() && line[i] == '>'
}

/// `code.IndexOf(value, start)` for a sequence of characters.
fn index_of(code: &[char], value: &[char], start: usize) -> Option<usize> {
    if start > code.len() {
        return None;
    }
    code[start..].windows(value.len()).position(|window| window == value).map(|index| index + start)
}

fn starts_with(code: &[char], start: usize, value: &str) -> bool {
    let mut index = start;
    for c in value.chars() {
        if index >= code.len() || code[index] != c {
            return false;
        }
        index += 1;
    }
    true
}

fn text_of(code: &[char]) -> String {
    code.iter().collect()
}

const CSHARP_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "base", "bool", "break", "byte", "case", "catch", "char", "class", "const",
    "continue", "decimal", "default", "delegate", "do", "double", "else", "enum", "event", "explicit", "false",
    "finally", "float", "for", "foreach", "get", "if", "implicit", "in", "int", "interface", "internal", "is", "lock",
    "long", "namespace", "new", "null", "object", "operator", "out", "override", "params", "private", "protected",
    "public", "readonly", "ref", "return", "sealed", "set", "short", "static", "string", "struct", "switch", "this",
    "throw", "true", "try", "typeof", "uint", "ulong", "ushort", "using", "var", "virtual", "void", "while", "yield",
];

/// A span of a snippet with the theme resource key that colours it.
pub(crate) type Token = (String, Option<&'static str>);

fn flush(tokens: &mut Vec<Token>, run: &mut String) {
    if !run.is_empty() {
        tokens.push((std::mem::take(run), None));
    }
}

impl CodeBlock {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            text: RefCell::new(None),
            copy_button: RefCell::new(None),
            copy_click: RefCell::new(None),
            copy_reset: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The snippet. Leading indentation common to every line is trimmed
    /// when it is rendered.
    pub fn code(&self) -> Option<String> {
        self.get_value(Self::code_property())
    }

    pub fn set_code(&self, value: Option<&str>) {
        self.set_value(Self::code_property(), value.map(str::to_string))
    }

    pub fn language(&self) -> CodeLanguage {
        self.get_value(Self::language_property())
    }

    pub fn set_language(&self, value: CodeLanguage) {
        self.set_value(Self::language_property(), value)
    }

    /// Whether the snippet is showing. Short snippets start open; ones
    /// longer than the collapse threshold start collapsed.
    pub fn is_expanded(&self) -> bool {
        self.get_value(Self::is_expanded_property())
    }

    pub fn set_is_expanded(&self, value: bool) {
        self.set_value(Self::is_expanded_property(), value)
    }

    /// The handler of the click of the copy button (`async void`).
    fn on_copy_click(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let visual: &Visual = self;
        let Some(clipboard) = TopLevel::get_top_level(Some(visual)).and_then(|top_level| top_level.clipboard()) else {
            return;
        };
        let Some(code) = self.code() else {
            return;
        };

        let pending = clipboard.set_text_async(Some(&self.format(&code)));
        let this = self.to_ref();
        let _ = start_async(async move {
            if let Err(error) = pending.await {
                // An exception of an `async void` method is raised on the dispatcher.
                panic!("{error}");
            }

            let Some(button) = this.copy_button.borrow().clone() else {
                return;
            };

            button.set_content(Some(text_content("Copied")));
            let previous = this.copy_reset.take();
            if let Some(previous) = previous {
                previous.dispose();
            }
            let reset = DispatcherTimer::run_once(
                move || button.set_content(Some(text_content("Copy"))),
                Duration::from_secs_f64(1.5),
                DispatcherPriority::DEFAULT,
            );
            *this.copy_reset.borrow_mut() = Some(reset);
        });
    }

    fn update_inlines(&self) {
        let Some(text) = self.text.borrow().clone() else {
            return;
        };

        if let Some(inlines) = text.inlines() {
            inlines.clear();
        }

        let Some(code) = self.code().filter(|code| !code.trim().is_empty()) else {
            return;
        };

        for (span, kind) in Self::tokenize(&self.format(&code), self.language()) {
            let run = Run::with_text(Some(&span));
            if let Some(kind) = kind {
                let resource = DynamicResourceExtension::with_resource_key(Some(text_content(kind)));
                run.bind_binding(TextElement::foreground_property().as_property(), &*resource.as_binding());
            }

            if let Some(inlines) = text.inlines() {
                inlines.add(run);
            }
        }
    }

    fn format(&self, code: &str) -> String {
        if self.language() == CodeLanguage::Xaml {
            Self::realign(&Self::dedent(code))
        } else {
            Self::dedent(code)
        }
    }

    /// Removes the indentation every line shares, so a snippet written
    /// inside deeply nested XAML does not render with that nesting.
    pub(crate) fn dedent(code: &str) -> String {
        let code = code.replace("\r\n", "\n");
        let lines: Vec<&str> = code.trim_matches('\n').split('\n').collect();
        let mut common = usize::MAX;

        for line in &lines {
            if line.trim().is_empty() {
                continue;
            }

            let indent = line.chars().count() - line.trim_start_matches(' ').chars().count();
            common = common.min(indent);
        }

        if common == 0 || common == usize::MAX {
            return lines.join("\n").trim_end().to_string();
        }

        let mut builder = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                builder.push('\n');
            }

            if line.chars().count() >= common {
                builder.extend(line.chars().skip(common));
            } else {
                builder.push_str(line.trim_start_matches(' '));
            }
        }

        builder.trim_end().to_string()
    }

    /// Aligns wrapped attributes under the first attribute of the element
    /// they belong to. Snippets are authored inside CDATA blocks, where the
    /// opening line sits at column zero but the continuation lines keep the
    /// indentation of the file they were written in, so the two no longer
    /// line up. Anything that is not a plain attribute continuation keeps
    /// the indentation it has.
    pub(crate) fn realign(code: &str) -> String {
        let mut lines: Vec<String> = code.split('\n').map(str::to_string).collect();
        let mut anchor: Option<usize> = None;

        for line in &mut lines {
            if line.trim().is_empty() {
                continue;
            }

            let chars: Vec<char> = line.chars().collect();
            if let Some(lead) = match_opening_tag(&chars) {
                // Column just past "<Tag ", measured on the line as it will be rendered.
                anchor = chars[lead..].iter().position(|c| *c == ' ').map(|index| lead + index + 1);
                continue;
            }

            if let Some(anchor) = anchor {
                if is_attribute_line(&chars) {
                    *line = " ".repeat(anchor) + line.trim_start_matches(' ');
                    continue;
                }
            }

            anchor = None;
        }

        lines.join("\n")
    }

    /// Splits a snippet into spans paired with the theme resource key that
    /// colours them. No key means the block's own foreground.
    pub(crate) fn tokenize(code: &str, language: CodeLanguage) -> Vec<Token> {
        if language == CodeLanguage::CSharp {
            Self::tokenize_csharp(code)
        } else {
            Self::tokenize_xaml(code)
        }
    }

    fn tokenize_xaml(code: &str) -> Vec<Token> {
        let code: Vec<char> = code.chars().collect();
        let mut tokens = Vec::new();
        let mut i = 0;
        let mut run = String::new();

        while i < code.len() {
            if starts_with(&code, i, "<!--") {
                flush(&mut tokens, &mut run);
                let end = index_of(&code, &['-', '-', '>'], i).map_or(code.len(), |end| end + 3);
                tokens.push((text_of(&code[i..end]), Some("CatalogCodeComment")));
                i = end;
                continue;
            }

            if code[i] == '<' {
                flush(&mut tokens, &mut run);
                let mut j = i + 1;
                if j < code.len() && code[j] == '/' {
                    j += 1;
                }

                while j < code.len() && (is_letter_or_digit(code[j]) || matches!(code[j], ':' | '.' | '_' | '-' | '/')) {
                    j += 1;
                }

                tokens.push((text_of(&code[i..j]), Some("CatalogCodeKeyword")));
                i = j;
                continue;
            }

            if code[i] == '"' {
                flush(&mut tokens, &mut run);
                let end = index_of(&code, &['"'], i + 1).map_or(code.len(), |end| end + 1);
                tokens.push((text_of(&code[i..end]), Some("CatalogCodeString")));
                i = end;
                continue;
            }

            if matches!(code[i], '>' | '/') && (i + 1 >= code.len() || code[i] == '>' || code[i + 1] == '>') {
                flush(&mut tokens, &mut run);
                let len = if code[i] == '/' && i + 1 < code.len() { 2 } else { 1 };
                tokens.push((text_of(&code[i..i + len]), Some("CatalogCodeKeyword")));
                i += len;
                continue;
            }

            if is_letter(code[i]) || code[i] == '_' {
                let mut j = i;
                while j < code.len() && (is_letter_or_digit(code[j]) || matches!(code[j], '.' | ':' | '_' | '-')) {
                    j += 1;
                }

                let word = text_of(&code[i..j]);
                let mut k = j;
                while k < code.len() && code[k] == ' ' {
                    k += 1;
                }

                if k < code.len() && code[k] == '=' {
                    flush(&mut tokens, &mut run);
                    tokens.push((word, Some("CatalogCodeAttribute")));
                    i = j;
                    continue;
                }

                run.push_str(&word);
                i = j;
                continue;
            }

            run.push(code[i]);
            i += 1;
        }

        flush(&mut tokens, &mut run);
        tokens
    }

    fn tokenize_csharp(code: &str) -> Vec<Token> {
        let code: Vec<char> = code.chars().collect();
        let mut tokens = Vec::new();
        let mut i = 0;
        let mut run = String::new();

        while i < code.len() {
            if starts_with(&code, i, "//") {
                flush(&mut tokens, &mut run);
                let end = index_of(&code, &['\n'], i).unwrap_or(code.len());
                tokens.push((text_of(&code[i..end]), Some("CatalogCodeComment")));
                i = end;
                continue;
            }

            if matches!(code[i], '"' | '\'') {
                flush(&mut tokens, &mut run);
                let quote = code[i];
                let mut j = i + 1;
                while j < code.len() && code[j] != quote {
                    j += if code[j] == '\\' { 2 } else { 1 };
                }

                j = (j + 1).min(code.len());
                tokens.push((text_of(&code[i..j]), Some("CatalogCodeString")));
                i = j;
                continue;
            }

            if is_letter(code[i]) || code[i] == '_' {
                let mut j = i;
                while j < code.len() && (is_letter_or_digit(code[j]) || code[j] == '_') {
                    j += 1;
                }

                let word = text_of(&code[i..j]);
                if CSHARP_KEYWORDS.contains(&word.as_str()) {
                    flush(&mut tokens, &mut run);
                    tokens.push((word, Some("CatalogCodeKeyword")));
                } else {
                    run.push_str(&word);
                }

                i = j;
                continue;
            }

            run.push(code[i]);
            i += 1;
        }

        flush(&mut tokens, &mut run);
        tokens
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    fn token(text: &str, key: Option<&'static str>) -> Token {
        (text.to_string(), key)
    }

    #[test]
    fn dedent_removes_the_shared_indentation() {
        assert_eq!("<A>\n  <B />\n</A>", CodeBlock::dedent("\r\n    <A>\r\n      <B />\r\n    </A>\r\n"));
        assert_eq!("a\n\n b", CodeBlock::dedent("  a\n\n   b  \n"));
        assert_eq!("a\n  b", CodeBlock::dedent("a\n  b\n"));
        assert_eq!("", CodeBlock::dedent("   \n  "));
    }

    #[test]
    fn realign_puts_wrapped_attributes_under_the_first_one() {
        let code = "<Button Content=\"A\"\n            Width=\"10\"\n  />\n<TextBlock />\n    Text";
        assert_eq!(
            "<Button Content=\"A\"\n        Width=\"10\"\n        />\n<TextBlock />\n    Text",
            CodeBlock::realign(code)
        );
        // An element without attributes on its line does not anchor the lines after it.
        assert_eq!("<Grid>\n      Width=\"1\"", CodeBlock::realign("<Grid>\n      Width=\"1\""));
    }

    #[test]
    fn tokenize_xaml_classifies_tags_attributes_strings_and_comments() {
        assert_eq!(
            vec![
                token("<!-- c -->", Some("CatalogCodeComment")),
                token("\n", None),
                token("<Button", Some("CatalogCodeKeyword")),
                token(" ", None),
                token("Content", Some("CatalogCodeAttribute")),
                token("=", None),
                token("\"A b\"", Some("CatalogCodeString")),
                token(" ", None),
                token("/>", Some("CatalogCodeKeyword")),
                token("text", None),
                token("</Button", Some("CatalogCodeKeyword")),
                token(">", Some("CatalogCodeKeyword")),
            ],
            CodeBlock::tokenize("<!-- c -->\n<Button Content=\"A b\" />text</Button>", CodeLanguage::Xaml)
        );
    }

    #[test]
    fn tokenize_csharp_classifies_keywords_strings_and_comments() {
        assert_eq!(
            vec![
                token("var", Some("CatalogCodeKeyword")),
                token(" x = ", None),
                token("new", Some("CatalogCodeKeyword")),
                token(" Foo(", None),
                token("\"a\\\"b\"", Some("CatalogCodeString")),
                token("); ", None),
                token("// done", Some("CatalogCodeComment")),
                token("\n", None),
                token("'c'", Some("CatalogCodeString")),
            ],
            CodeBlock::tokenize("var x = new Foo(\"a\\\"b\"); // done\n'c'", CodeLanguage::CSharp)
        );
    }

    #[test]
    fn a_long_snippet_starts_collapsed() {
        let block = CodeBlock::new();
        assert!(block.is_expanded());
        block.set_code(Some(&vec!["<A />"; 13].join("\n")));
        assert!(!block.is_expanded());
        block.set_code(Some("<A />"));
        assert!(block.is_expanded());
    }
}
