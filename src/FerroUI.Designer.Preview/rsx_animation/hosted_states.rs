use super::AuthoredRsxFrame;
use crate::hosted_animation::{StateDraft, source_patch};
use crate::protocol::MAX_SOURCE_BYTES;
use ferroui_designer_animation::{Easing, ProjectedStateGroup};
use ferroui_designer_core::{Document, NodeId, TextPatch};
use ferroui_designer_language::{LineIndex, Range};
use ferroui_remote_protocol::designer::{
    DesignAnimationCatalogMessage, DesignAnimationSourcePatch,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostedStateRoute {
    pub(crate) owner: NodeId,
    pub(crate) group: String,
    pub(crate) from: Option<String>,
    pub(crate) to: String,
    easing: String,
    source_range: Range,
}

impl AuthoredRsxFrame {
    pub(crate) fn bind_hosted_routes(
        &mut self,
        document: &Document,
        catalog: &mut DesignAnimationCatalogMessage,
    ) {
        let line_index = LineIndex::new(&self.source);
        let mut routes = BTreeMap::new();
        for group in ProjectedStateGroup::scan(document) {
            let key = (group.model.owner, group.model.name.clone());
            let Some(authored) = self.states.get(&key) else {
                continue;
            };
            for route in group.routes {
                let Ok(id) = i64::try_from(route.id) else {
                    continue;
                };
                let span = authored
                    .transition_span(route.transition.from.as_deref(), &route.transition.to);
                let Some(span) = span else {
                    continue;
                };
                let Ok(source_range) = line_index.range(span) else {
                    continue;
                };
                routes.insert(
                    id,
                    (
                        span.start,
                        HostedStateRoute {
                            owner: key.0,
                            group: key.1.clone(),
                            from: route.transition.from,
                            to: route.transition.to,
                            easing: route.transition.easing.to_xaml(),
                            source_range,
                        },
                    ),
                );
            }
            self.hosted_groups.insert(key, group.model);
        }
        for program in &mut catalog.programs {
            if let Some((offset, route)) = routes.remove(&program.id) {
                program.source_utf16 =
                    i32::try_from(self.source[..offset].encode_utf16().count()).unwrap_or(-1);
                self.hosted_routes.insert(program.id, route);
            } else {
                self.map_hosted_program(program);
            }
        }
    }

    pub(crate) fn annotate_hosted_routes(&self, catalog: &mut Value) {
        if let Some(programs) = catalog["programs"].as_array_mut() {
            for program in programs {
                if let Some(route) = program["id"]
                    .as_i64()
                    .and_then(|id| self.hosted_routes.get(&id))
                {
                    program["stateRoute"] = json!(route);
                }
            }
        }
    }

    pub(crate) fn plan_hosted_state(&self, draft: &StateDraft) -> Result<Value, String> {
        let route = self
            .hosted_routes
            .get(&draft.program)
            .ok_or("Choose a mapped Rust state route in the current application preview")?;
        let key = (route.owner, route.group.clone());
        let authored = self
            .states
            .get(&key)
            .ok_or("The state declaration changed")?;
        let mut group = self
            .hosted_groups
            .get(&key)
            .ok_or("The state group changed")?
            .clone();
        let transition = group
            .transitions
            .iter_mut()
            .find(|transition| transition.from == route.from && transition.to == route.to)
            .ok_or("The state route changed")?;
        transition.duration = draft.duration;
        transition.easing = Easing::parse(&draft.easing).map_err(|error| error.to_string())?;
        let patches = authored.patch_model(&group)?;
        let candidate = TextPatch::apply_all(&self.source, patches.clone())
            .map_err(|error| error.to_string())?;
        if candidate.len() > MAX_SOURCE_BYTES {
            return Err("Designer source exceeds 1 MiB".into());
        }
        let patches = patches
            .into_iter()
            .map(|patch| {
                DesignAnimationSourcePatch::from_utf8(
                    &self.source,
                    patch.span.range(),
                    patch.replacement,
                )
                .map(source_patch)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({ "patches": patches }))
    }
}
