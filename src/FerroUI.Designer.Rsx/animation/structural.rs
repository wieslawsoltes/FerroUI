//! Checked method-chain edits for one authored RSX animation.
use super::{KeySource, RsxAnimation, keyframe::BUILTINS, syntax, value::rust_literal};
use crate::{RsxCatalog, blocks::keep_comments, source_edits::line_prefix};
use ferroui_designer_animation::{
    AnimationValue, Easing, KeyframeEdit, KeyframeGroupEdit, KeyframeLocation, Timeline,
    native_key_time,
};
use ferroui_designer_core::{SourceSpan, TextPatch};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

struct NewKey {
    property: String,
    getter: String,
    time: f64,
    value: AnimationValue,
    easing: Easing,
}

impl RsxAnimation {
    /// Plan an edit against the original source of this accepted projection.
    pub fn author_keyframe(
        &self,
        source: &str,
        catalog: &RsxCatalog,
        property: &str,
        edit: KeyframeEdit<'_>,
    ) -> Result<Vec<TextPatch>, String> {
        match edit {
            KeyframeEdit::Upsert {
                previous: None,
                time,
                value,
                easing,
            } => {
                let existing = self.keys.iter().find(|key| key.property == property);
                let (getter, kind) = if let Some(key) = existing {
                    (key.getter.clone(), key.value.value.kind())
                } else {
                    let (path, shape) = BUILTINS
                        .iter()
                        .find(|(_, name, _)| *name == property)
                        .map(|(path, _, shape)| (*path, *shape))
                        .or_else(|| catalog.animation_getter(property, self.target_type.as_ref()))
                        .ok_or("This animation property has no unambiguous Rust getter")?;
                    let path = if path.starts_with("crate::") {
                        path.into()
                    } else {
                        format!("::{path}")
                    };
                    (path, shape.kind())
                };
                let key = NewKey {
                    property: property.into(),
                    getter,
                    time: self.authored_time(time)?,
                    value: AnimationValue::parse(value, kind).map_err(|error| error.to_string())?,
                    easing: Easing::parse(easing).map_err(|error| error.to_string())?,
                };
                self.insert(source, catalog, &[key])
            }
            KeyframeEdit::Remove(time) => self.remove(source, catalog, [(property, time)]),
            _ => self.patch_keyframe(property, edit),
        }
    }

    /// Apply the native group's target and timing checks before planning Rust patches.
    pub fn author_keyframes(
        &self,
        source: &str,
        catalog: &RsxCatalog,
        timeline: &Timeline,
        keys: &[KeyframeLocation],
        edit: KeyframeGroupEdit,
    ) -> Result<(Vec<TextPatch>, Vec<KeyframeLocation>), String> {
        let (_, mut destinations) = timeline
            .with_keyframe_edit(keys, edit)
            .map_err(|error| error.to_string())?;
        for key in &mut destinations {
            key.time = self.authored_time(key.time)?;
        }
        let locations = keys.iter().map(|key| (key.property.as_str(), key.time));
        let patches = match edit {
            KeyframeGroupEdit::Remove => self.remove(source, catalog, locations)?,
            KeyframeGroupEdit::Duplicate(_) => {
                let new = locations
                    .zip(&destinations)
                    .map(|((property, time), destination)| {
                        let key = self.find(property, time)?;
                        Ok::<_, String>(NewKey {
                            property: key.property.clone(),
                            getter: key.getter.clone(),
                            time: destination.time,
                            value: key.value.value.clone(),
                            easing: key.easing(),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                self.insert(source, catalog, &new)?
            }
            _ => self.patch_key_times(
                locations
                    .zip(&destinations)
                    .map(|((property, time), destination)| (property, time, destination.time)),
            )?,
        };
        Ok((patches, destinations))
    }

    fn find(&self, property: &str, time: f64) -> Result<&KeySource, String> {
        self.keys
            .iter()
            .find(|key| key.property == property && key.time == time)
            .ok_or_else(|| "Choose an authored Rust keyframe".into())
    }

    fn authored_time(&self, time: f64) -> Result<f64, String> {
        if !(0.0..=self.duration).contains(&time) {
            return Err("Choose a key time within the animation duration".into());
        }
        native_key_time(time).map_err(|error| error.to_string())
    }

    fn insert(
        &self,
        source: &str,
        catalog: &RsxCatalog,
        keys: &[NewKey],
    ) -> Result<Vec<TextPatch>, String> {
        let last = self.keys.last().map(|key| key.frame);
        let offset = last.map_or(self.factory_end, |frame| frame.end);
        let prefix = line_prefix(source, last.map(|frame| frame.start).or(self.first_method));
        let mut replacement = String::new();
        let mut times = self
            .keys
            .iter()
            .map(|key| (key.property.as_str(), key.time.to_bits()))
            .collect::<BTreeSet<_>>();
        for key in keys {
            if !times.insert((&key.property, key.time.to_bits())) {
                return Err("Another Rust keyframe occupies the destination time".into());
            }
            let cue = key.time / self.duration;
            let literal = rust_literal(&key.value)?;
            write!(
                replacement,
                "{prefix}.key_frame({cue:?}, |key| key.set({}(), {literal})",
                key.getter
            )
            .expect("writing to a string");
            match key.easing {
                Easing::Linear => {}
                Easing::CubicBezier { x1, y1, x2, y2 } => {
                    write!(replacement, ".key_spline({x1:?}, {y1:?}, {x2:?}, {y2:?})")
                        .expect("writing to a string");
                }
                _ => return Err("Choose linear or cubic Bézier easing for a Rust keyframe".into()),
            }
            replacement.push(')');
        }
        let patches = vec![TextPatch {
            span: SourceSpan::new(offset, offset),
            replacement,
        }];
        let rebuilt = self.reparse(source, catalog, &patches)?;
        let mapped = rebuilt
            .keys
            .iter()
            .map(|key| ((key.property.as_str(), key.time.to_bits()), key))
            .collect::<BTreeMap<_, _>>();
        if mapped.len() != times.len()
            || keys.iter().any(|key| {
                mapped
                    .get(&(key.property.as_str(), key.time.to_bits()))
                    .is_none_or(|mapped| {
                        mapped.value.value != key.value || mapped.easing() != key.easing
                    })
            })
        {
            return Err("The inserted keys do not round-trip through the Rust builder".into());
        }
        Ok(patches)
    }

    fn remove<'a>(
        &self,
        source: &str,
        catalog: &RsxCatalog,
        keys: impl IntoIterator<Item = (&'a str, f64)>,
    ) -> Result<Vec<TextPatch>, String> {
        let selected = keys
            .into_iter()
            .map(|(property, time)| {
                let key = self.find(property, time)?;
                Ok::<_, String>(key.setter.start)
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let mut patches = Vec::new();
        for frame in self.keys.chunk_by(|left, right| left.frame == right.frame) {
            if frame.iter().all(|key| selected.contains(&key.setter.start)) {
                patches.push(self.remove_method(source, frame[0].frame)?);
            } else {
                for key in frame
                    .iter()
                    .filter(|key| selected.contains(&key.setter.start))
                {
                    patches.push(self.remove_method(source, key.setter)?);
                }
            }
        }
        let rebuilt = self.reparse(source, catalog, &patches)?;
        if rebuilt.keys.len() + selected.len() != self.keys.len() {
            return Err("The removed keys do not round-trip through the Rust builder".into());
        }
        Ok(patches)
    }

    fn remove_method(&self, source: &str, mut span: SourceSpan) -> Result<TextPatch, String> {
        let method = source
            .get(span.range())
            .ok_or("The Rust keyframe source changed")?;
        let replacement = keep_comments(method)?;
        if replacement.is_empty() {
            let prefix = line_prefix(source, Some(span.start));
            if !prefix.is_empty() {
                span.start = (span.start - prefix.len()).max(self.declaration.start);
            }
        }
        Ok(TextPatch { span, replacement })
    }

    fn reparse(
        &self,
        source: &str,
        catalog: &RsxCatalog,
        patches: &[TextPatch],
    ) -> Result<Self, String> {
        let declaration = self.declaration;
        let original = source
            .get(declaration.range())
            .ok_or("The Rust animation source changed")?;
        let relative = patches
            .iter()
            .map(|patch| {
                if patch.span.start < declaration.start || patch.span.end > declaration.end {
                    return Err("Keep the edit in the mapped Rust animation");
                }
                Ok(TextPatch {
                    span: SourceSpan::new(
                        patch.span.start - declaration.start,
                        patch.span.end - declaration.start,
                    ),
                    replacement: patch.replacement.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let changed =
            TextPatch::apply_all(original, relative).map_err(|error| error.to_string())?;
        let expression = syn::parse_str(&changed).map_err(|error| error.to_string())?;
        syntax::clip(&expression, catalog)
            .ok_or_else(|| "The edited Rust animation has no source mapping".into())
    }
}
