//! Native frame page content and synchronous navigation lifecycle. See NOTICE.md for source attribution.

use crate::navigation::{NavigatingCancelEventArgs, NavigationCacheMode, NavigationEventArgs};
use crate::Frame;
use crate::primitives::TemplatedControlImpl;
use crate::{ContentControlImpl, ControlImpl, UserControl};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
    WeakRef,
};
use std::cell::{Cell, RefCell};

/// A navigable user control hosted by a [`Frame`].
///
/// The synchronous frame lifecycle is separate from the existing [`crate::Page`] navigation hierarchy.
#[repr(C)]
pub struct FramePage {
    base: UserControl,
    frame: RefCell<Option<WeakRef<Frame>>>,
    cache_key: Cell<Option<&'static TypeInfo>>,
}

ferro_class! {
    FramePage: UserControl, virtuals FramePageImpl: ContentControlImpl {
        fn on_navigating_from(this, args: &NavigatingCancelEventArgs);
        fn on_navigated_from(this, args: &NavigationEventArgs);
        fn on_navigated_to(this, args: &NavigationEventArgs);
    }
}
ferro_class_info!(FramePage {
    new: FramePage::new,
    markup: { namespace: "FerroUI.Controls", content: Content },
});
ferro_impl_classes!(FramePage: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);

impl StyledElementImpl for FramePage {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        UserControl::TYPE
    }
}

impl FerroObjectImpl for FramePage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::navigation_cache_mode_property().as_property()
            && this.navigation_cache_mode() == NavigationCacheMode::Disabled
        {
            if let (Some(frame), Some(key)) = (this.frame(), this.cache_key.get()) {
                frame.remove_cached_page(key);
            }
        }
    }
}

impl FramePageImpl for FramePage {
    fn on_navigating_from(_this: &Self, _args: &NavigatingCancelEventArgs) {}
    fn on_navigated_from(_this: &Self, _args: &NavigationEventArgs) {}
    fn on_navigated_to(_this: &Self, _args: &NavigationEventArgs) {}
}

ferro_properties! {
    impl FramePage {
        pub fn navigation_cache_mode_property() -> StyledProperty<NavigationCacheMode> {
            FerroProperty::register::<FramePage, _>("NavigationCacheMode", NavigationCacheMode::Disabled)
        }
        pub fn frame_property() -> DirectProperty<FramePage, Option<Ref<Frame>>> {
            FerroProperty::register_direct::<FramePage, _>("Frame", FramePage::frame, None, None)
        }
    }
}

impl FramePage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            frame: RefCell::new(None),
            cache_key: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn navigation_cache_mode(&self) -> NavigationCacheMode {
        self.get_value(Self::navigation_cache_mode_property())
    }
    pub fn set_navigation_cache_mode(&self, mode: NavigationCacheMode) {
        self.set_value(Self::navigation_cache_mode_property(), mode);
    }
    /// The containing frame, held weakly to avoid a cycle with frame content or cache entries.
    pub fn frame(&self) -> Option<Ref<Frame>> {
        self.frame.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub(crate) fn prepare(&self, frame: &Ref<Frame>, source_page_type: &'static TypeInfo) {
        self.cache_key.set(Some(source_page_type));
        let old = self.frame();
        if old.as_ref() != Some(frame) {
            *self.frame.borrow_mut() = Some(frame.downgrade());
            let new = Some(frame.clone());
            self.raise_property_changed(
                Self::frame_property().as_property(),
                Some(&old),
                &new,
                BindingPriority::LocalValue,
                true,
            );
        }
    }
}
