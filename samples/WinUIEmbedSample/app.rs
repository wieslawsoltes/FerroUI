//! `App` of the reference's sample: the WinUI application, which sets the framework up with the
//! application of the sample ([`winui_embed_sample::FerroApp`]) and a single view lifetime, and
//! shows the main window. The resources of its `App.xaml` (`XamlControlsResources`) are merged by
//! `ferroui_winui::XamlApplication` when it creates the application.

use crate::main_window::MainWindow;
use ferroui_base::Ref;
use ferroui_controls::application_lifetimes::{IApplicationLifetime, ISingleViewApplicationLifetime};
use ferroui_controls::{AppBuilder, Control};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_win32::Win32ApplicationExtensions;
use ferroui_winui::XamlApplication;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use winui_embed_sample::FerroApp;

ferroui_base::ferro_thread_local! {
    /// `App.Lifetime` of the reference.
    static LIFETIME: Rc<SingleViewLifetime> = Rc::new(SingleViewLifetime::default());
    /// `_window` of the reference's application.
    static WINDOW: RefCell<Option<Rc<MainWindow>>> = const { RefCell::new(None) };
}

pub struct App;

impl App {
    /// `App.Lifetime`.
    pub fn lifetime() -> Rc<SingleViewLifetime> {
        LIFETIME.with(Rc::clone)
    }

    /// Runs `body` with the main window, while the application holds it.
    pub fn with_main_window(body: impl FnOnce(&MainWindow)) {
        let window = WINDOW.with(|slot| slot.borrow().clone());
        if let Some(window) = window {
            body(&window);
        }
    }

    /// Lets go of the main window, if the application still holds it, and of the main view.
    ///
    /// The main view is let go of here, while the threads of the framework run, and not with
    /// the thread-local values of the main thread when the process ends: the system ends every
    /// other thread first (a thread inside a lock of the framework leaves it held), and then a
    /// destructor of the tree of controls that takes such a lock never returns
    /// (docs/porting/win32-platform.md, section 11.3; docs/porting/winui.md, section 7.3). The
    /// reference's runtime finalizes nothing at the end.
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

    /// `OnLaunched`.
    pub fn on_launched(smoke: bool) {
        let lifetime: Rc<dyn IApplicationLifetime> = Self::lifetime();
        AppBuilder::configure::<FerroApp>().use_win32().use_skia().use_harfbuzz().setup_with_lifetime(lifetime);
        crate::smoke::report(true, "the framework is set up: the Windows backend, Skia and HarfBuzz");

        let window = match MainWindow::new(smoke) {
            Ok(window) => window,
            Err(error) => panic!("creating the main window failed: {error}"),
        };
        crate::smoke::report(
            true,
            "the main window is created: the markup of the controls of WinUI loaded with the theme resources of XamlControlsResources, the panel beside them",
        );
        if let Err(error) = window.activate() {
            panic!("activating the main window failed: {error}");
        }
        // When the window is closed, the application lets go of it (its panel, whose content
        // stops rendering) and of the main view while XAML still runs, and ends: the objects of
        // XAML are not released after the loop of the application has returned.
        let closed = window.window().on_closed(|| {
            crate::smoke::end_step("Closed: the application lets go of the window and of the main view");
            App::shutdown();
            crate::smoke::end_step("Application.Exit");
            XamlApplication.exit();
        });
        if let Err(error) = closed {
            panic!("subscribing to the closing of the main window failed: {error}");
        }
        WINDOW.with(|slot| *slot.borrow_mut() = Some(window.clone()));
        if smoke {
            crate::smoke::start(window);
        }
    }
}

/// `SingleViewLifetime` of the reference's sample.
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
