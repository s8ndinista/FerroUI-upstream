use super::ink_events::InkEvent;
use super::*;
use ferroui_base::input::PointerType;
use ferroui_base::reactive::IDisposable;
use ferroui_base::Size;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub struct InkStrokesCollectedEventArgs {
    pub strokes: Vec<Rc<InkStroke>>,
}
impl InkStrokesCollectedEventArgs {
    pub fn strokes(&self) -> &[Rc<InkStroke>] {
        &self.strokes
    }
}
#[derive(Clone, PartialEq)]
pub struct InkStrokesErasedEventArgs {
    pub strokes: Vec<Rc<InkStroke>>,
}
impl InkStrokesErasedEventArgs {
    pub fn strokes(&self) -> &[Rc<InkStroke>] {
        &self.strokes
    }
}

struct ActiveStroke {
    points: Vec<InkPoint>,
    attributes: InkDrawingAttributes,
    last_event: InkPointerEventArgs,
    erasing: bool,
    preview: Option<Rc<InkStroke>>,
}

/// UI-thread ink coordinator. Services and input callbacks are stable, synchronous
/// FerroUI objects; geometry and persistence are portable Rust implementations.
pub struct InkPresenter {
    devices: Cell<CoreInputDeviceTypes>,
    enabled: Cell<bool>,
    attributes: Cell<InkDrawingAttributes>,
    high_contrast: Cell<InkHighContrastAdjustment>,
    configuration: Cell<InkPresenterPredefinedConfiguration>,
    container: Rc<InkStrokeContainer>,
    processing: Rc<InkInputProcessingConfiguration>,
    input: Rc<InkInputConfiguration>,
    stroke_input: Rc<InkStrokeInput>,
    unprocessed_input: Rc<InkUnprocessedInput>,
    synchronizer: Rc<InkSynchronizer>,
    ruler: Rc<InkPresenterRuler>,
    protractor: Rc<InkPresenterProtractor>,
    active: RefCell<BTreeMap<i32, ActiveStroke>>,
    epoch: Cell<u64>,
    custom_drying: Cell<bool>,
    input_activated: Cell<bool>,
    pending_dry: RefCell<Vec<Rc<InkStroke>>>,
    changed: InkEvent<()>,
    collected: InkEvent<InkStrokesCollectedEventArgs>,
    erased: InkEvent<InkStrokesErasedEventArgs>,
}

impl PartialEq for InkPresenter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl InkPresenter {
    pub fn new() -> Rc<Self> {
        let presenter: Rc<Self> = Rc::new_cyclic(|weak| Self {
            devices: Cell::new(CoreInputDeviceTypes::PEN),
            enabled: Cell::new(true),
            attributes: Cell::new(InkDrawingAttributes::default()),
            high_contrast: Cell::new(InkHighContrastAdjustment::default()),
            configuration: Cell::new(InkPresenterPredefinedConfiguration::default()),
            container: InkStrokeContainer::new(),
            processing: Rc::new(InkInputProcessingConfiguration::new(weak.clone())),
            input: Rc::new(InkInputConfiguration::default()),
            stroke_input: Rc::new(InkStrokeInput::new(weak.clone())),
            unprocessed_input: Rc::new(InkUnprocessedInput::new(weak.clone())),
            synchronizer: Rc::new(InkSynchronizer::new(weak.clone())),
            ruler: Rc::new(InkPresenterRuler::new(weak.clone())),
            protractor: Rc::new(InkPresenterProtractor::new(weak.clone())),
            active: RefCell::new(BTreeMap::new()),
            epoch: Cell::new(0),
            custom_drying: Cell::new(false),
            input_activated: Cell::new(false),
            pending_dry: RefCell::new(Vec::new()),
            changed: InkEvent::default(),
            collected: InkEvent::default(),
            erased: InkEvent::default(),
        });
        let weak = Rc::downgrade(&presenter);
        presenter.container.changed(move || {
            if let Some(p) = weak.upgrade() {
                let live = p.container.get_strokes();
                p.pending_dry
                    .borrow_mut()
                    .retain(|stroke| live.iter().any(|s| Rc::ptr_eq(s, stroke)));
                p.invalidate();
            }
        });
        presenter
    }
    pub fn input_device_types(&self) -> CoreInputDeviceTypes {
        self.devices.get()
    }
    pub fn set_input_device_types(&self, value: CoreInputDeviceTypes) {
        if self.devices.replace(value) != value {
            self.cancel_all();
        }
    }
    pub fn is_input_enabled(&self) -> bool {
        self.enabled.get()
    }
    pub fn set_is_input_enabled(&self, value: bool) {
        if self.enabled.replace(value) && !value {
            self.cancel_all();
        }
    }
    pub fn update_default_drawing_attributes(
        &self,
        value: &InkDrawingAttributes,
    ) -> Result<(), InkError> {
        value.validate()?;
        self.attributes.set(*value);
        Ok(())
    }
    pub fn copy_default_drawing_attributes(&self) -> InkDrawingAttributes {
        self.attributes.get()
    }
    pub fn set_predefined_configuration(&self, value: InkPresenterPredefinedConfiguration) {
        if self.configuration.replace(value) != value {
            self.cancel_all();
        }
    }
    pub fn high_contrast_adjustment(&self) -> InkHighContrastAdjustment {
        self.high_contrast.get()
    }
    pub fn set_high_contrast_adjustment(&self, value: InkHighContrastAdjustment) {
        self.high_contrast.set(value);
        self.invalidate();
    }
    pub fn stroke_container(&self) -> Rc<InkStrokeContainer> {
        self.container.clone()
    }
    pub fn input_processing_configuration(&self) -> Rc<InkInputProcessingConfiguration> {
        self.processing.clone()
    }
    pub fn input_configuration(&self) -> Rc<InkInputConfiguration> {
        self.input.clone()
    }
    pub fn stroke_input(&self) -> Rc<InkStrokeInput> {
        self.stroke_input.clone()
    }
    pub fn unprocessed_input(&self) -> Rc<InkUnprocessedInput> {
        self.unprocessed_input.clone()
    }
    pub fn ruler(&self) -> Rc<InkPresenterRuler> {
        self.ruler.clone()
    }
    pub fn protractor(&self) -> Rc<InkPresenterProtractor> {
        self.protractor.clone()
    }
    pub fn ruler_visible(&self) -> bool {
        self.ruler.is_visible()
    }
    pub fn protractor_visible(&self) -> bool {
        self.protractor.is_visible()
    }
    pub fn set_ruler_visible(&self, value: bool) {
        self.ruler.visible.set(value);
        if value {
            self.protractor.visible.set(false);
        }
        self.invalidate();
    }
    pub fn set_protractor_visible(&self, value: bool) {
        self.protractor.visible.set(value);
        if value {
            self.ruler.visible.set(false);
        }
        self.invalidate();
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.changed.subscribe(move |_| handler())
    }
    pub(crate) fn invalidate(&self) {
        self.changed.raise(&());
    }
    pub fn strokes_collected(
        &self,
        handler: impl Fn(&InkStrokesCollectedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.collected.subscribe(handler)
    }
    pub fn strokes_erased(
        &self,
        handler: impl Fn(&InkStrokesErasedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.erased.subscribe(handler)
    }
    pub fn activate_custom_drying(&self) -> Result<Rc<InkSynchronizer>, InkError> {
        if self.custom_drying.get() {
            return Ok(self.synchronizer.clone());
        }
        if self.input_activated.get() {
            return Err(InkError::InvalidOperation(
                "custom drying must be activated before layout or input",
            ));
        }
        self.custom_drying.set(true);
        self.invalidate();
        Ok(self.synchronizer.clone())
    }
    pub fn is_custom_drying(&self) -> bool {
        self.custom_drying.get()
    }
    pub(crate) fn set_size(&self, size: Size) {
        if size.width > 0.0 && size.height > 0.0 {
            self.input_activated.set(true);
        }
    }
    pub(crate) fn pending_dry_strokes(&self) -> Vec<Rc<InkStroke>> {
        self.pending_dry.borrow().clone()
    }
    pub(crate) fn finish_drying(&self, ids: &[u32]) {
        self.pending_dry
            .borrow_mut()
            .retain(|stroke| !ids.contains(&stroke.id()));
        self.invalidate();
    }
    pub fn has_active_pointer(&self, id: i32) -> bool {
        self.active.borrow().contains_key(&id)
    }
    pub fn cancel_pointer(&self, id: i32) {
        let active = self.active.borrow_mut().remove(&id);
        if let Some(active) = active {
            if !active.erasing {
                self.stroke_input.canceled.raise(&active.last_event);
            }
            self.unprocessed_input
                .raise(InkPointerPhase::Lost, &active.last_event);
            self.invalidate();
        }
    }
    pub fn cancel_all(&self) {
        self.epoch.set(self.epoch.get().wrapping_add(1));
        let active = std::mem::take(&mut *self.active.borrow_mut());
        for (_, stroke) in active {
            if !stroke.erasing {
                self.stroke_input.canceled.raise(&stroke.last_event);
            }
            self.unprocessed_input
                .raise(InkPointerPhase::Lost, &stroke.last_event);
        }
        self.invalidate();
    }
    fn admits(&self, device: PointerType) -> bool {
        self.devices.get().contains(match device {
            PointerType::Mouse => CoreInputDeviceTypes::MOUSE,
            PointerType::Pen => CoreInputDeviceTypes::PEN,
            PointerType::Touch => CoreInputDeviceTypes::TOUCH,
        })
    }
    fn constrained_point(&self, point: InkPoint) -> InkPoint {
        let mut result = point;
        result.position = self
            .protractor
            .constrain(self.ruler.constrain(point.position, 12.0), 12.0);
        result
    }
    /// Feeds a validated point from any platform. Returns whether the presenter
    /// consumed it; unprocessed listeners can also set `handled` synchronously.
    pub fn process_pointer(
        &self,
        phase: InkPointerPhase,
        e: &InkPointerEventArgs,
    ) -> Result<bool, InkError> {
        if let Err(error) = e.point.validate() {
            self.cancel_pointer(e.pointer_id);
            return Err(error);
        }
        if phase == InkPointerPhase::Lost {
            if self.has_active_pointer(e.pointer_id) {
                self.cancel_pointer(e.pointer_id);
            } else {
                self.unprocessed_input.raise(phase, e);
            }
            return Ok(e.handled());
        }
        if !self.enabled.get() || !self.admits(e.device_type) {
            return Ok(false);
        }
        if (e.is_eraser && !self.input.is_eraser_input_enabled())
            || (e.is_barrel_button_pressed && !self.input.is_primary_barrel_button_input_enabled())
        {
            self.cancel_pointer(e.pointer_id);
            return Ok(false);
        }
        if matches!(
            phase,
            InkPointerPhase::Entered | InkPointerPhase::Hovered | InkPointerPhase::Exited
        ) {
            self.unprocessed_input.raise(phase, e);
            return Ok(e.handled());
        }
        let secondary = e.is_right_button_pressed
            || (e.is_barrel_button_pressed && self.input.is_primary_barrel_button_input_enabled());
        let pass = self.processing.mode() == InkInputProcessingMode::None
            || (secondary
                && self.processing.right_drag_action()
                    == InkInputRightDragAction::LeaveUnprocessed);
        if pass && !self.has_active_pointer(e.pointer_id) {
            self.unprocessed_input.raise(phase, e);
            return Ok(e.handled());
        }
        self.input_activated.set(true);
        let point = self.constrained_point(e.point);
        match phase {
            InkPointerPhase::Pressed => {
                if self.has_active_pointer(e.pointer_id) {
                    self.cancel_pointer(e.pointer_id);
                }
                if self.configuration.get()
                    == InkPresenterPredefinedConfiguration::SimpleSinglePointer
                    && !self.active.borrow().is_empty()
                {
                    self.unprocessed_input.raise(phase, e);
                    return Ok(e.handled());
                }
                let erasing = self.processing.mode() == InkInputProcessingMode::Erasing
                    || (e.is_eraser && self.input.is_eraser_input_enabled());
                self.active.borrow_mut().insert(
                    e.pointer_id,
                    ActiveStroke {
                        points: vec![point],
                        attributes: self.attributes.get(),
                        last_event: e.clone(),
                        erasing,
                        preview: None,
                    },
                );
                if erasing {
                    self.erase_between(point.position, point.position)?;
                } else {
                    self.stroke_input.started.raise(e);
                }
                self.invalidate();
                Ok(true)
            }
            InkPointerPhase::Moved | InkPointerPhase::Released => {
                let exceeds_limit = self
                    .active
                    .borrow()
                    .get(&e.pointer_id)
                    .is_some_and(|stroke| {
                        !stroke.erasing
                            && stroke.points.last().is_some_and(|last| *last != point)
                            && stroke.points.len()
                                >= InkPersistenceLimits::DEFAULT.max_points_per_stroke
                    });
                if exceeds_limit {
                    self.cancel_pointer(e.pointer_id);
                    return Err(InkError::LimitExceeded);
                }
                let step = {
                    let mut active = self.active.borrow_mut();
                    active.get_mut(&e.pointer_id).map(|stroke| {
                        let previous = stroke.points.last().copied().unwrap_or(point);
                        if previous != point {
                            if stroke.erasing {
                                stroke.points.clear();
                            }
                            stroke.points.push(point);
                            stroke.preview = None;
                        }
                        stroke.last_event = e.clone();
                        (stroke.erasing, previous.position)
                    })
                };
                let Some((erasing, previous)) = step else {
                    self.unprocessed_input.raise(
                        if phase == InkPointerPhase::Moved {
                            InkPointerPhase::Hovered
                        } else {
                            phase
                        },
                        e,
                    );
                    return Ok(e.handled());
                };
                // Retire this pointer before erased/ended callbacks can start
                // another stroke with the same device identity.
                let released = if phase == InkPointerPhase::Released {
                    self.active.borrow_mut().remove(&e.pointer_id)
                } else {
                    None
                };
                if erasing {
                    if let Err(error) = self.erase_between(previous, point.position) {
                        self.invalidate();
                        return Err(error);
                    }
                }
                if phase == InkPointerPhase::Released {
                    if let Some(stroke) = released.filter(|s| !s.erasing) {
                        let epoch = self.epoch.get();
                        self.stroke_input.ended.raise(e);
                        if epoch == self.epoch.get() && self.enabled.get() {
                            let completed = match InkStroke::new(stroke.points, stroke.attributes) {
                                Ok(completed) => completed,
                                Err(error) => {
                                    self.invalidate();
                                    return Err(error);
                                }
                            };
                            if self.custom_drying.get() {
                                self.pending_dry.borrow_mut().push(completed.clone());
                            }
                            if let Err(error) = self.container.add_stroke(completed.clone()) {
                                self.pending_dry
                                    .borrow_mut()
                                    .retain(|pending| !Rc::ptr_eq(pending, &completed));
                                self.invalidate();
                                return Err(error);
                            }
                            self.collected.raise(&InkStrokesCollectedEventArgs {
                                strokes: vec![completed],
                            });
                        }
                    }
                } else if !erasing {
                    self.stroke_input.continued.raise(e);
                }
                self.invalidate();
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    fn erase_between(
        &self,
        from: ferroui_base::Point,
        to: ferroui_base::Point,
    ) -> Result<(), InkError> {
        let strokes = self.container.erase(from, to, 8.0)?;
        if !strokes.is_empty() {
            self.pending_dry
                .borrow_mut()
                .retain(|s| !strokes.iter().any(|v| v.id() == s.id()));
            self.erased.raise(&InkStrokesErasedEventArgs { strokes });
        }
        Ok(())
    }
    /// A snapshot for a renderer. Custom-dried strokes disappear only after EndDry.
    pub fn visible_strokes(&self) -> Vec<Rc<InkStroke>> {
        let mut strokes = if self.custom_drying.get() {
            self.pending_dry_strokes()
        } else {
            self.container.get_strokes()
        };
        let mut active = self.active.borrow_mut();
        for stroke in active.values_mut().filter(|s| !s.erasing) {
            if stroke.preview.is_none() {
                stroke.preview = InkStroke::new(stroke.points.clone(), stroke.attributes).ok();
            }
            if let Some(preview) = &stroke.preview {
                strokes.push(preview.clone());
            }
        }
        strokes
    }
}
