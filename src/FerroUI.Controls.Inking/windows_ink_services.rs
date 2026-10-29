//! Windows ink interoperability through the operating system's ink engine.
//!
//! Attach [`WindowsInkServices`] to a FerroUI `InkStrokeContainer` to use
//! genuine ISF/GIF persistence and the system ink clipboard. Drawing remains
//! in FerroUI. Native async operations are awaited on the calling UI executor;
//! this module never blocks on an asynchronous WinRT operation.
//!
//! Windows 10 version 1703 or newer is required for tilt/timestamp samples and
//! explicit ISF output. `new` uses FerroUI's existing WinRT initialization.
//! HRESULTs, unavailable recognizers and unsupported attributes become
//! `InkError::Platform`; there is no silent fallback to the portable format.
//!
//! Windows uses single-precision coordinates/matrices and can quantize ISF
//! telemetry. Its public `InkPoint` has no twist field. Nonzero twist is rejected
//! unless the application explicitly enables [`WindowsInkServices::allow_twist_loss`].
//! Imported strokes get fresh FerroUI identities; OS-only custom properties,
//! stroke-start wall-clock times and recognition state have no portable field.
//! Use the portable format for an exact FerroUI document round trip.
//!
//! The ink engine is called through the MicroCom bindings of the crate
//! (`windows_ink.idl`, `windows_ink_interop.rs`), as the WinUI integration
//! calls the Windows Runtime; WinUI's own ink control calls it in C++/WinRT.

use ferroui_base::media::Color;
use ferroui_base::{Matrix, Point, Rect, Size};
use crate::windows_ink_interop as winrt;
use crate::windows_ink_interop::{from_abi, object, take_string, to_abi, wait};
use crate::{
    IInkPlatformServices, InkDrawingAttributes, InkDrawingAttributesKind, InkError,
    InkPersistenceFormat, InkPersistenceLimits, InkPoint, InkServiceFuture, InkStroke,
    InkStrokeContainer, PenTipShape,
};
use ferroui_microcom::{ComPtr, HResult, IUnknown};
use ferroui_win32::interop::unmanaged_methods::{
    co_get_apartment_type, APTTYPE_MAINSTA, APTTYPE_STA,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

type NativeContainer = ComPtr<winrt::IInkStrokeContainer>;
type NativeStroke = ComPtr<winrt::IInkStroke>;
type NativeAttributes = ComPtr<winrt::IInkDrawingAttributes>;

const INK_STROKE_CONTAINER: &str = "Windows.UI.Input.Inking.InkStrokeContainer";
const INK_STROKE_BUILDER: &str = "Windows.UI.Input.Inking.InkStrokeBuilder";
const INK_POINT: &str = "Windows.UI.Input.Inking.InkPoint";
const INK_DRAWING_ATTRIBUTES: &str = "Windows.UI.Input.Inking.InkDrawingAttributes";
const INK_RECOGNIZER_CONTAINER: &str = "Windows.UI.Input.Inking.InkRecognizerContainer";
const IN_MEMORY_RANDOM_ACCESS_STREAM: &str = "Windows.Storage.Streams.InMemoryRandomAccessStream";
const DATA_READER: &str = "Windows.Storage.Streams.DataReader";
const DATA_WRITER: &str = "Windows.Storage.Streams.DataWriter";

/// One recognized word/group, with candidates ordered by the Windows engine.
/// `stroke_ids` refer to the input FerroUI strokes, not temporary native IDs.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowsInkRecognitionResult {
    pub bounding_rect: Rect,
    pub text_candidates: Vec<String>,
    pub stroke_ids: Vec<u32>,
}

/// UI-thread native persistence, clipboard and optional handwriting recognition.
///
/// Construction verifies that the native ink class is available and initializes
/// WinRT through the existing Windows backend. The marker keeps this service on
/// that thread; the futures also intentionally do not implement `Send`.
///
/// Clipboard operations require the single-threaded COM apartment of the
/// FerroUI Windows UI thread. Persistence also works on an MTA worker, but the
/// Windows clipboard's data-transfer class cannot activate there. Such calls
/// return a descriptive platform error; they do not move UI objects between
/// threads or change an existing apartment.
pub struct WindowsInkServices {
    limits: InkPersistenceLimits,
    allow_twist_loss: bool,
    _ui_thread: PhantomData<Rc<()>>,
}

impl WindowsInkServices {
    pub fn new() -> Result<Self, InkError> {
        winrt::activate::<IUnknown>(INK_STROKE_CONTAINER)
            .map_err(|error| InkError::Platform(format!("Windows ink activation failed: {error:?}")))?;
        Ok(Self {
            limits: InkPersistenceLimits::DEFAULT,
            allow_twist_loss: false,
            _ui_thread: PhantomData,
        })
    }

    /// Sets bounds for byte buffers and imported/exported stroke collections.
    /// The native decoder owns its internal allocations; byte limits apply
    /// before calling it and record limits apply before allocating Rust vectors.
    pub fn with_limits(mut self, limits: InkPersistenceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Explicitly permits dropping nonzero per-sample twist on native export.
    /// This does not modify the original FerroUI strokes. Import always sets
    /// twist to zero because the Windows `InkPoint` API does not expose it.
    pub fn allow_twist_loss(mut self, allow: bool) -> Self {
        self.allow_twist_loss = allow;
        self
    }

    /// Display names of the handwriting recognizers installed in Windows.
    pub fn recognizers(&self) -> Result<Vec<String>, InkError> {
        let container = winrt::activate::<winrt::IInkRecognizerContainer>(INK_RECOGNIZER_CONTAINER).map_err(platform)?;
        let recognizers = object(container.get_recognizers()).map_err(platform)?;
        let mut result = Vec::new();
        for index in 0..recognizers.size().map_err(platform)? {
            let recognizer = object(recognizers.get_at(index)).map_err(platform)?;
            result.push(take_string(recognizer.name().map_err(platform)?));
        }
        Ok(result)
    }

    /// Recognizes the supplied strokes with the OS default or a named installed
    /// recognizer. Results are returned separately and do not mutate ink.
    pub fn recognize(
        &self,
        strokes: Vec<Rc<InkStroke>>,
        recognizer_name: Option<String>,
    ) -> InkServiceFuture<Vec<WindowsInkRecognitionResult>> {
        // Convert before yielding so attributes/selection are a call-time snapshot.
        let native = self.native_container(&strokes);
        Box::pin(async move {
            let native = native?;
            if strokes.is_empty() {
                return Ok(Vec::new());
            }
            let source = object(native.get_strokes()).map_err(platform)?;
            let mut identities = HashMap::with_capacity(strokes.len());
            for (index, stroke) in strokes.iter().enumerate() {
                let native_stroke = object(source.get_at(index as u32)).map_err(platform)?;
                identities.insert(stroke_id(&native_stroke)?, stroke.id());
            }
            let recognizer = winrt::activate::<winrt::IInkRecognizerContainer>(INK_RECOGNIZER_CONTAINER).map_err(platform)?;
            let installed = object(recognizer.get_recognizers()).map_err(platform)?;
            let count = installed.size().map_err(platform)?;
            if count == 0 {
                return Err(InkError::Platform(
                    "No Windows ink recognizer is installed".into(),
                ));
            }
            if let Some(name) = recognizer_name {
                let mut found = false;
                for index in 0..count {
                    let candidate = object(installed.get_at(index)).map_err(platform)?;
                    if take_string(candidate.name().map_err(platform)?) == name {
                        recognizer
                            .set_default_recognizer(Some(&candidate))
                            .map_err(platform)?;
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(InkError::Platform(format!(
                        "Windows ink recognizer is not installed: {name}"
                    )));
                }
            }
            let operation = object(recognizer.recognize_async(Some(&native), winrt::InkRecognitionTarget::All))
                .map_err(platform)?;
            let recognized = object(wait(operation).await).map_err(platform)?;
            let mut result = Vec::new();
            for index in 0..recognized.size().map_err(platform)? {
                let group = object(recognized.get_at(index)).map_err(platform)?;
                let bounds = group.bounding_rect().map_err(platform)?;
                let candidates = object(group.get_text_candidates()).map_err(platform)?;
                let mut text_candidates = Vec::new();
                for index in 0..candidates.size().map_err(platform)? {
                    text_candidates.push(take_string(candidates.get_at(index).map_err(platform)?));
                }
                let native_strokes = object(group.get_strokes()).map_err(platform)?;
                let mut stroke_ids = Vec::new();
                for index in 0..native_strokes.size().map_err(platform)? {
                    let id = stroke_id(&object(native_strokes.get_at(index)).map_err(platform)?)?;
                    if let Some(id) = identities.get(&id) {
                        stroke_ids.push(*id);
                    }
                }
                result.push(WindowsInkRecognitionResult {
                    bounding_rect: Rect::new(
                        bounds.x as f64,
                        bounds.y as f64,
                        bounds.width as f64,
                        bounds.height as f64,
                    ),
                    text_candidates,
                    stroke_ids,
                });
            }
            Ok(result)
        })
    }

    fn native_container(&self, strokes: &[Rc<InkStroke>]) -> Result<NativeContainer, InkError> {
        validate_export(strokes, self.limits, self.allow_twist_loss)?;
        let container = new_container()?;
        let builder = winrt::activate::<winrt::IInkStrokeBuilder>(INK_STROKE_BUILDER).map_err(platform)?;
        let builder2 = builder.cast::<winrt::IInkStrokeBuilder2>().map_err(platform)?;
        let point_factory = winrt::factory::<winrt::IInkPointFactory2>(INK_POINT).map_err(platform)?;
        for stroke in strokes {
            let attributes = to_native_attributes(stroke.drawing_attributes())?;
            builder
                .set_default_drawing_attributes(Some(&attributes))
                .map_err(platform)?;
            let mut points = Vec::with_capacity(stroke.ink_points().len());
            for point in stroke.ink_points() {
                points.push(
                    object(point_factory.create_ink_point_with_tilt_and_timestamp(
                        to_native_point(point.position)?,
                        point.pressure,
                        point.tilt_x,
                        point.tilt_y,
                        point.timestamp,
                    ))
                    .map_err(platform)?,
                );
            }
            let points = winrt::InkPointIterable::new(points);
            // Keep raw points and the editable point transform as distinct values.
            let native = object(builder2.create_stroke_from_ink_points(Some(&points), IDENTITY))
                .map_err(platform)?;
            let point_transform = to_native_matrix(stroke.point_transform())?;
            native
                .cast::<winrt::IInkStroke2>()
                .and_then(|native| native.set_point_transform(point_transform))
                .map_err(platform)?;
            native.set_selected(to_abi(stroke.selected())).map_err(platform)?;
            container.add_stroke(Some(&native)).map_err(platform)?;
        }
        Ok(container)
    }
}

impl IInkPlatformServices for WindowsInkServices {
    fn save(
        &self,
        strokes: Vec<Rc<InkStroke>>,
        format: InkPersistenceFormat,
    ) -> InkServiceFuture<Vec<u8>> {
        let limits = self.limits;
        if format == InkPersistenceFormat::Portable {
            // The portable branch remains exact, including twist and identities.
            let container = InkStrokeContainer::new();
            let result = container
                .add_strokes(strokes)
                .and_then(|()| container.save_with_limits(limits));
            return Box::pin(async move { result });
        }
        let native = self.native_container(&strokes);
        Box::pin(async move {
            let native = native?;
            let stream = winrt::activate::<winrt::IRandomAccessStream>(IN_MEMORY_RANDOM_ACCESS_STREAM).map_err(platform)?;
            let output = object(stream.get_output_stream_at(0)).map_err(platform)?;
            match format {
                InkPersistenceFormat::Isf => {
                    let operation = native
                        .cast::<winrt::IInkStrokeContainer3>()
                        .and_then(|native| object(native.save_with_format_async(Some(&output), winrt::InkPersistenceFormat::Isf)))
                        .map_err(platform)?;
                    wait(operation).await.map_err(platform)?;
                }
                InkPersistenceFormat::GifWithEmbeddedIsf => {
                    // GIF rasterization is also bounded: at most a 16K side and
                    // a BGRA backing surface no larger than the byte budget.
                    validate_gif_bounds(&native, limits)?;
                    let operation = object(native.save_async(Some(&output))).map_err(platform)?;
                    wait(operation).await.map_err(platform)?;
                }
                InkPersistenceFormat::Portable => unreachable!(),
            }
            let size = stream.size().map_err(platform)?;
            if size > limits.max_bytes as u64 || size > u32::MAX as u64 {
                return Err(InkError::LimitExceeded);
            }
            let input = object(stream.get_input_stream_at(0)).map_err(platform)?;
            let reader = winrt::factory::<winrt::IDataReaderFactory>(DATA_READER)
                .and_then(|factory| object(factory.create_data_reader(Some(&input))))
                .map_err(platform)?;
            let operation = object(reader.load_async(size as u32)).map_err(platform)?;
            let loaded = wait(operation).await.map_err(platform)?;
            if loaded as u64 != size {
                return Err(InkError::InvalidData("Truncated native ink stream"));
            }
            let mut data = vec![0; size as usize];
            // SAFETY: the buffer has room for the count of bytes passed.
            unsafe { reader.read_bytes(data.len() as u32, data.as_mut_ptr()) }.map_err(platform)?;
            Ok(data)
        })
    }

    fn load(&self, data: Vec<u8>) -> InkServiceFuture<Vec<Rc<InkStroke>>> {
        let limits = self.limits;
        Box::pin(async move {
            if data.len() > limits.max_bytes || data.len() > u32::MAX as usize {
                return Err(InkError::LimitExceeded);
            }
            let stream = winrt::activate::<winrt::IRandomAccessStream>(IN_MEMORY_RANDOM_ACCESS_STREAM).map_err(platform)?;
            let output = object(stream.get_output_stream_at(0)).map_err(platform)?;
            let writer = winrt::factory::<winrt::IDataWriterFactory>(DATA_WRITER)
                .and_then(|factory| object(factory.create_data_writer(Some(&output))))
                .map_err(platform)?;
            // SAFETY: the buffer holds the count of bytes passed; the writer copies them.
            unsafe { writer.write_bytes(data.len() as u32, data.as_ptr()) }.map_err(platform)?;
            let operation = object(writer.store_async()).map_err(platform)?;
            let stored = wait(operation).await.map_err(platform)?;
            if stored as usize != data.len() {
                return Err(InkError::InvalidData("Truncated native ink stream"));
            }
            let native = new_container()?;
            let input = object(stream.get_input_stream_at(0)).map_err(platform)?;
            let operation = object(native.load_async(Some(&input))).map_err(platform)?;
            wait(operation).await.map_err(platform)?;
            from_native_container(&native, limits)
        })
    }

    fn copy_to_clipboard(&self, strokes: &[Rc<InkStroke>]) -> Result<(), InkError> {
        ensure_clipboard_apartment()?;
        let native = self.native_container(strokes)?;
        // The trait receives the strokes chosen for copying. The native method
        // only copies Selected strokes, so select the temporary native copies.
        let selected = object(native.get_strokes()).map_err(platform)?;
        for index in 0..selected.size().map_err(platform)? {
            object(selected.get_at(index))
                .and_then(|stroke| stroke.set_selected(to_abi(true)))
                .map_err(platform)?;
        }
        native.copy_selected_to_clipboard().map_err(platform)
    }

    fn can_paste(&self) -> Result<bool, InkError> {
        ensure_clipboard_apartment()?;
        new_container()?
            .can_paste_from_clipboard()
            .map(from_abi)
            .map_err(platform)
    }

    fn paste(&self, position: Point) -> Result<Vec<Rc<InkStroke>>, InkError> {
        ensure_clipboard_apartment()?;
        let position = to_native_point(position)?;
        let native = new_container()?;
        native.paste_from_clipboard(position).map_err(platform)?;
        from_native_container(&native, self.limits)
    }
}

fn platform(error: HResult) -> InkError {
    InkError::Platform(format!("Windows ink: {error}"))
}

/// The identity matrix of the ink engine.
const IDENTITY: winrt::Matrix3x2 = winrt::Matrix3x2 {
    m11: 1.0,
    m12: 0.0,
    m21: 0.0,
    m22: 1.0,
    m31: 0.0,
    m32: 0.0,
};

/// A new, empty `InkStrokeContainer` of the ink engine.
fn new_container() -> Result<NativeContainer, InkError> {
    winrt::activate::<winrt::IInkStrokeContainer>(INK_STROKE_CONTAINER).map_err(platform)
}

/// `InkStroke.Id` (`IInkStroke3`).
fn stroke_id(stroke: &NativeStroke) -> Result<u32, InkError> {
    stroke
        .cast::<winrt::IInkStroke3>()
        .and_then(|stroke| stroke.id())
        .map_err(platform)
}

fn ensure_clipboard_apartment() -> Result<(), InkError> {
    if matches!(
        co_get_apartment_type(),
        Some(APTTYPE_STA) | Some(APTTYPE_MAINSTA)
    ) {
        Ok(())
    } else {
        Err(InkError::Platform(
            "Windows ink clipboard requires a single-threaded COM apartment; call it on the FerroUI Windows UI thread".into(),
        ))
    }
}

fn validate_export(
    strokes: &[Rc<InkStroke>],
    limits: InkPersistenceLimits,
    allow_twist_loss: bool,
) -> Result<(), InkError> {
    if strokes.len() > limits.max_strokes {
        return Err(InkError::LimitExceeded);
    }
    let mut total = 0usize;
    for stroke in strokes {
        let points = stroke.ink_points();
        total = total
            .checked_add(points.len())
            .ok_or(InkError::LimitExceeded)?;
        if points.len() > limits.max_points_per_stroke || total > limits.max_total_points {
            return Err(InkError::LimitExceeded);
        }
        if !allow_twist_loss && points.iter().any(|point| point.twist != 0.0) {
            return Err(InkError::InvalidInput("Windows InkPoint cannot preserve twist; use Portable or explicitly allow_twist_loss"));
        }
    }
    Ok(())
}

fn validate_gif_bounds(
    native: &NativeContainer,
    limits: InkPersistenceLimits,
) -> Result<(), InkError> {
    let bounds = native.bounding_rect().map_err(platform)?;
    let width = (bounds.width as f64).ceil().max(1.0);
    let height = (bounds.height as f64).ceil().max(1.0);
    if !width.is_finite()
        || !height.is_finite()
        || width > 16_384.0
        || height > 16_384.0
        || width * height * 4.0 > limits.max_bytes as f64
    {
        return Err(InkError::LimitExceeded);
    }
    Ok(())
}

fn to_native_point(point: Point) -> Result<winrt::Point, InkError> {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || point.x.abs() > 1.0e12
        || point.y.abs() > 1.0e12
    {
        return Err(InkError::InvalidPoint);
    }
    Ok(winrt::Point {
        x: point.x as f32,
        y: point.y as f32,
    })
}

fn to_native_matrix(matrix: Matrix) -> Result<winrt::Matrix3x2, InkError> {
    let entries = [
        matrix.m11, matrix.m12, matrix.m21, matrix.m22, matrix.m31, matrix.m32,
    ];
    if matrix.m13 != 0.0
        || matrix.m23 != 0.0
        || matrix.m33 != 1.0
        || entries.iter().any(|v| !v.is_finite() || v.abs() > 1.0e12)
    {
        return Err(InkError::InvalidTransform);
    }
    Ok(winrt::Matrix3x2 {
        m11: matrix.m11 as f32,
        m12: matrix.m12 as f32,
        m21: matrix.m21 as f32,
        m22: matrix.m22 as f32,
        m31: matrix.m31 as f32,
        m32: matrix.m32 as f32,
    })
}

fn from_native_matrix(matrix: winrt::Matrix3x2) -> Matrix {
    Matrix::new(
        matrix.m11 as f64,
        matrix.m12 as f64,
        matrix.m21 as f64,
        matrix.m22 as f64,
        matrix.m31 as f64,
        matrix.m32 as f64,
    )
}

fn to_native_attributes(attributes: InkDrawingAttributes) -> Result<NativeAttributes, InkError> {
    attributes.validate()?;
    let native = match attributes.kind {
        InkDrawingAttributesKind::Default => winrt::activate::<winrt::IInkDrawingAttributes>(INK_DRAWING_ATTRIBUTES),
        InkDrawingAttributesKind::Pencil => winrt::factory::<winrt::IInkDrawingAttributesStatics>(INK_DRAWING_ATTRIBUTES)
            .and_then(|statics| object(statics.create_for_pencil())),
    }
    .map_err(platform)?;
    let native2 = native.cast::<winrt::IInkDrawingAttributes2>().map_err(platform)?;
    native
        .set_color(winrt::Color {
            a: attributes.color.a,
            r: attributes.color.r,
            g: attributes.color.g,
            b: attributes.color.b,
        })
        .map_err(platform)?;
    native
        .set_size(winrt::Size {
            width: attributes.size.width as f32,
            height: attributes.size.height as f32,
        })
        .map_err(platform)?;
    // Avoid redundant setters: pencil has fixed characteristics in some OS
    // versions. If an actual requested change is unsupported, report its HRESULT.
    let pen_tip = match attributes.pen_tip {
        PenTipShape::Circle => winrt::PenTipShape::Circle,
        PenTipShape::Rectangle => winrt::PenTipShape::Rectangle,
    };
    if native.pen_tip().map_err(platform)? != pen_tip {
        native.set_pen_tip(pen_tip).map_err(platform)?;
    }
    let transform = to_native_matrix(attributes.pen_tip_transform)?;
    if native2.pen_tip_transform().map_err(platform)? != transform {
        native2.set_pen_tip_transform(transform).map_err(platform)?;
    }
    if from_abi(native.ignore_pressure().map_err(platform)?) != attributes.ignore_pressure {
        native
            .set_ignore_pressure(to_abi(attributes.ignore_pressure))
            .map_err(platform)?;
    }
    if from_abi(native.fit_to_curve().map_err(platform)?) != attributes.fit_to_curve {
        native
            .set_fit_to_curve(to_abi(attributes.fit_to_curve))
            .map_err(platform)?;
    }
    if from_abi(native2.draw_as_highlighter().map_err(platform)?) != attributes.draw_as_highlighter {
        native2
            .set_draw_as_highlighter(to_abi(attributes.draw_as_highlighter))
            .map_err(platform)?;
    }
    let native4 = native.cast::<winrt::IInkDrawingAttributes4>().map_err(platform)?;
    if from_abi(native4.ignore_tilt().map_err(platform)?) != attributes.ignore_tilt {
        native4
            .set_ignore_tilt(to_abi(attributes.ignore_tilt))
            .map_err(platform)?;
    }
    if attributes.kind == InkDrawingAttributesKind::Pencil {
        native
            .cast::<winrt::IInkDrawingAttributes3>()
            .and_then(|native| object(native.pencil_properties()))
            .and_then(|pencil| pencil.set_opacity(to_native_pencil_opacity(attributes.pencil_opacity)))
            .map_err(platform)?;
    }
    Ok(native)
}

/// The range of `InkDrawingAttributesPencilProperties.Opacity` (Windows's default is 1.0).
const NATIVE_PENCIL_OPACITY: (f64, f64) = (0.01, 5.0);

/// The opacity of a pencil for Windows: the portable range is 0 to 1, which Windows reads on the
/// same scale; 0 is below its minimum and is sent as the minimum.
fn to_native_pencil_opacity(opacity: f64) -> f64 {
    opacity.clamp(NATIVE_PENCIL_OPACITY.0, NATIVE_PENCIL_OPACITY.1)
}

/// The portable opacity of a pencil of Windows: the values above 1 (a pencil darker than its
/// colour, which the portable renderer cannot draw) are read as 1, so a document of Windows loads
/// instead of failing validation.
fn from_native_pencil_opacity(opacity: f64) -> f64 {
    opacity.clamp(0.0, 1.0)
}

fn from_native_attributes(native: &NativeAttributes) -> Result<InkDrawingAttributes, InkError> {
    let native2 = native.cast::<winrt::IInkDrawingAttributes2>().map_err(platform)?;
    let native3 = native.cast::<winrt::IInkDrawingAttributes3>().map_err(platform)?;
    let native4 = native.cast::<winrt::IInkDrawingAttributes4>().map_err(platform)?;
    let color = native.color().map_err(platform)?;
    let size = native.size().map_err(platform)?;
    let kind = match native3.kind().map_err(platform)? {
        winrt::InkDrawingAttributesKind::Default => InkDrawingAttributesKind::Default,
        winrt::InkDrawingAttributesKind::Pencil => InkDrawingAttributesKind::Pencil,
        _ => return Err(InkError::UnsupportedFormat),
    };
    let attributes = InkDrawingAttributes {
        color: Color::from_argb(color.a, color.r, color.g, color.b),
        size: Size::new(size.width as f64, size.height as f64),
        pen_tip: match native.pen_tip().map_err(platform)? {
            winrt::PenTipShape::Circle => PenTipShape::Circle,
            winrt::PenTipShape::Rectangle => PenTipShape::Rectangle,
            _ => return Err(InkError::UnsupportedFormat),
        },
        pen_tip_transform: from_native_matrix(native2.pen_tip_transform().map_err(platform)?),
        draw_as_highlighter: from_abi(native2.draw_as_highlighter().map_err(platform)?),
        ignore_pressure: from_abi(native.ignore_pressure().map_err(platform)?),
        fit_to_curve: from_abi(native.fit_to_curve().map_err(platform)?),
        ignore_tilt: from_abi(native4.ignore_tilt().map_err(platform)?),
        kind,
        pencil_opacity: if kind == InkDrawingAttributesKind::Pencil {
            object(native3.pencil_properties())
                .and_then(|pencil| pencil.opacity())
                .map(from_native_pencil_opacity)
                .map_err(platform)?
        } else {
            1.0
        },
    };
    attributes.validate()?;
    Ok(attributes)
}

fn from_native_container(
    native: &NativeContainer,
    limits: InkPersistenceLimits,
) -> Result<Vec<Rc<InkStroke>>, InkError> {
    let strokes = object(native.get_strokes()).map_err(platform)?;
    let count = strokes.size().map_err(platform)? as usize;
    if count > limits.max_strokes {
        return Err(InkError::LimitExceeded);
    }
    let mut result = Vec::with_capacity(count);
    let mut total = 0usize;
    for index in 0..count {
        let native = object(strokes.get_at(index as u32)).map_err(platform)?;
        result.push(from_native_stroke(&native, limits, &mut total)?);
    }
    Ok(result)
}

fn from_native_stroke(
    native: &NativeStroke,
    limits: InkPersistenceLimits,
    total: &mut usize,
) -> Result<Rc<InkStroke>, InkError> {
    let native2 = native.cast::<winrt::IInkStroke2>().map_err(platform)?;
    let source = object(native2.get_ink_points()).map_err(platform)?;
    let count = source.size().map_err(platform)? as usize;
    *total = total.checked_add(count).ok_or(InkError::LimitExceeded)?;
    if count > limits.max_points_per_stroke || *total > limits.max_total_points {
        return Err(InkError::LimitExceeded);
    }
    let mut points = Vec::with_capacity(count);
    for index in 0..count {
        let point = object(source.get_at(index as u32)).map_err(platform)?;
        let point2 = point.cast::<winrt::IInkPoint2>().map_err(platform)?;
        let position = point.position().map_err(platform)?;
        points.push(InkPoint {
            position: Point::new(position.x as f64, position.y as f64),
            pressure: point.pressure().map_err(platform)?,
            tilt_x: point2.tilt_x().map_err(platform)?,
            tilt_y: point2.tilt_y().map_err(platform)?,
            twist: 0.0,
            timestamp: point2.timestamp().map_err(platform)?,
        });
    }
    let stroke = InkStroke::new(
        points,
        from_native_attributes(&object(native.drawing_attributes()).map_err(platform)?)?,
    )?;
    stroke.set_point_transform(from_native_matrix(
        native2.point_transform().map_err(platform)?,
    ))?;
    stroke.set_selected(from_abi(native.selected().map_err(platform)?));
    Ok(stroke)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    use std::time::{Duration, Instant};

    // The backend chooses MTA for a thread other than the dispatcher thread.
    // Native async completion can then wake this test without an STA message
    // pump. Objects and Rc values are constructed and used on that one worker.
    fn native_thread(test: impl FnOnce() + Send + 'static) {
        let _dispatcher = ferroui_base::threading::Dispatcher::ui_thread();
        std::thread::spawn(test).join().unwrap();
    }

    // Clipboard calls are synchronous and require STA activation. Reuse the
    // platform's safe COM initialization wrappers; all native/Rc values are
    // dropped before the paired COM uninitialization on this same worker.
    fn native_sta_thread(test: impl FnOnce() + Send + 'static) {
        native_thread(move || {
            use ferroui_win32::interop::unmanaged_methods::{
                co_initialize_apartment_threaded, co_uninitialize,
            };
            let result = co_initialize_apartment_threaded();
            assert!(
                result >= 0,
                "STA initialization failed: 0x{:08X}",
                result as u32
            );
            struct Apartment;
            impl Drop for Apartment {
                fn drop(&mut self) {
                    co_uninitialize();
                }
            }
            let _apartment = Apartment;
            test();
        });
    }

    // A test-only executor. Production uses the application's local executor.
    // A timeout turns a broken async completion into a useful CI failure.
    fn complete<T>(future: impl Future<Output = T>) -> T {
        struct Notify(std::thread::Thread);
        impl Wake for Notify {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
            fn wake_by_ref(self: &Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(Notify(std::thread::current())));
        let mut context = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Poll::Ready(result) = future.as_mut().poll(&mut context) {
                return result;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "Windows ink async operation timed out"
            );
            std::thread::park_timeout(remaining);
        }
    }

    fn fixture_strokes() -> Vec<Rc<InkStroke>> {
        let mut points = Vec::new();
        for (index, (x, y, pressure)) in [(10.0, 20.0, 0.25), (30.0, 40.0, 0.5), (60.0, 30.0, 0.75)]
            .into_iter()
            .enumerate()
        {
            let mut point = InkPoint::new(Point::new(x, y), pressure).unwrap();
            point.tilt_x = 10.0;
            point.tilt_y = -5.0;
            point.timestamp = 1_000_000 + index as u64 * 10_000;
            points.push(point);
        }
        let mut pen = InkDrawingAttributes::default();
        pen.color = Color::from_rgb(12, 34, 56);
        pen.size = Size::new(4.0, 8.0);
        pen.pen_tip = PenTipShape::Rectangle;
        pen.pen_tip_transform = Matrix::new(1.0, 0.25, 0.0, 1.0, 0.0, 0.0);
        pen.fit_to_curve = false;
        let first = InkStroke::new(points.clone(), pen).unwrap();
        first
            .set_point_transform(Matrix::new(1.25, 0.0, 0.0, 0.75, 30.0, 40.0))
            .unwrap();
        first.set_selected(true);

        let mut highlighter = InkDrawingAttributes::default();
        highlighter.color = Color::from_rgb(255, 230, 0);
        highlighter.draw_as_highlighter = true;
        highlighter.ignore_pressure = true;
        highlighter.ignore_tilt = true;
        highlighter.pen_tip = PenTipShape::Rectangle;
        highlighter.size = Size::new(5.0, 15.0);
        let second = InkStroke::new(points.clone(), highlighter).unwrap();

        let mut pencil = InkDrawingAttributes::create_for_pencil();
        pencil.color = Color::from_rgb(120, 30, 90);
        pencil.size = Size::new(6.0, 6.0);
        pencil.pencil_opacity = 0.4;
        let third = InkStroke::new(points, pencil).unwrap();
        third
            .set_point_transform(Matrix::create_translation(100.0, 0.0))
            .unwrap();
        vec![first, second, third]
    }

    /// The strokes of a native container, through its own getters.
    fn native_strokes(container: &NativeContainer) -> Vec<NativeStroke> {
        let strokes = object(container.get_strokes()).unwrap();
        (0..strokes.size().unwrap()).map(|index| object(strokes.get_at(index)).unwrap()).collect()
    }

    /// The points of a native stroke, through its own getters.
    fn native_points(stroke: &NativeStroke) -> Vec<ComPtr<winrt::IInkPoint>> {
        let points = object(stroke.cast::<winrt::IInkStroke2>().unwrap().get_ink_points()).unwrap();
        (0..points.size().unwrap()).map(|index| object(points.get_at(index)).unwrap()).collect()
    }

    fn native_point_transform(stroke: &NativeStroke) -> winrt::Matrix3x2 {
        stroke.cast::<winrt::IInkStroke2>().unwrap().point_transform().unwrap()
    }

    fn near(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected} +/- {tolerance}, got {actual}"
        );
    }

    #[test]
    fn affine_matrix_layout_matches_windows() {
        let matrix = Matrix::new(2.0, 3.0, 4.0, 5.0, 6.0, 7.0);
        let native = to_native_matrix(matrix).unwrap();
        assert_eq!(
            [native.m11, native.m12, native.m21, native.m22, native.m31, native.m32],
            [2.0, 3.0, 4.0, 5.0, 6.0, 7.0]
        );
        assert_eq!(from_native_matrix(native), matrix);
        let mut perspective = matrix;
        perspective.m13 = 1.0;
        assert_eq!(
            to_native_matrix(perspective),
            Err(InkError::InvalidTransform)
        );
    }

    #[test]
    fn pencil_opacity_maps_between_the_portable_and_the_windows_ranges() {
        assert_eq!(to_native_pencil_opacity(0.0), 0.01);
        assert_eq!(to_native_pencil_opacity(0.4), 0.4);
        assert_eq!(to_native_pencil_opacity(1.0), 1.0);
        assert_eq!(from_native_pencil_opacity(0.01), 0.01);
        assert_eq!(from_native_pencil_opacity(1.0), 1.0);
        assert_eq!(from_native_pencil_opacity(5.0), 1.0);
    }

    #[test]
    fn native_pencil_opacity_outside_the_portable_range_loads() {
        native_thread(|| {
            let services = WindowsInkServices::new().expect("Windows ink activation");
            let mut transparent = InkDrawingAttributes::create_for_pencil();
            transparent.pencil_opacity = 0.0;
            let point = InkPoint::new(Point::new(4.0, 5.0), 0.5).unwrap();
            let stroke = InkStroke::new(vec![point], transparent).unwrap();
            let native = services.native_container(&[stroke]).unwrap();
            let native_stroke = native_strokes(&native).remove(0);
            let pencil = |stroke: &NativeStroke| {
                object(stroke.drawing_attributes())
                    .and_then(|attributes| attributes.cast::<winrt::IInkDrawingAttributes3>())
                    .and_then(|attributes| object(attributes.pencil_properties()))
                    .unwrap()
            };
            // The portable 0 is sent as the minimum of Windows.
            near(pencil(&native_stroke).opacity().unwrap(), 0.01, 1e-6);
            for (native_opacity, portable_opacity) in [(1.0, 1.0), (5.0, 1.0)] {
                let attributes = object(native_stroke.drawing_attributes()).unwrap();
                object(attributes.cast::<winrt::IInkDrawingAttributes3>().unwrap().pencil_properties())
                    .unwrap()
                    .set_opacity(native_opacity)
                    .unwrap();
                native_stroke.set_drawing_attributes(Some(&attributes)).unwrap();
                near(pencil(&native_stroke).opacity().unwrap(), native_opacity, 1e-6);
                let imported = from_native_container(&native, InkPersistenceLimits::DEFAULT).unwrap();
                assert_eq!(imported[0].drawing_attributes().pencil_opacity, portable_opacity);
            }
        });
    }

    #[test]
    fn native_export_requires_explicit_twist_loss() {
        let mut point = InkPoint::new(Point::new(1.0, 2.0), 0.5).unwrap();
        point.twist = 30.0;
        let stroke = InkStroke::new(vec![point], InkDrawingAttributes::default()).unwrap();
        let strokes = [stroke];
        assert!(validate_export(&strokes, InkPersistenceLimits::DEFAULT, false).is_err());
        assert!(validate_export(&strokes, InkPersistenceLimits::DEFAULT, true).is_ok());
        let limits = InkPersistenceLimits {
            max_total_points: 0,
            ..InkPersistenceLimits::DEFAULT
        };
        assert_eq!(
            validate_export(&strokes, limits, true),
            Err(InkError::LimitExceeded)
        );
    }

    #[test]
    fn clipboard_position_must_be_finite_and_bounded() {
        assert_eq!(
            to_native_point(Point::new(f64::NAN, 1.0)),
            Err(InkError::InvalidPoint)
        );
        assert_eq!(
            to_native_point(Point::new(1.0e13, 1.0)),
            Err(InkError::InvalidPoint)
        );
        assert_eq!(
            to_native_point(Point::new(1.5, -2.0)).unwrap(),
            winrt::Point { x: 1.5, y: -2.0 }
        );
    }

    #[test]
    fn native_conversion_preserves_supported_attributes_samples_and_transform() {
        native_thread(|| {
            let services = WindowsInkServices::new().expect("Windows ink activation");
            let source = fixture_strokes();
            let native = services.native_container(&source).unwrap();
            // Assert through the actual Windows getters as well as our inverse
            // adapter, so a symmetric conversion bug cannot pass a round trip.
            let first = native_strokes(&native).remove(0);
            assert!(from_abi(first.selected().unwrap()));
            let transform = native_point_transform(&first);
            assert_eq!(
                [transform.m11, transform.m22, transform.m31, transform.m32],
                [1.25, 0.75, 30.0, 40.0]
            );
            let color = object(first.drawing_attributes()).unwrap().color().unwrap();
            assert_eq!([color.a, color.r, color.g, color.b], [255, 12, 34, 56]);
            let point = native_points(&first).remove(0);
            near(point.position().unwrap().x as f64, 10.0, 0.05);
            near(point.position().unwrap().y as f64, 20.0, 0.05);
            let imported = from_native_container(&native, InkPersistenceLimits::DEFAULT).unwrap();
            assert_eq!(imported.len(), source.len());
            for (expected, actual) in source.iter().zip(&imported) {
                assert_ne!(actual.id(), expected.id());
                assert_eq!(actual.selected(), expected.selected());
                assert_eq!(actual.point_transform(), expected.point_transform());
                assert_eq!(actual.drawing_attributes(), expected.drawing_attributes());
                assert_eq!(actual.ink_points().len(), expected.ink_points().len());
                for (actual, expected) in actual.ink_points().iter().zip(expected.ink_points()) {
                    near(actual.position.x, expected.position.x, 0.05);
                    near(actual.position.y, expected.position.y, 0.05);
                    near(actual.pressure as f64, expected.pressure as f64, 0.005);
                    near(actual.tilt_x as f64, expected.tilt_x as f64, 0.1);
                    near(actual.tilt_y as f64, expected.tilt_y as f64, 0.1);
                    assert_eq!(actual.twist, 0.0);
                }
            }
        });
    }

    #[test]
    fn native_move_selected_uses_relative_translation_like_portable_container() {
        native_thread(|| {
            let services = WindowsInkServices::new().expect("Windows ink activation");
            let source = fixture_strokes();
            let native = services.native_container(&source).unwrap();
            let strokes = native_strokes(&native);
            let selected = &strokes[0];
            let unselected = &strokes[1];
            let before = selected.bounding_rect().unwrap();
            let untouched = unselected.bounding_rect().unwrap();
            // The Learn parameter description calls this a destination, while
            // the API names it translation. Pin the installed OS behavior with
            // nonzero initial coordinates instead of assuming either wording.
            let moved = native
                .move_selected(winrt::Point { x: 25.0, y: -10.0 })
                .unwrap();
            let after = selected.bounding_rect().unwrap();
            near(after.x as f64, before.x as f64 + 25.0, 0.01);
            near(after.y as f64, before.y as f64 - 10.0, 0.01);
            assert_eq!(moved, after);
            assert_eq!(unselected.bounding_rect().unwrap(), untouched);

            let portable = InkStrokeContainer::new();
            portable.add_strokes(source).unwrap();
            let first = portable.get_strokes()[0].clone();
            let before = first.bounding_rect();
            let moved = portable.move_selected(Point::new(25.0, -10.0)).unwrap();
            let after = first.bounding_rect();
            near(after.x, before.x + 25.0, 1e-10);
            near(after.y, before.y - 10.0, 1e-10);
            assert_eq!(moved, after);
        });
    }

    #[test]
    fn native_isf_and_gif_round_trip_preserve_strokes_attributes_and_geometry() {
        native_thread(|| {
            let services = WindowsInkServices::new().expect("Windows ink activation");
            let source = fixture_strokes();
            for format in [
                InkPersistenceFormat::Isf,
                InkPersistenceFormat::GifWithEmbeddedIsf,
            ] {
                let bytes = complete(services.save(source.clone(), format)).unwrap();
                assert!(!bytes.is_empty());
                assert!(!bytes.starts_with(b"FERROINK"));
                let is_gif = bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a");
                assert_eq!(is_gif, format == InkPersistenceFormat::GifWithEmbeddedIsf);
                let imported = complete(services.load(bytes)).unwrap();
                assert_eq!(imported.len(), source.len());
                for (expected, actual) in source.iter().zip(&imported) {
                    let a = actual.drawing_attributes();
                    let e = expected.drawing_attributes();
                    assert_eq!(a.color, e.color);
                    assert_eq!(a.kind, e.kind);
                    assert_eq!(a.pen_tip, e.pen_tip);
                    assert_eq!(a.draw_as_highlighter, e.draw_as_highlighter);
                    assert_eq!(a.ignore_pressure, e.ignore_pressure);
                    assert_eq!(a.fit_to_curve, e.fit_to_curve);
                    near(a.size.width, e.size.width, 0.1);
                    near(a.size.height, e.size.height, 0.1);
                    near(a.pencil_opacity, e.pencil_opacity, 0.01);
                    assert_eq!(actual.ink_points().len(), expected.ink_points().len());
                    // ISF can bake a point transform into coordinates. Compare
                    // visible points rather than requiring the same factorization.
                    for (a, e) in actual.ink_points().iter().zip(expected.ink_points()) {
                        let a_position = actual.point_transform().transform(a.position);
                        let e_position = expected.point_transform().transform(e.position);
                        near(a_position.x, e_position.x, 0.2);
                        near(a_position.y, e_position.y, 0.2);
                        near(a.pressure as f64, e.pressure as f64, 0.005);
                    }
                    let a = actual.bounding_rect();
                    let e = expected.bounding_rect();
                    near(a.x, e.x, 0.5);
                    near(a.y, e.y, 0.5);
                    near(a.width, e.width, 0.5);
                    near(a.height, e.height, 0.5);
                }
            }
        });
    }

    #[test]
    fn malformed_native_load_preserves_existing_container() {
        native_thread(|| {
            let services = Rc::new(WindowsInkServices::new().expect("Windows ink activation"));
            let container = InkStrokeContainer::new();
            let source = fixture_strokes();
            container.add_strokes(source.clone()).unwrap();
            container.set_platform_services(Some(services));
            for invalid in [
                Vec::new(),
                b"This is not an ISF or GIF ink document".to_vec(),
            ] {
                let error = complete(container.load_async(invalid));
                assert!(error.is_err());
                let retained = container.get_strokes();
                assert_eq!(retained.len(), source.len());
                assert!(retained.iter().zip(&source).all(|(a, b)| Rc::ptr_eq(a, b)));
            }
        });
    }

    #[test]
    fn real_native_save_rejects_twist_until_application_opts_in() {
        native_thread(|| {
            let services = WindowsInkServices::new().expect("Windows ink activation");
            let mut point = InkPoint::new(Point::new(4.0, 5.0), 0.5).unwrap();
            point.twist = 25.0;
            let stroke = InkStroke::new(vec![point], InkDrawingAttributes::default()).unwrap();
            assert!(matches!(
                complete(services.save(vec![stroke.clone()], InkPersistenceFormat::Isf)),
                Err(InkError::InvalidInput(_))
            ));
            let services = services.allow_twist_loss(true);
            let bytes = complete(services.save(vec![stroke], InkPersistenceFormat::Isf)).unwrap();
            let imported = complete(services.load(bytes)).unwrap();
            assert_eq!(imported.len(), 1);
            assert!(imported[0].ink_points().iter().all(|p| p.twist == 0.0));
        });
    }

    #[test]
    fn installed_recognizer_enumeration_is_optional() {
        native_thread(|| {
            let services = WindowsInkServices::new().expect("Windows ink activation");
            match services.recognizers() {
                Ok(names) => {
                    println!("Windows installed ink recognizers: {names:?}");
                    assert!(names.iter().all(|name| !name.is_empty()));
                }
                Err(InkError::Platform(error)) => {
                    println!("Optional handwriting recognizer service unavailable: {error}");
                }
                Err(error) => panic!("Unexpected recognizer enumeration error: {error}"),
            }
        });
    }

    /// Explicit opt-in: this replaces the current user's system clipboard.
    /// Run only in an isolated interactive Windows test session with:
    /// `cargo test -p ferroui-controls-inking native_clipboard_round_trip -- --ignored`.
    #[test]
    #[ignore = "replaces the system clipboard; requires an isolated interactive Windows session"]
    fn native_clipboard_round_trip() {
        native_sta_thread(|| {
            let services = WindowsInkServices::new().expect("Windows ink activation");
            let source = fixture_strokes();
            services.copy_to_clipboard(&source).unwrap();
            assert!(services.can_paste().unwrap());
            let position = winrt::Point { x: 300.0, y: 200.0 };
            let reference = new_container().unwrap();
            let native_paste_bounds = reference.paste_from_clipboard(position).unwrap();
            let expected = native_strokes(&reference);
            assert_eq!(expected.len(), source.len());
            println!(
                "Native paste bounds at (300, 200): {native_paste_bounds:?}; collection: {:?}",
                reference.bounding_rect().unwrap()
            );
            let pasted = services.paste(Point::new(300.0, 200.0)).unwrap();
            assert_eq!(pasted.len(), source.len());
            for (index, actual) in pasted.iter().enumerate() {
                // Compare with actual native getters, including Selected. The
                // installed OS can return unselected pasted ink despite the
                // Learn remark promising a selected result.
                let native = &expected[index];
                assert_eq!(actual.selected(), from_abi(native.selected().unwrap()));
                assert_eq!(
                    actual.drawing_attributes(),
                    from_native_attributes(&object(native.drawing_attributes()).unwrap()).unwrap()
                );
                let transform = native_point_transform(native);
                let native_points = native_points(native);
                assert_eq!(actual.ink_points().len(), native_points.len());
                for (point_index, point) in actual.ink_points().iter().enumerate() {
                    let native_point = &native_points[point_index];
                    let native_point2 = native_point.cast::<winrt::IInkPoint2>().unwrap();
                    let position = native_point.position().unwrap();
                    let actual_position = actual.point_transform().transform(point.position);
                    // Apply the native matrix directly rather than running it
                    // through the same conversion that the adapter uses.
                    near(
                        actual_position.x,
                        position.x as f64 * transform.m11 as f64
                            + position.y as f64 * transform.m21 as f64
                            + transform.m31 as f64,
                        0.001,
                    );
                    near(
                        actual_position.y,
                        position.x as f64 * transform.m12 as f64
                            + position.y as f64 * transform.m22 as f64
                            + transform.m32 as f64,
                        0.001,
                    );
                    near(
                        point.pressure as f64,
                        native_point.pressure().unwrap() as f64,
                        0.00001,
                    );
                    near(
                        point.tilt_x as f64,
                        native_point2.tilt_x().unwrap() as f64,
                        0.001,
                    );
                    near(
                        point.tilt_y as f64,
                        native_point2.tilt_y().unwrap() as f64,
                        0.001,
                    );
                    assert_eq!(point.twist, 0.0);
                }
                assert!(source.iter().all(|original| original.id() != actual.id()));
            }
            // Native placement uses native rendered bounds. The portable
            // outline is intentionally independent, so verify the requested
            // translation rather than requiring identical nib coverage.
            let origin = services.paste(Point::default()).unwrap();
            for (at_position, at_origin) in pasted.iter().zip(&origin) {
                let position = at_position.bounding_rect();
                let origin = at_origin.bounding_rect();
                near(position.x - origin.x, 300.0, 0.001);
                near(position.y - origin.y, 200.0, 0.001);
                near(position.width, origin.width, 0.001);
                near(position.height, origin.height, 0.001);
            }

            // Pin a separate populated-container reference as well. Its
            // selection behavior must be observed before adapting the public
            // wrapper; an empty-container result alone does not establish
            // whether an earlier selection survives a native paste.
            let populated = services.native_container(&source[..1]).unwrap();
            let sentinel = native_strokes(&populated).remove(0);
            let sentinel_id = stroke_id(&sentinel).unwrap();
            sentinel.set_selected(to_abi(true)).unwrap();
            populated.paste_from_clipboard(position).unwrap();
            let after = native_strokes(&populated);
            assert_eq!(after.len(), source.len() + 1);
            let mut pasted_selection = Vec::new();
            for stroke in &after {
                if stroke_id(stroke).unwrap() != sentinel_id {
                    pasted_selection.push(from_abi(stroke.selected().unwrap()));
                }
            }
            assert_eq!(pasted_selection.len(), source.len());
            println!(
                "Native populated paste selection: old={}, new={pasted_selection:?}",
                from_abi(sentinel.selected().unwrap())
            );
        });
    }
}
