//! Port of `ManagedDragDrop/DragPreviewWindow.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, ControlImpl, SizeToContent, TopLevelImpl, Window, WindowBaseImpl, WindowDecorations, WindowImpl,
    WindowStartupLocation, WindowTransparencyLevel, WindowTransparencyLevelCollection,
};
use std::rc::Rc;

/// Lightweight always-on-top window used to display drag preview content: transparent,
/// non-interactive, topmost and sized to its content.
#[repr(C)]
pub struct DragPreviewWindow {
    base: Window,
}

ferro_class!(DragPreviewWindow: Window);
ferro_class_info!(DragPreviewWindow { new: DragPreviewWindow::new });
ferro_impl_classes!(
    DragPreviewWindow: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for DragPreviewWindow {
        /// The constructor body: configures a transparent, non-interactive, topmost window sized
        /// to its content.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_can_resize(false);
            this.set_show_in_taskbar(false);
            this.set_window_decorations(WindowDecorations::None);
            this.set_topmost(true);
            this.set_background(Some(Brushes::transparent() as Rc<dyn IBrush>));
            this.set_transparency_level_hint(WindowTransparencyLevelCollection::new(vec![
                WindowTransparencyLevel::transparent(),
            ]));
            // Disable focus/activation stealing where possible
            this.set_window_startup_location(WindowStartupLocation::Manual);
            this.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
            this.set_is_enabled(false); // don't accept input
        }
    }
}

impl DragPreviewWindow {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    ///
    /// # Panics
    /// Panics when no windowing platform is registered.
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    /// Creates the window.
    ///
    /// # Panics
    /// Panics when no windowing platform is registered.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
