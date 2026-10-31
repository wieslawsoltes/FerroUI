//! Camera ownership and cancellation contracts, without acquiring hardware.
use super::*;
use std::task::{Context, Poll, Wake, Waker};
struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}
fn ready<T>(mut future: CameraFuture<T>) -> Result<T, CameraError> {
    match future
        .as_mut()
        .poll(&mut Context::from_waker(&Waker::from(Arc::new(Noop))))
    {
        Poll::Ready(r) => r,
        Poll::Pending => panic!("expected ready"),
    }
}
struct Native {
    stops: Cell<u32>,
}
impl NativeCameraSession for Native {
    fn settings(&self) -> CameraSettings {
        CameraSettings {
            device: CameraDevice {
                id: "fixture".into(),
                label: None,
                facing: CameraFacing::Unknown,
                formats: vec![],
            },
            format: CameraFormat {
                width: 640,
                height: 480,
                frames_per_second: 30.,
            },
            native_still: false,
            torch: false,
            zoom: None,
        }
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, CameraError> {
        Ok(None)
    }
    fn photo(&self) -> CameraFuture<CameraPhoto> {
        Box::pin(async {
            Err(CameraError::new(
                CameraErrorKind::Unsupported,
                "No photo fixture",
            ))
        })
    }
    fn set_control(&self, _: CameraControl) -> Result<(), CameraError> {
        Err(CameraError::new(
            CameraErrorKind::Unsupported,
            "No native control",
        ))
    }
    fn stop(&self) {
        self.stops.set(self.stops.get() + 1);
    }
}
struct ProvisionalStart {
    native: Option<Rc<dyn NativeCameraSession>>,
}
impl std::future::Future for ProvisionalStart {
    type Output = Result<Rc<dyn NativeCameraSession>, CameraError>;
    fn poll(mut self: std::pin::Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(Ok(self.native.take().unwrap()))
    }
}
impl Drop for ProvisionalStart {
    fn drop(&mut self) {
        if let Some(native) = self.native.take() {
            native.stop();
        }
    }
}
struct Provider {
    native: Rc<Native>,
    starts: Cell<u32>,
}
impl CameraProvider for Provider {
    fn devices(&self) -> CameraFuture<Vec<CameraDevice>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn permission(&self) -> CameraPermission {
        CameraPermission::NotDetermined
    }
    fn start(
        &self,
        _: CameraStartOptions,
        _: u64,
        _: u64,
        _: FrameBudget,
    ) -> CameraFuture<Rc<dyn NativeCameraSession>> {
        self.starts.set(self.starts.get() + 1);
        let native: Rc<dyn NativeCameraSession> = self.native.clone();
        Box::pin(ProvisionalStart {
            native: Some(native),
        })
    }
}
#[test]
fn discovery_never_starts_capture_and_shared_readers_retire_independently() {
    let native = Rc::new(Native {
        stops: Cell::new(0),
    });
    let provider = Rc::new(Provider {
        native: native.clone(),
        starts: Cell::new(0),
    });
    let capture = CameraCapture::new(provider.clone(), FrameBudget::new(Default::default()));
    assert!(ready(capture.devices()).unwrap().is_empty());
    assert_eq!(capture.permission(), CameraPermission::NotDetermined);
    assert_eq!(provider.starts.get(), 0);
    let session = ready(capture.start(Default::default())).unwrap();
    let other = session.clone();
    drop(session);
    assert_eq!(native.stops.get(), 0);
    assert_eq!(provider.starts.get(), 1);
    other.stop();
    other.stop();
    drop(other);
    assert_eq!(native.stops.get(), 1);
}
#[test]
fn a_preview_weak_session_does_not_own_capture() {
    let native = Rc::new(Native {
        stops: Cell::new(0),
    });
    let provider = Rc::new(Provider {
        native: native.clone(),
        starts: Cell::new(0),
    });
    let capture = CameraCapture::new(provider, FrameBudget::new(Default::default()));
    let session = ready(capture.start(Default::default())).unwrap();
    let view = session.downgrade();
    assert!(view.upgrade().is_some_and(|s| !s.is_stopped()));
    drop(session);
    assert!(view.upgrade().is_none());
    assert_eq!(native.stops.get(), 1);
}
#[test]
fn canceled_start_stops_a_late_native_session() {
    let native = Rc::new(Native {
        stops: Cell::new(0),
    });
    let provider = Rc::new(Provider {
        native: native.clone(),
        starts: Cell::new(0),
    });
    let capture = CameraCapture::new(provider, FrameBudget::new(Default::default()));
    let pending = capture.start(Default::default());
    capture.cancel_pending_start();
    assert!(matches!(
        ready(pending),
        Err(CameraError {
            kind: CameraErrorKind::Cancelled,
            ..
        })
    ));
    assert_eq!(native.stops.get(), 1);
}

#[test]
fn abandoning_an_unpolled_start_retires_the_provisional_owner() {
    let native = Rc::new(Native {
        stops: Cell::new(0),
    });
    let provider = Rc::new(Provider {
        native: native.clone(),
        starts: Cell::new(0),
    });
    let capture = CameraCapture::new(provider, FrameBudget::new(Default::default()));
    let pending = capture.start(Default::default());
    drop(pending);
    assert_eq!(native.stops.get(), 1);
}

#[test]
fn malformed_requests_never_reach_a_provider_or_native_controls() {
    let native = Rc::new(Native {
        stops: Cell::new(0),
    });
    let provider = Rc::new(Provider {
        native,
        starts: Cell::new(0),
    });
    let capture = CameraCapture::new(provider.clone(), FrameBudget::new(Default::default()));
    let mut options = CameraStartOptions::default();
    options.selection.format = Some(CameraFormat {
        width: 0,
        height: 720,
        frames_per_second: 30.,
    });
    assert_eq!(
        ready(capture.start(options)).err().unwrap().kind,
        CameraErrorKind::InvalidRequest
    );
    assert_eq!(provider.starts.get(), 0);
    let session = ready(capture.start(Default::default())).unwrap();
    for control in [
        CameraControl::Zoom(f64::NAN),
        CameraControl::Focus(-0.1, 0.5),
        CameraControl::Exposure(f64::INFINITY),
    ] {
        assert_eq!(
            session.set_control(control).unwrap_err().kind,
            CameraErrorKind::InvalidRequest
        );
    }
    assert_eq!(
        session
            .set_control(CameraControl::Zoom(1.))
            .unwrap_err()
            .kind,
        CameraErrorKind::Unsupported
    );
}

#[test]
fn a_late_photo_is_canceled_but_completed_encoded_output_is_independent() {
    struct PhotoNative(Native);
    impl NativeCameraSession for PhotoNative {
        fn settings(&self) -> CameraSettings {
            self.0.settings()
        }
        fn take_frame(&self) -> Result<Option<FrameLease>, CameraError> {
            Ok(None)
        }
        fn photo(&self) -> CameraFuture<CameraPhoto> {
            // Opaque ownership fixture; native JPEG validity has its own gate.
            Box::pin(async {
                Ok(CameraPhoto {
                    encoded: Arc::from(&b"owned photo fixture"[..]),
                    content_type: "application/x-test-photo".into(),
                    width: 1,
                    height: 1,
                    timestamp: Duration::ZERO,
                    rotation_degrees: 0,
                    mirrored: false,
                    origin: PhotoOrigin::NativeStill,
                })
            })
        }
        fn set_control(&self, control: CameraControl) -> Result<(), CameraError> {
            self.0.set_control(control)
        }
        fn stop(&self) {
            self.0.stop();
        }
    }
    let session = || {
        CameraSession(Rc::new(SessionInner {
            native: Rc::new(PhotoNative(Native {
                stops: Cell::new(0),
            })),
            frames: Arc::new(FrameMailbox::new(1, 1)),
            stopped: Cell::new(false),
        }))
    };
    let first = session();
    let pending = first.photo();
    first.stop();
    assert_eq!(ready(pending).unwrap_err().kind, CameraErrorKind::Cancelled);
    let second = session();
    let encoded = ready(second.photo()).unwrap().encoded;
    second.stop();
    drop(second);
    assert_eq!(&*encoded, b"owned photo fixture");
}
