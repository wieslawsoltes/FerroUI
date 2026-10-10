//! The thread that owns the windows and the device contexts of OpenGL (the
//! port of `OpenGl/WglGdiResourceManager.cs`).
//!
//! - `ReleaseDC` can only happen from the same thread that has called
//!   `GetDC`
//! - When a thread exits all of its windows and device contexts are
//!   destroyed
//! - Contexts (which need a window and a device context) and render targets
//!   (which need a device context) are created by the thread that renders
//!
//! So this file hosts a dedicated thread for managing offscreen windows and
//! device contexts for OpenGL.
//!
//! The reference queues its jobs under a lock and wakes the thread with an
//! event, and makes the thread a single-threaded apartment so that the
//! wait of its runtime pumps the messages of the windows. Here the jobs
//! travel through a channel, the thread is woken by a message posted to
//! its queue, and it dispatches the messages of its windows itself.

use crate::interop::unmanaged_methods::{
    create_offscreen_gl_window, def_window_proc, destroy_window, dispatch_message, get_current_thread_id, get_dc,
    hwnd_to_isize, msg_wait_for_multiple_objects_ex, peek_message, post_thread_message, register_class_ex, release_dc,
};
use crate::interop::unmanaged_methods::{ClassStyles, MsgWaitForMultipleObjectsFlags, QueueStatusFlags, WindowsMessage};
use std::ffi::c_void;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

enum Op {
    GetDc { window: isize, result: Sender<isize> },
    ReleaseDc { window: isize, dc: isize, result: Sender<()> },
    CreateWindow { result: Sender<isize> },
    DestroyWindow { window: isize, result: Sender<()> },
}

struct Manager {
    queue: Mutex<Sender<Op>>,
    thread_id: u32,
}

static MANAGER: OnceLock<Manager> = OnceLock::new();

/// The window procedure of the offscreen windows: the default processing.
unsafe extern "system" fn wnd_proc(hwnd: *mut c_void, msg: u32, w_param: usize, l_param: isize) -> isize {
    def_window_proc(hwnd_to_isize(hwnd), msg, w_param, l_param)
}

fn worker(queue: Receiver<Op>, ready: Sender<u32>) {
    // The name of the class is one of the process (the reference appends a
    // new GUID).
    let class_name = format!("FerroGlWindow-{}", std::process::id());
    let window_class = register_class_ex(&class_name, ClassStyles::CS_OWNDC.bits(), wnd_proc, 0);
    // The first look at the queue of the thread creates it, so that a
    // message can be posted to the thread from now on.
    let _ = peek_message();
    if ready.send(get_current_thread_id()).is_err() {
        return;
    }

    loop {
        msg_wait_for_multiple_objects_ex(
            u32::MAX,
            QueueStatusFlags::QS_ALLINPUT,
            MsgWaitForMultipleObjectsFlags::MWMO_INPUTAVAILABLE,
        );
        while let Some(msg) = peek_message() {
            dispatch_message(&msg);
        }
        while let Ok(job) = queue.try_recv() {
            // A caller that went away has no use for the result.
            match job {
                Op::GetDc { window, result } => {
                    let _ = result.send(get_dc(window));
                }
                Op::ReleaseDc { window, dc, result } => {
                    release_dc(window, dc);
                    let _ = result.send(());
                }
                Op::CreateWindow { result } => {
                    let _ = result.send(create_offscreen_gl_window(window_class));
                }
                Op::DestroyWindow { window, result } => {
                    destroy_window(window);
                    let _ = result.send(());
                }
            }
        }
    }
}

fn manager() -> &'static Manager {
    MANAGER.get_or_init(|| {
        let (queue, jobs) = channel();
        let (ready, started) = channel();
        std::thread::Builder::new()
            .name("Win32 OpenGL HDC manager".to_owned())
            .spawn(move || worker(jobs, ready))
            .expect("the thread of the OpenGL device contexts starts");
        let thread_id = started.recv().expect("the thread of the OpenGL device contexts runs");
        Manager { queue: Mutex::new(queue), thread_id }
    })
}

fn run<T>(job: impl FnOnce(Sender<T>) -> Op) -> T {
    let manager = manager();
    let (result, done) = channel();
    manager
        .queue
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .send(job(result))
        .expect("the thread of the OpenGL device contexts lives as long as the process");
    post_thread_message(manager.thread_id, WindowsMessage::WM_NULL, 0, 0);
    done.recv().expect("the thread of the OpenGL device contexts answers")
}

pub(crate) struct WglGdiResourceManager;

impl WglGdiResourceManager {
    /// A window that is never shown; 0 if it could not be created.
    pub fn create_offscreen_window() -> isize {
        run(|result| Op::CreateWindow { result })
    }

    /// The device context of a window, owned by the thread of the manager.
    pub fn get_dc(hwnd: isize) -> isize {
        run(|result| Op::GetDc { window: hwnd, result })
    }

    pub fn release_dc(hwnd: isize, dc: isize) {
        run(|result| Op::ReleaseDc { window: hwnd, dc, result })
    }

    pub fn destroy_window(hwnd: isize) {
        run(|result| Op::DestroyWindow { window: hwnd, result })
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream. Against the system.
    use super::*;
    use crate::interop::unmanaged_methods::is_window;

    #[test]
    fn the_thread_of_the_manager_owns_windows_and_device_contexts() {
        let window = WglGdiResourceManager::create_offscreen_window();
        assert_ne!(0, window);
        assert!(is_window(window));

        let dc = WglGdiResourceManager::get_dc(window);
        assert_ne!(0, dc);
        // The class has a device context of its own per window: asked
        // again, it is the same one.
        let again = WglGdiResourceManager::get_dc(window);
        assert_eq!(dc, again);

        WglGdiResourceManager::release_dc(window, dc);
        WglGdiResourceManager::destroy_window(window);
        assert!(!is_window(window));
    }

    #[test]
    fn jobs_of_several_threads_are_answered() {
        let threads: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(|| {
                    for _ in 0..8 {
                        let window = WglGdiResourceManager::create_offscreen_window();
                        assert_ne!(0, window);
                        WglGdiResourceManager::destroy_window(window);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().expect("the thread ends");
        }
    }
}
