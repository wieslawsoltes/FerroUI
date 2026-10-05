use super::{
    HeaderedContentControl, HeaderedItemsControl, SelectingItemsControl, SelectingItemsControlImpl, TemplatedControlImpl,
};
use crate::metadata::TemplatePartAttribute;
use crate::presenters::{register_content_presenter_host, ContentPresenter, IContentPresenterHost};
use crate::templates::IDataTemplate;
use crate::{Control, ControlImpl, ItemsControl, ItemsControlImpl};
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl,
    FerroPropertyChangedEventArgs, Ref, StaticType, StyledElement, StyledElementImpl,
    StyledElementImplExt, StyledProperty, VisualImpl, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Represents a [`SelectingItemsControl`] with a related header.
#[repr(C)]
pub struct HeaderedSelectingItemsControl {
    base: SelectingItemsControl,
    items_binding: RefCell<Option<Rc<dyn IDisposable>>>,
    prepare_item_container_on_attach: RefCell<Option<WeakRef<ItemsControl>>>,
    header_presenter: RefCell<Option<Ref<ContentPresenter>>>,
}

ferro_class! {
    HeaderedSelectingItemsControl: SelectingItemsControl, virtuals HeaderedSelectingItemsControlImpl: SelectingItemsControlImpl {
        /// Called when a content presenter whose templated parent is this
        /// control is created. Returns true if the presenter was registered
        /// as a presenter of the control.
        fn register_content_presenter(this, presenter: &ContentPresenter) -> bool;
    }
}
ferroui_base::ferro_class_info!(HeaderedSelectingItemsControl { new: HeaderedSelectingItemsControl::new });

ferro_impl_classes!(
    HeaderedSelectingItemsControl: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ItemsControlImpl,
    SelectingItemsControlImpl
);

impl FerroObjectImpl for HeaderedSelectingItemsControl {}

impl StyledElementImpl for HeaderedSelectingItemsControl {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_logical_tree(this, e);

        let pending = this.prepare_item_container_on_attach.take();
        if let Some(parent) = pending.and_then(|parent| parent.upgrade()) {
            this.prepare_item_container(&parent);
        }
    }
}

impl HeaderedSelectingItemsControlImpl for HeaderedSelectingItemsControl {
    fn register_content_presenter(this: &Self, presenter: &ContentPresenter) -> bool {
        if presenter.name().as_deref() == Some("PART_HeaderPresenter") {
            *this.header_presenter.borrow_mut() = Some(presenter.to_ref());
            return true;
        }

        false
    }
}

impl IContentPresenterHost for HeaderedSelectingItemsControl {
    fn logical_children(&self) -> &FerroList<Ref<StyledElement>> {
        StyledElement::logical_children(self)
    }

    fn register_content_presenter(&self, presenter: &ContentPresenter) -> bool {
        HeaderedSelectingItemsControl::register_content_presenter(self, presenter)
    }
}

impl HeaderedSelectingItemsControl {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] =
        &[TemplatePartAttribute::new("PART_HeaderPresenter", <ContentPresenter as StaticType>::TYPE)];
}

ferroui_base::ferro_properties! { impl HeaderedSelectingItemsControl {
    ferro_property!(
        /// Defines the `Header` property.
        pub fn header_property() -> StyledProperty<Option<BoxedValue>> {
            HeaderedContentControl::header_property().add_owner::<HeaderedSelectingItemsControl>()
        }
    );

    ferro_property!(
        /// Defines the `HeaderTemplate` property.
        pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            HeaderedItemsControl::header_template_property().add_owner::<HeaderedSelectingItemsControl>()
        }
    );
} }

impl HeaderedSelectingItemsControl {
    fn static_constructor() {
        register_content_presenter_host::<HeaderedSelectingItemsControl>();
        Self::header_property().changed().add_class_handler::<HeaderedSelectingItemsControl>(|x, e| x.header_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: SelectingItemsControl::construct(),
            items_binding: RefCell::new(None),
            prepare_item_container_on_attach: RefCell::new(None),
            header_presenter: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the content of the control's header.
    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }

    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value)
    }

    /// Gets or sets the data template used to display the header content of
    /// the control.
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }

    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value)
    }

    /// Gets the header presenter from the control's template.
    pub fn header_presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.header_presenter.borrow().clone()
    }

    pub(crate) fn prepare_item_container(&self, parent: &ItemsControl) {
        if let Some(items_binding) = self.items_binding.take() {
            items_binding.dispose();
        }

        let Some(item) = self.header() else {
            *self.prepare_item_container_on_attach.borrow_mut() = None;
            return;
        };

        let mut header_template = self.header_template().or_else(|| parent.item_template());

        if header_template.is_none() {
            if self.is_attached_to_logical_tree() {
                header_template = self.find_data_template(Some(&item), None);
            } else {
                *self.prepare_item_container_on_attach.borrow_mut() = Some(parent.to_ref().downgrade());
            }
        }

        if let Some(tree_template) = header_template.as_ref().and_then(|t| t.as_tree_data_template()) {
            if tree_template.match_(Some(&item)) {
                let items_binding =
                    tree_template.bind_children(self, ItemsControl::items_source_property().as_property(), &item);
                *self.items_binding.borrow_mut() = Some(items_binding);
            }
        }
    }

    fn header_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = e.get_old_and_new_value::<Option<BoxedValue>>();

        if let Some(old_child) = old_value.as_ref().and_then(Control::logical_from_boxed) {
            StyledElement::logical_children(self).remove(&old_child);
        }

        if let Some(new_child) = new_value.as_ref().and_then(Control::logical_from_boxed) {
            StyledElement::logical_children(self).add(new_child);
        }
    }
}
