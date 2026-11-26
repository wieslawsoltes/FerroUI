use super::{CommandSite, Context, DesignerWorkspace, APPLY};
use crate::workspace::inspector_actions::{add_button, picker, text, SOURCE};
use ferroui_base::layout::Orientation;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::Ref;
use ferroui_controls::{ComboBox, Control, Flyout, StackPanel};
use ferroui_designer_language::{
    CommandBindingEdit, CommandMember, CommandSelection, CommandSlot, CommandValue,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(super) fn render(workspace: &Rc<DesignerWorkspace>, flyout: &Ref<Flyout>) {
    let body = StackPanel::new();
    body.set_width(324.0);
    body.set_spacing(6.0);
    flyout.set_content(Some(Control::boxed(body.clone())));
    let heading = text("Command binding");
    heading.set_font_weight(FontWeight::SemiBold);
    body.children().add(heading.upcast::<Control>());
    let status = text("");
    status.set_text_wrapping(TextWrapping::Wrap);
    status.set_is_visible(false);
    body.children().add(status.clone().upcast::<Control>());
    let owner = workspace.session.borrow().selection().first().copied();
    let Some(owner) = owner else { return };
    if workspace.session.borrow().uses_rust_source() {
        if let Err(error) = super::rust_picker::render(workspace, flyout, &body, &status, owner) {
            status.set_text(Some(&error));
            status.set_is_visible(true);
        }
        return;
    }
    let site = match workspace.command_site(owner) {
        Ok(site) => site,
        Err(error) => {
            status.set_text(Some(&error));
            status.set_is_visible(true);
            return;
        }
    };
    let context = Context::new(workspace, flyout, &status, site);
    let Some(model) =
        context.run(|workspace| workspace.command_model(&context.site, String::new()))
    else {
        return;
    };
    if model.slots.is_empty() {
        status.set_text(Some("This control has no registered command property."));
        status.set_is_visible(true);
    }
    for slot in model.slots {
        add_slot(&body, context.clone(), slot, &model.members);
    }
}

fn add_slot(
    body: &StackPanel,
    context: Context<CommandSite>,
    slot: CommandSlot,
    members: &[CommandMember],
) {
    let command = PathPicker::add(
        body,
        &slot.property,
        slot.binding,
        members,
        true,
        context.clone(),
    );
    let parameter = slot.parameter.map(|value| {
        PathPicker::add(
            body,
            &format!("{}Parameter", slot.property),
            value,
            members,
            false,
            context.clone(),
        )
    });
    add_button(
        body,
        APPLY,
        &format!("Apply {} binding", slot.property),
        move || {
            let change = CommandBindingEdit {
                property: slot.property.clone(),
                command: command.selected(),
                parameter: parameter
                    .as_ref()
                    .map_or(CommandSelection::Keep, |picker| picker.selected()),
            };
            if context
                .run(|workspace| workspace.apply_command_binding(&context.site, &change))
                .is_some()
            {
                context.close();
            }
        },
    );
}

#[derive(Clone)]
enum Choice {
    Value(CommandSelection),
    Browse(String),
}

struct PathPicker {
    combo: Ref<ComboBox>,
    choices: RefCell<Vec<Choice>>,
    value: CommandValue,
    command_only: bool,
    updating: Cell<bool>,
}

impl PathPicker {
    fn add(
        body: &StackPanel,
        property: &str,
        value: CommandValue,
        members: &[CommandMember],
        command_only: bool,
        context: Context<CommandSite>,
    ) -> Rc<Self> {
        let heading = StackPanel::new();
        heading.set_orientation(Orientation::Horizontal);
        heading.set_spacing(4.0);
        heading.children().add(text(property).upcast::<Control>());
        if let Some(range) = value.range {
            let context = context.clone();
            add_button(
                &heading,
                SOURCE,
                &format!("Reveal {property} in source"),
                move || {
                    if context
                        .run(|workspace| workspace.reveal_command_source(&context.site, range))
                        .is_some()
                    {
                        context.close();
                    }
                },
            );
        }
        body.children().add(heading.upcast::<Control>());
        let combo = picker(property, std::iter::empty(), 0);
        combo.set_name(Some(format!("CommandBinding{property}")));
        body.children().add(combo.clone().upcast::<Control>());
        let this = Rc::new(Self {
            combo,
            choices: RefCell::default(),
            value,
            command_only,
            updating: Cell::new(false),
        });
        this.populate("", members);
        let weak = Rc::downgrade(&this);
        this.combo.selection_changed(move |_, _| {
            let Some(this) = weak.upgrade().filter(|this| !this.updating.get()) else {
                return;
            };
            if let Some(Choice::Browse(prefix)) = this.choice() {
                if let Some(model) =
                    context.run(|workspace| workspace.command_model(&context.site, prefix.clone()))
                {
                    this.populate(&prefix, &model.members);
                }
            }
        });
        this
    }

    fn populate(&self, prefix: &str, members: &[CommandMember]) {
        let mut items = vec![(
            self.value.authored.as_deref().map_or_else(
                || "Keep current value".into(),
                |value| format!("Keep {value}"),
            ),
            Choice::Value(CommandSelection::Keep),
        )];
        if !prefix.is_empty() {
            let parent = prefix.rsplit_once('.').map_or("", |(parent, _)| parent);
            items.push(("Back to parent".into(), Choice::Browse(parent.into())));
        }
        if self.value.editable && self.value.authored.is_some() && prefix.is_empty() {
            items.push((
                "Clear binding".into(),
                Choice::Value(CommandSelection::Clear),
            ));
        }
        if self.value.editable {
            for member in members {
                if !self.command_only || member.command {
                    items.push((
                        format!("{} · {}", member.path, member.type_name),
                        Choice::Value(CommandSelection::Bind {
                            path: member.path.clone(),
                        }),
                    ));
                }
                if member.children {
                    items.push((
                        format!("Browse {}…", member.path),
                        Choice::Browse(member.path.clone()),
                    ));
                }
            }
        }
        self.updating.set(true);
        self.combo.items().clear();
        for (label, _) in &items {
            self.combo.items().add(Some(Rc::new(label.clone())));
        }
        self.choices
            .replace(items.into_iter().map(|(_, choice)| choice).collect());
        self.combo.set_selected_index(0);
        self.combo.set_is_enabled(self.choices.borrow().len() > 1);
        self.updating.set(false);
    }

    fn choice(&self) -> Option<Choice> {
        self.choices
            .borrow()
            .get(usize::try_from(self.combo.selected_index()).ok()?)
            .cloned()
    }

    fn selected(&self) -> CommandSelection {
        match self.choice() {
            Some(Choice::Value(value)) => value,
            _ => CommandSelection::Keep,
        }
    }
}
