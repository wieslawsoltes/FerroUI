//! Entry points for scoped style and palette authoring in Studio's definition inspector.
use super::super::inspector_actions::{text, tool_button, SOURCE};
use super::{ComponentContext, DesignerWorkspace, ResourceDefinitionEditor};
use crate::style_composition::{compatible_base, supports_palette, supports_styles};
use ferroui_base::layout::Orientation;
use ferroui_base::Ref;
use ferroui_controls::{Button, Control, Flyout, PlacementMode, StackPanel, TextBox};
use ferroui_designer_core::NodeId;
use ferroui_designer_tools::component_styles::{base_theme, StyleChange};
use ferroui_designer_tools::{component_property_elements, ComponentEntry, ComponentKind};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

#[path = "workspace_style_sources.rs"]
mod sources;
pub(super) use sources::StyleSourceVisit;
#[path = "workspace_palette_view.rs"]
mod palette;
#[cfg(test)]
#[path = "workspace_style_composition_tests.rs"]
mod tests;
#[cfg(all(test, feature = "mcp", feature = "render-smoke"))]
#[path = "workspace_palette_tests.rs"]
mod palette_tests;

const PALETTE: &str = "F1 M8,1 A7,7 0 1 0 8,15 H9.5 A1.5,1.5 0 0 0 10.5,12.4 A1.2,1.2 0 0 1 11.4,10.3 H13 A2,2 0 0 0 15,8.3 A7,7 0 0 0 8,1 Z M4,6 A1,1 0 1 1 4,8 A1,1 0 1 1 4,6 Z M7,3 A1,1 0 1 1 7,5 A1,1 0 1 1 7,3 Z M11,4 A1,1 0 1 1 11,6 A1,1 0 1 1 11,4 Z";
const ADD: &str = "M7,2 H9 V7 H14 V9 H9 V14 H7 V9 H2 V7 H7 Z";
const BACK: &str = "M7,3 L2,8 L7,13 L8,12 L4.75,8.75 H14 V7.25 H4.75 L8,4 Z";

#[derive(Clone)]
struct Context {
    workspace: Weak<DesignerWorkspace>,
    editor: Weak<ResourceDefinitionEditor>,
    guard: Rc<RefCell<ComponentContext>>,
    writable: bool,
}

impl Context {
    fn workspace(&self) -> Result<Rc<DesignerWorkspace>, String> {
        let workspace = self
            .workspace
            .upgrade()
            .ok_or("The designer workspace was closed")?;
        self.guard.borrow().check(&workspace)?;
        Ok(workspace)
    }

    fn preview(&self, theme: crate::PreviewTheme) -> Result<(), String> {
        let workspace = self.workspace()?;
        let environment = crate::PreviewEnvironment {
            theme,
            ..workspace.preview_environment()
        };
        workspace.set_preview_environment(environment)?;
        let current = ComponentContext::capture(&workspace);
        let mut guard = self.guard.borrow_mut();
        // Theme changes can replace the native scope while retaining the authored
        // document. Rebase only the popup that requested that change.
        if guard.uri != current.uri
            || guard.revision != current.revision
            || guard.scope != current.scope
            || guard.selection != current.selection
            || !Arc::ptr_eq(&guard.ast, &current.ast)
        {
            return Err("The authored scope changed. Reopen the palette.".into());
        }
        *guard = current;
        Ok(())
    }

    fn report(&self, result: Result<(), String>) {
        if let Some(editor) = self.editor.upgrade() {
            editor.show_error(result.as_ref().err().map(String::as_str));
        }
    }

    fn edit(
        &self,
        action: impl FnOnce(&mut crate::DesignerSession) -> Result<(), String>,
    ) -> Result<(), String> {
        let workspace = self.workspace()?;
        action(&mut workspace.session.borrow_mut())?;
        workspace.refresh(true);
        Ok(())
    }
}

impl DesignerWorkspace {
    pub(super) fn add_style_composition_tools(
        self: &Rc<Self>,
        editor: &Rc<ResourceDefinitionEditor>,
        guard: &ComponentContext,
        definition: Option<&ComponentEntry>,
        writable: bool,
    ) {
        let context = Context {
            workspace: Rc::downgrade(self),
            editor: Rc::downgrade(editor),
            guard: Rc::new(RefCell::new(guard.clone())),
            writable,
        };
        let candidates = self.component_owners();
        let session = self.session.borrow();
        if session.ensure_style_document().is_err() {
            return;
        }
        let document = session.document();
        let visit = self
            .components
            .borrow()
            .style_source_visit
            .clone()
            .filter(|visit| visit.uri == session.document_uri());
        let requested_palette = visit.as_ref().and_then(|visit| visit.palette());
        let owner = definition
            .map(|entry| entry.node)
            .or_else(|| requested_palette.map(|(owner, _)| owner))
            .filter(|id| {
                document
                    .ast()
                    .node(*id)
                    .is_some_and(|node| supports_palette(document, node))
            })
            .or_else(|| {
                candidates.iter().map(|(id, _)| *id).find(|id| {
                    document
                        .ast()
                        .node(*id)
                        .is_some_and(|node| supports_palette(document, node))
                })
            });
        let root = StackPanel::new();
        root.set_spacing(5.0);
        let actions = StackPanel::new();
        actions.set_orientation(Orientation::Horizontal);
        actions.set_spacing(4.0);
        if let Some(visit) = visit {
            let back = tool_button(BACK, "Return to consuming view");
            back.set_name(Some("DefinitionStyleBack".into()));
            let context = context.clone();
            back.click(move |_, _| {
                let result = context
                    .workspace
                    .upgrade()
                    .ok_or_else(|| "The designer workspace was closed".into())
                    .and_then(|workspace| workspace.return_from_style_source(&visit));
                context.report(result);
            });
            actions.children().add(back.upcast::<Control>());
        }
        if session
            .accepted_preview_includes()
            .is_some_and(|sources| sources.documents().next().is_some())
        {
            actions.children().add(
                composition_button(
                    SOURCE,
                    "Open included styles and palettes",
                    "DefinitionStyleSources",
                    context.clone(),
                    sources::picker,
                )
                .upcast::<Control>(),
            );
        }
        if let Some(owner) = owner {
            let layer = requested_palette
                .filter(|(id, _)| *id == owner)
                .map(|(_, layer)| layer);
            actions.children().add(
                composition_button(
                    PALETTE,
                    "Edit scoped palette",
                    "DefinitionPalette",
                    context.clone(),
                    move |context| palette::body(context, owner, layer),
                )
                .upcast::<Control>(),
            );
        }
        let style_owner = definition
            .map(|entry| entry.node)
            .filter(|id| {
                document
                    .ast()
                    .node(*id)
                    .is_some_and(|node| supports_styles(document, node))
            })
            .or_else(|| {
                candidates.iter().map(|(id, _)| *id).find(|id| {
                    document
                        .ast()
                        .node(*id)
                        .is_some_and(|node| supports_styles(document, node))
                })
            });
        if let Some(owner) = style_owner.filter(|_| writable) {
            actions.children().add(
                composition_button(
                    ADD,
                    "New style in this scope",
                    "DefinitionNewStyle",
                    context.clone(),
                    move |context| create_style(context, owner),
                )
                .upcast::<Control>(),
            );
        }
        root.children().add(actions.upcast::<Control>());
        if let Some(entry) = definition.filter(|entry| entry.kind == ComponentKind::ControlTheme) {
            let theme = entry.node;
            let base = base_theme(document, theme).ok().flatten();
            let choices = session
                .component_catalog()
                .unwrap_or_default()
                .entries
                .iter()
                .filter(|entry| compatible_base(document, theme, entry))
                .map(|entry| (entry.node, entry.display_name.clone()))
                .collect::<Vec<_>>();
            let protected = component_property_elements(document, theme, "BasedOn")
                .is_ok_and(|elements| !elements.is_empty());
            let current = crate::style_composition::authored_base_value(document, theme);
            root.children().add(base_editor(
                context, theme, base, current, choices, protected,
            ));
        }
        editor.add_composition_tools(root.upcast());
    }
}

fn base_editor(
    context: Context,
    theme: NodeId,
    base: Option<NodeId>,
    current: Option<String>,
    choices: Vec<(NodeId, String)>,
    protected: bool,
) -> Ref<Control> {
    let body = StackPanel::new();
    body.set_orientation(Orientation::Horizontal);
    body.set_spacing(5.0);
    let mut names = vec!["None".to_owned()];
    let selected = base
        .and_then(|node| choices.iter().position(|(id, _)| *id == node))
        .map(|index| index + 1);
    names.extend(choices.iter().map(|(_, label)| label.clone()));
    let index = selected.unwrap_or_else(|| {
        if let Some(value) = current {
            names.push(value);
            names.len() - 1
        } else {
            0
        }
    });
    let select = palette::choice(
        "DefinitionBaseTheme",
        "Based on control theme",
        names,
        index,
    );
    select.set_is_enabled(context.writable && !protected);
    let context_for_choice = context.clone();
    let choice = select.downgrade();
    select.selection_changed(move |_, _| {
        let Some(select) = choice.upgrade() else {
            return;
        };
        let index = select.selected_index();
        let base = if index == 0 {
            None
        } else if let Some((node, _)) = choices.get(index.saturating_sub(1) as usize) {
            Some(*node)
        } else {
            return;
        };
        context_for_choice.report(
            context_for_choice
                .edit(|session| session.change_style(StyleChange::SetBase { theme, base })),
        );
    });
    body.children().add(text("Based on").upcast::<Control>());
    body.children().add(select.upcast::<Control>());
    body.upcast()
}

fn composition_button(
    glyph: &str,
    label: &str,
    name: &str,
    context: Context,
    content: impl Fn(Context) -> Result<Ref<Control>, String> + 'static,
) -> Ref<Button> {
    let button = tool_button(glyph, label);
    button.set_name(Some(name.into()));
    let flyout = Flyout::new();
    flyout.set_placement(PlacementMode::Left);
    flyout.popup().set_should_use_overlay_layer(true);
    flyout.flyout_presenter_classes().add("designer-chrome");
    let weak = flyout.downgrade();
    flyout
        .opening(move |_| {
            if let Some(flyout) = weak.upgrade() {
                let value = content(context.clone()).unwrap_or_else(|error| text(&error).upcast());
                flyout.set_content(Some(Control::boxed(value)));
            }
        })
        .detach();
    button.set_flyout(Some(flyout.upcast()));
    button
}

fn create_style(context: Context, owner: NodeId) -> Result<Ref<Control>, String> {
    let workspace = context.workspace()?;
    let session = workspace.session.borrow();
    let node = session
        .document()
        .ast()
        .node(owner)
        .ok_or("The style scope changed")?;
    let selector = if matches!(node.local_name(), "Style" | "ControlTheme") {
        "^:pointerover".into()
    } else {
        node.name.replace(':', "|")
    };
    drop(session);
    let body = StackPanel::new();
    body.set_min_width(232.0);
    body.set_spacing(6.0);
    body.children().add(text("New style").upcast::<Control>());
    let field = TextBox::new();
    field.set_text(Some(&selector));
    field.set_name(Some("NewScopedStyleSelector".into()));
    ferroui_controls::automation::AutomationProperties::set_name(&field, Some("Style selector"));
    let button = tool_button(ADD, "Create style");
    button.set_name(Some("CreateScopedStyle".into()));
    let commit = field.clone();
    button.click(move |_, _| {
        let selector = commit.text().unwrap_or_default();
        context.report(context.edit(|session| {
            session.change_style(StyleChange::Create {
                owner,
                selector: selector.trim(),
            })
        }));
    });
    body.children().add(field.upcast::<Control>());
    body.children().add(button.upcast::<Control>());
    Ok(body.upcast())
}
