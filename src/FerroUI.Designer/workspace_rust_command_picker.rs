use super::{Context, DesignerWorkspace, APPLY};
use crate::design_time::DesignTimeSnapshot;
use crate::workspace::inspector_actions::{add_button, text, RESET, SOURCE};
use ferroui_base::{layout::Orientation, media::FontFamily, Ref};
use ferroui_controls::{
    automation::AutomationProperties, Control, Flyout, StackPanel, TextBlock, TextBox, ToolTip,
};
use ferroui_designer_core::NodeId;
use ferroui_designer_rsx::{RsxCommandEdit, RsxCommandSelection, RsxCommandSlot, RsxCommandValue};
use std::rc::Rc;

#[derive(Clone)]
struct RustSite {
    owner: NodeId,
    snapshot: DesignTimeSnapshot,
}

impl RustSite {
    fn validate(
        &self,
        workspace: &DesignerWorkspace,
        session: &crate::DesignerSession,
    ) -> Result<(), String> {
        if workspace.pending_source.get() || !self.snapshot.is_current(session) {
            return Err("The Rust source or selection changed. Reopen Command Binding.".into());
        }
        Ok(())
    }
}

pub(super) fn render(
    workspace: &Rc<DesignerWorkspace>,
    flyout: &Ref<Flyout>,
    body: &StackPanel,
    status: &Ref<TextBlock>,
    owner: NodeId,
) -> Result<(), String> {
    let ((site, slots), _) = workspace.with_current_rust_sources(|session, projection| {
        Ok((
            RustSite {
                owner,
                snapshot: DesignTimeSnapshot::capture(session),
            },
            session.rust_command_slots(projection, owner),
        ))
    })?;
    if slots.is_empty() {
        return Err("This control has no mapped Rust command property.".into());
    }
    let context = Context::new(workspace, flyout, status, site);
    let session = workspace.session.borrow();
    for slot in slots {
        add_slot(body, slot, session.authored_source(), context.clone());
    }
    Ok(())
}

fn add_slot(body: &StackPanel, slot: RsxCommandSlot, source: &str, context: Context<RustSite>) {
    let property = slot.binding.property.clone();
    let command = add_field(body, slot.binding, source, &context);
    let parameter = slot
        .parameter
        .and_then(|value| add_field(body, value, source, &context));
    if command.is_none() && parameter.is_none() {
        return;
    }
    add_button(
        body,
        APPLY,
        &format!("Apply {property} binding"),
        move || {
            let change = RsxCommandEdit {
                property: property.clone(),
                command: Draft::selection(command.as_ref()),
                parameter: Draft::selection(parameter.as_ref()),
            };
            if context
                .run(|workspace| {
                    let (plan, _) =
                        workspace.with_current_rust_sources(|session, projection| {
                            context.site.validate(workspace, session)?;
                            session.rust_command_plan(projection, context.site.owner, &change)
                        })?;
                    workspace.session.borrow_mut().apply_rust_command(plan)?;
                    workspace.refresh(true);
                    Ok(())
                })
                .is_some()
            {
                context.close();
            }
        },
    );
}

struct Draft {
    input: Ref<TextBox>,
    initial: String,
}

impl Draft {
    fn selection(field: Option<&Self>) -> RsxCommandSelection {
        let Some(field) = field else {
            return RsxCommandSelection::Keep;
        };
        let expression = field.input.text().unwrap_or_default();
        if expression == field.initial {
            RsxCommandSelection::Keep
        } else if expression.trim().is_empty() {
            RsxCommandSelection::Clear
        } else {
            RsxCommandSelection::Set { expression }
        }
    }
}

fn add_field(
    body: &StackPanel,
    value: RsxCommandValue,
    source: &str,
    context: &Context<RustSite>,
) -> Option<Draft> {
    let heading = StackPanel::new();
    heading.set_orientation(Orientation::Horizontal);
    heading.set_spacing(4.0);
    heading
        .children()
        .add(text(&value.property).upcast::<Control>());
    let input = TextBox::new();
    input.classes().add("designer-field");
    input.set_placeholder_text(Some(if value.editable {
        "Rust expression"
    } else {
        "Managed in Rust source"
    }));
    input.set_font_family(FontFamily::from("monospace"));
    input.set_accepts_return(true);
    input.set_max_height(92.0);
    input.set_is_read_only(!value.editable);
    AutomationProperties::set_name(&input, Some(&format!("{} Rust expression", value.property)));
    ToolTip::set_tip(&input, Some(Rc::new(value.type_name)));
    if let Some(span) = value.expression {
        input.set_text(source.get(span.range()));
        let context = context.clone();
        add_button(
            &heading,
            SOURCE,
            &format!("Reveal {} in source", value.property),
            move || {
                if let Some(workspace) = context.run(|workspace| {
                    context
                        .site
                        .validate(workspace, &workspace.session.borrow())?;
                    Ok(workspace.clone())
                }) {
                    context.close();
                    workspace.select_source_span(span);
                }
            },
        );
        if value.editable {
            let input = input.downgrade();
            add_button(
                &heading,
                RESET,
                &format!("Clear {} expression", value.property),
                move || {
                    if let Some(input) = input.upgrade() {
                        input.set_text(Some(""));
                    }
                },
            );
        }
    }
    body.children().add(heading.upcast::<Control>());
    body.children().add(input.clone().upcast::<Control>());
    value.editable.then(|| Draft {
        initial: input.text().unwrap_or_default(),
        input,
    })
}
