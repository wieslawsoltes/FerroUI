//! Studio's compact editor for the same Rust style transactions exposed in VS Code.
use super::inspector_actions::{add_button, icon_button, picker, text, RESET, SOURCE};
use super::DesignerWorkspace;
use crate::icons;
use ferroui_base::{layout::Orientation, WeakRef};
use ferroui_controls::{
    Button, Control, Flyout, PlacementMode, ScrollViewer, StackPanel, TextBlock,
};
use ferroui_designer_core::SourceSpan;
use ferroui_designer_rsx::{RsxStyle, RsxStyleEdit, RsxStyleResource};
use std::rc::{Rc, Weak};

#[path = "workspace_rust_style_fields.rs"]
mod fields;
#[path = "workspace_rust_style_resources.rs"]
mod resources;
#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_rust_styles_tests.rs"]
mod tests;

impl DesignerWorkspace {
    pub(super) fn connect_rust_styles(self: &Rc<Self>) {
        let anchor = self.root.get_control::<Button>("EditRustStyles");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Left);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let pane = Pane {
            workspace: Rc::downgrade(self),
            flyout: flyout.downgrade(),
        };
        flyout.opening(move |_| pane.render(0, None)).detach();
        anchor.set_flyout(Some(flyout.upcast()));
    }

    pub(super) fn refresh_rust_styles_command(&self) {
        self.root
            .get_control::<Button>("EditRustStyles")
            .set_is_visible(self.session.borrow().uses_rust_source());
    }
}

#[derive(Clone)]
struct Pane {
    workspace: Weak<DesignerWorkspace>,
    flyout: WeakRef<Flyout>,
}

#[derive(Clone)]
struct Scope {
    pane: Pane,
    generation: u64,
    selection: usize,
    status: WeakRef<TextBlock>,
}

impl Pane {
    fn render(&self, preferred: usize, selector: Option<&str>) {
        let (Some(workspace), Some(flyout)) = (self.workspace.upgrade(), self.flyout.upgrade())
        else {
            return;
        };
        let body = StackPanel::new();
        body.set_width(408.0);
        body.set_spacing(6.0);
        let scroll = ScrollViewer::new();
        scroll.set_max_height(450.0);
        scroll.set_content(Some(Control::boxed(body.clone())));
        flyout.set_content(Some(Control::boxed(scroll)));
        let session = workspace.session.borrow();
        let projection = if workspace.pending_source.get() {
            Err("Wait for the current Rust preview".into())
        } else {
            session.rust_style_projection()
        };
        let projection = match projection {
            Ok(projection) => projection,
            Err(reason) => {
                body.children().add(text(&reason).upcast::<Control>());
                return;
            }
        };
        let styles = projection.style_definitions();
        let selected = selector
            .and_then(|selector| {
                styles
                    .iter()
                    .rposition(|(_, rule)| rule.selector == selector)
            })
            .unwrap_or_else(|| preferred.min(styles.len().saturating_sub(1)));
        let status = text("");
        status.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
        status.set_is_visible(false);
        let scope = Scope {
            pane: self.clone(),
            generation: session.generation(),
            selection: selected,
            status: status.downgrade(),
        };
        let tools = StackPanel::new();
        tools.set_orientation(Orientation::Horizontal);
        tools.set_spacing(4.0);
        let hosts = projection
            .nodes
            .iter()
            .enumerate()
            .filter(|(index, _)| projection.can_create_style(*index))
            .map(|(index, node)| {
                (
                    index,
                    session.authored_source()[node.name.range()].to_owned(),
                )
            })
            .collect::<Vec<_>>();
        fields::new_style_button(&tools, &scope, hosts);
        let issues = projection.style_issues.clone();
        fields::issues_button(&tools, &scope, &issues);
        body.children().add(tools.clone().upcast::<Control>());
        if let Some((_, rule)) = styles.get(selected) {
            let labels: Vec<_> = styles
                .iter()
                .map(|(_, style)| {
                    if style.is_nested() {
                        format!("{} · Nested", style.selector)
                    } else {
                        style.selector.clone()
                    }
                })
                .collect();
            let choices = picker("Rust style", labels.iter().map(String::as_str), selected);
            let pane = self.clone();
            let active = scope.clone();
            let choice = choices.downgrade();
            choices.selection_changed(move |_, _| {
                if let Some(control) = choice.upgrade() {
                    if let Ok(index) = usize::try_from(control.selected_index()) {
                        if index != selected && active.current().is_ok() {
                            pane.render(index, None);
                        }
                    }
                }
            });
            body.children().add(choices.upcast::<Control>());
            let choices = projection.style_resource_choices(rule.declaration.start);
            self.rule(&body, &tools, &scope, rule, &projection.style_fields(), &choices);
        } else {
            body.children()
                .add(text("No inline styles in this RSX view.").upcast::<Control>());
        }
        body.children().add(status.upcast::<Control>());
    }

    fn rule(
        &self,
        body: &StackPanel,
        tools: &StackPanel,
        scope: &Scope,
        rule: &RsxStyle,
        fields: &[&ferroui_designer_rsx::RsxStyleField],
        choices: &[&RsxStyleResource],
    ) {
        let style = rule.declaration.start;
        let span = rule.selector_span;
        let active = scope.clone();
        add_button(tools, SOURCE, "Reveal Rust style selector", move || {
            active.reveal(span)
        });
        fields::new_nested_button(tools, scope, style);
        fields::add_setter_button(tools, scope, rule, fields);
        if rule.can_remove() {
            let active = scope.clone();
            add_button(tools, icons::REMOVE, "Remove Rust style", move || {
                active.apply(RsxStyleEdit::Remove { style }, None)
            });
        }
        let active = scope.clone();
        fields::value_row(body, "Selector", &rule.selector, &[], move |selector| {
            active.apply(
                RsxStyleEdit::Selector {
                    style,
                    selector: selector.clone(),
                },
                Some(&selector),
            );
        });
        for setter in &rule.setters {
            let field = setter.field.as_ref();
            let name = field.map_or("Computed property", |field| field.name.as_str());
            let source = setter.source;
            let member = setter.declaration.start;
            let row = if let Some(key) = setter.resource_key() {
                fields::read_only_row(body, name, key)
            } else if let Some(value) = setter.value() {
                let active = scope.clone();
                let options = field.map_or(&[][..], |field| field.options());
                let row = fields::value_row(body, name, &value, options, move |value| {
                    active.apply(
                        RsxStyleEdit::Setter {
                            style,
                            setter: member,
                            value: Some(value),
                        },
                        None,
                    );
                });
                row
            } else {
                fields::source_row(body, name)
            };
            if setter.value().is_some() {
                let active = scope.clone();
                add_button(&row, RESET, &format!("Remove {name} setter"), move || {
                    active.apply(RsxStyleEdit::Setter { style, setter: member, value: None }, None);
                });
                if field.is_some_and(|field| field.accepts_color_resource()) && !choices.is_empty() {
                    resources::change_setter(&row, scope, style, setter, choices);
                }
            }
            let active = scope.clone();
            add_button(
                &row,
                SOURCE,
                &format!("Reveal {name} in Rust source"),
                move || active.reveal(source),
            );
        }
        resources::section(body, scope, rule, fields, choices);
    }
}

impl Scope {
    fn current(&self) -> Result<Rc<DesignerWorkspace>, String> {
        let workspace = self
            .pane
            .workspace
            .upgrade()
            .ok_or("The designer workspace closed")?;
        if workspace.pending_source.get()
            || workspace.session.borrow().generation() != self.generation
        {
            return Err("The document or selection changed. Reopen Rust styles.".into());
        }
        Ok(workspace)
    }

    fn apply(&self, edit: RsxStyleEdit, preferred: Option<&str>) {
        let result = self.current().and_then(|workspace| {
            workspace.session.borrow_mut().edit_rust_style(&edit)?;
            workspace.refresh(true);
            self.pane.render(self.selection, preferred);
            Ok(())
        });
        self.report(result);
    }

    fn reveal(&self, span: SourceSpan) {
        let result = self.current().map(|workspace| {
            if let Some(flyout) = self.pane.flyout.upgrade() {
                flyout.hide();
            }
            workspace.select_source_span(span);
        });
        self.report(result);
    }

    fn report(&self, result: Result<(), String>) {
        if let (Err(error), Some(status)) = (result, self.status.upgrade()) {
            status.set_text(Some(&error));
            status.set_is_visible(true);
        }
    }
}
