//! Plan a name change in the original invocation and its authored references.
mod literals;
mod references;

use self::literals::rewrite_literal;
use self::references::NameReferences;
use crate::{blocks::blocks, local_rust::LocalRust, RsxProjection};
use ferroui_designer_core::TextPatch;
use ferroui_markup::markup::parsers::selector_grammar::SelectorGrammar;
use ferroui_view_syntax::Node;
use std::collections::BTreeSet;
use syn::LitStr;

impl RsxProjection<'_> {
    /// Update an authored name and its style and state references as one edit.
    pub fn edit_control_names(
        &self,
        indices: &[usize],
        name: Option<&str>,
    ) -> Result<Vec<TextPatch>, String> {
        let indices: BTreeSet<_> = indices.iter().copied().collect();
        if indices.is_empty() || name.is_some() && indices.len() != 1 {
            return Err("Choose one authored RSX control to name".into());
        }
        if let Some(name) = name {
            validate_name(name)?;
        }
        let block = blocks(self.source)?
            .into_iter()
            .find(|block| block.span == self.invocation)
            .ok_or("Open the current RSX invocation")?;
        let nodes =
            ferroui_view_syntax::parse(block.group.stream()).map_err(|error| error.to_string())?;
        let bindings = LocalRust::parse(self.source, self.invocation.start, self.modules);
        let mut patches = Vec::new();
        for index in indices {
            patches.extend(self.plan_control_name(&nodes, index, name, &bindings)?);
        }
        Ok(patches)
    }

    fn plan_control_name(
        &self,
        nodes: &[Node],
        index: usize,
        name: Option<&str>,
        bindings: &LocalRust,
    ) -> Result<Vec<TextPatch>, String> {
        let node = self.nodes.get(index).ok_or("Choose a mapped RSX control")?;
        if node.expressions.contains_key("Name") {
            return Err("Edit the computed control name in Rust source".into());
        }
        let declaration = node.properties.get("Name").copied();
        let previous = declaration
            .map(|span| {
                syn::parse_str::<LitStr>(&self.source[span.range()])
                    .map(|literal| literal.value())
                    .map_err(|_| "Choose a control with a mapped Name literal")
            })
            .transpose()?;
        if previous.as_deref() == name && declaration.is_some() {
            return Ok(Vec::new());
        }
        let mut references = NameReferences::new(
            self.source,
            self.catalog,
            node.span,
            declaration,
            previous.as_deref().filter(|name| !name.is_empty()),
            name,
            bindings,
        );
        references.visit(nodes, 0)?;
        let mut patches = references.into_patches();
        match (declaration, previous, name) {
            (Some(span), Some(previous), Some(name)) if !previous.is_empty() => {
                patches.extend(rewrite_literal(
                    self.source,
                    span,
                    &previous,
                    std::iter::once(0..previous.len()),
                    name,
                )?);
            }
            (None, _, None) => {}
            (_, _, name) => patches.push(self.edit_property(index, "Name", name)?),
        }
        Ok(patches)
    }
}

#[cfg(test)]
#[path = "names_tests.rs"]
mod tests;

fn validate_name(name: &str) -> Result<(), String> {
    let probe = format!("#{name}");
    if name.chars().count() > 128
        || !SelectorGrammar::name_ranges(&probe).is_ok_and(
            |(_, names)| matches!(names.as_slice(), [entry] if entry.range == (1..probe.len())),
        )
    {
        return Err("Enter a control name that FerroUI can use in a style selector".into());
    }
    Ok(())
}
