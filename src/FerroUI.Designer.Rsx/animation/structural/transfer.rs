use super::super::KeySource;
use super::{NewKey, RsxAnimation};
use crate::{RsxCatalog, blocks::keep_comments};
use ferroui_designer_animation::{Easing, Keyframe, KeyframeLocation, KeyframePaste, Timeline};
use ferroui_designer_core::{SourceSpan, TextPatch};
use std::collections::BTreeMap;

impl RsxAnimation {
    pub fn paste_keyframes(
        &self,
        source: &str,
        catalog: &RsxCatalog,
        timeline: &Timeline,
        paste: &KeyframePaste,
    ) -> Result<(Vec<TextPatch>, Vec<KeyframeLocation>), String> {
        let (candidate, inserted) = timeline
            .with_pasted_keyframes(paste)
            .map_err(|error| error.to_string())?;
        let mut keys = Vec::with_capacity(inserted.len());
        for key in &inserted {
            let authored = timeline
                .track(key.target, &key.property)
                .and_then(|track| track.keyframes.first())
                .ok_or("Choose a mapped Rust destination track")?;
            let getter = self.find(&key.property, authored.time)?.getter.clone();
            let typed = key_at(&candidate, key)?;
            keys.push(NewKey {
                property: key.property.clone(),
                getter,
                time: key.time,
                value: typed.value.clone(),
                easing: typed.easing.clone(),
            });
        }
        Ok((self.insert(source, catalog, &keys)?, inserted))
    }

    pub fn mirror_keyframes(
        &self,
        source: &str,
        catalog: &RsxCatalog,
        timeline: &Timeline,
        locations: &[KeyframeLocation],
        pivot: f64,
    ) -> Result<(Vec<TextPatch>, Vec<KeyframeLocation>), String> {
        let (candidate, mirrored) = timeline
            .with_mirrored_keyframes(locations, pivot)
            .map_err(|error| error.to_string())?;
        let mut patches = self.patch_key_times(
            locations
                .iter()
                .zip(&mirrored)
                .map(|(before, after)| (before.property.as_str(), before.time, after.time)),
        )?;
        let mut replacements = BTreeMap::new();
        for (before, after) in locations.iter().zip(&mirrored) {
            let source_key = self.find(&before.property, before.time)?;
            replacements.insert(
                source_key.setter.start,
                key_at(&candidate, after)?.easing.clone(),
            );
        }
        for frame in self.keys.chunk_by(|left, right| left.frame == right.frame) {
            let current = frame[0].easing();
            let easing = |key: &KeySource| replacements.get(&key.setter.start).unwrap_or(&current);
            let desired = easing(&frame[0]);
            if frame.iter().any(|key| easing(key) != desired) {
                return Err("The mirrored setters need different incoming easings. Give them separate Rust keyframes.".into());
            }
            if desired != &current {
                self.patch_frame_easing(source, &frame[0], desired, &mut patches)?;
            }
        }
        patches.sort_by_key(|patch| patch.span.start);
        let rebuilt = self.reparse(source, catalog, &patches)?;
        if rebuilt.keys.len() != self.keys.len()
            || mirrored.iter().any(|location| {
                let Ok(native) = rebuilt.find(&location.property, location.time) else {
                    return true;
                };
                !key_at(&candidate, location).is_ok_and(|planned| {
                    planned.value == native.value.value && planned.easing == native.easing()
                })
            })
        {
            return Err("The mirrored keys do not round-trip through the Rust builder".into());
        }
        Ok((patches, mirrored))
    }

    fn patch_frame_easing(
        &self,
        source: &str,
        key: &KeySource,
        easing: &Easing,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        match *easing {
            Easing::Linear => {
                if let Some(span) = key.spline_method {
                    let method = source
                        .get(span.range())
                        .ok_or("The Rust easing source changed")?;
                    patches.push(TextPatch {
                        span,
                        replacement: keep_comments(method)?,
                    });
                }
            }
            Easing::CubicBezier { x1, y1, x2, y2 } => {
                if let Some(existing) = &key.spline {
                    for (literal, point) in existing.iter().zip([x1, y1, x2, y2]) {
                        literal.plan(point, patches);
                    }
                } else {
                    patches.push(TextPatch {
                        span: SourceSpan::new(key.end, key.end),
                        replacement: format!(".key_spline({x1:?}, {y1:?}, {x2:?}, {y2:?})"),
                    });
                }
            }
            _ => return Err("Choose linear or cubic Bézier easing for this Rust keyframe".into()),
        }
        Ok(())
    }
}

fn key_at<'a>(timeline: &'a Timeline, location: &KeyframeLocation) -> Result<&'a Keyframe, String> {
    let track = timeline
        .track(location.target, &location.property)
        .ok_or("The destination track changed")?;
    let index = track
        .keyframes
        .binary_search_by(|key| key.time.total_cmp(&location.time))
        .map_err(|_| "The destination keyframe changed")?;
    Ok(&track.keyframes[index])
}
