use super::*;
use ferroui_controls::Flyout;

impl DesignerWorkspace {
    pub(super) fn rust_group_creator(
        self: &Rc<Self>,
        row: &StackPanel,
        owner: NodeId,
        accepted: &AcceptedSource,
    ) {
        self.rust_state_form(row, "Add state group", ADD, accepted, move |body| {
            let name = field(body, "Group name", "");
            let initial = field(body, "Initial state", "Normal");
            move |this: &Rc<Self>| {
                let name = text(&name);
                if this.state_group(owner, &name).is_ok() {
                    return Err("This control already has a group with that name".into());
                }
                let state = text(&initial);
                this.session
                    .borrow_mut()
                    .upsert_state_group(VisualStateGroup {
                        name: name.clone(),
                        owner,
                        states: vec![VisualState {
                            name: state.clone(),
                            setters: vec![],
                        }],
                        transitions: vec![],
                    })?;
                let mut authoring = this.authoring.borrow_mut();
                authoring.group = name;
                authoring.state = state;
                Ok(())
            }
        });
    }

    pub(super) fn rust_state_creator(
        self: &Rc<Self>,
        row: &StackPanel,
        group: Rc<VisualStateGroup>,
        accepted: &AcceptedSource,
    ) {
        self.rust_state_form(row, "Add state", ADD, accepted, move |body| {
            let name = field(body, "State name", "");
            move |this: &Rc<Self>| {
                let name = text(&name);
                let mut model = (*group).clone();
                model.states.push(VisualState {
                    name: name.clone(),
                    setters: vec![],
                });
                this.session.borrow_mut().upsert_state_group(model)?;
                this.authoring.borrow_mut().state = name;
                Ok(())
            }
        });
    }

    pub(super) fn rust_state_tools(
        self: &Rc<Self>,
        row: &StackPanel,
        group: Rc<VisualStateGroup>,
        index: usize,
        accepted: &AcceptedSource,
    ) {
        self.rust_state_form(row, "Add setter", ADD, accepted, {
            let group = group.clone();
            move |body| {
                let target = field(body, "Target (empty for the owner)", "");
                let property = field(body, "Property", "Opacity");
                let value = field(body, "Value", "0.5");
                move |this: &Rc<Self>| {
                    let mut model = (*group).clone();
                    model.states[index].setters.push(StateSetter {
                        target: text(&target),
                        property: text(&property),
                        value: text(&value),
                    });
                    this.session.borrow_mut().upsert_state_group(model)
                }
            }
        });
        if !group.states[index].setters.is_empty() {
            self.rust_state_form(row, "Remove setter", REMOVE, accepted, {
                let group = group.clone();
                move |body| {
                    let setters = group.states[index]
                        .setters
                        .iter()
                        .map(|setter| {
                            if setter.target.is_empty() {
                                setter.property.clone()
                            } else {
                                format!("{}.{}", setter.target, setter.property)
                            }
                        })
                        .collect::<Vec<_>>();
                    let choices = setters.iter().map(String::as_str).collect::<Vec<_>>();
                    let picker = choice_field(body, None, "Override", choices[0], &choices);
                    move |this: &Rc<Self>| {
                        let mut model = (*group).clone();
                        let setters = &mut model.states[index].setters;
                        let selected = picker.selected_index() as usize;
                        if selected >= setters.len() {
                            return Err("Choose a state override".into());
                        }
                        setters.remove(selected);
                        this.session.borrow_mut().upsert_state_group(model)
                    }
                }
            });
        }
        icon_action(
            row,
            "Remove state",
            REMOVE,
            self,
            accepted.clone(),
            move |this| {
                let mut model = (*group).clone();
                let state = model.states.remove(index);
                model.transitions.retain(|route| {
                    route.from.as_deref() != Some(&state.name) && route.to != state.name
                });
                this.session.borrow_mut().upsert_state_group(model)
            },
        );
    }

    pub(super) fn rust_route_creator(
        self: &Rc<Self>,
        row: &StackPanel,
        group: Rc<VisualStateGroup>,
        accepted: &AcceptedSource,
    ) {
        self.rust_state_form(row, "Add transition", ADD, accepted, move |body| {
            let names = group
                .states
                .iter()
                .map(|state| state.name.as_str())
                .collect::<Vec<_>>();
            let from = std::iter::once("Any state")
                .chain(names.iter().copied())
                .collect::<Vec<_>>();
            let start = choice_field(body, None, "From", from[0], &from);
            let end = choice_field(body, None, "To", names[0], &names);
            let duration = field(body, "Duration (seconds)", "0.2");
            let easing = field(body, "Easing", "LinearEasing");
            move |this: &Rc<Self>| {
                let mut model = (*group).clone();
                let from = (start.selected_index() > 0).then(|| choice(&start));
                let to = choice(&end);
                model.transitions.push(StateTransition {
                    from: from.clone(),
                    to: to.clone(),
                    duration: text(&duration)
                        .parse()
                        .map_err(|_| "Enter the duration in seconds")?,
                    easing: Easing::parse(&text(&easing)).map_err(|error| error.to_string())?,
                });
                this.session.borrow_mut().upsert_state_group(model)?;
                this.authoring.borrow_mut().rust_route = Some((from, to));
                Ok(())
            }
        });
    }

    fn rust_state_form<F>(
        self: &Rc<Self>,
        row: &StackPanel,
        title: &str,
        icon: &str,
        accepted: &AcceptedSource,
        build: impl FnOnce(&StackPanel) -> F,
    ) where
        F: Fn(&Rc<Self>) -> Result<(), String> + 'static,
    {
        let button = icon_button(title, icon);
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        let body = StackPanel::new();
        body.set_spacing(6.0);
        body.set_width(310.0);
        section(&body, title);
        let run = build(&body);
        let close = flyout.downgrade();
        icon_action(
            &toolbar(&body),
            "Apply",
            APPLY,
            self,
            accepted.clone(),
            move |this| {
                run(this)?;
                if let Some(flyout) = close.upgrade() {
                    flyout.hide();
                }
                Ok(())
            },
        );
        flyout.set_content(Some(Control::boxed(body)));
        button.set_flyout(Some(flyout.upcast()));
        row.children().add(button.upcast::<Control>());
    }
}
