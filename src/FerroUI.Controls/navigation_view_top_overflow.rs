//! Which items of the top pane of a `NavigationView` stay in it and which move to the overflow
//! menu: the end state of WinUI's `ArrangeTopNavItems`, `ShrinkTopNavigationSize` and
//! `FindMovableItemsRecoverToPrimaryList`, computed from the cached widths of the items.

/// `c_topNavigationRecoveryGracePeriodWidth`: the extra width needed before an overflowing pane
/// takes all its items back, so that a width at the limit does not flip between the two layouts.
pub(crate) const TOP_RECOVERY_GRACE_WIDTH: f64 = 5.0;

/// Returns, for each item, whether it stays in the primary list.
///
/// `widths` are the widths of the items in the pane, `available` the width the items and the
/// overflow button can share, `overflow_button` the width of that button, `selected` the index
/// of the selected top-level item and `overflowing` whether the pane has an overflow now.
///
/// Items are kept in their order until one does not fit. The selected item is always kept, and
/// at least one item stays in the primary list.
pub(crate) fn primary_top_items(
    widths: &[f64],
    available: f64,
    overflow_button: f64,
    selected: Option<usize>,
    overflowing: bool,
) -> Vec<bool> {
    let total: f64 = widths.iter().sum();
    let needed = if overflowing { total + TOP_RECOVERY_GRACE_WIDTH } else { total };
    // `ArrangeTopNavItems` overflows unless the desired width is below the available width.
    if needed < available {
        return vec![true; widths.len()];
    }

    let selected = selected.filter(|&index| index < widths.len());
    let budget = available - overflow_button;
    let mut primary = vec![false; widths.len()];
    let mut used = 0.0;
    if let Some(index) = selected {
        primary[index] = true;
        used = widths[index];
    }
    for (index, &width) in widths.iter().enumerate() {
        if Some(index) == selected {
            continue;
        }
        if used + width > budget {
            break;
        }
        primary[index] = true;
        used += width;
    }
    // `KeepAtLeastOneItemInPrimaryList`.
    if !primary.contains(&true) {
        if let Some(first) = primary.first_mut() {
            *first = true;
        }
    }
    primary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept(primary: &[bool]) -> Vec<usize> {
        primary.iter().enumerate().filter(|(_, &keep)| keep).map(|(index, _)| index).collect()
    }

    #[test]
    fn all_items_stay_when_they_fit() {
        assert_eq!(kept(&primary_top_items(&[50.0, 50.0], 101.0, 40.0, None, false)), [0, 1]);
    }

    #[test]
    fn a_pane_of_exactly_the_desired_width_overflows() {
        assert_eq!(kept(&primary_top_items(&[50.0, 50.0], 100.0, 40.0, None, false)), [0]);
    }

    #[test]
    fn items_are_kept_in_order_beside_the_overflow_button() {
        let widths = [50.0, 50.0, 50.0, 50.0];
        assert_eq!(kept(&primary_top_items(&widths, 150.0, 40.0, None, false)), [0, 1]);
    }

    #[test]
    fn the_selected_item_is_always_kept() {
        let widths = [50.0, 50.0, 50.0, 50.0];
        assert_eq!(kept(&primary_top_items(&widths, 150.0, 40.0, Some(3), false)), [0, 3]);
        assert_eq!(kept(&primary_top_items(&widths, 60.0, 40.0, Some(2), false)), [2]);
    }

    #[test]
    fn at_least_one_item_stays() {
        assert_eq!(kept(&primary_top_items(&[50.0, 50.0], 10.0, 40.0, None, false)), [0]);
        assert!(primary_top_items(&[], 10.0, 40.0, None, false).is_empty());
    }

    #[test]
    fn a_selection_out_of_range_is_ignored() {
        assert_eq!(kept(&primary_top_items(&[50.0, 50.0], 60.0, 10.0, Some(9), false)), [0]);
    }

    #[test]
    fn recovery_needs_the_grace_width() {
        let widths = [50.0, 50.0];
        assert_eq!(kept(&primary_top_items(&widths, 103.0, 40.0, None, true)), [0]);
        assert_eq!(kept(&primary_top_items(&widths, 106.0, 40.0, None, true)), [0, 1]);
    }
}
