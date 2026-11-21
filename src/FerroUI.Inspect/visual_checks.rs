use crate::{Inspector, MeasuredOverflow, layout_overflow};
use ferroui_base::{Ref, Visual};
use ferroui_controls::TextBlock;
use ferroui_controls::automation::peers::AutomationControlType;

#[derive(Clone, Debug, PartialEq)]
pub enum VisualFinding {
    MissingAccessibleName(AutomationControlType),
    ClippedLayout(String),
    LowTextContrast { ratio: f64, minimum: f64 },
}

impl VisualFinding {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingAccessibleName(_) => "missingAccessibleName",
            Self::ClippedLayout(_) => "clippedLayout",
            Self::LowTextContrast { .. } => "lowTextContrast",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::MissingAccessibleName(role) => format!("{role:?} needs an accessible name"),
            Self::ClippedLayout(clip) => format!("Arranged bounds extend beyond {clip}"),
            Self::LowTextContrast { ratio, minimum } => {
                format!("Text contrast is {ratio:.2}:1. WCAG AA requires {minimum:.1}:1.")
            }
        }
    }
}

/// Inspect a visible control in the coordinates of a laid-out native artboard.
pub fn check_visual(visual: &Ref<Visual>, viewport: &Ref<Visual>) -> Vec<VisualFinding> {
    let Some(VisualChecks {
        mut accessibility,
        overflow,
    }) = check_visual_with_layout(visual, viewport)
    else {
        return Vec::new();
    };
    if let Some(overflow) = overflow.filter(|issue| issue.clipped) {
        accessibility.push(VisualFinding::ClippedLayout(overflow.boundary));
    }
    accessibility
}

/// Accessibility and measured layout findings from the same visible visual path.
pub struct VisualChecks {
    pub accessibility: Vec<VisualFinding>,
    pub overflow: Option<MeasuredOverflow>,
}

pub fn check_visual_with_layout(
    visual: &Ref<Visual>,
    viewport: &Ref<Visual>,
) -> Option<VisualChecks> {
    if !visual.is_effectively_visible() || visual.bounds().is_empty() {
        return None;
    }
    let mut ancestry = vec![visual.clone()];
    while ancestry.last() != Some(viewport) {
        let parent = ancestry.last()?.visual_parent()?;
        ancestry.push(parent);
    }
    if ancestry
        .iter()
        .any(|entry| entry.is_render_suppressed() || entry.opacity() <= 0.0)
    {
        return None;
    }

    let mut findings = Vec::new();
    let peer = Inspector::peer(&visual.clone().upcast()).ok();
    let role = peer.as_ref().map(|peer| peer.get_automation_control_type());
    let interactive = role.is_some_and(needs_name);
    if let Some(peer) =
        peer.filter(|peer| interactive && peer.is_control_element() && peer.is_enabled())
    {
        if peer.get_name().trim().is_empty() {
            findings.push(VisualFinding::MissingAccessibleName(
                peer.get_automation_control_type(),
            ));
        }
    }
    if let Some(text) = visual.cast::<TextBlock>() {
        if let Some(finding) = crate::text_contrast::check(&text, &ancestry) {
            findings.push(finding);
        }
    }
    Some(VisualChecks {
        accessibility: findings,
        overflow: layout_overflow::measure(&ancestry),
    })
}

fn needs_name(role: AutomationControlType) -> bool {
    use AutomationControlType::*;
    matches!(
        role,
        Button
            | CheckBox
            | ComboBox
            | Edit
            | Expander
            | Hyperlink
            | MenuItem
            | RadioButton
            | Slider
            | Spinner
            | SplitButton
            | TabItem
    )
}
