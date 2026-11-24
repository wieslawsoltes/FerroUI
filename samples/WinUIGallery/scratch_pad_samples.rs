//! The code behind of the pinned ScratchPad page in Rust: `ScratchPadPage.xaml.cs` with its
//! `XamlTextFormatter`. The editor is a native `RichEditBox` over UTF-16 offsets, as the source's
//! text document is, and Load goes through the WinUI XAML importer.
//!
//! Three deviations from the source: the default markup has a Border around its StackPanel (see
//! [`DEFAULT_XAML`]), unindenting removes only the leading whitespace of a line (the source counts
//! whitespace anywhere in its first four characters and can delete text), and a selection never
//! moves before the start of its line.

use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, Colors, IBrush, TextWrapping};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, Ref, WeakRef};
use ferroui_controls::documents::CharacterFormat;
use ferroui_controls::rich_text::{RichEditTextDocument, TextGetOptions};
use ferroui_controls::{
    Button, ContentDialog, ContentDialogButton, ContentDialogResult, Control, RichEditBox, ScrollViewer, TextBlock,
};
use ferroui_markup_xaml_winui::{migrate_winui_xaml, parse_winui_as, WinUIXamlOptions};
use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

const LOAD_SUCCESSFUL: &str = "Load successful.";
const WINUI_NAMESPACES: &str = " xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation' \
xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

/// `GetDefaultScratchXAML`. The source's root is a StackPanel with a border and padding, which a
/// FerroUI panel does not have and the importer does not rewrite: here they are a Border around it.
pub(crate) const DEFAULT_XAML: &str = "<Border BorderThickness=\"1\" BorderBrush=\"Green\" CornerRadius=\"4\" Padding=\"3\">\n    <StackPanel>\n        <!-- Note: {x:Bind} is not supported in Scratch Pad. -->\n        <TextBlock>This is a sample TextBlock.</TextBlock>\n        <Button Content=\"Click me!\"/>\n\n        <!-- Note: Syntax highlighting updates on 'Load'. -->\n    </StackPanel>\n</Border>";

/// `ZoneType` of `XamlTextFormatter`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Zone {
    Unknown,
    OpenTag,
    EndTag,
    TagName,
    PropertyName,
    PropertyValue,
    Whitespace,
    Comment,
    Content,
}

fn is_whitespace(unit: u16) -> bool {
    char::from_u32(u32::from(unit)).is_some_and(char::is_whitespace)
}

fn is_eol(unit: u16) -> bool {
    unit == u16::from(b'\r') || unit == u16::from(b'\n')
}

fn find_from(text: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    if from > text.len() {
        return None;
    }
    text[from..].windows(needle.len()).position(|window| window == needle).map(|index| index + from)
}

/// `LastIndexOf`: the last index at or before `from` whose unit matches.
fn rfind(text: &[u16], from: usize, matches: impl Fn(u16) -> bool) -> Option<usize> {
    if text.is_empty() {
        return None;
    }
    let last = from.min(text.len() - 1);
    (0..=last).rev().find(|&index| matches(text[index]))
}

/// `ApplyColors`: the zones of the text, in UTF-16 offsets. As in the source, the zone still open
/// at the end of the text is not reported.
pub(crate) fn zones(text: &str) -> Vec<(Range<usize>, Zone)> {
    let text: Vec<u16> = text.encode_utf16().collect();
    let unit = |c: char| c as u16;
    let mut result: Vec<(Range<usize>, Zone)> = Vec::new();
    let mut start = 0;
    let mut zone = Zone::Unknown;
    let mut in_tag = false;
    let mut index = 0;

    macro_rules! begin {
        ($next:expr) => {{
            if start < index {
                // The source starts a new zone at every character of content; one range of the
                // same colour formats the same text with far fewer edits.
                match result.last_mut() {
                    Some((range, last)) if *last == zone && range.end == start => range.end = index,
                    _ => result.push((start..index, zone)),
                }
            }
            start = index;
            zone = $next;
        }};
    }

    while index < text.len() {
        let current = text[index];
        if is_whitespace(current) {
            if zone != Zone::Whitespace {
                begin!(Zone::Whitespace);
            }
        } else if current == unit('<') {
            let comment: Vec<u16> = "<!--".encode_utf16().collect();
            if text[index..].starts_with(&comment) {
                begin!(Zone::Comment);
                index += 3;
                let end: Vec<u16> = "-->".encode_utf16().collect();
                if let Some(end) = find_from(&text, &end, index + 1) {
                    index = end + 2;
                }
            } else {
                begin!(Zone::OpenTag);
                in_tag = true;
            }
        } else if current == unit('/') {
            // Either "</" at the start of a tag or "/>" at the end of one.
            if zone != Zone::OpenTag && zone != Zone::EndTag {
                begin!(Zone::EndTag);
                in_tag = false;
            }
        } else if current == unit('>') {
            if zone != Zone::EndTag {
                begin!(Zone::EndTag);
                in_tag = false;
            }
        } else if zone == Zone::OpenTag {
            begin!(Zone::TagName);
        } else if zone == Zone::PropertyName && current == unit('=') {
            begin!(Zone::PropertyValue);
            if index + 2 < text.len() && text[index + 1] == unit('"') {
                if let Some(end) = find_from(&text, &[unit('"')], index + 2) {
                    index = end;
                }
            }
        } else if zone != Zone::TagName && zone != Zone::PropertyName && zone != Zone::PropertyValue && in_tag {
            begin!(Zone::PropertyName);
        } else if !in_tag {
            begin!(Zone::Content);
        }
        index += 1;
    }
    result
}

/// The colour of a zone (`UpdateZone`).
pub(crate) fn zone_color(zone: Zone, light: bool) -> Color {
    let (on_light, on_dark) = match zone {
        Zone::OpenTag | Zone::EndTag => (Colors::BLUE, Colors::GRAY),
        Zone::TagName => (Colors::BROWN, Colors::WHITE),
        Zone::PropertyName => (Colors::RED, Colors::LIGHT_SKY_BLUE),
        Zone::PropertyValue => (Colors::BLUE, Colors::DODGER_BLUE),
        Zone::Comment => (Colors::GREEN, Colors::LIME_GREEN),
        Zone::Whitespace | Zone::Content | Zone::Unknown => (Colors::BLACK, Colors::WHITE),
    };
    if light {
        on_light
    } else {
        on_dark
    }
}

/// `AddXmlNamespace`: the namespaces on the root element; `None` without an end of its name.
pub(crate) fn add_xml_namespace(xml: &str) -> Option<String> {
    let xml = xml.trim();
    let insert = xml.find([' ', '/', '>'])?;
    Some(format!("{}{}{}", &xml[..insert], WINUI_NAMESPACES, &xml[insert..]))
}

/// `GetTextboxTextPreviousLine`: the line before the one of `selection`.
pub(crate) fn previous_line(text: &[u16], selection: usize) -> Vec<u16> {
    if selection == 0 {
        return Vec::new();
    }
    match rfind(text, selection - 1, is_eol) {
        Some(end) if end > 0 => {
            let start = rfind(text, end - 1, is_eol).map_or(0, |index| index + 1);
            text[start..end].to_vec()
        }
        _ => Vec::new(),
    }
}

/// What `textbox_TextChanged` adds after a typed character.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AutoInsert {
    /// The close tag of the tag that `>` ended, with the caret before it.
    CloseTag(String),
    /// A pair of quotes after `=`, with the caret between them.
    Quotes,
}

/// The insertion for a character just typed before `selection`.
pub(crate) fn auto_insert(text: &[u16], selection: usize) -> Option<AutoInsert> {
    let unit = |c: char| c as u16;
    if selection == 0 || selection > text.len() {
        return None;
    }
    let typed = text[selection - 1];
    if selection >= 2 && typed == unit('>') && text[selection - 2] != unit('/') {
        let tag_start = rfind(text, selection - 1, |c| c == unit('<'))?;
        let closed_before = rfind(text, selection - 2, |c| c == unit('>'));
        if closed_before.is_some_and(|index| index >= tag_start) {
            return None;
        }
        let name = &text[tag_start + 1..];
        let end = name.iter().position(|&c| {
            c == unit(' ') || c == unit('/') || c == unit('>') || c == unit('\t') || is_eol(c)
        })?;
        if end == 0 {
            return None;
        }
        let name = String::from_utf16_lossy(&name[..end]);
        return (name != "!--").then(|| AutoInsert::CloseTag(format!("</{name}>")));
    }
    if typed == unit('=') && (selection + 1 >= text.len() || text[selection] != unit('"')) {
        let last_tag = rfind(text, selection, |c| c == unit('<') || c == unit('>'))?;
        if text[last_tag] != unit('<') {
            return None;
        }
        // Not inside a property value: the last quote does not follow an '='.
        let in_value = rfind(text, selection, |c| c == unit('"'))
            .is_some_and(|quote| quote >= last_tag && quote > 0 && text[quote - 1] == unit('='));
        return (!in_value).then_some(AutoInsert::Quotes);
    }
    None
}

/// An edit of the lines of a selection, and the selection after all of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IndentEdit {
    /// In document order: the offset of the line and how many units it removes (0 to indent).
    pub(crate) lines: Vec<(usize, usize)>,
    pub(crate) selection: Range<usize>,
}

/// The Tab handling of `textbox_KeyDown`: indents or unindents the lines of the selection.
pub(crate) fn indent_edit(text: &[u16], selection: Range<usize>, unindent: bool) -> IndentEdit {
    let first = rfind(text, selection.start, is_eol).map_or(0, |index| index + 1);
    let mut starts = vec![first];
    let mut index = first;
    while let Some(eol) = (index..text.len()).find(|&at| is_eol(text[at])) {
        // "\r\n" is one paragraph break.
        let next = if text[eol] == u16::from(b'\r') && text.get(eol + 1) == Some(&u16::from(b'\n')) {
            eol + 2
        } else {
            eol + 1
        };
        if next >= selection.end || next >= text.len() {
            break;
        }
        starts.push(next);
        index = next;
    }

    if !unindent {
        return IndentEdit {
            selection: selection.start + 4..selection.end + 4 * starts.len(),
            lines: starts.into_iter().map(|start| (start, 0)).collect(),
        };
    }
    let lines: Vec<(usize, usize)> = starts
        .into_iter()
        .map(|start| {
            let count = text[start..]
                .iter()
                .take(4)
                .take_while(|&&c| is_whitespace(c) && !is_eol(c))
                .count();
            (start, count)
        })
        .collect();
    let removed: usize = lines.iter().map(|(_, count)| count).sum();
    let start = selection.start.saturating_sub(lines[0].1).max(first);
    IndentEdit { selection: start..selection.end.saturating_sub(removed).max(start), lines }
}

/// The controls and the state of `ScratchPadPage`. The handlers hold it; it holds the controls
/// weakly, so that they do not keep the page alive.
struct ScratchPad {
    root: WeakRef<Control>,
    textbox: WeakRef<RichEditBox>,
    host: WeakRef<ScrollViewer>,
    status: WeakRef<TextBlock>,
    last_change_from_typing: Cell<bool>,
    old_text: RefCell<String>,
}

/// The offsets of the selection of `document`.
fn selection_span(document: &RichEditTextDocument) -> Range<usize> {
    document.selection().range().map_or(0..0, |range| range.range())
}

impl ScratchPad {
    fn document_text(textbox: &RichEditBox) -> String {
        textbox.text_document().get_text(TextGetOptions::NONE).unwrap_or_default()
    }

    /// `XamlTextFormatter.ApplyColors`, in one undo group.
    fn apply_colors(&self) {
        let Some(textbox) = self.textbox.upgrade() else {
            return;
        };
        let light = textbox.actual_theme_variant() != Some(ThemeVariant::dark());
        let text = Self::document_text(&textbox);
        let document = textbox.text_document();
        // A range the document refuses keeps its colour; the text is not affected.
        drop(document.edit_group(|document| {
            for (range, zone) in zones(&text) {
                let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(zone_color(zone, light)));
                document.get_range(range.start, range.end)?.set_character_format(&CharacterFormat {
                    foreground: Some(Some(brush)),
                    ..CharacterFormat::default()
                })?;
            }
            Ok(())
        }));
    }

    /// `SetEmptyScratchPadContent`.
    fn set_empty_content(&self) {
        let Some(host) = self.host.upgrade() else {
            return;
        };
        let text = TextBlock::new();
        text.set_text(Some("Click the Load button to load the content below."));
        text.set_horizontal_alignment(HorizontalAlignment::Center);
        text.set_vertical_alignment(VerticalAlignment::Center);
        text.set_text_wrapping(TextWrapping::Wrap);
        host.set_content(Some(Control::boxed(text)));
    }

    fn set_text(&self, text: &str) {
        let Some(textbox) = self.textbox.upgrade() else {
            return;
        };
        self.last_change_from_typing.set(false);
        *self.old_text.borrow_mut() = text.to_owned();
        // The text of the page's own constant or of the editor: a read-only editor keeps its text.
        drop(textbox.text_document().set_text(text));
    }

    fn options(&self) -> WinUIXamlOptions {
        self.root
            .upgrade()
            .and_then(|root| root.tag())
            .and_then(|tag| tag.downcast_ref::<WinUIXamlOptions>().cloned())
            .unwrap_or_default()
    }

    /// `LoadContent`.
    fn load_content(&self) {
        let (Some(textbox), Some(host), Some(status)) =
            (self.textbox.upgrade(), self.host.upgrade(), self.status.upgrade())
        else {
            return;
        };
        status.set_text(Some(""));
        let text = Self::document_text(&textbox);
        let options = self.options();
        match add_xml_namespace(&text) {
            None => status.set_text(Some("No end tag.")),
            Some(xml) => match parse_winui_as::<Ref<Control>>(&xml, None, &options) {
                Ok(element) => {
                    host.set_content(Some(Control::boxed(element)));
                    let mut message = LOAD_SUCCESSFUL.to_owned();
                    if let Ok(migration) = migrate_winui_xaml(&xml, &options) {
                        for diagnostic in migration.diagnostics {
                            message.push_str(&format!(
                                "\n{} at {}:{}: {}",
                                diagnostic.code, diagnostic.span.line, diagnostic.span.column, diagnostic.message
                            ));
                        }
                    }
                    status.set_text(Some(&message));
                }
                Err(error) => status.set_text(Some(&error.to_string())),
            },
        }
        status.set_opacity(1.0);
    }

    /// `LoadContentAndApplyFormatting`.
    fn load_and_format(&self) {
        self.load_content();
        self.last_change_from_typing.set(false);
        self.apply_colors();
    }

    /// `InsertTextboxText`.
    fn insert_text(&self, text: &str, cursor_after: bool) {
        let Some(textbox) = self.textbox.upgrade() else {
            return;
        };
        let document = textbox.text_document();
        let start = selection_span(&document).start;
        self.last_change_from_typing.set(false);
        let Ok(range) = document.get_range(start, start) else {
            return;
        };
        if range.set_text(text).is_err() {
            return;
        }
        let caret = start + if cursor_after { text.encode_utf16().count() } else { 0 };
        // The caret stays where the edit left it if the document refuses the move.
        drop(document.selection().set_range(caret, caret));
    }

    /// `HandleEnter`: the indent of the previous line, and an end tag moved to its own line.
    fn handle_enter(&self) {
        let Some(textbox) = self.textbox.upgrade() else {
            return;
        };
        let document = textbox.text_document();
        // A refused edit leaves the text as typed.
        drop(document.edit_group(|document| {
            let text: Vec<u16> = Self::document_text(&textbox).encode_utf16().collect();
            let previous = previous_line(&text, selection_span(document).start);
            let indent = previous.iter().take_while(|&&c| is_whitespace(c)).count();
            let indent = String::from_utf16_lossy(&previous[..indent]);
            self.insert_text(&indent, true);

            let start = selection_span(document).start;
            let end = (start + 2).min(document.text_length());
            if document.get_range(start, end)?.text()? == "</" {
                self.insert_text("    ", true);
                let start = selection_span(document).start;
                document.get_range(start, start)?.set_text(&format!("\n{indent}"))?;
            }
            Ok(())
        }));
    }

    /// The Tab case of `textbox_KeyDown`.
    fn handle_tab(&self, e: &KeyEventArgs) {
        let Some(textbox) = self.textbox.upgrade() else {
            return;
        };
        let document = textbox.text_document();
        let selection = selection_span(&document);
        if selection.len() <= 1 {
            return;
        }
        let text: Vec<u16> = Self::document_text(&textbox).encode_utf16().collect();
        let unindent = e.key_modifiers.contains(KeyModifiers::SHIFT);
        let edit = indent_edit(&text, selection, unindent);
        self.last_change_from_typing.set(false);
        // From the last line up, so that the offsets of the others stay valid.
        let applied = document.edit_group(|document| {
            for &(start, remove) in edit.lines.iter().rev() {
                if unindent {
                    document.get_range(start, start + remove)?.set_text("")?;
                } else {
                    document.get_range(start, start)?.set_text("    ")?;
                }
            }
            document.selection().set_range(edit.selection.start, edit.selection.end)
        });
        // A read-only editor keeps Tab for focus navigation.
        if applied.is_ok() {
            e.set_handled(true);
        }
    }

    /// `textbox_TextChanged`.
    fn on_text_changed(&self) {
        let Some(textbox) = self.textbox.upgrade() else {
            return;
        };
        let document = textbox.text_document();
        let selection = selection_span(&document);
        if selection.is_empty() && self.last_change_from_typing.get() {
            if let Some(status) = self.status.upgrade() {
                if status.text().is_some_and(|text| text.starts_with(LOAD_SUCCESSFUL)) {
                    status.set_text(Some(""));
                } else {
                    // Dim the message, which is now old.
                    status.set_opacity(0.5);
                }
            }
            let new_text: Vec<u16> = Self::document_text(&textbox).encode_utf16().collect();
            let old_length = self.old_text.borrow().encode_utf16().count();
            if new_text.len() == old_length + 1 {
                match auto_insert(&new_text, selection.start) {
                    Some(AutoInsert::CloseTag(tag)) => self.insert_text(&tag, false),
                    Some(AutoInsert::Quotes) => {
                        self.insert_text("\"\"", false);
                        let caret = selection_span(&document).start + 1;
                        // The quotes stay; only the caret would not move between them.
                        drop(document.selection().set_range(caret, caret));
                    }
                    None => {}
                }
            }
        }
        *self.old_text.borrow_mut() = Self::document_text(&textbox);
    }

    /// `ResetToDefaultClick`.
    fn confirm_reset(this: &Rc<Self>, button: &Ref<Button>) {
        let dialog = ContentDialog::new();
        dialog.set_title(Some(Rc::new("Are you sure you want to reset?".to_string()) as BoxedValue));
        dialog.set_content(Some(Rc::new(
            "Resetting to the default content will replace your current content. Are you sure you want to continue?"
                .to_string(),
        ) as BoxedValue));
        dialog.set_primary_button_text(Some("Reset"));
        dialog.set_close_button_text(Some("Cancel"));
        dialog.set_default_button(ContentDialogButton::Primary);
        dialog.set_value(ThemeVariant::requested_theme_variant_property(), button.actual_theme_variant());
        let this = this.clone();
        dialog
            .closed(move |args| {
                if args.result() == ContentDialogResult::Primary {
                    this.reset();
                }
            })
            .detach();
        // A second click while a dialog is open does nothing.
        drop(dialog.show_async(button));
    }

    fn reset(&self) {
        self.set_text(DEFAULT_XAML);
        self.apply_colors();
        self.set_empty_content();
        if let Some(status) = self.status.upgrade() {
            status.set_text(Some(""));
        }
    }
}

pub(crate) fn wire(root: &Ref<Control>) {
    let Some(textbox) = root.find_control::<RichEditBox>("textbox") else {
        return;
    };
    let page = Rc::new(ScratchPad {
        root: root.downgrade(),
        textbox: textbox.downgrade(),
        host: root.get_control::<ScrollViewer>("scratchPad").downgrade(),
        status: root.get_control::<TextBlock>("loadStatus").downgrade(),
        last_change_from_typing: Cell::new(false),
        old_text: RefCell::new(String::new()),
    });
    page.set_text(DEFAULT_XAML);
    page.apply_colors();
    page.set_empty_content();
    *page.old_text.borrow_mut() = ScratchPad::document_text(&textbox);

    {
        let page = page.clone();
        textbox.add_handler_with(
            InputElement::key_down_event(),
            move |_, e: &KeyEventArgs| {
                page.last_change_from_typing.set(true);
                if e.key == Key::Tab {
                    page.handle_tab(e);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }
    {
        let page = page.clone();
        textbox.add_handler_with(
            InputElement::key_up_event(),
            move |_, e: &KeyEventArgs| {
                page.last_change_from_typing.set(true);
                match e.key {
                    Key::F5 => page.load_and_format(),
                    Key::Enter => page.handle_enter(),
                    _ => {}
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
    }
    {
        let page = page.clone();
        textbox.text_changed(move |_, _| page.on_text_changed());
    }
    {
        let page = page.clone();
        textbox
            .actual_theme_variant_changed(move || page.apply_colors())
            .detach();
    }
    {
        let page = page.clone();
        root.get_control::<Button>("loadButton").click(move |_, _| page.load_and_format());
    }
    let reset = root.get_control::<Button>("resetButton");
    let weak_reset = reset.downgrade();
    reset.click(move |_, _| {
        if let Some(button) = weak_reset.upgrade() {
            ScratchPad::confirm_reset(&page, &button);
        }
    });
}
