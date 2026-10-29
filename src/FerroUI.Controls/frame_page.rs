//! WinUI's `Page` as `FramePage` (`dxaml/xcp/dxaml/lib/Page_Partial.cpp` of microsoft-ui-xaml,
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::navigation::{NavigatingCancelEventArgs, NavigationCacheMode, NavigationEventArgs};
use crate::primitives::TemplatedControlImpl;
use crate::Frame;
use crate::{ContentControlImpl, ControlImpl, UserControl};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, TypeInfo,
    VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};

/// A page a [`Frame`] navigates to (WinUI's `Page`, a `UserControl`).
///
/// The name keeps `Page` for the upstream page of `ferroui-controls` ([`crate::Page`], a templated
/// control with asynchronous navigation); the two navigation models are separate
/// (docs/porting/winui-controls.md, section 7). A derived page overrides the callbacks of
/// [`FramePageImpl`] (`on_navigated_to`, `on_navigating_from`, `on_navigated_from`), which the frame
/// calls synchronously. The page is styled as a `UserControl` (WinUI's `Page` has no style of its
/// own).
///
/// Missing (docs/porting/winui-controls.md, section 10.4): `TopAppBar`, `BottomAppBar` and the
/// navigation transitions of the page.
#[repr(C)]
pub struct FramePage {
    base: UserControl,
    frame: RefCell<Option<WeakRef<Frame>>>,
    cache_key: Cell<Option<&'static TypeInfo>>,
}

ferro_class! {
    FramePage: UserControl, virtuals FramePageImpl: ContentControlImpl {
        /// Called before the frame navigates away from the page (WinUI's `OnNavigatingFrom`);
        /// `args.set_cancel(true)` cancels the navigation.
        fn on_navigating_from(this, args: &NavigatingCancelEventArgs);
        /// Called after the frame navigated away from the page (WinUI's `OnNavigatedFrom`).
        fn on_navigated_from(this, args: &NavigationEventArgs);
        /// Called after the frame navigated to the page (WinUI's `OnNavigatedTo`).
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
        // Page::OnPropertyChanged2: a page whose cache mode becomes Disabled leaves the cache of its
        // frame.
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
        /// Defines the `NavigationCacheMode` property: whether the frame keeps the page to show it
        /// again. `Disabled` by default.
        pub fn navigation_cache_mode_property() -> StyledProperty<NavigationCacheMode> {
            FerroProperty::register::<FramePage, _>("NavigationCacheMode", NavigationCacheMode::Disabled)
        }
        /// Defines the `Frame` property: the frame that shows the page (read-only).
        pub fn frame_property() -> DirectProperty<FramePage, Option<Ref<Frame>>> {
            FerroProperty::register_direct::<FramePage, _>("Frame", FramePage::frame, None, None)
        }
    }
}

impl FramePage {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), frame: RefCell::new(None), cache_key: Cell::new(None) }
    }

    /// Creates a page.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets whether the frame keeps the page to show it again.
    pub fn navigation_cache_mode(&self) -> NavigationCacheMode {
        self.get_value(Self::navigation_cache_mode_property())
    }

    /// Sets whether the frame keeps the page to show it again.
    pub fn set_navigation_cache_mode(&self, mode: NavigationCacheMode) {
        self.set_value(Self::navigation_cache_mode_property(), mode);
    }

    /// Gets the frame that shows the page, held weakly, so that a page in the content or the cache
    /// of a frame keeps no cycle.
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
