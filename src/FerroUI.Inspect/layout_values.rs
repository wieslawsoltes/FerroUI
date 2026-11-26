//! Interpret effective layout values using the native parent's actual arrangement rules.
use ferroui_base::layout::{Layoutable, MinMax};
use ferroui_base::{Ref, TypeInfo, Visual};
use ferroui_controls::primitives::UniformGrid;
use ferroui_controls::{
    Border, Canvas, Control, DockPanel, FlexPanel, Grid, Panel, RelativePanel,
    ReversibleStackPanel, StackPanel, WrapPanel,
};

const CANVAS: &[&str] = &["Canvas.Left", "Canvas.Top", "Canvas.Right", "Canvas.Bottom"];
const GRID: &[&str] = &["Grid.Row", "Grid.Column", "Grid.RowSpan", "Grid.ColumnSpan"];
const DOCK: &[&str] = &["DockPanel.Dock"];
const FLEX: &[&str] = &[
    "Flex.AlignSelf",
    "Flex.Order",
    "Flex.Basis",
    "Flex.Grow",
    "Flex.Shrink",
];
const RELATIVE: &[&str] = &[
    "RelativePanel.Above",
    "RelativePanel.Below",
    "RelativePanel.LeftOf",
    "RelativePanel.RightOf",
    "RelativePanel.AlignBottomWith",
    "RelativePanel.AlignTopWith",
    "RelativePanel.AlignLeftWith",
    "RelativePanel.AlignRightWith",
    "RelativePanel.AlignHorizontalCenterWith",
    "RelativePanel.AlignVerticalCenterWith",
    "RelativePanel.AlignBottomWithPanel",
    "RelativePanel.AlignTopWithPanel",
    "RelativePanel.AlignLeftWithPanel",
    "RelativePanel.AlignRightWithPanel",
    "RelativePanel.AlignHorizontalCenterWithPanel",
    "RelativePanel.AlignVerticalCenterWithPanel",
];

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutValueFinding {
    pub property: &'static str,
    pub summary: String,
    pub message: String,
}

/// Report authored values whose effect is determined by an accepted native layout. Application
/// adapters opt into parent rules by passing the authored visual parent of the control.
pub fn check_layout_values(
    visual: &Ref<Visual>,
    parent: Option<&Ref<Visual>>,
    authored: impl Fn(&str) -> bool,
) -> Vec<LayoutValueFinding> {
    let Some(layout) = visual
        .cast::<Layoutable>()
        .filter(|layout| layout.is_arrange_valid())
    else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    let mut check = |property, status: &'static str, message| {
        if authored(property) {
            findings.push(LayoutValueFinding {
                property,
                summary: format!("{property} {status}"),
                message,
            });
        }
    };
    check_size(&layout, &mut check);
    if let Some(parent) = parent.filter(|parent| built_in_parent(parent.get_type())) {
        let owner = parent.get_type();
        for (consumer, properties) in [
            (Canvas::TYPE, CANVAS),
            (Grid::TYPE, GRID),
            (DockPanel::TYPE, DOCK),
            (FlexPanel::TYPE, FLEX),
            (RelativePanel::TYPE, RELATIVE),
        ] {
            if owner == consumer {
                continue;
            }
            for &property in properties.iter().filter(|property| authored(property)) {
                check(
                    property,
                    "inactive",
                    format!(
                        "{property} applies to children of {}. The current parent is {}.",
                        consumer.name(),
                        owner.name()
                    ),
                );
            }
        }
        if owner == Canvas::TYPE {
            check_canvas(visual, &mut check);
        } else if owner == Grid::TYPE {
            if let Some((grid, child)) = parent.cast::<Grid>().zip(visual.cast::<Control>()) {
                check_grid(&grid, &child, &mut check);
            }
        } else if owner == DockPanel::TYPE {
            if let Some((dock, child)) = parent.cast::<DockPanel>().zip(visual.cast::<Control>()) {
                let children = dock.children();
                if dock.last_child_fill()
                    && children
                        .index_of(&child)
                        .is_some_and(|index| index + 1 == children.count())
                {
                    check("DockPanel.Dock", "inactive", "The final DockPanel child fills the remaining space when LastChildFill is enabled.".into());
                }
            }
        }
    }
    findings
}

fn built_in_parent(ty: &TypeInfo) -> bool {
    [
        Panel::TYPE,
        Canvas::TYPE,
        Grid::TYPE,
        Border::TYPE,
        StackPanel::TYPE,
        ReversibleStackPanel::TYPE,
        DockPanel::TYPE,
        WrapPanel::TYPE,
        UniformGrid::TYPE,
        FlexPanel::TYPE,
        RelativePanel::TYPE,
    ]
    .contains(&ty)
}

fn check_canvas(visual: &Visual, report: &mut impl FnMut(&'static str, &'static str, String)) {
    for (property, winning, value, resolved) in [
        (
            "Canvas.Right",
            "Canvas.Left",
            Canvas::get_right(visual),
            Canvas::get_left(visual),
        ),
        (
            "Canvas.Bottom",
            "Canvas.Top",
            Canvas::get_bottom(visual),
            Canvas::get_top(visual),
        ),
    ] {
        if value.is_finite() && resolved.is_finite() {
            report(
                property,
                "overridden",
                format!("{winning} takes precedence over {property}."),
            );
        }
    }
}

fn check_grid(
    grid: &Grid,
    child: &Control,
    report: &mut impl FnMut(&'static str, &'static str, String),
) {
    let Some((rows, columns)) = grid.arranged_track_counts() else {
        return;
    };
    for (property, span_property, count, index, span) in [
        (
            "Grid.Row",
            "Grid.RowSpan",
            rows,
            Grid::get_row(child),
            Grid::get_row_span(child),
        ),
        (
            "Grid.Column",
            "Grid.ColumnSpan",
            columns,
            Grid::get_column(child),
            Grid::get_column_span(child),
        ),
    ] {
        let effective = (index as usize).min(count - 1);
        if effective as i32 != index {
            report(
                property,
                "clamped",
                format!(
                    "{property} {index} is clamped to {effective} with {count} available tracks."
                ),
            );
        }
        let remaining = count - effective;
        if span as usize > remaining {
            report(
                span_property,
                "clamped",
                format!(
                    "{span_property} {span} is clamped to {remaining} starting at track {effective}."
                ),
            );
        }
    }
}

fn check_size(layout: &Layoutable, report: &mut impl FnMut(&'static str, &'static str, String)) {
    let effective = MinMax::new(layout);
    for (property, min_property, max_property, value, min, max, resolved) in [
        (
            "Width",
            "MinWidth",
            "MaxWidth",
            layout.width(),
            layout.min_width(),
            layout.max_width(),
            effective.min_width,
        ),
        (
            "Height",
            "MinHeight",
            "MaxHeight",
            layout.height(),
            layout.min_height(),
            layout.max_height(),
            effective.min_height,
        ),
    ] {
        if min > max {
            report(
                max_property,
                "overridden",
                format!(
                    "{min_property} ({min} px) takes precedence over {max_property} ({max} px)."
                ),
            );
        }
        if value.is_finite() && (value - resolved).abs() > 1e-6 {
            let constraint = if min > max || value < min {
                min_property
            } else {
                max_property
            };
            report(
                property,
                "constrained",
                format!("{constraint} constrains {property} from {value} px to {resolved} px."),
            );
        }
    }
}
