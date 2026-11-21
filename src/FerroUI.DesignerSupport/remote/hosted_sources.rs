use ferroui_base::utilities::UriRef;
use ferroui_markup_xaml::{XamlLoadError, source_overlay::SourceSnapshot};
use ferroui_remote_protocol::designer::{
    DESIGN_SOURCE_SNAPSHOT_QUERY, DESIGN_SOURCE_SNAPSHOT_VERSION, UpdateDesignSourcesMessage,
};
use std::collections::BTreeMap;
use url::Url;

const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
const MAX_GRAPH_BYTES: usize = 8 * MAX_DOCUMENT_BYTES;
const MAX_INCLUDES: usize = 128;

pub(super) fn requested(transport: Option<&Url>, remote: bool) -> bool {
    transport.filter(|uri| remote && uri.scheme() == "tcp-bson").is_some_and(|uri| {
        uri.query_pairs().any(|(name, version)| {
            name == DESIGN_SOURCE_SNAPSHOT_QUERY && version == DESIGN_SOURCE_SNAPSHOT_VERSION
        })
    })
}

pub(super) fn prepare(update: &UpdateDesignSourcesMessage) -> Result<(Url, SourceSnapshot), XamlLoadError> {
    let base = loader_uri(&update.base_uri)?;
    if base.scheme() != "ferres" || base.host_str().is_none() {
        return Err(XamlLoadError::with_message("An application preview needs a packaged resource base URI."));
    }
    if update.includes.len() > MAX_INCLUDES {
        return Err(XamlLoadError::with_message("The application preview has too many XAML includes."));
    }
    let mut remaining = MAX_GRAPH_BYTES;
    let mut sources = BTreeMap::new();
    for include in &update.includes {
        let uri = loader_uri(&include.uri)?;
        if include.xaml.len() > remaining.min(MAX_DOCUMENT_BYTES) {
            return Err(XamlLoadError::with_message("The application preview exceeds the XAML source limit."));
        }
        remaining -= include.xaml.len();
        if sources.get(uri.as_str()).is_some_and(|source| source != &include.xaml) {
            return Err(XamlLoadError::with_message("Two application preview sources resolve to the same URI."));
        }
        sources.insert(uri.into(), include.xaml.clone());
    }
    Ok((base, SourceSnapshot::new(sources, BTreeMap::new())))
}

fn loader_uri(value: &str) -> Result<Url, XamlLoadError> {
    UriRef::parse_absolute(value).ok().filter(|uri| matches!(uri.scheme(), "file" | "ferres"))
        .ok_or_else(|| XamlLoadError::with_message("Invalid application preview source URI."))
}
