//! A checked source location for a native value contributor.
use super::TraceLocation;
use crate::included_source::{IncludedSource, IncludedSourceIndex};
use ferroui_designer_core::{Document, SourceSpan};
#[cfg(feature = "mcp")]
use ferroui_designer_language::LineIndex;
use ferroui_designer_project::PreviewInclude;
use ferroui_designer_rsx::RsxSourceMap;
#[cfg(feature = "mcp")]
use serde_json::{json, Value};

pub(crate) enum TraceSource {
    Local(TraceLocation),
    Included {
        source: IncludedSource,
        #[cfg(feature = "mcp")]
        revision: u64,
        #[cfg(feature = "mcp")]
        text: Option<String>,
    },
}

pub(crate) enum TraceNavigation {
    Local(SourceSpan),
    Included(IncludedSource),
}

impl TraceSource {
    pub(super) fn included(include: &PreviewInclude, location: TraceLocation) -> Option<Self> {
        Some(Self::Included {
            source: IncludedSourceIndex::new(include)
                .locate_span(location.span)
                .ok()?,
            #[cfg(feature = "mcp")]
            revision: include.source_revision(),
            #[cfg(feature = "mcp")]
            text: location
                .text(&include.document, include.document.source(), None)
                .map(str::to_owned),
        })
    }

    pub(crate) fn navigation(
        &self,
        document: &Document,
        map: Option<&RsxSourceMap>,
    ) -> Option<TraceNavigation> {
        match self {
            Self::Local(location) => location.span(document, map).map(TraceNavigation::Local),
            Self::Included { source, .. } => Some(TraceNavigation::Included(source.clone())),
        }
    }

    #[cfg(feature = "mcp")]
    pub(super) fn json(
        &self,
        document: &Document,
        source: &str,
        map: Option<&RsxSourceMap>,
        index: &LineIndex,
    ) -> Option<Value> {
        match self {
            Self::Local(location) => location.json(document, source, map, index),
            Self::Included {
                source,
                revision,
                text,
            } => Some(json!({
                "included": source,
                "sourceRevision": format!("{revision:016x}"),
                "text": text,
            })),
        }
    }
}
