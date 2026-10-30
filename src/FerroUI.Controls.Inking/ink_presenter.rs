use super::ink_events::InkEvent;
use super::*;
use ferroui_base::input::PointerType;
use ferroui_base::reactive::IDisposable;
use ferroui_base::Size;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashSet};
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
    fragments: Vec<Vec<InkPoint>>,
    last_point: InkPoint,
    sample_count: usize,
    geometry_vertices: usize,
    attributes: InkDrawingAttributes,
    last_event: InkPointerEventArgs,
    erasing: bool,
    /// The preview strokes of the completed fragments, built once each.
    fragment_previews: Vec<Rc<InkStroke>>,
    /// The preview stroke of the fragment being drawn, built again after each sample.
    tail_preview: Option<Rc<InkStroke>>,
}

impl ActiveStroke {
    fn append_visible(
        &mut self,
        parts: Vec<Vec<InkPoint>>,
        point: InkPoint,
        geometry_limit: usize,
        point_limit: usize,
    ) -> Result<(), InkError> {
        self.last_point = point;
        self.tail_preview = None;
        for part in parts.into_iter().filter(|part| !part.is_empty()) {
            if !self.points.is_empty() && self.points.last() != part.first() {
                self.fragments.push(std::mem::take(&mut self.points));
            }
            for sample in part {
                if self.points.last() != Some(&sample) {
                    if self.sample_count
                        >= point_limit.min(InkPersistenceLimits::DEFAULT.max_points_per_stroke)
                    {
                        return Err(InkError::LimitExceeded);
                    }
                    let additional = self.points.last().map_or(32, |previous| {
                        super::ink_stroke::geometry_segment_vertex_bound(*previous, sample)
                    });
                    let vertices = self
                        .geometry_vertices
                        .checked_add(additional)
                        .ok_or(InkError::LimitExceeded)?;
                    if vertices > geometry_limit.min(InkStroke::MAX_RENDERED_VERTICES) {
                        return Err(InkError::LimitExceeded);
                    }
                    self.points.push(sample);
                    self.sample_count += 1;
                    self.geometry_vertices = vertices;
                }
            }
        }
        // The next outside point must start a new fragment after an interior gap.
        if !self.points.is_empty() && self.points.last() != Some(&point) {
            self.fragments.push(std::mem::take(&mut self.points));
        }
        Ok(())
    }
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
    /// Bounds synthetic multi-pointer input as well as native device input.
    pub const MAX_ACTIVE_POINTERS: usize = 256;

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
                let live: HashSet<_> = p.container.get_strokes().iter().map(Rc::as_ptr).collect();
                p.pending_dry
                    .borrow_mut()
                    .retain(|stroke| live.contains(&Rc::as_ptr(stroke)));
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
    pub(crate) fn is_inking(&self) -> bool {
        self.active.borrow().values().any(|stroke| !stroke.erasing)
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
        let active_ink = self
            .active
            .borrow()
            .get(&e.pointer_id)
            .is_some_and(|stroke| !stroke.erasing);
        let point = if phase == InkPointerPhase::Pressed || !active_ink {
            e.point
        } else {
            self.constrained_point(e.point)
        };
        match phase {
            InkPointerPhase::Pressed => {
                if self.has_active_pointer(e.pointer_id) {
                    let epoch = self.epoch.get();
                    self.cancel_pointer(e.pointer_id);
                    // Cancellation can synchronously disable input or start a
                    // replacement stroke. Do not overwrite that callback's work.
                    if self.has_active_pointer(e.pointer_id) {
                        return Ok(true);
                    }
                    if epoch != self.epoch.get()
                        || !self.enabled.get()
                        || !self.admits(e.device_type)
                        || (e.is_eraser && !self.input.is_eraser_input_enabled())
                        || (e.is_barrel_button_pressed
                            && !self.input.is_primary_barrel_button_input_enabled())
                    {
                        return Ok(false);
                    }
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
                {
                    let active = self.active.borrow();
                    let reserved = active.values().map(|s| s.geometry_vertices).sum::<usize>();
                    let points = active.values().map(|s| s.sample_count).sum::<usize>();
                    let initial_vertices = if erasing { 0 } else { 32 };
                    if active.len() >= Self::MAX_ACTIVE_POINTERS
                        || reserved + initial_vertices > InkStrokeContainer::MAX_RENDERED_VERTICES
                        || points >= InkPersistenceLimits::DEFAULT.max_total_points
                    {
                        return Err(InkError::LimitExceeded);
                    }
                }
                let point = if erasing {
                    point
                } else {
                    self.protractor.begin_ink();
                    self.constrained_point(point)
                };
                let visible = erasing
                    || (!self.ruler.blocks_ink(point.position)
                        && !self.protractor.blocks_ink(point.position));
                self.active.borrow_mut().insert(
                    e.pointer_id,
                    ActiveStroke {
                        points: if visible { vec![point] } else { Vec::new() },
                        fragments: Vec::new(),
                        last_point: point,
                        sample_count: usize::from(visible),
                        geometry_vertices: if visible && !erasing { 32 } else { 0 },
                        attributes: self.attributes.get(),
                        last_event: e.clone(),
                        erasing,
                        fragment_previews: Vec::new(),
                        tail_preview: None,
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
                let step = {
                    let mut active = self.active.borrow_mut();
                    let other_vertices = active
                        .iter()
                        .filter(|(id, _)| **id != e.pointer_id)
                        .map(|(_, stroke)| stroke.geometry_vertices)
                        .sum::<usize>();
                    let geometry_limit =
                        InkStrokeContainer::MAX_RENDERED_VERTICES.saturating_sub(other_vertices);
                    let other_points = active
                        .iter()
                        .filter(|(id, _)| **id != e.pointer_id)
                        .map(|(_, stroke)| stroke.sample_count)
                        .sum::<usize>();
                    let point_limit = InkPersistenceLimits::DEFAULT
                        .max_total_points
                        .saturating_sub(other_points);
                    active
                        .get_mut(&e.pointer_id)
                        .map(|stroke| {
                            let previous = stroke.last_point;
                            if point.timestamp < previous.timestamp {
                                return Err(InkError::InvalidInput(
                                    "Ink timestamps must be nondecreasing",
                                ));
                            }
                            if previous != point {
                                if stroke.erasing {
                                    stroke.points.clear();
                                    stroke.points.push(point);
                                    stroke.last_point = point;
                                } else {
                                    let parts = super::ink_presenter_stencil::clip_stencil_segment(
                                        previous,
                                        point,
                                        &self.ruler,
                                        &self.protractor,
                                    );
                                    stroke.append_visible(
                                        parts,
                                        point,
                                        geometry_limit,
                                        point_limit,
                                    )?;
                                }
                            }
                            stroke.last_event = e.clone();
                            Ok::<_, InkError>((stroke.erasing, previous.position))
                        })
                        .transpose()
                };
                let step = match step {
                    Ok(step) => step,
                    Err(error) => {
                        self.cancel_pointer(e.pointer_id);
                        return Err(error);
                    }
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
                    if let Some(mut stroke) = released.filter(|s| !s.erasing) {
                        let epoch = self.epoch.get();
                        self.stroke_input.ended.raise(e);
                        if epoch == self.epoch.get() && self.enabled.get() {
                            if !stroke.points.is_empty() {
                                stroke.fragments.push(stroke.points);
                            }
                            let completed = match stroke
                                .fragments
                                .into_iter()
                                .map(|points| InkStroke::new(points, stroke.attributes))
                                .collect::<Result<Vec<_>, _>>()
                            {
                                Ok(completed) => completed,
                                Err(error) => {
                                    self.invalidate();
                                    return Err(error);
                                }
                            };
                            if self.custom_drying.get() {
                                self.pending_dry
                                    .borrow_mut()
                                    .extend(completed.iter().cloned());
                            }
                            if let Err(error) =
                                self.container.add_strokes(completed.iter().cloned())
                            {
                                let rejected: HashSet<_> =
                                    completed.iter().map(Rc::as_ptr).collect();
                                self.pending_dry
                                    .borrow_mut()
                                    .retain(|pending| !rejected.contains(&Rc::as_ptr(pending)));
                                self.invalidate();
                                return Err(error);
                            }
                            if !completed.is_empty() {
                                self.collected
                                    .raise(&InkStrokesCollectedEventArgs { strokes: completed });
                            }
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
            // A completed fragment does not change, so its preview is built once; only the
            // fragment being drawn is built again (each sample used to rebuild them all).
            let mut complete = true;
            while stroke.fragment_previews.len() < stroke.fragments.len() {
                let points = &stroke.fragments[stroke.fragment_previews.len()];
                match InkStroke::new(points.clone(), stroke.attributes) {
                    Ok(preview) => stroke.fragment_previews.push(preview),
                    Err(_) => {
                        complete = false;
                        break;
                    }
                }
            }
            if stroke.tail_preview.is_none() && !stroke.points.is_empty() {
                match InkStroke::new(stroke.points.clone(), stroke.attributes) {
                    Ok(preview) => stroke.tail_preview = Some(preview),
                    Err(_) => complete = false,
                }
            }
            if complete {
                strokes.extend(stroke.fragment_previews.iter().cloned());
                strokes.extend(stroke.tail_preview.iter().cloned());
            }
        }
        strokes
    }
}
