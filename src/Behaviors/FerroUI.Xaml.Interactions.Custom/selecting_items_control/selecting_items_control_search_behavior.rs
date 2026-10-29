//! Port of `SelectingItemsControl/SelectingItemsControlSearchBehavior.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, RouteHandler, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_enum, ferro_properties, instantiate, BoxedValue, ElementRef,
    FerroObjectImpl, FerroProperty, Nullable, Ref, StaticType, StyledElementImpl, StyledProperty, WeakRef,
};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{TabItem, TextBlock, TextBox};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

/// Sort order for tab items.
///
/// Deviation (DEVIATIONS.md, Behaviors): a nested enumeration of
/// `SelectingItemsControlSearchBehavior` in the reference; Rust has no nested types, so it is a
/// type of the module with the same name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// Sort items ascending.
    #[default]
    Ascending,
    /// Sort items descending.
    Descending,
}

ferro_markup_enum!(SortDirection { Ascending, Descending });

impl std::fmt::Display for SortDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ascending => "Ascending",
            Self::Descending => "Descending",
        })
    }
}

/// Filters [`SelectingItemsControl`] items based on the text of a search box: the tab items whose
/// header contains the text stay visible, the first of them is selected.
#[repr(C)]
pub struct SelectingItemsControlSearchBehavior {
    base: StyledElementBehaviorOfT,
    // The handlers added to the search box (`RemoveHandler` of the reference): the text input and
    // the text changed events.
    search_box_handlers: RefCell<Option<(WeakRef<TextBox>, RoutedEventHandlerToken, RoutedEventHandlerToken)>>,
}

ferro_class!(SelectingItemsControlSearchBehavior: StyledElementBehaviorOfT);
ferro_class_info!(SelectingItemsControlSearchBehavior {
    new: SelectingItemsControlSearchBehavior::new,
    markup: {
        property_attributes: [
            SearchBox: [ResolveByName],
            NoMatchesControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SelectingItemsControlSearchBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SelectingItemsControlSearchBehavior {
        /// Identifies the `SearchBox` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the text box, so the
        /// value is an element reference.
        pub fn search_box_property() -> StyledProperty<Option<ElementRef<TextBox>>> {
            ValueTypes::register_element_ref::<TextBox>();
            FerroProperty::register::<SelectingItemsControlSearchBehavior, _>("SearchBox", None)
        }

        /// Identifies the `NoMatchesControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the reference registers the property for a text
        /// block and declares its accessor for any control; the port types both as the
        /// registered property, and holds the text block by an element reference.
        pub fn no_matches_control_property() -> StyledProperty<Option<ElementRef<TextBlock>>> {
            ValueTypes::register_element_ref::<TextBlock>();
            FerroProperty::register::<SelectingItemsControlSearchBehavior, _>("NoMatchesControl", None)
        }

        /// Identifies the `EnableSorting` property.
        pub fn enable_sorting_property() -> StyledProperty<bool> {
            FerroProperty::register::<SelectingItemsControlSearchBehavior, _>("EnableSorting", false)
        }

        /// Identifies the `SortOrder` property.
        pub fn sort_order_property() -> StyledProperty<SortDirection> {
            FerroProperty::register::<SelectingItemsControlSearchBehavior, _>("SortOrder", SortDirection::Ascending)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for SelectingItemsControlSearchBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            if let Some(search_box) = this.search_box() {
                this.remove_search_box_handlers();

                let behavior = this.to_ref().downgrade();
                let handler: RouteHandler = Rc::new(move |_sender: &Interactive, e: &dyn IRoutedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.search_box_text_changed(e);
                    }
                });
                let text_input = search_box.add_route_handler(
                    InputElement::text_input_event(),
                    handler.clone(),
                    RoutingStrategies::BUBBLE,
                    false,
                );
                let text_changed =
                    search_box.add_route_handler(TextBox::text_changed_event(), handler, RoutingStrategies::BUBBLE, false);
                *this.search_box_handlers.borrow_mut() = Some((search_box.downgrade(), text_input, text_changed));
            }

            this.sort_items();
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.remove_search_box_handlers();
        }
    }
}

impl SelectingItemsControlSearchBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<SelectingItemsControl as StaticType>::TYPE),
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
    pub fn no_matches_control(&self) -> Option<Ref<TextBlock>> {
        ElementRef::resolve(&self.get_value(Self::no_matches_control_property()))
    }

    pub fn set_no_matches_control(&self, value: impl Into<Nullable<TextBlock>>) {
        self.set_value(Self::no_matches_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets a value indicating whether items should be sorted.
    pub fn enable_sorting(&self) -> bool {
        self.get_value(Self::enable_sorting_property())
    }

    pub fn set_enable_sorting(&self, value: bool) {
        self.set_value(Self::enable_sorting_property(), value)
    }

    /// Gets or sets the sort order for the items.
    pub fn sort_order(&self) -> SortDirection {
        self.get_value(Self::sort_order_property())
    }

    pub fn set_sort_order(&self, value: SortDirection) {
        self.set_value(Self::sort_order_property(), value)
    }

    fn remove_search_box_handlers(&self) {
        let handlers = self.search_box_handlers.borrow_mut().take();
        if let Some((search_box, text_input, text_changed)) = handlers {
            if let Some(search_box) = search_box.upgrade() {
                search_box.remove_handler(InputElement::text_input_event(), text_input);
                search_box.remove_handler(TextBox::text_changed_event(), text_changed);
            }
        }
    }

    /// The header of a tab item as text (`(x as TabItem)?.Header?.ToString()`).
    fn header_of(item: &Option<BoxedValue>) -> Option<String> {
        let tab_item = item.as_ref().and_then(object_of).and_then(|item| item.cast::<TabItem>())?;
        let header = tab_item.header()?;
        Some(ValueTypes::to_display_string(Some(&header)))
    }

    fn sort_items(&self) {
        let Some(associated_object) = self.associated_object_as::<SelectingItemsControl>() else {
            return;
        };
        if !self.enable_sorting() {
            return;
        }

        let items = associated_object.items();
        // Deviation (DEVIATIONS.md, Behaviors): sorting the read-only view of an items source
        // throws in the reference; the port leaves it unsorted.
        if items.is_read_only() {
            return;
        }

        // Deviation (DEVIATIONS.md, Behaviors): the headers compare by their characters (the
        // reference compares them with the rules of the current culture), and two items without a
        // header are equal (the comparer of the reference orders them both ways), so the order is
        // total.
        let ascending = self.sort_order() == SortDirection::Ascending;
        let mut sorted: Vec<(Option<String>, Option<BoxedValue>)> =
            items.to_vec().into_iter().map(|item| (Self::header_of(&item), item)).collect();
        sorted.sort_by(|(x, _), (y, _)| match (x, y) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => {
                if ascending {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            (Some(_), None) => {
                if ascending {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (Some(x), Some(y)) => {
                if ascending {
                    x.cmp(y)
                } else {
                    y.cmp(x)
                }
            }
        });

        for (index, (_, item)) in sorted.into_iter().enumerate() {
            items.set(index, item);
        }
    }

    fn search_box_text_changed(&self, _e: &dyn IRoutedEventArgs) {
        let Some(associated_object) = self.associated_object_as::<SelectingItemsControl>() else {
            return;
        };

        let query = self.search_box().and_then(|search_box| search_box.text()).unwrap_or_default().to_lowercase();
        let mut visible_count = 0;

        self.sort_items();

        let tab_items: Vec<(BoxedValue, Ref<TabItem>)> = associated_object
            .items()
            .to_vec()
            .into_iter()
            .flatten()
            .filter_map(|item| {
                let tab_item = object_of(&item).and_then(|object| object.cast::<TabItem>())?;
                Some((item, tab_item))
            })
            .collect();

        for (_, item) in &tab_items {
            let header = item
                .header()
                .map(|header| ValueTypes::to_display_string(Some(&header)).to_lowercase())
                .unwrap_or_default();
            let visible = header.contains(&query);
            item.set_is_visible(visible);
            if visible {
                visible_count += 1;
            }
        }

        associated_object
            .set_selected_item(tab_items.iter().find(|(_, item)| item.is_visible()).map(|(value, _)| value.clone()));

        if let Some(no_matches_control) = self.no_matches_control() {
            no_matches_control.set_is_visible(visible_count == 0);
        }
    }
}
