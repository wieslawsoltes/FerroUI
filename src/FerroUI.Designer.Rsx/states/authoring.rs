//! Checked structural edits to one owner in an accepted RSX projection.

mod builder;

use super::{same_route, same_setter, RsxStateGroup};
use crate::{
    blocks::keep_comments, source_array::ArraySlot, source_edits::line_prefix, tree::Element,
    RsxProjection,
};
use ferroui_designer_animation::VisualStateGroup;
use ferroui_designer_core::{SourceSpan, TextPatch};

impl RsxProjection<'_> {
    pub fn can_create_state_group(&self, owner: usize) -> bool {
        self.nodes
            .get(owner)
            .is_some_and(|node| !matches!(node.state_slot, ArraySlot::Unavailable))
    }

    /// Update the named group or append a new group to this control's inline array.
    pub fn upsert_state_group(
        &self,
        owner: usize,
        model: &VisualStateGroup,
    ) -> Result<Vec<TextPatch>, String> {
        model.validate().map_err(|error| error.to_string())?;
        let node = self
            .nodes
            .get(owner)
            .ok_or("Choose a mapped Rust control")?;
        let mut ordinal = owner;
        let element =
            get_element(&self.tree, &mut ordinal).ok_or("Choose a mapped Rust control")?;
        let builder = builder::Builder::new(self.catalog, element);
        let patches = if let Some(group) = self.state_group(owner, &model.name) {
            group.plan_structure(self.source, model, &builder)?
        } else {
            let group = builder.group(model)?;
            node.state_slot
                .insert(self.source, node.open, "visual_states", |indent, eol| {
                    group.join(&format!("{eol}{indent}    "))
                })?
        };
        self.check_state_edit(owner, &model.name, Some(model), patches)
    }

    pub fn remove_state_group(&self, owner: usize, name: &str) -> Result<Vec<TextPatch>, String> {
        let group = self
            .state_group(owner, name)
            .ok_or("Choose an inline Rust state group")?;
        let mut declaration = remove(self.source, group.declaration)?;
        if declaration.replacement.is_empty()
            && declaration.span.start == group.declaration.start
            && self.source[..declaration.span.start].ends_with(' ')
        {
            declaration.span.start -= 1;
        }
        let mut patches = vec![declaration];
        if let Some(comma) = group.comma {
            patches.push(TextPatch {
                span: comma,
                replacement: String::new(),
            });
        }
        self.check_state_edit(owner, name, None, patches)
    }

    fn state_group(&self, owner: usize, name: &str) -> Option<&RsxStateGroup> {
        self.states
            .get(&owner)?
            .groups
            .iter()
            .find(|group| group.name() == name)
    }

    fn check_state_edit(
        &self,
        owner: usize,
        name: &str,
        expected: Option<&VisualStateGroup>,
        mut patches: Vec<TextPatch>,
    ) -> Result<Vec<TextPatch>, String> {
        patches.retain(|patch| {
            self.source.get(patch.span.range()) != Some(patch.replacement.as_str())
        });
        patches.sort_by_key(|patch| (patch.span.start, patch.span.end));
        let draft =
            TextPatch::apply_all(self.source, patches.clone()).map_err(|err| err.to_string())?;
        let projected = self.catalog.project(&draft, self.invocation.start)?;
        let error =
            || "The state group change does not round-trip in the active Rust view".to_owned();
        if projected.tree != self.tree {
            return Err(error());
        }
        match (projected.state_group(owner, name), expected) {
            (Some(group), Some(model)) => {
                // Checking a second value-only plan accounts for native literal normalization.
                let difference = group.patch_model(model)?;
                if TextPatch::apply_all(&draft, difference).map_err(|err| err.to_string())? != draft
                {
                    return Err(error());
                }
            }
            (None, None) => {}
            _ => return Err(error()),
        }
        Ok(patches)
    }
}

impl RsxStateGroup {
    fn plan_structure(
        &self,
        source: &str,
        model: &VisualStateGroup,
        builder: &builder::Builder<'_>,
    ) -> Result<Vec<TextPatch>, String> {
        let mut patches = Vec::new();
        self.patch_values(model, &mut patches)?;
        for (state, mapping) in self.model.states.iter().zip(&self.states) {
            let Some(next) = model.states.iter().find(|next| next.name == state.name) else {
                patches.push(remove(source, mapping.suffix)?);
                continue;
            };
            for (setter, mapping) in state.setters.iter().zip(&mapping.setters) {
                if !next.setters.iter().any(|next| same_setter(setter, next)) {
                    patches.push(remove(source, mapping.suffix)?);
                }
            }
            let methods = next
                .setters
                .iter()
                .filter(|next| !state.setters.iter().any(|setter| same_setter(setter, next)))
                .map(|setter| builder.setter(setter))
                .collect::<Result<Vec<_>, _>>()?;
            append(
                source,
                mapping.body.end,
                mapping.setters.first().map(|setter| setter.suffix.start),
                methods,
                &mut patches,
            );
        }
        for (route, mapping) in self.model.transitions.iter().zip(&self.routes) {
            if !model.transitions.iter().any(|next| same_route(route, next)) {
                patches.push(remove(source, mapping.suffix)?);
            }
        }

        let methods = model
            .states
            .iter()
            .filter(|next| {
                !self
                    .model
                    .states
                    .iter()
                    .any(|state| state.name == next.name)
            })
            .map(|state| builder.state(state))
            .chain(
                model
                    .transitions
                    .iter()
                    .filter(|next| {
                        !self
                            .model
                            .transitions
                            .iter()
                            .any(|route| same_route(route, next))
                    })
                    .map(builder::transition),
            )
            .collect::<Result<Vec<_>, _>>()?;
        let first = self
            .states
            .iter()
            .map(|state| state.suffix.start)
            .chain(self.routes.iter().map(|route| route.suffix.start))
            .min();
        append(source, self.declaration.end, first, methods, &mut patches);
        Ok(patches)
    }
}

fn remove(source: &str, mut span: SourceSpan) -> Result<TextPatch, String> {
    let replacement = keep_comments(&source[span.range()])?;
    if replacement.is_empty() {
        span.start -= line_prefix(source, Some(span.start)).len();
    }
    Ok(TextPatch { span, replacement })
}

fn append(
    source: &str,
    offset: usize,
    first: Option<usize>,
    methods: Vec<String>,
    patches: &mut Vec<TextPatch>,
) {
    if methods.is_empty() {
        return;
    }
    let prefix = line_prefix(source, first);
    patches.push(TextPatch {
        span: SourceSpan::new(offset, offset),
        replacement: format!("{prefix}{}", methods.join(prefix)),
    });
}

fn get_element<'a>(root: &'a Element, ordinal: &mut usize) -> Option<&'a Element> {
    if *ordinal == 0 {
        return Some(root);
    }
    *ordinal -= 1;
    root.children
        .iter()
        .find_map(|child| get_element(child, ordinal))
}
