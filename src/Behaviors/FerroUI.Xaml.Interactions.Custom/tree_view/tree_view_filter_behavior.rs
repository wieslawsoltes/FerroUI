//! Port of `TreeView/TreeViewFilterBehavior.cs`.

use super::tree_view_filter_helper::TreeViewFilterHelper;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElement, TextInputEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl, FerroProperty,
    Nullable, Ref, StaticType, StyledElementImpl, StyledProperty, WeakRef,
};
use ferroui_controls::{Control, TextBox, TextChangedEventArgs, TreeView};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::RefCell;

/// The search box a behavior listens to and its handlers.
struct SearchBoxHandlers {
    search_box: WeakRef<TextBox>,
    text_input: RoutedEventHandlerToken,
    text_changed: RoutedEventHandlerToken,
}

/// Filters [`TreeView`] items based on the text of a search box.
#[repr(C)]
pub struct TreeViewFilterBehavior {
    base: StyledElementBehaviorOfT,
    search_box_handlers: RefCell<Option<SearchBoxHandlers>>,
}

ferro_class!(TreeViewFilterBehavior: StyledElementBehaviorOfT);
ferro_class_info!(TreeViewFilterBehavior {
    new: TreeViewFilterBehavior::new,
    markup: {
        property_attributes: [
            SearchBox: [ResolveByName],
            NoMatchesControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(TreeViewFilterBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl TreeViewFilterBehavior {
        /// Identifies the `SearchBox` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the search box, so the
        /// value is an element reference.
        pub fn search_box_property() -> StyledProperty<Option<ElementRef<TextBox>>> {
            ValueTypes::register_element_ref::<TextBox>();
            FerroProperty::register::<TreeViewFilterBehavior, _>("SearchBox", None)
        }

        /// Identifies the `NoMatchesControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the control, so the
        /// value is an element reference.
        pub fn no_matches_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<TreeViewFilterBehavior, _>("NoMatchesControl", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for TreeViewFilterBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            this.remove_search_box_handlers();

            let Some(search_box) = this.search_box() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let text_input = search_box.add_handler_with(
                InputElement::text_input_event(),
                move |_sender: &Interactive, _e: &TextInputEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.search_box_text_changed();
                    }
                },
                RoutingStrategies::BUBBLE,
                false,
            );
            let behavior = this.to_ref().downgrade();
            let text_changed = search_box.add_handler_with(
                TextBox::text_changed_event(),
                move |_sender: &Interactive, _e: &TextChangedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.search_box_text_changed();
                    }
                },
                RoutingStrategies::BUBBLE,
                false,
            );

            *this.search_box_handlers.borrow_mut() =
                Some(SearchBoxHandlers { search_box: search_box.downgrade(), text_input, text_changed });
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.remove_search_box_handlers();
        }
    }
}

impl TreeViewFilterBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<TreeView as StaticType>::TYPE),
            search_box_handlers: RefCell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the search box control. Markup resolves a name given for it to the named
    /// element.
    pub fn search_box(&self) -> Option<Ref<TextBox>> {
        ElementRef::resolve(&self.get_value(Self::search_box_property()))
    }

    pub fn set_search_box(&self, value: impl Into<Nullable<TextBox>>) {
        self.set_value(Self::search_box_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the control displayed when no matches are found. Markup resolves a name given
    /// for it to the named element.
    pub fn no_matches_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::no_matches_control_property()))
    }

    pub fn set_no_matches_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::no_matches_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Removes the handlers from the search box they were added to (the reference removes them
    /// from the current search box).
    // Deviation (DEVIATIONS.md, Behaviors): handlers are removed by their tokens from the element
    // they were added to, which the reference does through the current search box.
    fn remove_search_box_handlers(&self) {
        let handlers = self.search_box_handlers.borrow_mut().take();
        if let Some(handlers) = handlers {
            if let Some(search_box) = handlers.search_box.upgrade() {
                search_box.remove_handler(InputElement::text_input_event(), handlers.text_input);
                search_box.remove_handler(TextBox::text_changed_event(), handlers.text_changed);
            }
        }
    }

    fn search_box_text_changed(&self) {
        let Some(associated_object) = self.associated_object_as::<TreeView>() else {
            return;
        };

        let query = self.search_box().and_then(|search_box| search_box.text()).unwrap_or_default();
        let count = TreeViewFilterHelper::filter(&associated_object, &query);

        if let Some(no_matches_control) = self.no_matches_control() {
            no_matches_control.set_is_visible(count == 0);
        }
    }
}
