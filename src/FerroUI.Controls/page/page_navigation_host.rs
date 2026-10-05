use super::{DefaultPageDataTemplate, NavigatedFromEventArgs, NavigatedToEventArgs, NavigationType, Page};
use crate::platform::{IInsetsManager, SafeAreaChangedArgs};
use crate::presenters::ContentPresenter;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::IDataTemplate;
use crate::{ContentControl, ContentControlImpl, Control, ControlImpl, TopLevel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref, StaticType, StyledElementImpl,
    StyledProperty, Thickness, TypeInfo, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The subscriptions made on the top-level the host is attached to.
struct TopLevelSubscriptions {
    top_level: Ref<TopLevel>,
    back_requested: RoutedEventHandlerToken,
    scaling_changed: Rc<dyn IDisposable>,
}

/// The subscription made on the insets manager of the top-level.
struct InsetsSubscription {
    insets_manager: Rc<dyn IInsetsManager>,
    safe_area_changed: Rc<dyn IDisposable>,
}

/// Hosts a page as the root of a top-level: forwards the safe-area padding
/// and the system back navigation of the top-level to the page.
#[repr(C)]
pub struct PageNavigationHost {
    base: ContentControl,
    top_level: RefCell<Option<TopLevelSubscriptions>>,
    inset_manager: RefCell<Option<InsetsSubscription>>,
    content_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    /// The handler added to the property-changed event of the content
    /// presenter. At most one is ever subscribed: every path that adds one
    /// removes the previous one first.
    content_presenter_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(PageNavigationHost: ContentControl);
ferro_class_info!(PageNavigationHost { new: PageNavigationHost::new });
ferro_impl_classes!(
    PageNavigationHost: LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for PageNavigationHost {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::page_property().as_property() {
            let (old_page, new_page) = change.get_old_and_new_value::<Option<Ref<Page>>>();

            this.set_current_value(
                ContentControl::content_property(),
                new_page.as_ref().map(|page| Control::boxed(page.clone())),
            );

            if let (Some(inset_manager), Some(new_page)) = (this.inset_manager(), &new_page) {
                new_page.set_safe_area_padding(inset_manager.safe_area_padding());
            }

            if let Some(old_page) = &old_page {
                old_page.send_navigated_from(&NavigatedFromEventArgs::new(new_page.clone(), NavigationType::Replace));
            }
            if let Some(new_page) = &new_page {
                new_page.send_navigated_to(&NavigatedToEventArgs::new(old_page.clone(), NavigationType::Replace));
            }
        }
    }
}

impl StyledElementImpl for PageNavigationHost {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <ContentControl as StaticType>::TYPE
    }
}

impl VisualImpl for PageNavigationHost {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        this.clean_up_subscriptions();

        let top_level = TopLevel::get_top_level(Some(this));
        let inset_manager = top_level.as_ref().and_then(|top_level| top_level.insets_manager());

        if let Some(inset_manager) = inset_manager {
            let weak = this.to_ref().downgrade();
            let safe_area_changed = inset_manager.safe_area_changed(Rc::new(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.inset_manager_safe_area_changed(e);
                }
            }));
            inset_manager.set_display_edge_to_edge_preference(true);
            *this.inset_manager.borrow_mut() = Some(InsetsSubscription { insets_manager: inset_manager, safe_area_changed });
        }

        if let Some(top_level) = top_level {
            let weak = this.to_ref().downgrade();
            let back_requested = top_level.back_requested(move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.top_level_back_requested(e);
                }
            });
            let weak = this.to_ref().downgrade();
            let scaling_changed = top_level.scaling_changed(move || {
                if let Some(this) = weak.upgrade() {
                    this.top_level_scaling_changed();
                }
            });
            *this.top_level.borrow_mut() = Some(TopLevelSubscriptions { top_level, back_requested, scaling_changed });
        }

        this.attach_content_presenter();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        this.clean_up_subscriptions();

        if this.content_presenter.borrow().is_some() {
            this.remove_content_presenter_property_changed();
        }
    }
}

impl TemplatedControlImpl for PageNavigationHost {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        if this.content_presenter.borrow().is_some() {
            this.remove_content_presenter_property_changed();
        }

        *this.content_presenter.borrow_mut() = Some(e.name_scope().get_as::<ContentPresenter>("PART_ContentPresenter"));

        this.attach_content_presenter();
    }
}

ferro_properties! {
    impl PageNavigationHost {
        /// Defines the `Page` property.
        pub fn page_property() -> StyledProperty<Option<Ref<Page>>> {
            FerroProperty::register::<PageNavigationHost, _>("Page", None)
        }
    }
}

impl PageNavigationHost {
    fn static_constructor() {
        let template: Rc<dyn IDataTemplate> = DefaultPageDataTemplate::new();
        ContentControl::content_template_property().override_default_value::<PageNavigationHost>(Some(template));
        TopLevel::auto_safe_area_padding_property().override_default_value::<PageNavigationHost>(false);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            top_level: RefCell::new(None),
            inset_manager: RefCell::new(None),
            content_presenter: RefCell::new(None),
            content_presenter_subscription: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The page hosted by the control.
    pub fn page(&self) -> Option<Ref<Page>> {
        self.get_value(Self::page_property())
    }

    pub fn set_page(&self, value: impl Into<Nullable<Page>>) {
        self.set_value(Self::page_property(), value.into().0)
    }

    fn inset_manager(&self) -> Option<Rc<dyn IInsetsManager>> {
        self.inset_manager.borrow().as_ref().map(|subscription| subscription.insets_manager.clone())
    }

    /// The page that the presenter of the control displays, if its child is
    /// a page.
    fn presented_page(presenter: Option<Ref<ContentPresenter>>) -> Option<Ref<Page>> {
        presenter.and_then(|presenter| presenter.child()).and_then(|child| child.cast::<Page>())
    }

    fn top_level_scaling_changed(&self) {
        let content_presenter = self.content_presenter.borrow().clone();
        if let (Some(inset_manager), Some(page)) = (self.inset_manager(), Self::presented_page(content_presenter)) {
            page.set_safe_area_padding(inset_manager.safe_area_padding());
        }
    }

    fn clean_up_subscriptions(&self) {
        let inset_manager = self.inset_manager.borrow_mut().take();
        if let Some(inset_manager) = inset_manager {
            inset_manager.safe_area_changed.dispose();
        }

        let top_level = self.top_level.borrow_mut().take();
        if let Some(top_level) = top_level {
            top_level.top_level.remove_handler(TopLevel::back_requested_event(), top_level.back_requested);
            top_level.scaling_changed.dispose();
        }
    }

    /// Removes the handler from the property-changed event of the content
    /// presenter.
    fn remove_content_presenter_property_changed(&self) {
        let subscription = self.content_presenter_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn attach_content_presenter(&self) {
        let Some(content_presenter) = self.content_presenter.borrow().clone() else {
            return;
        };

        if let (Some(inset_manager), Some(page)) =
            (self.inset_manager(), Self::presented_page(Some(content_presenter.clone())))
        {
            page.set_safe_area_padding(inset_manager.safe_area_padding());
        }

        if self.is_attached_to_visual_tree() {
            let weak = self.to_ref().downgrade();
            let subscription = content_presenter.property_changed(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.content_presenter_property_changed(e);
                }
            });
            let previous = self.content_presenter_subscription.borrow_mut().replace(subscription);
            if let Some(previous) = previous {
                previous.dispose();
            }
        }
    }

    fn content_presenter_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() != ContentPresenter::child_property().as_property() {
            return;
        }

        let (old_value, new_value) = e.get_old_and_new_value::<Option<Ref<Control>>>();

        if let Some(old_page) = old_value.and_then(|old| old.cast::<Page>()) {
            old_page.set_safe_area_padding(Thickness::default());
        }

        if let (Some(new_page), Some(inset_manager)) =
            (new_value.and_then(|new| new.cast::<Page>()), self.inset_manager())
        {
            new_page.set_safe_area_padding(inset_manager.safe_area_padding());
        }
    }

    fn top_level_back_requested(&self, e: &RoutedEventArgs) {
        if e.handled() {
            return;
        }

        if let Some(page) = Self::presented_page(self.presenter()) {
            let forwarded = RoutedEventArgs::with_event(Page::page_navigation_system_back_button_pressed_event());
            page.raise_event(&forwarded);
            e.set_handled(forwarded.handled());
        }
    }

    fn inset_manager_safe_area_changed(&self, e: &SafeAreaChangedArgs) {
        if self.content().is_some() {
            if let Some(page) = Self::presented_page(self.presenter()) {
                page.set_safe_area_padding(e.safe_area_padding());
            }
        }
    }
}
