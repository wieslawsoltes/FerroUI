//! WinUI's `TargetPropertyPath` (`Microsoft.UI.Xaml.TargetPropertyPath`,
//! `dxaml/xcp/core/core/elements/TargetPropertyPath.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md): the object and the property a [`Setter`](super::Setter) of a visual state sets.
//!
//! An addition (docs/porting/winui-calculator-gaps.md, section 5.2): upstream setters have no
//! target.

use crate::{FerroObject, FerroProperty, Ref, WeakRef};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// The target of a [`Setter`](super::Setter) of a visual state: an object and a property of it.
///
/// Markup writes it as the text of `Setter.Target`: `Element.Property` or
/// `Element.(Owner.Property)` for an attached property. The compiler keeps the name of the element
/// ([`target_name`](Self::target_name)) and resolves the property against the type of the
/// element; the property is the [`property`](super::Setter::property) of the setter. The element
/// is found by its name in the name scope of the template or the document when the state is
/// applied, as WinUI's `CTargetPropertyPath` does (`m_targetName`, `ResolveTargetForVisualStateSetter`),
/// so a setter may name an element declared after it. An explicit [`target`](Self::target)
/// object, set in code or markup, is used instead of the name (WinUI's manual target).
///
/// The target is held weakly: a setter of a visual state usually targets an element of the tree
/// that holds the visual states, and so the setter would keep that tree alive.
#[derive(Default)]
pub struct TargetPropertyPath {
    path: RefCell<String>,
    target_name: RefCell<String>,
    target: RefCell<Option<WeakRef<FerroObject>>>,
    property: Cell<Option<&'static FerroProperty>>,
}

/// Compares by identity (reference equality), as the reference type this mirrors.
impl PartialEq for TargetPropertyPath {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TargetPropertyPath {
    /// Creates a path without a target or a property.
    pub fn empty() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Creates the path of `property` of `target` (WinUI's constructor that takes a property,
    /// with the target set).
    pub fn new(target: &FerroObject, property: &'static FerroProperty) -> Rc<Self> {
        let path = Self::default();
        *path.target.borrow_mut() = Some(target.to_weak());
        path.property.set(Some(property));
        *path.path.borrow_mut() = property.name().to_string();
        Rc::new(path)
    }

    /// The target object, if it is set and alive.
    pub fn target(&self) -> Option<Ref<FerroObject>> {
        self.target.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub fn set_target(&self, value: Option<Ref<FerroObject>>) {
        *self.target.borrow_mut() = value.map(|target| target.downgrade());
    }

    /// The name of the target element, resolved in a name scope when the setter applies; empty
    /// when the target is an explicit object or not set.
    pub fn target_name(&self) -> String {
        self.target_name.borrow().clone()
    }

    pub fn set_target_name(&self, value: impl Into<String>) {
        *self.target_name.borrow_mut() = value.into();
    }

    /// The text of the path of the property (`Property` or `(Owner.Property)`), as markup wrote
    /// it.
    pub fn path(&self) -> String {
        self.path.borrow().clone()
    }

    pub fn set_path(&self, value: impl Into<String>) {
        *self.path.borrow_mut() = value.into();
    }

    /// The property of the target, when the path is resolved. A setter of a visual state keeps
    /// the resolved property in its own `Property`.
    pub fn property(&self) -> Option<&'static FerroProperty> {
        self.property.get()
    }

    pub fn set_property(&self, value: Option<&'static FerroProperty>) {
        self.property.set(value);
    }
}

impl fmt::Debug for TargetPropertyPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let target = self.target().map(|target| target.get_type().name().to_string());
        f.debug_struct("TargetPropertyPath")
            .field("target", &target)
            .field("target_name", &*self.target_name.borrow())
            .field("path", &*self.path.borrow())
            .finish()
    }
}
