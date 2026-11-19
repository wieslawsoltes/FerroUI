#[path = "hosted_keyframe_selection.rs"]
mod hosted_selection;

use super::AuthoredRsxFrame;
use ferroui_designer::{DesignerSession, TimelinePreviewPose};
use ferroui_designer_animation::{KeyframeEdit, KeyframeGroupEdit, KeyframeLocation, KeyframePaste};
use ferroui_designer_core::{NodeId, TextPatch};
use ferroui_designer_rsx::RsxAnimation;
use serde_json::Value;

pub(crate) struct SourceEdit {
    pub(crate) patches: Vec<TextPatch>,
    pub(crate) pose: Option<TimelinePreviewPose>,
    pub(crate) keys: Option<Vec<KeyframeLocation>>,
}

pub(crate) fn keyframe_edit(tool: &str, action: Option<&str>) -> bool {
    tool == "designer_program"
        && matches!(
            action,
            Some(
                "upsert_keyframe"
                    | "record_keyframe"
                    | "remove_keyframe"
                    | "move_keyframes"
                    | "scale_keyframes"
                    | "mirror_keyframes"
                    | "paste_keyframes"
                    | "duplicate_keyframes"
                    | "remove_keyframes"
            )
        )
}

impl AuthoredRsxFrame {
    pub(crate) fn validate_keyframe_draft(&self, args: &Value) -> Result<(), String> {
        let clip = self.clip(node(args, "program")?)?;
        clip.author_keyframe(
            &self.source,
            &self.catalog,
            string(args, "property")?,
            draft(args)?,
        )?;
        Ok(())
    }

    pub(crate) fn keyframe_patches(
        &self,
        session: &DesignerSession,
        args: &Value,
    ) -> Result<SourceEdit, String> {
        let program = node(args, "program")?;
        let clip = self.clip(program)?;
        let action = string(args, "action")?;
        if matches!(action, "mirror_keyframes" | "paste_keyframes") {
            let timeline = session.animation_programs().iter().find(|entry| entry.id == program)
                .and_then(|entry| entry.timeline.as_ref()).ok_or("Choose a typed native timeline")?;
            let (patches, keys) = if action == "mirror_keyframes" {
                let keys = serde_json::from_value::<Vec<KeyframeLocation>>(args["keyframes"].clone())
                    .map_err(|error| error.to_string())?;
                clip.mirror_keyframes(&self.source, &self.catalog, timeline, &keys, number(args, "pivot")?)?
            } else {
                let paste = serde_json::from_value::<KeyframePaste>(args["paste"].clone())
                    .map_err(|error| error.to_string())?;
                clip.paste_keyframes(&self.source, &self.catalog, timeline, &paste)?
            };
            return Ok(SourceEdit { patches, pose: None, keys: Some(keys) });
        }
        let group = match action {
            "move_keyframes" => Some(KeyframeGroupEdit::Move(number(args, "offset")?)),
            "scale_keyframes" => Some(KeyframeGroupEdit::Scale {
                pivot: number(args, "pivot")?,
                factor: number(args, "factor")?,
            }),
            "duplicate_keyframes" => Some(KeyframeGroupEdit::Duplicate(number(args, "offset")?)),
            "remove_keyframes" => Some(KeyframeGroupEdit::Remove),
            _ => None,
        };
        if let Some(edit) = group {
            let timeline = session
                .animation_programs()
                .iter()
                .find(|clip| clip.id == program)
                .and_then(|clip| clip.timeline.as_ref())
                .ok_or("Choose a typed native timeline")?;
            let keys: Vec<KeyframeLocation> = serde_json::from_value(args["keyframes"].clone())
                .map_err(|error| error.to_string())?;
            let (patches, keys) =
                clip.author_keyframes(&self.source, &self.catalog, timeline, &keys, edit)?;
            return Ok(SourceEdit { patches, pose: None, keys: Some(keys) });
        }
        let target = node(args, "target")?;
        let property = string(args, "property")?;
        let sample;
        let mut pose = None;
        let edit = match action {
            "upsert_keyframe" => draft(args)?,
            "remove_keyframe" => KeyframeEdit::Remove(number(args, "time")?),
            "record_keyframe" => {
                sample =
                    session.sample_keyframe(program, target, property, number(args, "time")?)?;
                pose = session.animation_program_pose(program, sample.time());
                sample.edit()
            }
            _ => return Err("Choose a mapped Rust keyframe action".into()),
        };
        session.plan_program_keyframe(program, target, property, edit)?;
        let patches = clip.author_keyframe(&self.source, &self.catalog, property, edit)?;
        Ok(SourceEdit { patches, pose, keys: None })
    }

    fn clip(&self, program: NodeId) -> Result<&RsxAnimation, String> {
        self.clips
            .get(&program)
            .ok_or_else(|| "Choose an animation with a mapped Rust declaration".into())
    }
}

fn draft(args: &Value) -> Result<KeyframeEdit<'_>, String> {
    Ok(KeyframeEdit::Upsert {
        previous: args["previous"].as_f64(),
        time: number(args, "time")?,
        value: string(args, "value")?,
        easing: string(args, "easing")?,
    })
}

fn node(args: &Value, name: &str) -> Result<NodeId, String> {
    args[name]
        .as_u64()
        .ok_or_else(|| format!("Choose a keyframe {name}"))
}

fn number(args: &Value, name: &str) -> Result<f64, String> {
    args[name]
        .as_f64()
        .ok_or_else(|| format!("Supply a keyframe {name}"))
}

fn string<'a>(args: &'a Value, name: &str) -> Result<&'a str, String> {
    args[name]
        .as_str()
        .ok_or_else(|| format!("Supply a keyframe {name}"))
}
