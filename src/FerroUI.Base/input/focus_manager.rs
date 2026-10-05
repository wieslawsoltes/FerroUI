use super::navigation::{XYFocus, XYFocusOptions};
use super::{
    FindNextElementOptions, FocusHelpers, IFocusManager, InputElement, KeyboardNavigation, KeyboardNavigationMode,
    NavigationDirection, KeyModifiers, KeyboardDevice, MouseButton, NavigationMethod, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use crate::interactivity::{IRoutedEventArgs, Interactive, RoutingStrategies};
use crate::{
    ferro_property, AttachedProperty, FerroLocator, FerroProperty, LocatorExtensions, Ref, StyledElement, Visual,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type Element = Ref<InputElement>;

/// Manages focus for the application.
pub struct FocusManager {
    this: Weak<FocusManager>,
    xy_focus: XYFocus,
    reusable_focus_options: RefCell<Option<XYFocusOptions>>,
    focus_root: RefCell<Option<Ref<StyledElement>>>,
    content_root: RefCell<Option<Ref<InputElement>>>,
}

crate::ferro_static_type!(FocusManager);

crate::ferro_properties! { impl FocusManager {
    ferro_property!(
        /// The element that currently has focus within a focus scope, set
        /// on the scope.
        ///
        /// A scope does not own its focused element, which can be the scope
        /// itself (a focused menu or top-level), so the value is an element
        /// reference.
        fn focused_element_property() -> AttachedProperty<Option<crate::ElementRef<InputElement>>> {
            crate::data::core::ValueTypes::register_element_ref::<InputElement>();
            FerroProperty::register_attached::<FocusManager, StyledElement, _>("FocusedElement", None)
        }
    );
} }

impl FocusManager {
    /// Registers the class handlers that move focus to the element under a
    /// pointer press or release.
    fn static_constructor() {
        // The class handlers of the input element itself come first.
        InputElement::TYPE.ensure_class_init();

        InputElement::pointer_pressed_event().add_class_handler_untyped(
            InputElement::TYPE,
            Self::on_preview_pointer_event_handler,
            RoutingStrategies::TUNNEL,
            false,
        );
        InputElement::pointer_released_event().add_class_handler_untyped(
            InputElement::TYPE,
            Self::on_preview_pointer_event_handler,
            RoutingStrategies::TUNNEL,
            false,
        );
    }

    /// Creates a focus manager.
    pub fn new() -> Rc<FocusManager> {
        Self::TYPE.ensure_class_init();
        Rc::new_cyclic(|this| FocusManager {
            this: this.clone(),
            xy_focus: XYFocus::new(),
            reusable_focus_options: RefCell::new(None),
            focus_root: RefCell::new(None),
            content_root: RefCell::new(None),
        })
    }

    /// The focus manager registered with the locator (as
    /// `dyn IFocusManager`): used for elements that are not attached to an
    /// input root.
    fn locator_instance() -> Option<Rc<FocusManager>> {
        FerroLocator::current()
            .get_service::<dyn IFocusManager>()
            .and_then(|manager| manager.as_any().downcast_ref::<FocusManager>().and_then(|m| m.this.upgrade()))
    }

    /// The root of the content the focus manager navigates within.
    pub fn content_root(&self) -> Option<Ref<InputElement>> {
        self.content_root.borrow().clone()
    }

    /// Sets the root of the content the focus manager navigates within.
    pub fn set_content_root(&self, value: Option<Ref<InputElement>>) {
        let old = self.content_root.replace(value);
        drop(old);
    }

    fn current() -> Option<Ref<InputElement>> {
        KeyboardDevice::instance().and_then(|keyboard| keyboard.focused_element())
    }

    fn focus_root(&self) -> Option<Ref<StyledElement>> {
        self.focus_root.borrow().clone()
    }

    fn set_focus_root(&self, value: Option<Ref<StyledElement>>) {
        let old = self.focus_root.replace(value);
        drop(old);
    }

    /// Gets the currently focused element.
    pub fn get_focused_element(&self) -> Option<Ref<InputElement>> {
        Self::current()
    }

    /// Focuses a control.
    ///
    /// `element` is the control to focus; `None` restores the focus of the
    /// current focus root or clears the focus. `method` is the method by
    /// which focus was changed and `key_modifiers` any key modifiers active
    /// at the time of focus. Returns true if the focus moved to a control.
    pub fn focus(
        &self,
        element: Option<&Ref<InputElement>>,
        method: NavigationMethod,
        key_modifiers: KeyModifiers,
    ) -> bool {
        let Some(keyboard_device) = KeyboardDevice::instance() else { return false };

        if let Some(element) = element {
            return self.focus_core(&keyboard_device, element, method, key_modifiers);
        }

        if let Some(focus_root) = self.focus_root() {
            let current = Self::current();
            let restore = crate::ElementRef::resolve(&focus_root.get_value(Self::focused_element_property()));

            if let Some(restore) = restore.filter(|restore| Some(restore) != current.as_ref()) {
                if !Self::can_focus(&restore) {
                    // The previous effective focus is no longer part of the
                    // focus root's visual tree. We clear the focused
                    // element.
                    focus_root.clear_value(Self::focused_element_property());

                    if let Some(current) = &current {
                        if Self::get_focus_scope(current) != Some(focus_root.clone()) {
                            self.set_focus_root(None);
                            return false;
                        }
                    }
                } else {
                    return self.focus_core(&keyboard_device, &restore, method, key_modifiers);
                }
            }
        }

        self.set_focus_root(None);
        keyboard_device.set_focused_element_with(None, NavigationMethod::Unspecified, KeyModifiers::NONE, false);
        false
    }

    fn focus_core(
        &self,
        keyboard_device: &KeyboardDevice,
        element: &Ref<InputElement>,
        method: NavigationMethod,
        key_modifiers: KeyModifiers,
    ) -> bool {
        if !Self::can_focus(element) {
            return false;
        }

        keyboard_device.set_focused_element(Some(element), method, key_modifiers);

        if let Some(effectively_focused_element) = keyboard_device.focused_element() {
            if let Some(scope) = Self::get_focus_scope(&effectively_focused_element) {
                scope.set_value(
                    Self::focused_element_property(),
                    Some(crate::ElementRef::new(&effectively_focused_element)),
                );
                self.set_focus_root(Self::get_focus_root(&scope));
            }

            return effectively_focused_element == *element;
        }

        self.set_focus_root(None);
        keyboard_device.set_focused_element_with(None, NavigationMethod::Unspecified, KeyModifiers::NONE, false);
        false
    }

    pub(crate) fn clear_focus_on_element_removed(&self, removed_element: &InputElement, old_parent: &Visual) {
        if let Some(parent_element) = old_parent.downcast_ref::<InputElement>() {
            if let Some(scope) = Self::get_focus_scope(parent_element) {
                if let Some(focused) = crate::ElementRef::resolve(&scope.get_value(Self::focused_element_property())) {
                    if std::ptr::eq::<InputElement>(&*focused, removed_element) {
                        scope.clear_value(Self::focused_element_property());
                    }
                }
            }
        }

        if Self::current().is_some_and(|current| std::ptr::eq::<InputElement>(&*current, removed_element)) {
            self.focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
        }
    }

    /// Gets the element that is focused within a focus scope.
    pub fn get_focused_element_in_scope(&self, scope: &StyledElement) -> Option<Ref<InputElement>> {
        crate::ElementRef::resolve(&scope.get_value(Self::focused_element_property()))
    }

    /// Sets the currently focused scope: focus moves to the element that
    /// was last focused in the scope, or to the scope itself.
    pub fn set_focus_scope(&self, scope: &Ref<InputElement>) {
        let Some(keyboard_device) = KeyboardDevice::instance() else { return };

        if let Some(focused) = self.get_focused_element_in_scope(scope) {
            self.focus(Some(&focused), NavigationMethod::Unspecified, KeyModifiers::NONE);
        } else if Self::can_focus(scope) {
            // Focus the scope itself. Selecting the first focusable control
            // or a control with default focus is left to keyboard
            // navigation.
            self.focus(Some(scope), NavigationMethod::Unspecified, KeyModifiers::NONE);
        } else {
            // If the scope isn't focusable, make sure we still set it as
            // the current focus root, otherwise it will be completely
            // ignored.
            self.set_focus_root(Some(scope.clone().upcast()));
            keyboard_device.set_focused_element_with(None, NavigationMethod::Unspecified, KeyModifiers::NONE, false);
        }
    }

    /// Clears the focus if `scope` is the current focus root.
    pub fn remove_focus_root(&self, scope: &StyledElement) {
        if self.focus_root().is_some_and(|root| std::ptr::eq::<StyledElement>(&*root, scope)) {
            self.focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
        }
    }

    /// Whether an element is a focus scope.
    pub fn get_is_focus_scope(e: &InputElement) -> bool {
        e.is_focus_scope()
    }

    /// The focus manager responsible for an element: the one of the input
    /// root the element is attached to, or the fallback instance.
    pub fn get_focus_manager(element: &InputElement) -> Option<Rc<FocusManager>> {
        element.get_input_root().and_then(|root| root.focus_manager()).or_else(Self::locator_instance)
    }

    /// Whether an element can receive focus: it is focusable, effectively
    /// enabled and visible.
    pub fn can_focus(e: &InputElement) -> bool {
        e.focusable() && e.is_effectively_enabled() && Self::is_visible(e)
    }

    fn can_pointer_focus(e: &InputElement, ev: &dyn IRoutedEventArgs) -> bool {
        if !Self::can_focus(e) {
            return false;
        }

        if let Some(released) = ev.downcast_ref::<PointerReleasedEventArgs>() {
            return released.pointer().type_() != PointerType::Mouse;
        }

        if let Some(pressed) = ev.downcast_ref::<PointerPressedEventArgs>() {
            return pressed.pointer().type_() == PointerType::Mouse;
        }

        false
    }

    fn get_focus_scope(control: &InputElement) -> Option<Ref<StyledElement>> {
        let mut c = Some(control.to_ref());

        while let Some(current) = c {
            if current.is_focus_scope() && current.visual_root().is_some_and(|root| root.is_visible()) {
                return Some(current.upcast());
            }

            c = current.get_visual_parent_of_type::<InputElement>().or_else(|| {
                current
                    .as_hosted_visual_tree_root()
                    .and_then(|hosted| hosted.host())
                    .and_then(|host| host.downcast::<InputElement>().ok())
            });
        }

        None
    }

    fn get_focus_root(scope: &StyledElement) -> Option<Ref<StyledElement>> {
        let visual = scope.downcast_ref::<Visual>()?;
        let mut root = visual.get_input_root()?.focus_root();

        loop {
            let parent_root = root
                .as_hosted_visual_tree_root()
                .and_then(|hosted| hosted.host())
                .and_then(|host| host.get_input_root())
                .map(|input_root| input_root.focus_root());

            match parent_root {
                Some(parent_root) => root = parent_root,
                None => break,
            }
        }

        Some(root.upcast())
    }

    /// Global handler for pointer pressed and released: moves focus to the
    /// nearest focusable element under the pointer.
    fn on_preview_pointer_event_handler(sender: &Interactive, e: &dyn IRoutedEventArgs) {
        let Some(ev) = e.downcast_ref::<PointerEventArgs>() else { return };

        if !e.is_source(sender) {
            return;
        }

        let left_pressed = ev.get_current_point(Some(sender)).properties.is_left_button_pressed;
        let left_released = e
            .downcast_ref::<PointerReleasedEventArgs>()
            .is_some_and(|released| released.initial_press_mouse_button() == MouseButton::Left);

        if !(left_pressed || left_released) {
            return;
        }

        let mut element: Option<Ref<Visual>> = match ev.pointer().captured() {
            Some(captured) => Some(captured.upcast()),
            None => e.source().and_then(|source| source.downcast::<Visual>().ok()),
        };

        while let Some(current) = element {
            if let Some(input_element) = current.downcast_ref::<InputElement>() {
                if Self::can_pointer_focus(input_element, e) {
                    input_element.focus_with(NavigationMethod::Pointer, ev.key_modifiers());
                    break;
                }
            }

            element = current.visual_parent();
        }
    }

    fn is_visible(e: &InputElement) -> bool {
        e.is_attached_to_visual_tree() && e.is_effectively_visible()
    }

    // --- navigation -------------------------------------------------------------

    /// Attempts to change focus from the element with focus to the next
    /// focusable element in the specified direction.
    ///
    /// Panics for directions other than next, previous, up, down, left and
    /// right.
    pub fn try_move_focus(&self, direction: NavigationDirection, options: Option<&FindNextElementOptions>) -> bool {
        Self::validate_direction(direction);

        let mut focus_options = self.to_focus_options(options, true);
        let start = options.and_then(|o| o.focused_element.clone()).or_else(Self::current);
        let result = self.find_and_set_next_focus(start.as_ref(), direction, &mut focus_options);
        self.put_back_focus_options(focus_options);

        result
    }

    fn content_root_has_input_element(&self) -> bool {
        self.content_root().is_some()
    }

    /// Retrieves the first element that can receive focus within the
    /// content root.
    pub fn find_first_focusable_element(&self) -> Option<Element> {
        if !self.content_root_has_input_element() {
            return None;
        }

        self.get_first_focusable_element_from_root(false)
    }

    /// Retrieves the first element that can receive focus based on the
    /// specified scope.
    pub fn find_first_focusable_element_in(search_scope: &Element) -> Option<Element> {
        Self::get_first_focusable_element(search_scope, None)
    }

    /// Retrieves the last element that can receive focus within the
    /// content root.
    pub fn find_last_focusable_element(&self) -> Option<Element> {
        if !self.content_root_has_input_element() {
            return None;
        }

        self.get_first_focusable_element_from_root(true)
    }

    /// Retrieves the last element that can receive focus based on the
    /// specified scope.
    pub fn find_last_focusable_element_in(search_scope: &Element) -> Option<Element> {
        Self::get_focus_manager(search_scope).and_then(|manager| manager.get_last_focusable_element(search_scope, None))
    }

    /// Retrieves the element that should receive focus based on the
    /// specified navigation direction, without moving focus.
    ///
    /// Panics for directions other than next, previous, up, down, left and
    /// right.
    pub fn find_next_element(
        &self,
        direction: NavigationDirection,
        options: Option<&FindNextElementOptions>,
    ) -> Option<Element> {
        Self::validate_direction(direction);

        let mut focus_options = self.to_focus_options(options, false);
        let start = options.and_then(|o| o.focused_element.clone()).or_else(Self::current);
        let result = self.find_next_focus(start.as_ref(), direction, &mut focus_options, true);
        self.put_back_focus_options(focus_options);

        result
    }

    fn validate_direction(direction: NavigationDirection) {
        if !matches!(
            direction,
            NavigationDirection::Next
                | NavigationDirection::Previous
                | NavigationDirection::Up
                | NavigationDirection::Down
                | NavigationDirection::Left
                | NavigationDirection::Right
        ) {
            panic!(
                "Only Next, Previous, Up, Down, Left and Right directions are supported (got {direction:?})"
            );
        }
    }

    fn to_focus_options(&self, options: Option<&FindNextElementOptions>, update_manifold: bool) -> XYFocusOptions {
        // XYFocus only uses the options and never modifies them; we can
        // cache and reset them between calls.
        let mut focus_options = match self.reusable_focus_options.borrow_mut().take() {
            Some(mut options) => {
                options.reset();
                options
            }
            None => XYFocusOptions::new(),
        };

        if let Some(options) = options {
            focus_options.search_root = options.search_root.clone();
            focus_options.exclusion_rect = options.exclusion_rect;
            focus_options.focus_hint_rectangle = options.focus_hint_rectangle;
            focus_options.navigation_strategy_override = options.navigation_strategy_override;
            focus_options.ignore_occlusivity = options.ignore_occlusivity;
        }

        focus_options.update_manifold = update_manifold;

        focus_options
    }

    fn put_back_focus_options(&self, options: XYFocusOptions) {
        let old = self.reusable_focus_options.replace(Some(options));
        drop(old);
    }

    fn find_next_focus(
        &self,
        focused_element: Option<&Element>,
        direction: NavigationDirection,
        focus_options: &mut XYFocusOptions,
        update_manifolds: bool,
    ) -> Option<Element> {
        match focused_element {
            Some(focused) if !matches!(direction, NavigationDirection::Previous | NavigationDirection::Next) => {
                if let Some(bounds) = XYFocus::get_bounds_for_ranking(focused, focus_options.ignore_clipping) {
                    focus_options.focused_element_bounds = Some(bounds);
                }

                self.xy_focus.get_next_focusable_element(direction, Some(focused), None, update_manifolds, focus_options)
            }
            _ => {
                let is_reverse = direction == NavigationDirection::Previous;
                self.process_tab_stop_internal(focused_element, is_reverse, true)
            }
        }
    }

    pub(crate) fn get_first_focusable_element_internal(
        search_start: &Element,
        focus_candidate: Option<Element>,
    ) -> Option<Element> {
        let mut focus_candidate = focus_candidate;
        let first_focusable_from_callback = search_start.get_first_focusable_element_override();
        let use_first_focusable_from_callback = first_focusable_from_callback.as_ref().is_some_and(|first| {
            FocusHelpers::is_focusable(Some(first)) || FocusHelpers::can_have_focusable_children(Some(first))
        });

        if use_first_focusable_from_callback {
            let better = match &focus_candidate {
                None => true,
                Some(candidate) => {
                    Self::get_tab_index(first_focusable_from_callback.as_ref()) < Self::get_tab_index(Some(candidate))
                }
            };
            if better {
                focus_candidate = first_focusable_from_callback;
            }
        } else {
            for child in FocusHelpers::get_input_element_children(search_start) {
                if !FocusHelpers::is_visible(&child) {
                    continue;
                }

                let has_focusable_children = FocusHelpers::can_have_focusable_children(Some(&child));

                if FocusHelpers::is_potential_tab_stop(Some(&child)) {
                    if (FocusHelpers::is_focusable(Some(&child)) || has_focusable_children)
                        && focus_candidate.as_ref().is_none_or(|candidate| {
                            Self::get_tab_index(Some(&child)) < Self::get_tab_index(Some(candidate))
                        })
                    {
                        focus_candidate = Some(child);
                    }
                } else if has_focusable_children {
                    focus_candidate = Self::get_first_focusable_element_internal(&child, focus_candidate);
                }
            }
        }

        focus_candidate
    }

    pub(crate) fn get_last_focusable_element_internal(
        search_start: &Element,
        last_focus: Option<Element>,
    ) -> Option<Element> {
        let mut last_focus = last_focus;
        let last_focusable_from_callback = search_start.get_last_focusable_element_override();
        let use_last_focusable_from_callback = last_focusable_from_callback.as_ref().is_some_and(|last| {
            FocusHelpers::is_focusable(Some(last)) || FocusHelpers::can_have_focusable_children(Some(last))
        });

        if use_last_focusable_from_callback {
            let better = match &last_focus {
                None => true,
                Some(last) => Self::get_tab_index(last_focusable_from_callback.as_ref()) > Self::get_tab_index(Some(last)),
            };
            if better {
                last_focus = last_focusable_from_callback;
            }
        } else {
            for child in FocusHelpers::get_input_element_children(search_start) {
                if !FocusHelpers::is_visible(&child) {
                    continue;
                }

                let has_focusable_children = FocusHelpers::can_have_focusable_children(Some(&child));

                if FocusHelpers::is_potential_tab_stop(Some(&child)) {
                    if (FocusHelpers::is_focusable(Some(&child)) || has_focusable_children)
                        && last_focus
                            .as_ref()
                            .is_none_or(|last| Self::get_tab_index(Some(&child)) >= Self::get_tab_index(Some(last)))
                    {
                        last_focus = Some(child);
                    }
                } else if has_focusable_children {
                    last_focus = Self::get_last_focusable_element_internal(&child, last_focus);
                }
            }
        }

        last_focus
    }

    fn process_tab_stop_internal(
        &self,
        focused_element: Option<&Element>,
        is_reverse: bool,
        query_only: bool,
    ) -> Option<Element> {
        let (default_candidate_tab_stop, did_cycle_focus_at_root_visual_scope) =
            self.get_tab_stop_candidate_element(focused_element, is_reverse, query_only);

        let (is_tab_stop_overriden, new_tab_stop_from_callback) = InputElement::process_tab_stop(
            self.content_root().as_ref(),
            focused_element,
            default_candidate_tab_stop.as_ref(),
            is_reverse,
            did_cycle_focus_at_root_visual_scope,
        );

        if is_tab_stop_overriden {
            new_tab_stop_from_callback
        } else {
            default_candidate_tab_stop
        }
    }

    fn get_tab_stop_candidate_element(
        &self,
        focused_element: Option<&Element>,
        is_reverse: bool,
        query_only: bool,
    ) -> (Option<Element>, bool) {
        let mut did_cycle_focus_at_root_visual_scope = false;
        let Some(root) = self.content_root() else { return (None, false) };

        let internal_cycle_workaround =
            focused_element.is_some_and(|focused| self.can_process_tab_stop(focused, is_reverse));

        let new_tab_stop = match focused_element {
            None => {
                did_cycle_focus_at_root_visual_scope = true;
                if !is_reverse {
                    Self::get_first_focusable_element(&root, None)
                } else {
                    self.get_last_focusable_element(&root, None)
                }
            }
            Some(focused) if !is_reverse => {
                let mut new_tab_stop = self.get_next_tab_stop(focused, false);

                if new_tab_stop.is_none() && (internal_cycle_workaround || query_only) {
                    new_tab_stop = Self::get_first_focusable_element(&root, None);
                    did_cycle_focus_at_root_visual_scope = true;
                }

                new_tab_stop
            }
            Some(focused) => {
                let mut new_tab_stop = self.get_previous_tab_stop(focused);

                if new_tab_stop.is_none() && (internal_cycle_workaround || query_only) {
                    new_tab_stop = self.get_last_focusable_element(&root, None);
                    did_cycle_focus_at_root_visual_scope = true;
                }

                new_tab_stop
            }
        };

        (new_tab_stop, did_cycle_focus_at_root_visual_scope)
    }

    fn content_root_visual_root(&self) -> Option<Element> {
        self.content_root().and_then(|root| root.visual_root()).and_then(|root| root.downcast::<InputElement>().ok())
    }

    fn tab_navigation_of(element: &InputElement) -> KeyboardNavigationMode {
        KeyboardNavigation::get_tab_navigation(element)
    }

    fn get_next_tab_stop(&self, focused: &Element, ignore_current_tab_stop: bool) -> Option<Element> {
        self.content_root()?;

        let mut current_compare = Some(focused.clone());
        let mut new_tab_stop = focused.get_next_tab_stop_override();

        if new_tab_stop.is_none() && !ignore_current_tab_stop && FocusHelpers::is_visible(focused) {
            new_tab_stop = Self::get_first_focusable_element(focused, new_tab_stop);
        }

        if new_tab_stop.is_none() {
            let mut current_passed = false;
            let mut current = focused.clone();
            let mut parent = FocusHelpers::get_focus_parent(Some(focused));
            let visual_root = self.content_root_visual_root();
            let mut parent_is_root_visual = parent == visual_root;

            while let Some(p) = parent.clone() {
                if parent_is_root_visual || new_tab_stop.is_some() {
                    break;
                }
                let mut p = p;

                if Self::is_valid_tab_stop_search_candidate(Some(&current))
                    && Self::tab_navigation_of(&current) == KeyboardNavigationMode::Cycle
                {
                    new_tab_stop = if Some(&current) == Self::get_parent_tab_stop_element(Some(focused)).as_ref() {
                        Self::get_first_focusable_element(focused, None)
                    } else {
                        Self::get_first_focusable_element(&current, Some(current.clone()))
                    };
                    break;
                }

                if Self::is_valid_tab_stop_search_candidate(Some(&p))
                    && Self::tab_navigation_of(&p) == KeyboardNavigationMode::Once
                {
                    current = p;
                    match FocusHelpers::get_focus_parent(Some(&current)) {
                        Some(next_parent) => p = next_parent,
                        None => break,
                    }
                } else if !Self::is_valid_tab_stop_search_candidate(Some(&p)) {
                    match Self::get_parent_tab_stop_element(Some(&p)) {
                        None => {
                            // There is no popup sub tree to look into: fall
                            // back to the root of the visual tree.
                            match visual_root.clone() {
                                Some(root) => p = root,
                                None => {
                                    new_tab_stop = Self::get_next_or_previous_tab_stop_internal(
                                        None,
                                        Some(&current),
                                        new_tab_stop,
                                        true,
                                        &mut current_passed,
                                        &mut current_compare,
                                    );
                                    break;
                                }
                            }
                        }
                        Some(parent_element)
                            if Self::tab_navigation_of(&parent_element) == KeyboardNavigationMode::None =>
                        {
                            current = parent_element;
                            match FocusHelpers::get_focus_parent(Some(&current)) {
                                Some(next_parent) => p = next_parent,
                                None => break,
                            }
                        }
                        Some(parent_element) => p = parent_element,
                    }
                }

                new_tab_stop = Self::get_next_or_previous_tab_stop_internal(
                    Some(&p),
                    Some(&current),
                    new_tab_stop,
                    true,
                    &mut current_passed,
                    &mut current_compare,
                );

                if let Some(stop) = &new_tab_stop {
                    if !FocusHelpers::is_focusable(Some(stop)) && FocusHelpers::can_have_focusable_children(Some(stop)) {
                        new_tab_stop = Self::get_first_focusable_element(stop, None);
                    }
                }

                if new_tab_stop.is_some() {
                    break;
                }

                if Self::is_valid_tab_stop_search_candidate(Some(&p)) {
                    current = p.clone();
                }

                parent = FocusHelpers::get_focus_parent(Some(&p));
                current_passed = false;
                parent_is_root_visual = parent == visual_root;
            }
        }

        new_tab_stop
    }

    fn get_previous_tab_stop(&self, focused: &Element) -> Option<Element> {
        self.content_root()?;

        let mut new_tab_stop = focused.get_previous_tab_stop_override();
        let mut current_compare = Some(focused.clone());

        if new_tab_stop.is_none() {
            let mut current_passed = false;
            let mut current = focused.clone();
            let mut parent = FocusHelpers::get_focus_parent(Some(focused));
            let visual_root = self.content_root_visual_root();
            let parent_is_root_visual = parent == visual_root;

            while let Some(p) = parent.clone() {
                if parent_is_root_visual || new_tab_stop.is_some() {
                    break;
                }
                let mut p = p;

                if Self::is_valid_tab_stop_search_candidate(Some(&current))
                    && Self::tab_navigation_of(&current) == KeyboardNavigationMode::Cycle
                {
                    new_tab_stop = self.get_last_focusable_element(&current, Some(current.clone()));
                    break;
                }

                if Self::is_valid_tab_stop_search_candidate(Some(&p))
                    && Self::tab_navigation_of(&p) == KeyboardNavigationMode::Once
                {
                    if FocusHelpers::is_focusable(Some(&p)) {
                        new_tab_stop = Some(p.clone());
                    } else {
                        current = p;
                        match FocusHelpers::get_focus_parent(Some(&current)) {
                            Some(next_parent) => p = next_parent,
                            None => break,
                        }
                    }
                } else if !Self::is_valid_tab_stop_search_candidate(Some(&p)) {
                    match Self::get_parent_tab_stop_element(Some(&p)) {
                        None => match visual_root.clone() {
                            Some(root) => p = root,
                            None => {
                                new_tab_stop = Self::get_next_or_previous_tab_stop_internal(
                                    None,
                                    Some(&current),
                                    new_tab_stop,
                                    false,
                                    &mut current_passed,
                                    &mut current_compare,
                                );
                                break;
                            }
                        },
                        Some(parent_element)
                            if Self::tab_navigation_of(&parent_element) == KeyboardNavigationMode::None =>
                        {
                            if FocusHelpers::is_focusable(Some(&p)) {
                                new_tab_stop = Some(p.clone());
                            } else {
                                current = p;
                                match FocusHelpers::get_focus_parent(Some(&current)) {
                                    Some(next_parent) => p = next_parent,
                                    None => break,
                                }
                            }
                        }
                        Some(parent_element) => p = parent_element,
                    }
                }

                new_tab_stop = Self::get_next_or_previous_tab_stop_internal(
                    Some(&p),
                    Some(&current),
                    new_tab_stop,
                    false,
                    &mut current_passed,
                    &mut current_compare,
                );

                if new_tab_stop.is_none()
                    && FocusHelpers::is_potential_tab_stop(Some(&p))
                    && FocusHelpers::is_focusable(Some(&p))
                {
                    new_tab_stop = if Self::tab_navigation_of(&p) == KeyboardNavigationMode::Cycle {
                        self.get_last_focusable_element(&p, None)
                    } else {
                        Some(p.clone())
                    };
                } else if let Some(stop) = &new_tab_stop {
                    if FocusHelpers::can_have_focusable_children(Some(stop)) {
                        new_tab_stop = self.get_last_focusable_element(stop, None);
                    }
                }

                if new_tab_stop.is_some() {
                    break;
                }

                if Self::is_valid_tab_stop_search_candidate(Some(&p)) {
                    current = p.clone();
                }

                parent = FocusHelpers::get_focus_parent(Some(&p));
                current_passed = false;
            }
        }

        new_tab_stop
    }

    fn get_next_or_previous_tab_stop_internal(
        parent: Option<&Element>,
        current: Option<&Element>,
        candidate: Option<Element>,
        find_next: bool,
        current_passed: &mut bool,
        current_compare: &mut Option<Element>,
    ) -> Option<Element> {
        let mut new_tab_stop = candidate;

        if Self::is_valid_tab_stop_search_candidate(current) {
            *current_compare = current.cloned();
        }

        let Some(parent) = parent else { return new_tab_stop };
        let mut found_current = false;

        for child in FocusHelpers::get_input_element_children(parent) {
            let mut child_stop: Option<Element> = None;
            let mut compare_current_for_previous_element = false;

            if Some(&child) == current {
                found_current = true;
                *current_passed = true;
                continue;
            }

            if FocusHelpers::is_visible(&child) {
                if Self::is_valid_tab_stop_search_candidate(Some(&child)) {
                    if !FocusHelpers::is_potential_tab_stop(Some(&child)) {
                        // The recursion is made on the (still empty) child
                        // stop upstream, which yields the current candidate.
                        child_stop = Self::get_next_or_previous_tab_stop_internal(
                            None,
                            current,
                            new_tab_stop.clone(),
                            find_next,
                            current_passed,
                            current_compare,
                        );
                        compare_current_for_previous_element = true;
                    } else {
                        child_stop = Some(child.clone());
                    }
                } else if FocusHelpers::can_have_focusable_children(Some(&child)) {
                    child_stop = Self::get_next_or_previous_tab_stop_internal(
                        Some(&child),
                        current,
                        new_tab_stop.clone(),
                        find_next,
                        current_passed,
                        current_compare,
                    );
                    compare_current_for_previous_element = true;
                }
            }

            let Some(child_stop) = child_stop else { continue };

            if FocusHelpers::is_focusable(Some(&child_stop))
                || FocusHelpers::can_have_focusable_children(Some(&child_stop))
            {
                let compare_index_result = Self::compare_tab_index(Some(&child_stop), current_compare.as_ref());

                let accept = if find_next {
                    (compare_index_result > 0 || ((found_current || *current_passed) && compare_index_result == 0))
                        && new_tab_stop
                            .as_ref()
                            .is_none_or(|stop| Self::compare_tab_index(Some(&child_stop), Some(stop)) < 0)
                } else {
                    (compare_index_result < 0
                        || (((!found_current && !*current_passed) || compare_current_for_previous_element)
                            && compare_index_result == 0))
                        && new_tab_stop
                            .as_ref()
                            .is_none_or(|stop| Self::compare_tab_index(Some(&child_stop), Some(stop)) >= 0)
                };

                if accept {
                    new_tab_stop = Some(child_stop);
                }
            }
        }

        new_tab_stop
    }

    fn compare_tab_index(control1: Option<&Element>, control2: Option<&Element>) -> i32 {
        match Self::get_tab_index(control1).cmp(&Self::get_tab_index(control2)) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    fn get_tab_index(element: Option<&Element>) -> i32 {
        element.map_or(i32::MAX, |element| element.tab_index())
    }

    fn can_process_tab_stop(&self, focused_element: &Element, is_reverse: bool) -> bool {
        let mut is_focus_on_first = false;
        let mut is_focus_on_last = false;
        let mut can_process_tab = true;

        if is_reverse {
            is_focus_on_first = self.is_focus_on_first_tab_stop(focused_element);
        } else {
            is_focus_on_last = self.is_focus_on_last_tab_stop(focused_element);
        }

        if is_focus_on_first || is_focus_on_last {
            can_process_tab = false;
        }

        if can_process_tab {
            match self.get_first_focusable_element_from_root(!is_reverse) {
                Some(edge) => {
                    let edge_parent = Self::get_parent_tab_stop_element(Some(&edge));
                    if let Some(edge_parent) = &edge_parent {
                        if Self::tab_navigation_of(edge_parent) == KeyboardNavigationMode::Once
                            && Some(edge_parent) == Self::get_parent_tab_stop_element(Some(focused_element)).as_ref()
                        {
                            can_process_tab = false;
                        }
                    }
                }
                None => can_process_tab = false,
            }
        } else if Self::tab_navigation_of(focused_element) == KeyboardNavigationMode::Cycle {
            can_process_tab = true;
        } else {
            let mut focused_parent = Self::get_parent_tab_stop_element(Some(focused_element));
            while let Some(parent) = focused_parent {
                if Self::tab_navigation_of(&parent) == KeyboardNavigationMode::Cycle {
                    can_process_tab = true;
                    break;
                }

                focused_parent = Self::get_parent_tab_stop_element(Some(&parent));
            }
        }

        can_process_tab
    }

    fn get_parent_tab_stop_element(current: Option<&Element>) -> Option<Element> {
        let mut parent = FocusHelpers::get_focus_parent(current.map(|c| &**c));

        while let Some(p) = parent {
            if Self::is_valid_tab_stop_search_candidate(Some(&p)) {
                return Some(p);
            }

            parent = FocusHelpers::get_focus_parent(Some(&p));
        }

        None
    }

    fn is_valid_tab_stop_search_candidate(element: Option<&Element>) -> bool {
        FocusHelpers::is_potential_tab_stop(element.map(|e| &**e))
            || element.is_some_and(|e| e.is_set(KeyboardNavigation::tab_navigation_property()))
    }

    fn get_first_focusable_element_from_root(&self, is_reverse: bool) -> Option<Element> {
        let root = self.content_root_visual_root()?;

        if !is_reverse {
            Self::get_first_focusable_element(&root, None)
        } else {
            self.get_last_focusable_element(&root, None)
        }
    }

    fn is_focus_on_last_tab_stop(&self, focused_element: &Element) -> bool {
        let Some(root) = self.content_root_visual_root() else { return false };
        self.get_last_focusable_element(&root, None).as_ref() == Some(focused_element)
    }

    fn is_focus_on_first_tab_stop(&self, focused_element: &Element) -> bool {
        let Some(root) = self.content_root_visual_root() else { return false };
        Self::get_first_focusable_element(&root, None).as_ref() == Some(focused_element)
    }

    fn get_first_focusable_element(search_start: &Element, first_focus: Option<Element>) -> Option<Element> {
        let mut first_focus = Self::get_first_focusable_element_internal(search_start, first_focus);

        if let Some(first) = &first_focus {
            if !first.focusable() && FocusHelpers::can_have_focusable_children(Some(first)) {
                first_focus = Self::get_first_focusable_element(first, None);
            }
        }

        first_focus
    }

    fn get_last_focusable_element(&self, search_start: &Element, last_focus: Option<Element>) -> Option<Element> {
        let mut last_focus = Self::get_last_focusable_element_internal(search_start, last_focus);

        if let Some(last) = &last_focus {
            if !last.focusable() && FocusHelpers::can_have_focusable_children(Some(last)) {
                last_focus = self.get_last_focusable_element(last, None);
            }
        }

        last_focus
    }

    fn find_and_set_next_focus(
        &self,
        focused_element: Option<&Element>,
        direction: NavigationDirection,
        xy_focus_options: &mut XYFocusOptions,
    ) -> bool {
        let mut focus_changed = false;

        if xy_focus_options.update_manifolds_from_focus_hint_rect {
            if let Some(hint) = xy_focus_options.focus_hint_rectangle {
                self.xy_focus.set_manifolds_from_bounds(hint);
            }
        }

        if let Some(next_focused_element) = self.find_next_focus(focused_element, direction, xy_focus_options, false) {
            focus_changed = next_focused_element.focus();

            if focus_changed && xy_focus_options.update_manifold {
                let bounds = xy_focus_options
                    .focus_hint_rectangle
                    .or(xy_focus_options.focused_element_bounds)
                    .unwrap_or_default();

                self.xy_focus.update_manifolds(
                    direction,
                    bounds,
                    &next_focused_element,
                    xy_focus_options.ignore_clipping,
                );
            }
        }

        focus_changed
    }
}

impl IFocusManager for FocusManager {
    fn get_focused_element(&self) -> Option<Ref<InputElement>> {
        FocusManager::get_focused_element(self)
    }

    fn focus(
        &self,
        element: Option<&Ref<InputElement>>,
        method: NavigationMethod,
        key_modifiers: KeyModifiers,
    ) -> bool {
        FocusManager::focus(self, element, method, key_modifiers)
    }

    fn try_move_focus(&self, direction: NavigationDirection, options: Option<&FindNextElementOptions>) -> bool {
        FocusManager::try_move_focus(self, direction, options)
    }

    fn find_first_focusable_element(&self) -> Option<Ref<InputElement>> {
        FocusManager::find_first_focusable_element(self)
    }

    fn find_last_focusable_element(&self) -> Option<Ref<InputElement>> {
        FocusManager::find_last_focusable_element(self)
    }

    fn find_next_element(
        &self,
        direction: NavigationDirection,
        options: Option<&FindNextElementOptions>,
    ) -> Option<Ref<InputElement>> {
        FocusManager::find_next_element(self, direction, options)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
