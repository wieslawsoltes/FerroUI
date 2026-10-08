//! The render-thread mode of the compositor: the thread that ticks the
//! render loop renders, under the compositor lock, and the thread of the
//! compositor renders too where a platform asks for it. Not from upstream,
//! where the two threads are a property of the platform set-up and not of
//! a test.

use super::Compositor;
use crate::media::MediaContext;
use crate::rendering::testing::{ManualRenderLoop, MockPlatformRenderInterface};
use crate::threading::Dispatcher;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn render_thread_compositor(
    render_loop: &Arc<ManualRenderLoop>,
    use_ui_thread_for_synchronous_commits: bool,
) -> Rc<Compositor> {
    Compositor::with_render_thread(
        render_loop.clone(),
        None,
        use_ui_thread_for_synchronous_commits,
        &MediaContext::instance().scheduler(),
        Dispatcher::ui_thread(),
        None,
        None,
    )
}

/// A render thread that ticks the loop until it is stopped.
struct RenderThread {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl RenderThread {
    fn start(render_loop: &Arc<ManualRenderLoop>) -> RenderThread {
        let stop = Arc::new(AtomicBool::new(false));
        let (thread_stop, thread_loop) = (stop.clone(), render_loop.clone());
        let handle = thread::spawn(move || {
            while !thread_stop.load(Ordering::SeqCst) {
                thread_loop.tick();
                thread::yield_now();
            }
        });
        RenderThread { stop, handle: Some(handle) }
    }
}

impl Drop for RenderThread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[test]
fn the_thread_that_ticks_the_loop_renders() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
    let render_loop = ManualRenderLoop::new();
    let compositor = render_thread_compositor(&render_loop, false);
    assert!(compositor.renders_on_render_thread());
    let ui_thread = thread::current().id();

    // Objects are created and changed on this thread; their server objects
    // are created by the batch, by the thread that renders.
    let parent = compositor.create_container_visual();
    let child = compositor.create_solid_color_visual();
    parent.children().add((*child).clone());

    let task = compositor
        .invoke_server_job_async(move |server| Ok((thread::current().id(), server.object_count())), false);
    let continued = Rc::new(Cell::new(false));
    let flag = continued.clone();
    task.on_completed(move || flag.set(true));

    // The media context commits the batch on this thread.
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!task.is_completed());

    let tick_loop = render_loop.clone();
    let render_thread = thread::spawn(move || {
        tick_loop.tick();
        thread::current().id()
    })
    .join()
    .expect("the render thread ran");
    assert_ne!(ui_thread, render_thread);

    // The job ran on the render thread and its result is here.
    assert!(task.is_completed_successfully());
    let (job_thread, object_count) = task.take_result().expect("completed").expect("the job succeeded");
    assert_eq!(render_thread, job_thread);
    assert!(object_count >= 2, "the server objects of the two visuals exist: {object_count}");

    // The continuation belongs to this thread: it runs from the dispatcher.
    assert!(!continued.get());
    Dispatcher::ui_thread().run_jobs(None);
    assert!(continued.get());
}

#[test]
fn a_synchronous_commit_waits_for_the_render_thread() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
    let render_loop = ManualRenderLoop::background();
    let compositor = render_thread_compositor(&render_loop, false);
    let ui_thread = thread::current().id();
    let _render_thread = RenderThread::start(&render_loop);

    let ran_on = Arc::new(std::sync::Mutex::new(None));
    let slot = ran_on.clone();
    compositor.post_server_job(move |_| *slot.lock().unwrap() = Some(thread::current().id()), false);

    // Returns once the render thread has applied the batch and rendered.
    MediaContext::instance().immediate_render_requested(&compositor);
    let ran_on = ran_on.lock().unwrap().expect("the job ran before the commit returned");
    assert_ne!(ui_thread, ran_on);
}

#[test]
fn a_synchronous_commit_renders_on_the_ui_thread_where_the_platform_asks_for_it() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
    let render_loop = ManualRenderLoop::background();
    // `UseUiThreadForSynchronousCommits`: what the native platform asks for.
    let compositor = render_thread_compositor(&render_loop, true);
    let ui_thread = thread::current().id();

    let ran_on = Arc::new(std::sync::Mutex::new(None));
    let slot = ran_on.clone();
    compositor.post_server_job(move |_| *slot.lock().unwrap() = Some(thread::current().id()), false);

    // No render thread is running: this thread renders the frame itself.
    MediaContext::instance().immediate_render_requested(&compositor);
    assert_eq!(Some(ui_thread), *ran_on.lock().unwrap());
}

#[test]
fn both_threads_render_one_after_the_other() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
    let render_loop = ManualRenderLoop::background();
    let compositor = render_thread_compositor(&render_loop, true);
    let _render_thread = RenderThread::start(&render_loop);

    // Every job checks that no other job is inside the server at the same
    // time: the frames of the two threads exclude each other.
    let inside = Arc::new(AtomicUsize::new(0));
    let overlaps = Arc::new(AtomicUsize::new(0));
    let ran = Arc::new(AtomicUsize::new(0));
    const ROUNDS: usize = 200;
    for _ in 0..ROUNDS {
        let (inside, overlaps, ran) = (inside.clone(), overlaps.clone(), ran.clone());
        compositor.post_server_job(
            move |_| {
                if inside.fetch_add(1, Ordering::SeqCst) != 0 {
                    overlaps.fetch_add(1, Ordering::SeqCst);
                }
                thread::yield_now();
                inside.fetch_sub(1, Ordering::SeqCst);
                ran.fetch_add(1, Ordering::SeqCst);
            },
            false,
        );
        // Commits and renders on this thread while the render thread ticks.
        MediaContext::instance().immediate_render_requested(&compositor);
    }

    assert_eq!(ROUNDS, ran.load(Ordering::SeqCst));
    assert_eq!(0, overlaps.load(Ordering::SeqCst));
}
