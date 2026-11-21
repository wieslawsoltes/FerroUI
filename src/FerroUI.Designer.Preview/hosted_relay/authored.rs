//! Authored source and edit guards for the accepted application preview.
use super::{sources::AuthoredSources, Relay};
use crate::hosted_animation::{checked_source_patch, source_patch};
use crate::hosted_frame::FrameIdentity;
use ferroui_designer_core::{Document, Edit, SourceSpan};
use ferroui_designer_language::{LineIndex, Range, Schema};
use ferroui_designer_tools::{theme_variants::HighContrastProvider, ComponentResourceContext};
use ferroui_remote_protocol::designer::DesignPaletteContextMessage;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Deserialize)]
pub(crate) struct Request<T> {
    #[serde(flatten)]
    pub(super) frame: FrameIdentity,
    #[serde(flatten)]
    pub(super) action: T,
}

pub(super) struct AuthoredPreview {
    pub(super) document: Document,
    uri: String,
    pub(super) sources: AuthoredSources,
    pub(super) contrast: Option<ComponentResourceContext>,
    pub(super) providers: Vec<HighContrastProvider>,
    pub(super) schema: Arc<Schema>,
}

impl Relay {
    pub(super) fn authored(&self, frame: FrameIdentity) -> Result<&AuthoredPreview, String> {
        self.require_authored_frame(frame)?;
        self.authored
            .as_ref()
            .ok_or_else(|| "Use an XAML App Preview with scoped authoring support".into())
    }

    pub(super) fn require_authored_frame(&self, frame: FrameIdentity) -> Result<(), String> {
        let state = self.events.0.lock().unwrap();
        if state.disconnected
            || self.sent != Some((frame.revision, frame.sequence))
            || state.requested != Some(frame.revision)
            || !matches!(state.outcome, Some(Ok(())))
        {
            return Err("Read the current app-hosted preview frame before editing its source".into());
        }
        Ok(())
    }
}

impl AuthoredPreview {
    pub(super) fn new(
        document: Document,
        uri: String,
        sources: AuthoredSources,
        context: DesignPaletteContextMessage,
        schema: Arc<Schema>,
    ) -> Self {
        let contrast = context.contrast.and_then(|keys| {
            let mut keys = keys.into_iter();
            Some(ComponentResourceContext {
                theme: Some(keys.next()?),
                inherited_themes: keys.collect(),
            })
        });
        let providers = HighContrastProvider::ALL
            .into_iter()
            .filter(|provider| {
                context
                    .assemblies
                    .iter()
                    .any(|name| name == provider.assembly())
            })
            .collect();
        Self {
            document,
            uri,
            sources,
            contrast,
            providers,
            schema,
        }
    }

    pub(super) fn documents(&self) -> impl Iterator<Item = (&str, &str, &Document)> {
        std::iter::once((
            self.sources.base_uri.as_str(),
            self.uri.as_str(),
            &self.document,
        ))
        .chain(
            self.sources
                .includes
                .authored_sources()
                .map(|(address, included)| (address, included.uri.as_str(), &included.document)),
        )
    }

    pub(super) fn source_at(&self, address: &str) -> Result<(&str, &Document), String> {
        if address == self.sources.base_uri {
            return Ok((&self.uri, &self.document));
        }
        self.sources
            .includes
            .authored_source(address)
            .map(|include| (include.uri.as_str(), &include.document))
            .ok_or_else(|| "Choose a source from the accepted app preview".into())
    }

    pub(super) fn plan(
        &self,
        address: &str,
        source: &str,
        edits: impl FnOnce(&Document) -> Result<Vec<Edit>, String>,
    ) -> Result<Value, String> {
        let (uri, document) = self.source_at(address)?;
        let bom = check_source(document, source)?;
        let edits = edits(document)?;
        let mut draft = document.clone();
        draft
            .apply("Edit application preview XAML", edits)
            .map_err(|error| error.to_string())?;
        let updated = format!("{bom}{}", without_bom(draft.source()));
        let patch = checked_source_patch(source, &updated)?.map(source_patch);
        Ok(json!({"uri":uri, "patches":patch.into_iter().collect::<Vec<_>>()}))
    }

    pub(super) fn locate(
        &self,
        address: &str,
        range: Range,
        source: &str,
    ) -> Result<Value, String> {
        let (uri, document) = self.source_at(address)?;
        let bom = check_source(document, source)?;
        let captured = document.source();
        let captured_bom = captured.len() - without_bom(captured).len();
        let span = LineIndex::new(captured)
            .span(range)
            .map_err(|error| error.to_string())?;
        let start = span
            .start
            .checked_sub(captured_bom)
            .ok_or("The range starts before the XAML content")?;
        let selected = SourceSpan::new(start + bom.len(), span.end - captured_bom + bom.len());
        let range = LineIndex::new(source)
            .range(selected)
            .map_err(|error| error.to_string())?;
        Ok(json!({"uri":uri,"range":range}))
    }
}

fn check_source<'a>(document: &Document, source: &'a str) -> Result<&'a str, String> {
    let text = without_bom(source);
    if text != without_bom(document.source()) {
        return Err("The XAML buffer changed. Wait for the application preview to update.".into());
    }
    Ok(&source[..source.len() - text.len()])
}

fn without_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}
