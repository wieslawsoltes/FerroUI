use super::*;
use ferroui_designer_language::LineIndex;

impl DesignerWorkspace {
    pub(super) fn build_rust_state_triggers(
        self: &Rc<Self>,
        panel: &StackPanel,
        source: NodeId,
        accepted: &AcceptedSource,
        writable: bool,
    ) {
        let Some(report) = self.session.borrow().rust_state_triggers(source) else {
            return;
        };
        let destinations = report
            .groups
            .into_iter()
            .flat_map(|group| {
                group
                    .states
                    .into_iter()
                    .map(move |state| (group.group.clone(), state))
            })
            .collect::<Vec<_>>();
        let can_link = writable && !report.events.is_empty() && !destinations.is_empty();
        if !can_link && report.triggers.is_empty() {
            return;
        }
        section(panel, "Event triggers");
        for trigger in report.triggers {
            let row = toolbar(panel);
            description(
                &row,
                &format!("{} → {} · {}", trigger.event, trigger.group, trigger.state),
            );
            let range = trigger.range;
            icon_action(
                &row,
                "Reveal Rust event trigger",
                REVEAL,
                self,
                accepted.clone(),
                move |this| {
                    let span = LineIndex::new(this.session.borrow().authored_source())
                        .span(range)
                        .map_err(|error| error.to_string())?;
                    this.select_source_span(span);
                    Ok(())
                },
            );
            if writable && report.events.contains(&trigger.event) {
                icon_action(
                    &row,
                    "Remove event trigger",
                    REMOVE,
                    self,
                    accepted.clone(),
                    move |this| {
                        this.session.borrow_mut().set_rust_state_trigger(
                            source,
                            &trigger.event,
                            &trigger.group,
                            None,
                        )
                    },
                );
            }
        }
        if can_link {
            let events = report.events.iter().map(String::as_str).collect::<Vec<_>>();
            let event = choice_field(
                panel,
                Some("RustStateTriggerEvent"),
                "Routed event",
                if events.contains(&"Click") {
                    "Click"
                } else {
                    events[0]
                },
                &events,
            );
            let labels = destinations
                .iter()
                .map(|(group, state)| format!("{group} · {state}"))
                .collect::<Vec<_>>();
            let labels = labels.iter().map(String::as_str).collect::<Vec<_>>();
            let destination = choice_field(
                panel,
                Some("RustStateTriggerDestination"),
                "Destination",
                labels[0],
                &labels,
            );
            let row = toolbar(panel);
            icon_action(
                &row,
                "Link event to state",
                ADD,
                self,
                accepted.clone(),
                move |this| {
                    let selected = destination.selected_index();
                    let (group, state) = destinations
                        .get(selected as usize)
                        .ok_or("Choose a destination state")?;
                    this.session.borrow_mut().set_rust_state_trigger(
                        source,
                        &choice(&event),
                        group,
                        Some(state),
                    )
                },
            );
        }
    }
}
