use super::{FrameIdentity, SourceMap, SourceMaps};
use crate::hosted_animation::{ClipDraft, checked_source_patch, source_patch};
use crate::rsx_animation::AuthoredRsxFrame;
use ferroui_designer_animation::{clip_clock_delays, xaml_clip_delay_edit};
use ferroui_designer_core::NodeId;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

impl SourceMap {
    pub(crate) fn annotate_clips(&self, catalog: &mut Value) {
        if !self.clips.values().any(|clip| clip.clock.is_some()) {
            return;
        }
        catalog["clipScope"] = Value::String(
            json!([self.scope.uri, self.scope.invocation, self.scope.fixture]).to_string(),
        );
        let keys = self
            .programs
            .iter()
            .filter_map(|program| Some((program.id, program.source_key.as_deref()?)))
            .collect::<BTreeMap<_, _>>();
        if let Some(programs) = catalog["programs"].as_array_mut() {
            for program in programs {
                if let Some(key) = program["id"].as_u64().and_then(|id| keys.get(&id)) {
                    program["sourceKey"] = json!(key);
                }
            }
        }
    }
}

impl SourceMaps {
    pub(crate) fn plan_clip_placement(
        &self,
        draft: &ClipDraft,
        authored_rsx: Option<&AuthoredRsxFrame>,
    ) -> Result<Value, String> {
        let frame = FrameIdentity {
            revision: draft.revision,
            sequence: draft.sequence,
        };
        let map = self
            .current
            .as_ref()
            .filter(|map| map.contains(&frame))
            .ok_or("Read the current app-hosted preview frame")?;
        let mut seen = BTreeSet::new();
        let clips = draft
            .programs
            .iter()
            .map(|id| {
                map.clips
                    .get(id)
                    .filter(|clip| clip.editable && seen.insert(*id))
                    .and_then(|clip| clip.clock)
                    .ok_or("Choose distinct current clips with editable delays")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let delays =
            clip_clock_delays(&clips, draft.start, draft.gap).map_err(|error| error.to_string())?;
        let placements = draft
            .programs
            .iter()
            .zip(delays)
            .map(|(&id, delay)| {
                NodeId::try_from(id)
                    .map(|id| (id, delay))
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let patch = if let Some(authored) = authored_rsx {
            authored.hosted_clip_patch(&placements)?
        } else {
            let mut document = map.document.clone();
            let edits = placements
                .into_iter()
                .map(|(id, delay)| xaml_clip_delay_edit(&document, id, delay))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            document
                .apply(
                    "Place animation clips",
                    edits.into_iter().flatten().collect(),
                )
                .map_err(|error| error.to_string())?;
            checked_source_patch(map.document.source(), document.source())?
        };
        let patches = patch.into_iter().map(source_patch).collect::<Vec<_>>();
        Ok(json!({ "patches": patches }))
    }
}
