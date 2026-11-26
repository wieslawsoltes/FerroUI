use crate::RsxProjection;
use ferroui_designer_animation::ProjectedStateTrigger;
use std::collections::BTreeMap;

/// Marker fragments carried with the mapped controls into a native preview.
#[derive(Default)]
pub(crate) struct TriggerMarkup(BTreeMap<usize, Vec<String>>);

impl TriggerMarkup {
    pub(crate) fn project(projection: &RsxProjection<'_>) -> Self {
        let mut markup = BTreeMap::<_, Vec<_>>::new();
        for (source, node) in projection.nodes.iter().enumerate() {
            let triggers = &node.state_triggers;
            if triggers.opaque || triggers.items.is_empty() {
                continue;
            }
            let groups = projection.state_trigger_groups(source);
            for trigger in &triggers.items {
                if groups.iter().any(|(_, group)| {
                    group.name == trigger.group
                        && group.states.iter().any(|state| state.name == trigger.state)
                }) {
                    let marker = ProjectedStateTrigger::comment(
                        &trigger.event,
                        &trigger.group,
                        &trigger.state,
                    );
                    markup.entry(source).or_default().push(marker);
                }
            }
        }
        Self(markup)
    }

    pub(crate) fn comments(&self, index: usize) -> &[String] {
        self.0.get(&index).map_or(&[], Vec::as_slice)
    }
}
