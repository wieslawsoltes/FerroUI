//! Explicit metadata for the plain service objects and the inking enum family.
use super::*;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::PointerType;
use ferroui_base::media::Color;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupType, MarkupTyped};
use ferroui_base::{ferro_markup_enum, ferro_markup_type, Matrix, Point, Rect, Ref, Size};
use std::rc::Rc;

ferro_markup_enum!(InkInputProcessingMode { None, Inking, Erasing }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(InkInputRightDragAction { LeaveUnprocessed, AllowProcessing }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(InkHighContrastAdjustment { UseSystemColorsWhenNecessary, UseSystemColors, UseOriginalColors }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(InkPresenterPredefinedConfiguration { SimpleSinglePointer, SimpleMultiplePointer }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(InkPersistenceFormat { GifWithEmbeddedIsf, Isf, Portable }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(flags CoreInputDeviceTypes { None = CoreInputDeviceTypes::NONE, Touch = CoreInputDeviceTypes::TOUCH,
    Pen = CoreInputDeviceTypes::PEN, Mouse = CoreInputDeviceTypes::MOUSE }, { namespace: "FerroUI.Controls" });

ferro_markup_type!(class InkPresenter {
    namespace: "FerroUI.Controls",
    handles: [Rc<InkPresenter>, Option<Rc<InkPresenter>>],
    this: Rc<InkPresenter>,
    constructors: [() => InkPresenter::new],
    properties: [
        InputDeviceTypes: CoreInputDeviceTypes { get: |p: &Rc<InkPresenter>| p.input_device_types(), set: |p: &Rc<InkPresenter>, v| p.set_input_device_types(v) },
        IsInputEnabled: bool { get: |p: &Rc<InkPresenter>| p.is_input_enabled(), set: |p: &Rc<InkPresenter>, v| p.set_is_input_enabled(v) },
        HighContrastAdjustment: InkHighContrastAdjustment { get: |p: &Rc<InkPresenter>| p.high_contrast_adjustment(), set: |p: &Rc<InkPresenter>, v| p.set_high_contrast_adjustment(v) },
        StrokeContainer: Rc<InkStrokeContainer> { get: |p: &Rc<InkPresenter>| p.stroke_container() },
        InputProcessingConfiguration: Rc<InkInputProcessingConfiguration> { get: |p: &Rc<InkPresenter>| p.input_processing_configuration() },
        InputConfiguration: Rc<InkInputConfiguration> { get: |p: &Rc<InkPresenter>| p.input_configuration() },
        StrokeInput: Rc<InkStrokeInput> { get: |p: &Rc<InkPresenter>| p.stroke_input() },
        UnprocessedInput: Rc<InkUnprocessedInput> { get: |p: &Rc<InkPresenter>| p.unprocessed_input() },
    ],
    methods: [
        try fn UpdateDefaultDrawingAttributes(InkDrawingAttributes) => |p: &Rc<InkPresenter>, attributes| p.update_default_drawing_attributes(&attributes),
        fn CopyDefaultDrawingAttributes() -> InkDrawingAttributes => |p: &Rc<InkPresenter>| p.copy_default_drawing_attributes(),
        fn SetPredefinedConfiguration(InkPresenterPredefinedConfiguration) => |p: &Rc<InkPresenter>, value| p.set_predefined_configuration(value),
        try fn ActivateCustomDrying() -> Rc<InkSynchronizer> => |p: &Rc<InkPresenter>| p.activate_custom_drying(),
    ],
    events: [
        StrokesCollected(Option<Rc<InkPresenter>>, InkStrokesCollectedEventArgs) => |p: &Rc<InkPresenter>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.strokes_collected(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        StrokesErased(Option<Rc<InkPresenter>>, InkStrokesErasedEventArgs) => |p: &Rc<InkPresenter>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.strokes_erased(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
    ],
});
ferro_markup_type!(class InkInputProcessingConfiguration {
    namespace: "FerroUI.Controls", handles: [Rc<InkInputProcessingConfiguration>, Option<Rc<InkInputProcessingConfiguration>>], this: Rc<InkInputProcessingConfiguration>,
    properties: [
        Mode: InkInputProcessingMode { get: |p: &Rc<InkInputProcessingConfiguration>| p.mode(), set: |p: &Rc<InkInputProcessingConfiguration>, v| p.set_mode(v) },
        RightDragAction: InkInputRightDragAction { get: |p: &Rc<InkInputProcessingConfiguration>| p.right_drag_action(), set: |p: &Rc<InkInputProcessingConfiguration>, v| p.set_right_drag_action(v) },
    ],
});
ferro_markup_type!(class InkInputConfiguration {
    namespace: "FerroUI.Controls", handles: [Rc<InkInputConfiguration>, Option<Rc<InkInputConfiguration>>], this: Rc<InkInputConfiguration>,
    properties: [
        IsPrimaryBarrelButtonInputEnabled: bool { get: |p: &Rc<InkInputConfiguration>| p.is_primary_barrel_button_input_enabled(), set: |p: &Rc<InkInputConfiguration>, v| p.set_is_primary_barrel_button_input_enabled(v) },
        IsEraserInputEnabled: bool { get: |p: &Rc<InkInputConfiguration>| p.is_eraser_input_enabled(), set: |p: &Rc<InkInputConfiguration>, v| p.set_is_eraser_input_enabled(v) },
    ],
});
ferro_markup_type!(class InkStrokeContainer {
    namespace: "FerroUI.Controls", handles: [Rc<InkStrokeContainer>, Option<Rc<InkStrokeContainer>>], this: Rc<InkStrokeContainer>,
    constructors: [() => InkStrokeContainer::new],
    properties: [BoundingRect: Rect { get: |p: &Rc<InkStrokeContainer>| p.bounding_rect() }],
    methods: [
        fn Clear() => |p: &Rc<InkStrokeContainer>| p.clear(),
        fn GetStrokes() -> Vec<Rc<InkStroke>> => |p: &Rc<InkStrokeContainer>| p.get_strokes(),
        try fn AddStroke(Rc<InkStroke>) => |p: &Rc<InkStrokeContainer>, stroke| p.add_stroke(stroke),
        try fn AddStrokes(Vec<Rc<InkStroke>>) => |p: &Rc<InkStrokeContainer>, strokes| p.add_strokes(strokes),
        fn GetStrokeById(u32) -> Option<Rc<InkStroke>> => |p: &Rc<InkStrokeContainer>, id| p.get_stroke_by_id(id),
        fn DeleteSelected() -> Rect => |p: &Rc<InkStrokeContainer>| p.delete_selected(),
        try fn MoveSelected(Point) -> Rect => |p: &Rc<InkStrokeContainer>, delta| p.move_selected(delta),
        fn SelectWithLine(Point, Point) -> Rect => |p: &Rc<InkStrokeContainer>, from, to| p.select_with_line(from, to),
        fn SelectWithPolyLine(Vec<Point>) -> Rect => |p: &Rc<InkStrokeContainer>, points: Vec<Point>| p.select_with_poly_line(&points),
        try fn CopySelectedToClipboard() => |p: &Rc<InkStrokeContainer>| p.copy_selected_to_clipboard(),
        try fn CanPasteFromClipboard() -> bool => |p: &Rc<InkStrokeContainer>| p.can_paste_from_clipboard(),
        try fn PasteFromClipboard(Point) -> Rect => |p: &Rc<InkStrokeContainer>, position| p.paste_from_clipboard(position),
    ],
});
ferro_markup_type!(class InkStroke {
    namespace: "FerroUI.Controls", handles: [Rc<InkStroke>, Option<Rc<InkStroke>>], this: Rc<InkStroke>,
    properties: [
        Id: u32 { get: |p: &Rc<InkStroke>| p.id() },
        DrawingAttributes: InkDrawingAttributes { get: |p: &Rc<InkStroke>| p.drawing_attributes(), try_set: |p: &Rc<InkStroke>, value| p.set_drawing_attributes(value) },
        Selected: bool { get: |p: &Rc<InkStroke>| p.selected(), set: |p: &Rc<InkStroke>, v| p.set_selected(v) },
        BoundingRect: Rect { get: |p: &Rc<InkStroke>| p.bounding_rect() },
        PointTransform: Matrix { get: |p: &Rc<InkStroke>| p.point_transform(), try_set: |p: &Rc<InkStroke>, v| p.set_point_transform(v) },
    ],
});
ferro_markup_type!(class InkStrokeInput { namespace: "FerroUI.Controls", handles: [Rc<InkStrokeInput>, Option<Rc<InkStrokeInput>>], this: Rc<InkStrokeInput>,
    properties: [InkPresenter: Option<Rc<InkPresenter>> { get: |p: &Rc<InkStrokeInput>| p.ink_presenter() }],
    events: [
        StrokeStarted(Option<Rc<InkStrokeInput>>, InkPointerEventArgs) => |p: &Rc<InkStrokeInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.stroke_started(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        StrokeContinued(Option<Rc<InkStrokeInput>>, InkPointerEventArgs) => |p: &Rc<InkStrokeInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.stroke_continued(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        StrokeEnded(Option<Rc<InkStrokeInput>>, InkPointerEventArgs) => |p: &Rc<InkStrokeInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.stroke_ended(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        StrokeCanceled(Option<Rc<InkStrokeInput>>, InkPointerEventArgs) => |p: &Rc<InkStrokeInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.stroke_canceled(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
    ],
});
ferro_markup_type!(class InkUnprocessedInput { namespace: "FerroUI.Controls", handles: [Rc<InkUnprocessedInput>, Option<Rc<InkUnprocessedInput>>], this: Rc<InkUnprocessedInput>,
    properties: [InkPresenter: Option<Rc<InkPresenter>> { get: |p: &Rc<InkUnprocessedInput>| p.ink_presenter() }],
    events: [
        PointerEntered(Option<Rc<InkUnprocessedInput>>, InkPointerEventArgs) => |p: &Rc<InkUnprocessedInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.pointer_entered(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        PointerHovered(Option<Rc<InkUnprocessedInput>>, InkPointerEventArgs) => |p: &Rc<InkUnprocessedInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.pointer_hovered(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        PointerExited(Option<Rc<InkUnprocessedInput>>, InkPointerEventArgs) => |p: &Rc<InkUnprocessedInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.pointer_exited(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        PointerPressed(Option<Rc<InkUnprocessedInput>>, InkPointerEventArgs) => |p: &Rc<InkUnprocessedInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.pointer_pressed(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        PointerMoved(Option<Rc<InkUnprocessedInput>>, InkPointerEventArgs) => |p: &Rc<InkUnprocessedInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.pointer_moved(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        PointerReleased(Option<Rc<InkUnprocessedInput>>, InkPointerEventArgs) => |p: &Rc<InkUnprocessedInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.pointer_released(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        PointerLost(Option<Rc<InkUnprocessedInput>>, InkPointerEventArgs) => |p: &Rc<InkUnprocessedInput>, handler: MarkupDelegate| {
            let sender = Rc::downgrade(p);
            p.pointer_lost(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
    ],
});
ferro_markup_type!(class InkSynchronizer {
    namespace: "FerroUI.Controls", handles: [Rc<InkSynchronizer>, Option<Rc<InkSynchronizer>>], this: Rc<InkSynchronizer>,
    methods: [
        try fn BeginDry() -> Vec<Rc<InkStroke>> => |p: &Rc<InkSynchronizer>| p.begin_dry(),
        try fn EndDry() => |p: &Rc<InkSynchronizer>| p.end_dry(),
    ],
});
ferro_markup_type!(class InkPresenterRuler {
    namespace: "FerroUI.Controls", handles: [Rc<InkPresenterRuler>, Option<Rc<InkPresenterRuler>>], this: Rc<InkPresenterRuler>,
    properties: [
        Kind: InkPresenterStencilKind { get: |p: &Rc<InkPresenterRuler>| p.kind() },
        IsVisible: bool { get: |p: &Rc<InkPresenterRuler>| p.is_visible(), set: |p: &Rc<InkPresenterRuler>, value| p.set_is_visible(value) },
        Transform: Matrix { get: |p: &Rc<InkPresenterRuler>| p.transform(), try_set: |p: &Rc<InkPresenterRuler>, value| p.set_transform(value) },
        Length: f64 { get: |p: &Rc<InkPresenterRuler>| p.length(), try_set: |p: &Rc<InkPresenterRuler>, value| p.set_length(value) },
        Width: f64 { get: |p: &Rc<InkPresenterRuler>| p.width(), try_set: |p: &Rc<InkPresenterRuler>, value| p.set_width(value) },
        AreTickMarksVisible: bool { get: |p: &Rc<InkPresenterRuler>| p.are_tick_marks_visible(), set: |p: &Rc<InkPresenterRuler>, value| p.set_are_tick_marks_visible(value) },
        IsCompassVisible: bool { get: |p: &Rc<InkPresenterRuler>| p.is_compass_visible(), set: |p: &Rc<InkPresenterRuler>, value| p.set_is_compass_visible(value) },
        BackgroundColor: Color { get: |p: &Rc<InkPresenterRuler>| p.background_color(), set: |p: &Rc<InkPresenterRuler>, value| p.set_background_color(value) },
        ForegroundColor: Color { get: |p: &Rc<InkPresenterRuler>| p.foreground_color(), set: |p: &Rc<InkPresenterRuler>, value| p.set_foreground_color(value) },
    ],
});
ferro_markup_type!(class InkPresenterProtractor {
    namespace: "FerroUI.Controls", handles: [Rc<InkPresenterProtractor>, Option<Rc<InkPresenterProtractor>>], this: Rc<InkPresenterProtractor>,
    properties: [
        Kind: InkPresenterStencilKind { get: |p: &Rc<InkPresenterProtractor>| p.kind() },
        IsVisible: bool { get: |p: &Rc<InkPresenterProtractor>| p.is_visible(), set: |p: &Rc<InkPresenterProtractor>, value| p.set_is_visible(value) },
        Transform: Matrix { get: |p: &Rc<InkPresenterProtractor>| p.transform(), try_set: |p: &Rc<InkPresenterProtractor>, value| p.set_transform(value) },
        Radius: f64 { get: |p: &Rc<InkPresenterProtractor>| p.radius(), try_set: |p: &Rc<InkPresenterProtractor>, value| p.set_radius(value) },
        AreTickMarksVisible: bool { get: |p: &Rc<InkPresenterProtractor>| p.are_tick_marks_visible(), set: |p: &Rc<InkPresenterProtractor>, value| p.set_are_tick_marks_visible(value) },
        AreRaysVisible: bool { get: |p: &Rc<InkPresenterProtractor>| p.are_rays_visible(), set: |p: &Rc<InkPresenterProtractor>, value| p.set_are_rays_visible(value) },
        IsCenterMarkerVisible: bool { get: |p: &Rc<InkPresenterProtractor>| p.is_center_marker_visible(), set: |p: &Rc<InkPresenterProtractor>, value| p.set_is_center_marker_visible(value) },
        IsAngleReadoutVisible: bool { get: |p: &Rc<InkPresenterProtractor>| p.is_angle_readout_visible(), set: |p: &Rc<InkPresenterProtractor>, value| p.set_is_angle_readout_visible(value) },
        IsResizable: bool { get: |p: &Rc<InkPresenterProtractor>| p.is_resizable(), set: |p: &Rc<InkPresenterProtractor>, value| p.set_is_resizable(value) },
        BackgroundColor: Color { get: |p: &Rc<InkPresenterProtractor>| p.background_color(), set: |p: &Rc<InkPresenterProtractor>, value| p.set_background_color(value) },
        ForegroundColor: Color { get: |p: &Rc<InkPresenterProtractor>| p.foreground_color(), set: |p: &Rc<InkPresenterProtractor>, value| p.set_foreground_color(value) },
        AccentColor: Color { get: |p: &Rc<InkPresenterProtractor>| p.accent_color(), set: |p: &Rc<InkPresenterProtractor>, value| p.set_accent_color(value) },
    ],
});

ferro_markup_type!(struct InkPoint {
    namespace: "FerroUI.Controls", handles: [InkPoint],
    constructors: [try (Point, f32) => InkPoint::new],
    properties: [
        Position: Point { get: |p: &InkPoint| p.position },
        Pressure: f32 { get: |p: &InkPoint| p.pressure },
        TiltX: f32 { get: |p: &InkPoint| p.tilt_x },
        TiltY: f32 { get: |p: &InkPoint| p.tilt_y },
        Twist: f32 { get: |p: &InkPoint| p.twist },
        Timestamp: u64 { get: |p: &InkPoint| p.timestamp },
    ],
});
ferro_markup_type!(struct InkDrawingAttributes {
    namespace: "FerroUI.Controls", handles: [InkDrawingAttributes, Option<InkDrawingAttributes>],
    constructors: [() => InkDrawingAttributes::new],
    properties: [
        Color: Color { get: |p: &InkDrawingAttributes| p.color },
        Size: Size { get: |p: &InkDrawingAttributes| p.size },
        PenTip: PenTipShape { get: |p: &InkDrawingAttributes| p.pen_tip },
        PenTipTransform: Matrix { get: |p: &InkDrawingAttributes| p.pen_tip_transform },
        DrawAsHighlighter: bool { get: |p: &InkDrawingAttributes| p.draw_as_highlighter },
        IgnorePressure: bool { get: |p: &InkDrawingAttributes| p.ignore_pressure },
        FitToCurve: bool { get: |p: &InkDrawingAttributes| p.fit_to_curve },
        IgnoreTilt: bool { get: |p: &InkDrawingAttributes| p.ignore_tilt },
        Kind: InkDrawingAttributesKind { get: |p: &InkDrawingAttributes| p.kind },
        PencilOpacity: f64 { get: |p: &InkDrawingAttributes| p.pencil_opacity },
    ],
});
ferro_markup_type!(class InkPointerEventArgs {
    namespace: "FerroUI.Controls", handles: [InkPointerEventArgs], this: InkPointerEventArgs,
    properties: [
        PointerId: i32 { get: |p: &InkPointerEventArgs| p.pointer_id },
        DeviceType: PointerType { get: |p: &InkPointerEventArgs| p.device_type },
        Point: InkPoint { get: |p: &InkPointerEventArgs| p.point },
        Handled: bool { get: InkPointerEventArgs::handled, set: InkPointerEventArgs::set_handled },
        IsBarrelButtonPressed: bool { get: |p: &InkPointerEventArgs| p.is_barrel_button_pressed },
        IsEraser: bool { get: |p: &InkPointerEventArgs| p.is_eraser },
        IsRightButtonPressed: bool { get: |p: &InkPointerEventArgs| p.is_right_button_pressed },
    ],
});
ferro_markup_type!(class InkStrokesCollectedEventArgs {
    namespace: "FerroUI.Controls", handles: [InkStrokesCollectedEventArgs], this: InkStrokesCollectedEventArgs,
    properties: [Strokes: Vec<Rc<InkStroke>> { get: |p: &InkStrokesCollectedEventArgs| p.strokes.clone() }],
});
ferro_markup_type!(class InkStrokesErasedEventArgs {
    namespace: "FerroUI.Controls", handles: [InkStrokesErasedEventArgs], this: InkStrokesErasedEventArgs,
    properties: [Strokes: Vec<Rc<InkStroke>> { get: |p: &InkStrokesErasedEventArgs| p.strokes.clone() }],
});
ferro_markup_type!(class InkToolbarIsStencilButtonCheckedChangedEventArgs {
    namespace: "FerroUI.Controls", handles: [InkToolbarIsStencilButtonCheckedChangedEventArgs], this: InkToolbarIsStencilButtonCheckedChangedEventArgs,
    properties: [
        StencilButton: Ref<InkToolbarStencilButton> { get: |p: &InkToolbarIsStencilButtonCheckedChangedEventArgs| p.stencil_button.clone() },
        StencilKind: InkToolbarStencilKind { get: |p: &InkToolbarIsStencilButtonCheckedChangedEventArgs| p.stencil_kind },
    ],
});

pub(crate) const TYPES: &[&MarkupType] = &[
    <InkPoint as MarkupTyped>::MARKUP,
    <InkDrawingAttributes as MarkupTyped>::MARKUP,
    <InkPointerEventArgs as MarkupTyped>::MARKUP,
    <InkStrokesCollectedEventArgs as MarkupTyped>::MARKUP,
    <InkStrokesErasedEventArgs as MarkupTyped>::MARKUP,
    <InkToolbarIsStencilButtonCheckedChangedEventArgs as MarkupTyped>::MARKUP,
    <CoreInputDeviceTypes as MarkupTyped>::MARKUP,
    <InkInputProcessingMode as MarkupTyped>::MARKUP,
    <InkInputRightDragAction as MarkupTyped>::MARKUP,
    <InkHighContrastAdjustment as MarkupTyped>::MARKUP,
    <InkPresenterPredefinedConfiguration as MarkupTyped>::MARKUP,
    <InkPersistenceFormat as MarkupTyped>::MARKUP,
    <PenTipShape as MarkupTyped>::MARKUP,
    <InkDrawingAttributesKind as MarkupTyped>::MARKUP,
    <InkToolbarButtonFlyoutPlacement as MarkupTyped>::MARKUP,
    <InkToolbarFlyoutItemKind as MarkupTyped>::MARKUP,
    <InkToolbarInitialControls as MarkupTyped>::MARKUP,
    <InkToolbarMenuKind as MarkupTyped>::MARKUP,
    <InkToolbarStencilKind as MarkupTyped>::MARKUP,
    <InkToolbarToggle as MarkupTyped>::MARKUP,
    <InkToolbarTool as MarkupTyped>::MARKUP,
    <InkPresenter as MarkupTyped>::MARKUP,
    <InkInputProcessingConfiguration as MarkupTyped>::MARKUP,
    <InkInputConfiguration as MarkupTyped>::MARKUP,
    <InkStrokeContainer as MarkupTyped>::MARKUP,
    <InkStroke as MarkupTyped>::MARKUP,
    <InkStrokeInput as MarkupTyped>::MARKUP,
    <InkUnprocessedInput as MarkupTyped>::MARKUP,
    <InkSynchronizer as MarkupTyped>::MARKUP,
    <InkPresenterRuler as MarkupTyped>::MARKUP,
    <InkPresenterProtractor as MarkupTyped>::MARKUP,
    <InkPresenterStencilKind as MarkupTyped>::MARKUP,
];

pub(crate) fn register_markup() {
    MarkupType::register_all(TYPES);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<Rc<InkPresenter>>();
        ValueTypes::register_nullable::<Rc<InkInputProcessingConfiguration>>();
        ValueTypes::register_nullable::<Rc<InkInputConfiguration>>();
        ValueTypes::register_nullable::<Rc<InkStrokeInput>>();
        ValueTypes::register_nullable::<Rc<InkUnprocessedInput>>();
        ValueTypes::register_nullable::<Rc<InkSynchronizer>>();
        ValueTypes::register_nullable::<InkDrawingAttributes>();
        ValueTypes::register_nullable::<Rc<InkStrokeContainer>>();
        ValueTypes::register_nullable::<Rc<InkStroke>>();
        ValueTypes::register_nullable::<Rc<InkPresenterRuler>>();
        ValueTypes::register_nullable::<Rc<InkPresenterProtractor>>();
    });
}
