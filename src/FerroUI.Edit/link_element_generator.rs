use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::i_text_view_connect::ITextViewConnect;
use crate::visual_line_element::VisualLineElement;
use crate::visual_line_element_generator::VisualLineElementGenerator;
use crate::visual_line_error::VisualLineError;
use crate::visual_line_link_text::{LinkTextStyle, VisualLineLinkText};
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_edit_core::search::{
    ISearchStrategy, SearchError, SearchMode, SearchResult, SearchStrategyFactory,
};
use std::cell::RefCell;
use std::rc::Rc;

const WEB_PATTERN: &str = r"\b(https?://|ftp://|www\.)[\w\d\._/\-~%@()+:?&=#!]*[\w\d/]";
const MAIL_PATTERN: &str = r"\b[\w\d\.\-\+]+@[\w\d\.\-]+\.[a-z]{2,6}\b";

#[derive(Clone)]
pub struct LinkMatch {
    pub offset: usize,
    pub matched: SearchResult,
    pub uri: Uri,
}
pub type LinkUriResolver = dyn Fn(&SearchResult) -> Option<Uri>;
pub type LinkElementFactory = dyn Fn(
    &LinkMatch,
    &dyn ITextRunConstructionContext,
)
    -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError>;

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
}
impl Default for LinkElementGenerator {
    fn default() -> Self {
        Self::with_pattern(WEB_PATTERN, |matched| {
            let text = matched.matched_text();
            let text = if text.starts_with("www.") {
                format!("http://{text}")
            } else {
                text.to_owned()
            };
            well_formed_web_uri(&text)
        })
        .expect("the built-in hyperlink expression is valid")
    }
}
impl LinkElementGenerator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_pattern(
        pattern: &str,
        resolve_uri: impl Fn(&SearchResult) -> Option<Uri> + 'static,
    ) -> Result<Self, SearchError> {
        Ok(Self::with_strategy(
            Rc::new(SearchStrategyFactory::create(
                pattern,
                false,
                false,
                SearchMode::RegEx,
            )?),
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
        let text = context.document().get_text(start, end - start)?;
        for matched in self
            .strategy
            .find_all(&text, 0, text.encode_utf16().count())
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
        Ok(())
    }
    fn finish_generation(&self) {
        self.cached.borrow_mut().take();
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
        Self(
            LinkElementGenerator::with_pattern(MAIL_PATTERN, |matched| {
                Uri::try_create(
                    &format!("mailto:{}", matched.matched_text()),
                    UriKind::Absolute,
                )
            })
            .expect("the built-in email expression is valid"),
        )
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
fn well_formed_web_uri(text: &str) -> Option<Uri> {
    let uri = Uri::try_create(text, UriKind::Absolute)?;
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
