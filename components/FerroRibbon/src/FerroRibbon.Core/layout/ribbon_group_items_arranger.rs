#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RibbonArrangeItem {
    pub width: f64,
    pub height: f64,
    pub is_full_height: bool,
    pub starts_new_column: bool,
}

impl RibbonArrangeItem {
    pub fn new(width: f64, height: f64, is_full_height: bool) -> Self {
        Self {
            width,
            height,
            is_full_height,
            starts_new_column: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RibbonArrangedItem {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub column: i32,
    pub row: i32,
}

#[derive(Clone, Debug)]
pub struct RibbonArrangeResult {
    pub items: Vec<RibbonArrangedItem>,
    pub width: f64,
    pub height: f64,
}

pub struct RibbonGroupItemsArranger;

impl RibbonGroupItemsArranger {
    pub fn arrange(items: &[RibbonArrangeItem], content_height: f64) -> RibbonArrangeResult {
        Self::arrange_with_options(items, content_height, 3, 2.0, false)
    }

    pub fn arrange_with_options(
        items: &[RibbonArrangeItem],
        content_height: f64,
        rows: i32,
        column_spacing: f64,
        distribute_rows: bool,
    ) -> RibbonArrangeResult {
        let rows = rows.clamp(1, 3) as usize;
        let slot_height = content_height / rows as f64;
        let mut result = vec![RibbonArrangedItem::default(); items.len()];
        let mut columns: Vec<Vec<usize>> = Vec::new();
        let mut full_height = Vec::new();
        let mut current: Option<usize> = None;
        for (i, item) in items.iter().enumerate() {
            if item.is_full_height {
                columns.push(vec![i]);
                full_height.push(true);
                current = None;
                continue;
            }
            if current.is_none_or(|c| columns[c].len() >= rows) || item.starts_new_column {
                current = Some(columns.len());
                columns.push(Vec::new());
                full_height.push(false);
            }
            columns[current.expect("column created above")].push(i);
        }
        let mut x = 0.0;
        for (c, column) in columns.iter().enumerate() {
            // Enumerable.Max(double) ignores NaNs unless every value is NaN;
            // equal values retain the first value (including the sign of zero).
            let mut width = items[column[0]].width;
            for &i in &column[1..] {
                let next = items[i].width;
                if next > width || width.is_nan() {
                    width = next;
                }
            }
            if full_height[c] {
                result[column[0]] = RibbonArrangedItem {
                    x,
                    y: 0.0,
                    width,
                    height: content_height,
                    column: c as i32,
                    row: 0,
                };
            } else {
                let gap = if distribute_rows && column.len() < rows {
                    (content_height - column.len() as f64 * slot_height) / (column.len() + 1) as f64
                } else {
                    0.0
                };
                for (r, &i) in column.iter().enumerate() {
                    let y = if distribute_rows {
                        gap + r as f64 * (slot_height + gap)
                    } else {
                        r as f64 * slot_height
                    };
                    let height = dotnet_min(items[i].height, slot_height);
                    result[i] = RibbonArrangedItem {
                        x,
                        y: y + (slot_height - height) / 2.0,
                        width: items[i].width,
                        height,
                        column: c as i32,
                        row: r as i32,
                    };
                }
            }
            x += width
                + if c < columns.len() - 1 {
                    column_spacing
                } else {
                    0.0
                };
        }
        RibbonArrangeResult {
            items: result,
            width: dotnet_max(0.0, x),
            height: content_height,
        }
    }
}

// Math.Min/Max propagate NaN and choose the correct signed zero, unlike f64::min/max.
fn dotnet_min(a: f64, b: f64) -> f64 {
    if a != b {
        if a.is_nan() || a < b { a } else { b }
    } else if a.is_sign_negative() {
        a
    } else {
        b
    }
}

fn dotnet_max(a: f64, b: f64) -> f64 {
    if a != b {
        if a.is_nan() || a > b { a } else { b }
    } else if b.is_sign_negative() {
        a
    } else {
        b
    }
}
