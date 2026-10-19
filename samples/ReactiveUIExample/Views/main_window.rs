//! Port of `Views/MainWindow.xaml.cs`: the class of the document `MainWindow.xaml`, the desktop
//! window that hosts [`MainView`](super::MainView).

use crate::markup::{reactive_window_class, xaml_class};
use crate::view_models::MainViewModel;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_reactiveui_controls::{ReactiveWindow, ReactiveWindowBase};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// The desktop window that hosts the main view (`MainWindow : ReactiveWindow<MainViewModel>`).
///
/// Deviation (DEVIATIONS.md, samples): the document derives from `ReactiveWindowBase`, the class
/// markup creates for `ReactiveWindow<TViewModel>`, and the code types the window with
/// [`ReactiveWindow<MainViewModel>`] ([`typed`](Self::typed)), as the integration's controls are
/// typed by a handle (DEVIATIONS.md, ReactiveUI controls).
#[repr(C)]
pub struct MainWindow {
    base: ReactiveWindowBase,
}

reactive_window_class!(MainWindow);
ferro_class_info!(MainWindow { new: MainWindow::new });
xaml_class!(MainWindow, "/Views/MainWindow.xaml");

impl MainWindow {
    /// Field initialisation, over a window of the windowing platform.
    pub fn construct() -> Self {
        Self { base: ReactiveWindowBase::construct() }
    }

    /// Creates the window (`MainWindow()`): it disposes its view model when it closes.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let _ = Self::typed(&this);
        let window = this.downgrade();
        let _ = this.closed(move || {
            if let Some(window) = window.upgrade() {
                Self::on_closed(&window);
            }
        });
        this
    }

    /// The window typed by its view model (`ReactiveWindow<MainViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveWindow<MainViewModel> {
        ReactiveWindow::<MainViewModel>::from_control(this.clone())
    }

    /// The view model of the window (`ViewModel`).
    pub fn view_model(this: &Ref<Self>) -> Option<Rc<MainViewModel>> {
        Self::typed(this).view_model()
    }

    /// The main view of the window (the element `Main`).
    pub fn main(&self) -> Ref<super::MainView> {
        self.get_control::<super::MainView>("Main")
    }

    /// Disposes the view model when the shell closes (`OnClosed`).
    fn on_closed(this: &Ref<Self>) {
        if let Some(view_model) = Self::view_model(this) {
            view_model.dispose();
        }
    }
}
