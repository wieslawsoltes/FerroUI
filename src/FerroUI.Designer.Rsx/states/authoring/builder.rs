//! Typed Rust syntax for new visual state declarations.

use super::super::{easing_literal, milliseconds};
use crate::{
    RsxCatalog,
    animation::{BUILTIN_PROPERTIES, TargetType, rust_literal},
    tree::Element,
};
use ferroui_designer_animation::{
    AnimationValue, StateSetter, StateTransition, VisualState, VisualStateGroup,
};
use std::fmt::Write;

pub(super) struct Builder<'a> {
    catalog: &'a RsxCatalog,
    owner: &'a Element,
}

impl<'a> Builder<'a> {
    pub(super) fn new(catalog: &'a RsxCatalog, owner: &'a Element) -> Self {
        Self { catalog, owner }
    }

    pub(super) fn group(&self, model: &VisualStateGroup) -> Result<Vec<String>, String> {
        std::iter::once(Ok(format!(
            "::ferroui_view::visual_state_group({:?})",
            model.name
        )))
        .chain(model.states.iter().map(|state| self.state(state)))
        .chain(model.transitions.iter().map(transition))
        .collect()
    }

    pub(super) fn state(&self, state: &VisualState) -> Result<String, String> {
        let mut source = format!(".state(::ferroui_view::visual_state({:?})", state.name);
        for setter in &state.setters {
            source.push_str(&self.setter(setter)?);
        }
        source.push(')');
        Ok(source)
    }

    pub(super) fn setter(&self, setter: &StateSetter) -> Result<String, String> {
        let mut target = self.owner;
        if !setter.target.is_empty() {
            let mut matches = Vec::new();
            named_children(self.owner, &setter.target, &mut matches);
            let [child] = matches.as_slice() else {
                return Err(format!(
                    "Choose one mapped descendant named {:?}",
                    setter.target
                ));
            };
            target = child;
        }
        let ty = TargetType {
            name: target.class.name().to_owned(),
            namespace: target.class.namespace().map(str::to_owned),
        };
        let (path, shape) = self
            .catalog
            .animation_getter(&setter.property, Some(&ty))
            .or_else(|| {
                BUILTIN_PROPERTIES
                    .iter()
                    .filter(|(_, name, _)| {
                        *name != "CornerRadius" || ty.name == "Border" && ty.namespace.is_none()
                    })
                    .find(|(_, name, _)| *name == setter.property)
                    .map(|(path, _, shape)| (*path, *shape))
            })
            .ok_or_else(|| {
                format!(
                    "{:?} has no mapped animation getter on {}",
                    setter.property, ty.name
                )
            })?;
        let value = AnimationValue::parse(&setter.value, shape.kind())
            .map_err(|error| error.to_string())?;
        let value = rust_literal(&value)?;
        let prefix = if path.starts_with("crate::") {
            ""
        } else {
            "::"
        };
        let mut source = if setter.target.is_empty() {
            ".set(".into()
        } else {
            format!(".set_on({:?}, ", setter.target)
        };
        let _ = write!(source, "{prefix}{path}(), {value})");
        Ok(source)
    }
}

pub(super) fn transition(route: &StateTransition) -> Result<String, String> {
    if route.from.as_deref() == Some(&route.to) {
        return Err("Choose distinct states for a Rust transition".into());
    }
    let from = route
        .from
        .as_ref()
        .map_or("None".into(), |from| format!("Some({from:?})"));
    let to = &route.to;
    let milliseconds = milliseconds(route.duration)?;
    let easing = easing_literal(&route.easing)?;
    Ok(format!(
        ".transition({from}, {to:?}, ::ferroui_view::ms({milliseconds}), {easing})"
    ))
}

fn named_children<'a>(owner: &'a Element, name: &str, matches: &mut Vec<&'a Element>) {
    for child in &owner.children {
        if child
            .properties
            .get("Name")
            .is_some_and(|value| value == name)
        {
            matches.push(child);
        }
        named_children(child, name, matches);
    }
}
