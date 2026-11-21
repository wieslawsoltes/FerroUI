use ferroui_base::layout::Layoutable;
use ferroui_base::{Rect, Ref, Visual};
use ferroui_controls::ScrollViewer;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;

// Match the existing clipping check's tolerance for layout rounding.
const EDGE_TOLERANCE: f64 = 1.0;

/// A child outside a native rectangular boundary, in logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasuredOverflow {
    pub boundary: String,
    pub clipped: bool,
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl MeasuredOverflow {
    fn distances(&self) -> String {
        let edges = [
            ("left", self.left),
            ("top", self.top),
            ("right", self.right),
            ("bottom", self.bottom),
        ];
        edges
            .into_iter()
            .filter(|(_, value)| *value > 0.0)
            .map(|(edge, value)| {
                let value = format!("{value:.1}");
                format!("{} px {edge}", value.strip_suffix(".0").unwrap_or(&value))
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn summary(&self) -> String {
        let name = if self.clipped { "Clipping" } else { "Overflow" };
        format!("{name} · {}", self.distances())
    }

    pub fn message(&self) -> String {
        let verb = if self.clipped {
            "Clipped by"
        } else {
            "Overflows"
        };
        format!("{verb} {} ({})", self.boundary, self.distances())
    }
}

pub(super) fn measure(ancestry: &[Ref<Visual>]) -> Option<MeasuredOverflow> {
    let visual = ancestry.first()?;
    let layout = visual.cast::<Layoutable>()?;
    if !layout.is_arrange_valid() {
        return None;
    }
    rectangular_clip(ancestry).or_else(|| local_overflow(ancestry, layout.previous_arrange()?))
}

fn rectangular_clip(ancestry: &[Ref<Visual>]) -> Option<MeasuredOverflow> {
    // An arbitrary geometry may already remove the part beyond a rectangular boundary.
    if ancestry.iter().any(|visual| visual.clip().is_some()) {
        return None;
    }
    let visual = ancestry.first()?;
    let viewport = ancestry.last()?;
    let border = Rect::from_size(visual.bounds().size());
    let (mut horizontal, mut vertical) = (true, true);
    for ancestor in ancestry.iter().skip(1) {
        if let Some(scroll) = ancestor.cast::<ScrollContentPresenter>() {
            horizontal &= !scroll.can_horizontally_scroll();
            vertical &= !scroll.can_vertically_scroll();
        } else if let Some(scroll) = ancestor.cast::<ScrollViewer>() {
            horizontal &=
                scroll.horizontal_scroll_bar_visibility() == ScrollBarVisibility::Disabled;
            vertical &= scroll.vertical_scroll_bar_visibility() == ScrollBarVisibility::Disabled;
        }
        if !horizontal && !vertical {
            break;
        }
        if ancestor != viewport && !ancestor.clip_to_bounds() {
            continue;
        }
        let Some(transform) = visual.transform_to_visual(ancestor) else {
            continue;
        };
        let limit = Rect::from_size(ancestor.bounds().size());
        let arranged = border.transform_to_aabb(transform);
        if let Some([left, top, right, bottom]) = overhang(arranged, limit, horizontal, vertical) {
            return Some(MeasuredOverflow {
                boundary: if ancestor == viewport {
                    "the preview viewport".into()
                } else {
                    format!("the {} bounds", ancestor.get_type().name())
                },
                clipped: true,
                left,
                top,
                right,
                bottom,
            });
        }
    }
    None
}

fn local_overflow(ancestry: &[Ref<Visual>], slot: Rect) -> Option<MeasuredOverflow> {
    let [visual, parent, ..] = ancestry else {
        return None;
    };
    let parent_layout = parent.cast::<Layoutable>()?;
    if !parent_layout.is_arrange_valid()
        || parent.cast::<ScrollContentPresenter>().is_some()
        || parent.cast::<ScrollViewer>().is_some()
    {
        return None;
    }
    let bounds = visual.bounds();
    let panel = Rect::from_size(parent.bounds().size());
    let parent_type = parent.get_type().name();
    let (boundary, [left, top, right, bottom]) =
        if let Some(edges) = overhang(bounds, panel, true, true) {
            (format!("the {parent_type} bounds"), edges)
        } else {
            (
                format!("the {parent_type} layout slot"),
                overhang(bounds, slot, true, true)?,
            )
        };
    Some(MeasuredOverflow {
        boundary,
        clipped: false,
        left,
        top,
        right,
        bottom,
    })
}

fn overhang(bounds: Rect, limit: Rect, horizontal: bool, vertical: bool) -> Option<[f64; 4]> {
    if bounds.width <= 0.0 || bounds.height <= 0.0 || limit.width < 0.0 || limit.height < 0.0 {
        return None;
    }
    let edges = [
        limit.x - bounds.x,
        limit.y - bounds.y,
        bounds.right() - limit.right(),
        bounds.bottom() - limit.bottom(),
    ];
    if edges.iter().any(|edge| !edge.is_finite()) {
        return None;
    }
    let visible = [horizontal, vertical, horizontal, vertical];
    let outsets = std::array::from_fn(|index| {
        let edge = edges[index];
        if visible[index] && edge > EDGE_TOLERANCE {
            edge
        } else {
            0.0
        }
    });
    outsets.iter().any(|edge| *edge > 0.0).then_some(outsets)
}
