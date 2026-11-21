//! Checked locations in the source buffers accepted by a native preview.
use crate::{DesignerSession, PreviewStatus};
use ferroui_designer_core::{Document, Node, NodeId, SourceSpan};
use ferroui_designer_language::{LineIndex, Range};
use ferroui_designer_project::{PreviewInclude, PreviewIncludes};
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IncludedSource {
    pub uri: String,
    pub range: Range,
    source_stamp: String,
}

impl IncludedSource {
    pub(crate) fn span_in(&self, document: &Document, uri: &str) -> Result<SourceSpan, String> {
        let text = document.source();
        let source = text.strip_prefix('\u{feff}').unwrap_or(text);
        if ferroui_designer_project::canonical_file_uri(uri)
            .ok()
            .as_ref()
            != Some(&self.uri)
            || !document.is_valid()
            || self.source_stamp != source_stamp(source)
        {
            return Err(
                "The included source changed. Reopen its definition from the consumer.".into(),
            );
        }
        let span = LineIndex::new(source)
            .span(self.range)
            .map_err(|error| error.to_string())?;
        let bom = text.len() - source.len();
        Ok(SourceSpan::new(span.start + bom, span.end + bom))
    }

    pub(crate) fn node_in<'a>(
        &self,
        document: &'a Document,
        uri: &str,
    ) -> Result<&'a Node, String> {
        let span = self.span_in(document, uri)?;
        let ast = document.ast();
        ast.element_at(span.start)
            .and_then(|id| ast.node(id))
            .filter(|node| node.name_span == span)
            .ok_or_else(|| "The included declaration changed.".into())
    }
}

/// Reuse the index and stamp when a dictionary supplies several locations.
pub(crate) struct IncludedSourceIndex<'a> {
    uri: &'a str,
    index: LineIndex,
    stamp: String,
}

impl<'a> IncludedSourceIndex<'a> {
    pub(crate) fn new(include: &'a PreviewInclude) -> Self {
        let source = include.document.source();
        Self {
            uri: &include.uri,
            index: LineIndex::new(source),
            stamp: source_stamp(source),
        }
    }

    pub(crate) fn locate(&self, node: &Node) -> Result<IncludedSource, String> {
        self.locate_span(node.name_span)
    }

    pub(crate) fn locate_span(&self, span: SourceSpan) -> Result<IncludedSource, String> {
        Ok(IncludedSource {
            uri: self.uri.into(),
            range: self
                .index
                .range(span)
                .map_err(|error| error.to_string())?,
            source_stamp: self.stamp.clone(),
        })
    }
}

impl DesignerSession {
    pub(crate) fn accepted_preview_includes(&self) -> Option<&PreviewIncludes> {
        matches!(self.preview_status(), PreviewStatus::Ready { revision } if *revision == self.document().revision())
            .then_some(&self.preview.resource_sources.0)
    }

    pub(crate) fn included_source_node(
        &self,
        info: &XamlSourceInfo,
        type_name: &str,
    ) -> Option<(&str, &PreviewInclude, NodeId)> {
        let uri = info.loader_uri().or_else(|| info.source_uri().map(|source| source.as_str()))?;
        let (address, include) = self.accepted_preview_includes()?
            .authored_revision(uri, info.source_revision()?)?;
        let node = crate::runtime::node_at_source_info(&include.document, info, type_name)?;
        Some((address, include, node))
    }
}

fn source_stamp(source: &str) -> String {
    let mut hash = DefaultHasher::new();
    source.hash(&mut hash);
    format!("{:016x}", hash.finish())
}
