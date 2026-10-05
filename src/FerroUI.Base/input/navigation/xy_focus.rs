use super::{
    XYFocusAlgorithms, XYFocusHelpers, XYFocusManifolds, XYFocusNavigationModes, XYFocusNavigationStrategy,
    XYFocusOptions,
};
use crate::input::{FocusManager, InputElement, KeyDeviceType, NavigationDirection};
use crate::media::FlowDirection;
use crate::utilities::MathUtilities;
use crate::{
    ferro_property, AttachedProperty, FerroProperty, Rect, Ref, StyledPropertyOptions, Visual,
};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;

type Element = Ref<InputElement>;

/// A candidate of a directional focus search.
struct XYFocusParams {
    element: Element,
    bounds: Rect,
    score: f64,
}

thread_local! {
    static INSTANCE: XYFocus = XYFocus::new();
}

/// Directional (XY) focus navigation: the attached properties that
/// configure it and the search for the next element in a direction.
pub struct XYFocus {
    manifolds: Cell<XYFocusManifolds>,
    pooled_candidates: RefCell<Vec<XYFocusParams>>,
}

crate::ferro_static_type!(XYFocus);

macro_rules! override_property {
    ($(#[$meta:meta])* $property:ident, $name:literal, $get:ident, $set:ident) => {
        ferro_property!(for XYFocus;
            $(#[$meta])*
            pub fn $property() -> AttachedProperty<Option<Ref<InputElement>>> {
                FerroProperty::register_attached::<XYFocus, InputElement, _>($name, None)
            }
        );

        pub fn $set(obj: &InputElement, value: Option<Ref<InputElement>>) {
            obj.set_value(Self::$property(), value)
        }

        pub fn $get(obj: &InputElement) -> Option<Ref<InputElement>> {
            obj.get_value(Self::$property())
        }
    };
}

macro_rules! strategy_property {
    ($(#[$meta:meta])* $property:ident, $name:literal, $get:ident, $set:ident) => {
        ferro_property!(for XYFocus;
            $(#[$meta])*
            pub fn $property() -> AttachedProperty<XYFocusNavigationStrategy> {
                FerroProperty::register_attached_with::<XYFocus, InputElement, _>(
                    $name,
                    StyledPropertyOptions::new(XYFocusNavigationStrategy::Auto).inherits(true),
                )
            }
        );

        pub fn $set(obj: &InputElement, value: XYFocusNavigationStrategy) {
            obj.set_value(Self::$property(), value)
        }

        pub fn $get(obj: &InputElement) -> XYFocusNavigationStrategy {
            obj.get_value(Self::$property())
        }
    };
}

crate::ferro_properties! {
    impl XYFocus, also [
        XYFocus::down_property,
        XYFocus::left_property,
        XYFocus::right_property,
        XYFocus::up_property,
        XYFocus::down_navigation_strategy_property,
        XYFocus::up_navigation_strategy_property,
        XYFocus::left_navigation_strategy_property,
        XYFocus::right_navigation_strategy_property,
        XYFocus::navigation_modes_property,
        XYFocus::is_focus_engagement_enabled_property,
        XYFocus::is_focus_engaged_property,
    ] {}
}

impl XYFocus {
    // --- properties ---------------------------------------------------------

    override_property!(
        /// The object that gets focus when a user presses the Dpad down.
        down_property, "Down", get_down, set_down
    );

    override_property!(
        /// The object that gets focus when a user presses the Dpad left.
        left_property, "Left", get_left, set_left
    );

    override_property!(
        /// The object that gets focus when a user presses the Dpad right.
        right_property, "Right", get_right, set_right
    );

    override_property!(
        /// The object that gets focus when a user presses the Dpad up.
        up_property, "Up", get_up, set_up
    );

    strategy_property!(
        /// The strategy used to determine the target element of a down
        /// navigation.
        down_navigation_strategy_property, "DownNavigationStrategy",
        get_down_navigation_strategy, set_down_navigation_strategy
    );

    strategy_property!(
        /// The strategy used to determine the target element of an up
        /// navigation.
        up_navigation_strategy_property, "UpNavigationStrategy",
        get_up_navigation_strategy, set_up_navigation_strategy
    );

    strategy_property!(
        /// The strategy used to determine the target element of a left
        /// navigation.
        left_navigation_strategy_property, "LeftNavigationStrategy",
        get_left_navigation_strategy, set_left_navigation_strategy
    );

    strategy_property!(
        /// The strategy used to determine the target element of a right
        /// navigation.
        right_navigation_strategy_property, "RightNavigationStrategy",
        get_right_navigation_strategy, set_right_navigation_strategy
    );

    ferro_property!(for XYFocus;
        /// Defines the XY focus navigation modes that are enabled for the
        /// element.
        pub fn navigation_modes_property() -> AttachedProperty<XYFocusNavigationModes> {
            FerroProperty::register_attached_with::<XYFocus, InputElement, _>(
                "NavigationModes",
                StyledPropertyOptions::new(XYFocusNavigationModes::GAMEPAD | XYFocusNavigationModes::REMOTE)
                    .inherits(true),
            )
        }
    );

    pub fn set_navigation_modes(obj: &InputElement, value: XYFocusNavigationModes) {
        obj.set_value(Self::navigation_modes_property(), value)
    }

    pub fn get_navigation_modes(obj: &InputElement) -> XYFocusNavigationModes {
        obj.get_value(Self::navigation_modes_property())
    }

    ferro_property!(for XYFocus;
        pub(crate) fn is_focus_engagement_enabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<XYFocus, InputElement, _>("IsFocusEngagementEnabled", false)
        }
    );

    #[allow(dead_code)]
    pub(crate) fn set_is_focus_engagement_enabled(obj: &InputElement, value: bool) {
        obj.set_value(Self::is_focus_engagement_enabled_property(), value)
    }

    pub(crate) fn get_is_focus_engagement_enabled(obj: &InputElement) -> bool {
        obj.get_value(Self::is_focus_engagement_enabled_property())
    }

    ferro_property!(for XYFocus;
        pub(crate) fn is_focus_engaged_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<XYFocus, Visual, _>(
                "IsFocusEngaged",
                StyledPropertyOptions::new(false).coerce(|sender, value| {
                    value
                        && sender
                            .downcast_ref::<InputElement>()
                            .is_some_and(XYFocus::get_is_focus_engagement_enabled)
                }),
            )
        }
    );

    #[allow(dead_code)]
    pub(crate) fn set_is_focus_engaged(obj: &Visual, value: bool) {
        obj.set_value(Self::is_focus_engaged_property(), value)
    }

    pub(crate) fn get_is_focus_engaged(obj: &Visual) -> bool {
        obj.get_value(Self::is_focus_engaged_property())
    }

    // --- find elements ------------------------------------------------------

    fn find_elements(
        focus_list: &mut Vec<XYFocusParams>,
        start_root: &InputElement,
        current_element: Option<&Element>,
        active_scroller: Option<&Element>,
        ignore_clipping: bool,
        input_key_device_type: Option<KeyDeviceType>,
    ) {
        let is_scrolling = active_scroller.is_some();
        let Some(collection) = start_root.visual_children_snapshot() else { return };

        for child in collection.iter() {
            let Some(child) = child.cast::<InputElement>() else { continue };

            let is_engagement_enabled_but_not_engaged =
                Self::get_is_focus_engagement_enabled(&child) && !Self::get_is_focus_engaged(&child);

            if Some(&child) != current_element && Self::is_valid_candidate(&child, input_key_device_type) {
                if let Some(bounds) = Self::get_bounds_for_ranking(&child, ignore_clipping) {
                    if is_scrolling {
                        if Self::is_candidate_participating_in_scroll(&child, active_scroller)
                            || !Self::is_occluded(&child, bounds)
                            || Self::is_candidate_child_of_ancestor_scroller(&child, active_scroller)
                        {
                            focus_list.push(XYFocusParams { element: child.clone(), bounds, score: 0.0 });
                        }
                    } else {
                        focus_list.push(XYFocusParams { element: child.clone(), bounds, score: 0.0 });
                    }
                }
            }

            if Self::is_valid_focus_subtree(&child) && !is_engagement_enabled_but_not_engaged {
                Self::find_elements(
                    focus_list,
                    &child,
                    current_element,
                    active_scroller,
                    ignore_clipping,
                    input_key_device_type,
                );
            }
        }
    }

    fn is_valid_focus_subtree(candidate: &InputElement) -> bool {
        // We don't need to check for effective values, as we've already
        // checked parents of this subtree on previous steps.
        candidate.is_visible() && candidate.is_enabled()
    }

    fn is_valid_candidate(candidate: &InputElement, input_key_device_type: Option<KeyDeviceType>) -> bool {
        candidate.focusable()
            && candidate.is_effectively_enabled()
            && candidate.is_effectively_visible()
            // Only allow candidate focus, if original key device type could
            // focus it.
            && XYFocusHelpers::is_allowed_xy_navigation_mode(candidate, input_key_device_type)
    }

    fn is_candidate_participating_in_scroll(candidate: &Element, active_scroller: Option<&Element>) -> bool {
        let Some(active_scroller) = active_scroller else { return false };

        let closest_scroller = candidate.find_ancestor_of_type_where::<InputElement>(true, |e| e.as_scrollable().is_some());
        closest_scroller.as_ref() == Some(active_scroller)
    }

    fn is_candidate_child_of_ancestor_scroller(candidate: &Element, active_scroller: Option<&Element>) -> bool {
        let Some(active_scroller) = active_scroller else { return false };

        let mut parent = active_scroller.parent();
        while let Some(current) = parent {
            if let Some(element) = current.downcast_ref::<InputElement>() {
                if element.as_scrollable().is_some() && element.is_visual_ancestor_of(candidate) {
                    return true;
                }
            }
            parent = current.parent();
        }

        false
    }

    fn is_occluded(element: &InputElement, element_bounds: Rect) -> bool {
        let Some(root) = element.visual_root() else { return true };

        // Check if the element is within the visible area of the window
        let visible_bounds = Rect::new(0.0, 0.0, root.bounds().width, root.bounds().height);

        !visible_bounds.intersects(element_bounds)
    }

    pub(crate) fn get_bounds_for_ranking(element: &InputElement, ignore_clipping: bool) -> Option<Rect> {
        element.get_transformed_bounds().map(|bounds| {
            if ignore_clipping {
                bounds.bounds.transform_to_aabb(bounds.transform)
            } else {
                bounds.clip
            }
        })
    }

    // --- bubbling -----------------------------------------------------------

    fn get_direction_override(
        element: &InputElement,
        search_root: Option<&Element>,
        direction: NavigationDirection,
        ignore_focusability: bool,
    ) -> Option<Element> {
        let index = Self::get_xy_focus_property_index(element, direction)?;
        let override_element = element.get_value(index)?;

        if !ignore_focusability && !FocusManager::can_focus(&override_element) {
            return None;
        }

        // If an override was specified but it is located outside the
        // search root, don't use it as the candidate.
        if let Some(search_root) = search_root {
            if !search_root.is_visual_ancestor_of(&override_element) {
                return None;
            }
        }

        Some(override_element)
    }

    fn try_xy_focus_bubble(
        element: &Element,
        candidate: Option<Element>,
        search_root: Option<&Element>,
        direction: NavigationDirection,
    ) -> Option<Element> {
        let candidate = candidate?;
        let mut next_focusable_element = candidate.clone();

        if let Some(direction_override_root) = Self::get_direction_override_root(element, search_root, direction) {
            let is_ancestor = direction_override_root.is_visual_ancestor_of(&candidate);
            if !is_ancestor {
                if let Some(overridden) =
                    Self::get_direction_override(&direction_override_root, search_root, direction, false)
                {
                    next_focusable_element = overridden;
                }
            }
        }

        Some(next_focusable_element)
    }

    fn get_direction_override_root(
        element: &Element,
        search_root: Option<&Element>,
        direction: NavigationDirection,
    ) -> Option<Element> {
        let mut root = Some(element.clone());

        while let Some(current) = &root {
            if Self::get_direction_override(current, search_root, direction, false).is_some() {
                break;
            }
            root = current.get_visual_parent_of_type::<InputElement>();
        }

        root
    }

    fn get_strategy(
        element: &Element,
        direction: NavigationDirection,
        navigation_strategy_override: Option<XYFocusNavigationStrategy>,
    ) -> XYFocusNavigationStrategy {
        let is_auto_override = navigation_strategy_override == Some(XYFocusNavigationStrategy::Auto);
        let mut element = element.clone();

        if let Some(strategy) = navigation_strategy_override.filter(|_| !is_auto_override) {
            // The override is converted by offsetting its value, as
            // upstream does.
            return match strategy as i32 - 1 {
                1 => XYFocusNavigationStrategy::Projection,
                2 => XYFocusNavigationStrategy::NavigationDirectionDistance,
                3 => XYFocusNavigationStrategy::RectilinearDistance,
                _ => XYFocusNavigationStrategy::Auto,
            };
        } else if is_auto_override {
            // Skip the element if we have an auto override and look at its
            // parent's strategy
            if let Some(parent) = element.get_visual_parent_of_type::<InputElement>() {
                element = parent;
            }
        }

        if let Some(index) = Self::get_xy_focus_navigation_strategy_property_index(&element, direction) {
            let mut current = Some(element);
            while let Some(c) = current {
                let mode = c.get_value(index);
                if mode != XYFocusNavigationStrategy::Auto {
                    return mode;
                }

                current = c.get_visual_parent_of_type::<InputElement>();
            }
        }

        XYFocusNavigationStrategy::Projection
    }

    fn flip_for_flow_direction(element: &InputElement, direction: NavigationDirection) -> NavigationDirection {
        if element.flow_direction() == FlowDirection::RightToLeft {
            match direction {
                NavigationDirection::Left => return NavigationDirection::Right,
                NavigationDirection::Right => return NavigationDirection::Left,
                _ => {}
            }
        }
        direction
    }

    fn get_xy_focus_property_index(
        element: &InputElement,
        direction: NavigationDirection,
    ) -> Option<&'static AttachedProperty<Option<Ref<InputElement>>>> {
        match Self::flip_for_flow_direction(element, direction) {
            NavigationDirection::Left => Some(Self::left_property()),
            NavigationDirection::Right => Some(Self::right_property()),
            NavigationDirection::Up => Some(Self::up_property()),
            NavigationDirection::Down => Some(Self::down_property()),
            _ => None,
        }
    }

    fn get_xy_focus_navigation_strategy_property_index(
        element: &InputElement,
        direction: NavigationDirection,
    ) -> Option<&'static AttachedProperty<XYFocusNavigationStrategy>> {
        match Self::flip_for_flow_direction(element, direction) {
            NavigationDirection::Left => Some(Self::left_navigation_strategy_property()),
            NavigationDirection::Right => Some(Self::right_navigation_strategy_property()),
            NavigationDirection::Up => Some(Self::up_navigation_strategy_property()),
            NavigationDirection::Down => Some(Self::down_navigation_strategy_property()),
            _ => None,
        }
    }

    // --- implementation -----------------------------------------------------

    pub(crate) fn new() -> Self {
        Self { manifolds: Cell::new(XYFocusManifolds::new()), pooled_candidates: RefCell::new(Vec::new()) }
    }

    pub(crate) fn set_manifolds_from_bounds(&self, bounds: Rect) {
        self.manifolds.set(XYFocusManifolds {
            v_manifold: (bounds.left(), bounds.right()),
            h_manifold: (bounds.top(), bounds.bottom()),
        });
    }

    pub(crate) fn update_manifolds(
        &self,
        direction: NavigationDirection,
        element_bounds: Rect,
        candidate: &InputElement,
        ignore_clipping: bool,
    ) {
        let candidate_bounds = Self::get_bounds_for_ranking(candidate, ignore_clipping)
            .expect("the candidate of a completed focus change has bounds");
        let mut manifolds = self.manifolds.get();
        XYFocusAlgorithms::update_manifolds(direction, element_bounds, candidate_bounds, &mut manifolds);
        self.manifolds.set(manifolds);
    }

    /// Finds the element directional navigation moves to from `element`,
    /// always using the bounds of the current element as the manifolds.
    pub(crate) fn try_directional_focus(
        direction: NavigationDirection,
        element: &Element,
        _owner: Option<&Element>,
        engaged_control: Option<&Element>,
        key_device_type: Option<KeyDeviceType>,
    ) -> Option<Element> {
        if !XYFocusHelpers::is_allowed_xy_navigation_mode(element, key_device_type) {
            return None;
        }

        let bounds = Self::get_bounds_for_ranking(element, true)?;
        let search_root = XYFocusHelpers::find_xy_search_root(element, key_device_type);

        INSTANCE.with(|instance| {
            instance.set_manifolds_from_bounds(bounds);

            let mut options = XYFocusOptions::new();
            options.key_device_type = key_device_type;
            options.focused_element_bounds = Some(bounds);
            options.update_manifold = true;
            options.search_root = Some(search_root);

            instance.get_next_focusable_element(direction, Some(element), engaged_control, true, &options)
        })
    }

    pub(crate) fn get_next_focusable_element(
        &self,
        direction: NavigationDirection,
        element: Option<&Element>,
        engaged_control: Option<&Element>,
        update_manifolds: bool,
        xy_focus_options: &XYFocusOptions,
    ) -> Option<Element> {
        let element = element?;
        let root = element.visual_root().and_then(|root| root.downcast::<InputElement>().ok())?;

        let is_right_to_left = element.flow_direction() == FlowDirection::RightToLeft;
        let mode = Self::get_strategy(element, direction, xy_focus_options.navigation_strategy_override);

        let mut focused_element_bounds =
            xy_focus_options.focused_element_bounds.expect("FocusedElementBounds needs to be set");

        let search_root = xy_focus_options.search_root.as_ref();

        if let Some(next) = Self::get_direction_override(element, search_root, direction, true) {
            return Some(next);
        }

        let active_scroller = Self::get_active_scroller_for_scroll(direction, element);
        let is_processing_input_for_scroll = active_scroller.is_some();
        let mut element = Some(element);

        if let Some(hint) = xy_focus_options.focus_hint_rectangle {
            focused_element_bounds = hint;
            element = None;
        }

        let ignore_clipping = xy_focus_options.ignore_clipping;
        let root_bounds = match (engaged_control, search_root) {
            (Some(engaged), _) => Self::get_bounds_for_ranking(engaged, ignore_clipping),
            (None, Some(search_root)) => Self::get_bounds_for_ranking(search_root, ignore_clipping),
            (None, None) => Self::get_bounds_for_ranking(&root, ignore_clipping),
        }
        .unwrap_or_else(|| root.bounds());

        // The candidate list is reused between searches; a nested search
        // (from user code run by a property getter) gets its own.
        let mut candidate_list = self.pooled_candidates.try_borrow_mut().map(|mut list| std::mem::take(&mut *list)).unwrap_or_default();
        let mut next_focusable_element = None;

        Self::get_all_valid_focusable_children(
            &mut candidate_list,
            &root,
            element,
            engaged_control,
            search_root,
            active_scroller.as_ref(),
            ignore_clipping,
            xy_focus_options.key_device_type,
        );

        if !candidate_list.is_empty() {
            let mut max_root_bounds_distance =
                (root_bounds.right() - root_bounds.left()).max(root_bounds.bottom() - root_bounds.top());
            max_root_bounds_distance = max_root_bounds_distance.max(Self::get_max_root_bounds_distance(
                &candidate_list,
                focused_element_bounds,
                direction,
            ));

            self.rank_elements(
                &mut candidate_list,
                direction,
                focused_element_bounds,
                max_root_bounds_distance,
                mode,
                xy_focus_options.exclusion_rect,
                xy_focus_options.ignore_cone,
            );

            let ignore_occlusivity = xy_focus_options.ignore_occlusivity || is_processing_input_for_scroll;

            // Choose the best candidate, after testing for occlusivity; if
            // we're currently scrolling, the test has been done already,
            // skip it.
            next_focusable_element = self.choose_best_focusable_element_from_list(
                &mut candidate_list,
                direction,
                focused_element_bounds,
                ignore_clipping,
                ignore_occlusivity,
                is_right_to_left,
                xy_focus_options.update_manifold && update_manifolds,
            );

            if let Some(element) = element {
                next_focusable_element =
                    Self::try_xy_focus_bubble(element, next_focusable_element, search_root, direction);
            }
        }

        candidate_list.clear();
        if let Ok(mut pooled) = self.pooled_candidates.try_borrow_mut() {
            *pooled = candidate_list;
        }

        next_focusable_element
    }

    #[allow(clippy::too_many_arguments)]
    fn choose_best_focusable_element_from_list(
        &self,
        score_list: &mut [XYFocusParams],
        direction: NavigationDirection,
        bounds: Rect,
        ignore_clipping: bool,
        ignore_occlusivity: bool,
        is_right_to_left: bool,
        update_manifolds: bool,
    ) -> Option<Element> {
        let compare = |a: f64, b: f64| a.partial_cmp(&b).unwrap_or(Ordering::Equal);

        score_list.sort_by(|element_a, element_b| {
            if element_a.element == element_b.element {
                return Ordering::Equal;
            }

            let compared = compare(element_b.score, element_a.score);
            if compared == Ordering::Equal {
                let first_bounds = element_a.bounds;
                let second_bounds = element_b.bounds;

                if first_bounds == second_bounds {
                    Ordering::Equal
                } else if direction == NavigationDirection::Up || direction == NavigationDirection::Down {
                    if is_right_to_left {
                        compare(second_bounds.left(), first_bounds.left())
                    } else {
                        compare(first_bounds.left(), second_bounds.left())
                    }
                } else {
                    compare(first_bounds.top(), second_bounds.top())
                }
            } else {
                compared
            }
        });

        for param in score_list.iter() {
            if param.score <= 0.0 {
                break;
            }

            let bounds_for_occ_testing = if ignore_clipping {
                Self::get_bounds_for_ranking(&param.element, false).unwrap_or(param.bounds)
            } else {
                param.bounds
            };

            // Don't check for occlusivity if we've already covered
            // occlusivity scenarios for scrollable content or have been
            // asked to ignore occlusivity by the caller.
            if (param.bounds.x - f64::MAX).abs() > MathUtilities::DOUBLE_EPSILON
                && (ignore_occlusivity || !Self::is_occluded(&param.element, bounds_for_occ_testing))
            {
                if update_manifolds {
                    // Update the manifolds with the newly selected focus
                    let mut manifolds = self.manifolds.get();
                    XYFocusAlgorithms::update_manifolds(direction, bounds, param.bounds, &mut manifolds);
                    self.manifolds.set(manifolds);
                }

                return Some(param.element.clone());
            }
        }

        None
    }

    #[allow(clippy::too_many_arguments)]
    fn get_all_valid_focusable_children(
        candidate_list: &mut Vec<XYFocusParams>,
        start_root: &Element,
        current_element: Option<&Element>,
        engaged_control: Option<&Element>,
        search_scope: Option<&Element>,
        active_scroller: Option<&Element>,
        ignore_clipping: bool,
        input_key_device_type: Option<KeyDeviceType>,
    ) {
        // If asked to scope the search within the given container, honor it
        // without any exceptions
        let root_for_tree_walk = search_scope.unwrap_or(start_root);

        match engaged_control {
            None => Self::find_elements(
                candidate_list,
                root_for_tree_walk,
                current_element,
                active_scroller,
                ignore_clipping,
                input_key_device_type,
            ),
            Some(engaged_control) => {
                // Only run through this when you are an engaged element.
                // Being an engaged element means that you should only look
                // at the children of the engaged element.
                Self::find_elements(
                    candidate_list,
                    engaged_control,
                    current_element,
                    active_scroller,
                    ignore_clipping,
                    input_key_device_type,
                );

                if current_element != Some(engaged_control) {
                    if let Some(bounds) = Self::get_bounds_for_ranking(engaged_control, ignore_clipping) {
                        candidate_list.push(XYFocusParams { element: engaged_control.clone(), bounds, score: 0.0 });
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn rank_elements(
        &self,
        candidate_list: &mut [XYFocusParams],
        direction: NavigationDirection,
        bounds: Rect,
        max_root_bounds_distance: f64,
        mode: XYFocusNavigationStrategy,
        exclusion_bounds: Rect,
        ignore_cone: bool,
    ) {
        let manifolds = self.manifolds.get();

        for candidate in candidate_list.iter_mut() {
            let candidate_bounds = candidate.bounds;

            if !(exclusion_bounds.intersects(candidate_bounds) || exclusion_bounds.contains_rect(candidate_bounds)) {
                if mode == XYFocusNavigationStrategy::Projection
                    && XYFocusAlgorithms::should_candidate_be_considered_for_ranking(
                        bounds,
                        candidate_bounds,
                        max_root_bounds_distance,
                        direction,
                        exclusion_bounds,
                        ignore_cone,
                    )
                {
                    candidate.score = XYFocusAlgorithms::get_score_projection(
                        direction,
                        bounds,
                        candidate_bounds,
                        &manifolds,
                        max_root_bounds_distance,
                    );
                } else if mode == XYFocusNavigationStrategy::NavigationDirectionDistance
                    || mode == XYFocusNavigationStrategy::RectilinearDistance
                {
                    candidate.score = XYFocusAlgorithms::get_score_proximity(
                        direction,
                        bounds,
                        candidate_bounds,
                        max_root_bounds_distance,
                        mode == XYFocusNavigationStrategy::RectilinearDistance,
                    );
                }
            }
        }
    }

    fn get_max_root_bounds_distance(list: &[XYFocusParams], bounds: Rect, direction: NavigationDirection) -> f64 {
        let mut max_element = &list[0];
        let mut max_value = f64::MIN;

        for param in list {
            let candidate_bounds = param.bounds;
            let value = match direction {
                NavigationDirection::Left => candidate_bounds.left(),
                NavigationDirection::Right => candidate_bounds.right(),
                NavigationDirection::Up => candidate_bounds.top(),
                NavigationDirection::Down => candidate_bounds.bottom(),
                _ => 0.0,
            };

            if value > max_value {
                max_value = value;
                max_element = param;
            }
        }

        let max_bounds = max_element.bounds;
        match direction {
            NavigationDirection::Left => (max_bounds.right() - bounds.left()).abs(),
            NavigationDirection::Right => (bounds.right() - max_bounds.left()).abs(),
            NavigationDirection::Up => (bounds.bottom() - max_bounds.top()).abs(),
            NavigationDirection::Down => (max_bounds.bottom() - bounds.top()).abs(),
            _ => 0.0,
        }
    }

    fn get_active_scroller_for_scroll(direction: NavigationDirection, focused_element: &Element) -> Option<Element> {
        let mut parent = Some(focused_element.clone());

        while let Some(element) = parent {
            if let Some(scrollable) = element.as_scrollable() {
                let is_horizontally_scrollable_for_direction =
                    matches!(direction, NavigationDirection::Left | NavigationDirection::Right)
                        && scrollable.can_horizontally_scroll();
                let is_vertically_scrollable_for_direction =
                    matches!(direction, NavigationDirection::Up | NavigationDirection::Down)
                        && scrollable.can_vertically_scroll();

                if is_horizontally_scrollable_for_direction || is_vertically_scrollable_for_direction {
                    return Some(element);
                }
            }

            parent = element.get_visual_parent_of_type::<InputElement>();
        }

        None
    }
}
