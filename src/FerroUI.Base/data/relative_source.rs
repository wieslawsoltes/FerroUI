use crate::TypeInfo;
use std::cell::Cell;
use std::rc::Rc;

/// Defines the mode of a [`RelativeSource`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelativeSourceMode {
    /// The binding will be to the element's data context.
    DataContext,
    /// The binding will be to the element's templated parent.
    TemplatedParent,
    /// The binding will be to the element itself.
    SelfMode,
    /// The binding will be to an ancestor of the element in the visual or
    /// logical tree.
    FindAncestor,
}

/// The type of tree via which to track an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeType {
    /// The visual tree.
    Visual,
    /// The logical tree.
    Logical,
}

/// Describes the location of a binding source, relative to the binding
/// target.
///
/// A relative source is a mutable reference object: it is created with
/// [`RelativeSource::empty`] or [`RelativeSource::new`], configured property
/// by property (or with the chaining `with_*` forms) and shared as
/// `Rc<RelativeSource>`. Handles compare by identity. A binding reads it
/// when it is instantiated on a target.
#[derive(Debug)]
pub struct RelativeSource {
    ancestor_level: Cell<i32>,
    ancestor_type: Cell<Option<&'static TypeInfo>>,
    mode: Cell<RelativeSourceMode>,
    tree: Cell<TreeType>,
}

impl Default for RelativeSource {
    /// A relative source in find-ancestor mode.
    fn default() -> Self {
        Self {
            ancestor_level: Cell::new(1),
            ancestor_type: Cell::new(None),
            mode: Cell::new(RelativeSourceMode::FindAncestor),
            tree: Cell::new(TreeType::Visual),
        }
    }
}

/// Compares by identity (reference equality), as the reference type this
/// mirrors.
impl PartialEq for RelativeSource {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl RelativeSource {
    /// Creates a relative source in find-ancestor mode (the parameterless
    /// constructor).
    pub fn empty() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Creates a relative source with the given mode.
    pub fn new(mode: RelativeSourceMode) -> Rc<Self> {
        Self::empty().with_mode(mode)
    }

    /// The level of ancestor to look for when in find-ancestor mode. Use the
    /// default value of 1 to look for the first ancestor of the specified
    /// type.
    pub fn ancestor_level(&self) -> i32 {
        self.ancestor_level.get()
    }

    /// Sets the ancestor level. Panics if it is not positive.
    pub fn set_ancestor_level(&self, value: i32) {
        if value <= 0 {
            panic!("AncestorLevel may not be set to less than 1.");
        }
        self.ancestor_level.set(value);
    }

    /// The chaining form of [`set_ancestor_level`](Self::set_ancestor_level).
    pub fn with_ancestor_level(self: Rc<Self>, value: i32) -> Rc<Self> {
        self.set_ancestor_level(value);
        self
    }

    /// The type of ancestor to look for when in
    /// [`RelativeSourceMode::FindAncestor`] mode.
    pub fn ancestor_type(&self) -> Option<&'static TypeInfo> {
        self.ancestor_type.get()
    }

    pub fn set_ancestor_type(&self, value: Option<&'static TypeInfo>) {
        self.ancestor_type.set(value)
    }

    /// The chaining form of [`set_ancestor_type`](Self::set_ancestor_type).
    pub fn with_ancestor_type(self: Rc<Self>, value: Option<&'static TypeInfo>) -> Rc<Self> {
        self.set_ancestor_type(value);
        self
    }

    /// A value that describes the type of relative source lookup.
    pub fn mode(&self) -> RelativeSourceMode {
        self.mode.get()
    }

    pub fn set_mode(&self, value: RelativeSourceMode) {
        self.mode.set(value)
    }

    /// The chaining form of [`set_mode`](Self::set_mode).
    pub fn with_mode(self: Rc<Self>, value: RelativeSourceMode) -> Rc<Self> {
        self.set_mode(value);
        self
    }

    /// The tree in which ancestors are looked up.
    pub fn tree(&self) -> TreeType {
        self.tree.get()
    }

    pub fn set_tree(&self, value: TreeType) {
        self.tree.set(value)
    }

    /// The chaining form of [`set_tree`](Self::set_tree).
    pub fn with_tree(self: Rc<Self>, value: TreeType) -> Rc<Self> {
        self.set_tree(value);
        self
    }
}
