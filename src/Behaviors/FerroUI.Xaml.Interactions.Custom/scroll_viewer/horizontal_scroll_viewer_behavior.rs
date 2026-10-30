//! Port of `ScrollViewer/HorizontalScrollViewerBehavior.cs`.

use ferroui_base::input::{InputElement, KeyModifiers, PointerWheelEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_enum, ferro_properties, instantiate, FerroObjectImpl, FerroProperty,
    Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::ScrollViewer;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};
use std::cell::Cell;

/// The scroll amount of a [`HorizontalScrollViewerBehavior`] for each wheel delta.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference nests the enumeration in
/// `HorizontalScrollViewerBehavior`; Rust has no nested types, so it is an enumeration of the
/// module of the behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChangeSize {
    /// Scrolls by a single line.
    #[default]
    Line,

    /// Scrolls by a full page.
    Page,
}

impl std::fmt::Display for ChangeSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Line => "Line",
            Self::Page => "Page",
        })
    }
}

ferro_markup_enum!(ChangeSize { Line, Page });

/// Enables horizontal scrolling of a [`ScrollViewer`] using the mouse wheel.
#[repr(C)]
pub struct HorizontalScrollViewerBehavior {
    base: StyledElementBehaviorOfT,
    // The handler is removed by its token (a closure has no identity to remove it by, as the
    // reference removes its method).
    pointer_wheel_changed: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(HorizontalScrollViewerBehavior: StyledElementBehaviorOfT);
ferro_class_info!(HorizontalScrollViewerBehavior { new: HorizontalScrollViewerBehavior::new });
ferroui_base::ferro_impl_classes!(HorizontalScrollViewerBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl HorizontalScrollViewerBehavior {
        /// Gets or sets a value indicating whether the Shift key must be held while scrolling.
        pub fn require_shift_key_property() -> StyledProperty<bool> {
            FerroProperty::register::<HorizontalScrollViewerBehavior, _>("RequireShiftKey", false)
        }

        /// Gets or sets the scroll amount used for each wheel delta.
        pub fn scroll_change_size_property() -> StyledProperty<ChangeSize> {
            FerroProperty::register::<HorizontalScrollViewerBehavior, _>("ScrollChangeSize", ChangeSize::Line)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for HorizontalScrollViewerBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            let associated_object =
                this.associated_object_as::<ScrollViewer>().expect("the behavior is attached to a scroll viewer");
            let behavior = this.to_ref().downgrade();
            let token = associated_object.add_handler_with(
                InputElement::pointer_wheel_changed_event(),
                move |_sender: &Interactive, e: &PointerWheelEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_pointer_wheel_changed(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            this.pointer_wheel_changed.set(Some(token));
        }

        fn on_detaching(this: &Self) {
            Self::parent_on_detaching(this);

            let associated_object =
                this.associated_object_as::<ScrollViewer>().expect("the behavior is attached to a scroll viewer");
            if let Some(token) = this.pointer_wheel_changed.take() {
                associated_object.remove_handler(InputElement::pointer_wheel_changed_event(), token);
            }
        }
    }
}

impl HorizontalScrollViewerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<ScrollViewer as StaticType>::TYPE),
            pointer_wheel_changed: Cell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the Shift key must be held while scrolling.
    pub fn require_shift_key(&self) -> bool {
        self.get_value(Self::require_shift_key_property())
    }

    pub fn set_require_shift_key(&self, value: bool) {
        self.set_value(Self::require_shift_key_property(), value)
    }

    /// The scroll amount used for each wheel delta.
    pub fn scroll_change_size(&self) -> ChangeSize {
        self.get_value(Self::scroll_change_size_property())
    }

    pub fn set_scroll_change_size(&self, value: ChangeSize) {
        self.set_value(Self::scroll_change_size_property(), value)
    }

    /// Handles the pointer wheel changed event.
    fn on_pointer_wheel_changed(&self, e: &PointerWheelEventArgs) {
        if !self.is_enabled() {
            e.set_handled(true);
            return;
        }

        let require_shift_key = self.require_shift_key();
        if !(require_shift_key && e.key_modifiers() == KeyModifiers::SHIFT || !require_shift_key) {
            return;
        }

        let Some(associated_object) = self.associated_object_as::<ScrollViewer>() else {
            return;
        };

        if e.delta().y < 0.0 {
            if self.scroll_change_size() == ChangeSize::Line {
                associated_object.line_right();
            } else {
                associated_object.page_right();
            }
        } else if self.scroll_change_size() == ChangeSize::Line {
            associated_object.line_left();
        } else {
            associated_object.page_left();
        }
    }
}
