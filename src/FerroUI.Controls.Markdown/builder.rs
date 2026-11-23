//! Turns the events of the Markdown parser into blocks of the rich text document.
//!
//! The text is untrusted, so nothing here recurses over its structure: the open elements are
//! kept on explicit stacks, spans nest only to [`MAX_INLINE_DEPTH`] (a deeper span adds its text
//! to the one around it, because layout walks nested spans recursively), and indentation and the
//! size of a table are bounded.

use crate::markdown_text_block::MarkdownImageResolver;
use ferroui_base::layout::Thickness;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, FontFamily, FontWeight, IBrush, TextDecorations, TextWrapping};
use ferroui_base::utilities::HandlerList;
use ferroui_base::Ref;
use ferroui_controls::documents::{
    Block, Bold, Hyperlink, HyperlinkClickEventArgs, InlineCollection, InlineUIContainer, Italic, LineBreak,
    Paragraph, Run, Span,
};
use ferroui_controls::{Border, ColumnDefinition, Control, Grid, GridLength, Image, RichTextBlock, RowDefinition};
use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::rc::Rc;

/// How deep spans nest; a deeper one adds its text to the span around it.
pub(crate) const MAX_INLINE_DEPTH: usize = 16;
/// How many levels of lists and quotes indent the text; deeper ones keep this indentation.
pub(crate) const MAX_INDENT_DEPTH: usize = 12;
/// The columns of a table that are shown.
pub(crate) const MAX_TABLE_COLUMNS: usize = 64;
/// The rows of a table that are shown; a last line says how many were left out.
pub(crate) const MAX_TABLE_ROWS: usize = 2000;

const INDENT: f64 = 24.0;
const BLOCK_SPACING: f64 = 12.0;
const CODE_FONT: &str = "Cascadia Code, Consolas, Menlo, monospace";
/// Readable on the light and the dark themes.
const SUBTLE: Color = Color::from_argb(0xFF, 0x80, 0x80, 0x80);
const CODE_BACKGROUND: Color = Color::from_argb(0x28, 0x80, 0x80, 0x80);

/// What the document is built with.
pub(crate) struct BuildOptions {
    pub image_resolver: Option<MarkdownImageResolver>,
    pub link_handlers: Rc<HandlerList<dyn Fn(&HyperlinkClickEventArgs)>>,
}

/// The built document.
pub(crate) struct Built {
    pub blocks: Vec<Ref<Block>>,
    pub rules: Vec<Ref<Border>>,
}

/// Whether a link may open its address: a relative address, which only an application's handler
/// can use, or one of the schemes that open a page or a mail. Others (`file:`, `javascript:`, an
/// application's own scheme) stay text, because the Markdown may come from anyone.
pub(crate) fn is_safe_link(address: &str) -> bool {
    let address = address.trim_start();
    let scheme_end = address.find(|c: char| matches!(c, ':' | '/' | '?' | '#'));
    match scheme_end {
        Some(end) if address.as_bytes()[end] == b':' => {
            let scheme = &address[..end];
            ["http", "https", "mailto"].iter().any(|safe| scheme.eq_ignore_ascii_case(safe))
        }
        _ => true,
    }
}

fn brush(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

fn code_font() -> Option<FontFamily> {
    FontFamily::parse(CODE_FONT).ok()
}

fn heading_size(level: HeadingLevel) -> f64 {
    // The type ramp of WinUI: title large to body.
    match level {
        HeadingLevel::H1 => 28.0,
        HeadingLevel::H2 => 24.0,
        HeadingLevel::H3 => 20.0,
        HeadingLevel::H4 => 18.0,
        HeadingLevel::H5 => 16.0,
        HeadingLevel::H6 => 14.0,
    }
}

/// A table while its cells are read.
struct TableState {
    alignments: Vec<Alignment>,
    rows: Vec<Vec<Ref<Paragraph>>>,
    row: Vec<Ref<Paragraph>>,
    in_head: bool,
    head_rows: usize,
    left_out_rows: usize,
}

struct Builder<'a> {
    options: &'a BuildOptions,
    blocks: Vec<Ref<Block>>,
    rules: Vec<Ref<Border>>,
    paragraph: Option<Ref<Paragraph>>,
    /// The open spans; `None` for one past [`MAX_INLINE_DEPTH`].
    spans: Vec<Option<Ref<Span>>>,
    /// The open lists: the next number of an ordered list.
    lists: Vec<Option<u64>>,
    quote_depth: usize,
    /// The marker of a list item whose first paragraph has not started.
    item_open: bool,
    code: Option<String>,
    /// The alternative text of the images being read, innermost last.
    images: Vec<(String, String)>,
    table: Option<TableState>,
    cell: Option<Ref<Paragraph>>,
}

/// Builds the blocks of `markdown`.
pub(crate) fn build(markdown: &str, options: &BuildOptions) -> Built {
    let parser_options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut builder = Builder {
        options,
        blocks: Vec::new(),
        rules: Vec::new(),
        paragraph: None,
        spans: Vec::new(),
        lists: Vec::new(),
        quote_depth: 0,
        item_open: false,
        code: None,
        images: Vec::new(),
        table: None,
        cell: None,
    };
    for event in Parser::new_ext(markdown, parser_options) {
        builder.event(event);
    }
    builder.close_paragraph();
    Built { blocks: builder.blocks, rules: builder.rules }
}

impl Builder<'_> {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text),
            // HTML is shown as the text it is, and never interpreted.
            Event::Html(text) | Event::InlineHtml(text) => self.text(&text),
            Event::Code(text) => self.inline_code(&text),
            Event::SoftBreak => self.text(" "),
            Event::HardBreak => {
                if self.images.is_empty() {
                    self.inlines().add(LineBreak::new());
                }
            }
            Event::Rule => self.rule(),
            Event::TaskListMarker(checked) => self.text(if checked { "\u{2611} " } else { "\u{2610} " }),
            // The options that produce the others are not turned on.
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {
                if self.cell.is_none() && !self.item_open {
                    self.close_paragraph();
                }
                self.item_open = false;
            }
            Tag::Heading { level, .. } => {
                self.close_paragraph();
                let paragraph = self.open_paragraph();
                paragraph.set_font_size(heading_size(level));
                paragraph.set_font_weight(FontWeight::SemiBold);
            }
            Tag::BlockQuote(_) => {
                self.close_paragraph();
                self.quote_depth += 1;
            }
            Tag::CodeBlock(_) => {
                self.close_paragraph();
                self.code = Some(String::new());
            }
            Tag::List(first) => {
                self.close_paragraph();
                self.item_open = false;
                self.lists.push(first);
            }
            Tag::Item => {
                self.close_paragraph();
                let marker = match self.lists.last_mut() {
                    Some(Some(number)) => {
                        let marker = format!("{number}. ");
                        *number = number.saturating_add(1);
                        marker
                    }
                    _ => "\u{2022} ".to_string(),
                };
                self.open_paragraph().inlines().add(Run::with_text(Some(&marker)));
                self.item_open = true;
            }
            Tag::Table(alignments) => {
                self.close_paragraph();
                self.table = Some(TableState {
                    alignments,
                    rows: Vec::new(),
                    row: Vec::new(),
                    in_head: false,
                    head_rows: 0,
                    left_out_rows: 0,
                });
            }
            Tag::TableHead => {
                if let Some(table) = &mut self.table {
                    table.in_head = true;
                }
            }
            Tag::TableCell => {
                let paragraph = Paragraph::new();
                if self.table.as_ref().is_some_and(|table| table.in_head) {
                    paragraph.set_font_weight(FontWeight::SemiBold);
                }
                self.cell = Some(paragraph);
            }
            Tag::Emphasis => self.push_span(|| Italic::new().upcast()),
            Tag::Strong => self.push_span(|| Bold::new().upcast()),
            Tag::Strikethrough => self.push_span(|| {
                let span = Span::new();
                span.set_text_decorations(Some(TextDecorations::strikethrough()));
                span
            }),
            Tag::Link { dest_url, .. } => {
                let address = dest_url.into_string();
                let handlers = self.options.link_handlers.clone();
                self.push_span(move || {
                    if !is_safe_link(&address) {
                        return Span::new();
                    }
                    let link = Hyperlink::new();
                    link.set_navigate_uri(Some(address));
                    link.click(move |args| {
                        for (_, handler) in handlers.snapshot().iter() {
                            handler(args);
                        }
                    })
                    .detach();
                    link.upcast()
                });
            }
            Tag::Image { dest_url, .. } => self.images.push((dest_url.into_string(), String::new())),
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) => {
                if self.cell.is_none() {
                    self.close_paragraph();
                }
            }
            TagEnd::BlockQuote(_) => {
                self.close_paragraph();
                self.quote_depth = self.quote_depth.saturating_sub(1);
            }
            TagEnd::CodeBlock => self.close_code_block(),
            TagEnd::List(_) => {
                self.close_paragraph();
                self.lists.pop();
            }
            TagEnd::Item => {
                self.close_paragraph();
                self.item_open = false;
            }
            TagEnd::Table => self.close_table(),
            TagEnd::TableHead => {
                if let Some(table) = &mut self.table {
                    table.in_head = false;
                    Self::close_row(table);
                    table.head_rows = table.rows.len();
                }
            }
            TagEnd::TableRow => {
                if let Some(table) = &mut self.table {
                    Self::close_row(table);
                }
            }
            TagEnd::TableCell => {
                if let (Some(table), Some(cell)) = (&mut self.table, self.cell.take()) {
                    table.row.push(cell);
                }
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                self.spans.pop();
            }
            TagEnd::Image => self.close_image(),
            _ => {}
        }
    }

    fn close_row(table: &mut TableState) {
        let row = std::mem::take(&mut table.row);
        if table.rows.len() < MAX_TABLE_ROWS {
            table.rows.push(row);
        } else {
            table.left_out_rows += 1;
        }
    }

    /// The indentation of a block at the current depth of lists and quotes.
    fn indent(&self) -> f64 {
        (self.lists.len() + self.quote_depth).min(MAX_INDENT_DEPTH) as f64 * INDENT
    }

    fn open_paragraph(&mut self) -> Ref<Paragraph> {
        if let Some(paragraph) = &self.paragraph {
            return paragraph.clone();
        }
        let paragraph = Paragraph::new();
        paragraph.set_left_indent(self.indent());
        paragraph.set_margin(Thickness::new(0.0, 0.0, 0.0, BLOCK_SPACING));
        if self.quote_depth > 0 {
            paragraph.set_foreground(Some(brush(SUBTLE)));
        }
        self.paragraph = Some(paragraph.clone());
        paragraph
    }

    fn close_paragraph(&mut self) {
        self.spans.clear();
        if let Some(paragraph) = self.paragraph.take() {
            self.blocks.push(paragraph.upcast());
        }
    }

    /// Where inline content goes now.
    fn inlines(&mut self) -> InlineCollection {
        if let Some(span) = self.spans.iter().rev().flatten().next() {
            return span.inlines();
        }
        if let Some(cell) = &self.cell {
            return cell.inlines();
        }
        self.open_paragraph().inlines()
    }

    fn push_span(&mut self, create: impl FnOnce() -> Ref<Span>) {
        if !self.images.is_empty() || self.spans.len() >= MAX_INLINE_DEPTH {
            self.spans.push(None);
            return;
        }
        let span = create();
        self.inlines().add(span.clone());
        self.spans.push(Some(span));
    }

    fn text(&mut self, text: &str) {
        if let Some(code) = &mut self.code {
            code.push_str(text);
        } else if let Some((_, alternative)) = self.images.last_mut() {
            alternative.push_str(text);
        } else {
            self.inlines().add(Run::with_text(Some(text)));
        }
    }

    fn inline_code(&mut self, text: &str) {
        if let Some((_, alternative)) = self.images.last_mut() {
            alternative.push_str(text);
            return;
        }
        let run = Run::with_text(Some(text));
        if let Some(family) = code_font() {
            run.set_font_family(family);
        }
        run.set_background(Some(brush(CODE_BACKGROUND)));
        self.inlines().add(run);
    }

    fn close_code_block(&mut self) {
        let Some(code) = self.code.take() else { return };
        let paragraph = self.open_paragraph();
        if let Some(family) = code_font() {
            paragraph.set_font_family(family);
        }
        let background = brush(CODE_BACKGROUND);
        let inlines = paragraph.inlines();
        for (index, line) in code.strip_suffix('\n').unwrap_or(&code).split('\n').enumerate() {
            if index != 0 {
                inlines.add(LineBreak::new());
            }
            let run = Run::with_text(Some(line));
            run.set_background(Some(background.clone()));
            inlines.add(run);
        }
        self.close_paragraph();
    }

    fn close_image(&mut self) {
        let Some((address, alternative)) = self.images.pop() else { return };
        if !self.images.is_empty() {
            // An image inside the description of an image is part of that description.
            if let Some((_, outer)) = self.images.last_mut() {
                outer.push_str(&alternative);
            }
            return;
        }
        let source = self.options.image_resolver.as_ref().and_then(|resolve| resolve(&address));
        match source {
            Some(source) => {
                let image = Image::new();
                image.set_source(Some(source));
                ferroui_controls::automation::AutomationProperties::set_name(&image, Some(alternative.as_str()));
                self.inlines().add(InlineUIContainer::with_child(image));
            }
            None if !alternative.is_empty() => self.text(&alternative),
            None => {}
        }
    }

    fn rule(&mut self) {
        self.close_paragraph();
        let rule = Border::new();
        rule.set_height(1.0);
        rule.set_background(Some(brush(SUBTLE)));
        let paragraph = self.open_paragraph();
        paragraph.inlines().add(InlineUIContainer::with_child(rule.clone()));
        self.rules.push(rule);
        self.close_paragraph();
    }

    fn close_table(&mut self) {
        let Some(table) = self.table.take() else { return };
        self.cell = None;
        let columns = table.rows.iter().map(Vec::len).max().unwrap_or(0).min(MAX_TABLE_COLUMNS);
        if columns == 0 {
            return;
        }
        let grid = Grid::new();
        for _ in 0..columns {
            grid.column_definitions().add(ColumnDefinition::with_width(GridLength::AUTO));
        }
        for _ in 0..table.rows.len() {
            grid.row_definitions().add(RowDefinition::with_height(GridLength::AUTO));
        }
        for (row_index, row) in table.rows.into_iter().enumerate() {
            for (column, paragraph) in row.into_iter().take(columns).enumerate() {
                if let Some(alignment) = table.alignments.get(column).and_then(|alignment| match alignment {
                    Alignment::None => None,
                    Alignment::Left => Some(ferroui_base::media::TextAlignment::Left),
                    Alignment::Center => Some(ferroui_base::media::TextAlignment::Center),
                    Alignment::Right => Some(ferroui_base::media::TextAlignment::Right),
                }) {
                    paragraph.set_text_alignment(alignment);
                }
                let cell = RichTextBlock::new();
                cell.set_text_wrapping(TextWrapping::NoWrap);
                cell.set_padding(Thickness::new(8.0, 4.0, 8.0, 4.0));
                cell.blocks().add(paragraph);
                let control: Ref<Control> = cell.upcast();
                Grid::set_column(&control, column as i32);
                Grid::set_row(&control, row_index as i32);
                grid.children().add(control);
            }
        }
        let paragraph = self.open_paragraph();
        paragraph.inlines().add(InlineUIContainer::with_child(grid));
        self.close_paragraph();
        if table.left_out_rows > 0 {
            let note = self.open_paragraph();
            note.set_foreground(Some(brush(SUBTLE)));
            note.inlines().add(Run::with_text(Some(&format!("{} more rows", table.left_out_rows))));
            self.close_paragraph();
        }
    }
}
