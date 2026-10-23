use crate::*;
use std::collections::BTreeSet;
#[path = "grid_track_layout.rs"]
mod grid_track_layout;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutResult {
    pub changed: Vec<NodeId>,
    pub warnings: Vec<String>,
    /// Invalid settings or exhausted placement bounds, separate from advisory
    /// Hug/Fill warnings. Authoring transactions reject these atomically.
    pub errors: Vec<String>,
}
/// Layout reads only geometry and sizing rules. Retaining an entire `Node`
/// here would copy text, vector paths, images, and paint stacks on every pass.
#[derive(Clone, Copy)]
struct LayoutItem {
    id: NodeId,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    layout_child: LayoutChild,
}
fn clamp_size(v: f64, min: f64, max: Option<f64>) -> f64 {
    v.max(min.max(0.0))
        .min(max.unwrap_or(f64::MAX).max(min.max(0.0)))
}
fn flex_sizes(total: f64, items: &[(f64, f64, Option<f64>)]) -> Vec<f64> {
    let mut sizes = vec![0.0; items.len()];
    let mut active: BTreeSet<usize> = (0..items.len()).collect();
    let mut remaining = total;
    while !active.is_empty() {
        let weight: f64 = active.iter().map(|i| items[*i].0.max(0.001)).sum();
        let mut saturated = Vec::new();
        for i in &active {
            let (w, min, max) = items[*i];
            let share = remaining.max(0.0) * w.max(0.001) / weight;
            let clamped = clamp_size(share, min, max);
            if (share - clamped).abs() > 1e-7 {
                saturated.push((*i, clamped))
            }
        }
        if saturated.is_empty() {
            for i in active {
                sizes[i] = remaining.max(0.0) * items[i].0.max(0.001) / weight
            }
            break;
        }
        for (i, size) in saturated {
            sizes[i] = size;
            remaining -= size;
            active.remove(&i);
        }
    }
    sizes
}
fn alignment_offset(a: LayoutAlign, extra: f64) -> f64 {
    match a {
        LayoutAlign::Center => extra.max(0.0) / 2.0,
        LayoutAlign::End => extra.max(0.0),
        _ => 0.0,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct GridCell { row: usize, column: usize, row_span: usize, column_span: usize }

fn has_grid_placement(children: &[LayoutItem]) -> bool {
    children.iter().any(|child| child.layout_child.grid
        .is_some_and(|grid| grid != GridPlacement::default()))
}

fn layout_inputs(parent: &Node, layout: &AutoLayout, children: &[LayoutItem]) -> Result<(), String> {
    layout.validate_grid_tracks().map_err(|error| error.to_string())?;
    if ![parent.width, parent.height, layout.gap, layout.cross_gap,
        layout.padding.top, layout.padding.right, layout.padding.bottom, layout.padding.left]
        .iter().all(|value| value.is_finite() && *value >= 0.0)
    {
        return Err("Layout sizes, padding and gaps must be finite nonnegative numbers".into());
    }
    if layout.axis == LayoutAxis::Grid && !(1..=MAX_GRID_COLUMNS).contains(&layout.columns) {
        return Err(format!("Grid columns must be between 1 and {MAX_GRID_COLUMNS}"));
    }
    for child in children {
        let sizing = child.layout_child;
        if ![child.width, child.height, sizing.min_width, sizing.min_height, sizing.grow]
            .iter().all(|value| value.is_finite() && *value >= 0.0)
            || sizing.max_width.is_some_and(|value| !value.is_finite() || value < sizing.min_width)
            || sizing.max_height.is_some_and(|value| !value.is_finite() || value < sizing.min_height)
        {
            return Err(format!("Node {} has invalid minimum, maximum or growth settings", child.id));
        }
        if let Some(grid) = sizing.grid { grid.validate().map_err(|error| error.to_string())?; }
    }
    Ok(())
}

/// At most 128 KiB of occupancy bits and a bounded number of word probes.
/// A monotonic auto-placement cursor avoids rescanning earlier occupied cells.
struct GridOccupancy {
    columns: usize,
    max_rows: usize,
    bits: Vec<u64>,
    work: usize,
}
impl GridOccupancy {
    fn new(columns: usize) -> Self {
        Self { columns, max_rows: MAX_GRID_ROWS.min(MAX_GRID_CELLS / columns),
            bits: Vec::new(), work: 8 * MAX_GRID_CELLS }
    }
    fn spend(&mut self) -> Result<(), String> {
        self.work = self.work.checked_sub(1)
            .ok_or_else(|| "Grid placement exceeded its bounded work budget".to_string())?;
        Ok(())
    }
    fn fits(&self, cell: GridCell) -> bool {
        cell.row.checked_add(cell.row_span).is_some_and(|end| end <= self.max_rows)
            && cell.column.checked_add(cell.column_span).is_some_and(|end| end <= self.columns)
    }
    fn conflict(&mut self, cell: GridCell) -> Result<Option<usize>, String> {
        if !self.fits(cell) {
            return Err(format!("Grid placement exceeds {MAX_GRID_CELLS} cells or the supported track bounds"));
        }
        for row in cell.row..cell.row + cell.row_span {
            let mut start = row * self.columns + cell.column;
            let end = start + cell.column_span;
            while start < end {
                self.spend()?;
                let offset = start % 64;
                let count = (end - start).min(64 - offset);
                let mask = if count == 64 { u64::MAX } else { ((1u64 << count) - 1) << offset };
                let occupied = self.bits.get(start / 64).copied().unwrap_or(0) & mask;
                if occupied != 0 {
                    let position = start / 64 * 64 + occupied.trailing_zeros() as usize;
                    return Ok(Some(position - row * self.columns));
                }
                start += count;
            }
        }
        Ok(None)
    }
    fn reserve(&mut self, cell: GridCell) -> Result<(), String> {
        if !self.fits(cell) { return Err("Grid placement exceeds the supported cell bounds".into()); }
        let end = (cell.row + cell.row_span - 1) * self.columns + cell.column + cell.column_span;
        self.bits.resize(self.bits.len().max(end.div_ceil(64)), 0);
        for row in cell.row..cell.row + cell.row_span {
            let mut start = row * self.columns + cell.column;
            let end = start + cell.column_span;
            while start < end {
                self.spend()?;
                let offset = start % 64;
                let count = (end - start).min(64 - offset);
                let mask = if count == 64 { u64::MAX } else { ((1u64 << count) - 1) << offset };
                self.bits[start / 64] |= mask;
                start += count;
            }
        }
        Ok(())
    }
}

fn grid_cells(children: &[LayoutItem], columns: usize) -> Result<Vec<GridCell>, String> {
    if !(1..=MAX_GRID_COLUMNS).contains(&columns) { return Err("Invalid grid column count".into()); }
    let mut occupancy = GridOccupancy::new(columns);
    let mut cells = vec![None; children.len()];
    for child in children {
        let grid = child.layout_child.grid.unwrap_or_default();
        grid.validate().map_err(|error| error.to_string())?;
        if grid.column_span > columns
            || grid.column.is_some_and(|column| column + grid.column_span > columns)
            || grid.row_span > occupancy.max_rows
        {
            return Err(format!("Node {} does not fit the grid's column or row bounds", child.id));
        }
    }
    // Fully pinned cells have priority independent of layer order.
    for (index, child) in children.iter().enumerate() {
        let grid = child.layout_child.grid.unwrap_or_default();
        if let (Some(row), Some(column)) = (grid.row, grid.column) {
            let cell = GridCell { row, column, row_span: grid.row_span, column_span: grid.column_span };
            if occupancy.conflict(cell)?.is_some() {
                return Err(format!("Node {} overlaps another explicitly placed grid child", child.id));
            }
            occupancy.reserve(cell)?;
            cells[index] = Some(cell);
        }
    }
    // A fixed row must find a column in that row; it cannot silently drift.
    for (index, child) in children.iter().enumerate() {
        let grid = child.layout_child.grid.unwrap_or_default();
        if cells[index].is_some() || grid.row.is_none() { continue; }
        let row = grid.row.unwrap();
        let mut column = 0;
        while column + grid.column_span <= columns {
            let cell = GridCell { row, column, row_span: grid.row_span, column_span: grid.column_span };
            match occupancy.conflict(cell)? {
                Some(blocked) => column = blocked + 1,
                None => { occupancy.reserve(cell)?; cells[index] = Some(cell); break; }
            }
        }
        if cells[index].is_none() {
            return Err(format!("Node {} has no free span in its specified grid row", child.id));
        }
    }
    let mut cursor = 0;
    for (index, child) in children.iter().enumerate() {
        if cells[index].is_some() { continue; }
        let grid = child.layout_child.grid.unwrap_or_default();
        let mut row = cursor / columns;
        let mut column = grid.column.unwrap_or(cursor % columns);
        if grid.column.is_some_and(|fixed| fixed < cursor % columns) { row += 1; }
        while row + grid.row_span <= occupancy.max_rows {
            if column + grid.column_span > columns {
                row += 1;
                column = grid.column.unwrap_or(0);
                continue;
            }
            let cell = GridCell { row, column, row_span: grid.row_span, column_span: grid.column_span };
            match occupancy.conflict(cell)? {
                Some(blocked) => {
                    if let Some(fixed) = grid.column { row += 1; column = fixed; }
                    else { column = blocked + 1; }
                }
                None => {
                    occupancy.reserve(cell)?;
                    cells[index] = Some(cell);
                    cursor = row * columns + column + grid.column_span;
                    break;
                }
            }
        }
        if cells[index].is_none() {
            return Err(format!("Node {} has no free grid span within the supported bounds", child.id));
        }
    }
    Ok(cells.into_iter().map(Option::unwrap).collect())
}

fn arrange_placed_grid(children: &mut [LayoutItem], layout: &AutoLayout,
    width: &mut f64, height: &mut f64, warnings: &mut Vec<String>) -> Result<(), String> {
    let cells = grid_cells(children, layout.columns)?;
    let columns = layout.columns;
    let rows = cells.iter().map(|cell| cell.row + cell.row_span).max().unwrap_or(0);
    let p = layout.padding;
    let gap = layout.gap;
    let cross_gap = layout.cross_gap;
    for child in children.iter_mut() {
        child.width = clamp_size(child.width, child.layout_child.min_width, child.layout_child.max_width);
        child.height = clamp_size(child.height, child.layout_child.min_height, child.layout_child.max_height);
    }
    if layout.width == SizingMode::Hug {
        let track = children.iter().zip(&cells).map(|(child, cell)|
            ((child.width - gap * (cell.column_span - 1) as f64) / cell.column_span as f64).max(0.0))
            .fold(0.0, f64::max);
        *width = p.left + p.right + track * columns as f64 + gap * (columns - 1) as f64;
    }
    let track_width = ((*width - p.left - p.right - gap * (columns - 1) as f64) / columns as f64).max(0.0);
    // Spans join row tracks into groups. Equal tracks within each joined group
    // give Hug/Fill a stable intrinsic policy: writing a Fill child's arranged
    // height back cannot increase its per-track demand in a later layout pass.
    // Unjoined rows retain their independent natural height.
    let mut joins = vec![0i64; rows + 1];
    for cell in &cells {
        if cell.row_span > 1 {
            joins[cell.row] += 1;
            joins[cell.row + cell.row_span - 1] -= 1;
        }
    }
    let mut groups = Vec::with_capacity(rows);
    let (mut active, mut group) = (0i64, 0usize);
    for joined in joins.iter().take(rows) {
        groups.push(group);
        active += *joined;
        if active == 0 { group += 1; }
    }
    let mut group_heights = vec![0.0f64; group + 1];
    for (child, cell) in children.iter().zip(&cells) {
        let demand = ((child.height - cross_gap * (cell.row_span - 1) as f64) / cell.row_span as f64).max(0.0);
        group_heights[groups[cell.row]] = group_heights[groups[cell.row]].max(demand);
    }
    let mut heights: Vec<_> = groups.iter().map(|group| group_heights[*group]).collect();
    if layout.height == SizingMode::Hug {
        *height = p.top + p.bottom + heights.iter().sum::<f64>() + cross_gap * rows.saturating_sub(1) as f64;
    } else if layout.cross_align == LayoutAlign::Stretch && rows != 0 {
        heights.fill(((*height - p.top - p.bottom - cross_gap * (rows - 1) as f64) / rows as f64).max(0.0));
    }
    let mut row_y = Vec::with_capacity(rows + 1);
    row_y.push(p.top);
    for track in &heights { row_y.push(row_y.last().copied().unwrap() + track + cross_gap); }
    for (child, cell) in children.iter_mut().zip(cells) {
        let available_width = track_width * cell.column_span as f64 + gap * (cell.column_span - 1) as f64;
        let available_height = row_y[cell.row + cell.row_span] - row_y[cell.row] - cross_gap;
        if child.layout_child.width == SizingMode::Fill || layout.cross_align == LayoutAlign::Stretch {
            child.width = clamp_size(available_width, child.layout_child.min_width, child.layout_child.max_width);
        }
        if child.layout_child.height == SizingMode::Fill || layout.cross_align == LayoutAlign::Stretch {
            child.height = clamp_size(available_height, child.layout_child.min_height, child.layout_child.max_height);
        }
        if child.width > available_width + GEOMETRY_EPSILON
            || child.height > available_height + GEOMETRY_EPSILON {
            warnings.push(format!("Node {}: fixed or minimum size exceeds its allocated grid span", child.id));
        }
        child.x = p.left + cell.column as f64 * (track_width + gap)
            + alignment_offset(layout.main_align, available_width - child.width);
        child.y = row_y[cell.row] + alignment_offset(layout.cross_align, available_height - child.height);
    }
    Ok(())
}
impl Document {
    fn layout_items(&self, parent: &Node) -> Vec<LayoutItem> {
        let mut children: Vec<_> = parent.children.iter().filter_map(|id| self.nodes.get(id))
            .filter(|node| node.visible && !node.layout_child.absolute)
            .map(|node| LayoutItem { id: node.id, x: node.x, y: node.y, width: node.width,
                height: node.height, layout_child: node.layout_child }).collect();
        if parent.layout.as_ref().is_some_and(|layout| layout.reverse) { children.reverse(); }
        children
    }

    /// Old documents omit placement entirely. Only authored placement needs
    /// occupancy validation during import; the original simple grid is intact.
    pub fn validate_grid_placements(&self) -> Result<(), DocumentError> {
        for parent in self.nodes.values().filter(|node| node.layout.as_ref()
            .is_some_and(|layout| layout.axis == LayoutAxis::Grid)) {
            let children = self.layout_items(parent);
            let layout = parent.layout.as_ref().unwrap();
            if !has_grid_placement(&children) && !layout.has_explicit_grid_tracks() { continue; }
            layout_inputs(parent, layout, &children)
                .and_then(|_| grid_cells(&children, layout.columns).map(|_| ()))
                .map_err(|error| DocumentError::Invalid(format!("Grid {}: {error}", parent.id)))?;
        }
        Ok(())
    }

    pub(crate) fn validate_layout_node_settings(&self, id: NodeId) -> Result<(), DocumentError> {
        let parent = self.nodes.get(&id).ok_or(DocumentError::MissingNode(id))?;
        let layout = parent.layout.as_ref().ok_or_else(|| DocumentError::Invalid("Parent does not use auto layout".into()))?;
        let children = self.layout_items(parent);
        layout_inputs(parent, layout, &children).map_err(DocumentError::Invalid)?;
        if layout.axis == LayoutAxis::Grid && (has_grid_placement(&children) || layout.has_explicit_grid_tracks()) {
            grid_cells(&children, layout.columns).map_err(DocumentError::Invalid)?;
        }
        Ok(())
    }

    pub fn apply_constraints(
        &mut self,
        parent: NodeId,
        old_width: f64,
        old_height: f64,
        new_width: f64,
        new_height: f64,
    ) {
        let Some(p) = self.nodes.get(&parent) else {
            return;
        };
        let has_layout = p.layout.is_some();
        let children = p.children.clone();
        let guides: Vec<_> = p.layout_guides().copied().collect();
        fn axis(p: &mut f64, s: &mut f64, old: f64, new: f64, c: Constraint) {
            let delta = new - old;
            match c {
                Constraint::Start => {}
                Constraint::End => *p += delta,
                Constraint::Center => *p += delta / 2.0,
                Constraint::Stretch => *s = (*s + delta).max(0.0),
                Constraint::Scale => {
                    if old.abs() > GEOMETRY_EPSILON {
                        *p *= new / old;
                        *s = (*s * new / old).max(0.0)
                    }
                }
            }
        }
        for id in children {
            if let Some(c) = self.nodes.get_mut(&id) {
                if has_layout && !c.layout_child.absolute {
                    continue;
                }
                if crate::layout_guides::apply_layout_guide_constraints(c, &guides,
                    crate::Point::new(old_width, old_height), crate::Point::new(new_width, new_height)) {
                    continue;
                }
                let (mut x, mut y, mut width, mut height) = (c.x, c.y, c.width, c.height);
                axis(
                    &mut x,
                    &mut width,
                    old_width,
                    new_width,
                    c.constraints.horizontal,
                );
                axis(
                    &mut y,
                    &mut height,
                    old_height,
                    new_height,
                    c.constraints.vertical,
                );
                if c.resize_vector_geometry(width, height) {
                    c.x = x;
                    c.y = y;
                    c.width = width;
                    c.height = height;
                }
            }
        }
    }
    pub fn layout_node(&mut self, id: NodeId) -> LayoutResult {
        let mut result = LayoutResult::default();
        let Some(parent) = self.nodes.get(&id) else {
            return result;
        };
        let Some(layout) = parent.layout.clone() else {
            return result;
        };
        let mut children = self.layout_items(parent);
        if let Err(error) = layout_inputs(parent, &layout, &children) {
            result.errors.push(format!("Node {id}: {error}"));
            return result;
        }
        let p = layout.padding;
        let mut pw = parent.width;
        let mut ph = parent.height;
        let gap = layout.gap.max(0.0);
        let cross_gap = layout.cross_gap.max(0.0);
        if layout.axis == LayoutAxis::Grid && layout.has_explicit_grid_tracks() {
            if let Err(error) = grid_track_layout::arrange_explicit_grid(&mut children, &layout, &mut pw, &mut ph, &mut result.warnings) {
                result.errors.push(format!("Node {id}: {error}"));
                return result;
            }
        } else if children.is_empty() {
            if layout.width == SizingMode::Hug {
                pw = p.left + p.right
            }
            if layout.height == SizingMode::Hug {
                ph = p.top + p.bottom
            }
        } else if layout.axis == LayoutAxis::Grid && has_grid_placement(&children) {
            if let Err(error) = arrange_placed_grid(&mut children, &layout, &mut pw, &mut ph, &mut result.warnings) {
                result.errors.push(format!("Node {id}: {error}"));
                return result;
            }
        } else if layout.axis == LayoutAxis::Grid {
            let columns = layout.columns.max(1).min(4096).min(children.len());
            let rows = children.len().div_ceil(columns);
            if layout.width == SizingMode::Hug {
                pw = p.left
                    + p.right
                    + (0..columns)
                        .map(|col| {
                            children
                                .iter()
                                .skip(col)
                                .step_by(columns)
                                .map(|n| n.width)
                                .fold(0.0, f64::max)
                        })
                        .sum::<f64>()
                    + gap * (columns - 1) as f64
            }
            let cell_width =
                ((pw - p.left - p.right - gap * (columns - 1) as f64) / columns as f64).max(0.0);
            let mut heights = vec![0.0f64; rows];
            for (i, c) in children.iter_mut().enumerate() {
                if c.layout_child.width == SizingMode::Fill
                    || layout.cross_align == LayoutAlign::Stretch
                {
                    c.width = clamp_size(
                        cell_width,
                        c.layout_child.min_width,
                        c.layout_child.max_width,
                    )
                }
                heights[i / columns] = heights[i / columns].max(c.height)
            }
            if layout.height == SizingMode::Hug {
                ph = p.top + p.bottom + heights.iter().sum::<f64>() + cross_gap * (rows - 1) as f64
            } else if layout.cross_align == LayoutAlign::Stretch {
                heights.fill(
                    ((ph - p.top - p.bottom - cross_gap * (rows - 1) as f64) / rows as f64)
                        .max(0.0),
                )
            }
            let mut y = p.top;
            for row in 0..rows {
                for col in 0..columns {
                    let Some(c) = children.get_mut(row * columns + col) else {
                        continue;
                    };
                    if c.layout_child.height == SizingMode::Fill {
                        c.height = clamp_size(
                            heights[row],
                            c.layout_child.min_height,
                            c.layout_child.max_height,
                        )
                    }
                    c.x = p.left
                        + col as f64 * (cell_width + gap)
                        + alignment_offset(layout.main_align, cell_width - c.width);
                    c.y = y + alignment_offset(layout.cross_align, heights[row] - c.height)
                }
                y += heights[row] + cross_gap
            }
        } else {
            let row = layout.axis == LayoutAxis::Row;
            let (mb, ma, cb, ca) = if row {
                (p.left, p.right, p.top, p.bottom)
            } else {
                (p.top, p.bottom, p.left, p.right)
            };
            let main_hug = (if row { layout.width } else { layout.height }) == SizingMode::Hug;
            let cross_hug = (if row { layout.height } else { layout.width }) == SizingMode::Hug;
            let mut main = if row { pw } else { ph };
            let mut cross = if row { ph } else { pw };
            let cm = |n: &LayoutItem| if row { n.width } else { n.height };
            let cc = |n: &LayoutItem| if row { n.height } else { n.width };
            if main_hug {
                main = mb
                    + ma
                    + children.iter().map(cm).sum::<f64>()
                    + gap * children.len().saturating_sub(1) as f64;
                if children.iter().any(|n| {
                    (if row {
                        n.layout_child.width
                    } else {
                        n.layout_child.height
                    }) == SizingMode::Fill
                }) {
                    result.warnings.push(format!(
                        "Node {id}: fill children use intrinsic sizes on a hug main axis"
                    ))
                }
            }
            let available = (main - mb - ma).max(0.0);
            let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
            let mut used = 0.0;
            for (i, c) in children.iter().enumerate() {
                let needed = cm(c)
                    + if lines.last().unwrap().is_empty() {
                        0.0
                    } else {
                        gap
                    };
                if layout.wrap
                    && !main_hug
                    && !lines.last().unwrap().is_empty()
                    && used + needed > available + GEOMETRY_EPSILON
                {
                    lines.push(Vec::new());
                    used = 0.0
                }
                if !lines.last().unwrap().is_empty() {
                    used += gap
                }
                used += cm(c);
                lines.last_mut().unwrap().push(i)
            }
            let mut cross_sizes = Vec::new();
            for line in &lines {
                let flexible: Vec<usize> = line
                    .iter()
                    .copied()
                    .filter(|i| {
                        !main_hug
                            && (if row {
                                children[*i].layout_child.width
                            } else {
                                children[*i].layout_child.height
                            }) == SizingMode::Fill
                    })
                    .collect();
                if !flexible.is_empty() {
                    let fixed: f64 = line
                        .iter()
                        .filter(|i| {
                            (if row {
                                children[**i].layout_child.width
                            } else {
                                children[**i].layout_child.height
                            }) != SizingMode::Fill
                        })
                        .map(|i| cm(&children[*i]))
                        .sum();
                    let space =
                        (available - fixed - gap * line.len().saturating_sub(1) as f64).max(0.0);
                    let items: Vec<_> = flexible
                        .iter()
                        .map(|i| {
                            let c = children[*i].layout_child;
                            if row {
                                (c.grow, c.min_width, c.max_width)
                            } else {
                                (c.grow, c.min_height, c.max_height)
                            }
                        })
                        .collect();
                    for (i, size) in flexible.iter().zip(flex_sizes(space, &items)) {
                        if row {
                            children[*i].width = size
                        } else {
                            children[*i].height = size
                        }
                    }
                }
                cross_sizes.push(line.iter().map(|i| cc(&children[*i])).fold(0.0, f64::max));
            }
            if cross_hug {
                cross = cb
                    + ca
                    + cross_sizes.iter().sum::<f64>()
                    + cross_gap * lines.len().saturating_sub(1) as f64
            } else if lines.len() == 1 {
                cross_sizes[0] = (cross - cb - ca).max(0.0)
            }
            let mut cross_cursor = cb;
            for (line, line_cross) in lines.iter().zip(cross_sizes) {
                let content: f64 = line.iter().map(|i| cm(&children[*i])).sum();
                let remaining =
                    (available - content - gap * line.len().saturating_sub(1) as f64).max(0.0);
                let effective_gap =
                    if layout.main_align == LayoutAlign::SpaceBetween && line.len() > 1 {
                        gap + remaining / (line.len() - 1) as f64
                    } else {
                        gap
                    };
                let mut main_cursor = mb + alignment_offset(layout.main_align, remaining);
                for i in line {
                    let c = &mut children[*i];
                    let fill_cross = (if row {
                        c.layout_child.height
                    } else {
                        c.layout_child.width
                    }) == SizingMode::Fill;
                    if fill_cross || layout.cross_align == LayoutAlign::Stretch {
                        if row {
                            c.height = clamp_size(
                                line_cross,
                                c.layout_child.min_height,
                                c.layout_child.max_height,
                            )
                        } else {
                            c.width = clamp_size(
                                line_cross,
                                c.layout_child.min_width,
                                c.layout_child.max_width,
                            )
                        }
                    }
                    let offset = alignment_offset(layout.cross_align, line_cross - cc(c));
                    if row {
                        c.x = main_cursor;
                        c.y = cross_cursor + offset
                    } else {
                        c.y = main_cursor;
                        c.x = cross_cursor + offset
                    }
                    main_cursor += cm(c) + effective_gap
                }
                cross_cursor += line_cross + cross_gap;
            }
            if row {
                pw = main;
                ph = cross
            } else {
                ph = main;
                pw = cross
            }
        }
        if ![pw, ph].iter().all(|value| value.is_finite() && *value >= 0.0)
            || children.iter().any(|child| ![child.x, child.y, child.width, child.height]
                .iter().all(|value| value.is_finite()) || child.width < 0.0 || child.height < 0.0)
        {
            result.errors.push(format!("Node {id}: layout geometry overflowed its finite bounds"));
            return result;
        }
        for c in children {
            if let Some(n) = self.nodes.get_mut(&c.id) {
                if n.x != c.x || n.y != c.y || n.width != c.width || n.height != c.height {
                    if !n.resize_vector_geometry(c.width, c.height) {
                        result.warnings.push(format!("Vector geometry could not fit layout bounds on node {}", n.id));
                        result.errors.push(format!("Vector geometry could not fit layout bounds on node {}", n.id));
                        continue;
                    }
                    n.x = c.x;
                    n.y = c.y;
                    n.width = c.width;
                    n.height = c.height;
                    result.changed.push(n.id)
                }
            }
        }
        if let Some(n) = self.nodes.get_mut(&id) {
            if n.width != pw || n.height != ph {
                n.width = pw.max(0.0);
                n.height = ph.max(0.0);
                result.changed.push(id)
            }
        }
        result
    }
    /// Bottom-up measure and top-down arrange, with bounded convergence for nested wrapping.
    pub fn layout_all(&mut self) -> LayoutResult {
        let order: Vec<_> = self
            .pages
            .iter()
            .flat_map(|p| self.draw_order(p.id))
            .collect();
        let mut changed = BTreeSet::new();
        let mut warnings = BTreeSet::new();
        let mut errors = BTreeSet::new();
        for _ in 0..6 {
            let mut count = 0;
            for id in order.iter().rev().chain(order.iter()) {
                let r = self.layout_node(*id);
                count += r.changed.len();
                changed.extend(r.changed);
                warnings.extend(r.warnings);
                errors.extend(r.errors);
            }
            if count == 0 || !errors.is_empty() {
                break;
            }
        }
        LayoutResult {
            changed: changed.into_iter().collect(),
            warnings: warnings.into_iter().collect(),
            errors: errors.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn dense_mixed_fill_layout_preserves_payloads_and_excluded_children() {
        let mut document = Document::new("Dense layout");
        let page = document.pages[0].id;
        let mut row = Node::new(NodeKind::Frame, "Row", 20.0, 30.0, 10_000.0, 80.0);
        row.layout = Some(AutoLayout {
            gap: 1.0,
            padding: Padding::all(4.0),
            height: SizingMode::Fixed,
            ..AutoLayout::default()
        });
        let row = document.insert_node(page, row).unwrap();
        let mut originals = Vec::new();
        for index in 0..1024 {
            let mut node = Node::new(NodeKind::Text, format!("Text {index}"),
                7.0, 9.0, 2.0, 24.0)
                .with_text("Layout retains café Ω 🚀. ".repeat(32), 16.0, Color::BLACK);
            if index % 4 != 0 {
                node.layout_child.width = SizingMode::Fill;
            }
            let id = document.insert_node(row, node).unwrap();
            originals.push(document.nodes[&id].clone());
        }
        for absolute in [false, true] {
            let mut node = Node::new(NodeKind::Rectangle, "Excluded", 700.0, 800.0, 99.0, 100.0);
            node.visible = absolute;
            node.layout_child.absolute = absolute;
            let id = document.insert_node(row, node).unwrap();
            originals.push(document.nodes[&id].clone());
        }
        let result = document.layout_all();
        assert!(result.warnings.is_empty());
        let fill_width = (10_000.0 - 8.0 - 1023.0 - 256.0 * 2.0) / 768.0;
        let mut x = 4.0;
        for (index, original) in originals.iter().enumerate() {
            let actual = &document.nodes[&original.id];
            if index < 1024 {
                let width = if index % 4 == 0 { 2.0 } else { fill_width };
                assert!((actual.x - x).abs() < 1e-6);
                assert!((actual.width - width).abs() < 1e-6);
                assert_eq!(actual.y, 4.0);
                x += width + 1.0;
                let mut payload = actual.clone();
                payload.x = original.x;
                payload.y = original.y;
                payload.width = original.width;
                payload.height = original.height;
                assert_eq!(&payload, original);
            } else {
                assert_eq!(actual, original);
                assert!(!result.changed.contains(&original.id));
            }
        }
        assert!((x - 1.0 - 9996.0).abs() < 1e-6);
        assert!(document.layout_all().changed.is_empty());
        document.validate().unwrap();
    }

    #[test]
    fn no_layout_document_retains_all_content_and_geometry() {
        let mut document = Document::new("No layout");
        let page = document.pages[0].id;
        for index in 0..128 {
            document.insert_node(page,
                Node::new(NodeKind::Text, format!("Text {index}"),
                    index as f64 * 24.0, 90.0, 200.0, 40.0)
                    .with_text("Unchanged rich content Ω 🚀 ".repeat(64), 18.0, Color::BLACK))
                .unwrap();
        }
        let before = document.to_json().unwrap();
        assert_eq!(document.layout_all(), LayoutResult::default());
        assert_eq!(document.to_json().unwrap(), before);
    }
}
