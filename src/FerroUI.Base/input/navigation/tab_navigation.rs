use crate::input::{
    FocusHelpers, FocusManager, InputElement, KeyboardNavigation, KeyboardNavigationMode,
};
use crate::Ref;

type Element = Ref<InputElement>;

/// The implementation for default tab navigation.
pub(crate) struct TabNavigation;

impl TabNavigation {
    /// Gets the next control in the specified tab direction, starting the
    /// search in the group that contains `e`.
    pub fn get_next_tab(e: &Element, go_down_only: bool) -> Option<Element> {
        Self::get_next_tab_in(Some(e), &Self::get_group_parent(e), go_down_only)
    }

    pub fn get_next_tab_in(e: Option<&Element>, container: &Element, go_down_only: bool) -> Option<Element> {
        let tabbing_type = Self::get_key_navigation_mode(container);

        match e {
            None => {
                if Self::is_tab_stop(container) {
                    return Some(container.clone());
                }

                // Using ActiveElement if set
                if let Some(active_element) = Self::get_active_element(container) {
                    return Self::get_next_tab_in(None, &active_element, true);
                }
            }
            Some(e) => {
                if (tabbing_type == KeyboardNavigationMode::Once || tabbing_type == KeyboardNavigationMode::None)
                    && container != e
                {
                    if go_down_only {
                        return None;
                    }
                    let parent_container = Self::get_group_parent(container);
                    return Self::get_next_tab_in(Some(container), &parent_container, go_down_only);
                }
            }
        }

        // All groups
        let mut loop_start_element: Option<Element> = None;
        let mut next_tab_element = e.cloned();
        let mut current_tabbing_type = tabbing_type;

        // Search down inside the container
        loop {
            next_tab_element = Self::get_next_tab_in_group(next_tab_element.as_ref(), container, current_tabbing_type);
            let Some(next) = &next_tab_element else { break };

            // Avoid the endless loop here for Cycle groups
            if loop_start_element.as_ref() == Some(next) {
                break;
            }
            if loop_start_element.is_none() {
                loop_start_element = Some(next.clone());
            }

            if let Some(first_tab_element_inside) = Self::get_next_tab_in(None, next, true) {
                return Some(first_tab_element_inside);
            }

            // If we want to continue searching inside the Once groups, we
            // should change the navigation mode
            if current_tabbing_type == KeyboardNavigationMode::Once {
                current_tabbing_type = KeyboardNavigationMode::Contained;
            }
        }

        // If there is no next element in the group (next_tab_element is
        // none), search up in the tree if allowed.
        if !go_down_only
            && current_tabbing_type != KeyboardNavigationMode::Contained
            && Self::get_parent(container).is_some()
        {
            return Self::get_next_tab_in(Some(container), &Self::get_group_parent(container), false);
        }

        None
    }

    pub fn get_next_tab_outside(e: &Element) -> Option<Element> {
        let last = Self::get_last_in_tree(e);
        Self::get_next_tab(&last, false)
    }

    pub fn get_prev_tab(e: Option<&Element>, container: Option<&Element>, go_down_only: bool) -> Option<Element> {
        let container = match container {
            Some(container) => container.clone(),
            None => Self::get_group_parent(e.expect("Either 'e' or 'container' must be non-null.")),
        };
        let container = &container;

        let tabbing_type = Self::get_key_navigation_mode(container);

        match e {
            None => {
                // Using ActiveElement if set
                if let Some(active_element) = Self::get_active_element(container) {
                    return Self::get_prev_tab(None, Some(&active_element), true);
                }

                // If we Shift+Tab on a container with
                // KeyboardNavigationMode=Once, and ActiveElement is null then
                // we want to go to the first item (not last) within the
                // container
                if tabbing_type == KeyboardNavigationMode::Once {
                    return match Self::get_next_tab_in_group(None, container, tabbing_type) {
                        None => {
                            if Self::is_tab_stop(container) {
                                return Some(container.clone());
                            }
                            if go_down_only {
                                return None;
                            }
                            Self::get_prev_tab(Some(container), None, false)
                        }
                        Some(first_tab_element) => Self::get_prev_tab(None, Some(&first_tab_element), true),
                    };
                }
            }
            Some(e) => {
                if tabbing_type == KeyboardNavigationMode::Once || tabbing_type == KeyboardNavigationMode::None {
                    if go_down_only || container == e {
                        return None;
                    }

                    // FocusedElement should not be e otherwise we will
                    // delegate focus to the same element
                    if Self::is_tab_stop(container) {
                        return Some(container.clone());
                    }

                    return Self::get_prev_tab(Some(container), None, false);
                }
            }
        }

        // All groups (except Once) - continue
        let mut loop_start_element: Option<Element> = None;
        let mut next_tab_element = e.cloned();

        // Look for element with the same TabIndex before the current element
        loop {
            next_tab_element = Self::get_prev_tab_in_group(next_tab_element.as_ref(), container, tabbing_type);
            let Some(next) = &next_tab_element else { break };

            if next == container && tabbing_type == KeyboardNavigationMode::Local {
                break;
            }

            // At this point next is TabStop or TabGroup. In case it is a
            // TabStop only return the element
            if Self::is_tab_stop(next) && !Self::is_group(next) {
                return Some(next.clone());
            }

            // Avoid the endless loop here
            if loop_start_element.as_ref() == Some(next) {
                break;
            }
            if loop_start_element.is_none() {
                loop_start_element = Some(next.clone());
            }

            // At this point next is TabGroup
            if let Some(last_tab_element_inside) = Self::get_prev_tab(None, Some(next), true) {
                return Some(last_tab_element_inside);
            }
        }

        if tabbing_type == KeyboardNavigationMode::Contained {
            return None;
        }

        if e != Some(container) && Self::is_tab_stop(container) {
            return Some(container.clone());
        }

        // If end of the subtree is reached or there no other elements above
        if !go_down_only && Self::get_parent(container).is_some() {
            return Self::get_prev_tab(Some(container), None, false);
        }

        None
    }

    pub fn get_prev_tab_outside(e: &Element) -> Option<Element> {
        let first = Self::get_first_child(e)?;
        Self::get_prev_tab(Some(&first), None, false)
    }

    fn focused_element(e: &Element) -> Option<Element> {
        // Focus delegation is enabled only if keyboard focus is outside the
        // container
        if !e.is_keyboard_focus_within() && e.is_focus_scope() {
            let focus_manager = FocusManager::get_focus_manager(e);
            let focused_element = focus_manager.and_then(|m| m.get_focused_element_in_scope(e));

            if let Some(focused_element) = focused_element {
                // Verify if focused_element is a visual descendant of e
                if !Self::is_focus_scope(e) && focused_element != *e && e.is_visual_ancestor_of(&focused_element) {
                    return Some(focused_element);
                }
            }
        }

        None
    }

    fn get_first_child(e: &Element) -> Option<Element> {
        // If the element has a FocusedElement it should be its first child
        if let Some(focused_element) = Self::focused_element(e) {
            return Some(focused_element);
        }

        // Return the first visible element.
        if Self::is_visible_and_enabled(e) {
            for ie in FocusHelpers::get_input_element_children(e) {
                if Self::is_visible_and_enabled(&ie) {
                    return Some(ie);
                }
                if let Some(first_child) = Self::get_first_child(&ie) {
                    return Some(first_child);
                }
            }
        }

        None
    }

    fn get_last_child(e: &Element) -> Option<Element> {
        // If the element has a FocusedElement it should be its last child
        if let Some(focused_element) = Self::focused_element(e) {
            return Some(focused_element);
        }

        // Return the last visible element.
        if Self::is_visible_and_enabled(e) {
            for ie in FocusHelpers::get_input_element_children(e).into_iter().rev() {
                if Self::is_visible_and_enabled(&ie) {
                    return Some(ie);
                }
                if let Some(last_child) = Self::get_last_child(&ie) {
                    return Some(last_child);
                }
            }
        }

        None
    }

    pub(crate) fn get_first_tab_in_group(container: &Element) -> Option<Element> {
        let mut first_tab_element: Option<Element> = None;
        let mut min_index_first_tab = i32::MIN;

        let mut curr_element = container.clone();
        while let Some(next) = Self::get_next_in_tree(&curr_element, container) {
            curr_element = next;
            if Self::is_tab_stop_or_group(&curr_element) {
                let curr_priority = KeyboardNavigation::get_tab_index(&curr_element);

                if curr_priority < min_index_first_tab || first_tab_element.is_none() {
                    min_index_first_tab = curr_priority;
                    first_tab_element = Some(curr_element.clone());
                }
            }
        }

        first_tab_element
    }

    fn get_last_in_tree(container: &Element) -> Element {
        let mut result;
        let mut c = Some(container.clone());

        loop {
            result = c.clone().expect("set before each iteration");
            c = Self::get_last_child(&result);
            match &c {
                Some(next) if !Self::is_group(next) => {}
                _ => break,
            }
        }

        c.unwrap_or(result)
    }

    fn get_last_tab_in_group(container: &Element) -> Option<Element> {
        let mut last_tab_element: Option<Element> = None;
        let mut max_index_first_tab = i32::MAX;
        let mut curr_element = Some(Self::get_last_in_tree(container));

        while let Some(curr) = curr_element {
            if curr == *container {
                break;
            }

            if Self::is_tab_stop_or_group(&curr) {
                let curr_priority = KeyboardNavigation::get_tab_index(&curr);

                if curr_priority > max_index_first_tab || last_tab_element.is_none() {
                    max_index_first_tab = curr_priority;
                    last_tab_element = Some(curr.clone());
                }
            }

            curr_element = Self::get_previous_in_tree(&curr, container);
        }

        last_tab_element
    }

    fn get_next_in_tree(e: &Element, container: &Element) -> Option<Element> {
        let mut result = None;

        if e == container || !Self::is_group(e) {
            result = Self::get_first_child(e);
        }

        if result.is_some() || e == container {
            return result;
        }

        let mut parent = Some(e.clone());

        while let Some(current) = parent {
            if let Some(sibling) = Self::get_next_sibling(&current) {
                return Some(sibling);
            }

            parent = Self::get_parent(&current);
            if parent.as_ref() == Some(container) {
                break;
            }
        }

        None
    }

    fn get_next_sibling(e: &Element) -> Option<Element> {
        let parent = Self::get_parent(e)?;
        let children = FocusHelpers::get_input_element_children(&parent);

        // go till itself, then search ahead
        let index = children.iter().position(|child| child == e).unwrap_or(children.len());
        children.into_iter().nth(index + 1)
    }

    fn get_next_tab_in_group(
        e: Option<&Element>,
        container: &Element,
        tabbing_type: KeyboardNavigationMode,
    ) -> Option<Element> {
        // None groups: Tab navigation is not supported
        if tabbing_type == KeyboardNavigationMode::None {
            return None;
        }

        // e == null or e == container -> return the first TabStopOrGroup
        let e = match e {
            Some(e) if e != container => e,
            _ => return Self::get_first_tab_in_group(container),
        };

        if tabbing_type == KeyboardNavigationMode::Once {
            return None;
        }

        Self::get_next_tab_with_same_index(e, container)
            .or_else(|| Self::get_next_tab_with_next_index(e, container, tabbing_type))
    }

    fn get_next_tab_with_same_index(e: &Element, container: &Element) -> Option<Element> {
        let element_tab_priority = KeyboardNavigation::get_tab_index(e);
        let mut curr_element = e.clone();

        while let Some(next) = Self::get_next_in_tree(&curr_element, container) {
            curr_element = next;
            if Self::is_tab_stop_or_group(&curr_element)
                && KeyboardNavigation::get_tab_index(&curr_element) == element_tab_priority
            {
                return Some(curr_element);
            }
        }

        None
    }

    fn get_next_tab_with_next_index(
        e: &Element,
        container: &Element,
        tabbing_type: KeyboardNavigationMode,
    ) -> Option<Element> {
        // Find the next min index in the tree: min (index > currentTabIndex)
        let mut next_tab_element: Option<Element> = None;
        let mut first_tab_element: Option<Element> = None;
        let mut min_index_first_tab = i32::MIN;
        let mut min_index = i32::MIN;
        let element_tab_priority = KeyboardNavigation::get_tab_index(e);

        let mut curr_element = container.clone();
        while let Some(next) = Self::get_next_in_tree(&curr_element, container) {
            curr_element = next;
            if Self::is_tab_stop_or_group(&curr_element) {
                let curr_priority = KeyboardNavigation::get_tab_index(&curr_element);

                if curr_priority > element_tab_priority && (curr_priority < min_index || next_tab_element.is_none()) {
                    min_index = curr_priority;
                    next_tab_element = Some(curr_element.clone());
                }

                if curr_priority < min_index_first_tab || first_tab_element.is_none() {
                    min_index_first_tab = curr_priority;
                    first_tab_element = Some(curr_element.clone());
                }
            }
        }

        // Cycle groups: if not found - return first element
        if tabbing_type == KeyboardNavigationMode::Cycle && next_tab_element.is_none() {
            next_tab_element = first_tab_element;
        }

        next_tab_element
    }

    fn get_prev_tab_in_group(
        e: Option<&Element>,
        container: &Element,
        tabbing_type: KeyboardNavigationMode,
    ) -> Option<Element> {
        // None groups: Tab navigation is not supported
        if tabbing_type == KeyboardNavigationMode::None {
            return None;
        }

        // Search the last index inside the group
        let Some(e) = e else { return Self::get_last_tab_in_group(container) };

        if tabbing_type == KeyboardNavigationMode::Once {
            return None;
        }

        if e == container {
            return None;
        }

        Self::get_prev_tab_with_same_index(e, container)
            .or_else(|| Self::get_prev_tab_with_prev_index(e, container, tabbing_type))
    }

    fn get_prev_tab_with_same_index(e: &Element, container: &Element) -> Option<Element> {
        let element_tab_priority = KeyboardNavigation::get_tab_index(e);
        let mut curr_element = Self::get_previous_in_tree(e, container);

        while let Some(curr) = curr_element {
            if Self::is_tab_stop_or_group(&curr)
                && KeyboardNavigation::get_tab_index(&curr) == element_tab_priority
                && curr != *container
            {
                return Some(curr);
            }

            curr_element = Self::get_previous_in_tree(&curr, container);
        }

        None
    }

    fn get_prev_tab_with_prev_index(
        e: &Element,
        container: &Element,
        tabbing_type: KeyboardNavigationMode,
    ) -> Option<Element> {
        // Find the next max index in the tree: max (index < currentTabIndex)
        let mut last_tab_element: Option<Element> = None;
        let mut next_tab_element: Option<Element> = None;
        let element_tab_priority = KeyboardNavigation::get_tab_index(e);
        let mut max_index_first_tab = i32::MAX;
        let mut max_index = i32::MAX;
        let mut curr_element = Some(Self::get_last_in_tree(container));

        while let Some(curr) = curr_element {
            if Self::is_tab_stop_or_group(&curr) && curr != *container {
                let curr_priority = KeyboardNavigation::get_tab_index(&curr);

                if curr_priority < element_tab_priority && (curr_priority > max_index || next_tab_element.is_none()) {
                    max_index = curr_priority;
                    next_tab_element = Some(curr.clone());
                }

                if curr_priority > max_index_first_tab || last_tab_element.is_none() {
                    max_index_first_tab = curr_priority;
                    last_tab_element = Some(curr.clone());
                }
            }

            curr_element = Self::get_previous_in_tree(&curr, container);
        }

        // Cycle groups: if not found - return first element
        if tabbing_type == KeyboardNavigationMode::Cycle && next_tab_element.is_none() {
            next_tab_element = last_tab_element;
        }

        next_tab_element
    }

    fn get_previous_in_tree(e: &Element, container: &Element) -> Option<Element> {
        if e == container {
            return None;
        }

        match Self::get_previous_sibling(e) {
            Some(result) => {
                if Self::is_group(&result) {
                    Some(result)
                } else {
                    Some(Self::get_last_in_tree(&result))
                }
            }
            None => Self::get_parent(e),
        }
    }

    fn get_previous_sibling(e: &Element) -> Option<Element> {
        let parent = Self::get_parent(e)?;
        let mut prev = None;

        for child in FocusHelpers::get_input_element_children(&parent) {
            if child == *e {
                break;
            }
            if Self::is_visible_and_enabled(&child) {
                prev = Some(child);
            }
        }

        prev
    }

    fn get_active_element(e: &Element) -> Option<Element> {
        e.get_value(KeyboardNavigation::tab_once_active_element_property())
    }

    fn get_group_parent(element: &Element) -> Element {
        // Keep the last non null element. We don't include the current
        // element: start at its parent. If the element is the root, then
        // just return it as the group parent.
        let mut result = element.clone();
        let mut e = Self::get_parent(element);

        while let Some(current) = e {
            if Self::is_group(&current) {
                return current;
            }

            e = Self::get_parent(&current);
            result = current;
        }

        result
    }

    fn get_parent(e: &Element) -> Option<Element> {
        // Go up the visual parent chain until we find an input element.
        e.find_ancestor_of_type::<InputElement>(false)
    }

    fn get_key_navigation_mode(e: &Element) -> KeyboardNavigationMode {
        e.get_value(KeyboardNavigation::tab_navigation_property())
    }

    fn is_focus_scope(e: &Element) -> bool {
        FocusManager::get_is_focus_scope(e) || Self::get_parent(e).is_none()
    }

    fn is_group(e: &Element) -> bool {
        Self::get_key_navigation_mode(e) != KeyboardNavigationMode::Continue
    }

    pub(crate) fn is_tab_stop(e: &InputElement) -> bool {
        e.focusable() && KeyboardNavigation::get_is_tab_stop(e) && e.is_visible() && e.is_effectively_enabled()
    }

    fn is_tab_stop_or_group(e: &Element) -> bool {
        Self::is_tab_stop(e) || Self::is_group(e)
    }

    fn is_visible_and_enabled(e: &InputElement) -> bool {
        e.is_visible() && e.is_effectively_enabled()
    }
}
