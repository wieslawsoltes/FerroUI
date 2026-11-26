use crate::protocol::Language;
use crate::rsx_animation::AuthoredRsxFrame;
use ferroui_designer_animation::{StatePreviewGroup, StatePreviewRoute, VisualStateGroup};
use ferroui_designer_core::{Document, NodeId};
use ferroui_designer_language::{LineIndex, Range};
use ferroui_remote_protocol::designer::DesignAnimationCatalogMessage;
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
    pub(crate) endpoint: bool,
    easing: String,
    source_range: Range,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostedStateGroup {
    pub(crate) model: VisualStateGroup,
    source_range: Range,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StateOwner {
    pub(crate) id: NodeId,
    pub(crate) can_create: bool,
    label: String,
    type_name: String,
    source_line: u32,
}

pub(crate) struct HostedStates {
    language: Language,
    pub(crate) owners: Option<Vec<StateOwner>>,
    pub(crate) groups: Vec<HostedStateGroup>,
    pub(crate) routes: BTreeMap<i64, HostedStateRoute>,
}

impl HostedStates {
    pub(crate) fn capture(
        document: &Document,
        catalog: &mut DesignAnimationCatalogMessage,
        rust: Option<&AuthoredRsxFrame>,
    ) -> Self {
        let language = if rust.is_some() {
            Language::Rust
        } else {
            Language::Xaml
        };
        let full = catalog.state_authoring == Some(true);
        let source = rust.map_or_else(|| document.source(), AuthoredRsxFrame::source_text);
        let index = LineIndex::new(source);
        let owners = full.then(|| {
            catalog
                .targets
                .iter()
                .filter_map(|control| {
                    let id = NodeId::try_from(control.id).ok()?;
                    let node = document.ast().node(id)?;
                    let (source_line, can_create) = match rust {
                        Some(rust) => rust.hosted_state_owner(id)?,
                        None => (index.position(node.name_span.start).ok()?.line + 1, true),
                    };
                    Some(StateOwner {
                        id,
                        label: control.label.clone(),
                        type_name: control.type_name.clone(),
                        source_line,
                        can_create,
                    })
                })
                .collect()
        });
        let playable = |id| catalog.programs.iter().any(|program| program.id == id);
        let mut routes = BTreeMap::new();
        let mut groups = Vec::new();
        for group in StatePreviewGroup::scan(document, full) {
            let span = match rust {
                Some(rust) => rust.hosted_state_span(&group.model, None),
                None => Some(group.span),
            };
            let Some(source_range) = span.and_then(|span| index.range(span).ok()) else {
                continue;
            };
            for route in &group.routes {
                let Some(id) = i64::try_from(route.id).ok().filter(|&id| playable(id)) else {
                    continue;
                };
                let span = match rust {
                    Some(rust) => rust.hosted_state_span(&group.model, Some(route)),
                    None => Some(route.span),
                };
                if let Some(span) = span {
                    if let Ok(range) = index.range(span) {
                        routes.insert(id, (span.start, descriptor(&group.model, route, range)));
                    }
                }
            }
            groups.push(HostedStateGroup {
                model: group.model,
                source_range,
            });
        }
        for program in &mut catalog.programs {
            if let Some((offset, _)) = routes.get(&program.id) {
                program.source_utf16 =
                    i32::try_from(source[..*offset].encode_utf16().count()).unwrap_or(-1);
            } else if let Some(rust) = rust {
                rust.map_hosted_program(program);
            }
        }
        let routes = routes
            .into_iter()
            .map(|(id, (_, route))| (id, route))
            .collect();
        Self {
            language,
            owners,
            groups,
            routes,
        }
    }

    pub(crate) fn annotate(&self, catalog: &mut Value) {
        if let Some(owners) = &self.owners {
            let groups = self
                .groups
                .iter()
                .filter(|group| owners.iter().any(|owner| owner.id == group.model.owner))
                .collect::<Vec<_>>();
            catalog["stateAuthoring"] =
                json!({ "language": self.language, "owners": owners, "groups": groups });
        }
        if let Some(programs) = catalog["programs"].as_array_mut() {
            for program in programs {
                if let Some(route) = program["id"].as_i64().and_then(|id| self.routes.get(&id)) {
                    program["stateRoute"] = json!(route);
                }
            }
        }
    }
}

fn descriptor(
    model: &VisualStateGroup,
    route: &StatePreviewRoute,
    source_range: Range,
) -> HostedStateRoute {
    HostedStateRoute {
        owner: model.owner,
        group: model.name.clone(),
        from: route.transition.from.clone(),
        to: route.transition.to.clone(),
        endpoint: route.endpoint,
        easing: route.transition.easing.to_xaml(),
        source_range,
    }
}
