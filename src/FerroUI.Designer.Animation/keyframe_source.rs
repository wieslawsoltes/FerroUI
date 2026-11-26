//! Preserve authored XAML while updating an existing native keyframe.
mod frame;
#[cfg(test)]
mod tests;
mod value;

use crate::programs::core_error;
use crate::xaml::{native_namespace, native_ticks};
use crate::{
    animation_fragment, require, AnimationCodec, AnimationError, AnimationProgram, ImportOptions,
    Keyframe, KeyframeLocation, NativeAnimationCodec, Result, Timeline,
};
use ferroui_designer_core::{Attribute, Document, Edit, Node, NodeId, TextPatch, XmlText};
use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};

/// Return simultaneous patches for existing native keys. Retiming supplies the
/// original and destination locations to keep the authored identity when keys
/// cross or move into another selected key's previous time.
pub fn native_keyframe_patches<O: Borrow<ImportOptions>>(
    document: &Document,
    program: &AnimationProgram,
    draft: &Timeline,
    retimed: &[(KeyframeLocation, KeyframeLocation)],
    options: impl FnOnce() -> O,
) -> Result<Option<Vec<TextPatch>>> {
    if program.codec.as_deref() != Some(NativeAnimationCodec.id()) {
        return Ok(None);
    }
    let Some(before) = program.timeline.as_ref() else {
        return Ok(None);
    };
    if !same_clock(before, draft) {
        return Ok(None);
    }
    let Some(mut keys) = key_pairs(before, draft, retimed) else {
        return Ok(None);
    };
    draft.validate()?;
    for track in &draft.tracks {
        let mut previous = None;
        for key in &track.keyframes {
            let time = native_ticks(key.time)?;
            require(
                previous != Some(time),
                "Keyframe times collide at native 100 ns precision",
            )?;
            previous = Some(time);
        }
    }
    require(
        document.is_valid(),
        "Wait for valid XAML before editing a keyframe",
    )?;
    if before == draft {
        return Ok(Some(Vec::new()));
    }
    let options = options();
    let options = options.borrow();
    let codec = NativeAnimationCodec;
    let original = animation_fragment(document, program.id)?;
    let authored = codec.decode(&original, program.primary_target(), options)?;
    require(
        same_authored_timeline(before, &authored.timeline),
        "The authored keyframes changed; wait for the current preview",
    )?;

    let mut output = Patches::default();
    for node in children(document, program.id).filter(|node| native_element(node, "KeyFrame")) {
        if !frame::plan(document, node, before.duration, &mut keys, &mut output)? {
            return Ok(None);
        }
    }
    require(
        keys.values().all(BTreeMap::is_empty),
        "The authored keyframe setters changed",
    )?;
    output.text.extend(
        document
            .plan_attribute_patches(output.attributes)
            .map_err(core_error)?,
    );
    output
        .text
        .sort_by_key(|patch| (patch.span.start, patch.span.end));

    let mut candidate = document.clone();
    candidate
        .apply_text_patches(
            candidate.revision(),
            "Check native keyframe patch",
            output.text.clone(),
        )
        .map_err(core_error)?;
    let fragment = animation_fragment(&candidate, program.id)?;
    let matches = codec
        .decode(&fragment, program.primary_target(), options)
        .is_ok_and(|decoded| same_authored_timeline(draft, &decoded.timeline));
    Ok(matches.then_some(output.text))
}

type KeyPairs<'a> = BTreeMap<&'a str, BTreeMap<u64, (&'a Keyframe, &'a Keyframe)>>;

fn key_pairs<'a>(
    before: &'a Timeline,
    draft: &'a Timeline,
    retimed: &[(KeyframeLocation, KeyframeLocation)],
) -> Option<KeyPairs<'a>> {
    if before.tracks.len() != draft.tracks.len() {
        return None;
    }
    let mut destinations = BTreeMap::<(_, _), BTreeMap<_, _>>::new();
    for (old, next) in retimed {
        if old.property != next.property || old.target != next.target {
            return None;
        }
        if destinations
            .entry((old.property.as_str(), old.target))
            .or_default()
            .insert(old.time.to_bits(), next.time)
            .is_some()
        {
            return None;
        }
    }
    let mut pairs = BTreeMap::new();
    for (old, next) in before.tracks.iter().zip(&draft.tracks) {
        if old.property != next.property
            || old.target != next.target
            || old.base_value != next.base_value
            || old.muted != next.muted
            || old.keyframes.len() != next.keyframes.len()
        {
            return None;
        }
        let mut timed = destinations
            .remove(&(old.property.as_str(), old.target))
            .unwrap_or_default();
        let mut used = BTreeSet::new();
        let mut track = BTreeMap::new();
        for key in &old.keyframes {
            let time = timed.remove(&key.time.to_bits()).unwrap_or(key.time);
            let index = next
                .keyframes
                .binary_search_by(|key| key.time.total_cmp(&time))
                .ok()?;
            if !used.insert(index) {
                return None;
            }
            track.insert(key.time.to_bits(), (key, &next.keyframes[index]));
        }
        if !timed.is_empty() || pairs.insert(old.property.as_str(), track).is_some() {
            return None;
        }
    }
    destinations.is_empty().then_some(pairs)
}

fn same_clock(left: &Timeline, right: &Timeline) -> bool {
    left.duration == right.duration
        && left.delay == right.delay
        && left.delay_between_iterations == right.delay_between_iterations
        && left.playback_behavior == right.playback_behavior
        && left.speed_ratio == right.speed_ratio
        && left.repeat == right.repeat
        && left.playback_direction == right.playback_direction
        && left.fill == right.fill
        && left.easing == right.easing
}

fn same_authored_timeline(expected: &Timeline, actual: &Timeline) -> bool {
    same_clock(expected, actual)
        && expected.tracks.len() == actual.tracks.len()
        && expected
            .tracks
            .iter()
            .zip(&actual.tracks)
            .all(|(left, right)| {
                left.target == right.target
                    && left.property == right.property
                    && left.keyframes.len() == right.keyframes.len()
                    && left
                        .keyframes
                        .iter()
                        .zip(&right.keyframes)
                        .all(|(left, right)| {
                            native_ticks(left.time)
                                .is_ok_and(|ticks| native_ticks(right.time).ok() == Some(ticks))
                                && left.value == right.value
                                && left.easing == right.easing
                        })
            })
}

fn children(document: &Document, id: NodeId) -> impl Iterator<Item = &Node> {
    document
        .ast()
        .element_children(id)
        .into_iter()
        .filter_map(|child| document.ast().node(child))
}

fn native_element(node: &Node, name: &str) -> bool {
    node.local_name() == name && native_namespace(node.namespace_uri.as_deref())
}

#[derive(Default)]
struct Patches {
    text: Vec<TextPatch>,
    attributes: Vec<Edit>,
}

impl Patches {
    fn attribute(
        &mut self,
        document: &Document,
        node: &Node,
        name: &str,
        value: Option<String>,
    ) -> Result<()> {
        match (node.attribute_info(name), value) {
            (Some(attribute), Some(value)) => {
                if let Some(patch) = attribute_text_patch(document.source(), attribute, &value)? {
                    self.text.push(patch);
                }
            }
            (_, Some(value)) => self.attributes.push(Edit::SetAttribute {
                node: node.id,
                name: name.into(),
                value,
            }),
            (Some(_), None) => self.attributes.push(Edit::RemoveAttribute {
                node: node.id,
                name: name.into(),
            }),
            _ => {}
        }
        Ok(())
    }
}

pub(crate) fn attribute_text_patch(
    source: &str,
    attribute: &Attribute,
    value: &str,
) -> Result<Option<TextPatch>> {
    let unavailable = || {
        AnimationError(
            "The authored keyframe attribute changed; wait for the current preview".into(),
        )
    };
    let text = XmlText::attribute(source, attribute.value_span).ok_or_else(unavailable)?;
    let Some(patch) = TextPatch::between(text.value(), value) else {
        return Ok(None);
    };
    let span = text
        .source_span(patch.span.range())
        .ok_or_else(unavailable)?;
    let replacement = crate::xaml::escape(&patch.replacement).replace('\'', "&apos;");
    Ok(Some(TextPatch { span, replacement }))
}
