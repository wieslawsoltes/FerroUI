use crate::Inspector;
use ferroui_base::{Rect, Ref, Visual};
use ferroui_controls::automation::peers::AutomationControlType;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::{ScrollViewer, TextBlock};

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
    if !visual.is_effectively_visible() || visual.bounds().is_empty() {
        return Vec::new();
    }
    let mut ancestry = vec![visual.clone()];
    while ancestry.last() != Some(viewport) {
        let Some(parent) = ancestry.last().and_then(|entry| entry.visual_parent()) else {
            return Vec::new();
        };
        ancestry.push(parent);
    }
    if ancestry
        .iter()
        .any(|entry| entry.is_render_suppressed() || entry.opacity() <= 0.0)
    {
        return Vec::new();
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
    let text = visual.cast::<TextBlock>();
    if (interactive || text.is_some()) && visual != viewport {
        if let Some(clip) = clipping_ancestor(visual, &ancestry) {
            findings.push(VisualFinding::ClippedLayout(clip));
        }
    }
    if let Some(text) = text {
        if let Some(finding) = crate::text_contrast::check(&text, &ancestry) {
            findings.push(finding);
        }
    }
    findings
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

fn clipping_ancestor(visual: &Ref<Visual>, ancestry: &[Ref<Visual>]) -> Option<String> {
    // Scrolling exposes clipped content. A shaped clip or transform needs a scene check.
    if ancestry.iter().any(|entry| {
        entry.clip().is_some()
            || entry.render_transform().is_some()
            || entry.cast::<ScrollContentPresenter>().is_some()
            || entry.cast::<ScrollViewer>().is_some()
    }) {
        return None;
    }
    let viewport = ancestry.last()?;
    for ancestor in ancestry.iter().skip(1) {
        if ancestor != viewport && !ancestor.clip_to_bounds() {
            continue;
        }
        let arranged = Inspector::box_model(visual, ancestor).ok()?.border;
        let clip = Rect::from_size(ancestor.bounds().size());
        if !clip.is_empty() && !clip.inflate(1.0).contains_rect(arranged) {
            return Some(if ancestor == viewport {
                "the preview viewport".into()
            } else {
                format!("the {} clip", ancestor.get_type().name())
            });
        }
    }
    None
}
