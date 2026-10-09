use crate::design_window_loader::DesignWindowLoader;
use crate::remote::file_watcher_transport::FileWatcherTransport;
use crate::remote::previewer_windowing_platform::PreviewerWindowingPlatform;
use ferroui_base::threading::{CancellationToken, Dispatcher, DispatcherPriority};
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::Ref;
use ferroui_controls::{AppBuilder, Design, Window};
use ferroui_markup_xaml::XamlLoadException;
use ferroui_remote_protocol::designer::{
    ExceptionDetails, StartDesignerSessionMessage, UpdateXamlMessage, UpdateXamlResultMessage,
};
use ferroui_remote_protocol::viewport::{
    ClientRenderInfoMessage, ClientSupportedPixelFormatsMessage, ClientViewportAllocatedMessage,
};
use ferroui_remote_protocol::{
    exception_handler, message_handler, BsonTcpTransport, Guid, IFerroRemoteTransportConnection, Message,
    TcpTransportBase,
};
use std::cell::{Cell, RefCell};
use std::hash::{BuildHasher, Hasher};
use std::net::IpAddr;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use xamlx::XamlError;

thread_local! {
    // The static fields of the original. They belong to the UI thread: the
    // handler of the connection posts every message to it.
    static SUPPORTED_PIXEL_FORMATS: RefCell<Option<Message>> = const { RefCell::new(None) };
    static VIEWPORT_ALLOCATED_MESSAGE: RefCell<Option<Message>> = const { RefCell::new(None) };
    static RENDER_INFO_MESSAGE: RefCell<Option<Message>> = const { RefCell::new(None) };
    static LAST_RENDER_SCALING: Cell<f64> = const { Cell::new(1.0) };
    static TRANSPORT: RefCell<Option<Arc<dyn IFerroRemoteTransportConnection>>> = const { RefCell::new(None) };
    static CURRENT_WINDOW: RefCell<Option<Ref<Window>>> = const { RefCell::new(None) };
}

/// The arguments of the previewer.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandLineArgs {
    pub app_path: Option<String>,
    pub transport: Option<Uri>,
    pub html_method_listen_uri: Option<Uri>,
    pub method: String,
    pub session_id: String,
}

impl CommandLineArgs {
    fn new() -> CommandLineArgs {
        CommandLineArgs {
            app_path: None,
            transport: None,
            html_method_listen_uri: None,
            method: methods::FERRO_REMOTE.to_string(),
            session_id: new_guid().to_string(),
        }
    }
}

/// `RemoteDesignerEntryPoint.Methods`: the ways the XAML is displayed.
pub mod methods {
    pub const FERRO_REMOTE: &str = "ferroui-remote";
    pub const WIN32: &str = "win32";
    pub const HTML: &str = "html";
}

/// The arguments are not the ones of the previewer: what `PrintUsage`
/// answers in the original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsageError;

/// `Guid.NewGuid()`: a random identifier of version four. The random bits
/// are the keys the standard library seeds its hash maps with.
fn new_guid() -> Guid {
    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        let random = std::collections::hash_map::RandomState::new().build_hasher().finish();
        chunk.copy_from_slice(&random.to_le_bytes());
    }
    bytes[7] = (bytes[7] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    Guid::from_byte_array(&bytes).expect("sixteen bytes are an identifier")
}

/// `Uri.Host` and `Uri.Port` of an absolute URI with an authority.
fn host_and_port(uri: &Uri) -> Option<(String, u16)> {
    let text = uri.absolute_uri();
    let rest = &text[text.find("://")? + 3..];
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    let authority = authority.rsplit('@').next()?;
    let (host, port) = match authority.strip_prefix('[') {
        // `[::1]:30243`
        Some(bracketed) => {
            let (host, port) = bracketed.split_once(']')?;
            (host, port.strip_prefix(':')?)
        }
        None => authority.rsplit_once(':')?,
    };
    Some((host.to_string(), port.parse().ok()?))
}

fn die(error: Option<&str>) -> ! {
    if let Some(error) = error {
        eprintln!("{error}");
    }
    std::process::exit(1)
}

fn log(message: &str) {
    println!("{message}");
}

/// The text `PrintUsage` prints.
fn usage() -> String {
    [
        "Usage: --transport transport_spec --session-id sid --method method app",
        "",
        "--transport: transport used for communication with the IDE",
        "    'tcp-bson' (e. g. 'tcp-bson://127.0.0.1:30243/') - TCP-based transport with BSON serialization of messages defined in FerroUI.Remote.Protocol",
        "    'file' (e. g. 'file://C://my/file.xaml' - pseudo-transport that triggers XAML updates on file changes, useful as a standalone previewer tool, always uses http preview method",
        "",
        "--session-id: session id to be sent to IDE process",
        "",
        "--method: the way the XAML is displayed",
        "    'ferroui-remote' - binary image is sent via transport connection in FrameMessage",
        "    'win32' - XAML is displayed in win32 window (handle could be obtained from UpdateXamlResultMessage), IDE is responsible to use user32!SetParent",
        "    'html' - Previewer starts an HTML server and displays XAML previewer as a web page",
        "",
        "--html-url - endpoint for HTML method to listen on, e. g. http://127.0.0.1:8081",
        "",
        "Example: --transport tcp-bson://127.0.0.1:30243/ --session-id 123 --method ferroui-remote MyApp.exe",
    ]
    .join("\n")
}

fn print_usage() -> ! {
    eprintln!("{}", usage());
    die(None)
}

/// The previewer: the entry point a host binary calls, and the handling of
/// the messages of the IDE.
pub struct RemoteDesignerEntryPoint;

impl RemoteDesignerEntryPoint {
    /// `ParseCommandLineArgs`. Where the original prints the usage and
    /// exits, this returns the error; [`main`](Self::main) prints and exits.
    pub fn parse_command_line_args(args: &[String]) -> Result<CommandLineArgs, UsageError> {
        #[derive(Clone, Copy)]
        enum Next {
            Transport,
            Method,
            HtmlUrl,
            SessionId,
        }

        let mut rv = CommandLineArgs::new();
        let mut next: Option<Next> = None;
        for arg in args {
            if let Some(option) = next.take() {
                match option {
                    Next::Transport => rv.transport = Some(Uri::new(arg, UriKind::Absolute).map_err(|_| UsageError)?),
                    Next::Method => rv.method = arg.clone(),
                    Next::HtmlUrl => {
                        rv.html_method_listen_uri = Some(Uri::new(arg, UriKind::Absolute).map_err(|_| UsageError)?)
                    }
                    Next::SessionId => rv.session_id = arg.clone(),
                }
            } else if arg == "--transport" {
                next = Some(Next::Transport);
            } else if arg == "--method" {
                next = Some(Next::Method);
            } else if arg == "--html-url" {
                next = Some(Next::HtmlUrl);
            } else if arg == "--session-id" {
                next = Some(Next::SessionId);
            } else if rv.app_path.is_none() {
                rv.app_path = Some(arg.clone());
            } else {
                return Err(UsageError);
            }
        }
        if rv.app_path.is_none() || rv.transport.is_none() {
            return Err(UsageError);
        }

        if next.is_some() {
            return Err(UsageError);
        }
        Ok(rv)
    }

    /// `CreateTransport`: the connection the transport of the arguments
    /// names and, for a transport that enforces one
    /// (`ITransportWithEnforcedMethod`), the method of the previewer.
    fn create_transport(
        args: &CommandLineArgs,
    ) -> Result<(Arc<dyn IFerroRemoteTransportConnection>, Option<&'static str>), String> {
        let Some(transport) = &args.transport else {
            print_usage()
        };
        if transport.scheme() == "tcp-bson" {
            let (host, port) =
                host_and_port(transport).ok_or_else(|| format!("Invalid URI: no host and port in '{}'", transport.original_string()))?;
            let address: IpAddr = host.parse().map_err(|_| "An invalid IP address was specified.".to_string())?;
            let connection = BsonTcpTransport::empty().connect(address, port).map_err(|e| e.to_string())?;
            return Ok((connection, None));
        }

        if transport.scheme() == "file" {
            let connection = FileWatcherTransport::new(transport, args.app_path.clone());
            let method = connection.previewer_method();
            return Ok((connection, Some(method)));
        }
        print_usage()
    }

    /// `AppInitializer.ConfigureApp`: gives the builder of the application
    /// the runtime platform and the windowing subsystem of the method, and
    /// sets the application up without starting it.
    fn configure_app(
        transport: Arc<dyn IFerroRemoteTransportConnection>,
        args: &CommandLineArgs,
        builder: &AppBuilder,
    ) -> Result<Arc<dyn IFerroRemoteTransportConnection>, String> {
        let builder = builder.use_standard_runtime_platform_subsystem();
        if args.method == methods::FERRO_REMOTE {
            let transport = transport.clone();
            drop(builder.use_windowing_subsystem(move || PreviewerWindowingPlatform::initialize(transport.clone()), ""));
        }
        // Left out (DEVIATIONS.md, Designer support): the HTML method wraps
        // the connection in `HtmlWebSocketTransport`, which serves the web
        // application of the upstream project; neither is ported.
        if args.method == methods::HTML {
            return Err("The 'html' method is not available: the HTML transport of the previewer is not ported.".to_string());
        }

        // Left out: the win32 method initializes the Win32 platform, which
        // the port does not have.
        if args.method == methods::WIN32 {
            return Err("The 'win32' method is not available: there is no Win32 windowing platform.".to_string());
        }
        drop(builder.setup_without_starting());
        Ok(transport)
    }

    /// `Main`: the entry point of the previewer. It does not return: the
    /// previewer runs until the IDE ends the process, and exits with the
    /// usage when the arguments are not valid.
    ///
    /// Deviation (DEVIATIONS.md, Designer support): the original loads the
    /// assembly of the application from the path in the arguments and finds
    /// the method that builds its application builder by reflection. An
    /// application of the port is linked into its binary, so the binary of
    /// the application is the host of its own previewer: it calls this
    /// function with its arguments and the function that builds its
    /// application builder. The run-time XAML loader is registered by the
    /// host, as the host application of the original does.
    pub fn main(cmdline: &[String], build_app: impl FnOnce() -> AppBuilder) -> ! {
        let mut args = match Self::parse_command_line_args(cmdline) {
            Ok(args) => args,
            Err(UsageError) => print_usage(),
        };
        let (transport, enforced_method) = match Self::create_transport(&args) {
            Ok(transport) => transport,
            Err(error) => die(Some(&error)),
        };
        if let Some(enforced_method) = enforced_method {
            args.method = enforced_method.to_string();
        }
        log("Initializing application in design mode");
        Design::set_is_design_mode(true);
        log("Obtaining AppBuilder instance from the application");
        let app_builder = build_app();
        let transport = match Self::configure_app(transport, &args, &app_builder) {
            Ok(transport) => transport,
            Err(error) => die(Some(&error)),
        };
        Self::attach_transport(transport.clone());
        transport.on_exception(exception_handler(|_, e| {
            die(Some(&e.to_string()));
        }));
        transport.start();
        log("Sending StartDesignerSessionMessage");
        transport.send(Arc::new(StartDesignerSessionMessage { session_id: Some(args.session_id.clone()) }));

        Dispatcher::ui_thread().main_loop(&CancellationToken::none());
        std::process::exit(0)
    }

    /// `s_transport = transport; transport.OnMessage += OnTransportMessage`.
    /// Called on the UI thread, whose dispatcher the handler posts to.
    pub(crate) fn attach_transport(transport: Arc<dyn IFerroRemoteTransportConnection>) {
        TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport.clone()));
        // `OnTransportMessage`: runs on the reader thread of the connection
        // and posts the message, which is plain data, to the UI thread.
        let dispatcher = Dispatcher::ui_thread();
        transport.on_message(message_handler(move |_, obj| {
            let arg = obj.clone();
            dispatcher.post(move || Self::on_transport_message(&arg), DispatcherPriority::DEFAULT);
        }));
    }

    fn rebuild_pre_flight() {
        // The original lists the three messages, the ones that have not
        // arrived as null; a window ignores a null message.
        let messages = [&SUPPORTED_PIXEL_FORMATS, &VIEWPORT_ALLOCATED_MESSAGE, &RENDER_INFO_MESSAGE]
            .into_iter()
            .filter_map(|slot| slot.with(|slot| slot.borrow().clone()))
            .collect();
        PreviewerWindowingPlatform::set_pre_flight_messages(messages);
    }

    /// The part of `OnTransportMessage` that runs on the UI thread.
    fn on_transport_message(arg: &Message) {
        if arg.is::<ClientSupportedPixelFormatsMessage>() {
            SUPPORTED_PIXEL_FORMATS.with(|slot| *slot.borrow_mut() = Some(arg.clone()));
            Self::rebuild_pre_flight();
        }
        if arg.is::<ClientRenderInfoMessage>() {
            RENDER_INFO_MESSAGE.with(|slot| *slot.borrow_mut() = Some(arg.clone()));
            Self::rebuild_pre_flight();
        }
        if arg.is::<ClientViewportAllocatedMessage>() {
            VIEWPORT_ALLOCATED_MESSAGE.with(|slot| *slot.borrow_mut() = Some(arg.clone()));
            Self::rebuild_pre_flight();
        }
        if let Some(xaml) = arg.downcast_ref::<UpdateXamlMessage>() {
            let current_window = CURRENT_WINDOW.with(|slot| slot.borrow_mut().take());
            if let Some(current_window) = current_window {
                LAST_RENDER_SCALING.set(current_window.render_scaling());

                //Ignore
                let _ = catch_unwind(AssertUnwindSafe(|| current_window.close()));
            }
            let Some(transport) = TRANSPORT.with(|slot| slot.borrow().clone()) else {
                return;
            };
            // `catch (Exception e)`: an error of the document, and whatever
            // else fails while the window is built and shown (a panic here).
            let loaded = catch_unwind(AssertUnwindSafe(|| {
                DesignWindowLoader::load_designer_window_with_scaling(
                    xaml.xaml.as_deref().unwrap_or(""),
                    xaml.assembly_path.as_deref(),
                    xaml.xaml_file_project_path.as_deref(),
                    LAST_RENDER_SCALING.get(),
                )
            }));
            let result = match loaded {
                Ok(Ok(window)) => {
                    let handle = window.platform_impl().and_then(|platform_impl| platform_impl.handle());
                    CURRENT_WINDOW.with(|slot| *slot.borrow_mut() = Some(window));
                    UpdateXamlResultMessage {
                        handle: handle.map(|handle| handle.handle().to_string()),
                        ..UpdateXamlResultMessage::default()
                    }
                }
                Ok(Err(e)) => UpdateXamlResultMessage {
                    error: Some(e.to_string()),
                    exception: Some(exception_details(&e)),
                    ..UpdateXamlResultMessage::default()
                },
                Err(payload) => {
                    let message = if let Some(text) = payload.downcast_ref::<&str>() {
                        (*text).to_string()
                    } else if let Some(text) = payload.downcast_ref::<String>() {
                        text.clone()
                    } else {
                        "The document could not be previewed.".to_string()
                    };
                    UpdateXamlResultMessage {
                        error: Some(message.clone()),
                        exception: Some(ExceptionDetails {
                            exception_type: Some("Panic".to_string()),
                            message: Some(message),
                            line_number: None,
                            line_position: None,
                        }),
                        ..UpdateXamlResultMessage::default()
                    }
                }
            };
            transport.send(Arc::new(result));
        }
    }

    /// Forgets the session. For tests, which run one after another on
    /// threads that are reused.
    #[cfg(test)]
    pub(crate) fn reset_for_unit_tests() {
        SUPPORTED_PIXEL_FORMATS.with(|slot| *slot.borrow_mut() = None);
        VIEWPORT_ALLOCATED_MESSAGE.with(|slot| *slot.borrow_mut() = None);
        RENDER_INFO_MESSAGE.with(|slot| *slot.borrow_mut() = None);
        LAST_RENDER_SCALING.set(1.0);
        TRANSPORT.with(|slot| *slot.borrow_mut() = None);
        CURRENT_WINDOW.with(|slot| *slot.borrow_mut() = None);
    }
}

/// `new ExceptionDetails(e)`: the name of the class of the error, its
/// message and, for an error of the document (an `XmlException` or a class
/// that derives from it), its line and position.
fn exception_details(e: &XamlLoadException) -> ExceptionDetails {
    if let Some(inner) = e.inner_exception().and_then(|inner| inner.downcast_ref::<XamlError>()) {
        let line_info = inner.is_xml_exception();
        return ExceptionDetails {
            exception_type: Some(inner.type_name().to_string()),
            message: Some(inner.message()),
            line_number: inner.line_number().filter(|_| line_info),
            line_position: inner.line_position().filter(|_| line_info),
        };
    }
    ExceptionDetails {
        exception_type: Some("XamlLoadException".to_string()),
        message: Some(e.message().to_string()),
        line_number: None,
        line_position: None,
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project tests the entry point by
    // starting the previewer process. The argument parser and the message
    // handling are tested here in the process of the test.
    use super::*;
    use crate::remote::test_connection::TestConnection;
    use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
    use ferroui_remote_protocol::viewport::{PixelFormat, RequestViewportResizeMessage};
    use std::rc::Rc;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn the_arguments_of_the_ide_are_parsed() {
        let parsed = RemoteDesignerEntryPoint::parse_command_line_args(&args(&[
            "--transport",
            "tcp-bson://127.0.0.1:30243/",
            "--session-id",
            "123",
            "--method",
            "win32",
            "--html-url",
            "http://127.0.0.1:8081",
            "MyApp.exe",
        ]))
        .unwrap();
        assert_eq!(Some("MyApp.exe".to_string()), parsed.app_path);
        assert_eq!("tcp-bson", parsed.transport.as_ref().unwrap().scheme());
        assert_eq!(Some(("127.0.0.1".to_string(), 30243)), host_and_port(parsed.transport.as_ref().unwrap()));
        assert_eq!("123", parsed.session_id);
        assert_eq!("win32", parsed.method);
        assert_eq!(
            Some(("127.0.0.1".to_string(), 8081)),
            host_and_port(parsed.html_method_listen_uri.as_ref().unwrap())
        );
    }

    #[test]
    fn the_method_and_the_session_have_defaults() {
        let parsed =
            RemoteDesignerEntryPoint::parse_command_line_args(&args(&["app", "--transport", "file:///tmp/a.xaml"])).unwrap();
        assert_eq!(methods::FERRO_REMOTE, parsed.method);
        assert!(parsed.html_method_listen_uri.is_none());
        // The session is a new identifier, different every time.
        let session = Guid::parse(&parsed.session_id).expect("an identifier");
        let other =
            RemoteDesignerEntryPoint::parse_command_line_args(&args(&["app", "--transport", "file:///tmp/a.xaml"])).unwrap();
        assert_ne!(session.to_string(), other.session_id);
        assert_eq!("file", parsed.transport.unwrap().scheme());
    }

    #[test]
    fn arguments_that_are_not_those_of_the_previewer_are_a_usage_error() {
        let parse = |values: &[&str]| RemoteDesignerEntryPoint::parse_command_line_args(&args(values));
        // No application, no transport.
        assert_eq!(Err(UsageError), parse(&[]));
        assert_eq!(Err(UsageError), parse(&["--transport", "tcp-bson://127.0.0.1:1/"]));
        assert_eq!(Err(UsageError), parse(&["app"]));
        // A second application.
        assert_eq!(Err(UsageError), parse(&["app", "other", "--transport", "tcp-bson://127.0.0.1:1/"]));
        // A transport that is not an absolute URI.
        assert_eq!(Err(UsageError), parse(&["app", "--transport", "not a uri"]));
        assert_eq!(Err(UsageError), parse(&["app", "--transport", "tcp-bson://127.0.0.1:1/", "--html-url", "x"]));
        // An option without its value.
        assert_eq!(Err(UsageError), parse(&["app", "--transport", "tcp-bson://127.0.0.1:1/", "--method"]));
        // The value of an option is taken as it is, whatever it looks like.
        let parsed = parse(&["app", "--transport", "tcp-bson://127.0.0.1:1/", "--session-id", "--method"]).unwrap();
        assert_eq!("--method", parsed.session_id);
    }

    #[test]
    fn the_host_and_the_port_of_a_transport_are_read_from_its_uri() {
        let uri = |text: &str| Uri::new(text, UriKind::Absolute).unwrap();
        assert_eq!(Some(("localhost".to_string(), 5000)), host_and_port(&uri("http://localhost:5000")));
        assert_eq!(Some(("::1".to_string(), 30243)), host_and_port(&uri("tcp-bson://[::1]:30243/")));
        assert_eq!(None, host_and_port(&uri("tcp-bson://127.0.0.1/")));
    }

    #[test]
    fn the_usage_names_the_options() {
        let usage = usage();
        for option in ["--transport", "--session-id", "--method", "--html-url", "tcp-bson", "ferroui-remote"] {
            assert!(usage.contains(option), "{option} is not in the usage");
        }
    }

    #[test]
    fn a_new_identifier_is_of_version_four() {
        let text = new_guid().to_string();
        assert_eq!(36, text.len());
        assert_eq!(Some('4'), text.chars().nth(14));
    }

    /// A session of the previewer in the test: a unit test application with
    /// the run-time loader, the windowing platform of the previewer over a
    /// connection the test holds the other end of, and the handler of the
    /// entry point on that connection.
    struct Session {
        connection: Arc<TestConnection>,
        app: UnitTestApplicationScope,
    }

    impl Session {
        fn start() -> Session {
            ferroui_markup_xaml::register_types();
            let app = UnitTestApplication::start(TestServices::styled_window());
            FerroRuntimeXamlLoader::register();
            RemoteDesignerEntryPoint::reset_for_unit_tests();
            PreviewerWindowingPlatform::reset_for_unit_tests();
            let connection = TestConnection::new();
            PreviewerWindowingPlatform::initialize(connection.clone());
            RemoteDesignerEntryPoint::attach_transport(connection.clone());
            Session { connection, app }
        }

        /// A message of the IDE, with the jobs it posts to the UI thread.
        fn receive(&self, message: Message) {
            self.connection.raise_message(message);
            Dispatcher::ui_thread().run_jobs(None);
        }

        fn update_xaml(&self, xaml: &str) -> UpdateXamlResultMessage {
            let before = self.connection.sent_of::<UpdateXamlResultMessage>().len();
            self.receive(Arc::new(UpdateXamlMessage { xaml: Some(xaml.to_string()), ..UpdateXamlMessage::default() }));
            let results = self.connection.sent_of::<UpdateXamlResultMessage>();
            assert_eq!(before + 1, results.len(), "an update is answered with one result");
            results.last().unwrap().clone()
        }

        fn end(self) {
            let window = CURRENT_WINDOW.with(|slot| slot.borrow_mut().take());
            if let Some(window) = window {
                window.close();
            }
            RemoteDesignerEntryPoint::reset_for_unit_tests();
            PreviewerWindowingPlatform::reset_for_unit_tests();
            self.app.dispose();
        }
    }

    const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

    #[test]
    fn an_update_of_the_xaml_is_answered_with_a_result_and_the_window_of_the_document() {
        let session = Session::start();
        let result = session.update_xaml(&format!("<Window {XMLNS} Width='120' Height='80'><Border/></Window>"));
        assert_eq!(None, result.error);
        assert_eq!(None, result.exception);
        // The window of the previewer has no handle of the platform.
        assert_eq!(None, result.handle);

        // The window is the embeddable window of the previewer, which asks
        // the client to resize its viewport to the size of the window.
        let window = CURRENT_WINDOW.with(|slot| slot.borrow().clone()).expect("the window of the document");
        let platform_impl = PreviewerWindowingPlatform::last_window().expect("the window of the previewer");
        assert!(window.platform_impl().is_some_and(|window_impl| {
            std::ptr::addr_eq(Rc::as_ptr(&window_impl), Rc::as_ptr(&platform_impl))
        }));
        let resizes = session.connection.sent_of::<RequestViewportResizeMessage>();
        assert_eq!(Some(&RequestViewportResizeMessage { width: 120.0, height: 80.0 }), resizes.last());

        // The next update closes the window and shows another one.
        let result = session.update_xaml(&format!("<UserControl {XMLNS}><Border Width='30' Height='20'/></UserControl>"));
        assert_eq!(None, result.error);
        assert!(!window.is_visible());
        let next = CURRENT_WINDOW.with(|slot| slot.borrow().clone()).unwrap();
        assert!(next != window);
        session.end();
    }

    #[test]
    fn an_update_with_invalid_xaml_is_answered_with_the_details_of_the_error() {
        let session = Session::start();
        let result = session.update_xaml(&format!("<Window {XMLNS}>\n  <NoSuchControl/>\n</Window>"));
        let error = result.error.expect("the error of the document");
        assert!(!error.is_empty());
        assert_eq!(None, result.handle);
        let details = result.exception.expect("the details of the error");
        assert!(details.exception_type.is_some_and(|name| !name.is_empty()));
        assert!(details.message.is_some_and(|message| !message.is_empty()));
        // The unknown element is on the second line of the document.
        assert_eq!(Some(2), details.line_number);
        assert!(details.line_position.is_some());
        assert!(CURRENT_WINDOW.with(|slot| slot.borrow().is_none()));

        // Text that is not XML.
        let result = session.update_xaml("<Window");
        assert!(result.error.is_some());
        assert!(result.exception.is_some());

        // A valid document after an invalid one is previewed.
        let result = session.update_xaml(&format!("<Border {XMLNS}/>"));
        assert_eq!(None, result.error);
        session.end();
    }

    #[test]
    fn the_messages_of_the_client_are_kept_for_the_windows_that_follow() {
        let session = Session::start();
        assert!(PreviewerWindowingPlatform::pre_flight_messages().is_empty());
        session.receive(Arc::new(ClientSupportedPixelFormatsMessage { formats: Some(vec![PixelFormat::Rgba8888]) }));
        session.receive(Arc::new(ClientViewportAllocatedMessage { width: 1024.0, height: 768.0, dpi_x: 192.0, dpi_y: 192.0 }));
        let messages = PreviewerWindowingPlatform::pre_flight_messages();
        assert_eq!(2, messages.len());
        assert!(messages[0].is::<ClientSupportedPixelFormatsMessage>());
        assert!(messages[1].is::<ClientViewportAllocatedMessage>());
        session.receive(Arc::new(ClientRenderInfoMessage { dpi_x: 192.0, dpi_y: 192.0 }));
        assert_eq!(3, PreviewerWindowingPlatform::pre_flight_messages().len());

        // A window created now is given them: it takes the scaling of the
        // client and not the size of its viewport, and asks for a viewport
        // of its own size in pixels.
        let result = session.update_xaml(&format!("<Window {XMLNS} Width='100' Height='50'><Border/></Window>"));
        assert_eq!(None, result.error);
        Dispatcher::ui_thread().run_jobs(None);
        let platform_impl = PreviewerWindowingPlatform::last_window().unwrap();
        assert_eq!(2.0, platform_impl.base().render_scaling());
        assert_eq!(ferroui_base::Size::new(100.0, 50.0), platform_impl.base().client_size());
        let resizes = session.connection.sent_of::<RequestViewportResizeMessage>();
        assert!(resizes.contains(&RequestViewportResizeMessage { width: 200.0, height: 100.0 })
            || resizes.contains(&RequestViewportResizeMessage { width: 100.0, height: 50.0 }));
        session.end();
    }
}
