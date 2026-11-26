use super::curve::matches_property;
use super::NativeSourceIndex;
use crate::StatePreviewGroup;
use ferroui_base::styling::{Setter, StyleBase};
use ferroui_base::{FerroObject, Ref, StyledElement};
use ferroui_designer_core::NodeId;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// The accepted native styles belonging to one source-backed state group.
#[derive(Default)]
pub struct NativeStateStyles(BTreeMap<NodeId, Option<Ref<StyleBase>>>);

impl NativeStateStyles {
    pub fn discover(
        source: &NativeSourceIndex,
        owner: &StyledElement,
        group: &StatePreviewGroup,
    ) -> Self {
        let authored: BTreeSet<_> = group.style_ids().collect();
        if authored.is_empty() {
            return Self::default();
        }
        let mut result = BTreeMap::new();
        let mut work = owner
            .styles()
            .snapshot()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut seen = BTreeSet::new();
        while let Some(style) = work.pop() {
            let Some(object) = style.as_object() else {
                continue;
            };
            if !seen.insert(object as *const FerroObject as usize) {
                continue;
            }
            let Some(base) = object.to_ref().cast::<StyleBase>() else {
                continue;
            };
            if let Some(id) = source
                .resolve_object(object)
                .filter(|id| authored.contains(id))
            {
                result
                    .entry(id)
                    .and_modify(|slot| *slot = None)
                    .or_insert(Some(base));
            }
            work.extend(style.children().iter().cloned());
        }
        Self(result)
    }

    pub(super) fn setter(
        &self,
        id: Option<NodeId>,
        name: &str,
    ) -> Result<Option<Rc<Setter>>, String> {
        let Some(style) = id.and_then(|id| self.0.get(&id)) else {
            return Ok(None);
        };
        let style = style
            .as_ref()
            .ok_or("The state style has more than one live instance")?;
        let setters = style.setters().snapshot();
        let mut matches = setters.iter().filter_map(|setter| {
            let setter = setter.as_any()?.downcast_ref::<Setter>()?;
            let property = setter
                .property()
                .filter(|property| matches_property(property, name))?;
            Some((property, setter))
        });
        let Some((property, setter)) = matches.next() else {
            return Ok(None);
        };
        if matches.next().is_some() {
            return Err(format!("The state style has multiple setters for {name}"));
        }
        let value = setter
            .value()
            .ok_or_else(|| format!("The state setter has no value for {name}"))?;
        Ok(Some(Setter::with_value(property, value)))
    }
}
