//! Port of `Vnc/Server/VncServerSession.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).
//!
//! The session is shared between the thread that reads the messages of the
//! client, the thread that sends the framebuffer updates and the code that
//! created it, so it lives in an `Arc` and its state is behind locks. The
//! monitors of the original are a reentrant lock where the session takes
//! one again while it holds it (`FramebufferUpdateRequestLock`, `SyncRoot`
//! of the stream); its other fields have locks of their own. Exceptions
//! are errors ([`VncError`]) where the protocol fails and panics where
//! the original throws on a misuse; a panic of an event handler or of the
//! framebuffer source ends the session as an exception of them does in the
//! original.
//!
//! Two corrections: the status of a desktop size change is sent in the
//! y-position of the extended desktop size rectangle, as the protocol says
//! (the original always sends 0, success), and the statistics are logged
//! only when the session has a logger (the original logs them through a
//! logger it does not check).

use super::{
    CreatingDesktopEventArgs, EncoderStatistics, FramebufferUpdateRequest,
    FramebufferUpdatingEventArgs, IVncFramebufferCache, IVncServerSession,
    PasswordProvidedEventArgs, RawEncoder, TightEncoder, VncEncoder, VncEvent, VncFramebufferCache,
    VncMessageType, VncServerSessionOptions,
};
use crate::remote_viewing::logging::{EventId, ILogger, LoggerExtensions};
use crate::remote_viewing::utility::{AutoClear, PeriodicThread, ReentrantLock};
use crate::remote_viewing::vnc::{
    AuthenticationMethod, ExtendedDesktopSizeReason, ExtendedDesktopSizeStatus,
    IVncFramebufferSource, IVncPasswordChallenge, KeyChangedEventArgs, KeySym,
    PointerChangedEventArgs, RemoteClipboardChangedEventArgs, TransportWriter, Version,
    VncEncoding, VncError, VncFailureReason, VncFramebuffer, VncPasswordChallenge,
    VncPixelFormat, VncRectangle, VncStream, VncTransport, VncUtility,
};
use std::any::Any;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, Weak};
use std::thread::{self, JoinHandle};

/// The function that initializes a new framebuffer cache for a session:
/// it gets the framebuffer and the logger of the session.
pub type CreateFramebufferCache = Arc<
    dyn Fn(Arc<VncFramebuffer>, Option<Arc<dyn ILogger>>) -> Arc<dyn IVncFramebufferCache>
        + Send
        + Sync,
>;

/// A rectangle of a framebuffer update.
struct Rectangle {
    region: VncRectangle,
    encoding: VncEncoding,
    contents: Vec<u8>,
}

fn lock<T: ?Sized>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn read<T: Clone>(lock: &RwLock<T>) -> T {
    lock.read().unwrap_or_else(PoisonError::into_inner).clone()
}

fn write<T>(lock: &RwLock<T>, value: T) {
    *lock.write().unwrap_or_else(PoisonError::into_inner) = value;
}

/// The message of a panic: the text it was raised with.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "a panic".to_owned()
    }
}

/// Serves a VNC client with framebuffer information and receives keyboard
/// and mouse interactions.
pub struct VncServerSession {
    this: Weak<VncServerSession>,
    logger: RwLock<Option<Arc<dyn ILogger>>>,
    password_challenge: RwLock<Arc<dyn IVncPasswordChallenge>>,
    c: VncStream,
    client_encoding: RwLock<Vec<VncEncoding>>,
    client_pixel_format: RwLock<VncPixelFormat>,
    client_width: Mutex<i32>,
    client_height: Mutex<i32>,
    client_version: Mutex<Option<Version>>,
    options: Mutex<VncServerSessionOptions>,
    fbu_auto_cache: Mutex<Option<Arc<dyn IVncFramebufferCache>>>,
    fbu_rectangles: Mutex<Vec<Rectangle>>,
    fbu_sync: ReentrantLock,
    fb_source: RwLock<Option<Arc<dyn IVncFramebufferSource>>>,
    max_update_rate: Mutex<f64>,
    requester: Mutex<Option<Arc<PeriodicThread>>>,
    thread_main: Mutex<Option<JoinHandle<()>>>,
    security_negotiated: AtomicBool,

    // Used by `handle_message` to avoid flooding the event log.
    previous_command: Mutex<VncMessageType>,
    command_count: Mutex<i32>,

    password_provided: VncEvent<PasswordProvidedEventArgs>,
    creating_desktop: VncEvent<CreatingDesktopEventArgs>,
    connected: VncEvent<()>,
    connection_failed: VncEvent<()>,
    closed: VncEvent<()>,
    framebuffer_capturing: VncEvent<()>,
    framebuffer_updating: VncEvent<FramebufferUpdatingEventArgs>,
    key_changed: VncEvent<KeyChangedEventArgs>,
    pointer_changed: VncEvent<PointerChangedEventArgs>,
    remote_clipboard_changed: VncEvent<RemoteClipboardChangedEventArgs>,

    framebuffer: RwLock<Option<Arc<VncFramebuffer>>>,
    framebuffer_update_request: Mutex<Option<FramebufferUpdateRequest>>,
    is_connected: AtomicBool,
    user_data: Mutex<Option<Arc<dyn Any + Send + Sync>>>,
    create_framebuffer_cache: RwLock<CreateFramebufferCache>,
    encoders: Mutex<Vec<Arc<dyn VncEncoder>>>,
    encoder: Mutex<Arc<dyn VncEncoder>>,
    statistics: Mutex<HashMap<VncEncoding, EncoderStatistics>>,
}

impl VncServerSession {
    /// Initializes a new instance of the [`VncServerSession`] class, with
    /// the password challenge of the library and no logger.
    pub fn new() -> Arc<VncServerSession> {
        Self::new_with(Arc::new(VncPasswordChallenge::new()), None)
    }

    /// Initializes a new instance of the [`VncServerSession`] class:
    /// `password_challenge` generates the password challenges, `logger`
    /// logs diagnostic messages.
    pub fn new_with(
        password_challenge: Arc<dyn IVncPasswordChallenge>,
        logger: Option<Arc<dyn ILogger>>,
    ) -> Arc<VncServerSession> {
        let session = Arc::new_cyclic(|this: &Weak<VncServerSession>| {
            let create_framebuffer_cache: CreateFramebufferCache =
                Arc::new(|framebuffer, log| Arc::new(VncFramebufferCache::new(framebuffer, log)));
            let raw: Arc<dyn VncEncoder> = Arc::new(RawEncoder);
            let parent: Weak<dyn IVncServerSession> = this.clone();
            let tight: Arc<dyn VncEncoder> = Arc::new(TightEncoder::new(parent));

            VncServerSession {
                this: this.clone(),
                logger: RwLock::new(logger),
                password_challenge: RwLock::new(password_challenge),
                c: VncStream::new(),
                client_encoding: RwLock::new(Vec::new()),
                client_pixel_format: RwLock::new(VncPixelFormat::RGB32),
                client_width: Mutex::new(0),
                client_height: Mutex::new(0),
                client_version: Mutex::new(None),
                options: Mutex::new(VncServerSessionOptions::default()),
                fbu_auto_cache: Mutex::new(None),
                fbu_rectangles: Mutex::new(Vec::new()),
                fbu_sync: ReentrantLock::new(),
                fb_source: RwLock::new(None),
                max_update_rate: Mutex::new(15.0),
                requester: Mutex::new(None),
                thread_main: Mutex::new(None),
                security_negotiated: AtomicBool::new(false),
                previous_command: Mutex::new(VncMessageType::Unknown),
                command_count: Mutex::new(0),
                password_provided: VncEvent::default(),
                creating_desktop: VncEvent::default(),
                connected: VncEvent::default(),
                connection_failed: VncEvent::default(),
                closed: VncEvent::default(),
                framebuffer_capturing: VncEvent::default(),
                framebuffer_updating: VncEvent::default(),
                key_changed: VncEvent::default(),
                pointer_changed: VncEvent::default(),
                remote_clipboard_changed: VncEvent::default(),
                framebuffer: RwLock::new(None),
                framebuffer_update_request: Mutex::new(None),
                is_connected: AtomicBool::new(false),
                user_data: Mutex::new(None),
                create_framebuffer_cache: RwLock::new(create_framebuffer_cache),
                encoders: Mutex::new(vec![tight]),
                encoder: Mutex::new(raw),
                statistics: Mutex::new(HashMap::new()),
            }
        });
        session
    }

    fn arc(&self) -> Arc<VncServerSession> {
        self.this
            .upgrade()
            .expect("the session is alive while it is used")
    }

    /// Writes to the logger of the session, if it has one.
    fn log(&self, write: impl FnOnce(&dyn ILogger)) {
        if let Some(logger) = self.logger() {
            write(&*logger);
        }
    }

    /// Occurs when the VNC client provides a password. Respond to this
    /// event by accepting or rejecting the password.
    pub fn password_provided(&self) -> &VncEvent<PasswordProvidedEventArgs> {
        &self.password_provided
    }

    /// Occurs when the client requests access to the desktop. It may
    /// request exclusive or shared access; this event will relay that
    /// information.
    pub fn creating_desktop(&self) -> &VncEvent<CreatingDesktopEventArgs> {
        &self.creating_desktop
    }

    /// Occurs when the VNC client has successfully connected to the server.
    pub fn connected(&self) -> &VncEvent<()> {
        &self.connected
    }

    /// Occurs when the VNC client has failed to connect to the server.
    pub fn connection_failed(&self) -> &VncEvent<()> {
        &self.connection_failed
    }

    /// Occurs when the VNC client is disconnected.
    pub fn closed(&self) -> &VncEvent<()> {
        &self.closed
    }

    /// Occurs when the framebuffer needs to be captured. If you have not
    /// called [`set_framebuffer_source`](Self::set_framebuffer_source),
    /// alter the framebuffer in response to this event.
    ///
    /// [`framebuffer_update_request_lock`](Self::framebuffer_update_request_lock)
    /// is held automatically while this event is raised.
    pub fn framebuffer_capturing(&self) -> &VncEvent<()> {
        &self.framebuffer_capturing
    }

    /// Occurs when the framebuffer needs to be updated. If you do not set
    /// [`FramebufferUpdatingEventArgs::set_sent_changes`], the session will
    /// determine the updated regions itself.
    ///
    /// [`framebuffer_update_request_lock`](Self::framebuffer_update_request_lock)
    /// is held automatically while this event is raised.
    pub fn framebuffer_updating(&self) -> &VncEvent<FramebufferUpdatingEventArgs> {
        &self.framebuffer_updating
    }

    /// Occurs when a key has been pressed or released.
    pub fn key_changed(&self) -> &VncEvent<KeyChangedEventArgs> {
        &self.key_changed
    }

    /// Occurs on a mouse movement, button click, etc.
    pub fn pointer_changed(&self) -> &VncEvent<PointerChangedEventArgs> {
        &self.pointer_changed
    }

    /// Occurs when the clipboard changes on the remote client. If you are
    /// implementing clipboard integration, use this to set the local
    /// clipboard.
    pub fn remote_clipboard_changed(&self) -> &VncEvent<RemoteClipboardChangedEventArgs> {
        &self.remote_clipboard_changed
    }

    /// Gets the protocol version of the client.
    pub fn client_version(&self) -> Option<Version> {
        *lock(&self.client_version)
    }

    /// Gets the framebuffer for the VNC session.
    pub fn framebuffer(&self) -> Option<Arc<VncFramebuffer>> {
        read(&self.framebuffer)
    }

    fn set_framebuffer(&self, value: Option<Arc<VncFramebuffer>>) {
        write(&self.framebuffer, value);
    }

    /// Gets the update request the session is answering, if any.
    pub fn framebuffer_update_request(&self) -> Option<FramebufferUpdateRequest> {
        *lock(&self.framebuffer_update_request)
    }

    /// Sets the update request the session is answering.
    pub fn set_framebuffer_update_request(&self, value: Option<FramebufferUpdateRequest>) {
        *lock(&self.framebuffer_update_request) = value;
    }

    /// Gets the password challenge to use when authenticating clients.
    pub fn password_challenge(&self) -> Arc<dyn IVncPasswordChallenge> {
        read(&self.password_challenge)
    }

    /// Sets the password challenge to use when authenticating clients.
    ///
    /// # Panics
    /// Once the security has been negotiated (`InvalidOperationException`).
    pub fn set_password_challenge(&self, value: Arc<dyn IVncPasswordChallenge>) {
        if self.security_negotiated.load(Ordering::SeqCst) {
            panic!(
                "You cannot change the password challenge once the security has been negotiated"
            );
        }

        write(&self.password_challenge, value);
    }

    /// Gets the logger to use when logging.
    pub fn logger(&self) -> Option<Arc<dyn ILogger>> {
        read(&self.logger)
    }

    /// Sets the logger to use when logging.
    pub fn set_logger(&self, value: Option<Arc<dyn ILogger>>) {
        write(&self.logger, value);
    }

    /// Gets a lock which should be used before performing any framebuffer
    /// updates.
    pub fn framebuffer_update_request_lock(&self) -> &ReentrantLock {
        &self.fbu_sync
    }

    /// Gets a value indicating whether the server is connected to a client.
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    fn set_is_connected(&self, value: bool) {
        self.is_connected.store(value, Ordering::SeqCst);
    }

    /// Gets the max rate to send framebuffer updates at, in frames per
    /// second. The default is 15.
    pub fn max_update_rate(&self) -> f64 {
        *lock(&self.max_update_rate)
    }

    /// Sets the max rate to send framebuffer updates at, in frames per
    /// second.
    ///
    /// # Panics
    /// When the rate is not positive (`ArgumentOutOfRangeException`).
    pub fn set_max_update_rate(&self, value: f64) {
        if value <= 0.0 {
            panic!("Max update rate must be positive.");
        }

        *lock(&self.max_update_rate) = value;
    }

    /// Gets user-specific data: store anything you want here.
    pub fn user_data(&self) -> Option<Arc<dyn Any + Send + Sync>> {
        lock(&self.user_data).clone()
    }

    /// Sets user-specific data.
    pub fn set_user_data(&self, value: Option<Arc<dyn Any + Send + Sync>>) {
        *lock(&self.user_data) = value;
    }

    /// Gets the function which initializes a new framebuffer cache for use
    /// by this session.
    pub fn create_framebuffer_cache(&self) -> CreateFramebufferCache {
        read(&self.create_framebuffer_cache)
    }

    /// Sets the function which initializes a new framebuffer cache for use
    /// by this session.
    pub fn set_create_framebuffer_cache(&self, value: CreateFramebufferCache) {
        write(&self.create_framebuffer_cache, value);
    }

    /// Gets the list of encoders which this session can use (the Tight
    /// encoder, unless it is changed).
    pub fn encoders(&self) -> MutexGuard<'_, Vec<Arc<dyn VncEncoder>>> {
        lock(&self.encoders)
    }

    /// Gets the list of encodings supported by the client.
    pub fn client_encodings(&self) -> Vec<VncEncoding> {
        read(&self.client_encoding)
    }

    /// Gets the encoder which is currently in use (the raw encoder until
    /// the client has said which encodings it supports).
    pub fn encoder(&self) -> Arc<dyn VncEncoder> {
        lock(&self.encoder).clone()
    }

    /// Sets the encoder which is currently in use.
    pub fn set_encoder(&self, value: Arc<dyn VncEncoder>) {
        *lock(&self.encoder) = value;
    }

    /// Gets performance statistics for the various encoders used by this
    /// session.
    pub fn statistics(&self) -> HashMap<VncEncoding, EncoderStatistics> {
        lock(&self.statistics).clone()
    }

    /// Gets the pixel format currently used by the client.
    pub fn client_pixel_format(&self) -> VncPixelFormat {
        read(&self.client_pixel_format)
    }

    /// Sets the pixel format currently used by the client.
    pub fn set_client_pixel_format(&self, value: VncPixelFormat) {
        write(&self.client_pixel_format, value);
    }

    fn fb_source(&self) -> Option<Arc<dyn IVncFramebufferSource>> {
        read(&self.fb_source)
    }

    fn client_size(&self) -> (i32, i32) {
        (*lock(&self.client_width), *lock(&self.client_height))
    }

    fn set_client_size(&self, width: i32, height: i32) {
        *lock(&self.client_width) = width;
        *lock(&self.client_height) = height;
    }

    /// Closes the connection with the remote client and waits for the
    /// thread of the session to end.
    pub fn close(&self) {
        let thread = lock(&self.thread_main).take();
        self.c.close();
        if let Some(thread) = thread {
            if thread.thread().id() != thread::current().id() {
                let _ = thread.join();
            }
        }
    }

    /// Starts a session with a VNC client over `stream`, with `options` if
    /// any.
    pub fn connect(&self, stream: Arc<dyn VncTransport>, options: Option<VncServerSessionOptions>) {
        self.connect_with(stream, options, true, false);
    }

    /// Starts a session with a VNC client over `stream`, with `options` if
    /// any. `start_thread` says whether to start the thread of the session;
    /// `force_connected` whether to immediately set
    /// [`is_connected`](Self::is_connected), intended for unit tests only.
    pub fn connect_with(
        &self,
        stream: Arc<dyn VncTransport>,
        options: Option<VncServerSessionOptions>,
        start_thread: bool,
        force_connected: bool,
    ) {
        let _sync = self.c.sync_root().lock();
        self.close();

        *lock(&self.options) = options.unwrap_or_default();
        self.c.set_stream(Some(stream));

        if start_thread {
            let this = self.arc();
            let thread_main = thread::Builder::new()
                .name("VNC server session".to_owned())
                .spawn(move || this.thread_main())
                .expect("the thread of the session starts");
            *lock(&self.thread_main) = Some(thread_main);
        }

        if force_connected {
            self.set_is_connected(true);
        }
    }

    /// Tells the client to play a bell sound.
    pub fn bell(&self) {
        let _sync = self.c.sync_root().lock();
        if !self.is_connected() {
            return;
        }

        self.c.send_byte(2);
    }

    /// Notifies the client that the local clipboard has changed. If you are
    /// implementing clipboard integration, use this to set the remote
    /// clipboard.
    pub fn send_local_clipboard_change(&self, data: &str) {
        let _sync = self.c.sync_root().lock();
        if !self.is_connected() {
            return;
        }

        self.c.send_byte(3);
        self.c.send(&[0; 3]);
        self.c.send_string(data, true);
    }

    /// Sets the framebuffer source; `None` if you intend to handle the
    /// framebuffer manually.
    pub fn set_framebuffer_source(&self, source: Option<Arc<dyn IVncFramebufferSource>>) {
        write(&self.fb_source, source);
    }

    /// Notifies the framebuffer update thread to check for recent changes.
    pub fn framebuffer_changed(&self) {
        if let Some(requester) = lock(&self.requester).as_ref() {
            requester.signal();
        }
    }

    /// Begins a manual update of the framebuffer.
    pub fn framebuffer_manual_begin_update(&self) {
        lock(&self.fbu_rectangles).clear();
    }

    /// Queues an update corresponding to one region of the framebuffer
    /// being copied to another: `target` is the updated rectangle,
    /// `source_x` and `source_y` the position of the source.
    ///
    /// Do not call this method without holding
    /// [`framebuffer_update_request_lock`](Self::framebuffer_update_request_lock).
    pub fn framebuffer_manual_copy_region(
        &self,
        target: VncRectangle,
        source_x: i32,
        source_y: i32,
    ) {
        if !self.client_encodings().contains(&VncEncoding::CopyRect) {
            let source = VncRectangle::new(source_x, source_y, target.width, target.height);
            let region = VncRectangle::union(source, target);

            if region.area() > source.area() + target.area() {
                self.framebuffer_manual_invalidate_regions(&[source, target]);
            } else {
                self.framebuffer_manual_invalidate(region);
            }

            return;
        }

        let mut contents = vec![0u8; 4];
        VncUtility::encode_uint16_be_into(&mut contents, 0, source_x as u16);
        VncUtility::encode_uint16_be_into(&mut contents, 2, source_y as u16);
        self.add_region(target, VncEncoding::CopyRect, contents);
    }

    /// Queues an update for the entire framebuffer.
    pub fn framebuffer_manual_invalidate_all(&self) {
        let Some(framebuffer) = self.framebuffer() else {
            return;
        };

        self.framebuffer_manual_invalidate(VncRectangle::new(
            0,
            0,
            framebuffer.width(),
            framebuffer.height(),
        ));
    }

    /// Queues an update for the specified region.
    pub fn framebuffer_manual_invalidate(&self, region: VncRectangle) {
        let Some(fb) = self.framebuffer() else {
            return;
        };
        let cpf = self.client_pixel_format();
        let (client_width, client_height) = self.client_size();
        let region =
            VncRectangle::intersect(region, VncRectangle::new(0, 0, client_width, client_height));
        // Not in the original: the framebuffer may have become smaller than the client knows.
        let region =
            VncRectangle::intersect(region, VncRectangle::new(0, 0, fb.width(), fb.height()));
        if region.is_empty() {
            return;
        }

        let (w, h, bpp) = (region.width, region.height, cpf.bytes_per_pixel());
        let mut contents = vec![0u8; (w * h * bpp) as usize];

        {
            let buffer = fb.get_buffer();
            VncPixelFormat::copy(
                &buffer,
                fb.width(),
                fb.stride(),
                fb.pixel_format(),
                region,
                &mut contents,
                w,
                w * bpp,
                &cpf,
                0,
                0,
            );
        }

        self.add_region(region, VncEncoding::Raw, contents);
    }

    /// Queues an update for each of the specified regions.
    pub fn framebuffer_manual_invalidate_regions(&self, regions: &[VncRectangle]) {
        for region in regions {
            self.framebuffer_manual_invalidate(*region);
        }
    }

    /// Completes a manual update of the framebuffer: sends what was queued,
    /// with a desktop size change first when the framebuffer changed its
    /// size and the client understands that.
    ///
    /// Returns true if updates were sent.
    pub fn framebuffer_manual_end_update(&self) -> bool {
        let Some(fb) = self.framebuffer() else {
            return false;
        };
        let (client_width, client_height) = self.client_size();
        if (client_width != fb.width() || client_height != fb.height())
            && self
                .client_encodings()
                .contains(&VncEncoding::PseudoDesktopSize)
        {
            let region = VncRectangle::new(0, 0, fb.width(), fb.height());
            self.add_region(region, VncEncoding::PseudoDesktopSize, Vec::new());
            self.set_client_size(fb.width(), fb.height());
        }

        let rectangles = std::mem::take(&mut *lock(&self.fbu_rectangles));
        if rectangles.is_empty() {
            return false;
        }

        self.set_framebuffer_update_request(None);

        self.send_rectangles(&rectangles);
        true
    }

    /// Sends the framebuffer changes to the client.
    ///
    /// Returns true if the changes have been pushed correctly; otherwise,
    /// false.
    pub fn framebuffer_send_changes(&self) -> bool {
        let e = FramebufferUpdatingEventArgs::new();

        let _lock = self.framebuffer_update_request_lock().lock();
        if self.framebuffer_update_request().is_some() {
            if let Some(fb_source) = self.fb_source() {
                match catch_unwind(AssertUnwindSafe(|| fb_source.capture())) {
                    Ok(Some(new_framebuffer)) => {
                        let changed = self
                            .framebuffer()
                            .is_none_or(|current| !Arc::ptr_eq(&current, &new_framebuffer));
                        if changed {
                            self.set_framebuffer(Some(new_framebuffer));
                        }
                    }
                    Ok(None) => {}
                    Err(payload) => {
                        let message = panic_message(payload.as_ref());
                        self.log(|logger| {
                            logger.log_error(
                                EventId::new(2),
                                None,
                                &format!("Capturing the framebuffer source failed: {message}."),
                            )
                        });
                    }
                }
            }

            self.on_framebuffer_capturing();
            self.on_framebuffer_updating(&e);

            if !e.handled() {
                let Some(framebuffer) = self.framebuffer() else {
                    return false;
                };
                let cache = {
                    let mut auto_cache = lock(&self.fbu_auto_cache);
                    let stale = auto_cache
                        .as_ref()
                        .is_none_or(|cache| !Arc::ptr_eq(&cache.framebuffer(), &framebuffer));
                    if stale {
                        *auto_cache = Some((self.create_framebuffer_cache())(
                            framebuffer,
                            self.logger(),
                        ));
                    }
                    auto_cache.clone().expect("the cache was just created")
                };

                e.set_handled(true);
                e.set_sent_changes(cache.respond_to_update_request(self));
            }
        }

        e.sent_changes()
    }

    /// Reads the next client message and handles it.
    ///
    /// The session starts a main thread which reads and processes messages.
    /// You don't normally need to call this method yourself; but it can be
    /// useful when writing unit tests.
    pub fn handle_message(&self) -> Result<(), VncError> {
        let command = VncMessageType(self.c.receive_byte()?);

        {
            let mut previous_command = lock(&self.previous_command);
            let mut command_count = lock(&self.command_count);

            // If the same command is sent repeatedly (e.g. FramebufferUpdateRequest), suppress
            // repeated requests.
            if *previous_command != command || *command_count >= 25 {
                if *command_count > 0 {
                    let count = *command_count;
                    self.log(|logger| {
                        logger.log_information(&format!(
                            "Suppressed {count} notifications of the {command} command at the Info level."
                        ))
                    });
                }

                self.log(|logger| {
                    logger.log_information(&format!("Received the {command} command."))
                });
                *command_count = 0;
            } else {
                self.log(|logger| logger.log_debug(&format!("Received the {command} command")));
                *command_count += 1;
            }

            *previous_command = command;
        }

        match command {
            VncMessageType::SetPixelFormat => self.handle_set_pixel_format(),
            VncMessageType::SetEncodings => self.handle_set_encodings(),
            VncMessageType::FrameBufferUpdateRequest => self.handle_framebuffer_update_request(),
            VncMessageType::KeyEvent => self.handle_key_event(),
            VncMessageType::PointerEvent => self.handle_pointer_event(),
            VncMessageType::ClientCutText => self.handle_receive_clipboard_data(),
            VncMessageType::SetDesktopSize => self.handle_set_desktop_size(),
            _ => VncStream::require(
                false,
                "Unsupported command.",
                VncFailureReason::UnrecognizedProtocolElement,
            ),
        }
    }

    /// Negotiates the desktop: the client says whether it shares the
    /// desktop, the server sends the size, the pixel format and the name of
    /// the framebuffer.
    pub fn negotiate_desktop(&self) -> Result<(), VncError> {
        self.log(|logger| logger.log_information("Negotiating desktop settings"));

        let share_desktop_setting = self.c.receive_byte()?;
        let share_desktop = share_desktop_setting != 0;

        let e = CreatingDesktopEventArgs::new(share_desktop);
        self.on_creating_desktop(&e);

        let framebuffer = self.fb_source().and_then(|fb_source| fb_source.capture());
        self.set_framebuffer(framebuffer.clone());
        VncStream::require(
            framebuffer.is_some(),
            "No framebuffer. Make sure you've called SetFramebufferSource. It can be set to a VncFramebuffer.",
            VncFailureReason::SanityCheckFailed,
        )?;
        let framebuffer = framebuffer.expect("checked above");
        self.set_client_pixel_format(*framebuffer.pixel_format());
        self.set_client_size(framebuffer.width(), framebuffer.height());
        *lock(&self.fbu_auto_cache) = None;

        let _sync = self.c.sync_root().lock();
        self.c.send_uint16_be(framebuffer.width() as u16);
        self.c.send_uint16_be(framebuffer.height() as u16);
        let mut pixel_format = [0u8; VncPixelFormat::SIZE];
        framebuffer.pixel_format().encode(&mut pixel_format, 0);
        self.c.send(&pixel_format);
        self.c.send_string(framebuffer.name(), true);

        let (client_width, client_height) = self.client_size();
        let client_pixel_format = self.client_pixel_format();
        self.log(|logger| {
            logger.log_information(&format!(
                "The desktop {} has initialized with pixel format {client_pixel_format}; the screen size is {client_width}x{client_height}",
                framebuffer.name()
            ))
        });
        Ok(())
    }

    /// Negotiates the version of the RFB protocol used by server and
    /// client: `methods` receives the authentication methods supported by
    /// the server, which are none if the client and server failed to
    /// negotiate version 3.8 of the RFB protocol.
    ///
    /// Returns always true, once the version was read.
    pub(crate) fn negotiate_version(
        &self,
        methods: &mut Vec<AuthenticationMethod>,
    ) -> Result<bool, VncError> {
        self.log(|logger| logger.log_information("Negotiating the version."));

        self.c.send_version(Version::new(3, 8));

        let client_version = self.c.receive_version()?;
        *lock(&self.client_version) = Some(client_version);
        *methods = if client_version == Version::new(3, 8) {
            let method =
                if lock(&self.options).authentication_method == AuthenticationMethod::Password {
                    AuthenticationMethod::Password
                } else {
                    AuthenticationMethod::None
                };
            vec![method]
        } else {
            // Clients using any version of the RFB protocol other than 3.8 are not supported.
            // We'll let them know by sending an empty list of authentication methods, which will
            // cause `negotiate_security` to fail.
            Vec::new()
        };

        let supported_methods = format!(
            "Supported authentication methods are {}",
            methods
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        );

        self.log(|logger| {
            logger.log_information(&format!("The client version is {client_version}"))
        });
        self.log(|logger| logger.log_information(&supported_methods));

        Ok(true)
    }

    /// Negotiates the security mechanism used to authenticate the client,
    /// and authenticates the client; `methods` are the authentication
    /// methods supported by this server. See
    /// <https://github.com/rfbproto/rfbproto/blob/master/rfbproto.rst#712security>.
    ///
    /// Returns true if the client authenticated successfully; otherwise,
    /// false.
    pub(crate) fn negotiate_security(
        &self,
        methods: &[AuthenticationMethod],
    ) -> Result<bool, VncError> {
        self.log(|logger| logger.log_information("Negotiating security"));

        self.c.send_byte(methods.len() as u8);

        if methods.is_empty() {
            self.log(|logger| {
                logger.log_warning(
                    "The server and client could not agree on any authentication method.",
                )
            });
            self.c.send_string(
                "The server and client could not agree on any authentication method.",
                true,
            );
            return Ok(false);
        }

        for method in methods {
            self.c.send_byte(*method as u8);
        }

        let selected_method = AuthenticationMethod::from_value(self.c.receive_byte()?);
        let Some(selected_method) = selected_method.filter(|method| methods.contains(method))
        else {
            self.c.send_uint32_be(1);
            self.log(|logger| logger.log_information("Invalid authentication method."));
            self.c.send_string("Invalid authentication method.", true);
            return Ok(false);
        };

        let mut success = true;
        if selected_method == AuthenticationMethod::Password {
            let password_challenge = self.password_challenge();
            let mut challenge = password_challenge.generate_challenge();
            let challenge = AutoClear::new(&mut challenge);
            self.c.send(challenge.array());

            let mut response = self.c.receive(16)?;
            let response = AutoClear::new(&mut response);
            let e = PasswordProvidedEventArgs::new(
                password_challenge,
                challenge.array(),
                response.array(),
            );
            self.on_password_provided(&e);
            success = e.is_authenticated();
        }

        self.c.send_uint32_be(if success { 0 } else { 1 });

        if !success {
            self.log(|logger| logger.log_information("The user failed to authenticate."));
            self.c.send_string("Failed to authenticate", true);
            return Ok(false);
        }

        self.log(|logger| logger.log_information("The user authenticated successfully."));
        self.security_negotiated.store(true, Ordering::SeqCst);

        Ok(true)
    }

    pub(crate) fn handle_set_encodings(&self) -> Result<(), VncError> {
        self.c.receive(1)?;

        let encoding_count = self.c.receive_uint16_be()?;
        VncStream::sanity_check(encoding_count <= 0x1ff)?;
        let mut client_encoding = Vec::with_capacity(usize::from(encoding_count));

        self.log(|logger| {
            logger.log_information(&format!("The client supports {encoding_count} encodings:"))
        });

        for _ in 0..encoding_count {
            let encoding = VncEncoding(self.c.receive_uint32_be()? as i32);
            client_encoding.push(encoding);

            self.log(|logger| logger.log_information(&format!("- {encoding}")));
        }

        write(&self.client_encoding, client_encoding);
        self.init_framebuffer_encoder();
        Ok(())
    }

    /// Raises the password provided event.
    fn on_password_provided(&self, e: &PasswordProvidedEventArgs) {
        self.password_provided.invoke(self, e);
    }

    /// Raises the creating desktop event.
    fn on_creating_desktop(&self, e: &CreatingDesktopEventArgs) {
        self.creating_desktop.invoke(self, e);
    }

    /// Raises the connected event.
    fn on_connected(&self) {
        self.connected.invoke(self, &());
    }

    /// Raises the connection failed event.
    fn on_connection_failed(&self) {
        self.connection_failed.invoke(self, &());
    }

    /// Raises the closed event, and logs the statistics of the encoders.
    fn on_closed(&self) {
        self.closed.invoke(self, &());

        let statistics = self.statistics();
        self.log(|logger| {
            logger.log_information("Encoder     Rectangles  Raw         Encoded     Ratio");

            for (encoding, stat) in &statistics {
                let ratio = 100.0 - (stat.encoded_bytes as f64 / stat.raw_bytes as f64 * 100.0);
                let ratio = format!("{ratio:.1}");
                let ratio = ratio.strip_suffix(".0").unwrap_or(&ratio);

                logger.log_information(&format!(
                    "{:<12}{:<12}{:<12}{:<12}{ratio}%",
                    encoding.to_string(),
                    stat.rectangles,
                    stat.raw_bytes,
                    stat.encoded_bytes
                ));
            }
        });
    }

    /// Raises the framebuffer capturing event.
    fn on_framebuffer_capturing(&self) {
        self.framebuffer_capturing.invoke(self, &());
    }

    /// Raises the framebuffer updating event.
    fn on_framebuffer_updating(&self, e: &FramebufferUpdatingEventArgs) {
        self.framebuffer_updating.invoke(self, e);
    }

    /// Raises the key changed event.
    fn on_key_changed(&self, e: &KeyChangedEventArgs) {
        self.key_changed.invoke(self, e);
    }

    /// Raises the pointer changed event.
    fn on_pointer_changed(&self, e: &PointerChangedEventArgs) {
        self.pointer_changed.invoke(self, e);
    }

    /// Raises the remote clipboard changed event.
    fn on_remote_clipboard_changed(&self, e: &RemoteClipboardChangedEventArgs) {
        self.remote_clipboard_changed.invoke(self, e);
    }

    /// Sends a framebuffer update message of `rectangles`. A rectangle of
    /// raw pixels is sent with the encoder in use; the others as they are.
    fn send_rectangles(&self, rectangles: &[Rectangle]) {
        let _sync = self.c.sync_root().lock();
        self.c.send(&[0, 0]);
        self.c.send_uint16_be(rectangles.len() as u16);

        for rectangle in rectangles {
            if rectangle.encoding != VncEncoding::Raw {
                self.c.send_rectangle(rectangle.region);
                self.c.send_uint32_be(rectangle.encoding.0 as u32);
                self.c.send(&rectangle.contents);

                self.record_encoder_transfer(
                    rectangle.encoding,
                    rectangle.contents.len(),
                    rectangle.contents.len(),
                );
            } else {
                let encoder = self.encoder();
                self.c.send_rectangle(rectangle.region);
                self.c.send_uint32_be(encoder.encoding().0 as u32);

                let Some(stream) = self.c.stream() else {
                    return;
                };
                let client_pixel_format = self.client_pixel_format();
                match encoder.send(
                    &mut TransportWriter(&*stream),
                    &client_pixel_format,
                    rectangle.region,
                    &rectangle.contents,
                ) {
                    Ok(sent) => self.record_encoder_transfer(
                        encoder.encoding(),
                        rectangle.contents.len(),
                        sent,
                    ),
                    Err(error) => {
                        // The original lets the exception end the thread of the updates; the
                        // connection is lost and the thread of the session ends the session.
                        self.log(|logger| {
                            logger.log_error(
                                EventId::new(3),
                                Some(&error),
                                &format!("Sending a rectangle failed: {error}"),
                            )
                        });
                        return;
                    }
                }
            }
        }
    }

    fn record_encoder_transfer(
        &self,
        encoding: VncEncoding,
        raw_length: usize,
        encoded_length: usize,
    ) {
        let mut statistics = lock(&self.statistics);
        let statistic = statistics.entry(encoding).or_default();
        statistic.encoded_bytes += encoded_length as u64;
        statistic.raw_bytes += raw_length as u64;
        statistic.rectangles += 1;
    }

    fn thread_main(self: Arc<Self>) {
        let requester = Arc::new(PeriodicThread::new());
        *lock(&self.requester) = Some(requester.clone());

        let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), VncError> {
            self.init_framebuffer_encoder();

            let mut methods = Vec::new();

            if self.negotiate_version(&mut methods)? && self.negotiate_security(&methods)? {
                self.negotiate_desktop()?;

                let weak = Arc::downgrade(&self);
                let rate = Arc::downgrade(&self);
                requester.start(
                    move || {
                        if let Some(session) = weak.upgrade() {
                            session.framebuffer_send_changes();
                        }
                    },
                    move || {
                        rate.upgrade()
                            .map_or(15.0, |session| session.max_update_rate())
                    },
                    false,
                );

                self.set_is_connected(true);
                self.log(|logger| logger.log_information("The client has connected successfully"));

                self.on_connected();

                loop {
                    self.handle_message()?;
                }
            }

            Ok(())
        }));

        let failure = match result {
            Ok(Ok(())) => None,
            Ok(Err(exception)) => Some(exception.message().to_owned()),
            Err(payload) => Some(panic_message(payload.as_ref())),
        };
        if let Some(message) = failure {
            self.log(|logger| {
                logger.log_error(
                    EventId::new(1),
                    None,
                    &format!("VNC server session stopped due to: {message}"),
                )
            });
        }

        requester.stop();

        self.c.set_stream(None);
        if self.is_connected() {
            self.set_is_connected(false);
            self.on_closed();
        } else {
            self.on_connection_failed();
        }
    }

    fn handle_set_pixel_format(&self) -> Result<(), VncError> {
        self.c.receive(3)?;

        let pixel_format = self.c.receive(VncPixelFormat::SIZE)?;
        self.set_client_pixel_format(VncPixelFormat::decode(&pixel_format, 0)?);

        self.init_framebuffer_encoder();
        Ok(())
    }

    /// The extended desktop size rectangle: one screen of the size of the
    /// framebuffer, the reason in the x-position and the status in the
    /// y-position. See
    /// <https://github.com/rfbproto/rfbproto/blob/master/rfbproto.rst#extendeddesktopsize-pseudo-encoding>.
    fn get_extended_desktop_size_rectangle(
        &self,
        reason: ExtendedDesktopSizeReason,
        status: ExtendedDesktopSizeStatus,
    ) -> Option<Rectangle> {
        // The server must send an ExtendedDesktopSize rectangle in response to a
        // FramebufferUpdateRequest with incremental set to zero, assuming the client has
        // requested the ExtendedDesktopSize pseudo-encoding using the SetEncodings message.
        let framebuffer = self.framebuffer()?;
        let mut encoding_data = vec![0u8; 20];

        // Header
        encoding_data[0] = 1; // number of screens

        // SCREEN array
        VncUtility::encode_uint32_be_into(&mut encoding_data, 4, 0); // Screen id
        VncUtility::encode_uint16_be_into(&mut encoding_data, 8, 0); // X-position
        VncUtility::encode_uint16_be_into(&mut encoding_data, 10, 0); // Y-position
        VncUtility::encode_uint16_be_into(&mut encoding_data, 12, framebuffer.width() as u16); // Width
        VncUtility::encode_uint16_be_into(&mut encoding_data, 14, framebuffer.height() as u16); // Height
        VncUtility::encode_uint32_be_into(&mut encoding_data, 16, 0); // Flags. Currently unused.

        Some(Rectangle {
            region: VncRectangle {
                x: reason as i32,
                y: status as i32,
                width: framebuffer.width(),
                height: framebuffer.height(),
            },
            contents: encoding_data,
            encoding: VncEncoding::ExtendedDesktopSize,
        })
    }

    fn handle_framebuffer_update_request(&self) -> Result<(), VncError> {
        let incremental = self.c.receive_byte()? != 0;
        let region = self.c.receive_rectangle()?;

        if !incremental
            && self
                .client_encodings()
                .contains(&VncEncoding::ExtendedDesktopSize)
            && self
                .fb_source()
                .is_some_and(|fb_source| fb_source.supports_resizing())
        {
            let rectangles: Vec<Rectangle> = self
                .get_extended_desktop_size_rectangle(
                    ExtendedDesktopSizeReason::External,
                    ExtendedDesktopSizeStatus::Success,
                )
                .into_iter()
                .collect();
            self.send_rectangles(&rectangles);
        }

        let _lock = self.framebuffer_update_request_lock().lock();
        self.log(|logger| {
            logger.log_debug(&format!(
                "Received a FramebufferUpdateRequest command for {region}"
            ))
        });

        let (width, height) = self.framebuffer().map_or((0, 0), |framebuffer| {
            (framebuffer.width(), framebuffer.height())
        });
        let region = VncRectangle::intersect(region, VncRectangle::new(0, 0, width, height));

        if region.is_empty() {
            return Ok(());
        }

        self.set_framebuffer_update_request(Some(FramebufferUpdateRequest::new(
            incremental,
            region,
        )));
        self.framebuffer_changed();
        Ok(())
    }

    fn handle_key_event(&self) -> Result<(), VncError> {
        let pressed = self.c.receive_byte()? != 0;
        self.c.receive(2)?;
        let keysym = KeySym(self.c.receive_uint32_be()?);

        self.on_key_changed(&KeyChangedEventArgs::new(keysym, pressed));
        Ok(())
    }

    fn handle_pointer_event(&self) -> Result<(), VncError> {
        let pressed_buttons = i32::from(self.c.receive_byte()?);
        let x = i32::from(self.c.receive_uint16_be()?);
        let y = i32::from(self.c.receive_uint16_be()?);

        self.on_pointer_changed(&PointerChangedEventArgs::new(x, y, pressed_buttons));
        Ok(())
    }

    fn handle_receive_clipboard_data(&self) -> Result<(), VncError> {
        self.c.receive(3)?; // padding

        let clipboard = self.c.receive_string(0xffffff)?;

        self.on_remote_clipboard_changed(&RemoteClipboardChangedEventArgs::new(clipboard));
        Ok(())
    }

    fn handle_set_desktop_size(&self) -> Result<(), VncError> {
        let message = self.c.receive(7)?;
        let width = VncUtility::decode_uint16_be(&message, 1);
        let height = VncUtility::decode_uint16_be(&message, 3);
        let number_of_screens = usize::from(message[5]);

        let _screens = self.c.receive(16 * number_of_screens)?;

        let result = match self.fb_source() {
            Some(fb_source) => fb_source.set_desktop_size(i32::from(width), i32::from(height)),
            None => ExtendedDesktopSizeStatus::Prohibited,
        };

        let rectangles: Vec<Rectangle> = self
            .get_extended_desktop_size_rectangle(ExtendedDesktopSizeReason::Client, result)
            .into_iter()
            .collect();
        self.send_rectangles(&rectangles);
        Ok(())
    }

    fn add_region(&self, region: VncRectangle, encoding: VncEncoding, contents: Vec<u8>) {
        let count = {
            let mut rectangles = lock(&self.fbu_rectangles);
            rectangles.push(Rectangle {
                region,
                encoding,
                contents,
            });
            rectangles.len()
        };

        // Avoid the overflow of updated rectangle count.
        // NOTE: EndUpdate may implicitly add one for desktop resizing.
        if count >= usize::from(u16::MAX) - 1 {
            self.framebuffer_manual_end_update();
            self.framebuffer_manual_begin_update();
        }
    }

    fn init_framebuffer_encoder(&self) {
        self.log(|logger| logger.log_information("Initializing the frame buffer encoder"));

        // For RealVNC compatibility: make sure we only start using encodings _after_ the
        // connection has been initialized correctly.
        if self.is_connected() {
            // Iterate over all encodings supported by the client. Select the first encoding
            // which is supported by this session.
            let encoders = self.encoders().clone();
            for encoding in self.client_encodings() {
                if let Some(encoder) = encoders
                    .iter()
                    .find(|encoder| encoder.encoding() == encoding)
                {
                    self.set_encoder(encoder.clone());
                    break;
                }
            }
        } else {
            self.log(|logger| {
                logger.log_information("Not selecting any encoder because the connection setup phase has not completed.")
            });
        }

        let encoder = self.encoder();
        self.log(|logger| {
            logger.log_information(&format!(
                "Initialized the frame buffer encoder. Using {} ({})",
                encoder.type_name(),
                encoder.encoding()
            ))
        });
    }
}

impl IVncServerSession for VncServerSession {
    fn key_changed(&self) -> &VncEvent<KeyChangedEventArgs> {
        &self.key_changed
    }

    fn pointer_changed(&self) -> &VncEvent<PointerChangedEventArgs> {
        &self.pointer_changed
    }

    fn connected(&self) -> &VncEvent<()> {
        &self.connected
    }

    fn connection_failed(&self) -> &VncEvent<()> {
        &self.connection_failed
    }

    fn closed(&self) -> &VncEvent<()> {
        &self.closed
    }

    fn password_provided(&self) -> &VncEvent<PasswordProvidedEventArgs> {
        &self.password_provided
    }

    fn logger(&self) -> Option<Arc<dyn ILogger>> {
        VncServerSession::logger(self)
    }

    fn set_logger(&self, value: Option<Arc<dyn ILogger>>) {
        VncServerSession::set_logger(self, value);
    }

    fn password_challenge(&self) -> Arc<dyn IVncPasswordChallenge> {
        VncServerSession::password_challenge(self)
    }

    fn set_password_challenge(&self, value: Arc<dyn IVncPasswordChallenge>) {
        VncServerSession::set_password_challenge(self, value);
    }

    fn max_update_rate(&self) -> f64 {
        VncServerSession::max_update_rate(self)
    }

    fn set_max_update_rate(&self, value: f64) {
        VncServerSession::set_max_update_rate(self, value);
    }

    fn create_framebuffer_cache(&self) -> CreateFramebufferCache {
        VncServerSession::create_framebuffer_cache(self)
    }

    fn set_create_framebuffer_cache(&self, value: CreateFramebufferCache) {
        VncServerSession::set_create_framebuffer_cache(self, value);
    }

    fn framebuffer_update_request(&self) -> Option<FramebufferUpdateRequest> {
        VncServerSession::framebuffer_update_request(self)
    }

    fn framebuffer_update_request_lock(&self) -> &ReentrantLock {
        VncServerSession::framebuffer_update_request_lock(self)
    }

    fn client_encodings(&self) -> Vec<VncEncoding> {
        VncServerSession::client_encodings(self)
    }

    fn connect(&self, stream: Arc<dyn VncTransport>, options: Option<VncServerSessionOptions>) {
        VncServerSession::connect(self, stream, options);
    }

    fn close(&self) {
        VncServerSession::close(self);
    }

    fn set_framebuffer_source(&self, source: Option<Arc<dyn IVncFramebufferSource>>) {
        VncServerSession::set_framebuffer_source(self, source);
    }

    fn framebuffer_manual_begin_update(&self) {
        VncServerSession::framebuffer_manual_begin_update(self);
    }

    fn framebuffer_manual_invalidate(&self, region: VncRectangle) {
        VncServerSession::framebuffer_manual_invalidate(self, region);
    }

    fn framebuffer_manual_invalidate_regions(&self, regions: &[VncRectangle]) {
        VncServerSession::framebuffer_manual_invalidate_regions(self, regions);
    }

    fn framebuffer_manual_invalidate_all(&self) {
        VncServerSession::framebuffer_manual_invalidate_all(self);
    }

    fn framebuffer_manual_end_update(&self) -> bool {
        VncServerSession::framebuffer_manual_end_update(self)
    }
}

#[cfg(all(test, unix))]
mod tests {
    // Not from upstream: the protocol of a session, driven over a socket pair.
    use super::*;
    use std::io::Read;
    use std::os::unix::net::UnixStream;
    use std::sync::mpsc;
    use std::time::Duration;

    fn read_exact(stream: &mut UnixStream, count: usize) -> Vec<u8> {
        let mut buffer = vec![0u8; count];
        stream.read_exact(&mut buffer).unwrap();
        buffer
    }

    /// Connects a client to a session whose source is `framebuffer`, up to
    /// the server init message. Returns the client end.
    fn handshake(
        session: &Arc<VncServerSession>,
        options: Option<VncServerSessionOptions>,
    ) -> UnixStream {
        let (server, mut client) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        session.connect(Arc::new(server), options);

        assert_eq!(b"RFB 003.008\n".to_vec(), read_exact(&mut client, 12));
        client.write_all(b"RFB 003.008\n").unwrap();
        client
    }

    #[test]
    fn a_client_without_authentication_gets_the_framebuffer() {
        let framebuffer = VncFramebuffer::new("test", 4, 2, VncPixelFormat::RGB32);
        framebuffer.set_pixel(1, 0, 0x0011_2233);
        let session = VncServerSession::new();
        session.set_framebuffer_source(Some(framebuffer.clone()));
        let (connected_sender, connected) = mpsc::channel();
        session
            .connected()
            .add(move |_, _| connected_sender.send(()).unwrap());

        let mut client = handshake(&session, None);
        assert_eq!(
            vec![1, 1],
            read_exact(&mut client, 2),
            "one security type: none"
        );
        client.write_all(&[1]).unwrap();
        assert_eq!(
            vec![0, 0, 0, 0],
            read_exact(&mut client, 4),
            "security result: OK"
        );

        client.write_all(&[1]).unwrap(); // shared
        let init = read_exact(&mut client, 2 + 2 + 16 + 4);
        assert_eq!(&[0, 4, 0, 2], &init[..4]);
        assert_eq!(&[32, 24, 0, 1], &init[4..8]);
        assert_eq!(&[0, 0, 0, 4], &init[20..24]);
        assert_eq!(b"test".to_vec(), read_exact(&mut client, 4));
        connected.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(session.is_connected());

        // A full update of the framebuffer, raw.
        client.write_all(&[2, 0, 0, 1, 0, 0, 0, 0]).unwrap();
        client.write_all(&[3, 0, 0, 0, 0, 0, 0, 4, 0, 2]).unwrap();
        let header = read_exact(&mut client, 4);
        assert_eq!(vec![0, 0, 0, 1], header);
        let rectangle = read_exact(&mut client, 12);
        assert_eq!(vec![0, 0, 0, 0, 0, 4, 0, 2, 0, 0, 0, 0], rectangle);
        let pixels = read_exact(&mut client, 4 * 2 * 4);
        assert_eq!(&[0x33, 0x22, 0x11, 0x00], &pixels[4..8]);
        assert_eq!(*framebuffer.get_buffer(), pixels);

        // An incremental update sends only what changed: the second line.
        framebuffer.set_pixel(2, 1, 0x00ff_ffff);
        client.write_all(&[3, 1, 0, 0, 0, 0, 0, 4, 0, 2]).unwrap();
        assert_eq!(vec![0, 0, 0, 1], read_exact(&mut client, 4));
        assert_eq!(
            vec![0, 0, 0, 1, 0, 4, 0, 1, 0, 0, 0, 0],
            read_exact(&mut client, 12)
        );
        let line = read_exact(&mut client, 16);
        assert_eq!(&[0xff, 0xff, 0xff, 0x00], &line[8..12]);

        drop(client);
        session.close();
        assert!(!session.is_connected());
        assert_eq!(2, session.statistics()[&VncEncoding::Raw].rectangles);
    }

    #[test]
    fn a_client_with_the_password_is_accepted_and_one_without_it_is_not() {
        for (password, accepted) in [("secret", true), ("wrong", false)] {
            let session = VncServerSession::new();
            session.set_framebuffer_source(Some(VncFramebuffer::new(
                "test",
                1,
                1,
                VncPixelFormat::RGB32,
            )));
            session.password_provided().add(|_, e| {
                e.accept_chars(&['s', 'e', 'c', 'r', 'e', 't']);
            });
            let options = VncServerSessionOptions {
                authentication_method: AuthenticationMethod::Password,
            };

            let mut client = handshake(&session, Some(options));
            assert_eq!(
                vec![1, 2],
                read_exact(&mut client, 2),
                "one security type: VNC authentication"
            );
            client.write_all(&[2]).unwrap();
            let challenge = read_exact(&mut client, 16);
            let mut response = [0u8; 16];
            VncPasswordChallenge::new().get_challenge_response(
                &challenge,
                password.as_bytes(),
                &mut response,
            );
            client.write_all(&response).unwrap();

            let result = read_exact(&mut client, 4);
            assert_eq!(
                if accepted {
                    vec![0, 0, 0, 0]
                } else {
                    vec![0, 0, 0, 1]
                },
                result
            );
            drop(client);
            session.close();
        }
    }

    #[test]
    fn another_protocol_version_is_refused() {
        let session = VncServerSession::new();
        session.set_framebuffer_source(Some(VncFramebuffer::new(
            "test",
            1,
            1,
            VncPixelFormat::RGB32,
        )));
        let (failed_sender, failed) = mpsc::channel();
        session
            .connection_failed()
            .add(move |_, _| failed_sender.send(()).unwrap());

        let (server, mut client) = UnixStream::pair().unwrap();
        session.connect(Arc::new(server), None);
        read_exact(&mut client, 12);
        client.write_all(b"RFB 003.003\n").unwrap();
        assert_eq!(vec![0], read_exact(&mut client, 1), "no security types");
        let length = read_exact(&mut client, 4);
        let reason = read_exact(&mut client, length[3] as usize);
        assert_eq!(
            b"The server and client could not agree on any authentication method.".to_vec(),
            reason
        );

        failed.recv_timeout(Duration::from_secs(10)).unwrap();
        session.close();
    }

    #[test]
    fn key_pointer_and_clipboard_messages_raise_the_events() {
        let session = VncServerSession::new();
        session.set_framebuffer_source(Some(VncFramebuffer::new(
            "test",
            1,
            1,
            VncPixelFormat::RGB32,
        )));
        let (sender, events) = mpsc::channel::<String>();
        let keys = sender.clone();
        session.key_changed().add(move |_, e| {
            keys.send(format!("key {:x} {}", e.keysym().0, e.pressed()))
                .unwrap()
        });
        let pointers = sender.clone();
        session.pointer_changed().add(move |_, e| {
            pointers
                .send(format!(
                    "pointer {} {} {}",
                    e.x(),
                    e.y(),
                    e.pressed_buttons()
                ))
                .unwrap()
        });
        session
            .remote_clipboard_changed()
            .add(move |_, e| sender.send(format!("clipboard {}", e.contents())).unwrap());

        let mut client = handshake(&session, None);
        read_exact(&mut client, 2);
        client.write_all(&[1]).unwrap();
        read_exact(&mut client, 4);
        client.write_all(&[1]).unwrap();
        read_exact(&mut client, 24 + 4);

        client.write_all(&[4, 1, 0, 0, 0, 0, 0, 0x61]).unwrap();
        client.write_all(&[5, 1, 0, 10, 0, 20]).unwrap();
        client
            .write_all(&[6, 0, 0, 0, 0, 0, 0, 2, b'h', b'i'])
            .unwrap();

        let timeout = Duration::from_secs(10);
        assert_eq!("key 61 true", events.recv_timeout(timeout).unwrap());
        assert_eq!("pointer 10 20 1", events.recv_timeout(timeout).unwrap());
        assert_eq!("clipboard hi", events.recv_timeout(timeout).unwrap());

        drop(client);
        session.close();
    }
}
