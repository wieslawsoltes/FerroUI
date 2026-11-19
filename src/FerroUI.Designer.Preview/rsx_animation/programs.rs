use crate::document::DesignDocument;
use ferroui_designer::validate_rsx_animation_selector;
use ferroui_designer_rsx::{RsxAnimationEdit, RsxAnimationPlan};
use serde_json::Value;

pub(crate) fn program_edit(tool: &str, action: Option<&str>) -> bool {
    tool == "designer_program"
        && matches!(action, Some("create" | "rename" | "duplicate" | "remove"))
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
