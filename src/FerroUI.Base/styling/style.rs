use super::{
    DuplicateSetterError, ContainerQuery, ControlTheme, OrSelector, Selector, SelectorMatch, SelectorMatchResult, Setter, StyleBase,
    StyleBaseImpl, StyleBaseImplExt, StyleHostRef, TemplateSelector,
};
use crate::property_store::FrameType;
use crate::{ferro_class, instantiate, FerroObjectImpl, Ref, StyledElement};
use std::cell::RefCell;
use std::rc::Rc;

/// Defines a style.
#[repr(C)]
pub struct Style {
    base: StyleBase,
    selector: RefCell<Option<Selector>>,
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
        Self { base: StyleBase::construct(), selector: RefCell::new(None) }
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
        *self.selector.borrow_mut() = Self::validate_selector(value);
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
