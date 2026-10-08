//! The render-thread mode of the compositor: the server compositor runs on
//! the thread that ticks the render loop. Not from upstream, where the two
//! threads are a property of the platform set-up and not of a test.

use super::Compositor;
use crate::media::MediaContext;
use crate::rendering::testing::{ManualRenderLoop, MockPlatformRenderInterface};
use crate::threading::Dispatcher;
use std::cell::Cell;
use std::rc::Rc;
use std::thread;

/// Ticks the loop once on a thread of its own, which has a render interface
/// as the render thread of an application has.
fn tick_on_another_thread(render_loop: &std::sync::Arc<ManualRenderLoop>, ticks: usize) -> thread::ThreadId {
    let render_loop = render_loop.clone();
    thread::spawn(move || {
        let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
        for _ in 0..ticks {
            render_loop.tick();
        }
        thread::current().id()
    })
    .join()
    .expect("the render thread ran")
}

#[test]
fn the_server_runs_on_the_thread_that_ticks_the_loop() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let render_loop = ManualRenderLoop::new();
    let compositor = Compositor::with_render_thread(
        render_loop.clone(),
        || None,
        &MediaContext::instance().scheduler(),
        Dispatcher::ui_thread(),
        None,
        None,
    );
    assert!(compositor.renders_on_render_thread());
    let ui_thread = thread::current().id();

    // Objects are created and changed on this thread; their server objects
    // are created by the batch, on the other one.
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

    let render_thread = tick_on_another_thread(&render_loop, 1);
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
fn the_server_compositor_is_released_on_its_thread() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    let render_loop = ManualRenderLoop::new();
    let compositor = Compositor::with_render_thread(
        render_loop.clone(),
        || None,
        &MediaContext::instance().scheduler(),
        Dispatcher::ui_thread(),
        None,
        None,
    );
    let dropped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = dropped.clone();
    compositor.post_server_job(
        move |_| {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        },
        false,
    );
    Dispatcher::ui_thread().run_jobs(None);

    // One render thread for the life of the server compositor: the tick
    // that creates it and the tick that releases it.
    let render_loop_for_thread = render_loop.clone();
    let (to_thread, from_ui) = std::sync::mpsc::channel::<()>();
    let (to_ui, from_thread) = std::sync::mpsc::channel::<()>();
    let handle = thread::spawn(move || {
        let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
        render_loop_for_thread.tick();
        to_ui.send(()).unwrap();
        from_ui.recv().unwrap();
        render_loop_for_thread.tick();
    });
    from_thread.recv().unwrap();
    assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));

    drop(compositor);
    to_thread.send(()).unwrap();
    handle.join().expect("the render thread released the server compositor");
}

#[test]
fn a_synchronous_commit_waits_for_the_render_thread() {
    let _dispatcher_scope = Dispatcher::unit_test_scope();
    // A synchronous commit is skipped without a render interface (unit tests
    // that set up no platform): this thread has one, as the other does.
    let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
    let render_loop = ManualRenderLoop::background();
    let compositor = Compositor::with_render_thread(
        render_loop.clone(),
        || None,
        &MediaContext::instance().scheduler(),
        Dispatcher::ui_thread(),
        None,
        None,
    );

    // The render thread ticks until it is told to stop.
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (thread_stop, thread_loop) = (stop.clone(), render_loop.clone());
    let render_thread = thread::spawn(move || {
        let (_locator_scope, _render_interface) = MockPlatformRenderInterface::install();
        while !thread_stop.load(std::sync::atomic::Ordering::SeqCst) {
            thread_loop.tick();
            thread::sleep(std::time::Duration::from_millis(1));
        }
    });

    let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = ran.clone();
    compositor.post_server_job(move |_| flag.store(true, std::sync::atomic::Ordering::SeqCst), false);

    // Returns once the render thread has applied the batch and rendered.
    MediaContext::instance().immediate_render_requested(&compositor);
    assert!(ran.load(std::sync::atomic::Ordering::SeqCst));

    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    render_thread.join().expect("the render thread ended");
}
