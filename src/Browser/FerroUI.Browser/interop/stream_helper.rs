//! Set of `FileSystemWritableFileStream` and `Blob` methods.

use super::promise_helper::{JsError, JsTask};
use super::JsObject;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    #[wasm_bindgen(js_namespace = StreamHelper, js_name = seek)]
    fn seek_raw(stream: &JsObject, position: f64) -> JsObject;

    #[wasm_bindgen(js_namespace = StreamHelper, js_name = truncate)]
    fn truncate_raw(stream: &JsObject, size: f64) -> JsObject;

    #[wasm_bindgen(js_namespace = StreamHelper, js_name = write)]
    fn write_raw(stream: &JsObject, pointer: usize, count: usize) -> JsObject;

    #[wasm_bindgen(js_namespace = StreamHelper, js_name = close)]
    fn close_raw(stream: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = StreamHelper, js_name = byteLength)]
    fn byte_length_raw(stream: &JsObject) -> f64;

    #[wasm_bindgen(js_namespace = StreamHelper, js_name = sliceArrayBuffer)]
    fn slice_to_array_buffer(stream: &JsObject, offset: f64, count: usize) -> JsObject;

    #[wasm_bindgen(js_namespace = StreamHelper, js_name = byteArrayLength)]
    fn byte_array_length(buffer: &JsObject) -> usize;

    #[wasm_bindgen(js_namespace = StreamHelper, js_name = toMemoryView)]
    fn array_buffer_to_memory_view(buffer: &JsObject, pointer: usize);
}

/// Moves the position of `stream` to `position`. The returned task settles
/// once the stream has processed the request.
pub fn seek(stream: &JsObject, position: u64) -> JsTask {
    JsTask::new(seek_raw(stream, position as f64))
}

/// Resizes the file of `stream` to `size` bytes.
pub fn truncate(stream: &JsObject, size: u64) -> JsTask {
    JsTask::new(truncate_raw(stream, size as f64))
}

/// Writes `data` at the position of `stream`. The bytes are copied before
/// this returns; the task settles once the stream has written them.
pub fn write_async(stream: &JsObject, data: &[u8]) -> JsTask {
    JsTask::new(write_raw(stream, data.as_ptr() as usize, data.len()))
}

/// Closes `stream`, which commits what was written to the file.
pub fn close_async(stream: &JsObject) -> JsTask {
    JsTask::new(close_raw(stream))
}

/// The size of the blob `stream` in bytes.
pub fn byte_length(stream: &JsObject) -> u64 {
    byte_length_raw(stream) as u64
}

/// Reads `count` bytes of the blob `stream` from `offset`. Fewer bytes are
/// returned when the blob ends before.
pub async fn slice_async(stream: &JsObject, offset: u64, count: usize) -> Result<Vec<u8>, JsError> {
    let buffer = JsTask::new(slice_to_array_buffer(stream, offset as f64, count)).await?;
    let mut bytes = vec![0u8; byte_array_length(&buffer)];
    // The script copies exactly `bytes.len()` bytes to the address of `bytes`, which nothing else
    // references while the call runs.
    array_buffer_to_memory_view(&buffer, bytes.as_mut_ptr() as usize);
    Ok(bytes)
}
