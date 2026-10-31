use super::{Ordering, FAILED};
use ferroui_base::{
    layout::{Orientation, VerticalAlignment},
    reactive::IDisposable,
    rendering::composition::{frames::*, ElementComposition},
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    Ref, WeakRef,
};
use ferroui_controls::{
    automation::AutomationProperties, Button, Panel, StackPanel, TextBlock, Window,
};
use ferroui_media_camera::{CameraCapture, CameraSession};
use ferroui_media_camera_apple::{AppleCameraProvider, AppleTextureImporter};
use ferroui_media_controls::TexturePresenter;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
struct State {
    capture: RefCell<Option<CameraCapture>>,
    session: RefCell<Option<CameraSession>>,
    presenter: WeakRef<TexturePresenter>,
    status: WeakRef<TextBlock>,
    timer: RefCell<Option<Rc<dyn IDisposable>>>,
    generation: Cell<u64>,
    seen_attached: Cell<bool>,
    initializing: Cell<bool>,
}
impl State {
    fn status(&self, text: &str) {
        if let Some(label) = self.status.upgrade() {
            label.set_text(Some(text));
        }
    }
    fn stop(&self) {
        self.generation.set(self.generation.get() + 1);
        if let Some(capture) = self.capture.borrow().as_ref() {
            capture.cancel_pending_start();
        }
        if let Some(session) = self.session.borrow_mut().take() {
            session.stop();
        }
        if let Some(presenter) = self.presenter.upgrade() {
            presenter.set_frame_source(None, None);
        }
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.stop();
        if let Some(timer) = self.timer.get_mut().take() {
            timer.dispose();
        }
    }
}
pub fn create(window: &Ref<Window>) -> Ref<Panel> {
    let root = Panel::new();
    let presenter = TexturePresenter::new();
    let actions = StackPanel::new();
    actions.set_orientation(Orientation::Horizontal);
    actions.set_spacing(8.);
    actions.set_vertical_alignment(VerticalAlignment::Bottom);
    let status = TextBlock::new();
    status.set_text(Some("Discovering cameras; hardware is not acquired"));
    let button = |label: &str| {
        let b = Button::new();
        b.set_content(Some(Rc::new(label.to_owned())));
        AutomationProperties::set_name(&b, Some(label));
        b
    };
    let start = button("Start camera");
    let stop = button("Stop camera");
    let photo = button("Capture JPEG");
    actions.children().add(&start);
    actions.children().add(&stop);
    actions.children().add(&photo);
    actions.children().add(&status);
    root.children().add(&presenter);
    root.children().add(&actions);
    let state = Rc::new(State {
        capture: RefCell::new(None),
        session: RefCell::new(None),
        presenter: presenter.downgrade(),
        status: status.downgrade(),
        timer: RefCell::new(None),
        generation: Cell::new(0),
        seen_attached: Cell::new(false),
        initializing: Cell::new(false),
    });
    let controller = state.clone();
    start.click(move |_, _| {
        let capture = controller.capture.borrow().clone();
        let Some(capture) = capture else {
            return;
        };
        controller.stop();
        controller.status("Waiting for camera permission and acquisition");
        let generation = controller.generation.get();
        let pending = capture.start(Default::default());
        let weak = Rc::downgrade(&controller);
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            match pending.await {
                Ok(session) => {
                    let Some(state) = weak.upgrade().filter(|s| s.generation.get() == generation)
                    else {
                        session.stop();
                        return;
                    };
                    let settings = session.settings();
                    state.status(&format!(
                        "{}×{} at {} fps",
                        settings.format.width,
                        settings.format.height,
                        settings.format.frames_per_second
                    ));
                    if let Some(presenter) = state.presenter.upgrade() {
                        presenter.set_frame_source(
                            Some(session.frames()),
                            Some(Rc::new(AppleTextureImporter)),
                        );
                    }
                    *state.session.borrow_mut() = Some(session);
                }
                Err(e) => {
                    if let Some(state) = weak.upgrade().filter(|s| s.generation.get() == generation)
                    {
                        state.status(&e.to_string());
                    }
                }
            }
        });
    });
    let controller = state.clone();
    stop.click(move |_, _| {
        controller.stop();
        controller.status("Camera stopped");
    });
    let controller = state.clone();
    photo.click(move |_, _| {
        let pending = controller
            .session
            .borrow()
            .as_ref()
            .map(|session| session.photo());
        let Some(pending) = pending else {
            return;
        };
        let weak = Rc::downgrade(&controller);
        let generation = controller.generation.get();
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let result = pending.await;
            let Some(state) = weak.upgrade().filter(|s| s.generation.get() == generation) else {
                return;
            };
            match result {
                Ok(photo) => {
                    // The sample validates an independently owned JPEG without
                    // writing private camera output to disk.
                    match image::load_from_memory(&photo.encoded) {
                        Ok(image)
                            if image.width() == photo.width && image.height() == photo.height =>
                        {
                            state.status(&format!(
                                "Captured {}×{} JPEG, {} bytes ({:?})",
                                photo.width,
                                photo.height,
                                photo.encoded.len(),
                                photo.origin
                            ))
                        }
                        _ => state.status("Captured JPEG failed independent decode"),
                    }
                }
                Err(e) => state.status(&e.to_string()),
            }
        });
    });
    let weak = Rc::downgrade(&state);
    let weak_window = window.downgrade();
    *state.timer.borrow_mut() = Some(DispatcherTimer::run(
        move || {
            let Some(state) = weak.upgrade() else {
                return false;
            };
            let Some(presenter) = state.presenter.upgrade() else {
                return false;
            };
            if !presenter.is_attached_to_visual_tree() {
                if state.seen_attached.get() {
                    state.stop();
                    return false;
                }
                return true;
            }
            state.seen_attached.set(true);
            if state.capture.borrow().is_none() && !state.initializing.get() {
                let Some(visual) = ElementComposition::get_element_visual(&presenter) else {
                    return true;
                };
                // Mark initialization in progress without starting a camera.
                let compositor = visual.compositor().clone();
                let weak = weak.clone();
                let weak_window = weak_window.clone();
                state.initializing.set(true);
                Dispatcher::ui_thread().invoke_async_task_local(move||async move{
                let Some(interop)=compositor.try_get_composition_gpu_interop().await else{return;};
                let Some(luid)=interop.device_luid() else{return;};
                let Some(state)=weak.upgrade() else{return;};
                let capture=CameraCapture::new(Rc::new(AppleCameraProvider{device:DeviceIdentity::Luid(luid),device_generation:interop.shared_context_generation()}),FrameBudget::new(Default::default()));
                let before=capture.permission();let devices=capture.devices().await;let after=capture.permission();
                match devices {
                    Ok(devices)=>{
                        state.status(&format!("{} camera(s); permission {after:?}; press Start",devices.len()));
                        if std::env::args().any(|a|a=="--camera-discovery"){
                            if before!=after{FAILED.store(true,Ordering::Relaxed);}
                            println!("CAMERA_DISCOVERY_PASS devices={} permission_before={before:?} permission_after={after:?} acquired=false",devices.len());
                            if let Some(window)=weak_window.upgrade(){window.close();}
                        }
                    }
                    Err(e)=>{state.status(&e.to_string());FAILED.store(true,Ordering::Relaxed);if std::env::args().any(|a|a=="--camera-discovery"){if let Some(window)=weak_window.upgrade(){window.close();}}}
                }
                *state.capture.borrow_mut()=Some(capture);
            });
            }
            let session = state.session.borrow().clone();
            if let Some(session) = session {
                if let Err(e) = session.poll() {
                    state.stop();
                    state.status(&e.to_string());
                } else {
                    presenter.present_latest();
                }
            }
            true
        },
        Duration::from_millis(33),
        DispatcherPriority::NORMAL,
    ));
    root
}
