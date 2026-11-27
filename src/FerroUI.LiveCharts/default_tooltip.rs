//! The default tooltip of the charts, as the pinned `SKDefaultTooltip.GetLayout`, `AlignWedge` and
//! `PopUpGeometry`: the header of the first point over a table of a miniature, the series name
//! and the value of each point, in a rounded box whose wedge points at the found points. The
//! primitives are ordinary scene geometry, so the tooltip draws on the GPU path of the chart.

use crate::axis::format_number;
use crate::label_lines::LabelLines;
use crate::scene::{
    Geometry, HitRegion, LegendItem, PaintStyle, PathCommand, Primitive, TextAnchor,
};
use crate::series::DATA_LABELS_MAX_WIDTH;
use crate::tooltips::PopUpPlacement;
use ferroui_base::media::Color;
use ferroui_base::{Point, Rect, Size};

/// The native text size of a tooltip.
const TEXT_SIZE: f64 = 12.0;
/// `Px` and `Py` of `SKDefaultTooltip`.
const PADDING_X: f64 = 8.0;
const PADDING_Y: f64 = 4.0;
/// `Wedge` and `WedgeThickness` of `SKDefaultTooltip`.
const WEDGE: f64 = 10.0;
const WEDGE_THICKNESS: f64 = 3.0;
/// `BorderRadius` of `PopUpGeometry`, the side of the oval of a corner: the radius is half of it.
const BORDER_RADIUS: f64 = 10.0;
/// The bottom padding of the header.
const HEADER_GAP: f64 = 8.0;
/// `MiniatureShapeSize` of a series.
const MINIATURE: f64 = 12.0;
/// The padding of a name at each side, and of a value at each side and above and below it.
const NAME_PADDING_X: f64 = 10.0;
const VALUE_PADDING_X: f64 = 8.0;
const VALUE_PADDING_Y: f64 = 2.0;
/// The control points of a quarter circle of radius one.
const QUARTER_CIRCLE: f64 = 0.552_284_749_830_793_6;

#[derive(Clone, Debug, PartialEq)]
struct Row {
    miniature: Option<Color>,
    name: Option<LabelLines>,
    value: LabelLines,
    height: f64,
}

/// The measured content of a tooltip. Its size depends on the side the wedge is at.
#[derive(Clone, Debug, PartialEq)]
pub struct DefaultTooltip {
    header: Option<LabelLines>,
    rows: Vec<Row>,
    miniature_width: f64,
    name_width: f64,
    value_width: f64,
    content: Size,
}

impl DefaultTooltip {
    /// Lays out the found `points`. With `miniatures`, each row starts with the colour of its
    /// series in `legend`; a point whose label has no parts is a value cell of its whole label.
    pub fn new(
        points: &[HitRegion],
        legend: &[LegendItem],
        miniatures: bool,
        measure: &mut impl FnMut(&str, f64) -> Size,
    ) -> Self {
        // A label breaks at `MaxTooltipsAndLegendsLabelsWidth` less its padding.
        let mut wrap = |text: &str, padding: f64| {
            LabelLines::wrap(
                &mut *measure,
                text,
                TEXT_SIZE,
                DATA_LABELS_MAX_WIDTH - padding,
            )
        };
        let header = points
            .first()
            .map(|point| point.label_parts.split(&point.label).1)
            .filter(|header| !header.is_empty())
            .map(|header| wrap(header, 0.0));
        let mut rows = Vec::with_capacity(points.len());
        let (mut miniature_width, mut name_width, mut value_width) = (0.0_f64, 0.0_f64, 0.0_f64);
        let mut table_height = 0.0_f64;
        for point in points {
            let (name, _, value) = point.label_parts.split(&point.label);
            let miniature = miniatures
                .then(|| legend.iter().find(|item| item.series == point.id.series))
                .flatten()
                .map(|item| item.color);
            let name = (!name.is_empty()).then(|| wrap(name, NAME_PADDING_X * 2.0));
            let value = if value.is_empty() && name.is_none() {
                wrap(&format_number(point.value), VALUE_PADDING_X * 2.0)
            } else {
                wrap(value, VALUE_PADDING_X * 2.0)
            };
            let name_size = name.as_ref().map_or(Size::default(), |name| name.size);
            let mut height = value.size.height + VALUE_PADDING_Y * 2.0;
            height = height.max(name_size.height);
            if miniature.is_some() {
                miniature_width = MINIATURE;
                height = height.max(MINIATURE);
            }
            if name.is_some() {
                name_width = name_width.max(name_size.width + NAME_PADDING_X * 2.0);
            }
            value_width = value_width.max(value.size.width + VALUE_PADDING_X * 2.0);
            table_height += height;
            rows.push(Row {
                miniature,
                name,
                value,
                height,
            });
        }
        let table_width = miniature_width + name_width + value_width;
        let header_size = header
            .as_ref()
            .map_or(Size::default(), |header| header.size);
        let header_height = if header.is_some() {
            header_size.height + HEADER_GAP
        } else {
            0.0
        };
        Self {
            header,
            rows,
            miniature_width,
            name_width,
            value_width,
            content: Size::new(
                table_width.max(header_size.width),
                header_height + table_height,
            ),
        }
    }

    /// The left, top, right and bottom padding of the content (`AlignWedge`).
    fn padding(placement: PopUpPlacement) -> (f64, f64, f64, f64) {
        let (x, y) = (PADDING_X, PADDING_Y);
        match placement {
            PopUpPlacement::Top => (x, y, x, y + WEDGE),
            PopUpPlacement::Bottom => (x, y + WEDGE, x, y),
            PopUpPlacement::Left => (x, y, x + WEDGE, y),
            PopUpPlacement::Right => (x + WEDGE, y, x, y),
        }
    }

    /// The size of the tooltip with its wedge at `placement`, at most the size of the control.
    pub fn size(&self, placement: PopUpPlacement, control: Size) -> Size {
        let (left, top, right, bottom) = Self::padding(placement);
        Size::new(
            (self.content.width + left + right).min(control.width.max(0.0)),
            (self.content.height + top + bottom).min(control.height.max(0.0)),
        )
    }

    /// The box, the miniatures and the texts of the tooltip with its top left corner at `origin`.
    /// The content is clipped to the box of a tooltip the control is too small for.
    pub fn primitives(
        &self,
        origin: Point,
        placement: PopUpPlacement,
        control: Size,
        background: PaintStyle,
        text: Color,
    ) -> Vec<Primitive> {
        let size = self.size(placement, control);
        let bounds = Rect::new(origin.x, origin.y, size.width, size.height);
        let body = pop_up_body(bounds, placement);
        let mut primitives = vec![Primitive::new(
            Geometry::Path(pop_up_path(bounds, placement)),
            background,
        )];
        primitives.reserve(self.rows.len() * 3);
        let (left, top, _, _) = Self::padding(placement);
        let (x, mut y) = (origin.x + left, origin.y + top);
        let lines = |block: &LabelLines, at: Point, primitives: &mut Vec<Primitive>| {
            for (i, line) in block.lines.iter().enumerate() {
                primitives.push(
                    Primitive::new(
                        Geometry::Text {
                            origin: Point::new(at.x, at.y + i as f64 * block.line_height),
                            text: line.clone(),
                            size: TEXT_SIZE,
                            rotation: 0.0,
                            anchor: TextAnchor::Start,
                        },
                        PaintStyle::fill(text),
                    )
                    .clipped(body),
                );
            }
        };
        if let Some(header) = &self.header {
            let at = Point::new(x + (self.content.width - header.size.width) * 0.5, y);
            lines(header, at, &mut primitives);
            y += header.size.height + HEADER_GAP;
        }
        let table_width = self.miniature_width + self.name_width + self.value_width;
        let table_x = x + (self.content.width - table_width) * 0.5;
        for row in &self.rows {
            let middle = |height: f64| y + (row.height - height) * 0.5;
            if let Some(color) = row.miniature {
                let rect = Rect::new(
                    table_x + (self.miniature_width - MINIATURE) * 0.5,
                    middle(MINIATURE),
                    MINIATURE,
                    MINIATURE,
                );
                primitives.push(
                    Primitive::new(
                        Geometry::Rectangle { rect, radius: 2.0 },
                        PaintStyle::fill(color),
                    )
                    .clipped(body),
                );
            }
            if let Some(name) = &row.name {
                let at = Point::new(
                    table_x + self.miniature_width + NAME_PADDING_X,
                    middle(name.size.height),
                );
                lines(name, at, &mut primitives);
            }
            let at = Point::new(
                table_x + table_width - VALUE_PADDING_X - row.value.size.width,
                middle(row.value.size.height),
            );
            lines(&row.value, at, &mut primitives);
            y += row.height;
        }
        primitives
    }
}

/// The rounded box of a pop-up of `bounds`, without its wedge.
fn pop_up_body(bounds: Rect, placement: PopUpPlacement) -> Rect {
    let (x, y, width, height) = (bounds.x, bounds.y, bounds.width, bounds.height);
    match placement {
        PopUpPlacement::Top => Rect::new(x, y, width, (height - WEDGE).max(0.0)),
        PopUpPlacement::Bottom => Rect::new(x, y + WEDGE, width, (height - WEDGE).max(0.0)),
        PopUpPlacement::Left => Rect::new(x, y, (width - WEDGE).max(0.0), height),
        PopUpPlacement::Right => Rect::new(x + WEDGE, y, (width - WEDGE).max(0.0), height),
    }
}

/// The outline of a pop-up (`PopUpGeometry.Draw`): a rounded box with a wedge in the middle of the
/// side toward its points, so the tip is on the edge of `bounds`.
fn pop_up_path(bounds: Rect, placement: PopUpPlacement) -> Vec<PathCommand> {
    let body = pop_up_body(bounds, placement);
    let (x, y, w, h) = (body.x, body.y, body.width, body.height);
    let r = (BORDER_RADIUS * 0.5).min(w * 0.5).min(h * 0.5);
    let half = WEDGE * WEDGE_THICKNESS * 0.5;
    let p = Point::new;
    let corner = |start: Point, corner: Point, end: Point| PathCommand::CubicTo {
        control1: p(
            start.x + (corner.x - start.x) * QUARTER_CIRCLE,
            start.y + (corner.y - start.y) * QUARTER_CIRCLE,
        ),
        control2: p(
            end.x + (corner.x - end.x) * QUARTER_CIRCLE,
            end.y + (corner.y - end.y) * QUARTER_CIRCLE,
        ),
        end,
    };
    let mut path = vec![PathCommand::MoveTo(p(x + r, y))];
    if placement == PopUpPlacement::Bottom {
        path.push(PathCommand::LineTo(p(x + w * 0.5 - half, y)));
        path.push(PathCommand::LineTo(p(x + w * 0.5, y - WEDGE)));
        path.push(PathCommand::LineTo(p(x + w * 0.5 + half, y)));
    }
    path.push(PathCommand::LineTo(p(x + w - r, y)));
    path.push(corner(p(x + w - r, y), p(x + w, y), p(x + w, y + r)));
    if placement == PopUpPlacement::Left {
        path.push(PathCommand::LineTo(p(x + w, y + h * 0.5 - half)));
        path.push(PathCommand::LineTo(p(x + w + WEDGE, y + h * 0.5)));
        path.push(PathCommand::LineTo(p(x + w, y + h * 0.5 + half)));
    }
    path.push(PathCommand::LineTo(p(x + w, y + h - r)));
    path.push(corner(
        p(x + w, y + h - r),
        p(x + w, y + h),
        p(x + w - r, y + h),
    ));
    if placement == PopUpPlacement::Top {
        path.push(PathCommand::LineTo(p(x + w * 0.5 + half, y + h)));
        path.push(PathCommand::LineTo(p(x + w * 0.5, y + h + WEDGE)));
        path.push(PathCommand::LineTo(p(x + w * 0.5 - half, y + h)));
    }
    path.push(PathCommand::LineTo(p(x + r, y + h)));
    path.push(corner(p(x + r, y + h), p(x, y + h), p(x, y + h - r)));
    if placement == PopUpPlacement::Right {
        path.push(PathCommand::LineTo(p(x, y + h * 0.5 + half)));
        path.push(PathCommand::LineTo(p(x - WEDGE, y + h * 0.5)));
        path.push(PathCommand::LineTo(p(x, y + h * 0.5 - half)));
    }
    path.push(PathCommand::LineTo(p(x, y + r)));
    path.push(corner(p(x, y + r), p(x, y), p(x + r, y)));
    path.push(PathCommand::Close);
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{HitShape, PointId, TooltipLabelParts};

    fn measure(text: &str, _size: f64) -> Size {
        Size::new(text.chars().count() as f64 * 6.0, 14.0)
    }

    fn point(series: usize, name: &str, header: &str, value: &str) -> HitRegion {
        let label = [name, header, value]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(": ");
        HitRegion {
            id: PointId { series, point: 0 },
            shape: HitShape::Rectangle(Rect::new(0.0, 0.0, 1.0, 1.0)),
            center: Point::new(0.0, 0.0),
            value: 1.0,
            label,
            label_parts: TooltipLabelParts::of(name, header),
        }
    }

    fn legend(series: usize, color: Color) -> LegendItem {
        LegendItem {
            series,
            name: String::new(),
            color,
        }
    }

    fn texts(primitives: &[Primitive]) -> Vec<(String, Point)> {
        primitives
            .iter()
            .filter_map(|primitive| match &primitive.geometry {
                Geometry::Text { text, origin, .. } => Some((text.clone(), *origin)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn label_parts_split_names_that_hold_the_separator() {
        let parts = TooltipLabelParts::of("a: b", "c");
        assert_eq!(parts.split("a: b: c: 5"), ("a: b", "c", "5"));
        let parts = TooltipLabelParts::of("", "c");
        assert_eq!(parts.split("c: 5"), ("", "c", "5"));
        let parts = TooltipLabelParts::of("a", "");
        assert_eq!(parts.split("a: 5"), ("a", "", "5"));
        assert_eq!(parts.split("a"), ("a", "", ""));
        assert_eq!(TooltipLabelParts::default().split("x: 5"), ("", "", "x: 5"));
        // A label changed after its parts were recorded is its value, never a panic.
        assert_eq!(
            TooltipLabelParts::of("long name", "").split("é"),
            ("", "", "é")
        );
    }

    /// The table of `SKDefaultTooltip.GetLayout`: the header of the first point in the middle,
    /// then a miniature, a name at the start of its column and a value at the end of its own.
    #[test]
    fn table_has_a_header_and_a_row_of_miniature_name_and_value_for_each_point() {
        let (red, blue) = (Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255));
        let points = [point(0, "Alpha", "Mon", "1"), point(1, "B", "Tue", "1000")];
        let items = [legend(0, red), legend(1, blue)];
        let tooltip = DefaultTooltip::new(&points, &items, true, &mut measure);
        // Columns: 12, 5 * 6 + 20 = 50, 4 * 6 + 16 = 40. Rows: 14 + 4 each, under 14 + 8.
        assert_eq!(tooltip.content, Size::new(102.0, 58.0));
        let control = Size::new(500.0, 500.0);
        assert_eq!(
            tooltip.size(PopUpPlacement::Top, control),
            Size::new(118.0, 76.0)
        );
        let drawn = tooltip.primitives(
            Point::new(10.0, 20.0),
            PopUpPlacement::Top,
            control,
            PaintStyle::fill(Color::from_rgb(1, 1, 1)),
            Color::from_rgb(2, 2, 2),
        );
        let texts = texts(&drawn);
        let names: Vec<_> = texts.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(names, ["Mon", "Alpha", "1", "B", "1000"]);
        // The header is centred over the content, which starts at 10 + 8.
        assert_eq!(texts[0].1, Point::new(18.0 + (102.0 - 18.0) * 0.5, 24.0));
        // The names start after the miniature and their padding.
        assert_eq!(texts[1].1.x, 18.0 + 12.0 + 10.0);
        assert_eq!(texts[3].1.x, 18.0 + 12.0 + 10.0);
        // The values end at the end of the column, less their padding.
        assert_eq!(texts[2].1.x + 6.0, 18.0 + 102.0 - 8.0);
        assert_eq!(texts[4].1.x + 24.0, 18.0 + 102.0 - 8.0);
        let miniatures: Vec<_> = drawn
            .iter()
            .filter_map(|primitive| match primitive.geometry {
                Geometry::Rectangle { rect, .. } => Some((rect, primitive.style.fill)),
                _ => None,
            })
            .collect();
        assert_eq!(miniatures.len(), 2);
        assert_eq!(
            miniatures[0].0,
            Rect::new(18.0, 24.0 + 22.0 + 3.0, 12.0, 12.0)
        );
        assert_eq!(miniatures[1].1, Some(blue));
    }

    #[test]
    fn point_without_parts_is_a_value_of_its_lines_and_no_miniature() {
        let land = HitRegion {
            label: "Brazil\nPopulation: 213".to_owned(),
            label_parts: TooltipLabelParts::default(),
            ..point(0, "", "", "x")
        };
        let items = [legend(0, Color::from_rgb(255, 0, 0))];
        let tooltip = DefaultTooltip::new(&[land], &items, false, &mut measure);
        let drawn = tooltip.primitives(
            Point::new(0.0, 0.0),
            PopUpPlacement::Top,
            Size::new(500.0, 500.0),
            PaintStyle::default(),
            Color::from_rgb(0, 0, 0),
        );
        let lines: Vec<_> = texts(&drawn).into_iter().map(|(text, _)| text).collect();
        assert_eq!(lines, ["Brazil", "Population: 213"]);
        assert!(
            !drawn
                .iter()
                .any(|primitive| matches!(primitive.geometry, Geometry::Rectangle { .. }))
        );
    }

    #[test]
    fn long_labels_break_at_the_largest_width() {
        let long = "word ".repeat(20);
        let tooltip =
            DefaultTooltip::new(&[point(0, "", "", long.trim())], &[], false, &mut measure);
        assert!(tooltip.value_width <= DATA_LABELS_MAX_WIDTH);
        assert!(tooltip.rows[0].value.lines.len() > 1);
    }

    /// The wedge adds to the side it is at, and its tip reaches the edge of the bounds there.
    #[test]
    fn wedge_points_out_of_the_side_of_its_placement() {
        let tooltip = DefaultTooltip::new(&[point(0, "", "", "42")], &[], false, &mut measure);
        let control = Size::new(500.0, 500.0);
        let content = tooltip.content;
        let vertical = Size::new(content.width + 16.0, content.height + 18.0);
        let horizontal = Size::new(content.width + 26.0, content.height + 8.0);
        for (placement, size, tip) in [
            (PopUpPlacement::Top, vertical, (0.5, 1.0)),
            (PopUpPlacement::Bottom, vertical, (0.5, 0.0)),
            (PopUpPlacement::Left, horizontal, (1.0, 0.5)),
            (PopUpPlacement::Right, horizontal, (0.0, 0.5)),
        ] {
            assert_eq!(tooltip.size(placement, control), size);
            let bounds = Rect::new(30.0, 40.0, size.width, size.height);
            let path = pop_up_path(bounds, placement);
            let expected = Point::new(
                bounds.x + bounds.width * tip.0,
                bounds.y + bounds.height * tip.1,
            );
            assert!(
                path.iter().any(|command| matches!(
                    command,
                    PathCommand::LineTo(at)
                        if (at.x - expected.x).abs() < 1e-9 && (at.y - expected.y).abs() < 1e-9
                )),
                "{placement:?}"
            );
            assert!(matches!(path.last(), Some(PathCommand::Close)));
        }
    }

    #[test]
    fn tooltip_is_never_larger_than_the_control() {
        let tooltip = DefaultTooltip::new(&[point(0, "Name", "", "42")], &[], false, &mut measure);
        let control = Size::new(20.0, 10.0);
        assert_eq!(tooltip.size(PopUpPlacement::Top, control), control);
        assert_eq!(
            tooltip.size(PopUpPlacement::Left, Size::new(0.0, 0.0)),
            Size::new(0.0, 0.0)
        );
    }
}
