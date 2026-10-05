use super::{DefaultPageDataTemplate, NavigationType, Page, PageImpl};
use crate::primitives::TemplatedControlImpl;
use crate::templates::IDataTemplate;
use crate::{
    Control, ControlImpl, IItemsList, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource, ItemsView,
};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElement, StyledElementImpl,
    StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The collection of child pages of a multi-page: a shared handle to an
/// observable list of pages.
pub type PageList = FerroList<Ref<Page>>;

/// The signature of a handler of the `PagesChanged` event.
pub type PagesChangedHandler = dyn for<'a> Fn(&NotifyCollectionChangedEventArgs<'a, Ref<Page>>);

/// Abstract base class for pages that host a collection of child pages.
#[repr(C)]
pub struct MultiPage {
    base: Page,
    current_page_changed: HandlerList<dyn Fn()>,
    pages_changed: HandlerList<PagesChangedHandler>,
    /// The collection whose changes are observed and the token of the
    /// subscription.
    pages_subscription: RefCell<Option<(PageList, u64)>>,
    /// Whether the value of the `Pages` property counts as a collection that
    /// notifies of changes. See [`MultiPage::set_observes_pages`].
    observes_pages: Cell<bool>,
}

ferro_class! {
    MultiPage: Page, virtuals MultiPageImpl: PageImpl {
        /// Called when the active child page changes.
        ///
        /// The base class calls this method in the following situations,
        /// passing the corresponding navigation type:
        ///
        /// - [`NavigationType::Replace`]: the `Pages` collection was
        ///   assigned, a new item was added, or the active page changed for
        ///   any reason not covered below.
        /// - [`NavigationType::Remove`]: the currently active page was
        ///   removed from the `Pages` collection or the collection was reset
        ///   and the active page is no longer present. Subclasses should
        ///   select a replacement page.
        ///
        /// Subclasses may also call this method directly with any navigation
        /// type appropriate for their own navigation model.
        fn update_active_page_with(this, navigation_type: NavigationType);
    }
}

ferro_class_info!(MultiPage {
    markup: {
        content: Pages,
    },
});

ferro_impl_classes!(
    MultiPage: StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl FerroObjectImpl for MultiPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::pages_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<PageList>>();
            if old_value == new_value {
                return;
            }

            if let Some(old_notify_collection) = &old_value {
                this.unsubscribe_from_pages(old_notify_collection);
            }

            let logical_children = StyledElement::logical_children(this);
            logical_children.clear();

            if let Some(new_items) = &new_value {
                for page in new_items.snapshot().iter() {
                    logical_children.add(page.clone().upcast());
                }
            }

            if let Some(new_notify_collection) = &new_value {
                this.subscribe_to_pages(new_notify_collection);
            }

            if new_value.is_some() {
                this.update_active_page();
            } else {
                this.set_current_value(Page::current_page_property(), None);
            }
        } else if change.property() == Page::current_page_property().as_property() {
            for (_, handler) in this.current_page_changed.snapshot().iter() {
                handler();
            }
        }
    }
}

impl Drop for MultiPage {
    fn drop(&mut self) {
        // A page that is dropped while it observes its pages (it never left the visual tree, or was
        // never in it) releases the subscription: the collection may outlive the page.
        if let Some((collection, token)) = self.pages_subscription.get_mut().take() {
            collection.remove_collection_changed(token);
        }
    }
}

impl VisualImpl for MultiPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if let Some(collection) = this.pages() {
            this.unsubscribe_from_pages(&collection);
            this.subscribe_to_pages(&collection);
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        if let Some(collection) = this.pages() {
            this.unsubscribe_from_pages(&collection);
        }
    }
}

impl PageImpl for MultiPage {
    /// Routes no-argument calls to
    /// [`update_active_page_with`](MultiPageImpl::update_active_page_with)
    /// using [`NavigationType::Replace`] as the default. Subclasses must
    /// override the typed member, not this one.
    fn update_active_page(this: &Self) {
        this.update_active_page_with(NavigationType::Replace)
    }
}

impl MultiPageImpl for MultiPage {
    fn update_active_page_with(_this: &Self, _navigation_type: NavigationType) {}
}

ferro_properties! {
    impl MultiPage {
        /// Defines the `Pages` property.
        pub fn pages_property() -> StyledProperty<Option<PageList>> {
            FerroProperty::register::<MultiPage, _>("Pages", None)
        }

        /// Defines the `ItemsSource` property.
        pub fn items_source_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<MultiPage, _>("ItemsSource", None)
        }

        /// Defines the `PageTemplate` property.
        pub fn page_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            let default_template: Rc<dyn IDataTemplate> = DefaultPageDataTemplate::new();
            FerroProperty::register::<MultiPage, _>("PageTemplate", Some(default_template))
        }
    }
}

impl MultiPage {
    fn static_constructor() {
        // Lets a binding deliver the value of the `Pages` property to the
        // items source of the inner tab or carousel control.
        ValueTypes::register_conversion::<PageList, ItemsSource>(|pages| Some(Self::items_source_of(pages)));
        ValueTypes::register_conversion::<PageList, Option<ItemsSource>>(|pages| Some(Some(Self::items_source_of(pages))));
        ValueTypes::register_conversion::<Option<PageList>, Option<ItemsSource>>(|pages| {
            Some(pages.as_ref().map(Self::items_source_of))
        });
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Page::construct(),
            current_page_changed: HandlerList::new(),
            pages_changed: HandlerList::new(),
            pages_subscription: RefCell::new(None),
            observes_pages: Cell::new(true),
        }
    }

    /// Gets or sets the collection of child pages.
    ///
    /// This control subscribes to the changes of the assigned collection.
    /// The subscription is released when a new collection is assigned or
    /// when the control is detached from the visual tree.
    pub fn pages(&self) -> Option<PageList> {
        self.get_value(Self::pages_property())
    }

    pub fn set_pages(&self, value: Option<PageList>) {
        self.set_value(Self::pages_property(), value)
    }

    /// Gets or sets a view-model collection to bind to. Use together with
    /// the page template to convert each item into a page. When set, takes
    /// precedence over the `Pages` property as the item source for the
    /// inner tab or carousel control.
    pub fn items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_source_property())
    }

    pub fn set_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::items_source_property(), value)
    }

    /// Gets or sets the data template used to create pages from view-model
    /// items.
    pub fn page_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::page_template_property())
    }

    pub fn set_page_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::page_template_property(), value)
    }

    /// Occurs when the `CurrentPage` property changes. Disposing the
    /// returned handle unsubscribes.
    pub fn current_page_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.current_page_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.current_page_changed.remove(token);
            }
        })
    }

    /// Occurs when the `Pages` collection changes. Disposing the returned
    /// handle unsubscribes.
    pub fn pages_changed(
        &self,
        handler: impl for<'a> Fn(&NotifyCollectionChangedEventArgs<'a, Ref<Page>>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.pages_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.pages_changed.remove(token);
            }
        })
    }

    /// The untyped items list of a collection of pages, for the items source
    /// of an items control: each page is an item boxed as a control. The
    /// same handle is returned for the same collection for as long as a
    /// handle is alive, so that assigning it twice is not a change.
    pub fn items_source_of(pages: &PageList) -> ItemsSource {
        thread_local! {
            static SOURCES: RefCell<Vec<Weak<PageItemsList>>> = const { RefCell::new(Vec::new()) };
        }

        SOURCES.with(|sources| {
            let mut sources = sources.borrow_mut();
            sources.retain(|source| source.strong_count() > 0);
            if let Some(existing) = sources.iter().filter_map(Weak::upgrade).find(|source| source.0.ptr_eq(pages)) {
                return ItemsSource::new(existing);
            }
            let list = Rc::new(PageItemsList(pages.clone()));
            sources.push(Rc::downgrade(&list));
            ItemsSource::new(list)
        })
    }

    /// The value of the `Pages` property as the items source of an items
    /// control. See [`items_source_of`](Self::items_source_of).
    pub fn pages_items_source(&self) -> Option<ItemsSource> {
        self.pages().as_ref().map(Self::items_source_of)
    }

    /// States whether the value of the `Pages` property is a collection
    /// that notifies of changes (the default). The reference implementation
    /// observes the collection only if it does; a subclass that exposes a
    /// collection it mutates itself (the navigation stack of a navigation
    /// page) opts out before assigning it.
    pub(crate) fn set_observes_pages(&self, value: bool) {
        self.observes_pages.set(value);
    }

    fn subscribe_to_pages(&self, collection: &PageList) {
        if !self.observes_pages.get() {
            return;
        }

        let weak = self.to_ref().downgrade();
        let token = collection.add_collection_changed(Rc::new(
            move |e: &NotifyCollectionChangedEventArgs<'_, Ref<Page>>| {
                if let Some(this) = weak.upgrade() {
                    this.notify_collection_collection_changed(e);
                }
            },
        ));
        let previous = self.pages_subscription.borrow_mut().replace((collection.clone(), token));
        if let Some((previous, token)) = previous {
            previous.remove_collection_changed(token);
        }
    }

    fn unsubscribe_from_pages(&self, collection: &PageList) {
        let subscription = {
            let mut subscription = self.pages_subscription.borrow_mut();
            if subscription.as_ref().is_some_and(|(subscribed, _)| subscribed.ptr_eq(collection)) {
                subscription.take()
            } else {
                None
            }
        };
        if let Some((subscribed, token)) = subscription {
            subscribed.remove_collection_changed(token);
        }
    }

    fn notify_collection_collection_changed(&self, e: &NotifyCollectionChangedEventArgs<'_, Ref<Page>>) {
        let logical_children = StyledElement::logical_children(self);
        match e.action {
            NotifyCollectionChangedAction::Reset => {
                logical_children.clear();
                if let Some(pages) = self.pages() {
                    for page in pages.snapshot().iter() {
                        logical_children.add(page.clone().upcast());
                    }
                }
            }
            _ => {
                for old in e.old_items {
                    logical_children.remove(&old.clone().upcast());
                }

                if !e.new_items.is_empty() {
                    let mut insert_idx =
                        if e.new_starting_index >= 0 { e.new_starting_index as usize } else { logical_children.count() };
                    for new_item in e.new_items {
                        logical_children.insert(insert_idx, new_item.clone().upcast());
                        insert_idx += 1;
                    }
                }
            }
        }

        for (_, handler) in self.pages_changed.snapshot().iter() {
            handler(e);
        }

        let mut nav_type = NavigationType::Replace;
        if let Some(current) = self.current_page() {
            let mut current_removed = false;
            if e.action == NotifyCollectionChangedAction::Remove || e.action == NotifyCollectionChangedAction::Replace
            {
                current_removed = e.old_items.iter().any(|old| old.ptr_eq(&current));
            } else if e.action == NotifyCollectionChangedAction::Reset {
                current_removed = true;
                if let Some(pages) = self.pages() {
                    if pages.snapshot().iter().any(|page| page.ptr_eq(&current)) {
                        current_removed = false;
                    }
                }
            }

            if current_removed {
                nav_type = NavigationType::Remove;
            }
        }

        self.update_active_page_with(nav_type);
    }
}

/// A collection of pages as an untyped items list: each page is an item
/// boxed as a control.
struct PageItemsList(PageList);

fn box_pages(pages: &[Ref<Page>]) -> Vec<Option<BoxedValue>> {
    pages.iter().map(|page| Some(Control::boxed(page.clone()))).collect()
}

impl IItemsList for PageItemsList {
    fn count(&self) -> usize {
        self.0.count()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        Some(Control::boxed(self.0.get(index)))
    }

    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        let Some(control) = item.as_ref().and_then(Control::from_boxed) else {
            return -1;
        };
        let control: &Control = &control;
        self.0
            .snapshot()
            .iter()
            .position(|page| {
                let page: &Control = page;
                std::ptr::eq(page, control)
            })
            .map_or(-1, |index| index as i32)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.0.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Ref<Page>>| {
            let new_items = box_pages(e.new_items);
            let old_items = box_pages(e.old_items);
            handler(&ItemsChangedEventArgs {
                action: e.action,
                new_items: ItemsView::from_slice(&new_items),
                old_items: ItemsView::from_slice(&old_items),
                new_starting_index: e.new_starting_index,
                old_starting_index: e.old_starting_index,
            });
        })))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.0.remove_collection_changed(token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
