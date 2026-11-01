//! Explicit counted GPU/composition/window diagnostics, excluded from default builds.
use ferroui_base::{
    media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions},
    rendering::composition::{frames::*, ElementComposition},
    Ref,
};
use ferroui_controls::Window;
use ferroui_media::{MediaError, MediaErrorKind};
use ferroui_media_opengl::NativeGlFrameProducer;
use ferroui_opengl::SharedFrameTexture;
use image::RgbaImage;
use std::{
    task::Poll,
    time::{Duration, Instant},
};
#[path = "camera_presented.rs"]
mod os;
pub use os::capture as presented;
pub async fn wait<T: 'static>(
    mut check: impl FnMut() -> Result<Option<T>, String> + 'static,
) -> Result<T, String> {
    let started = Instant::now();
    ferroui_media_windows::poll_native(move || match check() {
        Ok(Some(v)) => Poll::Ready(Ok(v)),
        Err(e) => Poll::Ready(Err(MediaError::new(MediaErrorKind::NativeFailure, e))),
        Ok(None) if started.elapsed() > Duration::from_secs(10) => Poll::Ready(Err(
            MediaError::new(MediaErrorKind::NativeFailure, "Pixel stage timed out"),
        )),
        _ => Poll::Pending,
    })
    .await
    .map_err(|e| e.to_string())
}
pub async fn settle() -> Result<(), String> {
    let start = Instant::now();
    wait(move || Ok((start.elapsed() > Duration::from_millis(100)).then_some(()))).await
}
pub async fn read_texture(
    producer: &NativeGlFrameProducer,
    frame: FrameLease,
) -> Result<RgbaImage, String> {
    let (width, height) = (frame.metadata().width, frame.metadata().height);
    let pixels = producer
        .invoke(move |context| {
            let texture = frame.resource::<SharedFrameTexture>().ok_or_else(|| {
                MediaError::new(MediaErrorKind::Unsupported, "Not a leased GL frame")
            })?;
            if texture.context_id() != context.shared_frame_context_id().unwrap_or(0) {
                return Err(MediaError::new(
                    MediaErrorKind::DeviceLost,
                    "Diagnostic context differs",
                ));
            }
            let gl = context.gl_interface();
            let draw = gl.get_integerv(0x8ca6);
            let read = gl.get_integerv(0x8caa);
            let pack_buffer = gl.get_integerv(0x88ed);
            let pack_names = [0x0d05, 0x0d02, 0x0d03, 0x0d04];
            let pack = pack_names.map(|name| gl.get_integerv(name));
            let fbo = gl.gen_framebuffer();
            gl.bind_framebuffer(0x8d40, fbo);
            gl.framebuffer_texture_2d(0x8d40, 0x8ce0, 0x0de1, texture.texture() as i32, 0);
            let result = (|| {
                if gl.check_framebuffer_status(0x8d40) != 0x8cd5 {
                    return Err(MediaError::new(
                        MediaErrorKind::NativeFailure,
                        "Diagnostic framebuffer incomplete",
                    ));
                }
                let mut pixels = vec![0u8; width as usize * height as usize * 4];
                frame
                    .counters()
                    .cpu_transfer(
                        PixelTransferPurpose::Diagnostic,
                        pixels.len() as u64,
                        PresentationRoute::SharedTexture,
                    )
                    .map_err(|e| MediaError::new(MediaErrorKind::NativeFailure, e.to_string()))?;
                let complete = GpuCompletion::new();
                complete
                    .retain(frame.clone())
                    .map_err(|e| MediaError::new(MediaErrorKind::NativeFailure, e.to_string()))?;
                gl.bind_buffer(0x88eb, 0);
                for name in pack_names {
                    gl.pixel_storei(name, if name == 0x0d05 { 1 } else { 0 });
                }
                unsafe {
                    gl.read_pixels(
                        0,
                        0,
                        width as i32,
                        height as i32,
                        0x1908,
                        0x1401,
                        pixels.as_mut_ptr().cast(),
                    );
                }
                let error = gl.get_error();
                context.schedule_frame_completion(complete);
                if error != 0 {
                    return Err(MediaError::new(
                        MediaErrorKind::NativeFailure,
                        format!("Diagnostic GL read 0x{error:x}"),
                    ));
                }
                Ok(pixels)
            })();
            gl.bind_framebuffer(0x8ca9, draw);
            gl.bind_framebuffer(0x8ca8, read);
            gl.delete_framebuffer(fbo);
            gl.bind_buffer(0x88eb, pack_buffer);
            for (name, value) in pack_names.into_iter().zip(pack) {
                gl.pixel_storei(name, value);
            }
            result
        })
        .await
        .map_err(|e| e.to_string())?;
    let image = RgbaImage::from_raw(width, height, pixels).ok_or("Invalid diagnostic pixels")?;
    Ok(image)
}
pub async fn composition(
    window: &Ref<Window>,
    counters: &FrameTransferCounters,
    name: &str,
) -> Result<RgbaImage, String> {
    let visual =
        ElementComposition::get_element_visual(window).ok_or("No window composition visual")?;
    let compositor = visual.compositor().clone();
    let scaling = window.render_scaling();
    let size = window.bounds().size();
    counters
        .cpu_transfer(
            PixelTransferPurpose::Diagnostic,
            (size.width * size.height * scaling * scaling * 4.) as u64,
            PresentationRoute::SharedTexture,
        )
        .map_err(|e| e.to_string())?;
    let job = compositor.create_composition_visual_snapshot(&visual, scaling);
    compositor.commit();
    let bitmap = wait(move || match job.take_result() {
        Some(result) => result.map(Some).map_err(|e| e.to_string()),
        None => Ok(None),
    })
    .await?;
    let mut encoded = Vec::new();
    bitmap
        .save(
            &mut encoded,
            &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT),
        )
        .map_err(|e| e.to_string())?;
    counters
        .cpu_transfer(
            PixelTransferPurpose::Diagnostic,
            encoded.len() as u64,
            PresentationRoute::SharedTexture,
        )
        .map_err(|e| e.to_string())?;
    let pixels = image::load_from_memory(&encoded)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    counters
        .cpu_transfer(
            PixelTransferPurpose::Diagnostic,
            pixels.len() as u64,
            PresentationRoute::SharedTexture,
        )
        .map_err(|e| e.to_string())?;
    if let Some(directory) = std::env::var_os("FERROUI_MEDIA_EVIDENCE") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let path = directory.join(format!("camera-{name}.png"));
        std::fs::write(&path, encoded).map_err(|e| e.to_string())?;
        println!("CAMERA_COMPOSITION_ARTIFACT {}", path.display());
    }
    Ok(pixels)
}
