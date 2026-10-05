use crate::metadata::PseudoClassesAttribute;
use crate::primitives::TemplatedControlImpl;
use crate::{Button, ButtonImpl, ButtonImplExt, ContentControlImpl, ControlImpl, TopLevel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};

// See: https://www.w3schools.com/cssref/sel_visited.php
const PC_VISITED: &str = ":visited";

/// A button control that functions as a navigateable hyperlink.
#[repr(C)]
pub struct HyperlinkButton {
    base: Button,
}

ferro_class!(HyperlinkButton: Button);
ferroui_base::ferro_class_info!(HyperlinkButton { new: HyperlinkButton::new });
ferro_impl_classes!(
    HyperlinkButton: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl ButtonImpl for HyperlinkButton {
    fn on_click(this: &Self) {
        Self::parent_on_click(this);

        let uri = this.navigate_uri();
        if let Some(uri) = uri {
            let this = this.to_ref();
            // The task is not observed, as the posted asynchronous action
            // of the reference.
            let _ = Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let Some(top_level) = TopLevel::get_top_level(Some(&this)) else {
                    return;
                };
                let success = top_level.launcher().launch_uri_async(&uri).await;
                if success {
                    this.set_current_value(Self::is_visited_property(), true);
                }
            });
        }
    }
}

impl FerroObjectImpl for HyperlinkButton {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::is_visited_property().as_property() {
            this.pseudo_classes().set(PC_VISITED, change.get_new_value::<bool>());
        }
    }
}

impl HyperlinkButton {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[PC_VISITED]);
}

ferroui_base::ferro_properties! { impl HyperlinkButton {
    ferro_property!(
        /// Defines the `IsVisited` property.
        pub fn is_visited_property() -> StyledProperty<bool> {
            FerroProperty::register::<HyperlinkButton, _>("IsVisited", false)
        }
    );

    ferro_property!(
        /// Defines the `NavigateUri` property.
        pub fn navigate_uri_property() -> StyledProperty<Option<Uri>> {
            FerroProperty::register::<HyperlinkButton, _>("NavigateUri", None)
        }
    );
} }

impl HyperlinkButton {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Button::construct() }
    }

    /// Creates a hyperlink button.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A value indicating whether the `NavigateUri` has been visited.
    pub fn is_visited(&self) -> bool {
        self.get_value(Self::is_visited_property())
    }

    pub fn set_is_visited(&self, value: bool) {
        self.set_value(Self::is_visited_property(), value)
    }

    /// The Uniform Resource Identifier (URI) automatically navigated to
    /// when the hyperlink button is clicked.
    ///
    /// The URI may be any website or file location that can be launched
    /// using the launcher service.
    ///
    /// If a URI should not be automatically launched, leave this property
    /// unset and use the `Click` event and `is_visited` directly.
    pub fn navigate_uri(&self) -> Option<Uri> {
        self.get_value(Self::navigate_uri_property())
    }

    pub fn set_navigate_uri(&self, value: Option<Uri>) {
        self.set_value(Self::navigate_uri_property(), value)
    }
}
