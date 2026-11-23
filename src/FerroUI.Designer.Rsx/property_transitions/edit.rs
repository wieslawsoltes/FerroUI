use super::syntax::{property_target, DurationOperand, EasingOperand};
use super::RsxPropertyTransition;
use crate::blocks::keep_comments;
use crate::literal::string_literal;
use crate::source_edits::insert_field;
use crate::RsxProjection;
use ferroui_designer_animation::{native_key_time, Easing, PropertyTransitionTiming};
use ferroui_designer_core::{SourceSpan, TextPatch};

pub struct RsxPropertyTransitionChoice<'a> {
    pub getter: &'a str,
    pub property: String,
    pub type_name: &'static str,
}

pub enum RsxPropertyTransitionEdit<'a> {
    Add {
        getter: &'a str,
        timing: &'a PropertyTransitionTiming,
    },
    Retarget {
        source: SourceSpan,
        getter: &'a str,
    },
    Timing {
        source: SourceSpan,
        timing: &'a PropertyTransitionTiming,
    },
    Move {
        source: SourceSpan,
        index: usize,
    },
    Remove {
        source: SourceSpan,
    },
}

impl RsxProjection<'_> {
    pub fn can_edit_property_transitions(&self, owner: usize) -> bool {
        self.nodes.get(owner).is_some_and(|node| {
            node.class.transition_base().is_some()
                && node.class.xaml_property(None, "transitions").is_none()
                && (node.property_transitions.is_some()
                    || !node.expressions.contains_key("Transitions"))
        })
    }

    pub fn property_transition_choices(
        &self,
        owner: usize,
    ) -> Vec<RsxPropertyTransitionChoice<'_>> {
        if !self.can_edit_property_transitions(owner) {
            return Vec::new();
        }
        let class = &self.nodes[owner].class;
        let mut choices = self
            .catalog
            .style_fields()
            .into_iter()
            .filter_map(|field| {
                let (property, target) = property_target(class, field)?;
                Some(RsxPropertyTransitionChoice {
                    getter: &field.getter,
                    property,
                    type_name: target.type_name,
                })
            })
            .collect::<Vec<_>>();
        choices.sort_by(|left, right| {
            left.property
                .cmp(&right.property)
                .then(left.getter.cmp(right.getter))
        });
        choices
    }

    pub fn edit_property_transitions(
        &self,
        owner: usize,
        edit: RsxPropertyTransitionEdit<'_>,
    ) -> Result<Vec<TextPatch>, String> {
        if !self.can_edit_property_transitions(owner) {
            return Err("Choose a mapped inline Rust transition collection or edit its expression in source".into());
        }
        let list = self.property_transitions(owner);
        let entries = list.map_or(&[][..], |list| list.entries.as_slice());
        let entry = |source| {
            entries
                .iter()
                .position(|entry| entry.source == source)
                .ok_or_else(|| {
                    "Choose an entry in the current Rust transition collection".to_owned()
                })
        };
        let getter = |getter| {
            self.property_transition_choices(owner)
                .into_iter()
                .find(|choice| choice.getter == getter)
                .map(|choice| choice.getter)
                .ok_or_else(|| "Choose a compatible registered Rust property getter".to_owned())
        };
        let mut expected = entries
            .iter()
            .map(TransitionValue::from)
            .collect::<Vec<_>>();
        let mut patches = match edit {
            RsxPropertyTransitionEdit::Add {
                getter: property,
                timing,
            } => {
                let spec = TransitionValue {
                    getter: getter(property)?,
                    duration: timing.duration.unwrap_or(0.2),
                    easing: timing.easing.as_ref(),
                };
                validate_timing(timing)?;
                let declaration = spec.declaration()?;
                expected.push(spec);
                let node = &self.nodes[owner];
                if let Some(list) = list {
                    list.array
                        .insert(self.source, node.open, "transitions", |_, _| declaration)?
                } else {
                    let field =
                        format!("transitions: ::ferroui_view::transitions([{declaration}])");
                    vec![insert_field(self.source, node.open, &field)]
                }
            }
            RsxPropertyTransitionEdit::Retarget {
                source,
                getter: property,
            } => {
                let index = entry(source)?;
                let property = getter(property)?;
                if property == expected[index].getter {
                    return Ok(Vec::new());
                }
                expected[index].getter = property;
                vec![operand(
                    self.source,
                    entries[index].operands.getter,
                    &absolute(property),
                )?]
            }
            RsxPropertyTransitionEdit::Timing { source, timing } => {
                let index = entry(source)?;
                validate_timing(timing)?;
                let mut patches = Vec::new();
                if let Some(duration) = timing
                    .duration
                    .filter(|value| *value != entries[index].duration)
                {
                    expected[index].duration = duration;
                    patches.extend(duration_patches(
                        self.source,
                        &entries[index].operands.duration,
                        duration,
                    )?);
                }
                if let Some(easing) = timing
                    .easing
                    .as_ref()
                    .filter(|value| Some(*value) != entries[index].easing.as_ref())
                {
                    expected[index].easing = Some(easing);
                    patches.extend(easing_patches(self.source, &entries[index], easing)?);
                }
                patches
            }
            RsxPropertyTransitionEdit::Move { source, index } => {
                let from = entry(source)?;
                if index >= entries.len() {
                    return Err(
                        "Choose a position in the current Rust transition collection".into(),
                    );
                }
                let moved = expected.remove(from);
                expected.insert(index, moved);
                let mut ordered = entries.iter().collect::<Vec<_>>();
                let moved = ordered.remove(from);
                ordered.insert(index, moved);
                entries
                    .iter()
                    .zip(ordered)
                    .filter(|(previous, next)| previous.source != next.source)
                    .map(|(previous, next)| {
                        patch(previous.source, self.source[next.source.range()].into())
                    })
                    .collect()
            }
            RsxPropertyTransitionEdit::Remove { source } => {
                let index = entry(source)?;
                expected.remove(index);
                let comma = entries[index].comma.or_else(|| {
                    index
                        .checked_sub(1)
                        .and_then(|before| entries[before].comma)
                });
                let mut patches = vec![patch(source, keep_comments(&self.source[source.range()])?)];
                patches.extend(comma.map(|comma| patch(comma, String::new())));
                patches
            }
        };
        patches.retain(|patch| self.source[patch.span.range()] != patch.replacement);
        if patches.is_empty() {
            return Ok(patches);
        }
        let source = TextPatch::apply_all(self.source, patches.clone())
            .map_err(|error| error.to_string())?;
        let modules = self.draft_module_sources(&source)?;
        let next =
            self.catalog
                .project_with_modules(&source, self.invocation.start, modules.as_ref())?;
        let accepted = next
            .property_transitions(owner)
            .ok_or("The Rust transition collection did not round-trip")?;
        if next.tree != self.tree
            || next.omitted_count != self.omitted_count
            || !accepted
                .entries
                .iter()
                .map(TransitionValue::from)
                .eq(expected)
        {
            return Err(
                "The edit changed other source or produced a different Rust transition collection"
                    .into(),
            );
        }
        Ok(patches)
    }
}

#[derive(PartialEq)]
struct TransitionValue<'a> {
    getter: &'a str,
    duration: f64,
    easing: Option<&'a Easing>,
}

impl<'a> From<&'a RsxPropertyTransition> for TransitionValue<'a> {
    fn from(source: &'a RsxPropertyTransition) -> Self {
        Self {
            getter: &source.getter,
            duration: source.duration,
            easing: source.easing.as_ref(),
        }
    }
}

impl TransitionValue<'_> {
    fn declaration(&self) -> Result<String, String> {
        let getter = absolute(self.getter);
        let millis = milliseconds(self.duration)?;
        if let Some(easing) = self.easing {
            Ok(format!(
                "::ferroui_view::transition_eased({getter}(), ::ferroui_view::ms({millis}), {})",
                easing_expression(easing)
            ))
        } else {
            Ok(format!(
                "::ferroui_view::transition({getter}(), ::ferroui_view::ms({millis}))"
            ))
        }
    }
}

fn validate_timing(timing: &PropertyTransitionTiming) -> Result<(), String> {
    if timing.delay.is_some_and(|delay| delay != 0.0) {
        return Err("The mapped Rust transition builders have no start delay argument".into());
    }
    if let Some(easing) = &timing.easing {
        easing.validate().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn milliseconds(duration: f64) -> Result<u64, String> {
    native_key_time(duration).map_err(|error| error.to_string())?;
    let millis = (duration * 1000.0).round();
    (millis / 1000.0 == duration)
        .then_some(millis as u64)
        .ok_or_else(|| "Use a duration in whole milliseconds for Rust transitions".to_owned())
}

fn duration_patches(
    source: &str,
    operand_: &DurationOperand,
    duration: f64,
) -> Result<Vec<TextPatch>, String> {
    let millis = milliseconds(duration)?;
    let mut patches = Vec::new();
    let count = if operand_.units == 1.0 && millis % 1000 == 0 {
        millis / 1000
    } else {
        if operand_.units == 1.0 {
            patches.push(operand(source, operand_.callee, "::ferroui_view::ms")?);
        }
        millis
    };
    let suffix = if operand_.explicit_type { "u64" } else { "" };
    patches.push(patch(operand_.literal, format!("{count}{suffix}")));
    Ok(patches)
}

fn easing_patches(
    source: &str,
    entry: &RsxPropertyTransition,
    easing: &Easing,
) -> Result<Vec<TextPatch>, String> {
    let value = easing.to_xaml();
    let operands = &entry.operands;
    match (&operands.easing, easing) {
        (
            Some(EasingOperand::Parsed { literal, .. }),
            Easing::Named(_) | Easing::Linear | Easing::CubicBezier { .. },
        ) => Ok(vec![patch(
            *literal,
            string_literal(&source[literal.range()], &value),
        )]),
        (
            Some(EasingOperand::NamedConstructor { callee, .. }),
            Easing::Named(_) | Easing::Linear,
        ) => Ok(vec![operand(
            source,
            *callee,
            &format!("::ferroui_base::animation::easings::{value}::new"),
        )?]),
        (
            Some(EasingOperand::Spring { parameters, .. }),
            Easing::Spring {
                mass,
                stiffness,
                damping,
                initial_velocity,
            },
        ) => {
            let mut patches = Vec::new();
            for (parameter, &next) in
                parameters
                    .iter()
                    .zip([mass, stiffness, damping, initial_velocity])
            {
                parameter.plan(next, &mut patches);
            }
            Ok(patches)
        }
        (Some(authored), _) => Ok(vec![operand(
            source,
            authored.source(),
            &easing_expression(easing),
        )?]),
        (None, _) => {
            let end = operands.duration.source.end;
            Ok(vec![
                operand(source, operands.callee, "::ferroui_view::transition_eased")?,
                patch(
                    SourceSpan::new(end, end),
                    format!(", {}", easing_expression(easing)),
                ),
            ])
        }
    }
}

fn easing_expression(easing: &Easing) -> String {
    match easing {
        Easing::Spring { mass, stiffness, damping, initial_velocity } => format!(
            "::ferroui_base::animation::easings::SpringEasing::with_values({mass:?}, {stiffness:?}, {damping:?}, {initial_velocity:?})"
        ),
        _ => format!("::ferroui_base::animation::easings::Easing::parse({:?}).unwrap()", easing.to_xaml()),
    }
}

fn absolute(path: &str) -> String {
    if path.starts_with("crate::") {
        path.into()
    } else {
        format!("::{path}")
    }
}

fn operand(source: &str, span: SourceSpan, next: &str) -> Result<TextPatch, String> {
    let comments = keep_comments(&source[span.range()])?;
    Ok(patch(span, format!("{comments}{next}")))
}

fn patch(span: SourceSpan, replacement: String) -> TextPatch {
    TextPatch { span, replacement }
}
