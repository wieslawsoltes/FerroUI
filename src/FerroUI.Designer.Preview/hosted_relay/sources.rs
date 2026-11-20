use super::Design;
use ferroui_designer_core::Document;
use ferroui_designer_project::{CompiledResources, PreviewSourceBuffer, hosted_preview_sources};
use ferroui_remote_protocol::{
    Message,
    designer::{DesignIncludedSource, UpdateDesignMessage, UpdateDesignSourcesMessage},
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Workspace {
    root: PathBuf,
    local_sources: Vec<PreviewSourceBuffer>,
}

pub(super) struct Sources {
    base_uri: String,
    includes: Vec<DesignIncludedSource>,
    uses_drafts: bool,
}

impl Workspace {
    pub(super) fn prepare(
        &self,
        design: &Design,
        document: &Document,
        resources: Option<&CompiledResources>,
        dependencies: &mut Option<Vec<String>>,
    ) -> Result<Sources, String> {
        let fallback = format!("ferres://{}{}", design.assembly, design.project_path);
        let base_uri = match resources {
            Some(resources) => resources.preview_base(&design.document.uri, &fallback)
                .map_err(|error| error.to_string())?,
            None => &fallback,
        };
        let graph = hosted_preview_sources(
            &self.root, &design.document.uri, base_uri, document, &self.local_sources, resources,
        );
        *dependencies = Some(graph.dependencies.into_iter().collect());
        if let Some((_, error)) = graph.unavailable.into_iter().next() {
            return Err(error);
        }
        Ok(Sources {
            base_uri: base_uri.into(),
            includes: graph.accepted.into_iter().map(|(uri, xaml)| DesignIncludedSource { uri, xaml }).collect(),
            uses_drafts: graph.uses_drafts,
        })
    }
}

impl Sources {
    pub(super) fn message(self, update: UpdateDesignMessage, supported: bool) -> Result<Message, String> {
        if supported {
            return Ok(Arc::new(UpdateDesignSourcesMessage {
                update,
                base_uri: self.base_uri,
                includes: self.includes,
            }));
        }
        if self.uses_drafts {
            return Err("Rebuild the application with current FerroUI DesignerSupport to preview unsaved XAML includes.".into());
        }
        Ok(Arc::new(update))
    }
}
