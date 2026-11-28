use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::i_text_view_connect::ITextViewConnect;
use crate::visual_line_element::VisualLineElement;
use crate::visual_line_element_generator::VisualLineElementGenerator;
use crate::visual_line_error::VisualLineError;
use crate::visual_line_link_text::{LinkTextStyle, VisualLineLinkText};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_edit_core::document::{DocumentError, TextSnapshot};
use url::Url;
use ferroui_edit_core::search::{
    ISearchStrategy, SearchError, SearchMode, SearchResult, SearchStrategyFactory,
};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

const WEB_PATTERN: &str = r"\b(https?://|ftp://|www\.)[\w\d\._/\-~%@()+:?&=#!]*[\w\d/]";
const MAIL_PATTERN: &str = r"\b[\w\d\.\-\+]+@[\w\d\.\-]+\.[a-z]{2,6}\b";

/// Finds necessary markers for the built-in patterns before formatting candidate lines.
/// The rolling prefix is carried across shared UTF-16 rope chunks.
pub(crate) fn builtin_link_markers(
    snapshot: &TextSnapshot,
    span: Range<usize>,
    web: bool,
    email: bool,
) -> Result<impl Iterator<Item = usize>, DocumentError> {
    let chunks = snapshot.get_rope().chunks_range(span.start, span.len()).ok_or(
        DocumentError::InvalidRange {
            offset: span.start,
            length: span.len(),
            text_length: snapshot.text_length(),
        },
    )?;
    let units = chunks.flat_map(|chunk| (0..chunk.len()).map(move |i| chunk[i]));
    Ok(link_marker_offsets(units, web, email).map(move |index| span.start + index))
}

fn link_marker_offsets(
    units: impl Iterator<Item = u16>,
    web: bool,
    email: bool,
) -> impl Iterator<Item = usize> {
    let mut prev = [0u16; 3];
    units.enumerate().filter_map(move |(index, unit)| {
        let mail = email && unit == u16::from(b'@');
        let url = web && (
            (unit == u16::from(b'/') && prev[1..] == [u16::from(b':'), u16::from(b'/')])
            || (unit == u16::from(b'.') && prev == [u16::from(b'w'); 3])
        );
        prev = [prev[1], prev[2], unit];
        (mail || url).then_some(index)
    })
}

ferroui_base::ferro_thread_local! {
    static WEB_SEARCH: Rc<dyn ISearchStrategy> = compile_link_strategy(WEB_PATTERN)
        .expect("the built-in hyperlink expression is valid");
    static MAIL_SEARCH: Rc<dyn ISearchStrategy> = compile_link_strategy(MAIL_PATTERN)
        .expect("the built-in email expression is valid");
}

fn compile_link_strategy(pattern: &str) -> Result<Rc<dyn ISearchStrategy>, SearchError> {
    Ok(Rc::new(SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx)?))
}

#[derive(Clone)]
pub struct LinkMatch {
    pub offset: usize,
    pub matched: SearchResult,
    pub uri: Url,
}
pub type LinkUriResolver = dyn Fn(&SearchResult) -> Option<Url>;
pub type LinkElementFactory = dyn Fn(
    &LinkMatch,
    &dyn ITextRunConstructionContext,
)
    -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError>;

#[derive(Clone, Copy)]
enum BuiltinPattern {
    Web,
    Email,
}

struct LinkCandidates {
    line_start: usize,
    search_start: usize,
    text: ReadOnlyMemory<u16>,
    tokens: Vec<Range<usize>>,
    matches: Vec<LinkMatch>,
}

fn link_separator(unit: u16) -> bool {
    matches!(unit, 9..=13 | 32)
}

/// Uses the editor's bounded regex/search service, including its explicit pattern
/// and match-budget diagnostics. A custom strategy can supply another matcher.
pub struct LinkElementGenerator {
    strategy: Rc<dyn ISearchStrategy>,
    resolve_uri: Rc<LinkUriResolver>,
    pub element_factory: Option<Rc<LinkElementFactory>>,
    pub require_control_modifier_for_click: bool,
    pub target_name: Option<String>,
    pub style: LinkTextStyle,
    cached: RefCell<Option<LinkMatch>>,
    builtin: Option<BuiltinPattern>,
    candidates: RefCell<Option<LinkCandidates>>,
}
impl Default for LinkElementGenerator {
    fn default() -> Self {
        let mut generator = Self::with_strategy(WEB_SEARCH.with(Rc::clone), Rc::new(|matched| {
            let text = matched.matched_text();
            let text = if text.starts_with("www.") {
                format!("http://{text}")
            } else {
                text.to_owned()
            };
            well_formed_web_uri(&text)
        }));
        generator.builtin = Some(BuiltinPattern::Web);
        generator
    }
}
impl LinkElementGenerator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_pattern(
        pattern: &str,
        resolve_uri: impl Fn(&SearchResult) -> Option<Url> + 'static,
    ) -> Result<Self, SearchError> {
        Ok(Self::with_strategy(
            compile_link_strategy(pattern)?,
            Rc::new(resolve_uri),
        ))
    }
    pub fn with_strategy(
        strategy: Rc<dyn ISearchStrategy>,
        resolve_uri: Rc<LinkUriResolver>,
    ) -> Self {
        Self {
            strategy,
            resolve_uri,
            element_factory: None,
            require_control_modifier_for_click: true,
            target_name: None,
            style: LinkTextStyle::default(),
            cached: RefCell::new(None),
            builtin: None,
            candidates: RefCell::new(None),
        }
    }
    fn find_match(
        &self,
        start: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<LinkMatch>, VisualLineError> {
        let end = context.last_document_line().end_offset();
        if start >= end {
            return Ok(None);
        }
        let line_start = context.first_document_line().offset();
        if let Some(pattern) = self.builtin.filter(|_| {
            self.element_factory.is_none() && line_start <= start
        }) {
            return self.find_builtin_match(start, line_start, end, pattern, context);
        }
        let text = context.document().get_text(start, end - start)?;
        self.find_in_text(start, &text)
    }

    fn find_builtin_match(
        &self,
        mut start: usize,
        line_start: usize,
        end: usize,
        pattern: BuiltinPattern,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<LinkMatch>, VisualLineError> {
        let text = context.get_text(line_start, end - line_start)?;
        let mut cache = self.candidates.borrow_mut();
        if !cache.as_ref().is_some_and(|cached| {
            cached.line_start == line_start
                && cached.search_start <= start
                && cached.text.len() == text.len()
                && cached.text.offset_in_owner() == text.offset_in_owner()
                && cached.text.shares_owner_with(&text)
        }) {
            let mut tokens = Vec::new();
            let mut matches = Vec::new();
            let mut offset = start;
            let web = matches!(pattern, BuiltinPattern::Web);
            let email = matches!(pattern, BuiltinPattern::Email);
            // The built-in expressions cannot consume ASCII whitespace. For
            // their word boundaries, a separator and a token edge agree.
            for part in text.span()[start - line_start..]
                .split_inclusive(|&unit| link_separator(unit))
            {
                let len = part.len()
                    - usize::from(part.last().is_some_and(|&unit| link_separator(unit)));
                let token = &part[..len];
                if link_marker_offsets(token.iter().copied(), web, email).next().is_some() {
                    tokens.push(offset..offset + len);
                    matches.extend(self.find_in_text(offset, &String::from_utf16_lossy(token))?);
                }
                offset += part.len();
            }
            *cache = Some(LinkCandidates {
                line_start,
                search_start: start,
                text,
                tokens,
                matches,
            });
        }
        let cached = cache.as_ref().expect("candidate index is initialized");
        let index = cached.tokens.partition_point(|token| token.end <= start);
        if let Some(first) = cached.tokens.get(index).filter(|token| token.start < start) {
            // An earlier generator can expose a new start inside a token.
            // Match that exact suffix, including its new leading boundary.
            let slice = &cached.text.span()[start - line_start..first.end - line_start];
            if let Some(matched) = self.find_in_text(start, &String::from_utf16_lossy(slice))? {
                return Ok(Some(matched));
            }
            start = first.end;
        }
        let index = cached.matches.partition_point(|matched| matched.offset < start);
        Ok(cached.matches.get(index).cloned())
    }

    fn find_in_text(&self, start: usize, text: &str) -> Result<Option<LinkMatch>, VisualLineError> {
        for matched in self
            .strategy
            .find_all(text, 0, text.encode_utf16().count())
            .map_err(|error| VisualLineError::Extension(error.to_string()))?
        {
            if matched.length == 0 {
                continue;
            }
            if let Some(uri) = (self.resolve_uri)(&matched) {
                return Ok(Some(LinkMatch {
                    offset: start + matched.offset,
                    matched,
                    uri,
                }));
            }
        }
        Ok(None)
    }
}
impl ITextViewConnect for LinkElementGenerator {}
impl VisualLineElementGenerator for LinkElementGenerator {
    fn start_generation(
        &self,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<(), VisualLineError> {
        self.cached.borrow_mut().take();
        self.candidates.borrow_mut().take();
        Ok(())
    }
    fn finish_generation(&self) {
        self.cached.borrow_mut().take();
        self.candidates.borrow_mut().take();
    }
    fn get_first_interested_offset(
        &self,
        start: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        let matched = self.find_match(start, context)?;
        let offset = matched.as_ref().map(|matched| matched.offset);
        *self.cached.borrow_mut() = matched;
        Ok(offset)
    }
    fn construct_element(
        &self,
        offset: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        let cached = self
            .cached
            .borrow()
            .clone()
            .filter(|matched| matched.offset == offset);
        let matched = if cached.is_some() {
            cached
        } else {
            self.find_match(offset, context)?
        };
        let Some(matched) = matched.filter(|matched| matched.offset == offset) else {
            return Ok(None);
        };
        if let Some(factory) = &self.element_factory {
            return factory(&matched, context);
        }
        let mut element = VisualLineLinkText::new(
            matched.matched.length,
            &**context.global_text_run_properties(),
        )?;
        element.navigate_uri = Some(matched.uri);
        element.target_name = self.target_name.clone();
        element.require_control_modifier_for_click = self.require_control_modifier_for_click;
        element.style = self.style.clone();
        Ok(Some(Box::new(element)))
    }
}

/// The email variant retains the source match rules and creates mailto URIs.
pub struct MailLinkElementGenerator(pub LinkElementGenerator);
impl Default for MailLinkElementGenerator {
    fn default() -> Self {
        let mut generator =
            LinkElementGenerator::with_strategy(MAIL_SEARCH.with(Rc::clone), Rc::new(|matched| {
                Url::parse(&format!("mailto:{}", matched.matched_text())).ok()
            }));
        generator.builtin = Some(BuiltinPattern::Email);
        Self(generator)
    }
}
impl MailLinkElementGenerator {
    pub fn new() -> Self {
        Self::default()
    }
}
impl ITextViewConnect for MailLinkElementGenerator {}
impl VisualLineElementGenerator for MailLinkElementGenerator {
    fn start_generation(
        &self,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<(), VisualLineError> {
        self.0.start_generation(context)
    }
    fn finish_generation(&self) {
        self.0.finish_generation();
    }
    fn get_first_interested_offset(
        &self,
        start: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        self.0.get_first_interested_offset(start, context)
    }
    fn construct_element(
        &self,
        offset: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        self.0.construct_element(offset, context)
    }
}
fn well_formed_web_uri(text: &str) -> Option<Url> {
    let uri = Url::parse(text).ok()?;
    if matches!(uri.scheme(), "http" | "https" | "ftp") {
        let (_, remainder) = text.split_once("://")?;
        let authority = remainder.split(['/', '?', '#']).next()?;
        if authority.is_empty() || authority.chars().any(char::is_whitespace) {
            return None;
        }
    }
    let bytes = text.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'%'
            && !(bytes.get(index + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(index + 2).is_some_and(u8::is_ascii_hexdigit))
        {
            return None;
        }
    }
    Some(uri)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VisualLineBuilder, VisualLineBuildOptions};
    use ferroui_base::media::text_formatting::GenericTextRunProperties;
    use ferroui_base::media::{FontFamily, Typeface};

    #[test]
    fn indexed_links_agree_with_suffix_matching_at_each_offset() {
        let _scope = crate::tests::test_scope();
        let signature = |matched: Option<LinkMatch>| {
            matched.map(|m| (m.offset, m.matched.length, m.matched.matched_text().to_owned(), m.uri))
        };
        let properties = Rc::new(GenericTextRunProperties::with_font_size(
            Typeface::new(FontFamily::new("Test Sans")),
            12.0,
        ));
        for (indexed, mut direct) in [
            (LinkElementGenerator::new(), LinkElementGenerator::new()),
            (MailLinkElementGenerator::new().0, MailLinkElementGenerator::new().0),
        ] {
            direct.builtin = None;
            for input in [
                "\txwww.example.com\t😀 mail+tag@host.example (https://a.example/path.)",
                "\u{000c}www.first.example,@then.example http://%zz.invalid\tftp://b.example/a",
                "😀.a+z@example.org\t(two@example.com,+three@example.net)\tno@domain",
                "https:\t//split.example\ta+b@\texample.org\tHTTPS://uppercase.example",
                "\tplain\ttext\twith\ttabs  ",
                "\twww.replacement.test\tanother@replacement.test",
            ] {
                let mut utf16 = input.encode_utf16().collect::<Vec<_>>();
                utf16.insert(1, 0xd800);
                let builder = VisualLineBuilder::generate(
                    TextSnapshot::from_utf16(&utf16), 1, properties.clone(), &[],
                    VisualLineBuildOptions::default(),
                ).unwrap();
                let context = builder.context();
                for start in (0..=utf16.len()).chain([1, 0]) {
                    assert_eq!(
                        signature(indexed.find_match(start, context).unwrap()),
                        signature(direct.find_match(start, context).unwrap()),
                        "{input:?} at UTF-16 offset {start}"
                    );
                }
            }
        }
    }

    #[test]
    fn custom_link_factory_receives_the_unabridged_search_suffix() {
        let _scope = crate::tests::test_scope();
        let text = "prefix (https://example.org) suffix";
        let mut web = LinkElementGenerator::new();
        let called = Rc::new(std::cell::Cell::new(false));
        let observed = called.clone();
        web.element_factory = Some(Rc::new(move |matched, _| {
            assert_eq!(matched.offset, 8);
            assert_eq!(matched.matched.offset, 8);
            assert_eq!(matched.matched.capture(1), Some("https://"));
            assert_eq!(matched.matched.replace_with("$_"), text);
            observed.set(true);
            Ok(None)
        }));
        let properties = Rc::new(GenericTextRunProperties::with_font_size(
            Typeface::new(FontFamily::new("Test Sans")),
            12.0,
        ));
        let _ = VisualLineBuilder::generate(
            TextSnapshot::from_text(text), 1, properties, &[Rc::new(web).into()],
            VisualLineBuildOptions::default(),
        ).unwrap();
        assert!(called.get());
    }

    #[test]
    fn built_in_link_markers_cross_utf16_rope_boundaries() {
        let mut units = vec![u16::from(b'x'); 2048];
        let snapshot = TextSnapshot::from_utf16(&units);
        let mut chunks = snapshot.get_rope().chunks();
        let first = chunks.next().unwrap().len();
        let second = first + chunks.next().unwrap().len();
        units[first - 2..first + 2].copy_from_slice(&b"www.".map(u16::from));
        units[second - 2..second + 1].copy_from_slice(&b"://".map(u16::from));
        units[second + 9] = u16::from(b'@');
        let snapshot = TextSnapshot::from_utf16(&units);
        let span = first - 2..second + 10;
        let offsets = |web, email| builtin_link_markers(&snapshot, span.clone(), web, email)
            .unwrap().collect::<Vec<_>>();
        assert_eq!(offsets(true, true), [first + 1, second, second + 9]);
        assert_eq!(offsets(true, false), [first + 1, second]);
        assert_eq!(offsets(false, true), [second + 9]);
    }
}
