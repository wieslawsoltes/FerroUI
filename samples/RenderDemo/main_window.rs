//! Port of `MainWindow.xaml.cs`: the class of the document `MainWindow.xaml`.

use crate::markup::xaml_class;
use crate::view_models::MainWindowViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::AnonymousObserver;
use ferroui_base::rendering::RendererDebugOverlays;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl, WindowTransparencyLevel,
    WindowTransparencyLevelCollection,
};
use mini_mvvm::PropertyChangedExtensions;
use std::rc::Rc;

#[repr(C)]
pub struct MainWindow {
    base: Window,
}

ferro_class!(MainWindow: Window);
ferro_impl_classes!(
    MainWindow: FerroObjectImpl,
    StyledElementImpl,
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
ferro_class_info!(MainWindow {
    new: MainWindow::new,
    markup: {
        methods: [
            fn SetNotTransparencyMenuItem_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.set_not_transparency_menu_item_on_click(&sender, e.as_routed_event_args())
                },
            fn SetTransparencyMenuItem_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.set_transparency_menu_item_on_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(MainWindow, "/MainWindow.xaml");

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let vm = MainWindowViewModel::new();

        // The managed original captures the window in the subscriptions of the view model,
        // which the window holds as its data context: a cycle its collector frees. Here the
        // subscriptions hold the window weakly.
        let bind_overlay = |property_name: &'static str, getter: fn(&MainWindowViewModel) -> bool, overlay: RendererDebugOverlays| {
            let window = this.downgrade();
            PropertyChangedExtensions::when_any_value(&vm, property_name, getter).subscribe(Rc::new(AnonymousObserver::new(
                move |x: bool| {
                    let Some(window) = window.upgrade() else { return };
                    let diagnostics = window.renderer_diagnostics();
                    diagnostics.set_debug_overlays(if x {
                        diagnostics.debug_overlays() | overlay
                    } else {
                        diagnostics.debug_overlays() & !overlay
                    });
                },
            )));
        };

        bind_overlay("DrawDirtyRects", |x| x.draw_dirty_rects(), RendererDebugOverlays::DIRTY_RECTS);
        bind_overlay("DrawFps", |x| x.draw_fps(), RendererDebugOverlays::FPS);
        bind_overlay("DrawLayoutTimeGraph", |x| x.draw_layout_time_graph(), RendererDebugOverlays::LAYOUT_TIME_GRAPH);
        bind_overlay("DrawRenderTimeGraph", |x| x.draw_render_time_graph(), RendererDebugOverlays::RENDER_TIME_GRAPH);

        this.set_data_context(Some(vm as BoxedValue));
        this
    }

    fn set_not_transparency_menu_item_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.set_transparency_level_hint(WindowTransparencyLevelCollection::new([WindowTransparencyLevel::none()]));
    }

    fn set_transparency_menu_item_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.set_transparency_level_hint(WindowTransparencyLevelCollection::new([WindowTransparencyLevel::transparent()]));
    }
}
