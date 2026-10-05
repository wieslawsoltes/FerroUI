use super::activators::StyleClassActivator;
use super::{Selector, SelectorMatch, SelectorNode, Style, StyleBase};
use crate::{StyledElement, TypeInfo};
use std::any::Any;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// A selector that matches a control's type, name and/or style classes.
///
/// This selector is used for both type-based and class/name-based matching:
/// the three kinds of condition are merged into one selector when they are
/// chained.
pub(crate) struct TypeNameAndClassSelector {
    previous: Option<Selector>,
    classes: RefCell<Rc<[String]>>,
    target_type: Option<&'static TypeInfo>,
    is_concrete_type: Cell<bool>,
    name: RefCell<Option<String>>,
    selector_string: OnceCell<String>,
}

impl TypeNameAndClassSelector {
    fn new(previous: Option<Selector>, target_type: Option<&'static TypeInfo>, is_concrete_type: bool) -> Self {
        Self {
            previous,
            classes: RefCell::new(Rc::from(Vec::new())),
            target_type,
            is_concrete_type: Cell::new(is_concrete_type),
            name: RefCell::new(None),
            selector_string: OnceCell::new(),
        }
    }

    pub fn of_type(previous: Option<Selector>, target_type: &'static TypeInfo) -> Self {
        Self::new(previous, Some(target_type), true)
    }

    pub fn is(previous: Option<Selector>, target_type: &'static TypeInfo) -> Self {
        Self::new(previous, Some(target_type), false)
    }

    pub fn for_name(previous: Option<Selector>, name: &str) -> Self {
        let result = Self::new(previous, None, false);
        result.set_name(name);
        result
    }

    pub fn for_class(previous: Option<Selector>, class_name: &str) -> Self {
        let result = Self::new(previous, None, false);
        result.add_class(class_name);
        result
    }

    /// The name of the control to match.
    #[allow(dead_code)]
    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, name: &str) {
        *self.name.borrow_mut() = Some(name.to_string());
    }

    /// Whether the selector matches the target type exactly, as opposed to
    /// the target type and the types derived from it.
    pub fn is_concrete_type(&self) -> bool {
        self.is_concrete_type.get()
    }

    /// The style classes to match.
    #[allow(dead_code)]
    pub fn classes(&self) -> Rc<[String]> {
        self.classes.borrow().clone()
    }

    pub fn add_class(&self, name: &str) {
        let mut classes = self.classes.borrow_mut();
        let mut list: Vec<String> = classes.to_vec();
        list.push(name.to_string());
        *classes = Rc::from(list);
    }

    fn build_selector_string(&self, owner: Option<&Style>) -> String {
        let mut builder = String::new();

        if let Some(previous) = &self.previous {
            builder.push_str(&previous.to_string_with_next(owner, true));
        }

        if let Some(target_type) = SelectorNode::target_type(self) {
            if self.is_concrete_type() {
                builder.push_str(target_type.name());
            } else {
                builder.push_str(":is(");
                builder.push_str(target_type.name());
                builder.push(')');
            }
        }

        if let Some(name) = &*self.name.borrow() {
            builder.push('#');
            builder.push_str(name);
        }

        for c in self.classes.borrow().iter() {
            if !c.starts_with(':') {
                builder.push('.');
            }
            builder.push_str(c);
        }

        builder
    }
}

impl SelectorNode for TypeNameAndClassSelector {
    fn in_template(&self) -> bool {
        self.previous.as_ref().is_some_and(Selector::in_template)
    }

    fn is_combinator(&self) -> bool {
        false
    }

    fn target_type(&self) -> Option<&'static TypeInfo> {
        self.target_type.or_else(|| self.previous.as_ref().and_then(Selector::target_type))
    }

    fn to_string(&self, owner: Option<&Style>) -> String {
        self.selector_string.get_or_init(|| self.build_selector_string(owner)).clone()
    }

    fn evaluate(&self, control: &StyledElement, _parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch {
        if let Some(target_type) = SelectorNode::target_type(self) {
            let control_type = control.style_key();

            if self.is_concrete_type() {
                if control_type != target_type {
                    return SelectorMatch::NEVER_THIS_TYPE;
                }
            } else if !target_type.is_assignable_from(control_type) {
                return SelectorMatch::NEVER_THIS_TYPE;
            }
        }

        let has_name = {
            let name = self.name.borrow();
            if let Some(name) = &*name {
                if control.name().as_deref() != Some(name.as_str()) {
                    return SelectorMatch::NEVER_THIS_INSTANCE;
                }
            }
            name.is_some()
        };

        let classes = self.classes.borrow();
        if !classes.is_empty() {
            if subscribe {
                return SelectorMatch::sometimes(StyleClassActivator::new(control, classes.clone()));
            }

            if !StyleClassActivator::are_classes_matching(&control.classes().snapshot(), &classes) {
                return SelectorMatch::NEVER_THIS_INSTANCE;
            }
        }

        if has_name {
            SelectorMatch::ALWAYS_THIS_INSTANCE
        } else {
            SelectorMatch::ALWAYS_THIS_TYPE
        }
    }

    fn move_previous(&self) -> Option<&Selector> {
        self.previous.as_ref()
    }

    fn move_previous_or_parent(&self) -> Option<&Selector> {
        self.previous.as_ref()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
