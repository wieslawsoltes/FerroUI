//! Track sizing shares the bounded occupancy placement used by pinned grids.
//! Spans connect intrinsic tracks into stable weighted groups: arranging a
//! Fill child cannot repeatedly increase overlapping Hug spans on each pass.
use super::*;

fn track_axis(
    tracks: &[GridTrack],
    items: &[(usize, usize, f64)],
    extent: f64,
    gap: f64,
    hug: bool,
) -> Result<Vec<f64>, String> {
    if tracks.is_empty() {
        return Ok(Vec::new());
    }
    let count = tracks.len();
    let mut joins = vec![0i64; count + 1];
    for &(start, span, _) in items {
        if start.checked_add(span).is_none_or(|end| end > count) || span == 0 {
            return Err("A grid span exceeds its track axis".into());
        }
        if span > 1 {
            joins[start] += 1;
            joins[start + span - 1] -= 1;
        }
    }
    let mut groups = Vec::with_capacity(count);
    let (mut active, mut group) = (0i64, 0usize);
    for joined in joins.iter().take(count) {
        groups.push(group);
        active += *joined;
        if active == 0 {
            group += 1;
        }
    }
    let weight = |track: GridTrack| match track {
        GridTrack::Fixed(_) => 0.0,
        GridTrack::Content => 1.0,
        GridTrack::Fraction(value) => value,
    };
    let mut scale = vec![0.0f64; group + 1];
    for (track, group) in tracks.iter().zip(&groups) {
        scale[*group] = scale[*group].max(weight(*track));
    }
    let mut fixed_prefix = Vec::with_capacity(count + 1);
    let mut weight_prefix = Vec::with_capacity(count + 1);
    fixed_prefix.push(0.0);
    weight_prefix.push(0.0);
    for (track, group) in tracks.iter().zip(&groups) {
        let fixed = if let GridTrack::Fixed(value) = track {
            *value
        } else {
            0.0
        };
        let relative = if scale[*group] > 0.0 {
            weight(*track) / scale[*group]
        } else {
            0.0
        };
        fixed_prefix.push(fixed_prefix.last().copied().unwrap() + fixed);
        weight_prefix.push(weight_prefix.last().copied().unwrap() + relative);
    }
    let mut units = vec![0.0f64; group + 1];
    for &(start, span, demand) in items {
        let fixed = fixed_prefix[start + span] - fixed_prefix[start];
        let weight = weight_prefix[start + span] - weight_prefix[start];
        if weight > 0.0 {
            let required = ((demand - fixed - gap * (span - 1) as f64) / weight).max(0.0);
            units[groups[start]] = units[groups[start]].max(required);
        }
    }
    let mut sizes: Vec<_> = tracks
        .iter()
        .zip(&groups)
        .map(|(track, group)| match track {
            GridTrack::Fixed(value) => *value,
            _ if scale[*group] > 0.0 => units[*group] * (weight(*track) / scale[*group]),
            _ => 0.0,
        })
        .collect();
    if !hug {
        let fraction_scale = tracks
            .iter()
            .filter_map(|track| {
                if let GridTrack::Fraction(value) = track {
                    Some(*value)
                } else {
                    None
                }
            })
            .fold(0.0, f64::max);
        if fraction_scale > 0.0 {
            let fixed: f64 = tracks
                .iter()
                .zip(&sizes)
                .filter(|(track, _)| !matches!(track, GridTrack::Fraction(_)))
                .map(|(_, size)| *size)
                .sum();
            let available = (extent - gap * count.saturating_sub(1) as f64 - fixed).max(0.0);
            let total: f64 = tracks
                .iter()
                .filter_map(|track| {
                    if let GridTrack::Fraction(value) = track {
                        Some(*value / fraction_scale)
                    } else {
                        None
                    }
                })
                .sum();
            for (track, size) in tracks.iter().zip(&mut sizes) {
                if let GridTrack::Fraction(value) = track {
                    *size = available * (*value / fraction_scale) / total;
                }
            }
        }
    }
    if fixed_prefix
        .iter()
        .chain(&sizes)
        .any(|value| !value.is_finite())
        || !sizes.iter().sum::<f64>().is_finite()
    {
        return Err("Grid track sizing exceeds finite geometry bounds".into());
    }
    Ok(sizes)
}

pub(super) fn arrange_explicit_grid(
    children: &mut [LayoutItem],
    layout: &AutoLayout,
    width: &mut f64,
    height: &mut f64,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let cells = grid_cells(children, layout.columns)?;
    let columns = layout.columns;
    let rows = cells
        .iter()
        .map(|cell| cell.row + cell.row_span)
        .max()
        .unwrap_or(0)
        .max(layout.row_tracks.len());
    if rows > MAX_GRID_ROWS
        || rows
            .checked_mul(columns)
            .is_none_or(|cells| cells > MAX_GRID_CELLS)
    {
        return Err("Explicit grid tracks exceed the supported cell bounds".into());
    }
    let column_tracks: Vec<_> = (0..columns)
        .map(|index| {
            layout
                .column_tracks
                .get(index)
                .copied()
                .unwrap_or(GridTrack::Fraction(1.0))
        })
        .collect();
    let implicit_row = if layout.row_tracks.is_empty()
        && layout.height != SizingMode::Hug
        && layout.cross_align == LayoutAlign::Stretch
    {
        GridTrack::Fraction(1.0)
    } else {
        GridTrack::Content
    };
    let row_tracks: Vec<_> = (0..rows)
        .map(|index| {
            layout
                .row_tracks
                .get(index)
                .copied()
                .unwrap_or(implicit_row)
        })
        .collect();
    for child in children.iter_mut() {
        child.width = clamp_size(
            child.width,
            child.layout_child.min_width,
            child.layout_child.max_width,
        );
        child.height = clamp_size(
            child.height,
            child.layout_child.min_height,
            child.layout_child.max_height,
        );
    }
    let p = layout.padding;
    let widths = track_axis(
        &column_tracks,
        &children
            .iter()
            .zip(&cells)
            .map(|(child, cell)| (cell.column, cell.column_span, child.width))
            .collect::<Vec<_>>(),
        (*width - p.left - p.right).max(0.0),
        layout.gap,
        layout.width == SizingMode::Hug,
    )?;
    let heights = track_axis(
        &row_tracks,
        &children
            .iter()
            .zip(&cells)
            .map(|(child, cell)| (cell.row, cell.row_span, child.height))
            .collect::<Vec<_>>(),
        (*height - p.top - p.bottom).max(0.0),
        layout.cross_gap,
        layout.height == SizingMode::Hug,
    )?;
    let mut column_x = vec![p.left];
    for width in &widths {
        column_x.push(column_x.last().copied().unwrap() + width + layout.gap);
    }
    let mut row_y = vec![p.top];
    for height in &heights {
        row_y.push(row_y.last().copied().unwrap() + height + layout.cross_gap);
    }
    let natural_width = p.left
        + p.right
        + widths.iter().sum::<f64>()
        + layout.gap * columns.saturating_sub(1) as f64;
    let natural_height = p.top
        + p.bottom
        + heights.iter().sum::<f64>()
        + layout.cross_gap * rows.saturating_sub(1) as f64;
    if !natural_width.is_finite()
        || !natural_height.is_finite()
        || column_x
            .iter()
            .chain(&row_y)
            .any(|value| !value.is_finite())
    {
        return Err("Grid track positions exceed finite geometry bounds".into());
    }
    if layout.width == SizingMode::Hug {
        *width = natural_width;
    } else if natural_width > *width + GEOMETRY_EPSILON {
        warnings.push("Fixed or content column tracks exceed the frame width".into());
    }
    if layout.height == SizingMode::Hug {
        *height = natural_height;
    } else if natural_height > *height + GEOMETRY_EPSILON {
        warnings.push("Fixed or content row tracks exceed the frame height".into());
    }
    for (child, cell) in children.iter_mut().zip(cells) {
        let available_width =
            (column_x[cell.column + cell.column_span] - column_x[cell.column] - layout.gap)
                .max(0.0);
        let available_height =
            (row_y[cell.row + cell.row_span] - row_y[cell.row] - layout.cross_gap).max(0.0);
        if child.layout_child.width == SizingMode::Fill
            || layout.cross_align == LayoutAlign::Stretch
        {
            child.width = clamp_size(
                available_width,
                child.layout_child.min_width,
                child.layout_child.max_width,
            );
        }
        if child.layout_child.height == SizingMode::Fill
            || layout.cross_align == LayoutAlign::Stretch
        {
            child.height = clamp_size(
                available_height,
                child.layout_child.min_height,
                child.layout_child.max_height,
            );
        }
        if child.width > available_width + GEOMETRY_EPSILON
            || child.height > available_height + GEOMETRY_EPSILON
        {
            warnings.push(format!(
                "Node {}: fixed or minimum size exceeds its allocated grid span",
                child.id
            ));
        }
        child.x = column_x[cell.column]
            + alignment_offset(layout.main_align, available_width - child.width);
        child.y =
            row_y[cell.row] + alignment_offset(layout.cross_align, available_height - child.height);
    }
    Ok(())
}
