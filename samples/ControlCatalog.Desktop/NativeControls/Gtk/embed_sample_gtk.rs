//! Port of `EmbedSample.Gtk.cs`: the native control demo of Linux. The
//! first sample is a window of the X server in which `mplayer` plays a
//! video; the second is a file chooser of GTK.
//!
//! The video (`nodes.mp4`) is looked for beside the executable, as in the
//! reference, whose build copies it there, and then in the directory of
//! this file, because a cargo build copies nothing. Without `mplayer` the
//! first sample is the empty default control (the reference fails to start
//! the process).

use crate::gtk_helper;
use control_catalog::pages::INativeDemoControl;
use ferroui_controls::platform::IPlatformHandle;
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::rc::Rc;

#[derive(Default)]
pub struct EmbedSampleGtk {
    mplayer: RefCell<Option<Child>>,
}

/// The file of the video.
fn nodes_file() -> PathBuf {
    let relative = PathBuf::from("NativeControls").join("Gtk").join("nodes.mp4");
    let beside_the_executable = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|directory| directory.join(&relative)))
        .filter(|path| path.exists());
    beside_the_executable.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative))
}

impl INativeDemoControl for EmbedSampleGtk {
    fn create_control(
        &self,
        is_second: bool,
        parent: Rc<dyn IPlatformHandle>,
        create_default: &dyn Fn() -> Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn IPlatformHandle> {
        if is_second {
            if let Some(chooser) = gtk_helper::create_gtk_file_chooser(parent.handle()) {
                return chooser;
            }
        }

        let control = create_default();
        let nodes_file = nodes_file();

        let mplayer = Command::new("mplayer")
            .args(["-vo", "x11", "-zoom", "-loop", "0", "-wid"])
            .arg(control.handle().to_string())
            .arg(&nodes_file)
            .spawn();
        match mplayer {
            Ok(mplayer) => *self.mplayer.borrow_mut() = Some(mplayer),
            Err(error) => eprintln!("ControlCatalog: mplayer did not start ({error}); the native control stays empty"),
        }
        control
    }
}
