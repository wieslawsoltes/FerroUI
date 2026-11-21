use crate::RibbonGroupState;
use std::{cell::RefCell, rc::Rc};

/// Measured widths retain their original shared mutable collection.
/// General custom IReadOnlyList implementations and record metadata remain unported.
#[derive(Clone, Debug)]
pub struct RibbonGroupLayoutInfo {
    pub widths: Option<Rc<RefCell<Vec<f64>>>>,
    pub reduction_order: i32,
    pub can_collapse: bool,
}

impl RibbonGroupLayoutInfo {
    pub fn new(widths: Vec<f64>) -> Self {
        Self {
            widths: Some(Rc::new(RefCell::new(widths))),
            reduction_order: 0,
            can_collapse: true,
        }
    }

    pub fn get_width(&self, state: RibbonGroupState) -> f64 {
        let Some(widths) = &self.widths else {
            return f64::NAN;
        };
        let widths = widths.borrow();
        // The source tests only the upper bound, so a negative state is an indexing error.
        if state.raw() < widths.len() as i32 {
            widths[usize::try_from(state.raw()).expect("negative group state")]
        } else {
            f64::NAN
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonReductionStrategy(i32);
ferroui_system::styled_value!(RibbonReductionStrategy);

#[allow(non_upper_case_globals)]
impl RibbonReductionStrategy {
    pub const Stepwise: Self = Self(0);
    pub const GroupByGroup: Self = Self(1);
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}
impl std::fmt::Display for RibbonReductionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Stepwise => f.write_str("Stepwise"),
            Self::GroupByGroup => f.write_str("GroupByGroup"),
            _ => self.0.fmt(f),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RibbonAdaptiveLayoutResult {
    pub states: Vec<RibbonGroupState>,
    pub total_width: f64,
    pub fits: bool,
}

pub struct RibbonAdaptiveLayout;

impl RibbonAdaptiveLayout {
    pub fn compute(
        groups: &[RibbonGroupLayoutInfo],
        available_width: f64,
    ) -> RibbonAdaptiveLayoutResult {
        Self::compute_with_spacing(groups, available_width, 0.0)
    }

    pub fn compute_with_spacing(
        groups: &[RibbonGroupLayoutInfo],
        available_width: f64,
        spacing: f64,
    ) -> RibbonAdaptiveLayoutResult {
        Self::compute_with_strategy(
            groups,
            available_width,
            spacing,
            RibbonReductionStrategy::Stepwise,
        )
    }

    pub fn compute_with_strategy(
        groups: &[RibbonGroupLayoutInfo],
        available_width: f64,
        spacing: f64,
        strategy: RibbonReductionStrategy,
    ) -> RibbonAdaptiveLayoutResult {
        let mut states = vec![RibbonGroupState::Large; groups.len()];
        let mut total = Self::total(groups, &states, spacing);
        if total <= available_width || groups.is_empty() {
            return RibbonAdaptiveLayoutResult {
                states,
                total_width: total,
                fits: true,
            };
        }
        let mut order: Vec<_> = (0..groups.len()).collect();
        order.sort_by(|a, b| {
            groups[*b]
                .reduction_order
                .cmp(&groups[*a].reduction_order)
                .then(b.cmp(a))
        });
        let targets = [
            RibbonGroupState::Medium,
            RibbonGroupState::Small,
            RibbonGroupState::Collapsed,
        ];
        if strategy == RibbonReductionStrategy::GroupByGroup {
            for index in order {
                let info = &groups[index];
                for target in targets {
                    if target == RibbonGroupState::Collapsed && !info.can_collapse {
                        continue;
                    }
                    let current = info.get_width(states[index]);
                    let next = info.get_width(target);
                    if next.is_nan() || next > current {
                        continue;
                    }
                    total -= current - next;
                    states[index] = target;
                    if total <= available_width {
                        return RibbonAdaptiveLayoutResult {
                            states,
                            total_width: total,
                            fits: true,
                        };
                    }
                }
            }
        } else {
            for target in targets {
                for &index in &order {
                    let info = &groups[index];
                    if target == RibbonGroupState::Collapsed && !info.can_collapse {
                        continue;
                    }
                    let current = info.get_width(states[index]);
                    let next = info.get_width(target);
                    if next.is_nan() || next >= current - 0.5 {
                        if !next.is_nan()
                            && target != RibbonGroupState::Collapsed
                            && next <= current
                        {
                            total -= current - next;
                            states[index] = target;
                        }
                        continue;
                    }
                    total -= current - next;
                    states[index] = target;
                    if total <= available_width {
                        return RibbonAdaptiveLayoutResult {
                            states,
                            total_width: total,
                            fits: true,
                        };
                    }
                }
            }
        }
        total = Self::total(groups, &states, spacing);
        RibbonAdaptiveLayoutResult {
            states,
            total_width: total,
            fits: false,
        }
    }

    fn total(groups: &[RibbonGroupLayoutInfo], states: &[RibbonGroupState], spacing: f64) -> f64 {
        let mut total = 0.0;
        for (info, state) in groups.iter().zip(states) {
            total += info.get_width(*state);
        }
        total + groups.len().saturating_sub(1) as f64 * spacing
    }
}
