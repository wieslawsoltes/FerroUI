//! Snap native artboard geometry before the existing placement or resize planner authors edits.
use crate::workspace_layout_helpers::{insertion_target, layout_target};
use crate::{DesignerSession, PlacementOrigin};
use ferroui_base::{layout::Layoutable, Point, Rect as NativeRect, Ref, Visual};
use ferroui_controls::Canvas;
use ferroui_designer_core::NodeId;
use ferroui_designer_tools::{
    Axis, CanvasGuideSettings, Point as Offset, Rect, SnapGuide, SnapIndex, SnapResult,
    CANVAS_MIN_ZOOM,
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Default, Serialize)]
pub struct CanvasSnapAdjustment {
    pub delta: Offset,
    pub guides: Vec<SnapGuide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Rect>,
}

pub struct CanvasSnapQuery<'a> {
    index: &'a SnapIndex,
    grid: Option<f64>,
    tolerance: f64,
}

impl<'a> CanvasSnapQuery<'a> {
    pub fn new(
        index: &'a SnapIndex,
        settings: &CanvasGuideSettings,
        zoom: f64,
    ) -> Result<Self, String> {
        if !zoom.is_finite() || zoom <= 0.0 {
            return Err("Choose a positive finite canvas zoom".into());
        }
        Ok(Self {
            index,
            grid: settings.snap_to_grid.then_some(settings.grid_spacing),
            tolerance: settings.snap_tolerance / zoom.max(CANVAS_MIN_ZOOM),
        })
    }

    fn snap(&self, bounds: Rect, axis: Option<Axis>) -> Result<SnapResult, String> {
        self.index
            .snap_with_axis(bounds, self.grid, self.tolerance, axis)
            .map_err(|error| error.to_string())
    }
}

impl DesignerSession {
    pub(crate) fn snap_peer_bounds(&self, excluded: &[NodeId], surface: &Ref<Visual>) -> Vec<Rect> {
        let ast = self.document().ast();
        let moving = excluded
            .iter()
            .flat_map(|node| ast.subtree(*node))
            .map(|node| node.id)
            .collect::<BTreeSet<_>>();
        ast.elements()
            .filter(|node| !moving.contains(&node.id))
            .filter(|node| {
                let layer = self.layer_state(node.id);
                !layer.is_hidden() && !layer.outside_isolation
            })
            .filter_map(|node| self.preview.bounds(node.id, surface))
            .map(|rect| Rect::new(rect.x, rect.y, rect.width, rect.height))
            .collect()
    }

    pub fn prepare_canvas_snap(
        &self,
        settings: &CanvasGuideSettings,
        excluded: &[NodeId],
    ) -> Result<SnapIndex, String> {
        settings.validate()?;
        let peers = if settings.snap_to_objects {
            self.snap_peer_bounds(excluded, &self.preview.host.clone().upcast::<Visual>())
        } else {
            Vec::new()
        };
        settings
            .snap_settings(1.0)
            .prepare(&peers)
            .map_err(|error| error.to_string())
    }

    pub fn snap_canvas_placement(
        &self,
        origin: &PlacementOrigin<'_>,
        pointer: Point,
        query: &CanvasSnapQuery<'_>,
        recording: bool,
    ) -> Result<CanvasSnapAdjustment, String> {
        let (nodes, anchor, grab) = match origin {
            PlacementOrigin::Asset(_) => (&[][..], None, None),
            PlacementOrigin::Node { node, grab } => {
                (std::slice::from_ref(node), Some(*node), *grab)
            }
            PlacementOrigin::Nodes {
                nodes,
                anchor,
                grab,
            } => (*nodes, Some(*anchor), *grab),
        };
        let resolve = |position| {
            if nodes.is_empty() {
                insertion_target(self, position).ok()
            } else {
                layout_target(self, position, nodes).map(|target| (target, position))
            }
        };
        let hit = resolve(pointer);
        if !recording && !hit.is_some_and(|(target, _)| self.is_free_canvas(target)) {
            return Ok(CanvasSnapAdjustment::default());
        }
        let point = hit.map_or(pointer, |(_, point)| point);
        let moving = if let Some(anchor) = anchor {
            let artboard = self.preview.host.clone().upcast::<Visual>();
            let anchor_bounds = self
                .preview
                .bounds(anchor, &artboard)
                .ok_or("The drag anchor has no measured bounds")?;
            let same_canvas = recording
                || hit.is_some_and(|(target, _)| {
                    nodes.iter().all(|node| {
                        crate::workspace_tools::parent(self, *node).ok() == Some(target)
                    })
                });
            let bounds = self.canvas_selection_bounds(nodes, same_canvas)?;
            let grab = grab.unwrap_or(Point::new(anchor_bounds.x, anchor_bounds.y));
            Rect::new(
                bounds.x + point.x - grab.x,
                bounds.y + point.y - grab.y,
                bounds.width,
                bounds.height,
            )
        } else {
            Rect::new(point.x, point.y, 0.0, 0.0)
        };
        let snapped = query.snap(moving, None)?;
        if snapped.guides.is_empty() {
            return Ok(CanvasSnapAdjustment::default());
        }
        let proposed = Point::new(point.x + snapped.delta.x, point.y + snapped.delta.y);
        // The pointer still belongs to the same authored drop target after applying the offset.
        // Flow, Grid, content slots and nearby fallback positions retain their native semantics.
        if !recording && (resolve(proposed) != hit.map(|(target, _)| (target, proposed))) {
            return Ok(CanvasSnapAdjustment::default());
        }
        Ok(CanvasSnapAdjustment {
            delta: Offset::new(proposed.x - pointer.x, proposed.y - pointer.y),
            guides: snapped.guides,
            bounds: anchor.map(|_| snapped.bounds),
        })
    }

    pub fn snap_canvas_resize(
        &self,
        node: NodeId,
        handle: u8,
        dx: f64,
        dy: f64,
        query: &CanvasSnapQuery<'_>,
    ) -> Result<CanvasSnapAdjustment, String> {
        self.validate_resize_handle(handle, dx, dy)?;
        self.ensure_layer_manipulable(node)?;
        let host = self.preview.host.clone().upcast::<Visual>();
        let before = self
            .preview
            .bounds(node, &host)
            .ok_or("The selected control has no measured bounds")?;
        let visual = self
            .preview
            .visual(node)
            .ok_or("No live layout for the selected control")?;
        let geometry = crate::affine_manipulation::Geometry::capture(&visual, &host);
        if geometry.is_none_or(|geometry| {
            geometry.to_surface.m12.abs() > 1e-9 || geometry.to_surface.m21.abs() > 1e-9
        }) {
            return Ok(CanvasSnapAdjustment::default());
        }
        let axis = match handle {
            1 | 5 => Some(Axis::Vertical),
            3 | 7 => Some(Axis::Horizontal),
            0 | 2 | 4 | 6 => None,
            _ => return Err("Choose a resize handle from 0 through 7".into()),
        };
        let before = self.logical_canvas_bounds(node, before, &host);
        let resize = |dx, dy| crate::affine_manipulation::resize(before, dx, dy, handle);
        let handle_at = |bounds| crate::affine_manipulation::local_handles(bounds)[handle as usize];
        let point = handle_at(resize(dx, dy));
        let mut snapped = query.snap(Rect::new(point.x, point.y, 0.0, 0.0), axis)?;
        let after = handle_at(resize(dx + snapped.delta.x, dy + snapped.delta.y));
        snapped.guides.retain(|guide| {
            let actual = match guide.axis {
                Axis::Horizontal => after.x,
                Axis::Vertical => after.y,
            };
            (actual - guide.position).abs() < 0.000_001
        });
        if !snapped
            .guides
            .iter()
            .any(|guide| guide.axis == Axis::Horizontal)
        {
            snapped.delta.x = 0.0;
        }
        if !snapped
            .guides
            .iter()
            .any(|guide| guide.axis == Axis::Vertical)
        {
            snapped.delta.y = 0.0;
        }
        if self
            .validate_resize_handle(handle, dx + snapped.delta.x, dy + snapped.delta.y)
            .is_err()
        {
            return Ok(CanvasSnapAdjustment::default());
        }
        let bounds = resize(dx + snapped.delta.x, dy + snapped.delta.y);
        Ok(CanvasSnapAdjustment {
            delta: snapped.delta,
            bounds: (!snapped.guides.is_empty())
                .then(|| Rect::new(bounds.x, bounds.y, bounds.width, bounds.height)),
            guides: snapped.guides,
        })
    }

    pub(crate) fn canvas_selection_bounds(
        &self,
        nodes: &[NodeId],
        authored: bool,
    ) -> Result<NativeRect, String> {
        let artboard = self.preview.host.clone().upcast();
        let mut bounds = nodes.iter().map(|&node| {
            self.preview.bounds(node, &artboard).map(|rect| {
                if authored {
                    self.logical_canvas_bounds(node, rect, &artboard)
                } else {
                    rect
                }
            })
        });
        let first = bounds
            .next()
            .flatten()
            .ok_or("The selection has no measured bounds")?;
        bounds
            .try_fold(first, |union, rect| rect.map(|rect| union.union(rect)))
            .ok_or_else(|| "The selection has no measured bounds".into())
    }

    /// A Canvas writes its unrounded arrange position. Use that anchor before adding a
    /// gesture delta so a rounded bitmap cannot shift the authored guide coordinate.
    fn logical_canvas_bounds(
        &self,
        node: NodeId,
        bounds: NativeRect,
        artboard: &Visual,
    ) -> NativeRect {
        let Some(child) = self
            .preview
            .visual(node)
            .and_then(|visual| visual.cast::<Layoutable>())
        else {
            return bounds;
        };
        let Some(parent) = child
            .visual_parent()
            .filter(|parent| parent.cast::<Canvas>().is_some())
        else {
            return bounds;
        };
        let Some(slot) = child.previous_arrange() else {
            return bounds;
        };
        let Some(matrix) = parent
            .transform_to_visual(artboard)
            .and_then(crate::affine_manipulation::affine)
        else {
            return bounds;
        };
        let margin = child.margin();
        let visible = child.bounds();
        let delta = Point::new(
            slot.x + margin.left - visible.x,
            slot.y + margin.top - visible.y,
        )
        .transform(matrix)
            - Point::default().transform(matrix);
        NativeRect::new(
            bounds.x + delta.x,
            bounds.y + delta.y,
            bounds.width,
            bounds.height,
        )
    }

    pub(crate) fn is_free_canvas(&self, node: NodeId) -> bool {
        self.layout_tools()
            .is_builtin_adapter_for(self.document(), node)
            && !crate::workspace_tools::is_custom_panel(self, node)
            && self
                .preview
                .visual(node)
                .is_some_and(|visual| visual.cast::<Canvas>().is_some())
    }
}
