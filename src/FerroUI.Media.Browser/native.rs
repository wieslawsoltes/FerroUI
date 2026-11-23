use ferroui_media::{frames::FrameRetirement, MediaError};
use ferroui_opengl::SharedFrameTexture;
use std::thread::{self, ThreadId};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    #[wasm_bindgen(js_namespace=SharedGlFrames,js_name=graphics)]
    pub fn graphics(context: i32) -> i32;
    #[wasm_bindgen(js_namespace=WebGlRenderTarget,js_name=frameGeneration)]
    fn generation(context: i32) -> u32;
    #[wasm_bindgen(js_namespace=WebGlRenderTarget,js_name=contextLost)]
    fn lost(context: i32) -> bool;
    #[wasm_bindgen(catch,js_namespace=SharedGlFrames,js_name=create)]
    fn create_texture(context: i32, generation: u32) -> Result<u32, JsValue>;
    #[wasm_bindgen(catch,js_namespace=SharedGlFrames,js_name=release)]
    fn release(texture: u32) -> Result<(), JsValue>;
    #[wasm_bindgen(catch,js_namespace=SharedGlFrames,js_name=quadrants)]
    fn write_quadrants(texture: u32, width: u32, height: u32) -> Result<(), JsValue>;
    #[wasm_bindgen(catch,js_namespace=SharedGlFrames,js_name=upload)]
    fn write_image(texture: u32, source: &JsValue, width: u32, height: u32) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace=SharedGlFrames,js_name=isVideoSource)]
    pub fn is_video_source(source: &JsValue) -> bool;
    #[wasm_bindgen(js_namespace=SharedGlFrames,js_name=canTransfer)]
    pub fn can_transfer() -> bool;
    #[wasm_bindgen(catch,js_namespace=SharedGlFrames,js_name=beginTransfer)]
    fn transfer_image(
        source: &JsValue,
        width: u32,
        height: u32,
        analysis: bool,
    ) -> Result<TransferRequest, JsValue>;
    type TransferRequest;
    #[wasm_bindgen(method, getter)]
    fn id(this: &TransferRequest) -> u32;
    #[wasm_bindgen(method, getter)]
    fn ready(this: &TransferRequest) -> JsValue;
    #[wasm_bindgen(js_namespace=SharedGlFrames,js_name=discard)]
    fn discard(frame: u32) -> JsValue;
    #[cfg(feature = "qr")]
    #[wasm_bindgen(catch,js_namespace=SharedGlFrames,js_name=recognize)]
    pub fn recognize(
        frame: u32,
        transferred: bool,
        width: u32,
        height: u32,
    ) -> Result<JsValue, JsValue>;
}
/// A browser media frame transferred to the render worker (#601): the number the page's script
/// gave it. The frame lives on the worker until the compositor imports it; a lease dropped
/// before that discards it, on whichever thread the last lease goes.
pub struct TransferredFrame {
    id: u32,
    retirement: Option<FrameRetirement>,
    thread: ThreadId,
}
impl TransferredFrame {
    pub fn id(&self) -> u32 {
        self.id
    }
}
impl Drop for TransferredFrame {
    fn drop(&mut self) {
        let id = self.id;
        let retirement = self.retirement.take().unwrap();
        let release = move || {
            let pending = ferroui_browser::interop::completion_helper::await_promise(&discard(id));
            ferroui_base::threading::Dispatcher::ui_thread().spawn_local(async move {
                if pending.await.is_err() {
                    retirement.quarantine();
                }
            });
        };
        if self.thread == thread::current().id() {
            release();
        } else {
            ferroui_base::threading::Dispatcher::ui_thread()
                .post(release, ferroui_base::threading::DispatcherPriority::NORMAL);
        }
    }
}
/// Wraps the current image of the media element as a WebCodecs frame and transfers it to the
/// render worker.
pub fn transfer(
    source: &JsValue,
    width: u32,
    height: u32,
    analysis: bool,
    retirement: FrameRetirement,
) -> Result<(TransferredFrame, JsValue), MediaError> {
    let request = transfer_image(source, width, height, analysis).map_err(error)?;
    Ok((
        TransferredFrame {
            id: request.id(),
            retirement: Some(retirement),
            thread: thread::current().id(),
        },
        request.ready(),
    ))
}
pub fn current(context: i32, expected: u64) -> bool {
    u64::from(generation(context)) == expected && !lost(context)
}
struct TextureOwner {
    texture: u32,
    thread: ThreadId,
    retirement: Option<FrameRetirement>,
}
impl Drop for TextureOwner {
    fn drop(&mut self) {
        if self.thread != thread::current().id() || release(self.texture).is_err() {
            self.retirement.take().unwrap().quarantine();
        }
    }
}
pub fn create(
    context: i32,
    generation: u64,
    retirement: FrameRetirement,
) -> Result<SharedFrameTexture, MediaError> {
    let texture = create_texture(context, generation as u32).map_err(error)?;
    // SAFETY: create_texture registers this new GL_TEXTURE_2D in the current
    // admitted page context. TextureOwner releases it only after all producer
    // and consumer fences retire. Allocation is charged before GPU storage.
    Ok(unsafe {
        SharedFrameTexture::new(
            context as u64,
            generation,
            texture,
            TextureOwner {
                texture,
                thread: thread::current().id(),
                retirement: Some(retirement),
            },
        )
    })
}
pub fn quadrants(texture: u32, width: u32, height: u32) -> Result<(), MediaError> {
    write_quadrants(texture, width, height).map_err(error)
}
pub fn upload(texture: u32, source: &JsValue, width: u32, height: u32) -> Result<(), MediaError> {
    write_image(texture, source, width, height).map_err(error)
}
fn error(error: JsValue) -> MediaError {
    super::unsupported(
        error
            .as_string()
            .unwrap_or_else(|| format!("Browser GPU operation failed: {error:?}")),
    )
}
