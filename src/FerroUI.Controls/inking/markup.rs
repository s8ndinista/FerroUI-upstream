//! Explicit metadata for the plain service objects and the inking enum family.
use super::*;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{ferro_markup_enum, ferro_markup_type, Matrix, Rect};
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
    methods: [fn Clear() => |p: &Rc<InkStrokeContainer>| p.clear()],
});
ferro_markup_type!(class InkStroke {
    namespace: "FerroUI.Controls", handles: [Rc<InkStroke>, Option<Rc<InkStroke>>], this: Rc<InkStroke>,
    properties: [
        Id: u32 { get: |p: &Rc<InkStroke>| p.id() },
        Selected: bool { get: |p: &Rc<InkStroke>| p.selected(), set: |p: &Rc<InkStroke>, v| p.set_selected(v) },
        BoundingRect: Rect { get: |p: &Rc<InkStroke>| p.bounding_rect() },
        PointTransform: Matrix { get: |p: &Rc<InkStroke>| p.point_transform(), try_set: |p: &Rc<InkStroke>, v| p.set_point_transform(v) },
    ],
});
ferro_markup_type!(class InkStrokeInput { namespace: "FerroUI.Controls", handles: [Rc<InkStrokeInput>, Option<Rc<InkStrokeInput>>], this: Rc<InkStrokeInput>,
    properties: [InkPresenter: Option<Rc<InkPresenter>> { get: |p: &Rc<InkStrokeInput>| p.ink_presenter() }],
});
ferro_markup_type!(class InkUnprocessedInput { namespace: "FerroUI.Controls", handles: [Rc<InkUnprocessedInput>, Option<Rc<InkUnprocessedInput>>], this: Rc<InkUnprocessedInput>,
    properties: [InkPresenter: Option<Rc<InkPresenter>> { get: |p: &Rc<InkUnprocessedInput>| p.ink_presenter() }],
});
ferro_markup_type!(class InkSynchronizer { namespace: "FerroUI.Controls", handles: [Rc<InkSynchronizer>, Option<Rc<InkSynchronizer>>] });
ferro_markup_type!(class InkPresenterRuler { namespace: "FerroUI.Controls", handles: [Rc<InkPresenterRuler>, Option<Rc<InkPresenterRuler>>], this: Rc<InkPresenterRuler>,
    properties: [IsVisible: bool { get: |p: &Rc<InkPresenterRuler>| p.is_visible(), set: |p: &Rc<InkPresenterRuler>, v| p.set_is_visible(v) }],
});
ferro_markup_type!(class InkPresenterProtractor { namespace: "FerroUI.Controls", handles: [Rc<InkPresenterProtractor>, Option<Rc<InkPresenterProtractor>>], this: Rc<InkPresenterProtractor>,
    properties: [IsVisible: bool { get: |p: &Rc<InkPresenterProtractor>| p.is_visible(), set: |p: &Rc<InkPresenterProtractor>, v| p.set_is_visible(v) }],
});

pub(crate) const TYPES: &[&MarkupType] = &[
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
];

pub(crate) fn register_markup() {
    MarkupType::register_all(TYPES);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<Rc<InkPresenter>>();
        ValueTypes::register_nullable::<Rc<InkStrokeContainer>>();
        ValueTypes::register_nullable::<Rc<InkStroke>>();
        ValueTypes::register_nullable::<Rc<InkPresenterRuler>>();
        ValueTypes::register_nullable::<Rc<InkPresenterProtractor>>();
    });
}
