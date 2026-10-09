//! Designer support: what an IDE needs to preview the XAML documents of an
//! application.
//!
//! - [`DesignWindowLoader`] loads the text of a document into the window
//!   that is previewed: a window as it is, a control as the content of a
//!   window, a style, a resource dictionary or a data template inside the
//!   control the document names with `Design.PreviewWith`.
//! - [`remote::RemoteDesignerEntryPoint`] is the previewer: it connects to
//!   the IDE over the remote protocol, receives the text of the document
//!   whenever it changes, and sends back frames of the previewed window
//!   and the errors of the document. With the `html` method
//!   ([`remote::html_transport::HtmlWebSocketTransport`]) the frames go to
//!   a web page the previewer serves, which sends back the mouse input.
//!
//! An application of the port is linked into its binary, so there is no
//! separate host process that loads the application: the binary of the
//! application is the host of its own previewer. It registers the run-time
//! XAML loader and calls [`remote::RemoteDesignerEntryPoint::main`] with
//! the arguments the IDE gave it and the function that builds its
//! application builder:
//!
//! ```ignore
//! fn main() {
//!     let args: Vec<String> = std::env::args().skip(1).collect();
//!     if args.iter().any(|arg| arg == "--transport") {
//!         FerroRuntimeXamlLoader::register();
//!         RemoteDesignerEntryPoint::main(&args, build_app);
//!     }
//!     // ... otherwise start the application as usual.
//! }
//! ```

mod design_window_loader;

pub mod remote;

pub use design_window_loader::DesignWindowLoader;
