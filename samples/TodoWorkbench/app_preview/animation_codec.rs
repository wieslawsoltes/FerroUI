use ferroui_base::media::BoxShadows;
use ferroui_designer_animation::{
    AnimationCodec, AnimationError, AnimationImport, AnimationValue, ImportOptions,
    NativeAnimationCodec, Result, Timeline, ValueKind,
};

pub(super) struct ReviewShadowCodec;

impl AnimationCodec for ReviewShadowCodec {
    fn id(&self) -> &str {
        "todo-workbench.review-shadow"
    }

    fn can_decode(&self, source: &str) -> bool {
        source.contains("<!--todo-workbench-review-shadow-->")
            && NativeAnimationCodec.can_decode(source)
    }

    fn decode(
        &self,
        source: &str,
        target: u64,
        options: &ImportOptions,
    ) -> Result<AnimationImport> {
        let mut options = options.clone();
        // The editor stores the CSS literal. FerroUI still interpolates native BoxShadows.
        options
            .property_kinds
            .insert("BoxShadow".into(), ValueKind::Discrete);
        options.interpolation_errors.remove("BoxShadow");
        options.base_values.remove("BoxShadow");
        options
            .literal_values
            .retain(|(property, _), _| property != "BoxShadow");
        let imported = NativeAnimationCodec.decode(source, target, &options)?;
        validate(&imported.timeline)?;
        Ok(imported)
    }

    fn encode(&self, original: &str, timeline: &Timeline, target: u64) -> Result<String> {
        validate(timeline)?;
        NativeAnimationCodec.encode(original, timeline, target)
    }
}

fn validate(timeline: &Timeline) -> Result<()> {
    for track in &timeline.tracks {
        if track.property != "BoxShadow" {
            return Err(AnimationError(
                "Review shadow supports only BoxShadow tracks".into(),
            ));
        }
        for key in &track.keyframes {
            let AnimationValue::Discrete(literal) = &key.value else {
                return Err(AnimationError("Enter a BoxShadow literal".into()));
            };
            BoxShadows::parse(literal)
                .map_err(|_| AnimationError("Enter a valid BoxShadow literal".into()))?;
        }
    }
    Ok(())
}
