//! Passive view populated entirely from compiled XAML.
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, controls::NameScope,
    input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{
    ContentControlImpl, Control, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
    platform::PlatformManager, primitives::TemplatedControlImpl,
};
#[repr(C)]
pub struct MainWindow {
    base: Window,
}
ferroui_base::ferro_class!(MainWindow:Window);
ferroui_base::ferro_class_info!(MainWindow {
    new: MainWindow::new
});
ferroui_base::ferro_impl_classes!(MainWindow:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,TopLevelImpl,WindowBaseImpl,WindowImpl);
impl MainWindow {
    pub fn new() -> Ref<Self> {
        let this = ferroui_base::instantiate(Self {
            base: Window::construct(PlatformManager::create_window()),
        });
        crate::compiled_main_window::populate(None, &this).unwrap_or_else(|e| panic!("{e}"));
        // The native scope supports multiple weak owners. Keep the original desktop
        // named-field lookup on the Window while the shared view owns those fields.
        NameScope::set_name_scope(&this, NameScope::get_name_scope(&this.main_view()));
        this
    }
    pub fn main_view(&self) -> Ref<crate::MainView> {
        self.content()
            .as_ref()
            .and_then(Control::from_boxed)
            .and_then(|view| view.cast::<crate::MainView>())
            .expect("compiled MainWindow content is MainView")
    }
}
