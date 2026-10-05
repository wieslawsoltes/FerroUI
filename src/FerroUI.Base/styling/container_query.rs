use super::{
    DuplicateSetterError, ControlTheme, SelectorMatch, SelectorMatchResult, StyleBase, StyleBaseImpl, StyleBaseImplExt, StyleHostRef,
    StyleQuery,
};
use crate::property_store::FrameType;
use crate::{ferro_class, instantiate, FerroObjectImpl, Ref, StyledElement};
use std::cell::RefCell;

/// Defines a container: a set of styles that apply while a query on an
/// ancestor container of the control matches.
#[repr(C)]
pub struct ContainerQuery {
    base: StyleBase,
    query: RefCell<Option<StyleQuery>>,
    name: RefCell<Option<String>>,
}

ferro_class!(ContainerQuery: StyleBase);
crate::ferro_class_info!(ContainerQuery { new: ContainerQuery::new });

impl FerroObjectImpl for ContainerQuery {}

impl StyleBaseImpl for ContainerQuery {
    fn set_parent(this: &Self, parent: Option<&Ref<StyleBase>>) {
        if parent.is_some_and(|p| p.is::<ControlTheme>()) {
            Self::parent_set_parent(this, parent);
        } else {
            panic!("Container cannot be added as a nested style.");
        }
    }

    /// Returns a string representation of the container.
    fn to_display_string(this: &Self) -> String {
        match &*this.query.borrow() {
            Some(query) => query.to_string_with_owner(Some(this)),
            None => "ContainerQuery".to_string(),
        }
    }
}

impl ContainerQuery {
    /// Creates the class data; see [`FerroObject::construct`](crate::FerroObject::construct).
    pub fn construct() -> Self {
        Self { base: StyleBase::construct(), query: RefCell::new(None), name: RefCell::new(None) }
    }

    /// Creates a container without a query.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a container with a query and, optionally, the name of the
    /// container to query.
    pub fn with_query(query: StyleQuery, container_name: Option<String>) -> Ref<Self> {
        let result = Self::new();
        result.set_query(Some(query));
        result.set_name(container_name);
        result
    }

    /// The container's query.
    pub fn query(&self) -> Option<StyleQuery> {
        self.query.borrow().clone()
    }

    pub fn set_query(&self, value: Option<StyleQuery>) {
        *self.query.borrow_mut() = value;
    }

    /// The name of the container to query.
    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: Option<String>) {
        *self.name.borrow_mut() = value;
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

        if self.has_children() {
            let parent = self.parent();
            let match_ = match self.query() {
                Some(query) => query.match_(target, parent.as_deref(), true, self.name().as_deref()),
                None => {
                    let is_host = host
                        .and_then(StyleHostRef::as_element)
                        .is_some_and(|h| std::ptr::eq::<StyledElement>(&**h, target));
                    if is_host {
                        SelectorMatch::ALWAYS_THIS_INSTANCE
                    } else {
                        SelectorMatch::NEVER_THIS_INSTANCE
                    }
                }
            };

            result = match_.result();

            if match_.is_match() {
                self.try_attach_instance(target, match_.into_activator(), type_, true)?;
            }
        }

        Ok(result)
    }
}
