use super::*;
use crate::frames::*;
use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};
struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}
fn poll<T>(future: &mut Pin<Box<dyn Future<Output = T>>>) -> Poll<T> {
    future
        .as_mut()
        .poll(&mut Context::from_waker(&Waker::from(Arc::new(Noop))))
}
struct Renderer {
    completions: RefCell<Vec<GpuCompletion>>,
    budget: FrameBudget,
}
impl TextureRenderer for Renderer {
    fn render(
        &self,
        v: RendererViewport,
        _: Vec<FrameLease>,
        completion: GpuCompletion,
    ) -> MediaFuture<FrameLease> {
        self.completions.borrow_mut().push(completion.clone());
        let bytes = v.width as u64 * v.height as u64 * 4;
        let frame = self
            .budget
            .lease(
                FrameMetadata {
                    identity: FrameIdentity {
                        source: 9,
                        generation: v.generation,
                        sequence: 1,
                    },
                    width: v.width,
                    height: v.height,
                    visible: FrameRegion {
                        x: 0,
                        y: 0,
                        width: v.width,
                        height: v.height,
                    },
                    format: FrameFormat::Bgra8,
                    color: FrameColor::Srgb,
                    alpha: FrameAlpha::Premultiplied,
                    rotation: FrameRotation::None,
                    mirrored: false,
                    top_left_origin: true,
                    timestamp: Duration::ZERO,
                    device: Some(DeviceIdentity::Context(1)),
                    device_generation: 1,
                    planes: vec![FramePlane {
                        offset: 0,
                        stride: v.width as u64 * 4,
                        length: bytes,
                    }],
                    retained_bytes: bytes,
                },
                (),
            )
            .unwrap();
        completion.retain(frame.clone()).unwrap();
        Box::pin(async move { Ok(frame) })
    }
}
#[test]
fn cancellation_retains_output_until_gpu_completion_and_invalidated_work_retries() {
    let budget = FrameBudget::new(Default::default());
    let renderer = Rc::new(Renderer {
        completions: RefCell::new(Vec::new()),
        budget: budget.clone(),
    });
    let host = TextureRendererHost::new(renderer.clone(), 9);
    host.resize(2., 2., 1., 8192).unwrap();
    host.set_active(true);
    let mut pending = host.render(vec![]).unwrap();
    assert!(poll(&mut pending).is_pending());
    assert!(host.output().latest().is_none());
    drop(pending);
    assert_eq!(budget.usage().frames, 1);
    renderer.completions.borrow()[0].complete();
    assert_eq!(budget.usage().frames, 0);
    let mut retried = host.render(vec![]).expect("retry invalidation");
    assert!(poll(&mut retried).is_pending());
    renderer.completions.borrow()[1].complete();
    assert!(matches!(poll(&mut retried), Poll::Ready(Ok(()))));
    assert!(host.output().latest().is_some());
    assert!(host.render(vec![]).is_none());
}
#[test]
fn zero_size_hidden_and_points_behind_the_view_plane_are_rejected() {
    let renderer = Rc::new(Renderer {
        completions: RefCell::new(Vec::new()),
        budget: FrameBudget::new(Default::default()),
    });
    let host = TextureRendererHost::new(renderer, 9);
    host.set_active(true);
    assert!(host.render(vec![]).is_none());
    host.resize(2., 2., 1., 8192).unwrap();
    host.set_active(false);
    assert!(host.render(vec![]).is_none());
    let mut matrix = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 2.,
    ];
    assert_eq!(project_texture_point(&matrix, 0.5, 1.), Some((0.25, 0.5)));
    matrix[15] = -1.;
    assert_eq!(project_texture_point(&matrix, 0., 0.), None);
}

#[test]
fn saturated_reads_coalesce_without_calling_the_renderer_or_leaking_reservations() {
    let budget = FrameBudget::new(FrameBudgetLimits {
        gpu_reads: 2,
        ..Default::default()
    });
    let renderer = Rc::new(Renderer {
        completions: RefCell::new(Vec::new()),
        budget: budget.clone(),
    });
    let viewport = RendererViewport {
        width: 2,
        height: 2,
        generation: 1,
    };
    let mut frames = Vec::new();
    for _ in 0..2 {
        let completed = GpuCompletion::new();
        let mut pending = renderer.render(viewport, vec![], completed.clone());
        let Poll::Ready(Ok(frame)) = poll(&mut pending) else {
            panic!("fixture ready")
        };
        completed.complete();
        frames.push(frame);
    }
    let other_consumer = GpuCompletion::new();
    other_consumer.retain(frames[0].clone()).unwrap();
    let host = TextureRendererHost::new(renderer.clone(), 9);
    host.resize(2., 2., 1., 8192).unwrap();
    host.set_active(true);
    // The first input fits; the second must roll back the first reservation.
    assert!(host.render(frames.clone()).is_none());
    assert_eq!(renderer.completions.borrow().len(), 2);
    assert_eq!(budget.gpu_reads(), 1);
    other_consumer.complete();
    drop(frames);
    let mut retry = host.render(vec![]).expect("invalidation retained");
    assert!(poll(&mut retry).is_pending());
    renderer.completions.borrow()[2].complete();
    assert!(matches!(poll(&mut retry), Poll::Ready(Ok(()))));
}

#[test]
fn detach_and_reattach_rejects_the_previous_submitted_output() {
    let renderer = Rc::new(Renderer {
        completions: RefCell::new(Vec::new()),
        budget: FrameBudget::new(Default::default()),
    });
    let host = TextureRendererHost::new(renderer.clone(), 9);
    host.resize(2., 2., 1., 8192).unwrap();
    host.set_active(true);
    let mut pending = host.render(vec![]).unwrap();
    assert!(poll(&mut pending).is_pending());
    let old = host.viewport();
    host.set_active(false);
    host.set_active(true);
    assert!(host.viewport().generation > old.generation);
    renderer.completions.borrow()[0].complete();
    assert!(matches!(
        poll(&mut pending),
        Poll::Ready(Err(MediaError {
            kind: MediaErrorKind::Cancelled,
            ..
        }))
    ));
    assert!(host.output().latest().is_none());
    let mut retry = host.render(vec![]).unwrap();
    assert!(poll(&mut retry).is_pending());
    renderer.completions.borrow()[1].complete();
    assert!(matches!(poll(&mut retry), Poll::Ready(Ok(()))));
    assert_eq!(
        host.output()
            .latest()
            .unwrap()
            .metadata()
            .identity
            .generation,
        host.viewport().generation
    );
}

#[test]
fn replacing_input_bindings_rejects_a_late_render_even_if_old_input_is_still_valid() {
    let renderer = Rc::new(Renderer {
        completions: RefCell::new(Vec::new()),
        budget: FrameBudget::new(Default::default()),
    });
    let host = TextureRendererHost::new(renderer.clone(), 9);
    host.resize(2., 2., 1., 8192).unwrap();
    host.set_active(true);
    let mut pending = host.render(vec![]).unwrap();
    assert!(poll(&mut pending).is_pending());
    host.clear_output();
    renderer.completions.borrow()[0].complete();
    assert!(matches!(
        poll(&mut pending),
        Poll::Ready(Err(MediaError {
            kind: MediaErrorKind::Cancelled,
            ..
        }))
    ));
    assert!(host.output().latest().is_none());
}
