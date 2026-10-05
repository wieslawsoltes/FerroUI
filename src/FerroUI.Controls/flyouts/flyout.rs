use super::{
    FlyoutBaseImpl, FlyoutPresenter, PopupFlyoutBase, PopupFlyoutBaseImpl, PopupFlyoutBaseImplExt,
};
use crate::templates::IDataTemplate;
use crate::{ContentControl, Control};
use ferroui_base::controls::Classes;
use ferroui_base::styling::ControlTheme;
use ferroui_base::utilities::CancelEventArgs;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl, FerroProperty,
    Nullable, Ref, StyledElement, StyledProperty,
};
use std::cell::OnceCell;
use std::rc::Rc;

/// A flyout that displays arbitrary content.
#[repr(C)]
pub struct Flyout {
    base: PopupFlyoutBase,
    classes: OnceCell<Classes>,
}

ferro_class!(Flyout: PopupFlyoutBase);
ferroui_base::ferro_class_info!(Flyout { new: Flyout::new });
ferro_impl_classes!(Flyout: FerroObjectImpl, FlyoutBaseImpl);

impl PopupFlyoutBaseImpl for Flyout {
    fn create_presenter(this: &Self) -> Ref<Control> {
        let presenter = FlyoutPresenter::new();
        presenter.bind_indexer(
            &ContentControl::content_property().bind(),
            &this.indexer(&Self::content_property().bind()),
        );
        presenter.bind_indexer(
            &ContentControl::content_template_property().bind(),
            &this.indexer(&Self::content_template_property().bind()),
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

ferroui_base::ferro_properties! { impl Flyout {
    ferro_property!(
        /// Defines the `Content` property.
        pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Flyout, _>("Content", None)
        }
    );

    ferro_property!(
        /// Defines the `ContentTemplate` property.
        pub fn content_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<Flyout, _>("ContentTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `FlyoutPresenterTheme` property.
        pub fn flyout_presenter_theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<Flyout, _>("FlyoutPresenterTheme", None)
        }
    );
} }

impl Flyout {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: PopupFlyoutBase::construct(), classes: OnceCell::new() }
    }

    /// Creates a flyout.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The classes collection to apply to the flyout presenter this flyout
    /// is hosting.
    pub fn flyout_presenter_classes(&self) -> Classes {
        self.classes.get_or_init(Classes::new).clone()
    }

    /// The control theme that is applied to the container element generated
    /// for the flyout presenter.
    pub fn flyout_presenter_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::flyout_presenter_theme_property())
    }

    pub fn set_flyout_presenter_theme(&self, value: impl Into<Nullable<ControlTheme>>) {
        self.set_value(Self::flyout_presenter_theme_property(), value.into().0)
    }

    /// The content to display in this flyout.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value)
    }

    /// The data template used to display the content of the flyout.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }

    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value)
    }
}
