//! Read-only navigation ranks, scoped to the native group owning each TabIndex.
use super::*;
use ferroui_base::media::{
    BoxShadows, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, Pen, SolidColorBrush,
    Typeface,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Point, Rect};
use std::rc::Rc;

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
    let scale = 1.0 / zoom;
    let foreground: Rc<dyn IBrush> = SolidColorBrush::from_uint32(0xffffffff).into();
    let background: Rc<dyn IBrush> = SolidColorBrush::from_uint32(0xff6148d6).into();
    let line: Rc<dyn IPen> = Pen::from_uint32(0xc06148d6, 1.25 * scale).into();
    let bounds = |node| {
        let state = session.layer_state(node);
        if state.is_hidden() || state.outside_isolation {
            return None;
        }
        session.preview.bounds(node, relative_to).filter(|rect| {
            [rect.x, rect.y, rect.width, rect.height]
                .iter()
                .all(|value| value.is_finite())
                && rect.width > 0.0
                && rect.height > 0.0
        })
    };
    for (scope_index, scope) in catalog.scopes.iter().enumerate() {
        if scope.blocked_by_ancestor || scope.mode == KeyboardNavigationMode::None {
            continue;
        }
        let mut previous = None;
        for (index, item) in scope
            .items
            .iter()
            .filter(|item| item.is_active())
            .enumerate()
        {
            let Some(node) = item.node else {
                previous = None;
                continue;
            };
            let Some(rect) = bounds(node) else {
                previous = None;
                continue;
            };
            let position = Point::new(rect.x + 3.0 * scale, rect.y + 3.0 * scale);
            // The first number is the native navigation-group identity in this
            // view. A Once group has local ranks but no fictitious sequential link.
            let value = format!("{}.{}", scope_index + 1, index + 1);
            let width = (12.0 + value.len() as f64 * 6.0) * scale;
            let badge = Rect::new(position.x, position.y, width, 19.0 * scale);
            if scope.mode != KeyboardNavigationMode::Once {
                if let Some(previous) = previous {
                    context.draw_line(&line, previous, badge.center());
                }
                previous = Some(badge.center());
            }
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
                CultureInfo::current_culture(),
                FlowDirection::LeftToRight,
                Typeface::default(),
                10.0 * scale,
                Some(foreground.clone()),
            );
            context.draw_text(
                &text,
                Point::new(position.x + 6.0 * scale, position.y + 2.0 * scale),
            );
        }
    }
    let label_line: Rc<dyn IPen> = Pen::from_uint32(0xffbd781b, 1.5 * scale).into();
    for association in catalog.labels {
        if let Some((source, target)) =
            bounds(association.node).zip(association.current_target.and_then(bounds))
        {
            context.draw_line(&label_line, source.center(), target.center());
        }
    }
}
