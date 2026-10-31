//! Client-owned rendering example. No scene/engine is installed in the framework.
use ferroui_base::{media::Stretch, Point, Size};
use ferroui_media::{frames::*, MediaError, MediaErrorKind};
#[cfg(target_os = "android")]
use std::sync::Arc;
pub const PLANE: [f64; 16] = [
    1.4, 0., 0., 0.35, -0.25, -1.35, 0., 0.15, 0., 0., 1., 0., -0.55, 0.7, 0., 1.,
];
fn error(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::NativeFailure, message)
}
fn uv(frame: Option<&FrameLease>) -> Result<[f32; 12], MediaError> {
    let Some(frame) = frame else {
        return Ok([0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0.]);
    };
    let m = frame.metadata();
    if !matches!(m.color, FrameColor::Srgb | FrameColor::Rec709Full)
        || m.alpha != FrameAlpha::Opaque
    {
        return Err(error("Example admits opaque SDR inputs only"));
    }
    let layout = ferroui_media_controls::frame_layout(m, Size::new(1., 1.), Stretch::Fill)
        .ok_or_else(|| error("Input layout unavailable"))?;
    let inverse = layout.transform.invert();
    let map = |x, y| {
        let p = inverse.transform(Point::new(x, y));
        let u = p.x / m.width as f64;
        let v = p.y / m.height as f64;
        (u as f32, if m.top_left_origin { v } else { 1. - v } as f32)
    };
    let a = map(0., 0.);
    let b = map(1., 0.);
    let c = map(0., 1.);
    Ok([
        a.0,
        a.1,
        0.,
        0.,
        b.0 - a.0,
        b.1 - a.1,
        0.,
        0.,
        c.0 - a.0,
        c.1 - a.1,
        0.,
        0.,
    ])
}
fn count_color(frame: Option<&FrameLease>) {
    if let Some(frame) = frame.filter(|v| v.metadata().color == FrameColor::Rec709Full) {
        frame
            .counters()
            .gpu_conversion(GpuTransferReason::ColorSpaceConversion);
    }
}
#[cfg(target_vendor = "apple")]
mod metal {
    use super::*;
    use ferroui_media::RendererViewport;
    use ferroui_media_metal::{MetalRenderContext, MetalTextureRender};
    use std::{ffi::c_void, ptr::NonNull, sync::Mutex};
    extern "C" {
        fn portable_media_plane_create(device: *mut c_void) -> *mut c_void;
        fn portable_media_plane_encode(
            p: *mut c_void,
            commands: *mut c_void,
            output: *mut c_void,
            input: *mut c_void,
            parameters: *const c_void,
            length: usize,
        ) -> i32;
        fn portable_media_plane_release(p: *mut c_void);
    }
    struct Pipeline(NonNull<c_void>);
    // Immutable Metal pipeline state supports concurrent ownership/release.
    unsafe impl Send for Pipeline {}
    unsafe impl Sync for Pipeline {}
    impl Drop for Pipeline {
        fn drop(&mut self) {
            unsafe { portable_media_plane_release(self.0.as_ptr()) }
        }
    }
    #[derive(Default)]
    pub struct Scene(Mutex<Option<Pipeline>>);
    #[repr(C, align(16))]
    struct Params {
        projection: [f32; 16],
        uv: [f32; 12],
        textured: i32,
        rec709: i32,
        padding: [i32; 2],
    }
    impl MetalTextureRender for Scene {
        fn encode(
            &self,
            context: &MetalRenderContext<'_>,
            _: RendererViewport,
            inputs: &[FrameLease],
        ) -> Result<(), MediaError> {
            let mut pipeline = self.0.lock().unwrap();
            if pipeline.is_none() {
                *pipeline = Some(Pipeline(
                    NonNull::new(unsafe { portable_media_plane_create(context.device()) })
                        .ok_or_else(|| error("Metal sample pipeline unavailable"))?,
                ));
            }
            let input = inputs.first();
            let view = input.map(|v| context.input_texture(v)).transpose()?;
            let params = Params {
                projection: PLANE.map(|v| v as f32),
                uv: uv(input)?,
                textured: i32::from(input.is_some()),
                rec709: i32::from(
                    input.is_some_and(|v| v.metadata().color == FrameColor::Rec709Full),
                ),
                padding: [0; 2],
            };
            count_color(input);
            let result = unsafe {
                portable_media_plane_encode(
                    pipeline.as_ref().unwrap().0.as_ptr(),
                    context.command_buffer(),
                    context.output_texture(),
                    view.as_ref()
                        .map(|v| v.handle())
                        .unwrap_or(std::ptr::null_mut()),
                    (&params as *const Params).cast(),
                    std::mem::size_of::<Params>(),
                )
            };
            if result != 0 {
                Ok(())
            } else {
                Err(error("Sample plane encoder unavailable"))
            }
        }
    }
}
#[cfg(target_vendor = "apple")]
pub use metal::Scene;
#[cfg(target_os = "android")]
mod gl {
    use super::*;
    use ferroui_android::interop::java::*;
    use ferroui_media::RendererViewport;
    use ferroui_media_opengl::GlTextureRender;
    use ferroui_opengl::{IGlContext, SharedFrameTexture, SharedGlTextureRetirementQueue};
    use std::{rc::Rc, sync::Mutex};
    struct Native {
        java: JavaObject,
        retirement: SharedGlTextureRetirementQueue,
        _wake: Arc<dyn Fn() + Send + Sync>,
    }
    impl Drop for Native {
        fn drop(&mut self) {
            let java = self.java.clone();
            self.retirement.retire_native(move |gl| {
                call_void(
                    &java,
                    "release",
                    "(Z)V",
                    &[JavaValue::Boolean(gl.is_some())],
                )
            });
        }
    }
    pub struct Scene {
        state: Mutex<Option<Native>>,
        wake: Arc<dyn Fn() + Send + Sync>,
    }
    impl Scene {
        pub fn new(wake: Arc<dyn Fn() + Send + Sync>) -> Self {
            JavaClass::find("org/ferroui/portablemedia/PlaneRenderer");
            Self {
                state: Mutex::new(None),
                wake,
            }
        }
    }
    impl GlTextureRender for Scene {
        fn draw(
            &self,
            context: &Rc<dyn IGlContext>,
            output: u32,
            viewport: RendererViewport,
            inputs: &[FrameLease],
        ) -> Result<(), MediaError> {
            let mut state = self.state.lock().unwrap();
            if state.is_none() {
                let retirement = context
                    .shared_frame_retirement_queue()
                    .ok_or_else(|| error("No owning GL retirement queue"))?;
                *state = Some(Native {
                    java: new_object(
                        &JavaClass::find("org/ferroui/portablemedia/PlaneRenderer"),
                        "()V",
                        &[],
                    )
                    .to_global(),
                    _wake: retirement.register_wakeup(self.wake.clone()),
                    retirement,
                });
            }
            let input = inputs.first();
            let texture = input
                .map(|v| {
                    v.resource::<SharedFrameTexture>()
                        .filter(|t| {
                            Some(t.context_id()) == context.shared_frame_context_id()
                                && t.generation() == context.shared_frame_context_generation()
                        })
                        .ok_or_else(|| error("Example input is not a shared GL texture"))
                })
                .transpose()?
                .map(|v| v.texture())
                .unwrap_or(0);
            let mut parameters = PLANE.map(|v| v as f32).to_vec();
            parameters.extend_from_slice(&uv(input)?);
            let parameters = parameters
                .iter()
                .map(f32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            count_color(input);
            let result = call_object(
                &state.as_ref().unwrap().java,
                "draw",
                "(IIIILjava/lang/String;Z)Ljava/lang/String;",
                &[
                    JavaValue::Int(output as i32),
                    JavaValue::Int(texture as i32),
                    JavaValue::Int(viewport.width as i32),
                    JavaValue::Int(viewport.height as i32),
                    JavaValue::String(&parameters),
                    JavaValue::Boolean(
                        input.is_some_and(|v| v.metadata().color == FrameColor::Rec709Full),
                    ),
                ],
            )
            .map(|v| string_of(&v))
            .unwrap_or_else(|| "No sample draw result".into());
            if result.is_empty() {
                Ok(())
            } else {
                Err(error(result))
            }
        }
    }
}
#[cfg(target_os = "android")]
pub use gl::Scene;

/// Independent expected coverage: four transparent margins, center, and four
/// points distinguishing the projected trapezoid from an axis-aligned quad.
pub fn validate_probe(alpha: &[u8]) -> Result<(), MediaError> {
    let expected = [0u8, 0, 0, 0, 217, 217, 0, 217, 0];
    if alpha.len() != expected.len() || alpha.iter().zip(expected).any(|(a, b)| a.abs_diff(b) > 1) {
        return Err(error(format!(
            "Unexpected projected coverage/alpha {alpha:?}"
        )));
    }
    Ok(())
}
#[cfg(target_vendor = "apple")]
pub fn probe(frame: &FrameLease) -> Result<(), MediaError> {
    use std::ffi::c_void;
    extern "C" {
        fn portable_media_plane_probe(buffer: *mut c_void, alpha: *mut u8) -> i32;
    }
    let buffer = frame
        .resource::<ferroui_media_apple::PixelBuffer>()
        .ok_or_else(|| error("Probe output not CoreVideo"))?;
    frame
        .counters()
        .cpu_transfer(
            PixelTransferPurpose::Diagnostic,
            frame.metadata().retained_bytes,
            PresentationRoute::SharedTexture,
        )
        .map_err(|e| error(e.to_string()))?;
    let mut alpha = [0; 9];
    if unsafe { portable_media_plane_probe(buffer.as_raw(), alpha.as_mut_ptr()) } == 0 {
        return Err(error("Native projection probe failed"));
    }
    validate_probe(&alpha)
}
#[cfg(target_os = "android")]
pub async fn probe(
    frame: FrameLease,
    producer: ferroui_media_android::NativeGlFrameProducer,
) -> Result<(), MediaError> {
    use ferroui_android::interop::java::*;
    use ferroui_opengl::SharedFrameTexture;
    JavaClass::find("org/ferroui/portablemedia/PlaneRenderer");
    producer
        .invoke(move |_| {
            frame
                .counters()
                .cpu_transfer(
                    PixelTransferPurpose::Diagnostic,
                    9 * 4,
                    PresentationRoute::SharedTexture,
                )
                .map_err(|e| error(e.to_string()))?;
            let texture = frame
                .resource::<SharedFrameTexture>()
                .ok_or_else(|| error("Probe output not GL storage"))?;
            let text = call_static_object(
                &JavaClass::find("org/ferroui/portablemedia/PlaneRenderer"),
                "probe",
                "(III)Ljava/lang/String;",
                &[
                    JavaValue::Int(texture.texture() as i32),
                    JavaValue::Int(frame.metadata().width as i32),
                    JavaValue::Int(frame.metadata().height as i32),
                ],
            )
            .map(|v| string_of(&v))
            .unwrap_or_default();
            let alpha = text
                .split(',')
                .map(str::parse::<u8>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| error("Native GL projection probe failed"))?;
            validate_probe(&alpha)
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn sampling_and_qr_projection_share_crop_rotation_mirror_and_origin() {
        let budget = FrameBudget::new(Default::default());
        let mut metadata = FrameMetadata {
            identity: FrameIdentity {
                source: 1,
                generation: 1,
                sequence: 1,
            },
            width: 400,
            height: 200,
            visible: FrameRegion {
                x: 100,
                y: 50,
                width: 200,
                height: 100,
            },
            format: FrameFormat::Bgra8,
            color: FrameColor::Srgb,
            alpha: FrameAlpha::Opaque,
            rotation: FrameRotation::Clockwise90,
            mirrored: true,
            top_left_origin: true,
            timestamp: Duration::ZERO,
            device: None,
            device_generation: 1,
            planes: vec![FramePlane {
                offset: 0,
                stride: 1600,
                length: 320000,
            }],
            retained_bytes: 320000,
        };
        let frame = budget.lease(metadata.clone(), ()).unwrap();
        assert_eq!(
            uv(Some(&frame)).unwrap(),
            [0.25, 0.25, 0., 0., 0., 0.5, 0., 0., 0.5, 0., 0., 0.]
        );
        let matrix = [
            2., 0., 0., 0., 0., -2., 0., 0., 0., 0., 1., 0., -1., 1., 0., 1.,
        ];
        assert_eq!(
            ferroui_media::project_frame_point(&metadata, &matrix, (0.25, 0.25)),
            Some((-1., 1.))
        );
        assert_eq!(
            ferroui_media::project_frame_point(&metadata, &matrix, (0.75, 0.75)),
            Some((1., -1.))
        );
        metadata.top_left_origin = false;
        let frame = budget.lease(metadata, ()).unwrap();
        assert_eq!(
            uv(Some(&frame)).unwrap(),
            [0.25, 0.75, 0., 0., 0., -0.5, 0., 0., 0.5, 0., 0., 0.]
        );
    }
}
