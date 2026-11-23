use super::theme_candidates::SelectorBound;
use super::{
    DuplicateSetterError, ContainerQuery, ControlTheme, OrSelector, Selector, SelectorMatch, SelectorMatchResult, Setter, StyleBase,
    StyleBaseImpl, StyleBaseImplExt, StyleHostRef, TemplateSelector,
};
use crate::property_store::FrameType;
use crate::{ferro_class, instantiate, FerroObjectImpl, Ref, StyledElement};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Defines a style.
#[repr(C)]
pub struct Style {
    base: StyleBase,
    selector: RefCell<Option<Selector>>,
    /// What the selector says of the type of the elements it matches, once it is
    /// index-enabled.
    bound: Cell<SelectorBound>,
    index_link: super::selector_candidates::IndexLink,
}

ferro_class!(Style: StyleBase);
crate::ferro_class_info!(Style { new: Style::new });

impl FerroObjectImpl for Style {}

impl StyleBaseImpl for Style {
    fn set_parent(this: &Self, parent: Option<&Ref<StyleBase>>) {
        if let Some(parent) = parent {
            let parent_has_selector = parent.downcast_ref::<Style>().is_some_and(|s| s.selector.borrow().is_some());
            if parent_has_selector {
                match &*this.selector.borrow() {
                    None => panic!("Child styles must have a selector."),
                    Some(selector) => selector.validate_nesting_selector(false),
                }
            } else if parent.is::<ControlTheme>() {
                match &*this.selector.borrow() {
                    None => panic!("Child styles must have a selector."),
                    Some(selector) => selector.validate_nesting_selector(true),
                }
            }
        }

        Self::parent_set_parent(this, parent);
        // Below a container query the style has no type candidate.
        if this.bound.get().type_candidate().is_some() {
            this.index_link.changed();
        }
    }

    /// Returns a string representation of the style.
    fn to_display_string(this: &Self) -> String {
        match &*this.selector.borrow() {
            Some(selector) => selector.to_string_with_owner(Some(this)),
            None => "Style".to_string(),
        }
    }
}

impl Style {
    /// Creates the class data; see [`FerroObject::construct`](crate::FerroObject::construct).
    pub fn construct() -> Self {
        Self {
            base: StyleBase::construct(),
            selector: RefCell::new(None),
            bound: Cell::new(SelectorBound::Unknown),
            index_link: Default::default(),
        }
    }

    /// Creates a style without a selector.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a style with a selector.
    pub fn with_selector(selector: Selector) -> Ref<Self> {
        let style = Self::new();
        style.set_selector(Some(selector));
        style
    }

    /// Creates a style whose selector is built by `selector`.
    pub fn with_selector_fn(selector: impl FnOnce(Option<Selector>) -> Selector) -> Ref<Self> {
        Self::with_selector(selector(None))
    }

    /// The style's selector.
    pub fn selector(&self) -> Option<Selector> {
        self.selector.borrow().clone()
    }

    pub fn set_selector(&self, value: Option<Selector>) {
        let value = Self::validate_selector(value);
        let previous = self.bound.take();
        if previous.type_candidate().is_some() {
            self.index_link.candidate_changed(true, false);
        }
        *self.selector.borrow_mut() = value;
        if previous != SelectorBound::Unknown {
            self.structure_changed();
        }
    }

    /// Enables conservative type indexing for a generated selector. Replacing
    /// the selector automatically returns the style to the ordinary matcher.
    ///
    /// Call it before the style is added to its collection: the collection
    /// counts the style as index-enabled when it is added. A style that is
    /// enabled for the first time while it is already in a collection stays a
    /// candidate for every element until it is added again; one that was
    /// enabled when it was added can be enabled again in place after its
    /// selector was replaced.
    #[doc(hidden)]
    pub fn enable_selector_index(&self) {
        let bound = match self.selector.borrow().as_ref() {
            Some(selector) => match selector.type_candidate() {
                Some(candidate) => SelectorBound::Type(candidate),
                None if selector.begins_with_nesting() => SelectorBound::Parent,
                None => SelectorBound::Unknown,
            },
            None => SelectorBound::Unknown,
        };
        let previous = self.bound.replace(bound);
        if previous != bound {
            let (was, is) = (previous.type_candidate(), bound.type_candidate());
            if was != is {
                self.index_link.candidate_changed(was.is_some(), is.is_some());
            }
            self.structure_changed();
        }
    }

    /// What the index-enabled selector says of the type of the elements it matches.
    pub(super) fn selector_bound(&self) -> SelectorBound {
        self.bound.get()
    }

    pub(super) fn index_link(&self) -> &super::selector_candidates::IndexLink {
        &self.index_link
    }

    /// Whether the selector is index-enabled, whatever the parent is.
    pub(super) fn has_type_candidate(&self) -> bool {
        self.bound.get().type_candidate().is_some()
    }

    pub(super) fn type_candidate(&self) -> Option<super::selector_candidates::TypeCandidate> {
        let candidate = self.bound.get().type_candidate()?;
        // Parent container queries run before the selector's type check.
        if self.parent().is_some_and(|p| p.is::<ContainerQuery>()) {
            return None;
        }
        Some(candidate)
    }

    /// Creates a style with a selector and setters.
    pub fn with_setters(selector: Selector, setters: impl IntoIterator<Item = Rc<Setter>>) -> Ref<Self> {
        let style = Self::with_selector(selector);
        for setter in setters {
            style.add_setter(setter);
        }
        style
    }

    /// As [`try_attach_checked`](Self::try_attach_checked); a duplicate
    /// setter panics with the message of the error.
    #[allow(dead_code)]
    pub(crate) fn try_attach(
        &self,
        target: &StyledElement,
        host: Option<&StyleHostRef>,
        type_: FrameType,
    ) -> SelectorMatchResult {
        match self.try_attach_checked(target, host, type_) {
            Ok(result) => result,
            Err(error) => panic!("{error}"),
        }
    }

    /// Attaches the style to `target` if it matches. A style with two
    /// setters for the same property is an error.
    pub(crate) fn try_attach_checked(
        &self,
        target: &StyledElement,
        host: Option<&StyleHostRef>,
        type_: FrameType,
    ) -> Result<SelectorMatchResult, DuplicateSetterError> {
        let mut result = SelectorMatchResult::NeverThisType;

        if self.has_setters_or_animations() {
            crate::perf_count!(StylesEvaluated);
            let parent = self.parent();
            let (match_, can_share) = {
                let selector = self.selector.borrow();
                match &*selector {
                    Some(selector) => {
                        (selector.match_(target, parent.as_deref(), true), selector.node::<OrSelector>().is_none())
                    }
                    None => {
                        let is_host =
                            host.and_then(StyleHostRef::as_element).is_some_and(|h| std::ptr::eq::<StyledElement>(&**h, target));
                        let match_ = if is_host {
                            match parent.as_deref().and_then(|p| p.downcast_ref::<ContainerQuery>()) {
                                Some(container_query) => match container_query.query() {
                                    Some(query) => {
                                        let query_parent = container_query.parent();
                                        query.match_(
                                            target,
                                            query_parent.as_deref(),
                                            true,
                                            container_query.name().as_deref(),
                                        )
                                    }
                                    None => SelectorMatch::NEVER_THIS_INSTANCE,
                                },
                                None => SelectorMatch::ALWAYS_THIS_INSTANCE,
                            }
                        } else {
                            SelectorMatch::NEVER_THIS_INSTANCE
                        };
                        (match_, true)
                    }
                }
            };

            result = match_.result();

            if match_.is_match() {
                crate::perf_count!(StylesMatched);
                self.try_attach_instance(target, match_.into_activator(), type_, can_share)?;
            }
        }

        Ok(result)
    }

    fn validate_selector(selector: Option<Selector>) -> Option<Selector> {
        if selector.as_ref().is_some_and(|s| s.node::<TemplateSelector>().is_some()) {
            panic!("Invalid selector: Template selector must be followed by control selector.");
        }
        selector
    }
}
