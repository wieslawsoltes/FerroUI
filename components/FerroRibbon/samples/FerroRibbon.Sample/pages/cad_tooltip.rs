// Copyright (c) 2026 Wiesław Šoltés. MIT. CadPage.ShowToolTip native popup helper.
use super::{CadPage, cad_session::ribbon_item};
use ferroribbon::controls::{RibbonButton, RibbonControlBase, RibbonScreenTipService};
use ferroui_base::{BoxedValue, Point, Rect, Visual, styling::ThemeVariant};
use ferroui_controls::{
    Control, PlacementMode, TopLevel,
    primitives::{
        Popup,
        popup_positioning::{PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment},
    },
};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CadToolTipError {
    Text(&'static str),
    NotAttached,
    Transform,
}
impl std::fmt::Display for CadToolTipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(field) => write!(
                f,
                "CAD ScreenTip {field} cannot be represented by native text"
            ),
            Self::NotAttached => f.write_str("CAD ScreenTip requires an attached page"),
            Self::Transform => {
                f.write_str("CAD ScreenTip item has no transform to the page's visual root")
            }
        }
    }
}
impl std::error::Error for CadToolTipError {}

impl CadPage {
    #[cfg(all(test, feature = "desktop"))]
    pub(crate) fn captured_tip_popup(&self) -> Option<ferroui_base::Ref<Popup>> {
        self.tip_popup.borrow().clone()
    }
    pub fn show_tool_tip(&self, item_id: Option<&str>) -> Result<bool, CadToolTipError> {
        let Some(element) = self.cad_ribbon().find_item(item_id.map(Into::into)) else {
            return Ok(false);
        };
        let Some(item) = ribbon_item(&element) else {
            return Ok(false);
        };
        let label = item
            .label()
            .map(|s| s.to_utf8().map_err(|_| CadToolTipError::Text("Label")))
            .transpose()?;
        // The original intentionally does not include toggle/check/radio controls
        // in these two rich-content branches, and uses Create's enabled default.
        let content = if let Some(button) = element.cast::<RibbonButton>() {
            RibbonScreenTipService::create(label, button.screen_tip(), button.shortcut())
                .map_err(|error| CadToolTipError::Text(error.property))?
        } else if let Some(control) = element.cast::<RibbonControlBase>() {
            RibbonScreenTipService::create(label, control.screen_tip(), control.shortcut())
                .map_err(|error| CadToolTipError::Text(error.property))?
        } else {
            label.map(|label| Rc::new(label) as BoxedValue)
        };
        let Some(tip) = RibbonScreenTipService::create_tool_tip(content) else {
            return Ok(false);
        };
        tip.set_value(
            ThemeVariant::requested_theme_variant_property(),
            self.actual_theme_variant(),
        );
        let root = TopLevel::get_top_level(Some(self)).ok_or(CadToolTipError::NotAttached)?;
        let point = Point::new(0., element.bounds().height + 4.);
        // Pinned Uno UIElement.GetTransform returns identity for an element
        // outside the live tree. Hidden slide-out commands are still findable.
        let origin = if element.is_attached_to_visual_tree() {
            point_in_page_root(&element, &root, point).ok_or(CadToolTipError::Transform)?
        } else {
            point
        };
        // Clone out before Close: source Closed callbacks may reenter this helper.
        let previous = self.tip_popup.borrow().clone();
        if let Some(previous) = previous {
            previous.set_is_open(false)
        }
        let popup = Popup::new();
        popup.set_child(tip);
        popup.set_popup_parent(Some(&root.clone().upcast::<Control>()));
        popup.set_placement_target(root.upcast::<Control>());
        // WinUI Popup offsets are root-relative. Native Popup's default Bottom
        // placement is target-relative, so anchor a zero rect at the root origin.
        popup.set_placement(PlacementMode::AnchorAndGravity);
        popup.set_placement_rect(Some(Rect::default()));
        popup.set_placement_anchor(PopupAnchor::TOP | PopupAnchor::LEFT);
        popup.set_placement_gravity(PopupGravity::BOTTOM | PopupGravity::RIGHT);
        // ShowToolTip uses an explicit root-relative position, including after help expands.
        popup.set_placement_constraint_adjustment(PopupPositionerConstraintAdjustment::NONE);
        popup.set_horizontal_offset(origin.x);
        popup.set_vertical_offset(origin.y);
        popup.set_is_light_dismiss_enabled(true);
        self.tip_popup.replace(Some(popup.clone()));
        popup.set_is_open(true);
        Ok(true)
    }
}

/// WinUI's XamlRoot contains popup presentations that FerroUI hosts as separate
/// visual roots. Compose the native continuous desktop matrices for those roots.
/// The ordinary same-root path needs no platform conversion capability.
pub(crate) fn point_in_page_root(
    element: &Control,
    target: &Visual,
    point: Point,
) -> Option<Point> {
    if let Some(point) = element.translate_point(point, target) {
        return Some(point);
    }
    let source = element.presentation_source()?;
    let source_root = source.root_visual()?;
    let target_source = Visual::presentation_source(target)?;
    let target_root = target_source.root_visual()?;
    let source_point = element.translate_point(point, &source_root)?;
    let desktop_point = source_point.transform(source.client_to_screen_transform()?);
    let target_point =
        desktop_point.transform(target_source.client_to_screen_transform()?.try_invert()?);
    target_root.translate_point(target_point, target)
}
