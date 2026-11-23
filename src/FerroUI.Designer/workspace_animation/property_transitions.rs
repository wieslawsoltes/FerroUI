use super::{description, field, section, text, DesignerWorkspace};
use crate::icons;
use crate::workspace::inspector_actions::{add_button, SOURCE};
use ferroui_base::layout::Orientation;
use ferroui_base::Ref;
use ferroui_controls::{Control, StackPanel, TextBox};
use ferroui_designer_animation::{AuthoredPropertyTransition, Easing, PropertyTransitionTiming};
use ferroui_designer_core::{NodeId, SourceSpan};
use std::rc::Rc;

impl DesignerWorkspace {
    pub(super) fn build_property_transition_panel(
        self: &Rc<Self>,
        panel: &StackPanel,
        owner: NodeId,
    ) {
        let session = self.session.borrow();
        let Ok(model) = session.property_transitions(owner) else {
            return;
        };
        let writable = session.can_manipulate();
        let generation = session.generation();
        drop(session);
        if model.entries.is_empty() {
            if let Some(source) = model.source {
                description(panel, "Open the transition collection in source.");
                reveal(panel, self, source, generation);
            } else {
                description(panel, "No authored transitions on this control.");
            }
        }
        for entry in model.entries {
            section(panel, entry.property.as_deref().unwrap_or(&entry.type_name));
            if entry.property.is_some() {
                description(panel, &entry.type_name);
            }
            let toolbar = StackPanel::new();
            toolbar.set_orientation(Orientation::Horizontal);
            toolbar.set_spacing(4.0);
            reveal(&toolbar, self, entry.source, generation);
            if writable && !entry.editable_timing.is_empty() {
                self.add_timing_fields(panel, &toolbar, owner, entry, generation);
            }
            panel.children().add(toolbar.upcast::<Control>());
        }
    }

    fn add_timing_fields(
        self: &Rc<Self>,
        panel: &StackPanel,
        toolbar: &StackPanel,
        owner: NodeId,
        entry: AuthoredPropertyTransition,
        generation: u64,
    ) {
        let duration = entry
            .duration_seconds
            .map(|seconds| field(panel, "Duration (seconds)", &seconds.to_string()));
        let delay = entry
            .delay_seconds
            .map(|seconds| field(panel, "Start delay (seconds)", &seconds.to_string()));
        let easing = entry.editable_timing.contains(&"Easing").then(|| {
            field(
                panel,
                "Easing",
                entry.easing.as_deref().unwrap_or("LinearEasing"),
            )
        });
        let weak = Rc::downgrade(self);
        add_button(
            toolbar,
            icons::CHECK,
            "Apply transition timing",
            move || {
                let Some(workspace) = weak.upgrade() else {
                    return;
                };
                let result = (|| {
                    let timing = PropertyTransitionTiming {
                        duration: edited_seconds(duration.as_ref(), entry.duration_seconds)?,
                        delay: edited_seconds(delay.as_ref(), entry.delay_seconds)?,
                        easing: easing
                            .as_ref()
                            .map(|field| text(field))
                            .filter(|value| {
                                value.trim() != entry.easing.as_deref().unwrap_or("LinearEasing")
                            })
                            .map(|value| Easing::parse(&value).map_err(|error| error.to_string()))
                            .transpose()?,
                    };
                    let mut session = workspace.session.borrow_mut();
                    if session.generation() != generation {
                        return Err("The source changed. Open the current transition again.".into());
                    }
                    session.edit_property_transition_timing(owner, entry.id, &timing)
                })();
                workspace.refresh(true);
                workspace.report(result);
            },
        );
    }
}

fn edited_seconds(
    field: Option<&Ref<TextBox>>,
    initial: Option<f64>,
) -> Result<Option<f64>, String> {
    let Some(field) = field else { return Ok(None) };
    let seconds = text(field)
        .trim()
        .parse::<f64>()
        .map_err(|_| "Enter a nonnegative number of seconds")?;
    if !seconds.is_finite() || seconds < 0.0 {
        return Err("Enter a nonnegative number of seconds".into());
    }
    Ok((Some(seconds) != initial).then_some(seconds))
}

fn reveal(
    panel: &StackPanel,
    workspace: &Rc<DesignerWorkspace>,
    source: SourceSpan,
    generation: u64,
) {
    let weak = Rc::downgrade(workspace);
    add_button(panel, SOURCE, "Reveal transition in source", move || {
        if let Some(workspace) = weak.upgrade() {
            if workspace.session.borrow().generation() == generation {
                workspace.select_source_span(source);
            }
        }
    });
}
