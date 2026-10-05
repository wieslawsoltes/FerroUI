use super::{
    Flyout, FlyoutBaseImpl, MenuFlyoutPresenter, PopupFlyoutBase, PopupFlyoutBaseImpl, PopupFlyoutBaseImplExt,
};
use crate::item_collection::ItemCollection;
use crate::items_source::{IItemsList, ItemsChangedHandler, ItemsSource};
use crate::templates::IDataTemplate;
use crate::{Control, ItemsControl, ItemsSourceView};
use ferroui_base::controls::Classes;
use ferroui_base::styling::ControlTheme;
use ferroui_base::utilities::CancelEventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref, StyledElement, StyledProperty,
};
use std::any::Any;
use std::cell::OnceCell;
use std::rc::Rc;

/// A flyout that displays a menu.
#[repr(C)]
pub struct MenuFlyout {
    base: PopupFlyoutBase,
    items: ItemCollection,
    /// The items as the items source of the presenter.
    presenter_items_source: ItemsSource,
    classes: OnceCell<Classes>,
}

ferro_class!(MenuFlyout: PopupFlyoutBase);
ferro_class_info!(MenuFlyout {
    new: MenuFlyout::new,
    markup: {
        content: Items,
        properties: [
            Items: ItemCollection { get: MenuFlyout::items },
        ],
    },
});
ferro_impl_classes!(MenuFlyout: FlyoutBaseImpl);

impl FerroObjectImpl for MenuFlyout {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::items_source_property().as_property() {
            this.items.set_items_source(change.get_new_value::<Option<ItemsSource>>());
        }
    }
}

impl PopupFlyoutBaseImpl for MenuFlyout {
    fn create_presenter(this: &Self) -> Ref<Control> {
        let presenter = MenuFlyoutPresenter::new();
        presenter.set_items_source(Some(this.presenter_items_source.clone()));
        presenter.bind_indexer(
            &ItemsControl::item_template_property().bind(),
            &this.indexer(&Self::item_template_property().bind()),
        );
        presenter.bind_indexer(
            &ItemsControl::item_container_theme_property().bind(),
            &this.indexer(&Self::item_container_theme_property().bind()),
        );
        presenter.upcast()
    }

    fn on_opening(this: &Self, args: &CancelEventArgs) {
        if let Some(presenter) = this.popup().child() {
            if let Some(classes) = this.classes.get() {
                PopupFlyoutBase::set_presenter_classes(Some(&presenter), classes);
            }

            if let Some(theme) = this.flyout_presenter_theme() {
                presenter.set_value(StyledElement::theme_property(), Some(theme));
            }
        }

        Self::parent_on_opening(this, args);
    }
}

/// The items of a menu flyout as an untyped, notifying list: what the items
/// collection is to an items control that uses it as its items source.
struct ItemCollectionList(Rc<ItemsSourceView>);

impl IItemsList for ItemCollectionList {
    fn count(&self) -> usize {
        self.0.count()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self.0.get_at(index)
    }

    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        self.0.index_of(item)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.0.add_collection_changed(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.0.remove_collection_changed(token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

ferro_properties! {
    impl MenuFlyout {
        /// Defines the `ItemsSource` property.
        pub fn items_source_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<MenuFlyout, _>("ItemsSource", None)
        }

        /// Defines the `ItemTemplate` property.
        pub fn item_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<MenuFlyout, _>("ItemTemplate", None)
        }

        /// Defines the `ItemContainerTheme` property.
        pub fn item_container_theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            ItemsControl::item_container_theme_property().add_owner::<MenuFlyout>()
        }

        /// Defines the `FlyoutPresenterTheme` property.
        pub fn flyout_presenter_theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            Flyout::flyout_presenter_theme_property().add_owner::<MenuFlyout>()
        }
    }
}

impl MenuFlyout {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        let items = ItemCollection::new();
        let presenter_items_source = ItemsSource::new(Rc::new(ItemCollectionList(items.view().clone())));
        Self { base: PopupFlyoutBase::construct(), items, presenter_items_source, classes: OnceCell::new() }
    }

    /// Creates a menu flyout.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The classes collection to apply to the flyout presenter this flyout
    /// is hosting.
    pub fn flyout_presenter_classes(&self) -> Classes {
        self.classes.get_or_init(Classes::new).clone()
    }

    /// The items of the menu flyout.
    pub fn items(&self) -> ItemCollection {
        self.items.clone()
    }

    /// Gets or sets the items of the menu flyout.
    pub fn items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_source_property())
    }

    pub fn set_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::items_source_property(), value)
    }

    /// Gets or sets the template used for the items.
    pub fn item_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::item_template_property())
    }

    pub fn set_item_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::item_template_property(), value)
    }

    /// Gets or sets the control theme that is applied to the container
    /// element generated for each item.
    pub fn item_container_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::item_container_theme_property())
    }

    pub fn set_item_container_theme(&self, value: impl Into<Nullable<ControlTheme>>) {
        self.set_value(Self::item_container_theme_property(), value.into().0)
    }

    /// Gets or sets the control theme that is applied to the container
    /// element generated for the flyout presenter.
    pub fn flyout_presenter_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::flyout_presenter_theme_property())
    }

    pub fn set_flyout_presenter_theme(&self, value: impl Into<Nullable<ControlTheme>>) {
        self.set_value(Self::flyout_presenter_theme_property(), value.into().0)
    }
}
