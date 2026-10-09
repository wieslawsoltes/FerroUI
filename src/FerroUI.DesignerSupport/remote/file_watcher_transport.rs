use crate::remote::remote_designer_entry_point::methods;
use ferroui_base::utilities::Uri;
use ferroui_remote_protocol::designer::UpdateXamlMessage;
use ferroui_remote_protocol::metsys_bson::{BsonObject, ValueRef};
use ferroui_remote_protocol::{
    Delegate, Error, ExceptionHandler, HandlerToken, IFerroRemoteTransportConnection, Message, MessageHandler, Task,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::Duration;

/// A pseudo-transport that triggers XAML updates on file changes: the
/// "other end" is a file, whose text is sent as an [`UpdateXamlMessage`]
/// whenever it changes. What is sent to it is printed.
pub(crate) struct FileWatcherTransport {
    this: Weak<FileWatcherTransport>,
    app_path: Option<String>,
    path: String,
    last_contents: Mutex<Option<String>>,
    disposed: AtomicBool,
    on_message: Mutex<Delegate<Message>>,
    // `OnException { add { } remove { } }`: the handlers are accepted and
    // never called.
    on_exception: Mutex<Delegate<Error>>,
}

impl FileWatcherTransport {
    pub(crate) fn new(file: &Uri, app_path: Option<String>) -> Arc<FileWatcherTransport> {
        Arc::new_cyclic(|this| FileWatcherTransport {
            this: this.clone(),
            app_path,
            path: file.local_path(),
            last_contents: Mutex::new(None),
            disposed: AtomicBool::new(false),
            on_message: Mutex::new(Delegate::new()),
            on_exception: Mutex::new(Delegate::new()),
        })
    }

    // Debug method.
    // Deviation (DEVIATIONS.md, Designer support): the original walks the
    // properties of any object by reflection, so that an array or an
    // enumeration prints the properties of its runtime type. Here the
    // declared properties of a message are printed; a value that is neither
    // a simple value nor an object of a declared class prints nothing after
    // its name.
    fn dump(o: &dyn BsonObject, pad: &str) {
        for (index, p) in o.get_type().properties.iter().enumerate() {
            print!("{pad}{}: ", p.name);
            match o.get_property_value(index) {
                ValueRef::Null => println!(),
                ValueRef::Int32(v) => println!("{v}"),
                ValueRef::Int64(v) => println!("{v}"),
                ValueRef::Boolean(v) => println!("{}", if v { "True" } else { "False" }),
                ValueRef::String(v) => println!("{v}"),
                ValueRef::Double(v) => println!("{v}"),
                ValueRef::Single(v) => println!("{v}"),
                ValueRef::Guid(v) => println!("{v}"),
                ValueRef::Object(v) => {
                    println!();
                    Self::dump(v, &format!("{pad}    "));
                }
                _ => println!(),
            }
        }
    }

    /// One turn of `UpdaterThread`: reads the file and raises the message
    /// when its text is not the text of the last turn.
    fn update(&self) -> std::io::Result<()> {
        let data = std::fs::read_to_string(&self.path)?;
        let changed = {
            let mut last_contents = self.last_contents.lock().unwrap_or_else(PoisonError::into_inner);
            if last_contents.as_deref() != Some(data.as_str()) {
                *last_contents = Some(data.clone());
                true
            } else {
                false
            }
        };
        if changed {
            println!("Triggering XAML update");
            let handlers = self.on_message.lock().unwrap_or_else(PoisonError::into_inner).snapshot();
            let message: Message = Arc::new(UpdateXamlMessage {
                xaml: Some(data),
                assembly_path: self.app_path.clone(),
                xaml_file_project_path: None,
            });
            for handler in &handlers {
                handler(self, &message);
            }
        }
        Ok(())
    }

    // I couldn't get FileSystemWatcher working on Linux, so I came up with this abomination
    //
    // Deviation (DEVIATIONS.md, Designer support): the original is an
    // `async void` loop that continues on the synchronization context of
    // the thread that called `Start`. Here it is a thread, the reader
    // thread of this connection in the sense of the threading contract of
    // the protocol. A file that cannot be read ends the thread with a panic
    // that names the error (the exception of the original is unhandled).
    fn updater_thread(this: Weak<FileWatcherTransport>) {
        loop {
            {
                let Some(this) = this.upgrade() else {
                    return;
                };
                if this.disposed.load(Ordering::SeqCst) {
                    return;
                }
                if let Err(e) = this.update() {
                    panic!("Unable to read '{}': {e}", this.path);
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// `ITransportWithEnforcedMethod.PreviewerMethod`.
    pub(crate) fn previewer_method(&self) -> &'static str {
        methods::HTML
    }
}

impl IFerroRemoteTransportConnection for FileWatcherTransport {
    fn dispose(&self) {
        self.disposed.store(true, Ordering::SeqCst);
    }

    fn send(&self, data: Message) -> Task {
        println!("{}", data.get_type().name);
        Self::dump(&*data, "    ");
        Task::completed()
    }

    fn on_message(&self, handler: MessageHandler) -> HandlerToken {
        self.on_message.lock().unwrap_or_else(PoisonError::into_inner).add(handler)
    }

    fn remove_on_message(&self, token: HandlerToken) {
        self.on_message.lock().unwrap_or_else(PoisonError::into_inner).remove(token);
    }

    fn on_exception(&self, handler: ExceptionHandler) -> HandlerToken {
        self.on_exception.lock().unwrap_or_else(PoisonError::into_inner).add(handler)
    }

    fn remove_on_exception(&self, token: HandlerToken) {
        self.on_exception.lock().unwrap_or_else(PoisonError::into_inner).remove(token);
    }

    fn start(&self) {
        let this = self.this.clone();
        std::thread::Builder::new()
            .name("designer-file-watcher".to_string())
            .spawn(move || Self::updater_thread(this))
            .expect("failed to start the file watcher thread");
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use ferroui_base::utilities::UriKind;
    use ferroui_remote_protocol::message_handler;
    use ferroui_remote_protocol::viewport::MeasureViewportMessage;
    use std::sync::mpsc::channel;

    const TIMEOUT: Duration = Duration::from_secs(10);

    #[test]
    fn a_change_of_the_file_is_an_update_of_the_xaml() {
        let path = std::env::temp_dir().join(format!("ferroui-file-watcher-{}.xaml", std::process::id()));
        std::fs::write(&path, "<A/>").unwrap();
        let uri = Uri::new(&format!("file://{}", path.display()), UriKind::Absolute).unwrap();
        let transport = FileWatcherTransport::new(&uri, Some("app".to_string()));
        assert_eq!("html", transport.previewer_method());

        let (sender, updates) = channel();
        let sender = Mutex::new(sender);
        transport.on_message(message_handler(move |_, message| {
            let update = message.downcast_ref::<UpdateXamlMessage>().expect("an update of the XAML").clone();
            let _ = sender.lock().unwrap().send(update);
        }));
        transport.start();

        let first = updates.recv_timeout(TIMEOUT).expect("the text of the file as it is");
        assert_eq!(Some("<A/>".to_string()), first.xaml);
        assert_eq!(Some("app".to_string()), first.assembly_path);
        assert_eq!(None, first.xaml_file_project_path);

        std::fs::write(&path, "<B/>").unwrap();
        let second = updates.recv_timeout(TIMEOUT).expect("the text of the file after the change");
        assert_eq!(Some("<B/>".to_string()), second.xaml);

        // What is sent is printed, and the task is completed.
        assert!(transport.send(Arc::new(MeasureViewportMessage { width: 1.0, height: 2.0 })).is_completed());

        transport.dispose();
        std::thread::sleep(Duration::from_millis(250));
        std::fs::write(&path, "<C/>").unwrap();
        assert!(updates.recv_timeout(Duration::from_millis(400)).is_err());
        let _ = std::fs::remove_file(&path);
    }
}
