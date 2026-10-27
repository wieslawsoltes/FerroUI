//! Finite, selection-relative measurements used during native pointer manipulation.
use ferroui_base::Rect;
use ferroui_designer_tools::{
    Axis, LayoutGuide, LayoutGuideGeometry, LayoutGuideRole, Point, SnapGuide,
};

pub(super) fn number(value: f64) -> String {
    if value.abs() < 0.05 {
        "0".into()
    } else if (value.round() - value).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

pub(super) fn manipulation_guides(
    moving: Rect,
    peers: &[Rect],
    snap: &[SnapGuide],
    scale: f64,
) -> Vec<LayoutGuide> {
    let peers = peers
        .iter()
        .copied()
        .filter(|rect| super::valid_rect(*rect));
    let mut neighbors: [Option<(f64, Rect)>; 4] = [None; 4];
    let mut result = Vec::new();
    for peer in peers.clone() {
        let overlap_y = moving.bottom().min(peer.bottom()) - moving.y.max(peer.y);
        let overlap_x = moving.right().min(peer.right()) - moving.x.max(peer.x);
        let candidates = [
            (
                overlap_y > 0.0 && peer.right() <= moving.x,
                moving.x - peer.right(),
            ),
            (
                overlap_y > 0.0 && peer.x >= moving.right(),
                peer.x - moving.right(),
            ),
            (
                overlap_x > 0.0 && peer.bottom() <= moving.y,
                moving.y - peer.bottom(),
            ),
            (
                overlap_x > 0.0 && peer.y >= moving.bottom(),
                peer.y - moving.bottom(),
            ),
        ];
        for (index, (eligible, gap)) in candidates.into_iter().enumerate() {
            if eligible && neighbors[index].is_none_or(|(best, _)| gap < best) {
                neighbors[index] = Some((gap, peer));
            }
        }
    }
    for guide in snap {
        if !guide.position.is_finite() {
            continue;
        }
        let pad = 6.0 * scale;
        let mut start = match guide.axis {
            Axis::Horizontal => moving.y,
            Axis::Vertical => moving.x,
        };
        let mut end = match guide.axis {
            Axis::Horizontal => moving.bottom(),
            Axis::Vertical => moving.right(),
        };
        // A snap guide should connect the moving bounds to the nearest aligned
        // object, rather than stretching across unrelated controls/the artboard.
        let nearest = peers
            .clone()
            .filter(|peer| {
                let positions = match guide.axis {
                    Axis::Horizontal => [peer.x, peer.center().x, peer.right()],
                    Axis::Vertical => [peer.y, peer.center().y, peer.bottom()],
                };
                positions
                    .iter()
                    .any(|value| (value - guide.position).abs() <= 0.001)
            })
            .min_by(|a, b| {
                let distance = |peer: Rect| match guide.axis {
                    Axis::Horizontal => (peer.center().y - moving.center().y).abs(),
                    Axis::Vertical => (peer.center().x - moving.center().x).abs(),
                };
                distance(*a).total_cmp(&distance(*b))
            });
        if let Some(peer) = nearest {
            match guide.axis {
                Axis::Horizontal => {
                    start = start.min(peer.y);
                    end = end.max(peer.bottom());
                }
                Axis::Vertical => {
                    start = start.min(peer.x);
                    end = end.max(peer.right());
                }
            }
        }
        let (a, b) = match guide.axis {
            Axis::Horizontal => (
                Point::new(guide.position, start - pad),
                Point::new(guide.position, end + pad),
            ),
            Axis::Vertical => (
                Point::new(start - pad, guide.position),
                Point::new(end + pad, guide.position),
            ),
        };
        result.push(line(LayoutGuideRole::Relation, a, b));
    }
    for (index, neighbor) in neighbors.iter().enumerate() {
        let Some((gap, peer)) = *neighbor else {
            continue;
        };
        if gap <= 0.001 {
            continue;
        }
        let equal = neighbors[index ^ 1]
            .is_some_and(|(opposite, _)| (opposite - gap).abs() <= 0.25 * scale);
        let role = if equal {
            LayoutGuideRole::Spacing
        } else {
            LayoutGuideRole::Measurement
        };
        let (a, b, label) = if index < 2 {
            let y = (moving.y.max(peer.y) + moving.bottom().min(peer.bottom())) * 0.5;
            let (x1, x2) = if index == 0 {
                (peer.right(), moving.x)
            } else {
                (moving.right(), peer.x)
            };
            (
                Point::new(x1, y),
                Point::new(x2, y),
                Point::new((x1 + x2) * 0.5, y - 12.0 * scale),
            )
        } else {
            let x = (moving.x.max(peer.x) + moving.right().min(peer.right())) * 0.5;
            let (y1, y2) = if index == 2 {
                (peer.bottom(), moving.y)
            } else {
                (moving.bottom(), peer.y)
            };
            (
                Point::new(x, y1),
                Point::new(x, y2),
                Point::new(x + 26.0 * scale, (y1 + y2) * 0.5),
            )
        };
        result.push(line(role, a, b));
        result.push(LayoutGuide {
            role,
            geometry: LayoutGuideGeometry::Label {
                position: label,
                text: if equal {
                    format!("{} equal", number(gap))
                } else {
                    number(gap)
                },
            },
        });
    }
    result
}

fn line(role: LayoutGuideRole, start: Point, end: Point) -> LayoutGuide {
    LayoutGuide {
        role,
        geometry: LayoutGuideGeometry::Line { start, end },
    }
}
