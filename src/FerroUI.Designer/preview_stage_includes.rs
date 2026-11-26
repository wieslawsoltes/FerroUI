//! Material used for native includes and for comparing accepted stage dependencies.
use super::{native_source_uri, PreviewStageSource, PreviewStageStatus, SourceStamp};
use crate::PreviewAssets;
use ferroui_designer_project::{LocalPreviewSources, PackagedAssets};
use ferroui_markup_xaml::source_overlay::SourceSnapshot;
use std::{collections::BTreeMap, rc::Rc, sync::Arc};

#[derive(Clone, Default)]
pub(super) struct IncludeCatalog {
    documents: BTreeMap<String, IncludeSource>,
    pub(super) assets: Arc<PackagedAssets>,
}

impl IncludeCatalog {
    pub(super) fn get(&self, uri: &str) -> Option<&IncludeSource> {
        self.documents.get(uri)
    }
}

#[derive(Clone)]
pub(super) enum IncludeSource {
    Open(Rc<SourceStamp>),
    Local(Rc<str>),
    Unavailable(Rc<str>),
}

#[derive(Clone, Default)]
pub(super) struct IncludeCatalogs {
    open: Rc<IncludeCatalog>,
    scoped: BTreeMap<String, Rc<IncludeCatalog>>,
}

impl IncludeCatalogs {
    pub(super) fn new(
        sources: &BTreeMap<String, PreviewStageSource>,
        local: &BTreeMap<&str, &LocalPreviewSources>,
        options_epoch: u64,
    ) -> Result<Self, String> {
        let mut open = IncludeCatalog::default();
        for source in sources.values() {
            let key = native_source_uri(&source.uri)?;
            let value = IncludeSource::Open(Rc::new(SourceStamp::new(source, options_epoch)));
            if open.documents.insert(key, value).is_some() {
                return Err("Preview source URIs resolve to the same native document".into());
            }
        }
        let scoped = local
            .iter()
            .map(|(&owner, sources)| {
                let mut catalog = open.clone();
                // Explicit local results override open buffers at the same loader URI.
                extend_local(&mut catalog, sources);
                (owner.to_owned(), Rc::new(catalog))
            })
            .collect();
        Ok(Self {
            open: Rc::new(open),
            scoped,
        })
    }

    pub(super) fn for_document(&self, uri: &str) -> &Rc<IncludeCatalog> {
        self.scoped.get(uri).unwrap_or(&self.open)
    }
}

fn extend_local(catalog: &mut IncludeCatalog, sources: &LocalPreviewSources) {
    catalog.assets = sources.includes.packaged_assets().clone();
    catalog.documents.extend(
        sources
            .accepted
            .iter()
            .map(|(uri, text)| (uri.clone(), IncludeSource::Local(Rc::from(text.as_str()))))
            .chain(sources.unavailable.iter().map(|(uri, message)| {
                (
                    uri.clone(),
                    IncludeSource::Unavailable(Rc::from(message.as_str())),
                )
            })),
    );
}

pub(super) fn snapshot(catalog: &IncludeCatalog, assets: &PreviewAssets) -> SourceSnapshot {
    let mut accepted = BTreeMap::new();
    let mut unavailable = BTreeMap::new();
    for (uri, source) in &catalog.documents {
        match source {
            IncludeSource::Open(source) if source.pending => {
                unavailable.insert(
                    uri.clone(),
                    format!("Apply the pending included source before refreshing: {uri}"),
                );
            }
            IncludeSource::Open(source) if !source.valid => {
                unavailable.insert(
                    uri.clone(),
                    format!("Repair the included XAML before refreshing: {uri}"),
                );
            }
            IncludeSource::Open(source) => {
                accepted.insert(uri.clone(), source.source.clone());
            }
            IncludeSource::Local(source) => {
                accepted.insert(uri.clone(), source.to_string());
            }
            IncludeSource::Unavailable(message) => {
                unavailable.insert(uri.clone(), message.to_string());
            }
        }
    }
    assets.apply(SourceSnapshot::new(accepted, unavailable))
}

impl IncludeSource {
    pub(super) fn same_buffer(&self, current: &Self) -> bool {
        match (self, current) {
            (Self::Open(accepted), Self::Open(current)) => accepted.same_buffer(current),
            (Self::Local(accepted), Self::Local(current))
            | (Self::Unavailable(accepted), Self::Unavailable(current)) => accepted == current,
            _ => false,
        }
    }

    pub(super) fn matches_open_buffer(&self, current: Option<&PreviewStageSource>) -> bool {
        match (self, current) {
            (Self::Open(accepted), Some(current)) => accepted.matches_buffer(current),
            (Self::Local(accepted), Some(current)) => {
                !current.pending
                    && current.document.is_valid()
                    && current.document.source() == accepted.as_ref()
            }
            (Self::Local(_), None) => true,
            _ => false,
        }
    }

    pub(super) fn blocked(&self, uri: &str, revision: u64) -> Option<(PreviewStageStatus, String)> {
        match self {
            Self::Open(source) if source.pending => Some((
                PreviewStageStatus::PendingSource { revision },
                format!("Apply the pending included source before refreshing: {uri}"),
            )),
            Self::Open(source) if !source.valid => Some((
                PreviewStageStatus::InvalidSource { revision },
                format!("Repair the included XAML before refreshing: {uri}"),
            )),
            Self::Unavailable(message) => Some((
                PreviewStageStatus::Failed {
                    revision: Some(revision),
                    message: message.to_string(),
                },
                message.to_string(),
            )),
            _ => None,
        }
    }
}
