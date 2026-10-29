//! Port of `TreeView/ApplyTreeViewFilterAction.cs`.

use super::tree_view_filter_helper::TreeViewFilterHelper;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::TreeView;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Filters a [`TreeView`] using the provided query string.
#[repr(C)]
pub struct ApplyTreeViewFilterAction {
    base: StyledElementAction,
}

ferro_class!(ApplyTreeViewFilterAction: StyledElementAction);
ferro_class_info!(ApplyTreeViewFilterAction {
    new: ApplyTreeViewFilterAction::new,
    markup: {
        content: Query,
        property_attributes: [
            TreeView: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ApplyTreeViewFilterAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ApplyTreeViewFilterAction {
        /// Identifies the `TreeView` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the tree view, so the
        /// value is an element reference.
        pub fn tree_view_property() -> StyledProperty<Option<ElementRef<TreeView>>> {
            ValueTypes::register_element_ref::<TreeView>();
            FerroProperty::register::<ApplyTreeViewFilterAction, _>("TreeView", None)
        }

        /// Identifies the `Query` property.
        pub fn query_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ApplyTreeViewFilterAction, _>("Query", None)
        }
    }
}

impl StyledElementActionImpl for ApplyTreeViewFilterAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let tree_view = this
            .tree_view()
            .or_else(|| sender.and_then(object_of).and_then(|sender| sender.cast::<TreeView>()));
        let Some(tree_view) = tree_view else {
            return Some(Rc::new(false));
        };

        let query = this.query().unwrap_or_default();
        TreeViewFilterHelper::filter(&tree_view, &query);
        Some(Rc::new(true))
    }
}

impl ApplyTreeViewFilterAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the tree view to filter. Markup resolves a name given for it to the named
    /// element.
    pub fn tree_view(&self) -> Option<Ref<TreeView>> {
        ElementRef::resolve(&self.get_value(Self::tree_view_property()))
    }

    pub fn set_tree_view(&self, value: impl Into<Nullable<TreeView>>) {
        self.set_value(Self::tree_view_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the filter query string.
    pub fn query(&self) -> Option<String> {
        self.get_value(Self::query_property())
    }

    pub fn set_query(&self, value: Option<String>) {
        self.set_value(Self::query_property(), value)
    }
}
