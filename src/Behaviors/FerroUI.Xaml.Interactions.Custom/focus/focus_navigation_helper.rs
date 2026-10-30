//! Port of `Focus/FocusNavigationHelper.cs` (an internal class of the reference).
//!
//! Not in the folders of part A: `FocusNextElementAction` (`Actions/`) and its tests need it, so
//! part A ports it here, in the folder of the reference, and declares it from `part_a.rs`.

use ferroui_base::input::{InputElement, KeyboardNavigation, KeyboardNavigationMode, NavigationDirection};
use ferroui_base::{Ref, Visual};

/// The tab order of the elements of a scope: the element that receives the focus for an entry
/// (none for an element that does not take part in tab navigation) and the element the entry
/// stands for.
#[derive(Clone)]
struct TabOrderEntry {
    target: Option<Ref<InputElement>>,
    owner: Ref<InputElement>,
}

/// Finds the next or previous element of the tab navigation of a scope, following the
/// keyboard navigation modes of its containers (`Cycle`, `Contained`, `Once`, `None`, `Local`).
pub(crate) struct FocusNavigationHelper;

impl FocusNavigationHelper {
    /// The first (or, for [`NavigationDirection::Previous`], the last) element of the tab order of
    /// `scope`.
    pub(crate) fn find_boundary(scope: &Visual, direction: NavigationDirection) -> Option<Ref<InputElement>> {
        Self::find_boundary_in(&Self::get_tab_order_entries(scope), direction)
    }

    /// The element after (or before) `current` in the tab order of `scope`; with `wrap`, the
    /// boundary when there is none.
    pub(crate) fn find_adjacent(
        scope: &Visual,
        current: Option<&InputElement>,
        direction: NavigationDirection,
        wrap: bool,
    ) -> Option<Ref<InputElement>> {
        let Some(current_element) = current else {
            return if wrap { Self::find_boundary(scope, direction) } else { None };
        };

        for navigation_scope in Self::get_special_navigation_scopes(current_element, scope) {
            if let Some(candidate) = Self::try_find_in_navigation_scope(&navigation_scope, current_element, direction) {
                return candidate;
            }
        }

        if let Some(scope_element) = scope.to_ref().cast::<InputElement>() {
            if let Some(scoped_candidate) = Self::try_find_in_navigation_scope(&scope_element, current_element, direction)
            {
                return scoped_candidate;
            }
        }

        Self::find_adjacent_in_scope(scope, current_element, direction, wrap)
    }

    fn get_special_navigation_scopes(current: &InputElement, root: &Visual) -> Vec<Ref<InputElement>> {
        let mut scopes = Vec::new();
        let mut visual = current.get_visual_parent();
        while let Some(current_visual) = visual {
            if std::ptr::addr_eq(&*current_visual as *const Visual, root as *const Visual) {
                break;
            }

            if let Some(input_element) = current_visual.cast::<InputElement>() {
                let mode = KeyboardNavigation::get_tab_navigation(&input_element);
                if mode == KeyboardNavigationMode::Cycle || mode == KeyboardNavigationMode::Contained {
                    scopes.push(input_element);
                }
            }

            visual = current_visual.get_visual_parent();
        }

        scopes
    }

    /// `TryFindInNavigationScope`: `None` when the scope does not decide (the `false` of the
    /// reference), else the candidate it found (the `out` value).
    fn try_find_in_navigation_scope(
        scope: &InputElement,
        current: &InputElement,
        direction: NavigationDirection,
    ) -> Option<Option<Ref<InputElement>>> {
        let mode = KeyboardNavigation::get_tab_navigation(scope);
        if mode != KeyboardNavigationMode::Cycle && mode != KeyboardNavigationMode::Contained {
            return None;
        }

        let candidate =
            Self::find_adjacent_in_scope(scope, current, direction, mode == KeyboardNavigationMode::Cycle);
        if candidate.is_some() {
            return Some(candidate);
        }

        if mode == KeyboardNavigationMode::Contained {
            return Some(Some(current.to_ref()));
        }

        None
    }

    fn find_adjacent_in_scope(
        scope: &Visual,
        current: &InputElement,
        direction: NavigationDirection,
        wrap: bool,
    ) -> Option<Ref<InputElement>> {
        let entries = Self::get_tab_order_entries(scope);
        if entries.is_empty() {
            return None;
        }

        let Some(current_index) = Self::find_current_entry_index(&entries, current, scope) else {
            return if wrap { Self::find_boundary_in(&entries, direction) } else { None };
        };

        let candidate = if direction == NavigationDirection::Previous {
            entries[..current_index].iter().rev().find_map(|entry| entry.target.clone())
        } else {
            entries[current_index + 1..].iter().find_map(|entry| entry.target.clone())
        };
        if candidate.is_some() {
            return candidate;
        }

        if wrap {
            Self::find_boundary_in(&entries, direction)
        } else {
            None
        }
    }

    fn find_current_entry_index(entries: &[TabOrderEntry], current: &InputElement, scope: &Visual) -> Option<usize> {
        let is = |element: &Ref<InputElement>, other: &InputElement| {
            std::ptr::addr_eq(&**element as *const InputElement, other as *const InputElement)
        };

        if let Some(index) =
            entries.iter().position(|entry| entry.target.as_ref().is_some_and(|target| is(target, current)))
        {
            return Some(index);
        }

        let mut visual = current.get_visual_parent();
        while let Some(current_visual) = visual {
            if let Some(input_element) = current_visual.cast::<InputElement>() {
                if let Some(index) = entries.iter().position(|entry| is(&entry.owner, &*input_element)) {
                    return Some(index);
                }
            }

            if std::ptr::addr_eq(&*current_visual as *const Visual, scope as *const Visual) {
                break;
            }

            visual = current_visual.get_visual_parent();
        }

        None
    }

    fn get_tab_order_entries(scope: &Visual) -> Vec<TabOrderEntry> {
        let mut entries = Vec::new();
        Self::build_tab_order_entries(scope, &mut entries, true);
        entries
    }

    fn build_tab_order_entries(scope: &Visual, entries: &mut Vec<TabOrderEntry>, include_self: bool) {
        if let Some(input_element) = scope.to_ref().cast::<InputElement>() {
            if include_self && Self::can_participate_in_tab_navigation(&input_element) {
                entries.push(TabOrderEntry { target: Some(input_element.clone()), owner: input_element });
            }
        }

        for child_visual in Self::get_ordered_visual_children(scope) {
            let Some(child_input_element) = child_visual.cast::<InputElement>() else {
                Self::build_tab_order_entries(&child_visual, entries, false);
                continue;
            };

            let mode = KeyboardNavigation::get_tab_navigation(&child_input_element);
            match mode {
                KeyboardNavigationMode::None => {
                    let target = Self::can_participate_in_tab_navigation(&child_input_element)
                        .then(|| child_input_element.clone());
                    entries.push(TabOrderEntry { target, owner: child_input_element });
                }
                KeyboardNavigationMode::Once => {
                    let target = match Self::resolve_tab_once_target(&child_input_element) {
                        Some(once_target) => Some(once_target),
                        None => Self::can_participate_in_tab_navigation(&child_input_element)
                            .then(|| child_input_element.clone()),
                    };
                    entries.push(TabOrderEntry { target, owner: child_input_element });
                }
                _ => Self::build_tab_order_entries(&child_input_element, entries, true),
            }
        }
    }

    fn resolve_tab_once_target(container: &InputElement) -> Option<Ref<InputElement>> {
        if let Some(active_element) = KeyboardNavigation::get_tab_once_active_element(container) {
            if Self::is_within_scope(container, &active_element) && Self::can_participate_in_tab_navigation(&active_element)
            {
                return Some(active_element);
            }
        }

        let mut child_entries = Vec::new();
        for child_visual in Self::get_ordered_visual_children(container) {
            let include_self = child_visual.is::<InputElement>();
            Self::build_tab_order_entries(&child_visual, &mut child_entries, include_self);
        }

        Self::find_boundary_in(&child_entries, NavigationDirection::Next)
    }

    /// The visual children of the scope by tab index (children that are not input elements
    /// last), then in their order.
    fn get_ordered_visual_children(scope: &Visual) -> Vec<Ref<Visual>> {
        let mut children: Vec<(Ref<Visual>, usize)> =
            scope.get_visual_children().iter().cloned().enumerate().map(|(order, child)| (child, order)).collect();
        // A stable sort keeps the order of the children of the same tab index (`ThenBy(Order)`).
        children.sort_by_key(|(child, _)| {
            child.cast::<InputElement>().map_or(i32::MAX, |input_element| input_element.tab_index())
        });
        children.into_iter().map(|(child, _)| child).collect()
    }

    fn can_participate_in_tab_navigation(element: &InputElement) -> bool {
        element.focusable() && element.is_tab_stop() && element.is_effectively_enabled() && element.is_effectively_visible()
    }

    fn find_boundary_in(entries: &[TabOrderEntry], direction: NavigationDirection) -> Option<Ref<InputElement>> {
        if direction == NavigationDirection::Previous {
            return entries.iter().rev().find_map(|entry| entry.target.clone());
        }

        entries.iter().find_map(|entry| entry.target.clone())
    }

    fn is_within_scope(scope: &Visual, candidate: &InputElement) -> bool {
        let candidate_visual: &Visual = candidate;
        std::ptr::addr_eq(scope as *const Visual, candidate_visual as *const Visual)
            || scope.is_visual_ancestor_of(candidate_visual)
    }
}
