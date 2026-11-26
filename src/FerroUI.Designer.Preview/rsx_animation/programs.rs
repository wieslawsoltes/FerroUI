use super::AuthoredRsxFrame;
use crate::document::DesignDocument;
use ferroui_designer::{validate_rsx_animation_selector, DesignerSession};
use ferroui_designer_animation::{AnimationClipEdit, AnimationClipPlan};
use ferroui_designer_core::{NodeId, TextPatch};
use ferroui_designer_rsx::{RsxAnimationEdit, RsxAnimationPlan};
use ferroui_remote_protocol::designer::DesignAnimationSourcePatch;
use serde_json::Value;

impl AuthoredRsxFrame {
    pub(crate) fn hosted_clip_owner_line(&self, owner: NodeId) -> Option<u32> {
        self.clip_owners.get(&owner).map(|&(_, line)| line)
    }

    pub(crate) fn hosted_editable_clip(&self, program: NodeId) -> bool {
        self.clips.get(&program).is_some_and(|clip| clip.can_edit_program())
    }

    pub(crate) fn plan_hosted_clip_edit(
        &self, command: &AnimationClipEdit,
    ) -> Result<AnimationClipPlan, String> {
        let projection = self.catalog.project(&self.source, self.invocation.start)?;
        let plan = match command {
            AnimationClipEdit::Create { owner, name, selector, duration } => {
                let &(index, _) = self.clip_owners.get(owner).ok_or("Choose a Rust control with editable inline styles")?;
                projection.create_animation(index, name, selector, *duration)
            }
            AnimationClipEdit::Rename { program, name } =>
                projection.edit_animation(self.hosted_declaration(*program)?, RsxAnimationEdit::Rename(name)),
            AnimationClipEdit::Duplicate { program, name } =>
                projection.edit_animation(self.hosted_declaration(*program)?, RsxAnimationEdit::Duplicate(name)),
            AnimationClipEdit::Remove { program } =>
                projection.edit_animation(self.hosted_declaration(*program)?, RsxAnimationEdit::Remove),
        }?;
        let edited = TextPatch::apply_all(&self.source, plan.patches).map_err(|error| error.to_string())?;
        Ok(AnimationClipPlan {
            patches: TextPatch::between(&self.source, &edited).into_iter().collect(),
            selected_key: plan.selected_key,
        })
    }

    pub(crate) fn source_text(&self) -> &str { &self.source }

    fn hosted_declaration(&self, program: NodeId) -> Result<ferroui_designer_core::SourceSpan, String> {
        self.clips.get(&program).filter(|clip| clip.can_edit_program()).map(|clip| clip.declaration)
            .ok_or_else(|| "Choose a clip on an inline Rust style".into())
    }

    pub(crate) fn hosted_clip_patch(&self, placements: &[(NodeId, f64)]) -> Result<Option<DesignAnimationSourcePatch>, String> {
        let placements = placements.iter().map(|&(id, delay)| {
            let clip = self.clips.get(&id).ok_or("Choose a clip from the active Rust invocation")?;
            Ok((clip.declaration, delay))
        }).collect::<Result<Vec<_>, String>>()?;
        let projection = self.catalog.project(&self.source, self.invocation.start)?;
        let patches = projection.place_animation_clips(&placements)?;
        let edited = TextPatch::apply_all(&self.source, patches).map_err(|error| error.to_string())?;
        crate::hosted_animation::checked_source_patch(&self.source, &edited)
    }
}

pub(crate) fn program_edit(tool: &str, action: Option<&str>) -> bool {
    tool == "designer_program"
        && matches!(action, Some("create" | "rename" | "duplicate" | "remove"))
}

pub(crate) fn clip_placement_edit(tool: &str, action: Option<&str>) -> bool {
    tool == "designer_program" && action == Some("place_clips")
}

pub(crate) fn clip_placement_patches(
    design: &DesignDocument<'_>, session: &DesignerSession, args: &Value,
) -> Result<Vec<TextPatch>, String> {
    let rsx = design.rsx.as_ref().ok_or("Open the current Rust view")?;
    let ids: Vec<NodeId> = serde_json::from_value(args["programs"].clone()).map_err(|error| error.to_string())?;
    let placements = session.plan_animation_clip_placement(&ids, args["start"].as_f64(), args["gap"].as_f64().unwrap_or(0.0))?;
    let placements = placements.into_iter().map(|(id, delay)| {
        let clip = design.rsx_animations.get(&id).ok_or("Choose a clip from the active Rust invocation")?;
        Ok((clip.declaration, delay))
    }).collect::<Result<Vec<_>, String>>()?;
    rsx.place_animation_clips(&placements)
}

pub(crate) fn program_patches(
    design: &DesignDocument<'_>,
    args: &Value,
) -> Result<RsxAnimationPlan, String> {
    let rsx = design.rsx.as_ref().ok_or("Open the current Rust view")?;
    let action = args["action"]
        .as_str()
        .ok_or("Choose a Rust animation action")?;
    let name = || args["name"].as_str().ok_or("Enter an animation name");
    if action == "create" {
        let (owner, _) = args["owner"]
            .as_u64()
            .and_then(|id| design.rsx_node(id))
            .ok_or("Choose a mapped Rust control to own the animation")?;
        let selector = args["selector"]
            .as_str()
            .ok_or("Enter a native style selector")?;
        let duration = args["duration"]
            .as_f64()
            .ok_or("Enter the animation duration")?;
        validate_rsx_animation_selector(selector)?;
        return rsx.create_animation(owner, name()?, selector, duration);
    }
    let clip = args["program"]
        .as_u64()
        .and_then(|id| design.rsx_animations.get(&id))
        .ok_or("Choose an animation declared inline on a mapped Rust style")?;
    rsx.edit_animation(
        clip.declaration,
        match action {
            "rename" => RsxAnimationEdit::Rename(name()?),
            "duplicate" => RsxAnimationEdit::Duplicate(name()?),
            "remove" => RsxAnimationEdit::Remove,
            _ => return Err("Choose a Rust animation action".into()),
        },
    )
}
