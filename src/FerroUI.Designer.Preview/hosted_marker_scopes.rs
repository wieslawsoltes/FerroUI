use super::SourceMap;
use ferroui_designer_animation::AnimationProgramLocator;
use ferroui_designer_core::SourceAnchor;
use serde_json::{Value, json};

impl SourceMap {
    pub(crate) fn annotate_marker_scopes(&self, catalog: &mut Value) {
        let scope = &self.scope;
        catalog["markerScope"] = json!([
            scope.uri,
            scope.assembly,
            scope.project_path,
            scope.invocation,
            scope.fixture
        ])
        .to_string()
        .into();
        let Some(items) = catalog["programs"].as_array_mut() else {
            return;
        };
        for item in items {
            let Some(id) = item["id"].as_i64() else {
                continue;
            };
            let identity = if let Some(route) = self.state_routes.get(&id) {
                SourceAnchor::capture(&self.document, route.owner)
                    .ok()
                    .map(|owner| {
                        json!(["route", owner, route.group, route.from, route.to]).to_string()
                    })
            } else {
                self.programs
                    .iter()
                    .find(|program| i64::try_from(program.id).ok() == Some(id))
                    .and_then(|program| {
                        AnimationProgramLocator::preview_key(
                            &self.document,
                            &self.programs,
                            program,
                        )
                    })
            };
            if let Some(identity) = identity {
                item["markerKey"] = identity.into();
            }
        }
    }
}
