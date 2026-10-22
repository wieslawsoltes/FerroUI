use super::rich_text::{check_range, checked_end};
use super::{
    HighlightingColor, IHighlighter, RichText, RichTextError, RichTextModel, RichTextModelWriter,
    RichTextWriter,
};
use crate::document::{DocumentError, SimpleSegment, TextDocument};

/// Options for HTML fragments. Tabs expand to a fixed number of nonbreaking spaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlOptions {
    pub tab_size: usize,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self { tab_size: 4 }
    }
}

impl HtmlOptions {
    pub fn new() -> Self {
        Self::default()
    }

    fn validate(&self) -> Result<(), RichTextError> {
        if self.tab_size == 0 {
            return Err(RichTextError::InvalidOption(
                "HTML tab size must be positive",
            ));
        }
        Ok(())
    }

    pub fn color_needs_span_for_styling(
        &self,
        color: &HighlightingColor,
    ) -> Result<bool, RichTextError> {
        Ok(!color.to_css()?.is_empty())
    }

    pub fn write_style_attribute_for_color(
        &self,
        output: &mut String,
        color: &HighlightingColor,
    ) -> Result<(), RichTextError> {
        let css = color.to_css()?;
        output.push_str(" style=\"");
        push_escaped(output, &css);
        output.push('"');
        Ok(())
    }
}

impl HighlightingColor {
    /// Produces inline CSS from supported formatting properties.
    ///
    /// Colors accept alphabetic names, hexadecimal notation, and numeric CSS
    /// color functions. Declaration delimiters, URLs, variables, and nested
    /// expressions are rejected rather than copied into a style attribute.
    /// Font sizes are finite positive pixel values; color names are metadata.
    pub fn to_css(&self) -> Result<String, RichTextError> {
        let mut css = String::new();
        for (property, value) in [
            ("color", &self.foreground),
            ("background-color", &self.background),
        ] {
            if let Some(value) = value {
                let color = checked_css_color(value, property)?;
                css.push_str(property);
                css.push_str(": ");
                css.push_str(color);
                css.push_str("; ");
            }
        }
        if let Some(size) = self.font_size {
            if !size.is_finite() || size <= 0.0 {
                return Err(RichTextError::InvalidCssValue {
                    property: "font-size",
                    value: size.to_string(),
                });
            }
            css.push_str(&format!("font-size: {size}px; "));
        }
        if let Some(bold) = self.bold {
            css.push_str(if bold {
                "font-weight: bold; "
            } else {
                "font-weight: normal; "
            });
        }
        if let Some(italic) = self.italic {
            css.push_str(if italic {
                "font-style: italic; "
            } else {
                "font-style: normal; "
            });
        }
        if let Some(underline) = self.underline {
            css.push_str(if underline {
                "text-decoration: underline; "
            } else {
                "text-decoration: none; "
            });
        }
        Ok(css)
    }
}

fn checked_css_color<'a>(value: &'a str, property: &'static str) -> Result<&'a str, RichTextError> {
    let color = value.trim();
    let valid = if let Some(hex) = color.strip_prefix('#') {
        matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    } else if !color.is_empty() && color.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        true
    } else if let Some((function, arguments)) = color.split_once('(') {
        let known = matches!(
            function.to_ascii_lowercase().as_str(),
            "rgb" | "rgba" | "hsl" | "hsla" | "hwb" | "lab" | "lch" | "oklab" | "oklch" | "color"
        );
        known
            && arguments.strip_suffix(')').is_some_and(|args| {
                !args.is_empty()
                    && args.bytes().any(|byte| byte.is_ascii_digit())
                    && args.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric()
                            || matches!(byte, b' ' | b'.' | b',' | b'%' | b'+' | b'-' | b'/')
                    })
            })
    } else {
        false
    };
    if valid && !value.chars().any(char::is_control) {
        Ok(color)
    } else {
        Err(RichTextError::InvalidCssValue {
            property,
            value: value.to_owned(),
        })
    }
}

fn push_escaped_char(output: &mut String, value: char) {
    match value {
        '&' => output.push_str("&amp;"),
        '<' => output.push_str("&lt;"),
        '>' => output.push_str("&gt;"),
        '"' => output.push_str("&quot;"),
        '\'' => output.push_str("&#39;"),
        '\0' => output.push('\u{fffd}'),
        value => output.push(value),
    }
}

fn push_escaped(output: &mut String, value: &str) {
    for character in value.chars() {
        push_escaped_char(output, character);
    }
}

fn validate_hyperlink(uri: &str) -> Result<(), RichTextError> {
    let unsafe_character = uri
        .chars()
        .any(|character| character.is_control() || character.is_whitespace() || character == '\\');
    let prefix = uri.split(['/', '?', '#']).next().unwrap_or("");
    let supported_scheme = prefix.split_once(':').is_none_or(|(scheme, _)| {
        matches!(
            scheme.to_ascii_lowercase().as_str(),
            "http" | "https" | "mailto" | "ftp" | "tel"
        )
    });
    if unsafe_character || !supported_scheme {
        return Err(RichTextError::InvalidHyperlink(uri.to_owned()));
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct HyperlinkRun {
    start: usize,
    end: usize,
    uri: String,
}

#[derive(Clone, Debug)]
enum HtmlSpan {
    Color,
    Hyperlink(Option<String>),
}

/// Buffered HTML writer that renders flattened effective formatting.
///
/// A surrogate pair split across writes or formatting boundaries remains one
/// character, using the format of its leading code unit. A lone surrogate is
/// replaced only when producing the UTF-8 HTML result. Flat spans also allow an
/// inner format to clear an outer underline, which nested CSS cannot do.
#[derive(Clone, Debug)]
pub struct HtmlRichTextWriter {
    writer: RichTextModelWriter,
    options: HtmlOptions,
    spans: Vec<HtmlSpan>,
    current_link: Option<String>,
    links: Vec<HyperlinkRun>,
}

impl Default for HtmlRichTextWriter {
    fn default() -> Self {
        Self {
            writer: RichTextModelWriter::new(),
            options: HtmlOptions::default(),
            spans: Vec::new(),
            current_link: None,
            links: Vec::new(),
        }
    }
}

impl HtmlRichTextWriter {
    pub fn new(options: HtmlOptions) -> Result<Self, RichTextError> {
        options.validate()?;
        Ok(Self {
            options,
            ..Default::default()
        })
    }

    pub fn set_indentation_string(&mut self, value: &str) -> Result<(), RichTextError> {
        self.writer.set_indentation_string(value)
    }

    pub fn finish(self) -> Result<String, RichTextError> {
        if !self.spans.is_empty() {
            return Err(RichTextError::UnclosedSpans(self.spans.len()));
        }
        render_html(&self.writer.finish()?, &self.options, &self.links)
    }
}

impl RichTextWriter for HtmlRichTextWriter {
    fn write_utf16(&mut self, text: &[u16]) -> Result<(), RichTextError> {
        let start = self.writer.len();
        self.writer.write_utf16(text)?;
        let end = self.writer.len();
        if let Some(uri) = self.current_link.as_ref().filter(|_| start != end) {
            if let Some(last) = self
                .links
                .last_mut()
                .filter(|last| last.end == start && last.uri == *uri)
            {
                last.end = end;
            } else {
                self.links.push(HyperlinkRun {
                    start,
                    end,
                    uri: uri.clone(),
                });
            }
        }
        Ok(())
    }

    fn begin_span(&mut self, color: &HighlightingColor) -> Result<(), RichTextError> {
        color.to_css()?;
        self.writer.begin_span(color)?;
        self.spans.push(HtmlSpan::Color);
        Ok(())
    }

    fn begin_hyperlink_span(&mut self, uri: &str) -> Result<(), RichTextError> {
        validate_hyperlink(uri)?;
        self.writer.begin_span(&HighlightingColor::default())?;
        self.spans.push(HtmlSpan::Hyperlink(
            self.current_link.replace(uri.to_owned()),
        ));
        Ok(())
    }

    fn end_span(&mut self) -> Result<(), RichTextError> {
        let span = self.spans.pop().ok_or(RichTextError::SpanUnderflow)?;
        self.writer.end_span()?;
        if let HtmlSpan::Hyperlink(previous) = span {
            self.current_link = previous;
        }
        Ok(())
    }

    fn indent(&mut self) -> Result<(), RichTextError> {
        self.writer.indent()
    }

    fn unindent(&mut self) -> Result<(), RichTextError> {
        self.writer.unindent()
    }
}

fn render_html(
    text: &RichText,
    options: &HtmlOptions,
    links: &[HyperlinkRun],
) -> Result<String, RichTextError> {
    let sections = text.get_highlighted_sections(0, text.len())?;
    let styles = sections
        .iter()
        .map(|section| section.color.to_css())
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = String::new();
    let mut decoded = char::decode_utf16(text.text_utf16().iter().copied()).peekable();
    let mut position = 0;
    let mut section_index = 0;
    let mut link_index = 0;
    let mut current_style = "";
    let mut current_link: Option<&str> = None;
    let mut previous_whitespace = true;
    while let Some(character) = decoded.next() {
        let mut width = character
            .as_ref()
            .map_or(1, |character| character.len_utf16());
        let character = character.unwrap_or('\u{fffd}');
        while sections[section_index].offset + sections[section_index].length <= position {
            section_index += 1;
        }
        while link_index < links.len() && links[link_index].end <= position {
            link_index += 1;
        }
        let link = links
            .get(link_index)
            .filter(|link| link.start <= position)
            .map(|link| link.uri.as_str());
        if link != current_link {
            if !current_style.is_empty() {
                output.push_str("</span>");
                current_style = "";
            }
            if current_link.is_some() {
                output.push_str("</a>");
            }
            if let Some(link) = link {
                output.push_str("<a href=\"");
                push_escaped(&mut output, link);
                output.push_str("\">");
            }
            current_link = link;
        }
        let style = styles[section_index].as_str();
        if style != current_style {
            if !current_style.is_empty() {
                output.push_str("</span>");
            }
            if !style.is_empty() {
                output.push_str("<span style=\"");
                push_escaped(&mut output, style);
                output.push_str("\">");
            }
            current_style = style;
        }
        match character {
            ' ' => {
                let next_whitespace = decoded
                    .peek()
                    .is_none_or(|next| next.as_ref().is_ok_and(|next| next.is_whitespace()));
                output.push_str(if previous_whitespace || next_whitespace {
                    "&nbsp;"
                } else {
                    " "
                });
            }
            '\t' => {
                for _ in 0..options.tab_size {
                    output.push_str("&nbsp;");
                }
            }
            '\r' => {
                if decoded.peek().is_some_and(|next| *next == Ok('\n')) {
                    decoded.next();
                    width += 1;
                }
                output.push_str("<br/>");
            }
            '\n' => output.push_str("<br/>"),
            value => push_escaped_char(&mut output, value),
        }
        previous_whitespace = character.is_whitespace();
        position += width;
    }
    if !current_style.is_empty() {
        output.push_str("</span>");
    }
    if current_link.is_some() {
        output.push_str("</a>");
    }
    Ok(output)
}

/// Creates HTML fragments and CF_HTML clipboard payloads without accessing an OS clipboard.
pub struct HtmlClipboard;

impl HtmlClipboard {
    /// Exports exactly the selected UTF-16 range, including blank and trailing lines.
    /// CR, LF, and CRLF each become one HTML line break. A supplied highlighter
    /// must belong to the same shared document handle.
    pub fn create_html_fragment(
        document: &TextDocument,
        highlighter: Option<&dyn IHighlighter>,
        segment: Option<SimpleSegment>,
        options: &HtmlOptions,
    ) -> Result<String, RichTextError> {
        options.validate()?;
        if highlighter.is_some_and(|highlighter| highlighter.document() != document) {
            return Err(DocumentError::DifferentDocument.into());
        }
        let segment = segment.unwrap_or(SimpleSegment::new(0, document.text_length()));
        let end = check_range(segment.offset, segment.length, document.text_length())?;
        let text = document.get_text_utf16(segment.offset, segment.length)?;
        let mut model = RichTextModel::new();
        if let Some(highlighter) = highlighter.filter(|_| segment.length != 0) {
            let mut number = document.get_line_by_offset(segment.offset)?.line_number;
            while number <= document.line_count() {
                let line = document.get_line_by_number(number)?;
                if line.offset >= end {
                    break;
                }
                let highlighted = highlighter.highlight_line(number)?;
                highlighted.validate_invariants(document)?;
                for section in highlighted.sections {
                    let start = section.offset.max(segment.offset);
                    let section_end = (section.offset + section.length).min(end);
                    if start < section_end {
                        model.apply_highlighting(
                            start - segment.offset,
                            section_end - start,
                            &section.color,
                        )?;
                    }
                }
                number += 1;
            }
        }
        RichText::from_utf16_with_model(&text, &model).to_html(options)
    }

    /// Wraps an already-generated HTML fragment in CF_HTML, with UTF-8 byte offsets.
    /// The fragment is HTML, so it is not escaped a second time.
    pub fn create_cf_html(fragment: &str) -> Result<String, RichTextError> {
        const PREFIX: &str = "<!DOCTYPE html>\r\n<html><head><meta charset=\"utf-8\"></head><body>\r\n<!--StartFragment-->";
        const SUFFIX: &str = "<!--EndFragment-->\r\n</body></html>";
        let start_html = build_header(0, 0, 0, 0).len();
        let start_fragment = checked_end(start_html, PREFIX.len())?;
        let end_fragment = checked_end(start_fragment, fragment.len())?;
        let end_html = checked_end(end_fragment, SUFFIX.len())?;
        if end_html as u64 > 9_999_999_999 {
            return Err(RichTextError::InvalidOption(
                "CF_HTML offsets exceed ten decimal digits",
            ));
        }
        let mut output = build_header(start_html, end_html, start_fragment, end_fragment);
        output.push_str(PREFIX);
        output.push_str(fragment);
        output.push_str(SUFFIX);
        Ok(output)
    }

    pub fn create_cf_html_for_document(
        document: &TextDocument,
        highlighter: Option<&dyn IHighlighter>,
        segment: Option<SimpleSegment>,
        options: &HtmlOptions,
    ) -> Result<String, RichTextError> {
        Self::create_cf_html(&Self::create_html_fragment(
            document,
            highlighter,
            segment,
            options,
        )?)
    }
}

fn build_header(
    start_html: usize,
    end_html: usize,
    start_fragment: usize,
    end_fragment: usize,
) -> String {
    format!(
        "Version:0.9\r\nStartHTML:{start_html:010}\r\nEndHTML:{end_html:010}\r\nStartFragment:{start_fragment:010}\r\nEndFragment:{end_fragment:010}\r\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlighting::{DocumentHighlighter, HighlightingDefinition, HighlightingEngine};
    use std::rc::Rc;

    fn test_highlighter(document: &TextDocument) -> DocumentHighlighter {
        let definition = HighlightingDefinition::load(
            r##"<SyntaxDefinition name="Test"><RuleSet>
            <Keywords foreground="#008000" fontWeight="bold"><Word>using</Word></Keywords>
            <Keywords foreground="#ff0000"><Word>string</Word></Keywords>
            <Rule foreground="#191970" fontWeight="bold">SomeMethod</Rule>
        </RuleSet></SyntaxDefinition>"##,
        )
        .unwrap();
        DocumentHighlighter::new(
            document.clone(),
            HighlightingEngine::new(Rc::new(definition)),
        )
    }

    #[test]
    fn document_fragment_keeps_keyword_styles_and_partial_highlighted_words() {
        let document = TextDocument::from_text("using System.Text;\n\tstring text = SomeMethod();");
        let highlighter = test_highlighter(&document);
        let html = HtmlClipboard::create_html_fragment(
            &document,
            Some(&highlighter),
            None,
            &HtmlOptions::default(),
        )
        .unwrap();
        assert_eq!(
            html,
            "<span style=\"color: #008000; font-weight: bold; \">using</span> System.Text;<br/>&nbsp;&nbsp;&nbsp;&nbsp;<span style=\"color: #ff0000; \">string</span> text = <span style=\"color: #191970; font-weight: bold; \">SomeMethod</span>();"
        );
        let part = HtmlClipboard::create_html_fragment(
            &document,
            Some(&highlighter),
            Some(SimpleSegment::new(1, 3)),
            &HtmlOptions::default(),
        )
        .unwrap();
        assert_eq!(
            part,
            "<span style=\"color: #008000; font-weight: bold; \">sin</span>"
        );
    }

    #[test]
    fn text_html_metacharacters_are_escaped() {
        let html = RichText::new("<tag a=\"x\">'&</tag>")
            .to_html(&HtmlOptions::default())
            .unwrap();
        assert_eq!(html, "&lt;tag a=&quot;x&quot;&gt;&#39;&amp;&lt;/tag&gt;");
        let mut writer = HtmlRichTextWriter::default();
        writer
            .begin_span(&HighlightingColor {
                name: Some("\"><script>".into()),
                ..Default::default()
            })
            .unwrap();
        writer.write("safe").unwrap();
        writer.end_span().unwrap();
        assert_eq!(writer.finish().unwrap(), "safe");
    }

    #[test]
    fn css_rejects_injected_declarations_and_unsafe_functions() {
        for value in [
            "red; background-image: url(https://example.test)",
            "red\" onclick=\"x",
            "url(//example.test)",
            "var(--color)",
            "rgb(1, 2, 3); color: red",
            "rgb(1, calc(2), 3)",
            "red\n",
            "#zzzzzz",
        ] {
            let color = HighlightingColor {
                foreground: Some(value.into()),
                ..Default::default()
            };
            assert!(
                matches!(color.to_css(), Err(RichTextError::InvalidCssValue { .. })),
                "{value}"
            );
            let mut writer = HtmlRichTextWriter::default();
            assert!(writer.begin_span(&color).is_err());
            assert_eq!(writer.finish().unwrap(), "");
        }
        for value in [
            "DarkSlateGray",
            "#ABC",
            "#1234",
            "#12ab34",
            "#12ab34ff",
            "rgb(1, 2, 3)",
            "rgba(1 2 3 / 0.5)",
            "hsl(90deg 50% 20%)",
            "oklch(50% 0.1 20)",
            "color(display-p3 1 0 0)",
        ] {
            assert!(
                HighlightingColor {
                    foreground: Some(value.into()),
                    ..Default::default()
                }
                .to_css()
                .is_ok(),
                "{value}"
            );
        }
        for size in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(HighlightingColor {
                font_size: Some(size),
                ..Default::default()
            }
            .to_css()
            .is_err());
        }
        let color = HighlightingColor {
            background: Some("#abc".into()),
            font_size: Some(12.5),
            ..Default::default()
        };
        assert_eq!(
            color.to_css().unwrap(),
            "background-color: #abc; font-size: 12.5px; "
        );
    }

    #[test]
    fn whitespace_blank_lines_and_trailing_delimiters_are_preserved() {
        let document = TextDocument::from_text("\r\n a  b \rc\n\n\t\r\n");
        let html = HtmlClipboard::create_html_fragment(
            &document,
            None,
            None,
            &HtmlOptions { tab_size: 2 },
        )
        .unwrap();
        assert_eq!(
            html,
            "<br/>&nbsp;a&nbsp;&nbsp;b&nbsp;<br/>c<br/><br/>&nbsp;&nbsp;<br/>"
        );
        let plain = RichText::new("a b")
            .to_html(&HtmlOptions::default())
            .unwrap();
        assert_eq!(plain, "a b");
        assert!(HtmlRichTextWriter::new(HtmlOptions { tab_size: 0 }).is_err());
    }

    #[test]
    fn crlf_and_surrogate_pairs_can_cross_writes_and_style_boundaries() {
        let mut writer = HtmlRichTextWriter::default();
        writer.begin_foreground_span("red").unwrap();
        writer.write_utf16(&[0xd83d]).unwrap();
        writer.end_span().unwrap();
        writer.begin_foreground_span("blue").unwrap();
        writer.write_utf16(&[0xde00, 13]).unwrap();
        writer.end_span().unwrap();
        writer.write_utf16(&[10, 0xd800]).unwrap();
        assert_eq!(
            writer.finish().unwrap(),
            "<span style=\"color: red; \">😀</span><span style=\"color: blue; \"><br/></span>�"
        );
    }

    #[test]
    fn nested_false_properties_render_as_independent_effective_spans() {
        let mut writer = HtmlRichTextWriter::default();
        writer
            .begin_span(&HighlightingColor {
                foreground: Some("red".into()),
                underline: Some(true),
                ..Default::default()
            })
            .unwrap();
        writer.write("a").unwrap();
        writer
            .begin_span(&HighlightingColor {
                underline: Some(false),
                ..Default::default()
            })
            .unwrap();
        writer.write("b").unwrap();
        writer.end_span().unwrap();
        writer.write("c").unwrap();
        writer.end_span().unwrap();
        assert_eq!(
            writer.finish().unwrap(),
            "<span style=\"color: red; text-decoration: underline; \">a</span><span style=\"color: red; text-decoration: none; \">b</span><span style=\"color: red; text-decoration: underline; \">c</span>"
        );
    }

    #[test]
    fn links_escape_attributes_and_restore_nested_link_context() {
        let mut writer = HtmlRichTextWriter::default();
        writer
            .begin_hyperlink_span("https://example.test/?q=\"x\"&a=1")
            .unwrap();
        writer.write("a").unwrap();
        writer.begin_hyperlink_span("#part").unwrap();
        writer.write("b").unwrap();
        writer.end_span().unwrap();
        writer.write("c").unwrap();
        writer.end_span().unwrap();
        assert_eq!(
            writer.finish().unwrap(),
            "<a href=\"https://example.test/?q=&quot;x&quot;&amp;a=1\">a</a><a href=\"#part\">b</a><a href=\"https://example.test/?q=&quot;x&quot;&amp;a=1\">c</a>"
        );
        for uri in [
            "javascript:alert(1)",
            "data:text/html,test",
            "java\nscript:alert(1)",
            " javascript:alert(1)",
        ] {
            assert!(HtmlRichTextWriter::default()
                .begin_hyperlink_span(uri)
                .is_err());
        }
    }

    #[test]
    fn document_range_validation_and_highlighter_identity_are_explicit() {
        let document = TextDocument::from_text("using 😀\r\n");
        let highlighter = test_highlighter(&document);
        let other = TextDocument::from_text(&document.text());
        assert!(matches!(
            HtmlClipboard::create_html_fragment(
                &other,
                Some(&highlighter),
                None,
                &HtmlOptions::default()
            ),
            Err(RichTextError::Document(DocumentError::DifferentDocument))
        ));
        assert!(HtmlClipboard::create_html_fragment(
            &document,
            None,
            Some(SimpleSegment::new(usize::MAX, 2)),
            &HtmlOptions::default()
        )
        .is_err());
        let part = HtmlClipboard::create_html_fragment(
            &document,
            None,
            Some(SimpleSegment::new(6, 2)),
            &HtmlOptions::default(),
        )
        .unwrap();
        assert_eq!(part, "😀");
        let partial_pair = HtmlClipboard::create_html_fragment(
            &document,
            None,
            Some(SimpleSegment::new(7, 1)),
            &HtmlOptions::default(),
        )
        .unwrap();
        assert_eq!(partial_pair, "�");
        assert_eq!(
            HtmlClipboard::create_html_fragment(
                &document,
                None,
                Some(SimpleSegment::new(document.text_length(), 0)),
                &HtmlOptions::default()
            )
            .unwrap(),
            ""
        );
    }

    #[test]
    fn cf_html_offsets_count_utf8_bytes_and_select_exact_fragment() {
        let fragment = "<span>é😀&amp;漢</span>";
        let payload = HtmlClipboard::create_cf_html(fragment).unwrap();
        let field = |name: &str| -> usize {
            payload
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .unwrap()
                .parse()
                .unwrap()
        };
        let start_html = field("StartHTML:");
        let end_html = field("EndHTML:");
        let start = field("StartFragment:");
        let end = field("EndFragment:");
        assert_eq!(&payload[start..end], fragment);
        assert_eq!(end - start, fragment.len());
        assert!(end - start > fragment.chars().count());
        assert!(payload[start_html..end_html].starts_with("<!DOCTYPE html>"));
        assert!(payload[start_html..end_html].ends_with("</body></html>"));
        assert_eq!(
            start,
            payload.find("<!--StartFragment-->").unwrap() + "<!--StartFragment-->".len()
        );
        assert_eq!(end, payload.find("<!--EndFragment-->").unwrap());
        assert_eq!(end_html, payload.len());
        assert!(payload[..start_html]
            .split_inclusive('\n')
            .all(|line| line.ends_with("\r\n")));
    }
}
