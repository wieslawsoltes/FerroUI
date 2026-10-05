use super::activators::AndActivatorBuilder;
use super::{
    ContainerQuery, NestingSelector, SelectorMatch, SelectorMatchResult, Style, StyleBase, TemplateSelector,
};
use crate::{StyledElement, TypeInfo};
use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// The behaviour of one kind of selector.
pub(crate) trait SelectorNode: 'static {
    /// Whether this selector is in a template.
    fn in_template(&self) -> bool;

    /// Whether this selector is a combinator.
    ///
    /// A combinator is a selector such as child or descendent which links
    /// simple selectors.
    fn is_combinator(&self) -> bool;

    /// The target type of the selector, if available.
    fn target_type(&self) -> Option<&'static TypeInfo>;

    /// Gets a string representing the selector, with the nesting separator
    /// (`^`) replaced with the parent selector.
    fn to_string(&self, owner: Option<&Style>) -> String;

    fn to_string_with_next(&self, owner: Option<&Style>, _has_next: bool) -> String {
        self.to_string(owner)
    }

    /// Evaluates the selector for a match.
    fn evaluate(&self, control: &StyledElement, parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch;

    /// Moves to the previous selector.
    fn move_previous(&self) -> Option<&Selector>;

    /// Moves to the previous selector or the parent selector.
    fn move_previous_or_parent(&self) -> Option<&Selector>;

    fn validate_nesting_selector(&self, in_control_theme: bool, mut template_count: i32) {
        if in_control_theme {
            if !self.in_template() && self.is_combinator() {
                panic!("ControlTheme style may not directly contain a child or descendent selector.");
            }
            if self.as_any().is::<TemplateSelector>() {
                let count = template_count;
                template_count += 1;
                if count > 0 {
                    panic!("ControlTemplate styles cannot contain multiple template selectors.");
                }
            }
        }

        match self.move_previous_or_parent() {
            None => {
                if !self.as_any().is::<NestingSelector>() {
                    panic!("Child styles must have a nesting selector.");
                }
            }
            Some(previous) => previous.0.validate_nesting_selector(in_control_theme, template_count),
        }
    }

    fn as_any(&self) -> &dyn Any;
}

/// A selector in a [`Style`].
///
/// Selectors are built with the functions of [`Selectors`](super::Selectors)
/// and the chaining methods of this type. A selector is a shared, immutable
/// description: cloning the handle is cheap.
#[derive(Clone)]
pub struct Selector(pub(crate) Rc<dyn SelectorNode>);


/// Compares by identity (reference equality), as the reference type this
/// mirrors.
impl PartialEq for Selector {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl Selector {
    pub(crate) fn new(node: impl SelectorNode) -> Self {
        Selector(Rc::new(node))
    }

    /// Whether this selector is in a template.
    #[inline]
    pub(crate) fn in_template(&self) -> bool {
        self.0.in_template()
    }

    /// Whether this selector is a combinator.
    #[inline]
    pub(crate) fn is_combinator(&self) -> bool {
        self.0.is_combinator()
    }

    /// The target type of the selector, if available.
    #[inline]
    pub fn target_type(&self) -> Option<&'static TypeInfo> {
        self.0.target_type()
    }

    pub(crate) fn node<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref::<T>()
    }

    /// Tries to match the selector with a control.
    ///
    /// `parent` is the parent style, if the style containing the selector is
    /// a nested style. With `subscribe`, a match whose state can change over
    /// time carries an activator that tracks it; without, the match is only
    /// evaluated for the current state.
    pub fn match_(&self, control: &StyledElement, parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch {
        // First match the selector until a combinator is found. Selectors are
        // stored from right-to-left, so the recursion reverses this order
        // because the type selector will be on the left.
        let (mut match_, combinator) = Self::match_until_combinator(control, self, parent, subscribe);

        // If the pre-combinator selector matches, we can now match the
        // combinator, if any.
        if match_.is_match() {
            if let Some(combinator) = combinator {
                match_ = match_.and(combinator.match_(control, parent, subscribe));

                // If we have a combinator then we can never say that we always
                // match a control of this type, because by definition the
                // combinator matches on things outside of the control.
                match_ = match match_.result() {
                    SelectorMatchResult::AlwaysThisType => SelectorMatch::ALWAYS_THIS_INSTANCE,
                    SelectorMatchResult::NeverThisType => SelectorMatch::NEVER_THIS_INSTANCE,
                    _ => match_,
                };
            }
        }

        match_
    }

    /// Gets a string representing the selector, with the nesting separator
    /// (`^`) replaced with the parent selector.
    pub fn to_string_with_owner(&self, owner: Option<&Style>) -> String {
        self.0.to_string(owner)
    }

    pub(crate) fn to_string_with_next(&self, owner: Option<&Style>, has_next: bool) -> String {
        self.0.to_string_with_next(owner, has_next)
    }

    pub(crate) fn validate_nesting_selector(&self, in_control_theme: bool) {
        self.0.validate_nesting_selector(in_control_theme, 0)
    }

    fn match_until_combinator<'a>(
        control: &StyledElement,
        start: &'a Selector,
        parent: Option<&StyleBase>,
        subscribe: bool,
    ) -> (SelectorMatch, Option<&'a Selector>) {
        let mut combinator = None;
        let mut activators = AndActivatorBuilder::new();
        let result = Self::match_selector(control, start, parent, subscribe, &mut activators, &mut combinator);
        let match_ = if result == SelectorMatchResult::Sometimes {
            SelectorMatch::sometimes(activators.get())
        } else {
            SelectorMatch::new(result)
        };
        (match_, combinator)
    }

    fn match_selector<'a>(
        control: &StyledElement,
        selector: &'a Selector,
        parent: Option<&StyleBase>,
        subscribe: bool,
        activators: &mut AndActivatorBuilder,
        combinator: &mut Option<&'a Selector>,
    ) -> SelectorMatchResult {
        let previous = selector.0.move_previous();

        // Selectors are stored from right-to-left, so we recurse into the
        // selector in order to reverse this order, because the type selector
        // will be on the left and is our best opportunity to exit early.
        if let Some(previous) = previous {
            if !previous.is_combinator() {
                let previous_match =
                    Self::match_selector(control, previous, parent, subscribe, activators, combinator);

                if previous_match < SelectorMatchResult::Sometimes {
                    return previous_match;
                }
            }
        }

        let mut container_matches_sometimes = false;

        // Match any parent container query.
        if let Some(container) = parent.and_then(|p| p.downcast_ref::<ContainerQuery>()) {
            let match_ = match container.query() {
                Some(query) => {
                    let container_parent = container.parent();
                    query.evaluate(control, container_parent.as_deref(), subscribe, container.name().as_deref())
                }
                None => SelectorMatch::NEVER_THIS_INSTANCE,
            };

            if !match_.is_match() {
                return match_.result();
            }

            container_matches_sometimes = match_.result() == SelectorMatchResult::Sometimes;

            if container_matches_sometimes {
                activators.add(match_.into_activator());
            }
        }

        // Match this selector.
        let match_ = selector.0.evaluate(control, parent, subscribe);

        if !match_.is_match() {
            *combinator = None;
            return match_.result();
        }

        let result = match_.result();
        activators.add(match_.into_activator());

        if let Some(previous) = previous {
            if previous.is_combinator() {
                *combinator = Some(previous);
            }
        }

        if container_matches_sometimes {
            SelectorMatchResult::Sometimes
        } else {
            result
        }
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string(None))
    }
}

impl fmt::Debug for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string(None))
    }
}
