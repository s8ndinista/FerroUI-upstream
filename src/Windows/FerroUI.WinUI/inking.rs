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

use ferroui_base::media::Color;
use ferroui_base::{Matrix, Point, Rect, Size};
use ferroui_controls::inking::{
    IInkPlatformServices, InkDrawingAttributes, InkDrawingAttributesKind, InkError,
    InkPersistenceFormat, InkPersistenceLimits, InkPoint, InkServiceFuture, InkStroke,
    InkStrokeContainer, PenTipShape,
};
use ferroui_microcom::IUnknown;
use ferroui_win32::win_rt::NativeWinRTMethods;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use windows::Foundation::{Point as NativePoint, Size as NativeSize};
use windows::Storage::Streams::{DataReader, DataWriter, InMemoryRandomAccessStream};
use windows::UI::Color as NativeColor;
use windows::UI::Input::Inking::{
    InkDrawingAttributes as NativeAttributes, InkDrawingAttributesKind as NativeAttributesKind,
    InkPersistenceFormat as NativeFormat, InkPoint as NativeInkPoint, InkRecognitionTarget,
    InkRecognizerContainer, InkStroke as NativeStroke, InkStrokeBuilder,
    InkStrokeContainer as NativeContainer, PenTipShape as NativePenTip,
};
use windows_collections::IIterable;
use windows_numerics::Matrix3x2;

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
pub struct WindowsInkServices {
    limits: InkPersistenceLimits,
    allow_twist_loss: bool,
    _ui_thread: PhantomData<Rc<()>>,
}

impl WindowsInkServices {
    pub fn new() -> Result<Self, InkError> {
        NativeWinRTMethods::create_instance::<IUnknown>(
            "Windows.UI.Input.Inking.InkStrokeContainer",
        )
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
        let container = InkRecognizerContainer::new().map_err(platform)?;
        let recognizers = container.GetRecognizers().map_err(platform)?;
        let mut result = Vec::new();
        for index in 0..recognizers.Size().map_err(platform)? {
            result.push(
                recognizers
                    .GetAt(index)
                    .map_err(platform)?
                    .Name()
                    .map_err(platform)?
                    .to_string(),
            );
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
            let source = native.GetStrokes().map_err(platform)?;
            let mut identities = HashMap::with_capacity(strokes.len());
            for (index, stroke) in strokes.iter().enumerate() {
                identities.insert(
                    source
                        .GetAt(index as u32)
                        .map_err(platform)?
                        .Id()
                        .map_err(platform)?,
                    stroke.id(),
                );
            }
            let recognizer = InkRecognizerContainer::new().map_err(platform)?;
            let installed = recognizer.GetRecognizers().map_err(platform)?;
            let count = installed.Size().map_err(platform)?;
            if count == 0 {
                return Err(InkError::Platform(
                    "No Windows ink recognizer is installed".into(),
                ));
            }
            if let Some(name) = recognizer_name {
                let mut found = false;
                for index in 0..count {
                    let candidate = installed.GetAt(index).map_err(platform)?;
                    if candidate.Name().map_err(platform)?.to_string() == name {
                        recognizer
                            .SetDefaultRecognizer(&candidate)
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
            let recognized = recognizer
                .RecognizeAsync(&native, InkRecognitionTarget::All)
                .map_err(platform)?
                .await
                .map_err(platform)?;
            let mut result = Vec::new();
            for index in 0..recognized.Size().map_err(platform)? {
                let group = recognized.GetAt(index).map_err(platform)?;
                let bounds = group.BoundingRect().map_err(platform)?;
                let candidates = group.GetTextCandidates().map_err(platform)?;
                let mut text_candidates = Vec::new();
                for index in 0..candidates.Size().map_err(platform)? {
                    text_candidates.push(candidates.GetAt(index).map_err(platform)?.to_string());
                }
                let native_strokes = group.GetStrokes().map_err(platform)?;
                let mut stroke_ids = Vec::new();
                for index in 0..native_strokes.Size().map_err(platform)? {
                    let id = native_strokes
                        .GetAt(index)
                        .map_err(platform)?
                        .Id()
                        .map_err(platform)?;
                    if let Some(id) = identities.get(&id) {
                        stroke_ids.push(*id);
                    }
                }
                result.push(WindowsInkRecognitionResult {
                    bounding_rect: Rect::new(
                        bounds.X as f64,
                        bounds.Y as f64,
                        bounds.Width as f64,
                        bounds.Height as f64,
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
        let container = NativeContainer::new().map_err(platform)?;
        let builder = InkStrokeBuilder::new().map_err(platform)?;
        for stroke in strokes {
            let attributes = to_native_attributes(stroke.drawing_attributes())?;
            builder
                .SetDefaultDrawingAttributes(&attributes)
                .map_err(platform)?;
            let mut points = Vec::with_capacity(stroke.ink_points().len());
            for point in stroke.ink_points() {
                points.push(Some(
                    NativeInkPoint::CreateInkPointWithTiltAndTimestamp(
                        to_native_point(point.position)?,
                        point.pressure,
                        point.tilt_x,
                        point.tilt_y,
                        point.timestamp,
                    )
                    .map_err(platform)?,
                ));
            }
            let points: IIterable<NativeInkPoint> = points.into();
            // Keep raw points and the editable point transform as distinct values.
            let native = builder
                .CreateStrokeFromInkPoints(&points, Matrix3x2::identity())
                .map_err(platform)?;
            native
                .SetPointTransform(to_native_matrix(stroke.point_transform())?)
                .map_err(platform)?;
            native.SetSelected(stroke.selected()).map_err(platform)?;
            container.AddStroke(&native).map_err(platform)?;
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
            let stream = InMemoryRandomAccessStream::new().map_err(platform)?;
            let output = stream.GetOutputStreamAt(0).map_err(platform)?;
            match format {
                InkPersistenceFormat::Isf => {
                    native
                        .SaveWithFormatAsync(&output, NativeFormat::Isf)
                        .map_err(platform)?
                        .await
                        .map_err(platform)?;
                }
                InkPersistenceFormat::GifWithEmbeddedIsf => {
                    // GIF rasterization is also bounded: at most a 16K side and
                    // a BGRA backing surface no larger than the byte budget.
                    validate_gif_bounds(&native, limits)?;
                    native
                        .SaveAsync(&output)
                        .map_err(platform)?
                        .await
                        .map_err(platform)?;
                }
                InkPersistenceFormat::Portable => unreachable!(),
            }
            let size = stream.Size().map_err(platform)?;
            if size > limits.max_bytes as u64 || size > u32::MAX as u64 {
                return Err(InkError::LimitExceeded);
            }
            let input = stream.GetInputStreamAt(0).map_err(platform)?;
            let reader = DataReader::CreateDataReader(&input).map_err(platform)?;
            let loaded = reader
                .LoadAsync(size as u32)
                .map_err(platform)?
                .await
                .map_err(platform)?;
            if loaded as u64 != size {
                return Err(InkError::InvalidData("Truncated native ink stream"));
            }
            let mut data = vec![0; size as usize];
            reader.ReadBytes(&mut data).map_err(platform)?;
            Ok(data)
        })
    }

    fn load(&self, data: Vec<u8>) -> InkServiceFuture<Vec<Rc<InkStroke>>> {
        let limits = self.limits;
        Box::pin(async move {
            if data.len() > limits.max_bytes || data.len() > u32::MAX as usize {
                return Err(InkError::LimitExceeded);
            }
            let stream = InMemoryRandomAccessStream::new().map_err(platform)?;
            let output = stream.GetOutputStreamAt(0).map_err(platform)?;
            let writer = DataWriter::CreateDataWriter(&output).map_err(platform)?;
            writer.WriteBytes(&data).map_err(platform)?;
            let stored = writer
                .StoreAsync()
                .map_err(platform)?
                .await
                .map_err(platform)?;
            if stored as usize != data.len() {
                return Err(InkError::InvalidData("Truncated native ink stream"));
            }
            let native = NativeContainer::new().map_err(platform)?;
            let input = stream.GetInputStreamAt(0).map_err(platform)?;
            native
                .LoadAsync(&input)
                .map_err(platform)?
                .await
                .map_err(platform)?;
            from_native_container(&native, limits)
        })
    }

    fn copy_to_clipboard(&self, strokes: &[Rc<InkStroke>]) -> Result<(), InkError> {
        let native = self.native_container(strokes)?;
        // The trait receives the strokes chosen for copying. The native method
        // only copies Selected strokes, so select the temporary native copies.
        let selected = native.GetStrokes().map_err(platform)?;
        for index in 0..selected.Size().map_err(platform)? {
            selected
                .GetAt(index)
                .map_err(platform)?
                .SetSelected(true)
                .map_err(platform)?;
        }
        native.CopySelectedToClipboard().map_err(platform)
    }

    fn can_paste(&self) -> Result<bool, InkError> {
        NativeContainer::new()
            .map_err(platform)?
            .CanPasteFromClipboard()
            .map_err(platform)
    }

    fn paste(&self, position: Point) -> Result<Vec<Rc<InkStroke>>, InkError> {
        let position = to_native_point(position)?;
        let native = NativeContainer::new().map_err(platform)?;
        native.PasteFromClipboard(position).map_err(platform)?;
        from_native_container(&native, self.limits)
    }
}

fn platform(error: windows::core::Error) -> InkError {
    InkError::Platform(format!("Windows ink: {error}"))
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
    let bounds = native.BoundingRect().map_err(platform)?;
    let width = (bounds.Width as f64).ceil().max(1.0);
    let height = (bounds.Height as f64).ceil().max(1.0);
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

fn to_native_point(point: Point) -> Result<NativePoint, InkError> {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || point.x.abs() > 1.0e12
        || point.y.abs() > 1.0e12
    {
        return Err(InkError::InvalidPoint);
    }
    Ok(NativePoint {
        X: point.x as f32,
        Y: point.y as f32,
    })
}

fn to_native_matrix(matrix: Matrix) -> Result<Matrix3x2, InkError> {
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
    Ok(Matrix3x2 {
        M11: matrix.m11 as f32,
        M12: matrix.m12 as f32,
        M21: matrix.m21 as f32,
        M22: matrix.m22 as f32,
        M31: matrix.m31 as f32,
        M32: matrix.m32 as f32,
    })
}

fn from_native_matrix(matrix: Matrix3x2) -> Matrix {
    Matrix::new(
        matrix.M11 as f64,
        matrix.M12 as f64,
        matrix.M21 as f64,
        matrix.M22 as f64,
        matrix.M31 as f64,
        matrix.M32 as f64,
    )
}

fn to_native_attributes(attributes: InkDrawingAttributes) -> Result<NativeAttributes, InkError> {
    attributes.validate()?;
    let native = match attributes.kind {
        InkDrawingAttributesKind::Default => NativeAttributes::new(),
        InkDrawingAttributesKind::Pencil => NativeAttributes::CreateForPencil(),
    }
    .map_err(platform)?;
    native
        .SetColor(NativeColor {
            A: attributes.color.a,
            R: attributes.color.r,
            G: attributes.color.g,
            B: attributes.color.b,
        })
        .map_err(platform)?;
    native
        .SetSize(NativeSize {
            Width: attributes.size.width as f32,
            Height: attributes.size.height as f32,
        })
        .map_err(platform)?;
    // Avoid redundant setters: pencil has fixed characteristics in some OS
    // versions. If an actual requested change is unsupported, report its HRESULT.
    let pen_tip = match attributes.pen_tip {
        PenTipShape::Circle => NativePenTip::Circle,
        PenTipShape::Rectangle => NativePenTip::Rectangle,
    };
    if native.PenTip().map_err(platform)? != pen_tip {
        native.SetPenTip(pen_tip).map_err(platform)?;
    }
    let transform = to_native_matrix(attributes.pen_tip_transform)?;
    if native.PenTipTransform().map_err(platform)? != transform {
        native.SetPenTipTransform(transform).map_err(platform)?;
    }
    if native.IgnorePressure().map_err(platform)? != attributes.ignore_pressure {
        native
            .SetIgnorePressure(attributes.ignore_pressure)
            .map_err(platform)?;
    }
    if native.FitToCurve().map_err(platform)? != attributes.fit_to_curve {
        native
            .SetFitToCurve(attributes.fit_to_curve)
            .map_err(platform)?;
    }
    if native.DrawAsHighlighter().map_err(platform)? != attributes.draw_as_highlighter {
        native
            .SetDrawAsHighlighter(attributes.draw_as_highlighter)
            .map_err(platform)?;
    }
    if native.IgnoreTilt().map_err(platform)? != attributes.ignore_tilt {
        native
            .SetIgnoreTilt(attributes.ignore_tilt)
            .map_err(platform)?;
    }
    if attributes.kind == InkDrawingAttributesKind::Pencil {
        native
            .PencilProperties()
            .map_err(platform)?
            .SetOpacity(attributes.pencil_opacity)
            .map_err(platform)?;
    }
    Ok(native)
}

fn from_native_attributes(native: &NativeAttributes) -> Result<InkDrawingAttributes, InkError> {
    let color = native.Color().map_err(platform)?;
    let size = native.Size().map_err(platform)?;
    let kind = match native.Kind().map_err(platform)? {
        NativeAttributesKind::Default => InkDrawingAttributesKind::Default,
        NativeAttributesKind::Pencil => InkDrawingAttributesKind::Pencil,
        _ => return Err(InkError::UnsupportedFormat),
    };
    let attributes = InkDrawingAttributes {
        color: Color::from_argb(color.A, color.R, color.G, color.B),
        size: Size::new(size.Width as f64, size.Height as f64),
        pen_tip: match native.PenTip().map_err(platform)? {
            NativePenTip::Circle => PenTipShape::Circle,
            NativePenTip::Rectangle => PenTipShape::Rectangle,
            _ => return Err(InkError::UnsupportedFormat),
        },
        pen_tip_transform: from_native_matrix(native.PenTipTransform().map_err(platform)?),
        draw_as_highlighter: native.DrawAsHighlighter().map_err(platform)?,
        ignore_pressure: native.IgnorePressure().map_err(platform)?,
        fit_to_curve: native.FitToCurve().map_err(platform)?,
        ignore_tilt: native.IgnoreTilt().map_err(platform)?,
        kind,
        pencil_opacity: if kind == InkDrawingAttributesKind::Pencil {
            native
                .PencilProperties()
                .map_err(platform)?
                .Opacity()
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
    let strokes = native.GetStrokes().map_err(platform)?;
    let count = strokes.Size().map_err(platform)? as usize;
    if count > limits.max_strokes {
        return Err(InkError::LimitExceeded);
    }
    let mut result = Vec::with_capacity(count);
    let mut total = 0usize;
    for index in 0..count {
        let native = strokes.GetAt(index as u32).map_err(platform)?;
        result.push(from_native_stroke(&native, limits, &mut total)?);
    }
    Ok(result)
}

fn from_native_stroke(
    native: &NativeStroke,
    limits: InkPersistenceLimits,
    total: &mut usize,
) -> Result<Rc<InkStroke>, InkError> {
    let source = native.GetInkPoints().map_err(platform)?;
    let count = source.Size().map_err(platform)? as usize;
    *total = total.checked_add(count).ok_or(InkError::LimitExceeded)?;
    if count > limits.max_points_per_stroke || *total > limits.max_total_points {
        return Err(InkError::LimitExceeded);
    }
    let mut points = Vec::with_capacity(count);
    for index in 0..count {
        let point = source.GetAt(index as u32).map_err(platform)?;
        let position = point.Position().map_err(platform)?;
        points.push(InkPoint {
            position: Point::new(position.X as f64, position.Y as f64),
            pressure: point.Pressure().map_err(platform)?,
            tilt_x: point.TiltX().map_err(platform)?,
            tilt_y: point.TiltY().map_err(platform)?,
            twist: 0.0,
            timestamp: point.Timestamp().map_err(platform)?,
        });
    }
    let stroke = InkStroke::new(
        points,
        from_native_attributes(&native.DrawingAttributes().map_err(platform)?)?,
    )?;
    stroke.set_point_transform(from_native_matrix(
        native.PointTransform().map_err(platform)?,
    ))?;
    stroke.set_selected(native.Selected().map_err(platform)?);
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
            [native.M11, native.M12, native.M21, native.M22, native.M31, native.M32],
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
            NativePoint { X: 1.5, Y: -2.0 }
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
            let first = native.GetStrokes().unwrap().GetAt(0).unwrap();
            assert!(first.Selected().unwrap());
            let transform = first.PointTransform().unwrap();
            assert_eq!(
                [transform.M11, transform.M22, transform.M31, transform.M32],
                [1.25, 0.75, 30.0, 40.0]
            );
            let color = first.DrawingAttributes().unwrap().Color().unwrap();
            assert_eq!([color.A, color.R, color.G, color.B], [255, 12, 34, 56]);
            let point = first.GetInkPoints().unwrap().GetAt(0).unwrap();
            near(point.Position().unwrap().X as f64, 10.0, 0.05);
            near(point.Position().unwrap().Y as f64, 20.0, 0.05);
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
    /// `cargo test -p ferroui-winui native_clipboard_round_trip -- --ignored`.
    #[test]
    #[ignore = "replaces the system clipboard; requires an isolated interactive Windows session"]
    fn native_clipboard_round_trip() {
        native_thread(|| {
            let services = WindowsInkServices::new().expect("Windows ink activation");
            let source = fixture_strokes();
            services.copy_to_clipboard(&source).unwrap();
            assert!(services.can_paste().unwrap());
            let pasted = services.paste(Point::new(300.0, 200.0)).unwrap();
            assert_eq!(pasted.len(), source.len());
            assert!(pasted.iter().all(|stroke| stroke.selected()));
            let container = InkStrokeContainer::new();
            container.add_strokes(pasted).unwrap();
            let bounds = container.bounding_rect();
            near(bounds.x, 300.0, 1.0);
            near(bounds.y, 200.0, 1.0);
        });
    }
}
