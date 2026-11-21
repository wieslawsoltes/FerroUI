//! Source RibbonPopupPlacement.cs. Native placement is anchored at the window's
//! client origin after applying the source's own window-clamping calculations.
use ferroui_base::{Point, Rect, Size};
use ferroui_controls::{
    Control, PlacementMode, TopLevel,
    primitives::{
        Popup,
        popup_positioning::{PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment},
    },
};

pub struct RibbonPopupPlacement;
ferroui_base::ferro_markup_type!(static RibbonPopupPlacement {
    namespace:"FerroRibbon.Controls.Primitives",
    methods:[
        static try fn PlaceBelow(Option<ferroui_base::Ref<Popup>>,Option<ferroui_base::Ref<Control>>)=>|popup,anchor|markup_place(popup,anchor,false,2.),
        static try fn PlaceBelow(Option<ferroui_base::Ref<Popup>>,Option<ferroui_base::Ref<Control>>,bool)=>|popup,anchor,overlap|markup_place(popup,anchor,overlap,2.),
        static try fn PlaceBelow(Option<ferroui_base::Ref<Popup>>,Option<ferroui_base::Ref<Control>>,bool,f64)=>markup_place,
    ],
});
fn markup_place(
    popup: Option<ferroui_base::Ref<Popup>>,
    anchor: Option<ferroui_base::Ref<Control>>,
    overlap: bool,
    gap: f64,
) -> Result<(), String> {
    let popup = popup.ok_or_else(|| "Value cannot be null. (Parameter 'popup')".to_string())?;
    let anchor = anchor.ok_or_else(|| "Value cannot be null. (Parameter 'anchor')".to_string())?;
    RibbonPopupPlacement::place_below_with_options(&popup, &anchor, overlap, gap);
    Ok(())
}
impl RibbonPopupPlacement {
    pub fn place_below(popup: &Popup, anchor: &Control) {
        Self::place_below_with_options(popup, anchor, false, 2.)
    }
    pub fn place_below_with_options(
        popup: &Popup,
        anchor: &Control,
        overlap_anchor: bool,
        gap: f64,
    ) {
        let root = TopLevel::get_top_level(Some(anchor));
        let origin = root
            .as_ref()
            .and_then(|root| anchor.translate_point(Point::default(), root))
            .unwrap_or_default();
        let size = if let Some(child) = popup.child() {
            child.measure(Size::INFINITY);
            child.desired_size()
        } else {
            Size::default()
        };
        let window = root
            .as_ref()
            .map(|root| root.client_size())
            .unwrap_or(Size::INFINITY);
        let offset = position(
            origin,
            anchor.bounds().height,
            size,
            window,
            overlap_anchor,
            gap,
        );
        // Zero-sized client-origin rect: bottom/right gravity leaves the popup's
        // top-left at the calculated offset. Disable the SDK's additional flipping.
        if let Some(root) = root {
            popup.set_placement_target(root.upcast::<Control>());
        } else {
            popup.set_placement_target(anchor.to_ref());
        }
        popup.set_placement(PlacementMode::AnchorAndGravity);
        popup.set_placement_rect(Some(Rect::default()));
        popup.set_placement_anchor(PopupAnchor::TOP | PopupAnchor::LEFT);
        popup.set_placement_gravity(PopupGravity::BOTTOM | PopupGravity::RIGHT);
        popup.set_placement_constraint_adjustment(PopupPositionerConstraintAdjustment::NONE);
        popup.set_horizontal_offset(offset.x);
        popup.set_vertical_offset(offset.y);
    }
}
pub(crate) fn position(
    origin: Point,
    anchor_height: f64,
    size: Size,
    window: Size,
    overlap: bool,
    gap: f64,
) -> Point {
    let mut x = origin.x;
    if x + size.width > window.width - 4. {
        x = ferroui_base::utilities::math_utilities::max(4., window.width - size.width - 4.);
    }
    let mut y = if overlap {
        origin.y
    } else {
        origin.y + anchor_height + gap
    };
    if y + size.height > window.height - 4. && origin.y - size.height - gap > 4. && !overlap {
        y = origin.y - size.height - gap;
    } else if y + size.height > window.height - 4. {
        y = ferroui_base::utilities::math_utilities::max(4., window.height - size.height - 4.);
    }
    Point::new(x, y)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unchanged_source_helper_numeric_corpus() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/popup-placement.json"))
                .unwrap();
        let value = |bits: &serde_json::Value| {
            f64::from_bits(u64::from_str_radix(bits.as_str().unwrap(), 16).unwrap())
        };
        let pair = |pair: &serde_json::Value| [value(&pair[0]), value(&pair[1])];
        for (index, case) in corpus["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let [x, y] = pair(&case["origin"]);
            let [w, h] = pair(&case["window"]);
            let [sw, sh] = pair(&case["size"]);
            let point = position(
                Point::new(x, y),
                value(&case["height"]),
                Size::new(sw, sh),
                Size::new(w, h),
                case["overlap"].as_bool().unwrap(),
                value(&case["gap"]),
            );
            assert_eq!(
                [point.x.to_bits(), point.y.to_bits()],
                pair(&case["position"]).map(f64::to_bits),
                "source case {index}"
            );
        }
        assert_eq!(
            corpus["corpus"]["errors"],
            serde_json::json!(["popup", "anchor"])
        );
    }
}
