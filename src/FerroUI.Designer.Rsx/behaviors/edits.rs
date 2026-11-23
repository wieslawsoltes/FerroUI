use super::{render, syntax, ActionSource, RsxBehavior};
use crate::blocks::keep_comments;
use crate::source_edits::line_indent;
use crate::RsxProjection;
use ferroui_designer_animation::{BehaviorAction, BehaviorModel, BehaviorTrigger};
use ferroui_designer_core::{SourceSpan, TextPatch};

/// An edit on the active invocation's mapped behavior list.
pub enum RsxBehaviorEdit {
    Attach(BehaviorModel),
    Replace {
        behavior: usize,
        model: BehaviorModel,
    },
    Remove(usize),
    SetTrigger {
        behavior: usize,
        value: BehaviorTrigger,
    },
    AppendAction {
        behavior: usize,
        value: BehaviorAction,
    },
    SetAction {
        behavior: usize,
        action: usize,
        value: BehaviorAction,
    },
    MoveAction {
        behavior: usize,
        action: usize,
        index: usize,
    },
    RemoveAction {
        behavior: usize,
        action: usize,
    },
}

impl RsxProjection<'_> {
    pub(crate) fn rename_behavior_targets(
        &self,
        previous: &str,
        name: Option<&str>,
    ) -> Result<Vec<TextPatch>, String> {
        let update = |target: &mut Option<String>| -> Result<bool, String> {
            if target.as_deref().and_then(syntax::control_name) != Some(previous) {
                return Ok(false);
            }
            let name = name.ok_or("The name is used by an authored behavior")?;
            *target = Some(
                syntax::name_markup(name)
                    .ok_or("The new name cannot be used in a behavior binding")?,
            );
            Ok(true)
        };
        let mut patches = Vec::new();
        for item in self.nodes.iter().flat_map(|node| &node.behaviors.entries) {
            let mut trigger = item.model.trigger.clone();
            if let BehaviorTrigger::Event { source, .. } = &mut trigger {
                if update(source)? {
                    patches.extend(item.patch_trigger(self.source, &trigger)?);
                }
            }
            for (index, action) in item.model.actions.iter().enumerate() {
                let mut next = action.clone();
                let target = match &mut next {
                    BehaviorAction::ChangeProperty { target, .. }
                    | BehaviorAction::CallMethod { target, .. }
                    | BehaviorAction::AddClass { target, .. }
                    | BehaviorAction::RemoveClass { target, .. } => target,
                    _ => continue,
                };
                if update(target)? {
                    patches.extend(item.patch_action(self.source, index, &next)?);
                }
            }
        }
        Ok(patches)
    }

    pub fn edit_behavior(
        &self,
        owner: usize,
        edit: RsxBehaviorEdit,
    ) -> Result<Vec<TextPatch>, String> {
        let node = self
            .nodes
            .get(owner)
            .ok_or("Choose a mapped behavior owner")?;
        let mut expected = self
            .behaviors(owner)
            .iter()
            .map(|item| item.model().clone())
            .collect::<Vec<_>>();
        let mut patches = match edit {
            RsxBehaviorEdit::Attach(mut model) => {
                render::normalize(&mut model)?;
                let patches = node.behaviors.slot.insert(
                    self.source,
                    node.open,
                    "behaviors",
                    |indent, eol| {
                        render::declaration(&model, indent, eol).expect("normalized behavior")
                    },
                )?;
                expected.push(model);
                patches
            }
            RsxBehaviorEdit::Remove(index) => {
                let behavior = self.behavior_at(owner, index)?;
                expected.remove(index);
                remove(self.source, behavior.span(), behavior.comma)?
            }
            edit => {
                let (index, change) = Change::from_edit(edit);
                let behavior = self.behavior_at(owner, index)?;
                behavior.edit(self.source, &mut expected[index], change)?
            }
        };
        patches.retain(|patch| {
            self.source.get(patch.span.range()) != Some(patch.replacement.as_str())
        });
        patches.sort_by_key(|patch| (patch.span.start, patch.span.end));
        let draft = TextPatch::apply_all(self.source, patches.clone())
            .map_err(|error| error.to_string())?;
        let modules = self.draft_module_sources(&draft)?;
        let projected =
            self.catalog
                .project_with_modules(&draft, self.invocation.start, modules.as_ref())?;
        if self.tree != projected.tree
            || !projected
                .behaviors(owner)
                .iter()
                .map(RsxBehavior::model)
                .eq(&expected)
        {
            return Err(
                "The behavior edit does not round-trip in the current Rust invocation".into(),
            );
        }
        Ok(patches)
    }

    fn behavior_at(&self, owner: usize, index: usize) -> Result<&RsxBehavior, String> {
        self.behaviors(owner)
            .get(index)
            .ok_or_else(|| "Choose an authored Rust behavior".into())
    }
}

enum Change {
    Replace(BehaviorModel),
    Trigger(BehaviorTrigger),
    Append(BehaviorAction),
    Set(usize, BehaviorAction),
    Move(usize, usize),
    Remove(usize),
}

impl Change {
    fn from_edit(edit: RsxBehaviorEdit) -> (usize, Self) {
        match edit {
            RsxBehaviorEdit::Replace { behavior, model } => (behavior, Self::Replace(model)),
            RsxBehaviorEdit::SetTrigger { behavior, value } => (behavior, Self::Trigger(value)),
            RsxBehaviorEdit::AppendAction { behavior, value } => (behavior, Self::Append(value)),
            RsxBehaviorEdit::SetAction {
                behavior,
                action,
                value,
            } => (behavior, Self::Set(action, value)),
            RsxBehaviorEdit::MoveAction {
                behavior,
                action,
                index,
            } => (behavior, Self::Move(action, index)),
            RsxBehaviorEdit::RemoveAction { behavior, action } => (behavior, Self::Remove(action)),
            RsxBehaviorEdit::Attach(_) | RsxBehaviorEdit::Remove(_) => {
                unreachable!("whole behavior insertion or removal")
            }
        }
    }
}

impl RsxBehavior {
    fn edit(
        &self,
        source: &str,
        model: &mut BehaviorModel,
        change: Change,
    ) -> Result<Vec<TextPatch>, String> {
        let patches = match change {
            Change::Replace(value) => {
                *model = value;
                render::normalize(model)?;
                self.patch_model(source, model)?
            }
            Change::Trigger(value) => {
                model.trigger = value;
                render::normalize(model)?;
                self.patch_trigger(source, &model.trigger)?
            }
            Change::Append(value) => {
                model.actions.push(value);
                render::normalize(model)?;
                let declaration =
                    render::declaration_action(model.actions.last().expect("appended action"))?;
                self.slot
                    .insert(source, self.array.start, "actions", |_, _| declaration)?
            }
            Change::Set(index, value) => {
                self.action_at(index)?;
                model.actions[index] = value;
                render::normalize(model)?;
                self.patch_action(source, index, &model.actions[index])?
            }
            Change::Move(from, to) => {
                self.action_at(from)?;
                self.action_at(to)?;
                let mut order = (0..self.actions.len()).collect::<Vec<_>>();
                order.remove(from);
                order.insert(to, from);
                model.actions = order
                    .iter()
                    .map(|index| self.model.actions[*index].clone())
                    .collect();
                (from.min(to)..=from.max(to))
                    .map(|destination| {
                        let span = self.actions[destination].call.span;
                        let moved = self.actions[order[destination]].call.span;
                        TextPatch {
                            span,
                            replacement: source[moved.range()].into(),
                        }
                    })
                    .collect()
            }
            Change::Remove(index) => {
                let action = self.action_at(index)?;
                model.actions.remove(index);
                remove(source, action.call.span, action.comma)?
            }
        };
        Ok(patches)
    }

    pub(crate) fn patch_model(
        &self,
        source: &str,
        model: &BehaviorModel,
    ) -> Result<Vec<TextPatch>, String> {
        let mut patches = self.patch_trigger(source, &model.trigger)?;
        if model.actions.len() == self.actions.len() {
            for (index, value) in model.actions.iter().enumerate() {
                patches.extend(self.patch_action(source, index, value)?);
            }
        } else {
            let array = render::action_array(
                &model.actions,
                line_indent(source, self.array.start),
                eol(source),
            )?;
            patches.push(replace(source, self.array, &array)?);
        }
        Ok(patches)
    }

    fn action_at(&self, index: usize) -> Result<&ActionSource, String> {
        self.actions
            .get(index)
            .ok_or_else(|| "Choose an authored Rust behavior action".into())
    }

    fn patch_action(
        &self,
        source: &str,
        index: usize,
        next: &BehaviorAction,
    ) -> Result<Vec<TextPatch>, String> {
        let action = self.action_at(index)?;
        let before = render::action(&self.model.actions[index])?;
        let after = render::action(next)?;
        if before.0 == after.0 {
            return patch_operands(source, &action.call, before.1, after.1);
        }
        Ok(vec![replace(
            source,
            action.call.span,
            &render::declaration_action(next)?,
        )?])
    }

    fn patch_trigger(
        &self,
        source: &str,
        next: &BehaviorTrigger,
    ) -> Result<Vec<TextPatch>, String> {
        let before = render::trigger(&self.model.trigger)?;
        let after = render::trigger(next)?;
        if before.0 == after.0 {
            return patch_operands(source, &self.trigger, before.1, after.1);
        }
        let prefix = SourceSpan::new(self.span().start, self.array.start);
        let head = format!("{}({}, ", syntax::factory(after.0), after.1.join(", "));
        Ok(vec![replace(source, prefix, &head)?])
    }
}

fn patch_operands(
    source: &str,
    call: &syntax::Arguments,
    before: Vec<String>,
    after: Vec<String>,
) -> Result<Vec<TextPatch>, String> {
    call.operands
        .iter()
        .zip(before.into_iter().zip(after))
        .filter(|(_, (before, after))| before != after)
        .map(|(&span, (_, after))| replace(source, span, &after))
        .collect()
}

fn replace(source: &str, span: SourceSpan, replacement: &str) -> Result<TextPatch, String> {
    let leading = keep_comments(&source[span.range()])?;
    Ok(TextPatch {
        span,
        replacement: format!("{leading}{replacement}"),
    })
}

fn remove(
    source: &str,
    span: SourceSpan,
    comma: Option<SourceSpan>,
) -> Result<Vec<TextPatch>, String> {
    let mut patches = vec![replace(source, span, "")?];
    if let Some(span) = comma {
        patches.push(TextPatch {
            span,
            replacement: String::new(),
        });
    }
    Ok(patches)
}

fn eol(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}
