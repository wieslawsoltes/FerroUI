//! The WinUI application of the comparison host: it sets the framework up with the application
//! of the gallery (whose theme is the theme under test) and a single view lifetime, creates the
//! window of the comparison and starts the run. The resources of WinUI (`XamlControlsResources`)
//! are merged by `ferroui_winui::XamlApplication` when it creates the application.

use crate::compare_window::CompareWindow;
use ferroui_base::Ref;
use ferroui_controls::application_lifetimes::{IApplicationLifetime, ISingleViewApplicationLifetime};
use ferroui_controls::{AppBuilder, Control};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_win32::Win32ApplicationExtensions;
use ferroui_winui::XamlApplication;
use std::any::Any;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    static LIFETIME: Rc<SingleViewLifetime> = Rc::new(SingleViewLifetime::default());
    static WINDOW: RefCell<Option<Rc<CompareWindow>>> = const { RefCell::new(None) };
}

pub struct App;

impl App {
    pub fn lifetime() -> Rc<SingleViewLifetime> {
        LIFETIME.with(Rc::clone)
    }

    /// Lets go of the window, if the application still holds it, and of the main view, while
    /// the threads of the framework run (docs/porting/winui.md, sections 7.3 and 7.7).
    pub fn shutdown() {
        let window = WINDOW.with(|slot| slot.borrow_mut().take());
        drop(window);
        let main_view = LIFETIME.with(|lifetime| {
            let main_view = lifetime.main_view();
            lifetime.set_main_view(None);
            main_view
        });
        drop(main_view);
    }

    /// `OnLaunched`: the framework, the window, the run into `out`.
    pub fn on_launched(out: PathBuf) {
        let lifetime: Rc<dyn IApplicationLifetime> = Self::lifetime();
        AppBuilder::configure::<winui_theme_gallery::App>()
            .use_win32()
            .use_skia()
            .use_harfbuzz()
            .setup_with_lifetime(lifetime);
        println!("compare: the framework is set up with the gallery's application and the theme under test");

        let window = match CompareWindow::new() {
            Ok(window) => window,
            Err(error) => panic!("creating the window of the comparison failed: {error}"),
        };
        if let Err(error) = window.activate() {
            panic!("activating the window of the comparison failed: {error}");
        }
        // When the window is closed, the application lets go of it and of the main view while
        // XAML still runs, and ends.
        let closed = window.window().on_closed(|| {
            App::shutdown();
            XamlApplication.exit();
        });
        if let Err(error) = closed {
            panic!("subscribing to the closing of the window failed: {error}");
        }
        WINDOW.with(|slot| *slot.borrow_mut() = Some(window.clone()));
        crate::run::start(window, out);
    }
}

/// The single view lifetime of the host: its main view is the host of the framework's page.
#[derive(Default)]
pub struct SingleViewLifetime {
    main_view: RefCell<Option<Ref<Control>>>,
}

impl IApplicationLifetime for SingleViewLifetime {

    fn as_single_view_application_lifetime(&self) -> Option<&dyn ISingleViewApplicationLifetime> {
        Some(self)
    }
}

impl ISingleViewApplicationLifetime for SingleViewLifetime {
    fn main_view(&self) -> Option<Ref<Control>> {
        self.main_view.borrow().clone()
    }

    fn set_main_view(&self, value: Option<Ref<Control>>) {
        *self.main_view.borrow_mut() = value;
    }
}
