use super::safe_area_padding_extensions::SafeAreaPaddingExtensions;
use super::{Page, PageImpl};
use crate::presenters::ContentPresenter;
use crate::primitives::{
    TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::templates::IDataTemplate;
use crate::{ContentControl, Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElement,
    StyledElementImpl, StyledProperty, StyledPropertyMetadata, TypeInfo, VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A page that displays a single piece of content with optional top and
/// bottom command bars.
#[repr(C)]
pub struct ContentPage {
    base: Page,
    content_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    top_command_bar_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    bottom_command_bar_presenter: RefCell<Option<Ref<ContentPresenter>>>,
}

ferro_class!(ContentPage: Page);

ferro_class_info!(ContentPage {
    new: ContentPage::new,
    markup: {
        // XAML-SEAM: the `Content` property depends on `ContentTemplate` (markup assignment order).
        content: Content,
        property_attributes: [Content: [DependsOn("ContentTemplate")]],
        attributes: [
            TemplatePart("PART_ContentPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_TopCommandBar", type(Ref<ContentPresenter>)),
            TemplatePart("PART_BottomCommandBar", type(Ref<ContentPresenter>)),
        ],
    },
});

ferro_impl_classes!(ContentPage: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

impl FerroObjectImpl for ContentPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::automatically_apply_safe_area_padding_property().as_property()
            || change.property() == Self::content_property().as_property()
        {
            this.update_content_safe_area_padding();
        } else if change.property() == Self::top_command_bar_property().as_property()
            || change.property() == Self::bottom_command_bar_property().as_property()
        {
            this.update_command_bars();
        }
    }
}

impl StyledElementImpl for ContentPage {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <ContentPage as StaticType>::TYPE
    }
}

impl TemplatedControlImpl for ContentPage {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        *this.content_presenter.borrow_mut() = Some(e.name_scope().get_as::<ContentPresenter>("PART_ContentPresenter"));
        *this.top_command_bar_presenter.borrow_mut() = e.name_scope().find_as::<ContentPresenter>("PART_TopCommandBar");
        *this.bottom_command_bar_presenter.borrow_mut() =
            e.name_scope().find_as::<ContentPresenter>("PART_BottomCommandBar");

        this.update_command_bars();
        this.update_content_safe_area_padding();
    }
}

impl PageImpl for ContentPage {
    fn update_content_safe_area_padding(this: &Self) {
        let content_presenter = this.content_presenter.borrow().clone();
        if let Some(content_presenter) = content_presenter {
            content_presenter.set_padding(if this.automatically_apply_safe_area_padding() {
                this.padding().apply_safe_area_padding(this.safe_area_padding())
            } else {
                this.padding()
            });
            content_presenter.invalidate_measure();
        }
    }
}

impl ControlImpl for ContentPage {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ContentPageAutomationPeer::new(this).upcast()
    }
}

ferro_properties! {
    impl ContentPage {
        /// Defines the `Content` property.
        pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
            ContentControl::content_property()
                .add_owner_with::<ContentPage>(StyledPropertyMetadata::new(None).with_coerce(ContentPage::coerce_content))
        }

        /// Defines the `ContentTemplate` property.
        pub fn content_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            ContentControl::content_template_property().add_owner::<ContentPage>()
        }

        /// Defines the `AutomaticallyApplySafeAreaPadding` property.
        pub fn automatically_apply_safe_area_padding_property() -> StyledProperty<bool> {
            FerroProperty::register::<ContentPage, _>("AutomaticallyApplySafeAreaPadding", true)
        }

        /// Defines the `TopCommandBar` property.
        pub fn top_command_bar_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContentPage, _>("TopCommandBar", None)
        }

        /// Defines the `BottomCommandBar` property.
        pub fn bottom_command_bar_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContentPage, _>("BottomCommandBar", None)
        }

        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            ContentControl::horizontal_content_alignment_property().add_owner::<ContentPage>()
        }

        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<ContentPage>()
        }
    }
}

impl ContentPage {
    fn static_constructor() {
        Self::content_property().changed().add_class_handler::<ContentPage>(|x, e| x.content_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Page::construct(),
            content_presenter: RefCell::new(None),
            top_command_bar_presenter: RefCell::new(None),
            bottom_command_bar_presenter: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A page cannot be the content of a content page.
    fn coerce_content(_instance: &FerroObject, value: Option<BoxedValue>) -> Option<BoxedValue> {
        let is_page =
            value.as_ref().and_then(Control::from_boxed).is_some_and(|control| control.is::<Page>());
        if is_page {
            panic!(
                "A Page cannot be used as the content of a ContentPage. Use a MultiPage subclass such as NavigationPage, TabbedPage, DrawerPage, or CarouselPage to host child pages."
            );
        }
        value
    }

    /// The horizontal alignment of the content within the page.
    pub fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_content_alignment_property())
    }

    pub fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_content_alignment_property(), value)
    }

    /// The vertical alignment of the content within the page.
    pub fn vertical_content_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_content_alignment_property())
    }

    pub fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_content_alignment_property(), value)
    }

    /// The page content.
    ///
    /// The content must not be another page. Use a page that hosts child
    /// pages (a navigation page, tabbed page, drawer page or carousel page)
    /// for that.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }

    /// Sets the page content.
    ///
    /// # Panics
    /// Panics when the assigned value is a page.
    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value)
    }

    /// The data template used to display the content.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }

    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value)
    }

    /// Whether safe-area padding is automatically applied to the content
    /// presenter.
    pub fn automatically_apply_safe_area_padding(&self) -> bool {
        self.get_value(Self::automatically_apply_safe_area_padding_property())
    }

    pub fn set_automatically_apply_safe_area_padding(&self, value: bool) {
        self.set_value(Self::automatically_apply_safe_area_padding_property(), value)
    }

    /// The content displayed in the top command bar area.
    pub fn top_command_bar(&self) -> Option<BoxedValue> {
        self.get_value(Self::top_command_bar_property())
    }

    pub fn set_top_command_bar(&self, value: Option<BoxedValue>) {
        self.set_value(Self::top_command_bar_property(), value)
    }

    /// The content displayed in the bottom command bar area.
    pub fn bottom_command_bar(&self) -> Option<BoxedValue> {
        self.get_value(Self::bottom_command_bar_property())
    }

    pub fn set_bottom_command_bar(&self, value: Option<BoxedValue>) {
        self.set_value(Self::bottom_command_bar_property(), value)
    }

    fn content_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = e.get_old_and_new_value::<Option<BoxedValue>>();

        if let Some(old_child) = old_value.as_ref().and_then(Control::logical_from_boxed) {
            StyledElement::logical_children(self).remove(&old_child);
        }

        if let Some(new_child) = new_value.as_ref().and_then(Control::logical_from_boxed) {
            StyledElement::logical_children(self).add(new_child);
        }
    }

    fn update_command_bars(&self) {
        let top_command_bar_presenter = self.top_command_bar_presenter.borrow().clone();
        if let Some(presenter) = top_command_bar_presenter {
            let top_command_bar = self.top_command_bar();
            presenter.set_content(top_command_bar.clone());
            presenter.set_is_visible(top_command_bar.is_some());
        }

        let bottom_command_bar_presenter = self.bottom_command_bar_presenter.borrow().clone();
        if let Some(presenter) = bottom_command_bar_presenter {
            let bottom_command_bar = self.bottom_command_bar();
            presenter.set_content(bottom_command_bar.clone());
            presenter.set_is_visible(bottom_command_bar.is_some());
        }
    }
}
