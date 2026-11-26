//! Read-only navigation ranks, scoped to the native group owning each TabIndex.
use super::{DesignerSession, FocusOrderCatalog, KeyboardNavigationMode, NodeId, Ref, Visual};
use ferroui_base::media::{
    BoxShadows, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, Pen, SolidColorBrush,
    Typeface,
};
use ferroui_base::utilities::current_locale;
use ferroui_base::{Point, Rect};
use ferroui_designer_tools::{Point as OverlayPoint, Rect as OverlayRect};
use serde::Serialize;
use std::rc::Rc;

#[derive(Serialize)]
pub(crate) struct FocusOrderOverlay {
    badges: Vec<FocusBadge>,
    links: Vec<[usize; 2]>,
    labels: Vec<LabelLink>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FocusBadge {
    node: NodeId,
    group: usize,
    rank: usize,
    label: String,
    bounds: OverlayRect,
    group_entry: bool,
}

#[derive(Serialize)]
struct LabelLink {
    node: NodeId,
    target: NodeId,
    label: String,
    from: OverlayPoint,
    to: OverlayPoint,
}

impl FocusOrderOverlay {
    fn build(
        session: &DesignerSession,
        relative_to: &Ref<Visual>,
        catalog: &FocusOrderCatalog,
    ) -> Self {
        let mut overlay = Self {
            badges: Vec::new(),
            links: Vec::new(),
            labels: Vec::new(),
        };
        let bounds = |node| {
            let state = session.layer_state(node);
            if state.is_hidden() || state.outside_isolation {
                return None;
            }
            let rect = session.preview.bounds(node, relative_to)?;
            let rect = OverlayRect::new(rect.x, rect.y, rect.width, rect.height);
            (rect.width > 0.0 && rect.height > 0.0 && rect.validate().is_ok()).then_some(rect)
        };
        for (index, scope) in catalog.scopes.iter().enumerate() {
            if scope.blocked_by_ancestor || scope.mode == KeyboardNavigationMode::None {
                continue;
            }
            let mut previous = None;
            for (rank, item) in scope
                .items
                .iter()
                .filter(|item| item.is_active())
                .enumerate()
            {
                let Some((node, bounds)) = item
                    .node
                    .and_then(|node| bounds(node).map(|rect| (node, rect)))
                else {
                    previous = None;
                    continue;
                };
                let next = overlay.badges.len();
                if scope.mode != KeyboardNavigationMode::Once {
                    if let Some(previous) = previous {
                        overlay.links.push([previous, next]);
                    }
                    previous = Some(next);
                }
                overlay.badges.push(FocusBadge {
                    node,
                    group: index + 1,
                    rank: rank + 1,
                    label: item.label.clone(),
                    bounds,
                    group_entry: item.group.is_some(),
                });
            }
        }
        for association in &catalog.labels {
            let Some(target) = association.current_target else {
                continue;
            };
            if let Some((source, destination)) = bounds(association.node).zip(bounds(target)) {
                overlay.labels.push(LabelLink {
                    node: association.node,
                    target,
                    label: association.label.clone(),
                    from: source.center(),
                    to: destination.center(),
                });
            }
        }
        overlay
    }
}

impl DesignerSession {
    #[cfg(feature = "mcp")]
    pub(crate) fn focus_order_overlay(&self, catalog: &FocusOrderCatalog) -> FocusOrderOverlay {
        FocusOrderOverlay::build(self, &self.preview_host().upcast(), catalog)
    }
}

pub(crate) fn render_focus_order(
    context: &mut DrawingContext,
    session: &DesignerSession,
    relative_to: &Ref<Visual>,
    zoom: f64,
) {
    if !zoom.is_finite() || zoom <= 0.0 {
        return;
    }
    let Ok(catalog) = session.focus_order_catalog() else {
        return;
    };
    let overlay = FocusOrderOverlay::build(session, relative_to, &catalog);
    let scale = 1.0 / zoom;
    let foreground: Rc<dyn IBrush> = SolidColorBrush::from_uint32(0xffffffff).into();
    let background: Rc<dyn IBrush> = SolidColorBrush::from_uint32(0xff6148d6).into();
    let line: Rc<dyn IPen> = Pen::from_uint32(0xc06148d6, 1.25 * scale).into();
    let placements: Vec<_> = overlay
        .badges
        .iter()
        .map(|badge| {
            let value = format!("{}.{}", badge.group, badge.rank);
            let width = (12.0 + value.len() as f64 * 6.0) * scale;
            // Put the group's entry on the opposite corner from its first child.
            let x = if badge.group_entry {
                badge.bounds.right() - 3.0 * scale - width
            } else {
                badge.bounds.x + 3.0 * scale
            };
            (
                Rect::new(x, badge.bounds.y + 3.0 * scale, width, 19.0 * scale),
                value,
            )
        })
        .collect();
    for [from, to] in overlay.links {
        context.draw_line(
            &line,
            placements[from].0.center(),
            placements[to].0.center(),
        );
    }
    let label_line: Rc<dyn IPen> = Pen::from_uint32(0xffbd781b, 1.5 * scale).into();
    for label in overlay.labels {
        context.draw_line(
            &label_line,
            Point::new(label.from.x, label.from.y),
            Point::new(label.to.x, label.to.y),
        );
    }
    for (badge, value) in placements {
        context.draw_rectangle(
            Some(&background),
            None,
            badge,
            4.0 * scale,
            4.0 * scale,
            &BoxShadows::default(),
        );
        let text = FormattedText::new(
            &value,
            current_locale(),
            FlowDirection::LeftToRight,
            Typeface::default(),
            10.0 * scale,
            Some(foreground.clone()),
        );
        context.draw_text(
            &text,
            Point::new(badge.x + 6.0 * scale, badge.y + 2.0 * scale),
        );
    }
}
