//! Bounded Cartesian comparisons of real preview environments and authored states.
//! Planning observes source only; every descriptor is validated before a host adds any card.
use crate::{
    PreviewFlowDirection, PreviewSourceAnchor, PreviewStageConfiguration, PreviewStageState,
    PreviewStageTarget, PreviewTheme, MAX_PREVIEW_STAGES,
};
use ferroui_designer_animation::VisualStateGroup;
use ferroui_designer_core::Document;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageMatrixSize {
    pub name: String,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageStateAxis {
    pub owner: PreviewSourceAnchor,
    pub group: String,
    pub states: Vec<String>,
}

/// An empty environment axis keeps the base stage's value. Each nonempty axis
/// participates in the Cartesian product. State axes override only their named
/// owner/group; the base stage's other selected state endpoints remain intact.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageMatrix {
    #[serde(default)]
    pub sizes: Vec<PreviewStageMatrixSize>,
    #[serde(default)]
    pub themes: Vec<PreviewTheme>,
    #[serde(default)]
    pub directions: Vec<PreviewFlowDirection>,
    #[serde(default)]
    pub design_time_data: Vec<bool>,
    #[serde(default)]
    pub state_groups: Vec<PreviewStageStateAxis>,
}

impl PreviewStageMatrix {
    /// Validate descriptors and count without allocating the expanded product.
    pub fn count(&self) -> Result<usize, String> {
        if self.sizes.len() > MAX_PREVIEW_STAGES
            || self.themes.len() > 3
            || self.directions.len() > 3
            || self.design_time_data.len() > 2
            || self.state_groups.len() > 32
        {
            return Err("The comparison contains too many values on an axis".into());
        }
        let mut names = BTreeSet::new();
        let mut dimensions = BTreeSet::new();
        for size in &self.sizes {
            label(&size.name, 64, "Size name")?;
            if !names.insert(&size.name)
                || !dimensions.insert((size.width.to_bits(), size.height.to_bits()))
            {
                return Err("Comparison sizes must have distinct names and dimensions".into());
            }
            crate::PreviewEnvironment {
                width: size.width,
                height: size.height,
                ..Default::default()
            }
            .validate()?;
        }
        unique(&self.themes, "themes")?;
        unique(&self.directions, "reading directions")?;
        unique(&self.design_time_data, "design-data modes")?;
        let mut groups = BTreeSet::new();
        for axis in &self.state_groups {
            axis.owner.validate()?;
            label(&axis.group, 256, "State group")?;
            if axis.states.is_empty() || axis.states.len() > MAX_PREVIEW_STAGES {
                return Err("Select between 1 and 16 values for each state axis".into());
            }
            if !groups.insert((
                serde_json::to_string(&axis.owner).map_err(|error| error.to_string())?,
                &axis.group,
            )) {
                return Err(
                    "A state group on one owner may appear only once in a comparison".into(),
                );
            }
            for state in &axis.states {
                label(state, 256, "Visual state")?;
            }
            unique(&axis.states, "visual states")?;
        }
        let lengths = [
            self.sizes.len(),
            self.themes.len(),
            self.directions.len(),
            self.design_time_data.len(),
        ];
        let mut count = 1_usize;
        for length in lengths
            .into_iter()
            .chain(self.state_groups.iter().map(|axis| axis.states.len()))
        {
            count = count
                .checked_mul(length.max(1))
                .ok_or("The comparison is too large")?;
        }
        Ok(count)
    }

    /// Produce deterministic configurations using the current authored source.
    /// Invalid/ambiguous owners, definitions and state names reject the entire plan.
    /// Native rendering failures after installation remain individual card diagnostics.
    pub fn expand(
        &self,
        base: &PreviewStageConfiguration,
        document: &Document,
        available: usize,
    ) -> Result<Vec<PreviewStageConfiguration>, String> {
        base.validate()?;
        let count = self.count()?;
        if count > available.min(MAX_PREVIEW_STAGES) {
            return Err(format!(
                "This comparison creates {count} stages, but only {} of {MAX_PREVIEW_STAGES} slots are available. Reduce the selected variations or remove stages.",
                available.min(MAX_PREVIEW_STAGES)
            ));
        }
        if !document.is_valid() {
            return Err("Repair the comparison source before selecting authored states".into());
        }
        let scope = match &base.target {
            PreviewStageTarget::Document => document.ast().root(),
            PreviewStageTarget::Definition { anchor, options } => {
                crate::definition_projection::scope_for(
                    document,
                    anchor.resolve(document)?,
                    options.clone(),
                    Vec::new(),
                )?
                .node
            }
        };
        let groups = VisualStateGroup::read(document);
        for axis in &self.state_groups {
            let owner = axis.owner.resolve(document)?;
            if !document.ast().subtree(scope).any(|node| node.id == owner) {
                return Err(format!(
                    "State group '{}' is outside this preview's authored scope",
                    axis.group
                ));
            }
            let matches = groups
                .iter()
                .filter(|group| group.owner == owner && group.name == axis.group)
                .collect::<Vec<_>>();
            let [group] = matches.as_slice() else {
                return Err(format!(
                    "State group '{}' is missing or ambiguous",
                    axis.group
                ));
            };
            group.validate().map_err(|error| error.to_string())?;
            for state in &axis.states {
                if !group.states.iter().any(|value| value.name == *state) {
                    return Err(format!("Unknown state '{state}' in group '{}'", axis.group));
                }
            }
        }
        let mut variants = vec![(base.clone(), Vec::<String>::new())];
        multiply(&mut variants, &self.sizes, |config, size| {
            config.environment.width = size.width;
            config.environment.height = size.height;
            size.name.clone()
        });
        multiply(&mut variants, &self.themes, |config, theme| {
            config.environment.theme = *theme;
            match theme {
                PreviewTheme::Light => "Light",
                PreviewTheme::Dark => "Dark",
                PreviewTheme::Inherit => "App theme",
            }
            .into()
        });
        multiply(&mut variants, &self.directions, |config, direction| {
            config.environment.flow_direction = *direction;
            match direction {
                PreviewFlowDirection::Inherit => "Inherited direction",
                PreviewFlowDirection::LeftToRight => "LTR",
                PreviewFlowDirection::RightToLeft => "RTL",
            }
            .into()
        });
        multiply(&mut variants, &self.design_time_data, |config, enabled| {
            config.design_time_data = *enabled;
            if *enabled {
                "Design data"
            } else {
                "Runtime data"
            }
            .into()
        });
        for axis in &self.state_groups {
            multiply(&mut variants, &axis.states, |config, state| {
                config.states.retain(|selected| {
                    selected.owner != axis.owner || selected.group != axis.group
                });
                config.states.push(PreviewStageState {
                    owner: axis.owner.clone(),
                    group: axis.group.clone(),
                    state: state.clone(),
                });
                format!("{}={state}", axis.group)
            });
        }
        let mut names = BTreeSet::new();
        variants
            .into_iter()
            .enumerate()
            .map(|(index, (mut config, labels))| {
                let full = if labels.is_empty() {
                    format!("{} · Copy", base.name)
                } else {
                    format!("{} · {}", base.name, labels.join(" · "))
                };
                config.name = bounded_name(&full, index, false);
                let mut suffix = index;
                while names.contains(&config.name) {
                    config.name = bounded_name(&full, suffix, true);
                    suffix += 1;
                }
                names.insert(config.name.clone());
                config.validate()?;
                for state in &config.states {
                    let owner = state.owner.resolve(document)?;
                    if !document.ast().subtree(scope).any(|node| node.id == owner) {
                        return Err(format!(
                            "State group '{}' is outside this preview's authored scope",
                            state.group
                        ));
                    }
                    let matching = groups
                        .iter()
                        .filter(|group| group.owner == owner && group.name == state.group)
                        .collect::<Vec<_>>();
                    let [group] = matching.as_slice() else {
                        return Err(format!(
                            "State group '{}' is missing or ambiguous",
                            state.group
                        ));
                    };
                    group.validate().map_err(|error| error.to_string())?;
                    if !group.states.iter().any(|value| value.name == state.state) {
                        return Err(format!(
                            "Unknown state '{}' in group '{}'",
                            state.state, state.group
                        ));
                    }
                }
                Ok(config)
            })
            .collect()
    }
}

fn multiply<T>(
    variants: &mut Vec<(PreviewStageConfiguration, Vec<String>)>,
    values: &[T],
    update: impl Fn(&mut PreviewStageConfiguration, &T) -> String,
) {
    if values.is_empty() {
        return;
    }
    *variants = variants
        .iter()
        .flat_map(|(base, labels)| {
            values.iter().map(|value| {
                let mut config = base.clone();
                let mut labels = labels.clone();
                labels.push(update(&mut config, value));
                (config, labels)
            })
        })
        .collect();
}

fn unique<T: PartialEq>(values: &[T], name: &str) -> Result<(), String> {
    if values
        .iter()
        .enumerate()
        .any(|(index, value)| values[..index].contains(value))
    {
        Err(format!("Choose distinct {name} in a comparison"))
    } else {
        Ok(())
    }
}

fn label(value: &str, limit: usize, name: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > limit || value.contains('\0') {
        Err(format!("{name} must be nonempty and at most {limit} bytes"))
    } else {
        Ok(())
    }
}

fn bounded_name(full: &str, index: usize, force_suffix: bool) -> String {
    if full.len() <= 128 && !force_suffix {
        return full.into();
    }
    let suffix = format!(" … {}", index + 1);
    let mut end = (128 - suffix.len()).min(full.len());
    while !full.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{suffix}", full[..end].trim_end())
}

#[cfg(test)]
#[path = "preview_stage_matrix_tests.rs"]
mod tests;
