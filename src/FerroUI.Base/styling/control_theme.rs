use super::{DuplicateSetterError, SelectorMatchResult, Setter, StyleBase, StyleBaseImpl};
use crate::property_store::FrameType;
use crate::{ferro_class, instantiate, FerroObjectImpl, ObjectType, Ref, StyledElement, TypeInfo};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Defines a switchable theme for a control.
#[repr(C)]
pub struct ControlTheme {
    base: StyleBase,
    target_type: Cell<Option<&'static TypeInfo>>,
    based_on: RefCell<Option<Ref<ControlTheme>>>,
}

ferro_class!(ControlTheme: StyleBase);
crate::ferro_class_info!(ControlTheme { new: ControlTheme::new });

impl FerroObjectImpl for ControlTheme {}

impl StyleBaseImpl for ControlTheme {
    fn set_parent(_this: &Self, _parent: Option<&Ref<StyleBase>>) {
        panic!("ControlThemes cannot be added as a nested style.");
    }

    fn to_display_string(this: &Self) -> String {
        match this.target_type.get() {
            Some(target_type) => target_type.name().to_string(),
            None => "ControlTheme".to_string(),
        }
    }
}

impl ControlTheme {
    /// Creates the class data; see [`FerroObject::construct`](crate::FerroObject::construct).
    pub fn construct() -> Self {
        Self { base: StyleBase::construct(), target_type: Cell::new(None), based_on: RefCell::new(None) }
    }

    /// Creates a control theme without a target type.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a control theme for the given control type.
    pub fn with_target_type(target_type: &'static TypeInfo) -> Ref<Self> {
        let theme = Self::new();
        theme.set_target_type(Some(target_type));
        theme
    }

    /// Creates a control theme for control class `T`.
    pub fn for_type<T: ObjectType>() -> Ref<Self> {
        Self::with_target_type(T::TYPE)
    }

    /// The type for which this control theme is intended.
    pub fn target_type(&self) -> Option<&'static TypeInfo> {
        self.target_type.get()
    }

    pub fn set_target_type(&self, value: Option<&'static TypeInfo>) {
        self.target_type.set(value)
    }

    /// A control theme that is the basis of the current theme.
    pub fn based_on(&self) -> Option<Ref<ControlTheme>> {
        self.based_on.borrow().clone()
    }

    pub fn set_based_on(&self, value: Option<Ref<ControlTheme>>) {
        *self.based_on.borrow_mut() = value;
    }

    /// Creates a control theme for the given control type with setters.
    pub fn with_setters(
        target_type: &'static TypeInfo,
        setters: impl IntoIterator<Item = Rc<Setter>>,
    ) -> Ref<Self> {
        let theme = Self::with_target_type(target_type);
        for setter in setters {
            theme.add_setter(setter);
        }
        theme
    }

    /// As [`try_attach_checked`](Self::try_attach_checked); a duplicate
    /// setter panics with the message of the error.
    #[allow(dead_code)]
    pub(crate) fn try_attach(&self, target: &StyledElement, type_: FrameType) -> SelectorMatchResult {
        match self.try_attach_checked(target, type_) {
            Ok(result) => result,
            Err(error) => panic!("{error}"),
        }
    }

    /// Attaches the theme to `target` if it applies to it. A theme with two
    /// setters for the same property is an error.
    pub(crate) fn try_attach_checked(
        &self,
        target: &StyledElement,
        type_: FrameType,
    ) -> Result<SelectorMatchResult, DuplicateSetterError> {
        debug_assert!(matches!(type_, FrameType::Theme | FrameType::TemplatedParentTheme));

        let Some(target_type) = self.target_type.get() else {
            panic!("ControlTheme has no TargetType.");
        };

        if self.has_setters_or_animations() && target_type.is_assignable_from(target.style_key()) {
            self.try_attach_instance(target, None, type_, true)?;
            return Ok(SelectorMatchResult::AlwaysThisType);
        }

        Ok(SelectorMatchResult::NeverThisType)
    }
}
