//! A WinUI 3 application and its windows for a Rust program.
//!
//! The reference hosts the panel in an application written in C# and XAML,
//! which the tools of the SDK compile: `App : Application` with its
//! `OnLaunched`, `MainWindow : Window` with a grid that holds the panel, and
//! a `Main` that calls `Application.Start`. A Rust program has no XAML
//! compiler; this module is what that generated code does: [`XamlApplication::start`]
//! runs the application with a closure in place of `OnLaunched` (the
//! application object is composed, `interop/composable.rs`, and overrides
//! `OnLaunched`), gives it the resources of the samples' `App.xaml` (the
//! default styles of the controls of WinUI, `XamlControlsResources`) and the
//! type information of the controls library, and [`XamlWindow`] and
//! [`XamlGrid`] are the window and the grid of the sample's markup
//! (docs/porting/winui.md, section 3.4).

#[cfg(windows)]
pub use imp::{XamlApplication, XamlGrid, XamlWindow};

#[cfg(windows)]
mod imp {
    use crate::interop::composable::{compose, Composed, ComposedBase};
    use crate::interop::event_handlers::{application_initialization_callback, window_closed_handler};
    use crate::interop::win_rt::{activate, factory, hstring};
    use crate::interop::*;
    use ferroui_microcom::{ComPtr, HResult, InterfaceEntry};
    use ferroui_win32::wnd_proc_guard;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[cfg_attr(not(target_arch = "x86"), link(name = "combase", kind = "raw-dylib"))]
    #[cfg_attr(target_arch = "x86", link(name = "combase", kind = "raw-dylib", import_name_type = "undecorated"))]
    extern "system" {
        fn RoInitialize(init_type: i32) -> i32;
    }

    /// `RO_INIT_SINGLETHREADED`: the thread of a XAML application is a
    /// single-threaded apartment (`[STAThread]` of the C# application).
    const RO_INIT_SINGLETHREADED: i32 = 0;

    type Launched = RefCell<Option<Box<dyn FnOnce(&XamlApplication)>>>;

    /// The outer object of the composed `Application`: `App` of the
    /// reference's sample, whose `OnLaunched` it overrides. It is the
    /// provider of the XAML type information of the application too, as the
    /// class the XAML compiler generates for `App` is: XAML asks the
    /// application object for the types of the markup it loads, the
    /// resources of `XamlControlsResources` among them, and the generated
    /// provider hands the types of the controls library to that library's
    /// provider (`XamlControlsXamlMetaDataProvider`), which is all a program
    /// without markup types of its own has to answer.
    struct ApplicationOuter {
        base: ComposedBase,
        launched: Rc<Launched>,
        /// The provider of the controls library, created when XAML first
        /// asks for a type, as the generated provider creates it.
        provider: RefCell<Option<ComPtr<IXamlMetadataProvider>>>,
    }

    impl ApplicationOuter {
        fn provider(&self) -> Result<ComPtr<IXamlMetadataProvider>, HResult> {
            if let Some(provider) = self.provider.borrow().clone() {
                return Ok(provider);
            }
            step("XamlControlsXamlMetaDataProvider: ActivateInstance");
            let provider = activate::<IXamlMetadataProvider>(XAML_CONTROLS_METADATA_PROVIDER)?;
            *self.provider.borrow_mut() = Some(provider.clone());
            Ok(provider)
        }
    }

    const XAML_CONTROLS_METADATA_PROVIDER: &str = "Microsoft.UI.Xaml.XamlTypeInfo.XamlControlsXamlMetaDataProvider";

    /// A `[step]` line before a call of the start of the application, when
    /// `FERROUI_WINUI_TRACE` is set: a run that ends in the frames of XAML
    /// names the call it was in (docs/porting/winui.md, section 7.2).
    fn step(what: &str) {
        use std::io::Write;
        if std::env::var_os("FERROUI_WINUI_TRACE").is_some() {
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "[step] application: {what}");
            let _ = out.flush();
        }
    }

    /// The result of a call of the start of the application, with the call
    /// named in its error.
    fn named<T>(what: &'static str, result: Result<T, HResult>) -> Result<T, String> {
        result.map_err(|error| format!("{what}: {error}"))
    }

    impl Composed for ApplicationOuter {
        fn composed_base(&self) -> &ComposedBase {
            &self.base
        }
    }

    inspectable!(ApplicationOuter, "FerroUI.WinUI.XamlApplication", [IApplicationOverrides, IXamlMetadataProvider]);

    impl IApplicationOverridesImpl for ApplicationOuter {
        fn on_launched(&self, _args: Option<&IInspectable>) -> Result<(), HResult> {
            // The resources of the `App.xaml` of the samples, before the
            // windows of the application are created.
            let merged = self
                .base
                .base::<IApplication>()
                .map_err(|error| format!("the Application of the composed application: {error}"))
                .and_then(|application| merge_xaml_controls_resources(&application));
            if let Err(error) = merged {
                wnd_proc_guard::guard((), || panic!("merging XamlControlsResources into the resources of the application failed: {error}"));
                return Ok(());
            }
            let launched = self.launched.borrow_mut().take();
            if let Some(launched) = launched {
                wnd_proc_guard::guard((), || launched(&XamlApplication));
            }
            Ok(())
        }
    }

    impl IXamlMetadataProviderImpl for ApplicationOuter {
        fn get_xaml_type(&self, type_: TypeName) -> Result<*mut std::ffi::c_void, HResult> {
            // The result (an owned reference) is the caller's.
            self.provider()?.get_xaml_type(type_)
        }

        fn get_xaml_type_by_full_name(&self, full_name: isize) -> Result<*mut std::ffi::c_void, HResult> {
            self.provider()?.get_xaml_type_by_full_name(full_name)
        }

        fn get_xmlns_definitions(&self, result_size: *mut u32) -> Result<*mut XmlnsDefinition, HResult> {
            // SAFETY: the out parameter of the caller; the array (memory of
            // the COM allocator) is the caller's.
            unsafe { self.provider()?.get_xmlns_definitions(result_size) }
        }
    }

    /// `<Application.Resources>` of the `App.xaml` of the reference's
    /// samples: `XamlControlsResources` (the theme resources and the default
    /// styles of the controls of WinUI) among the merged dictionaries of the
    /// resources of the application.
    ///
    /// The XAML compiler loads the markup in the constructor of `App`
    /// (`InitializeComponent`); there the parser creates the dictionary
    /// through the type information of the application. A program without
    /// markup creates it in `OnLaunched`, before its first window: the
    /// creation of the application object inside the callback of
    /// `Application.Start` is not finished for XAML at that point (the
    /// activation there failed with `E_UNEXPECTED` on the CI runner, run
    /// 38787520732), and the effect for the windows is the same. The library
    /// of the controls is initialised first
    /// (`XamlControlsXamlMetaDataProvider.Initialize`, which the SDK provides
    /// for applications without markup). A `[step]` line names each call.
    fn merge_xaml_controls_resources(application: &IApplication) -> Result<(), String> {
        step("XamlControlsXamlMetaDataProvider.Initialize");
        let statics = named(
            "RoGetActivationFactory(XamlControlsXamlMetaDataProvider)",
            factory::<IXamlControlsXamlMetaDataProviderStatics>(XAML_CONTROLS_METADATA_PROVIDER),
        )?;
        named("XamlControlsXamlMetaDataProvider.Initialize", statics.initialize())?;

        step("Application.Resources");
        let resources = match named("Application.Resources", application.resources())? {
            Some(resources) => resources,
            None => {
                step("new ResourceDictionary(), Application.Resources = it");
                let dictionary_factory = named(
                    "RoGetActivationFactory(ResourceDictionary)",
                    factory::<IResourceDictionaryFactory>("Microsoft.UI.Xaml.ResourceDictionary"),
                )?;
                let mut inner: *mut IInspectable = std::ptr::null_mut();
                // SAFETY: no outer object (the dictionary is not derived
                // from); the inner object is not asked for and released
                // below.
                let dictionary = named("ResourceDictionary.CreateInstance", unsafe {
                    dictionary_factory.create_instance(None, &mut inner)
                })?
                .ok_or_else(|| "ResourceDictionary.CreateInstance: no dictionary".to_string())?;
                if !inner.is_null() {
                    // SAFETY: an owned reference the factory wrote.
                    unsafe { (*(inner as *const ferroui_microcom::IUnknown)).release() };
                }
                named("Application.Resources = a new dictionary", application.set_resources(Some(&dictionary)))?;
                dictionary
            }
        };

        step("new XamlControlsResources()");
        let controls_resources = named(
            "XamlControlsResources: ActivateInstance",
            activate::<IResourceDictionary>("Microsoft.UI.Xaml.Controls.XamlControlsResources"),
        )?;
        step("Application.Resources.MergedDictionaries");
        let merged = named("ResourceDictionary.MergedDictionaries", resources.merged_dictionaries())?
            .ok_or_else(|| "ResourceDictionary.MergedDictionaries: no list".to_string())?;
        step("MergedDictionaries.Append(XamlControlsResources)");
        named("MergedDictionaries.Append", merged.append(Some(&controls_resources)))?;
        step("XamlControlsResources merged");
        Ok(())
    }

    /// The application. See the module.
    pub struct XamlApplication;

    impl XamlApplication {
        /// Initialises the thread as a single-threaded apartment and runs
        /// `Application.Start`: the message loop of the application, until it
        /// exits. `on_launched` is called once, from `OnLaunched`. A panic of
        /// a handler the system called ends the loop and is raised again here.
        pub fn start(on_launched: impl FnOnce(&XamlApplication) + 'static) -> Result<(), HResult> {
            // SAFETY: takes a number. S_FALSE (initialised already) is a
            // success.
            let hr = unsafe { RoInitialize(RO_INIT_SINGLETHREADED) };
            if hr < 0 {
                return Err(HResult(hr as u32));
            }

            wnd_proc_guard::set_pending_hook(Some(Rc::new(|| XamlApplication.exit())));

            let launched: Rc<Launched> = Rc::new(RefCell::new(Some(Box::new(on_launched))));
            let application: Rc<RefCell<Option<ComPtr<IApplicationOverrides>>>> = Rc::new(RefCell::new(None));
            let callback = {
                let application = application.clone();
                application_initialization_callback(move |_| {
                    // `Application.Start(p => new App())` of the generated
                    // program: the application object is created on the
                    // thread of the application.
                    step("Application: CreateInstance of the composed application");
                    let created = factory::<IApplicationFactory>("Microsoft.UI.Xaml.Application").and_then(|application_factory| {
                        compose::<IApplicationOverrides, ApplicationOuter, 1>(
                            ApplicationOuter { base: ComposedBase::default(), launched: launched.clone(), provider: RefCell::new(None) },
                            [InterfaceEntry::of::<IXamlMetadataProvider>()],
                            |outer, inner| {
                                // SAFETY: the outer object and a place of the
                                // caller's frame.
                                unsafe { application_factory.create_instance(Some(outer), inner) }.map(|_| ())
                            },
                        )
                    });
                    let created = match created {
                        Ok(created) => created,
                        Err(error) => panic!("creating the XAML application failed: {error}"),
                    };
                    step("the application is created");
                    *application.borrow_mut() = Some(created);
                })
            };

            let statics = factory::<IApplicationStatics>("Microsoft.UI.Xaml.Application")?;
            step("Application.Start");
            let result = statics.start(Some(&callback));
            drop(application.take());
            wnd_proc_guard::set_pending_hook(None);
            wnd_proc_guard::resume_pending();
            result
        }

        /// `Application.Current.Exit()`: ends the message loop.
        pub fn exit(&self) {
            let current = factory::<IApplicationStatics>("Microsoft.UI.Xaml.Application")
                .and_then(|statics| statics.current())
                .ok()
                .flatten();
            if let Some(current) = current {
                let _ = current.exit();
            }
        }
    }

    /// A window of WinUI (`Microsoft.UI.Xaml.Window`).
    pub struct XamlWindow {
        window: ComPtr<IWindow>,
    }

    impl XamlWindow {
        pub fn new() -> Result<XamlWindow, HResult> {
            let window_factory = factory::<IWindowFactory>("Microsoft.UI.Xaml.Window")?;
            let mut inner: *mut IInspectable = std::ptr::null_mut();
            // SAFETY: no outer object (the window is not derived from); the
            // inner object is not asked for and released below.
            let window = unsafe { window_factory.create_instance(None, &mut inner) }?.ok_or(HResult::POINTER)?;
            if !inner.is_null() {
                // SAFETY: an owned reference the factory wrote.
                unsafe { (*(inner as *const ferroui_microcom::IUnknown)).release() };
            }
            Ok(XamlWindow { window })
        }

        pub fn set_title(&self, title: &str) -> Result<(), HResult> {
            self.window.set_title(hstring(title)?.handle())
        }

        pub fn set_content(&self, content: &IUIElement) -> Result<(), HResult> {
            self.window.set_content(Some(content))
        }

        pub fn activate(&self) -> Result<(), HResult> {
            self.window.activate()
        }

        pub fn close(&self) -> Result<(), HResult> {
            self.window.close()
        }

        /// `Closed`.
        pub fn on_closed(&self, handler: impl Fn() + 'static) -> Result<(), HResult> {
            self.window.add_closed(Some(&window_closed_handler(move |_, _| handler()))).map(|_| ())
        }

        /// The window of the bindings.
        pub fn window(&self) -> &ComPtr<IWindow> {
            &self.window
        }
    }

    /// A grid of WinUI (`Microsoft.UI.Xaml.Controls.Grid`).
    pub struct XamlGrid {
        grid: ComPtr<IInspectable>,
    }

    impl XamlGrid {
        pub fn new() -> Result<XamlGrid, HResult> {
            let grid_factory = factory::<IGridFactory>("Microsoft.UI.Xaml.Controls.Grid")?;
            let mut inner: *mut IInspectable = std::ptr::null_mut();
            // SAFETY: as `XamlWindow::new`.
            let grid = unsafe { grid_factory.create_instance(None, &mut inner) }?.ok_or(HResult::POINTER)?;
            if !inner.is_null() {
                // SAFETY: an owned reference the factory wrote.
                unsafe { (*(inner as *const ferroui_microcom::IUnknown)).release() };
            }
            Ok(XamlGrid { grid })
        }

        pub fn add_child(&self, child: &IUIElement) -> Result<(), HResult> {
            self.grid.cast::<IPanel>()?.children()?.ok_or(HResult::POINTER)?.append(Some(child))
        }

        pub fn as_ui_element(&self) -> Result<ComPtr<IUIElement>, HResult> {
            self.grid.cast::<IUIElement>()
        }
    }
}
