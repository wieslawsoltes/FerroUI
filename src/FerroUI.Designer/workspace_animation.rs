//! Native program, visual-state and behavior authoring panels.
use super::*;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{ColumnDefinitions, ComboBox, ScrollViewer, ToolTip};
use ferroui_designer_animation::{
    BehaviorAction, BehaviorModel, BehaviorTrigger, FillMode, PlaybackDirection, Repeat,
    StateSetter, StateTransition, VisualState, VisualStateGroup,
};

#[derive(Default)]
pub(super) struct AuthoringUi {
    mode: u8,
    group: String,
    state: String,
}
fn field(panel: &StackPanel, label: &str, value: &str) -> Ref<TextBox> {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("104,*").unwrap());
    let title = TextBlock::new();
    let caption = label.split(" (").next().unwrap_or(label);
    title.set_text(Some(caption));
    title.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
    title.set_font_size(11.0);
    title.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    title.set_text_wrapping(TextWrapping::Wrap);
    title.set_margin(Thickness::new(0.0, 0.0, 8.0, 0.0));
    row.children().add(title.upcast::<Control>());
    let text = TextBox::new();
    text.set_text(Some(value));
    text.classes().add("designer-field");
    text.set_min_width(0.0);
    AutomationProperties::set_name(&text, Some(label));
    ToolTip::set_tip(&text, Some(Rc::new(label.to_owned())));
    Grid::set_column(&text, 1);
    row.children().add(text.clone().upcast::<Control>());
    panel.children().add(row.upcast::<Control>());
    text
}
fn choice_field(panel: &StackPanel, label: &str, value: &str, choices: &[&str]) -> Ref<ComboBox> {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("104,*").unwrap());
    let title = TextBlock::new();
    title.set_text(Some(label));
    title.set_font_size(11.0);
    title.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
    title.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    row.children().add(title.upcast::<Control>());
    let picker = ComboBox::new();
    picker.classes().add("designer-field");
    picker.set_horizontal_alignment(HorizontalAlignment::Stretch);
    for choice in choices {
        picker.items().add(Some(Rc::new((*choice).to_owned())));
    }
    picker.set_selected_index(
        choices
            .iter()
            .position(|choice| *choice == value)
            .map_or(-1, |index| index as i32),
    );
    AutomationProperties::set_name(&picker, Some(label));
    Grid::set_column(&picker, 1);
    row.children().add(picker.clone().upcast::<Control>());
    panel.children().add(row.upcast::<Control>());
    picker
}
fn choice(picker: &ComboBox) -> String {
    picker
        .selected_item()
        .and_then(|value| value.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}
fn section(panel: &StackPanel, title: &str) {
    let label = TextBlock::new();
    label.set_text(Some(title));
    label.set_font_size(11.0);
    label.set_font_weight(FontWeight::SemiBold);
    label.set_margin(Thickness::new(
        0.0,
        if panel.children().is_empty() {
            2.0
        } else {
            12.0
        },
        0.0,
        2.0,
    ));
    panel.children().add(label.upcast::<Control>());
}
fn description(panel: &StackPanel, value: &str) {
    let label = TextBlock::new();
    label.set_text(Some(value));
    label.set_font_size(11.0);
    label.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
    label.set_text_wrapping(TextWrapping::Wrap);
    label.set_margin(Thickness::new(0.0, 0.0, 0.0, 6.0));
    panel.children().add(label.upcast::<Control>());
}
fn code_field(panel: &StackPanel, label: &str, value: &str) -> Ref<TextBox> {
    section(panel, label);
    let text = TextBox::new();
    text.set_text(Some(value));
    text.classes().add("designer-field");
    text.set_font_family(ferroui_base::media::FontFamily::from("monospace"));
    text.set_font_size(11.0);
    text.set_accepts_return(true);
    text.set_min_height(112.0);
    AutomationProperties::set_name(&text, Some(label));
    panel.children().add(text.clone().upcast::<Control>());
    text
}
fn action(
    panel: &StackPanel,
    title: &str,
    workspace: &Rc<DesignerWorkspace>,
    run: impl Fn(&Rc<DesignerWorkspace>) -> Result<(), String> + 'static,
) {
    let row = button(title);
    row.classes().add("designer-authoring-action");
    if title.starts_with("Apply") || title.starts_with("Attach") {
        row.classes().add("designer-motion");
    } else {
        row.classes().add("nav");
    }
    row.set_horizontal_alignment(HorizontalAlignment::Stretch);
    row.set_horizontal_content_alignment(HorizontalAlignment::Left);
    let weak = Rc::downgrade(workspace);
    row.click(move |_, _| {
        if let Some(this) = weak.upgrade() {
            let result = run(&this);
            this.refresh(true);
            this.report(result);
        }
    });
    panel.children().add(row.upcast::<Control>());
}
fn text(control: &TextBox) -> String {
    control.text().unwrap_or_default()
}
fn optional(control: &TextBox) -> Option<String> {
    let value = text(control);
    (!value.is_empty()).then_some(value)
}
impl DesignerWorkspace {
    pub(super) fn connect_authoring(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        let picker = self.root.get_control::<ComboBox>("AnimationPrograms");
        let picker_weak = picker.downgrade();
        picker.selection_changed(move |_, _| {
            let (Some(this), Some(picker)) = (weak.upgrade(), picker_weak.upgrade()) else {
                return;
            };
            if this.guard.get() {
                return;
            }
            let id = this
                .animation_program_ids
                .borrow()
                .get(picker.selected_index().max(0) as usize)
                .copied();
            if let Some(id) = id {
                let result = this.session.borrow_mut().select_animation_program(id);
                this.refresh(true);
                this.report(result);
            }
        });
        for name in [
            "NewAnimation",
            "RenameAnimation",
            "DuplicateAnimation",
            "DeleteAnimation",
            "SetAnimationSelector",
            "AnimationSource",
            "AnimationSettings",
            "PlayAll",
            "StatesMode",
            "BehaviorsMode",
            "CloseAuthoring",
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<Button>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let result = this.authoring_command(name);
                    this.refresh(true);
                    this.report(result);
                }
            });
        }
    }
    fn authoring_command(self: &Rc<Self>, command: &str) -> Result<(), String> {
        let owner = self
            .session
            .borrow()
            .selection()
            .first()
            .copied()
            .unwrap_or(self.session.borrow().document().ast().root());
        let name = text(&self.root.get_control::<TextBox>("AnimationName"));
        let selector = text(&self.root.get_control::<TextBox>("AnimationSelector"));
        let active = self.session.borrow().active_animation_program();
        match command {
            "NewAnimation" => {
                let session = self.session.borrow();
                let programs = session.animation_programs();
                let active = programs.iter().find(|program| Some(program.id) == active);
                let name = if name.trim().is_empty()
                    || programs.iter().any(|program| program.name == name)
                {
                    (1..)
                        .map(|index| format!("Animation {index}"))
                        .find(|name| programs.iter().all(|program| program.name != *name))
                        .expect("a finite document has an available animation name")
                } else {
                    name
                };
                let selector = if selector.trim().is_empty()
                    || active.is_some_and(|program| program.selector == selector)
                {
                    let node = session
                        .document()
                        .ast()
                        .node(owner)
                        .ok_or("Select an animation owner")?;
                    let type_name = node.name.replace(':', "|");
                    node.xaml_name()
                        .map_or(type_name.clone(), |name| format!("{type_name}#{name}"))
                } else {
                    selector
                };
                let duration = session.timeline().duration;
                drop(session);
                self.session
                    .borrow_mut()
                    .create_animation_program(&name, owner, &selector, duration)
                    .map(|_| ())
            }
            "RenameAnimation" => self
                .session
                .borrow_mut()
                .rename_animation_program(active.ok_or("Choose an animation")?, &name),
            "DuplicateAnimation" => {
                let id = active.ok_or("Choose an animation")?;
                let session = self.session.borrow();
                let programs = session.animation_programs();
                let name = if name.trim().is_empty()
                    || programs.iter().any(|program| program.name == name)
                {
                    let original = programs
                        .iter()
                        .find(|program| program.id == id)
                        .ok_or("Choose an animation")?;
                    let base = format!("{} copy", original.name);
                    std::iter::once(base.clone())
                        .chain((2..).map(|index| format!("{base} {index}")))
                        .find(|name| programs.iter().all(|program| program.name != *name))
                        .expect("a finite document has an available animation name")
                } else {
                    name
                };
                drop(session);
                self.session
                    .borrow_mut()
                    .duplicate_animation_program(id, &name)
                    .map(|_| ())
            }
            "DeleteAnimation" => self
                .session
                .borrow_mut()
                .remove_animation_program(active.ok_or("Choose an animation")?),
            "SetAnimationSelector" => self
                .session
                .borrow_mut()
                .set_animation_program_selector(active.ok_or("Choose an animation")?, &selector),
            "AnimationSource" => {
                if self.view_mode() == crate::WorkspaceViewMode::Design {
                    self.set_view_mode(crate::WorkspaceViewMode::Split);
                }
                self.reveal_source(active.ok_or("Choose an animation")?);
                Ok(())
            }
            "PlayAll" => {
                self.session.borrow_mut().play_all_animation_programs();
                Ok(())
            }
            "StatesMode" => {
                self.authoring.borrow_mut().mode = 1;
                Ok(())
            }
            "BehaviorsMode" => {
                self.authoring.borrow_mut().mode = 2;
                Ok(())
            }
            "AnimationSettings" => {
                self.authoring.borrow_mut().mode = 3;
                Ok(())
            }
            _ => {
                self.authoring.borrow_mut().mode = 0;
                self.session.borrow_mut().clear_interaction_preview();
                Ok(())
            }
        }
    }
    pub(super) fn refresh_animation_programs(&self) {
        let session = self.session.borrow();
        let picker = self.root.get_control::<ComboBox>("AnimationPrograms");
        let previous = self.guard.replace(true);
        picker.items().clear();
        let mut ids = Vec::new();
        for program in session.animation_programs() {
            ids.push(program.id);
            picker.items().add(Some(Rc::new(format!(
                "{}{}",
                program.name,
                if program.timeline.is_none() {
                    " · source"
                } else {
                    ""
                }
            ))));
        }
        picker.set_selected_index(
            session
                .active_animation_program()
                .and_then(|id| ids.iter().position(|candidate| *candidate == id))
                .map_or(-1, |index| index as i32),
        );
        if let Some(program) = session
            .animation_programs()
            .iter()
            .find(|program| Some(program.id) == session.active_animation_program())
        {
            ToolTip::set_tip(
                &picker,
                Some(Rc::new(format!(
                    "{}\n{} target{} · {}\nSelect the animation to edit and preview",
                    program.name,
                    program.targets.len(),
                    if program.targets.len() == 1 { "" } else { "s" },
                    program.selector,
                ))),
            );
            let name = self.root.get_control::<TextBox>("AnimationName");
            if !name.is_focused() {
                name.set_text(Some(&program.name));
            }
            let selector = self.root.get_control::<TextBox>("AnimationSelector");
            if !selector.is_focused() {
                selector.set_text(Some(&program.selector));
            }
        } else {
            ToolTip::set_tip(
                &picker,
                Some(Rc::new(
                    "Create an animation or add a keyframe to the selected object".to_owned(),
                )),
            );
        }
        *self.animation_program_ids.borrow_mut() = ids;
        self.guard.set(previous);
    }
    pub(super) fn rebuild_authoring(self: &Rc<Self>) {
        let panel = self.root.get_control::<StackPanel>("AuthoringPanel");
        panel.children().clear();
        panel.set_spacing(8.0);
        let mode = self.authoring.borrow().mode;
        self.root
            .get_control::<Grid>("InspectorDesignPane")
            .set_is_visible(mode == 0);
        self.root
            .get_control::<ScrollViewer>("AuthoringScroll")
            .set_is_visible(mode != 0);
        for (name, active) in [
            ("CloseAuthoring", 0),
            ("StatesMode", 1),
            ("BehaviorsMode", 2),
        ] {
            self.root
                .get_control::<Button>(name)
                .classes()
                .set("active", mode == active);
        }
        if let Some(title) = self.root.find_control::<TextBlock>("InspectorModeTitle") {
            title.set_text(Some(match mode {
                1 => "Visual states",
                2 => "Behaviors",
                3 => "Animation settings",
                _ => "Properties",
            }));
        }
        let owner = self
            .session
            .borrow()
            .selection()
            .first()
            .copied()
            .unwrap_or(self.session.borrow().document().ast().root());
        match mode {
            1 => self.build_state_panel(&panel, owner),
            2 => self.build_behavior_panel(&panel, owner),
            3 => self.build_animation_settings(&panel),
            _ => {}
        }
    }
    fn build_animation_settings(self: &Rc<Self>, panel: &StackPanel) {
        let Some(id) = self.session.borrow().active_animation_program() else {
            return;
        };
        let timeline = self.session.borrow().timeline().clone();
        section(panel, "Timing");
        description(
            panel,
            "Control the selected animation. Duration changes scale every keyframe together.",
        );
        let duration = field(panel, "Duration (seconds)", &timeline.duration.to_string());
        let delay = field(panel, "Delay (seconds)", &timeline.delay.to_string());
        let iteration_delay = field(
            panel,
            "Repeat delay (seconds)",
            &timeline.delay_between_iterations.to_string(),
        );
        section(panel, "Playback");
        let playback = choice_field(
            panel,
            "Behavior",
            &format!("{:?}", timeline.playback_behavior),
            &["Auto", "Always", "OnlyIfVisible"],
        );
        let speed = field(panel, "Playback speed", &timeline.speed_ratio.to_string());
        let repeat = field(
            panel,
            "Repeat (count or Infinite)",
            &match timeline.repeat {
                Repeat::Count(n) => n.to_string(),
                Repeat::Infinite => "Infinite".into(),
            },
        );
        let direction = choice_field(
            panel,
            "Direction",
            &format!("{:?}", timeline.playback_direction),
            &["Normal", "Reverse", "Alternate", "AlternateReverse"],
        );
        let fill = choice_field(
            panel,
            "Fill mode",
            &format!("{:?}", timeline.fill),
            &["None", "Forward", "Backward", "Both"],
        );
        let easing = field(
            panel,
            "Easing (native name, x1,y1,x2,y2 or Spring(mass,stiffness,damping,velocity))",
            &timeline.easing.to_xaml(),
        );
        action(panel, "Apply animation settings", self, move |this| {
            let mut timeline = timeline.clone();
            let next_duration: f64 = text(&duration).parse().map_err(|_| "Invalid duration")?;
            if !next_duration.is_finite() || next_duration <= 0.0 {
                return Err("Duration must be finite and positive".into());
            }
            let scale = next_duration / timeline.duration;
            for track in &mut timeline.tracks {
                for key in &mut track.keyframes {
                    key.time *= scale;
                }
            }
            timeline.duration = next_duration;
            timeline.delay = text(&delay).parse().map_err(|_| "Invalid delay")?;
            timeline.delay_between_iterations = text(&iteration_delay)
                .parse()
                .map_err(|_| "Invalid iteration delay")?;
            timeline.playback_behavior = match choice(&playback).as_str() {
                "Auto" => ferroui_designer_animation::PlaybackBehavior::Auto,
                "Always" => ferroui_designer_animation::PlaybackBehavior::Always,
                "OnlyIfVisible" => ferroui_designer_animation::PlaybackBehavior::OnlyIfVisible,
                _ => return Err("Invalid playback behavior".into()),
            };
            timeline.speed_ratio = text(&speed).parse().map_err(|_| "Invalid speed")?;
            timeline.repeat = if text(&repeat).eq_ignore_ascii_case("Infinite") {
                Repeat::Infinite
            } else {
                Repeat::Count(text(&repeat).parse().map_err(|_| "Invalid repeat count")?)
            };
            timeline.playback_direction = match choice(&direction).as_str() {
                "Normal" => PlaybackDirection::Normal,
                "Reverse" => PlaybackDirection::Reverse,
                "Alternate" => PlaybackDirection::Alternate,
                "AlternateReverse" => PlaybackDirection::AlternateReverse,
                _ => return Err("Invalid playback direction".into()),
            };
            timeline.fill = match choice(&fill).as_str() {
                "None" => FillMode::None,
                "Forward" => FillMode::Forward,
                "Backward" => FillMode::Backward,
                "Both" => FillMode::Both,
                _ => return Err("Invalid fill mode".into()),
            };
            timeline.easing = Easing::parse(&text(&easing)).map_err(|e| e.to_string())?;
            this.session
                .borrow_mut()
                .update_animation_program(id, timeline)
        });
        section(panel, "Preview targets");
        action(panel, "Preview on current selection", self, move |this| {
            let targets = this.session.borrow().selection().to_vec();
            this.session
                .borrow_mut()
                .set_animation_program_preview_targets(id, Some(targets))
        });
        action(
            panel,
            "Restore selector preview targets",
            self,
            move |this| {
                this.session
                    .borrow_mut()
                    .set_animation_program_preview_targets(id, None)
            },
        );
        let source = self
            .session
            .borrow()
            .document()
            .ast()
            .node(id)
            .map(|node| {
                self.session.borrow().document().source()[node.span.start..node.span.end].to_owned()
            })
            .unwrap_or_default();
        let editor = code_field(panel, "Animation XAML", &source);
        action(panel, "Apply animation XAML", self, move |this| {
            this.session
                .borrow_mut()
                .set_animation_program_source(id, &text(&editor))
        });
    }
    fn build_state_panel(self: &Rc<Self>, panel: &StackPanel, owner: NodeId) {
        section(panel, "State groups");
        description(
            panel,
            "Define how the selected element changes for an interaction. Choose a state to preview its property overrides.",
        );
        let groups = self
            .session
            .borrow()
            .state_groups()
            .into_iter()
            .filter(|group| group.owner == owner)
            .collect::<Vec<_>>();
        let group_name = field(panel, "New state group", "Common");
        action(panel, "Add group", self, move |this| {
            let name = text(&group_name);
            this.authoring.borrow_mut().group = name.clone();
            this.session
                .borrow_mut()
                .upsert_state_group(VisualStateGroup {
                    name,
                    owner,
                    states: vec![VisualState {
                        name: "Normal".into(),
                        setters: vec![],
                    }],
                    transitions: vec![],
                })
        });
        for group in &groups {
            let name = group.name.clone();
            action(panel, &format!("Group: {name}"), self, move |this| {
                this.authoring.borrow_mut().group = name.clone();
                Ok(())
            });
        }
        let group = groups
            .iter()
            .find(|group| group.name == self.authoring.borrow().group)
            .or_else(|| groups.first())
            .cloned();
        let Some(group) = group else {
            return;
        };
        section(panel, &format!("{} states", group.name));
        let group_key = group.name.clone();
        let remove_name = group.name.clone();
        action(panel, "Delete group", self, move |this| {
            this.session
                .borrow_mut()
                .remove_state_group(owner, &remove_name)
        });
        let new_state = field(panel, "New state name", "Active");
        let key = group_key.clone();
        action(panel, "Add state", self, move |this| {
            let mut group = this.state_group(owner, &key)?;
            let name = text(&new_state);
            group.states.push(VisualState {
                name: name.clone(),
                setters: vec![],
            });
            this.authoring.borrow_mut().state = name;
            this.session.borrow_mut().upsert_state_group(group)
        });
        for state in &group.states {
            let name = state.name.clone();
            let key = group_key.clone();
            action(
                panel,
                &format!("State: {name} · preview"),
                self,
                move |this| {
                    this.authoring.borrow_mut().state = name.clone();
                    this.session
                        .borrow_mut()
                        .preview_state(owner, &key, Some(&name))
                },
            );
        }
        let state = group
            .states
            .iter()
            .find(|state| state.name == self.authoring.borrow().state)
            .or_else(|| group.states.first())
            .cloned();
        let Some(state) = state else {
            return;
        };
        section(panel, &format!("{} overrides", state.name));
        let state_key = state.name.clone();
        let key = group_key.clone();
        let state_name = state_key.clone();
        action(panel, "Delete state", self, move |this| {
            let mut group = this.state_group(owner, &key)?;
            group.states.retain(|state| state.name != state_name);
            group.transitions.retain(|transition| {
                transition.to != state_name && transition.from.as_deref() != Some(&state_name)
            });
            this.session.borrow_mut().upsert_state_group(group)
        });
        for (index, setter) in state.setters.iter().enumerate() {
            let target = field(panel, "Target Name (empty = owner)", &setter.target);
            let property = field(panel, "Property", &setter.property);
            let value = field(panel, "State value", &setter.value);
            let key = group_key.clone();
            let name = state_key.clone();
            action(panel, "Update setter", self, move |this| {
                let mut group = this.state_group(owner, &key)?;
                let state = group
                    .states
                    .iter_mut()
                    .find(|state| state.name == name)
                    .ok_or("State changed")?;
                let setter = state.setters.get_mut(index).ok_or("Setter changed")?;
                *setter = StateSetter {
                    target: text(&target),
                    property: text(&property),
                    value: text(&value),
                };
                this.session.borrow_mut().upsert_state_group(group)
            });
            let key = group_key.clone();
            let name = state_key.clone();
            action(panel, "Remove setter", self, move |this| {
                let mut group = this.state_group(owner, &key)?;
                let state = group
                    .states
                    .iter_mut()
                    .find(|state| state.name == name)
                    .ok_or("State changed")?;
                if index < state.setters.len() {
                    state.setters.remove(index);
                }
                this.session.borrow_mut().upsert_state_group(group)
            });
        }
        section(panel, "New property override");
        let target = field(panel, "Target (empty = selected object)", "");
        let property = field(panel, "Property", "Opacity");
        let value = field(panel, "Value", "0.5");
        let key = group_key.clone();
        let name = state_key.clone();
        action(panel, "Add setter", self, move |this| {
            let mut group = this.state_group(owner, &key)?;
            group
                .states
                .iter_mut()
                .find(|state| state.name == name)
                .ok_or("State changed")?
                .setters
                .push(StateSetter {
                    target: text(&target),
                    property: text(&property),
                    value: text(&value),
                });
            this.session.borrow_mut().upsert_state_group(group)
        });
        section(panel, "Transitions");
        for (index, transition) in group.transitions.iter().enumerate() {
            let key = group_key.clone();
            action(
                panel,
                &format!(
                    "Remove transition {} → {} ({:.2}s)",
                    transition.from.as_deref().unwrap_or("Any"),
                    transition.to,
                    transition.duration
                ),
                self,
                move |this| {
                    let mut group = this.state_group(owner, &key)?;
                    if index < group.transitions.len() {
                        group.transitions.remove(index);
                    }
                    this.session.borrow_mut().upsert_state_group(group)
                },
            );
        }
        let from = field(panel, "Transition from (empty = any)", "");
        let to = field(panel, "Transition to", &state_key);
        let seconds = field(panel, "Transition duration", "0.3");
        let easing = field(panel, "Transition easing", "LinearEasing");
        let key = group_key.clone();
        action(panel, "Add transition", self, move |this| {
            let mut group = this.state_group(owner, &key)?;
            group.transitions.push(StateTransition {
                from: optional(&from),
                to: text(&to),
                duration: text(&seconds).parse().map_err(|_| "Invalid duration")?,
                easing: Easing::parse(&text(&easing)).map_err(|e| e.to_string())?,
            });
            this.session.borrow_mut().upsert_state_group(group)
        });
        section(panel, "Event trigger");
        let event = field(panel, "Event", "Click");
        let key = group_key;
        let state = state_key;
        action(panel, "Attach state event trigger", self, move |this| {
            let actions = this
                .session
                .borrow()
                .state_group_actions(owner, &key, &state)?;
            this.session
                .borrow_mut()
                .upsert_behavior(BehaviorModel {
                    id: None,
                    owner,
                    trigger: BehaviorTrigger::Event {
                        event: text(&event),
                        source: None,
                    },
                    actions,
                })
                .map(|_| ())
        });
    }
    fn state_group(&self, owner: NodeId, name: &str) -> Result<VisualStateGroup, String> {
        self.session
            .borrow()
            .state_groups()
            .into_iter()
            .find(|group| group.owner == owner && group.name == name)
            .ok_or_else(|| "State group changed".into())
    }
    fn build_behavior_panel(self: &Rc<Self>, panel: &StackPanel, owner: NodeId) {
        section(panel, "Trigger");
        description(
            panel,
            "Connect an event or condition to an action. Custom trigger and action types remain editable by name.",
        );
        let trigger = field(panel, "Type (Event, Data or custom type)", "Event");
        let event = field(
            panel,
            "Source (event name, binding or custom properties JSON)",
            "Click",
        );
        let comparison = field(panel, "Data comparison", "Equal");
        let compare_value = field(panel, "Compare value", "True");
        section(panel, "Action");
        let kind = field(
            panel,
            "Type (ChangeProperty, InvokeCommand, CallMethod, AddClass, RemoveClass or custom type)",
            "ChangeProperty",
        );
        let target = field(panel, "Target (Name or binding)", "");
        let property = field(
            panel,
            "Member (property, method, class or command)",
            "Opacity",
        );
        let value = field(
            panel,
            "Value (value, command parameter or custom properties JSON)",
            "0.5",
        );
        action(panel, "Attach behavior", self, move |this| {
            let trigger = match text(&trigger).as_str() {
                "Event" => BehaviorTrigger::Event {
                    event: text(&event),
                    source: None,
                },
                "Data" => BehaviorTrigger::Data {
                    binding: text(&event),
                    comparison: text(&comparison),
                    value: text(&compare_value),
                    revert_on_false: false,
                },
                name => BehaviorTrigger::Custom {
                    type_name: name.into(),
                    properties: serde_json::from_str(&text(&event)).map_err(|e| e.to_string())?,
                },
            };
            let target = optional(&target);
            let action = match text(&kind).as_str() {
                "ChangeProperty" => BehaviorAction::ChangeProperty {
                    target,
                    property: text(&property),
                    value: text(&value),
                },
                "InvokeCommand" => BehaviorAction::InvokeCommand {
                    command: text(&property),
                    parameter: optional(&value),
                },
                "CallMethod" => BehaviorAction::CallMethod {
                    target,
                    method: text(&property),
                },
                "AddClass" => BehaviorAction::AddClass {
                    target,
                    class: text(&property),
                },
                "RemoveClass" => BehaviorAction::RemoveClass {
                    target,
                    class: text(&property),
                },
                name => BehaviorAction::Custom {
                    type_name: name.into(),
                    properties: serde_json::from_str(&text(&value)).map_err(|e| e.to_string())?,
                    children: String::new(),
                },
            };
            this.session
                .borrow_mut()
                .upsert_behavior(BehaviorModel {
                    id: None,
                    owner,
                    trigger,
                    actions: vec![action],
                })
                .map(|_| ())
        });
        let behaviors = self.session.borrow().behaviors(owner);
        if !behaviors.is_empty() {
            section(panel, "Attached behaviors");
        }
        for model in behaviors {
            let Some(id) = model.id else {
                continue;
            };
            let trigger = match &model.trigger {
                BehaviorTrigger::Event { event, .. } => format!("On {event}"),
                BehaviorTrigger::Data { binding, .. } => format!("When {binding}"),
                BehaviorTrigger::Custom { type_name, .. } => type_name.clone(),
            };
            let title = format!(
                "{trigger} · {} action{}",
                model.actions.len(),
                if model.actions.len() == 1 { "" } else { "s" }
            );
            let label = TextBlock::new();
            label.set_text(Some(&title));
            label.set_font_size(11.0);
            label.set_font_weight(FontWeight::SemiBold);
            label.set_text_wrapping(TextWrapping::Wrap);
            panel.children().add(label.upcast::<Control>());
            action(panel, "Preview actions", self, move |this| {
                this.session.borrow_mut().preview_behavior(id)
            });
            action(panel, "Remove behavior", self, move |this| {
                this.session.borrow_mut().remove_behavior(id)
            });
            let source = model.to_xaml().unwrap_or_default();
            let editor = code_field(panel, "Behavior XAML", &source);
            action(panel, "Apply behavior XAML", self, move |this| {
                this.session
                    .borrow_mut()
                    .set_behavior_source(id, &text(&editor))
            });
            action(panel, "Navigate to behavior source", self, move |this| {
                this.reveal_source(id);
                Ok(())
            });
        }
    }
}
