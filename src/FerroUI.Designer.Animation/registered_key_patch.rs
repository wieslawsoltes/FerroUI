use crate::{
    AnimationCodec, AnimationEffect, AnimationError, AnimationProgram, AnimationValue,
    NativeAnimationCodec, Result, Timeline, require,
};
use ferroui_designer_core::{Attribute, Document, TextPatch, XmlText};

/// Map an existing, value-only registered effect edit to its original XAML
/// attribute. Timing edits, structural changes and project codecs retain
/// their own authoring paths.
pub fn registered_key_value_patch(
    document: &Document,
    program: &AnimationProgram,
    draft: &Timeline,
) -> Result<Option<TextPatch>> {
    if program.codec.as_deref() != Some(NativeAnimationCodec.id()) {
        return Ok(None);
    }
    let Some(original) = &program.timeline else {
        return Ok(None);
    };
    let Some((track, index)) = changed_key(original, draft) else {
        return Ok(None);
    };
    let before = &original.tracks[track].keyframes[index];
    let after = &draft.tracks[track].keyframes[index];
    let (
        AnimationValue::Effect(AnimationEffect::Registered { source: old }),
        AnimationValue::Effect(AnimationEffect::Registered { source: new }),
    ) = (&before.value, &after.value)
    else {
        return Ok(None);
    };
    let mut check = draft.clone();
    check.tracks[track].keyframes[index].value = before.value.clone();
    if check != *original {
        return Ok(None);
    }
    require(
        document.source() == document.valid_source(),
        "Wait for valid XAML before editing an effect key",
    )?;
    let property = &original.tracks[track].property;
    let ast = document.ast();
    let frames = ast
        .element_children(program.id)
        .into_iter()
        .filter_map(|id| ast.node(id))
        .filter(|node| {
            node.local_name() == "KeyFrame"
                && crate::xaml::native_namespace(node.namespace_uri.as_deref())
        });
    let mut attributes = Vec::new();
    for frame in frames {
        if crate::xaml::key_time(
            frame.attribute("Cue"),
            frame.attribute("KeyTime"),
            original.duration,
        )? == before.time
        {
            attributes.extend(
                ast.element_children(frame.id)
                    .into_iter()
                    .filter_map(|id| ast.node(id))
                    .filter(|node| {
                        node.local_name() == "Setter"
                            && node.attribute("Property") == Some(property)
                    })
                    .filter_map(|setter| setter.attribute_info("Value")),
            );
        }
    }
    let [attribute] = attributes.as_slice() else {
        return Err(AnimationError(
            "The authored effect key no longer has a unique literal setter".into(),
        ));
    };
    lexical_patch(document.source(), attribute, old, new).map(Some)
}

fn changed_key(original: &Timeline, draft: &Timeline) -> Option<(usize, usize)> {
    original
        .tracks
        .iter()
        .zip(&draft.tracks)
        .enumerate()
        .find_map(|(track, (a, b))| {
            a.keyframes
                .iter()
                .zip(&b.keyframes)
                .position(|(a, b)| a.value != b.value)
                .map(|key| (track, key))
        })
}

fn lexical_patch(source: &str, attribute: &Attribute, old: &str, new: &str) -> Result<TextPatch> {
    let unmapped = || {
        AnimationError("The authored effect literal changed; wait for the current preview".into())
    };
    let text = XmlText::attribute(source, attribute.value_span).ok_or_else(unmapped)?;
    let literal = text.value().strip_prefix("{}").unwrap_or(text.value());
    require(literal == old, unmapped().0)?;
    let escape = text.value().len() - literal.len();
    let patch = TextPatch::between(old, new).ok_or_else(unmapped)?;
    let span = text
        .source_span(patch.span.start + escape..patch.span.end + escape)
        .ok_or_else(unmapped)?;
    let replacement = crate::xaml::escape(&patch.replacement).replace('\'', "&apos;");
    Ok(TextPatch { span, replacement })
}
