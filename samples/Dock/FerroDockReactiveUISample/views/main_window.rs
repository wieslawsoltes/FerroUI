// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{
    platform::PlatformManager, primitives::TemplatedControlImpl, ContentControlImpl, ControlImpl,
    TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};
/// Original passive MainWindow, populated solely from compiled source markup.
#[repr(C)]
pub struct MainWindow {
    base: Window,
}
ferro_class!(MainWindow:Window);
ferro_class_info!(MainWindow {
    new: MainWindow::new
});
ferro_impl_classes!(MainWindow:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,TopLevelImpl,WindowBaseImpl,WindowImpl);
impl MainWindow {
    /// Creates the source view and initializes its compiled markup.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self {
            base: Window::construct(PlatformManager::create_window()),
        });
        crate::register_types();
        crate::compiled_main_window::populate(None, &this)
            .unwrap_or_else(|error| panic!("{error}"));
        this
    }
}
