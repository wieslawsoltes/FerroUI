use crate::RibbonSimplifiedVisibility;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RibbonSimplifiedItem {
    pub width: f64,
    pub visibility: RibbonSimplifiedVisibility,
}

#[derive(Clone, Debug)]
pub struct RibbonSimplifiedGroup {
    pub items: Vec<RibbonSimplifiedItem>,
    pub reduction_order: i32,
}

impl RibbonSimplifiedGroup {
    pub fn new(items: Vec<RibbonSimplifiedItem>) -> Self {
        Self {
            items,
            reduction_order: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RibbonSimplifiedLayoutResult {
    pub in_line: Vec<Vec<bool>>,
    pub has_overflow: bool,
    pub total_width: f64,
}

pub struct RibbonSimplifiedLayout;

impl RibbonSimplifiedLayout {
    pub fn compute(
        groups: &[RibbonSimplifiedGroup],
        available_width: f64,
        overflow_button_width: f64,
    ) -> RibbonSimplifiedLayoutResult {
        Self::compute_with_spacing(groups, available_width, overflow_button_width, 0.0)
    }

    pub fn compute_with_spacing(
        groups: &[RibbonSimplifiedGroup],
        available_width: f64,
        overflow_button_width: f64,
        group_spacing: f64,
    ) -> RibbonSimplifiedLayoutResult {
        use RibbonSimplifiedVisibility as V;
        let mut in_line: Vec<Vec<_>> = groups
            .iter()
            .map(|g| {
                g.items
                    .iter()
                    .map(|i| matches!(i.visibility, V::Auto | V::Pinned))
                    .collect()
            })
            .collect();
        let mut has_overflow = groups
            .iter()
            .any(|g| g.items.iter().any(|i| i.visibility == V::Overflow));
        let width = |in_line: &[Vec<bool>], has_overflow| {
            let mut total = 0.0;
            let mut visible_groups: usize = 0;
            for (g, group) in groups.iter().enumerate() {
                let mut any = false;
                for (i, item) in group.items.iter().enumerate() {
                    if in_line[g][i] {
                        total += item.width;
                        any = true;
                    }
                }
                if any {
                    visible_groups += 1;
                }
            }
            total
                + visible_groups.saturating_sub(1) as f64 * group_spacing
                + if has_overflow {
                    overflow_button_width
                } else {
                    0.0
                }
        };
        let mut order: Vec<_> = (0..groups.len()).collect();
        order.sort_by(|a, b| {
            groups[*b]
                .reduction_order
                .cmp(&groups[*a].reduction_order)
                .then(b.cmp(a))
        });
        for pass in [V::Auto, V::Pinned] {
            for &g in &order {
                for i in (0..groups[g].items.len()).rev() {
                    if width(&in_line, has_overflow) <= available_width {
                        let total_width = width(&in_line, has_overflow);
                        return RibbonSimplifiedLayoutResult {
                            in_line,
                            has_overflow,
                            total_width,
                        };
                    }
                    if in_line[g][i] && groups[g].items[i].visibility == pass {
                        in_line[g][i] = false;
                        has_overflow = true;
                    }
                }
            }
        }
        let total_width = width(&in_line, has_overflow);
        RibbonSimplifiedLayoutResult {
            in_line,
            has_overflow,
            total_width,
        }
    }
}
