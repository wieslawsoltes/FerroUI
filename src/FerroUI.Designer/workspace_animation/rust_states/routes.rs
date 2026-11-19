use super::*;

impl DesignerWorkspace {
    pub(super) fn build_rust_state_routes(
        self: &Rc<Self>,
        panel: &StackPanel,
        group: Rc<VisualStateGroup>,
        accepted: AcceptedSource,
    ) {
        if group.transitions.is_empty() {
            return;
        }
        section(panel, "Transitions");
        let labels = group
            .transitions
            .iter()
            .map(|route| format!("{} → {}", route.from.as_deref().unwrap_or("Base"), route.to))
            .collect::<Vec<_>>();
        let active = self.authoring.borrow().rust_route.clone();
        let initial = group
            .transitions
            .iter()
            .position(|route| {
                active
                    .as_ref()
                    .is_some_and(|(from, to)| &route.from == from && &route.to == to)
            })
            .unwrap_or(0);
        let items = labels.iter().map(String::as_str).collect::<Vec<_>>();
        let picker = choice_field(
            panel,
            Some("RustStateRoutes"),
            "Route",
            items[initial],
            &items,
        );
        let route = &group.transitions[initial];
        let duration = field(panel, "Duration (seconds)", &route.duration.to_string());
        let ease = field(panel, "Easing", &route.easing.to_xaml());
        let elapsed = field(panel, "Playhead (seconds)", "0");
        let weak_picker = picker.downgrade();
        let weak = Rc::downgrade(self);
        picker.selection_changed({
            let (group, duration, ease, elapsed) = (
                group.clone(),
                duration.clone(),
                ease.clone(),
                elapsed.clone(),
            );
            move |_, _| {
                let (Some(picker), Some(this)) = (weak_picker.upgrade(), weak.upgrade()) else {
                    return;
                };
                let Some(route) = group.transitions.get(picker.selected_index() as usize) else {
                    return;
                };
                this.authoring.borrow_mut().rust_route =
                    Some((route.from.clone(), route.to.clone()));
                duration.set_text(Some(&route.duration.to_string()));
                ease.set_text(Some(&route.easing.to_xaml()));
                elapsed.set_text(Some("0"));
            }
        });
        let tools = toolbar(panel);
        let index = || "Choose a Rust state route".to_owned();
        icon_action(
            &tools,
            "Seek state transition",
            SEEK,
            self,
            accepted.clone(),
            {
                let (group, picker) = (group.clone(), picker.clone());
                move |this| {
                    let route = group
                        .transitions
                        .get(picker.selected_index() as usize)
                        .ok_or_else(index)?;
                    let time = text(&elapsed)
                        .parse()
                        .map_err(|_| "Enter a time within the transition")?;
                    this.session.borrow_mut().preview_state_transition(
                        group.owner,
                        &group.name,
                        route.from.as_deref(),
                        &route.to,
                        time,
                    )
                }
            },
        );
        icon_action(
            &tools,
            "Apply transition settings",
            APPLY,
            self,
            accepted.clone(),
            {
                let (group, picker) = (group.clone(), picker.clone());
                move |this| {
                    let mut model = (*group).clone();
                    let route = model
                        .transitions
                        .get_mut(picker.selected_index() as usize)
                        .ok_or_else(index)?;
                    route.duration = text(&duration)
                        .parse()
                        .map_err(|_| "Enter the duration in seconds")?;
                    route.easing =
                        Easing::parse(&text(&ease)).map_err(|error| error.to_string())?;
                    this.session.borrow_mut().upsert_state_group(model)
                }
            },
        );
        icon_action(
            &tools,
            "Reveal Rust transition source",
            REVEAL,
            self,
            accepted.clone(),
            move |this| {
                let route = group
                    .transitions
                    .get(picker.selected_index() as usize)
                    .ok_or_else(index)?;
                this.reveal_rust_state_source(
                    accepted.revision,
                    group.owner,
                    &group.name,
                    route.from.as_deref(),
                    Some(&route.to),
                )
            },
        );
    }
}
