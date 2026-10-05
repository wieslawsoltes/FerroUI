use super::completion_helper::{await_promise, PromiseOutcome};
use super::{non_null, JsObject};
use crate::browser_top_level_impl::BrowserTopLevelImpl;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// The hidden input element of a view, for the properties the text
    /// input method reads from it.
    pub type HtmlInputElement;

    #[wasm_bindgen(method, getter, js_name = selectionStart)]
    fn selection_start_raw(this: &HtmlInputElement) -> Option<i32>;

    #[wasm_bindgen(method, getter, js_name = selectionEnd)]
    fn selection_end_raw(this: &HtmlInputElement) -> Option<i32>;

    /// Subscribes to the input events of the element of a top-level.
    /// Returns the subscription, to be passed to
    /// [`unsubscribe_input_events`].
    #[wasm_bindgen(js_namespace = InputHelper, js_name = subscribeInputEvents)]
    pub fn subscribe_input_events(html_element: &JsObject, top_level_id: i32) -> JsObject;

    /// Ends a subscription made by [`subscribe_input_events`].
    // Differs from the original, which drops the subscription and never ends it: the top-level
    // unsubscribes when it is disposed.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = unsubscribeInputEvents)]
    pub fn unsubscribe_input_events(subscription: &JsObject);

    /// The points the browser coalesced into a pointer move, six numbers
    /// per point: x, y, pressure, x tilt, y tilt, twist.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = getCoalescedEvents)]
    pub fn get_coalesced_events(pointer_event: &JsObject) -> Vec<f64>;

    /// Empties the hidden input element.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = clearInput)]
    pub fn clear_input_element(html_element: &JsObject);

    /// Gives the keyboard focus of the page to the element.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = focusElement)]
    pub fn focus_element(html_element: &JsObject);

    /// Sets the CSS cursor of the element; `"default"` removes it.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = setCursor)]
    pub fn set_cursor(html_element: &JsObject, kind: &str);

    #[wasm_bindgen(js_namespace = InputHelper, js_name = hide)]
    pub fn hide_element(html_element: &JsObject);

    #[wasm_bindgen(js_namespace = InputHelper, js_name = show)]
    pub fn show_element(html_element: &JsObject);

    /// Mirrors the text around the caret and the selection into the hidden
    /// input element.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = setSurroundingText)]
    pub fn set_surrounding_text(html_element: &JsObject, text: &str, start: i32, end: i32);

    /// Moves the hidden input element so that its caret is where the caret
    /// of the text is.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = setBounds)]
    pub fn set_bounds(html_element: &JsObject, x: i32, y: i32, width: i32, height: i32, caret: i32);

    /// Installs the handlers of the page that are not bound to a view.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = initializeBackgroundHandlers)]
    pub fn initialize_background_handlers(global_this: &JsObject);

    /// Whether the clipboard of the page can hold values of a format.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = isClipboardFormatSupported)]
    pub fn is_clipboard_format_supported(format: &str) -> bool;

    /// Creates the list of items written to the clipboard.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = createWriteableClipboardSource)]
    pub fn create_writeable_clipboard_source() -> JsObject;

    /// Adds an item to a list created by
    /// [`create_writeable_clipboard_source`] and returns it.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = createWriteableClipboardItem)]
    pub fn create_writeable_clipboard_item(source: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = InputHelper, js_name = addStringToWriteableClipboardItem)]
    pub fn add_string_to_writeable_clipboard_item(item: &JsObject, format: &str, value: &str);

    #[wasm_bindgen(js_namespace = InputHelper, js_name = addBytesToWriteableClipboardItem)]
    pub fn add_bytes_to_writeable_clipboard_item(item: &JsObject, format: &str, value: &[u8]);

    /// Reads the clipboard. Answers with a promise of the result (see
    /// [`ClipboardResult`]).
    #[wasm_bindgen(js_namespace = InputHelper, js_name = readClipboard)]
    fn read_clipboard_raw(window: &JsObject) -> JsObject;

    /// Writes the items of a list to the clipboard, or empties it when
    /// there is no list. Answers with a promise of `"denied"` when the page
    /// is not allowed to write, and of an empty string otherwise.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = writeClipboard)]
    fn write_clipboard_raw(global_this: &JsObject, source: Option<&JsObject>) -> JsObject;

    /// The format strings of a readable data item.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = getReadableDataItemFormats)]
    pub fn get_readable_data_item_formats(item: &JsObject) -> Vec<String>;

    /// Reads the value of a readable data item in a format. Answers with a
    /// promise of the value (see [`ReadableDataValue`]) or of `null`.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = tryGetReadableDataItemValueAsync)]
    fn try_get_readable_data_item_value_async_raw(item: &JsObject, format: &str) -> JsObject;

    /// Reads the value of an item of a drag operation in a format, while
    /// the event of the operation is being dispatched: the value (see
    /// [`ReadableDataValue`]) or `null`.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = tryGetReadableDataItemValue)]
    fn try_get_readable_data_item_value_raw(item: &JsObject, format: &str) -> JsObject;

    // The identifier of a pointer is a whole number the page holds as a number.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = setPointerCapture)]
    fn set_pointer_capture_raw(container_element: &JsObject, pointer_id: f64);

    #[wasm_bindgen(js_namespace = InputHelper, js_name = releasePointerCapture)]
    fn release_pointer_capture_raw(container_element: &JsObject, pointer_id: f64);

    /// What `readClipboard` answers with: an error (`"denied"`) or the items
    /// of the clipboard.
    pub type ClipboardResult;

    #[wasm_bindgen(method, getter)]
    fn error(this: &ClipboardResult) -> Option<String>;

    #[wasm_bindgen(method, getter)]
    fn result(this: &ClipboardResult) -> JsObject;

    /// An array of the page.
    pub type JsArray;

    #[wasm_bindgen(method, getter)]
    fn length(this: &JsArray) -> u32;

    #[wasm_bindgen(method, indexing_getter)]
    fn get(this: &JsArray, index: u32) -> JsObject;

    /// A value read from a readable data item: `type` is `"string"`,
    /// `"bytes"` or `"file"`, and `value` the string, the bytes or the file.
    pub type ReadableDataValue;

    #[wasm_bindgen(method, getter, js_name = type)]
    fn value_type(this: &ReadableDataValue) -> Option<String>;

    #[wasm_bindgen(method, getter, js_name = value)]
    fn string_value(this: &ReadableDataValue) -> Option<String>;

    #[wasm_bindgen(method, getter, js_name = value)]
    fn bytes_value(this: &ReadableDataValue) -> Vec<u8>;

    #[wasm_bindgen(method, getter, js_name = value)]
    fn object_value(this: &ReadableDataValue) -> JsObject;

    /// The data transfer of a drag event of the page.
    pub type DataTransfer;

    #[wasm_bindgen(method, getter, js_name = effectAllowed)]
    fn effect_allowed_raw(this: &DataTransfer) -> Option<String>;

    #[wasm_bindgen(method, setter, js_name = dropEffect)]
    fn set_drop_effect_raw(this: &DataTransfer, value: &str);
}

/// The outcome of reading the clipboard of the page.
pub struct ClipboardReadResult {
    /// `"denied"` when the page is not allowed to read the clipboard.
    pub error: Option<String>,
    /// The array of readable data items, if there is one.
    pub items: Option<JsObject>,
}

/// Reads the clipboard of the page.
pub async fn read_clipboard_async(window: &JsObject) -> PromiseOutcome<ClipboardReadResult> {
    let result = await_promise(&read_clipboard_raw(window)).await?;
    let result = result.unchecked_ref::<ClipboardResult>();
    Ok(ClipboardReadResult { error: result.error(), items: non_null(result.result()) })
}

/// Writes a list of items to the clipboard of the page, or empties it.
/// Resolves to `"denied"` when the page is not allowed to write.
pub async fn write_clipboard_async(global_this: &JsObject, source: Option<&JsObject>) -> PromiseOutcome<String> {
    let result = await_promise(&write_clipboard_raw(global_this, source)).await?;
    Ok(result.as_string().unwrap_or_default())
}

/// Reads the value of a readable data item in a format.
pub async fn try_get_readable_data_item_value_async(item: &JsObject, format: &str) -> PromiseOutcome<Option<JsObject>> {
    let value = await_promise(&try_get_readable_data_item_value_async_raw(item, format)).await?;
    Ok(non_null(value))
}

/// Reads the value of an item of a drag operation in a format.
pub fn try_get_readable_data_item_value(item: &JsObject, format: &str) -> Option<JsObject> {
    non_null(try_get_readable_data_item_value_raw(item, format))
}

/// The length of an array of the page.
pub fn get_array_length(array: &JsObject) -> u32 {
    array.unchecked_ref::<JsArray>().length()
}

/// An item of an array of the page.
pub fn get_array_item(array: &JsObject, index: u32) -> JsObject {
    array.unchecked_ref::<JsArray>().get(index)
}

/// What a readable data value holds.
pub enum ReadableDataValueContent {
    String(String),
    Bytes(Vec<u8>),
    File(JsObject),
}

/// The content of a readable data value; `None` for a type it cannot have.
pub fn get_readable_data_value(value: &JsObject) -> Option<ReadableDataValueContent> {
    let value = value.unchecked_ref::<ReadableDataValue>();
    match value.value_type().as_deref() {
        Some("string") => value.string_value().map(ReadableDataValueContent::String),
        Some("bytes") => Some(ReadableDataValueContent::Bytes(value.bytes_value())),
        Some("file") => non_null(value.object_value()).map(ReadableDataValueContent::File),
        _ => None,
    }
}

/// The effects the source of a drag operation allows (`effectAllowed`).
pub fn get_effect_allowed(data_transfer: &JsObject) -> Option<String> {
    data_transfer.unchecked_ref::<DataTransfer>().effect_allowed_raw()
}

/// Sets the effect of a drag operation the page shows (`dropEffect`).
pub fn set_drop_effect(data_transfer: &JsObject, value: &str) {
    data_transfer.unchecked_ref::<DataTransfer>().set_drop_effect_raw(value);
}

/// The start of the selection of an input element; 0 when it has none.
pub fn get_selection_start(input_element: &JsObject) -> i32 {
    input_element.unchecked_ref::<HtmlInputElement>().selection_start_raw().unwrap_or(0)
}

/// The end of the selection of an input element; 0 when it has none.
pub fn get_selection_end(input_element: &JsObject) -> i32 {
    input_element.unchecked_ref::<HtmlInputElement>().selection_end_raw().unwrap_or(0)
}

/// Captures a pointer to the element.
pub fn set_pointer_capture(container_element: &JsObject, pointer_id: i64) {
    set_pointer_capture_raw(container_element, pointer_id as f64);
}

/// Releases the capture of a pointer, if the element has it.
pub fn release_pointer_capture(container_element: &JsObject, pointer_id: i64) {
    release_pointer_capture_raw(container_element, pointer_id as f64);
}

// The original answers the page with a task (`RedirectInputAsync`, `RedirectInputRetunAsync`)
// that the page awaits. Here every callback runs synchronously on the UI thread and returns its
// result directly, so that the page can decide about the default action of the event while it
// is still being dispatched.

/// A key went down. Returns whether the application handled it.
#[wasm_bindgen(js_name = InputHelper_OnKeyDown)]
pub fn on_key_down(top_level_id: i32, code: Option<String>, key: Option<String>, modifier: i32) -> bool {
    match BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        Some(top_level) => top_level.input_handler().on_key_down(code.as_deref(), key.as_deref(), modifier),
        None => false,
    }
}

/// A key went up. Returns whether the application handled it.
#[wasm_bindgen(js_name = InputHelper_OnKeyUp)]
pub fn on_key_up(top_level_id: i32, code: Option<String>, key: Option<String>, modifier: i32) -> bool {
    match BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        Some(top_level) => top_level.input_handler().on_key_up(code.as_deref(), key.as_deref(), modifier),
        None => false,
    }
}

#[wasm_bindgen(js_name = InputHelper_OnBeforeInput)]
pub fn on_before_input(top_level_id: i32, input_type: &str, start: i32, end: i32) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().text_input_method().on_before_input(input_type, start, end);
    }
}

#[wasm_bindgen(js_name = InputHelper_OnCompositionStart)]
pub fn on_composition_start(top_level_id: i32) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().text_input_method().on_composition_start();
    }
}

#[wasm_bindgen(js_name = InputHelper_OnCompositionUpdate)]
pub fn on_composition_update(top_level_id: i32, data: Option<String>) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().text_input_method().on_composition_update(data.as_deref());
    }
}

#[wasm_bindgen(js_name = InputHelper_OnCompositionEnd)]
pub fn on_composition_end(top_level_id: i32, data: Option<String>) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().text_input_method().on_composition_end(data.as_deref());
    }
}

/// A pointer moved. `args_obj` is the event of the page, kept for the
/// coalesced points of the move.
#[wasm_bindgen(js_name = InputHelper_OnPointerMove)]
#[allow(clippy::too_many_arguments)]
pub fn on_pointer_move(
    top_level_id: i32,
    pointer_type: &str,
    pointer_id: f64,
    offset_x: f64,
    offset_y: f64,
    pressure: f64,
    tilt_x: f64,
    tilt_y: f64,
    twist: f64,
    modifier: i32,
    args_obj: JsObject,
) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().on_pointer_move(
            pointer_type,
            pointer_id as i64,
            offset_x,
            offset_y,
            pressure,
            tilt_x,
            tilt_y,
            twist,
            modifier,
            args_obj,
        );
    }
}

/// A pointer went down. `buttons` is the button of the event that changed
/// its state.
#[wasm_bindgen(js_name = InputHelper_OnPointerDown)]
#[allow(clippy::too_many_arguments)]
pub fn on_pointer_down(
    top_level_id: i32,
    pointer_type: &str,
    pointer_id: f64,
    buttons: i32,
    offset_x: f64,
    offset_y: f64,
    pressure: f64,
    tilt_x: f64,
    tilt_y: f64,
    twist: f64,
    modifier: i32,
) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().on_pointer_down(
            pointer_type,
            pointer_id as i64,
            buttons,
            offset_x,
            offset_y,
            pressure,
            tilt_x,
            tilt_y,
            twist,
            modifier,
        );
    }
}

/// A pointer went up. `buttons` is the button of the event that changed its
/// state.
#[wasm_bindgen(js_name = InputHelper_OnPointerUp)]
#[allow(clippy::too_many_arguments)]
pub fn on_pointer_up(
    top_level_id: i32,
    pointer_type: &str,
    pointer_id: f64,
    buttons: i32,
    offset_x: f64,
    offset_y: f64,
    pressure: f64,
    tilt_x: f64,
    tilt_y: f64,
    twist: f64,
    modifier: i32,
) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().on_pointer_up(
            pointer_type,
            pointer_id as i64,
            buttons,
            offset_x,
            offset_y,
            pressure,
            tilt_x,
            tilt_y,
            twist,
            modifier,
        );
    }
}

#[wasm_bindgen(js_name = InputHelper_OnPointerCancel)]
#[allow(clippy::too_many_arguments)]
pub fn on_pointer_cancel(
    top_level_id: i32,
    pointer_type: &str,
    pointer_id: f64,
    offset_x: f64,
    offset_y: f64,
    pressure: f64,
    tilt_x: f64,
    tilt_y: f64,
    twist: f64,
    modifier: i32,
) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().on_pointer_cancel(
            pointer_type,
            pointer_id as i64,
            offset_x,
            offset_y,
            pressure,
            tilt_x,
            tilt_y,
            twist,
            modifier,
        );
    }
}

/// The wheel turned. `delta_mode` is the unit of the deltas, as the event
/// of the page reports it.
// Differs from the original, which does not pass the unit and takes every delta for pixels.
#[wasm_bindgen(js_name = InputHelper_OnWheel)]
pub fn on_wheel(
    top_level_id: i32,
    offset_x: f64,
    offset_y: f64,
    delta_x: f64,
    delta_y: f64,
    delta_mode: i32,
    modifier: i32,
) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().on_wheel(offset_x, offset_y, delta_x, delta_y, delta_mode, modifier);
    }
}

/// A drag operation of the page entered, moved over, left or dropped on the
/// element of a top-level. `data_transfer` is the data transfer of the
/// event and `items` its items as readable data items.
#[wasm_bindgen(js_name = InputHelper_OnDragDrop)]
pub fn on_drag_drop(
    top_level_id: i32,
    type_: &str,
    offset_x: f64,
    offset_y: f64,
    modifiers: i32,
    data_transfer: JsObject,
    items: JsObject,
) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().on_drag_event(type_, offset_x, offset_y, modifiers, &data_transfer, items);
    }
}

/// The on-screen keyboard changed the part of the view it covers.
#[wasm_bindgen(js_name = InputHelper_OnKeyboardGeometryChange)]
pub fn on_keyboard_geometry_change(top_level_id: i32, x: f64, y: f64, width: f64, height: f64) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.input_handler().input_pane().on_geometry_change(x, y, width, height);
    }
}

/// The keyboard focus of the page left the element of a top-level.
// Not in the original, where the lost-focus notification of the top-level is never raised.
#[wasm_bindgen(js_name = InputHelper_OnLostFocus)]
pub fn on_lost_focus(top_level_id: i32) {
    if let Some(top_level) = BrowserTopLevelImpl::try_get_top_level(top_level_id) {
        top_level.on_lost_focus();
    }
}
