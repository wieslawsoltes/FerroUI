//! Port of `MarkupExtensions/RelativeSourceExtension.cs`.

use ferroui_base::data::{RelativeSource, RelativeSourceMode, TreeType};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::{ferro_markup_type, TypeInfo};
use std::cell::Cell;
use std::rc::Rc;

/// `{RelativeSource Mode}`: describes the source of a binding relative to
/// its target.
pub struct RelativeSourceExtension {
    mode: Cell<RelativeSourceMode>,
    ancestor_type: Cell<Option<&'static TypeInfo>>,
    tree: Cell<TreeType>,
    ancestor_level: Cell<i32>,
}

impl RelativeSourceExtension {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            mode: Cell::new(RelativeSourceMode::FindAncestor),
            ancestor_type: Cell::new(None),
            tree: Cell::new(TreeType::Visual),
            ancestor_level: Cell::new(1),
        })
    }

    /// Creates the extension with a mode.
    pub fn with_mode(mode: RelativeSourceMode) -> Rc<Self> {
        let this = Self::new();
        this.set_mode(mode);
        this
    }

    /// Creates the relative source the extension describes.
    ///
    /// # Panics
    /// Panics if the ancestor level is less than 1. Use
    /// [`try_provide_value`](Self::try_provide_value) to handle it.
    pub fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Rc<RelativeSource> {
        crate::throw(self.try_provide_value(service_provider))
    }

    /// [`provide_value`](Self::provide_value) without the panic: an
    /// ancestor level less than 1 is an error.
    pub fn try_provide_value(
        &self,
        _service_provider: &Rc<dyn IServiceProvider>,
    ) -> Result<Rc<RelativeSource>, crate::XamlLoadException> {
        if self.ancestor_level() <= 0 {
            return Err(crate::XamlLoadException::with_message("AncestorLevel may not be set to less than 1."));
        }
        let result = RelativeSource::new(self.mode());
        result.set_ancestor_type(self.ancestor_type());
        result.set_ancestor_level(self.ancestor_level());
        result.set_tree(self.tree());
        Ok(result)
    }

    pub fn mode(&self) -> RelativeSourceMode {
        self.mode.get()
    }

    pub fn set_mode(&self, value: RelativeSourceMode) {
        self.mode.set(value);
    }

    pub fn ancestor_type(&self) -> Option<&'static TypeInfo> {
        self.ancestor_type.get()
    }

    pub fn set_ancestor_type(&self, value: Option<&'static TypeInfo>) {
        self.ancestor_type.set(value);
    }

    pub fn tree(&self) -> TreeType {
        self.tree.get()
    }

    pub fn set_tree(&self, value: TreeType) {
        self.tree.set(value);
    }

    pub fn ancestor_level(&self) -> i32 {
        self.ancestor_level.get()
    }

    pub fn set_ancestor_level(&self, value: i32) {
        self.ancestor_level.set(value);
    }
}

crate::identity_eq!(RelativeSourceExtension);

ferro_markup_type!(class RelativeSourceExtension {
    this: Rc<RelativeSourceExtension>,
    handles: [RelativeSourceExtension, Rc<RelativeSourceExtension>, Option<Rc<RelativeSourceExtension>>],
    constructors: [
        () => RelativeSourceExtension::new,
        (RelativeSourceMode) => RelativeSourceExtension::with_mode,
    ],
    properties: [
        Mode: RelativeSourceMode { get: RelativeSourceExtension::mode, set: RelativeSourceExtension::set_mode }
            [ConstructorArgument("mode")],
        AncestorType: Option<&'static TypeInfo> {
            get: RelativeSourceExtension::ancestor_type,
            set: RelativeSourceExtension::set_ancestor_type
        },
        Tree: TreeType { get: RelativeSourceExtension::tree, set: RelativeSourceExtension::set_tree },
        AncestorLevel: i32 {
            get: RelativeSourceExtension::ancestor_level,
            set: RelativeSourceExtension::set_ancestor_level
        },
    ],
    methods: [
        try fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<RelativeSource> =>
            |this: &Rc<RelativeSourceExtension>, service_provider: Rc<dyn IServiceProvider>| {
                this.try_provide_value(&service_provider)
            },
    ],
});
