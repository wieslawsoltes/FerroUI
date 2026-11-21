use crate::{Inspector, VisualFinding};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::{Rect, Ref, Visual};
use ferroui_controls::{Border, Panel, TextBlock};

pub(super) fn check(text: &Ref<TextBlock>, ancestry: &[Ref<Visual>]) -> Option<VisualFinding> {
    if text.text()?.trim().is_empty()
        || text.text_runs().is_some()
        || ancestry.iter().any(|entry| {
            entry.opacity() != 1.0 || entry.opacity_mask().is_some() || entry.effect().is_some()
        })
    {
        return None;
    }
    let foreground = opaque_color(text.foreground()?.as_ref())?;
    let background = text_background(text, ancestry)?;
    let first = luminance(foreground);
    let second = luminance(background);
    let ratio = (first.max(second) + 0.05) / (first.min(second) + 0.05);
    // WCAG large text is 18pt or at least 14pt bold, at 96 DIPs per inch.
    let size = text.font_size();
    let large = size >= 24.0 || size >= 14.0 * 96.0 / 72.0 && text.font_weight().0 >= 700;
    let minimum = if large { 3.0 } else { 4.5 };
    (ratio < minimum).then_some(VisualFinding::LowTextContrast { ratio, minimum })
}

fn text_background(text: &Ref<TextBlock>, ancestry: &[Ref<Visual>]) -> Option<Color> {
    if let Some(brush) = text.background() {
        return opaque_color(brush.as_ref());
    }
    for ancestor in ancestry.iter().skip(1) {
        let brush = if let Some(panel) = ancestor.cast::<Panel>() {
            panel.background()
        } else if let Some(border) = ancestor.cast::<Border>() {
            if border.corner_radius() != Default::default()
                || border.border_thickness() != Default::default()
            {
                return None;
            }
            border.background()
        } else {
            return None;
        };
        if let Some(brush) = brush {
            let layout = Inspector::box_model(&text.clone().upcast(), ancestor).ok()?;
            if !Rect::from_size(ancestor.bounds().size()).contains_rect(layout.border) {
                return None;
            }
            return opaque_color(brush.as_ref());
        }
    }
    None
}

fn opaque_color(brush: &dyn IBrush) -> Option<Color> {
    let color = brush.as_solid_color_brush()?.color();
    (brush.opacity() == 1.0 && color.a == u8::MAX).then_some(color)
}

fn luminance(color: Color) -> f64 {
    fn linear(channel: u8) -> f64 {
        let channel = f64::from(channel) / 255.0;
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}
