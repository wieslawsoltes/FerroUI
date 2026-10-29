use super::ink_platform_services::{IInkPlatformServices, InkPersistenceFormat, InkServiceFuture};
use super::ink_point::{valid_position, InkError};
use super::ink_stroke::{union_rect, InkStroke};
use super::persistence::{self, InkPersistenceLimits};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{Matrix, Point, Rect};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

#[derive(Default)]
struct ContainerState {
    strokes: Vec<Rc<InkStroke>>,
    by_id: HashMap<u32, Rc<InkStroke>>,
    subscriptions: HashMap<u32, Rc<dyn IDisposable>>,
    total_points: usize,
    geometry_vertices: usize,
}

/// UI-thread-affine stroke collection with stable identities and notifying
/// mutation. No collection borrow is held while user callbacks are invoked.
pub struct InkStrokeContainer {
    state: RefCell<ContainerState>,
    weak_self: Weak<Self>,
    changed: Rc<HandlerList<dyn Fn()>>,
    batch_depth: Cell<usize>,
    pending_change: Cell<bool>,
    platform_services: RefCell<Option<Rc<dyn IInkPlatformServices>>>,
}

impl PartialEq for InkStrokeContainer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl std::fmt::Debug for InkStrokeContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InkStrokeContainer")
            .field("count", &self.len())
            .finish()
    }
}

impl InkStrokeContainer {
    /// Maximum aggregate conservative contour-vertex reservation. Attribute
    /// edits cannot increase the reservation made when a stroke is admitted.
    /// Point storage is at most 64 MiB; vector and backend caches are extra.
    pub const MAX_RENDERED_VERTICES: usize = 4_194_304;

    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            state: RefCell::new(ContainerState::default()),
            weak_self: weak.clone(),
            changed: Rc::new(HandlerList::new()),
            batch_depth: Cell::new(0),
            pending_change: Cell::new(false),
            platform_services: RefCell::new(None),
        })
    }
    pub fn len(&self) -> usize {
        self.state.borrow().strokes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get_strokes(&self) -> Vec<Rc<InkStroke>> {
        self.state.borrow().strokes.clone()
    }
    pub fn get_stroke_by_id(&self, id: u32) -> Option<Rc<InkStroke>> {
        self.state.borrow().by_id.get(&id).cloned()
    }
    pub fn add_stroke(&self, stroke: Rc<InkStroke>) -> Result<(), InkError> {
        self.add_strokes([stroke])
    }
    pub fn add_strokes(
        &self,
        strokes: impl IntoIterator<Item = Rc<InkStroke>>,
    ) -> Result<(), InkError> {
        let limits = InkPersistenceLimits::DEFAULT;
        let strokes: Vec<_> = strokes.into_iter().take(limits.max_strokes + 1).collect();
        if strokes.len() > limits.max_strokes {
            return Err(InkError::LimitExceeded);
        }
        let mut seen = HashSet::with_capacity(strokes.len());
        let mut additional_points = 0usize;
        let mut additional_vertices = 0usize;
        {
            let state = self.state.borrow();
            if state
                .strokes
                .len()
                .checked_add(strokes.len())
                .is_none_or(|n| n > limits.max_strokes)
            {
                return Err(InkError::LimitExceeded);
            }
            for s in &strokes {
                if state.by_id.contains_key(&s.id()) || !seen.insert(s.id()) {
                    return Err(InkError::DuplicateStrokeId(s.id()));
                }
                additional_points = additional_points
                    .checked_add(s.ink_points().len())
                    .ok_or(InkError::LimitExceeded)?;
                additional_vertices = additional_vertices
                    .checked_add(s.geometry_vertex_bound())
                    .ok_or(InkError::LimitExceeded)?;
            }
            if state
                .total_points
                .checked_add(additional_points)
                .is_none_or(|n| n > limits.max_total_points)
            {
                return Err(InkError::LimitExceeded);
            }
            if state
                .geometry_vertices
                .checked_add(additional_vertices)
                .is_none_or(|n| n > Self::MAX_RENDERED_VERTICES)
            {
                return Err(InkError::LimitExceeded);
            }
        }
        if strokes.is_empty() {
            return Ok(());
        }
        {
            let mut state = self.state.borrow_mut();
            state.total_points += additional_points;
            state.geometry_vertices += additional_vertices;
            for s in strokes {
                let weak = self.weak_self.clone();
                let subscription = s.changed(move || {
                    if let Some(this) = weak.upgrade() {
                        this.notify();
                    }
                });
                state.subscriptions.insert(s.id(), subscription);
                state.by_id.insert(s.id(), s.clone());
                state.strokes.push(s);
            }
        }
        self.notify();
        Ok(())
    }
    pub fn clear(&self) {
        let previous = std::mem::take(&mut *self.state.borrow_mut());
        if previous.strokes.is_empty() {
            return;
        }
        for subscription in previous.subscriptions.into_values() {
            subscription.dispose();
        }
        self.notify();
    }
    pub fn bounding_rect(&self) -> Rect {
        bounds(self.get_strokes().iter().map(|s| s.bounding_rect()))
    }

    /// Deletes the selected strokes and returns their former bounds.
    pub fn delete_selected(&self) -> Rect {
        let selected: Vec<_> = self
            .get_strokes()
            .into_iter()
            .filter(|s| s.selected())
            .collect();
        let rect = bounds(selected.iter().map(|s| s.bounding_rect()));
        self.remove_strokes(&selected);
        rect
    }

    /// Moves a snapshot of the selection. All transforms are validated before
    /// the first mutation and the collection emits one change notification.
    pub fn move_selected(&self, translation: Point) -> Result<Rect, InkError> {
        if !valid_position(translation) {
            return Err(InkError::InvalidPoint);
        }
        let selected: Vec<_> = self
            .get_strokes()
            .into_iter()
            .filter(|s| s.selected())
            .collect();
        let delta = Matrix::create_translation(translation.x, translation.y);
        let transforms: Vec<_> = selected
            .iter()
            .map(|s| s.point_transform() * delta)
            .collect();
        for (s, m) in selected.iter().zip(&transforms) {
            super::ink_drawing_attributes::validate_affine(*m)?;
            if s.ink_points()
                .iter()
                .any(|p| !valid_position(p.position * *m))
            {
                return Err(InkError::InvalidTransform);
            }
        }
        let _batch = ChangeBatch::new(self);
        for (s, m) in selected.iter().zip(transforms) {
            s.set_point_transform(m)?;
        }
        Ok(bounds(selected.iter().map(|s| s.bounding_rect())))
    }
    pub fn select_with_line(&self, from: Point, to: Point) -> Rect {
        let snapshot = self.get_strokes();
        let _batch = ChangeBatch::new(self);
        for s in &snapshot {
            s.set_selected(s.hit_test_segment(from, to, 0.0));
        }
        bounds(
            snapshot
                .iter()
                .filter(|s| s.selected())
                .map(|s| s.bounding_rect()),
        )
    }
    pub fn select_with_poly_line(&self, points: impl AsRef<[Point]>) -> Rect {
        let points = points.as_ref();
        let valid = points.len() >= 3 && points.iter().all(|p| valid_position(*p));
        let snapshot = self.get_strokes();
        let _batch = ChangeBatch::new(self);
        for s in &snapshot {
            s.set_selected(valid && s.contained_in_lasso(points));
        }
        bounds(
            snapshot
                .iter()
                .filter(|s| s.selected())
                .map(|s| s.bounding_rect()),
        )
    }

    /// Erases whole strokes touched by the swept eraser. The returned snapshot
    /// retains the erased strokes for an undo record or StrokesErased event.
    pub fn erase(
        &self,
        from: Point,
        to: Point,
        radius: f64,
    ) -> Result<Vec<Rc<InkStroke>>, InkError> {
        if !valid_position(from) || !valid_position(to) || !radius.is_finite() || radius < 0.0 {
            return Err(InkError::InvalidInput(
                "Eraser geometry must be finite and radius nonnegative",
            ));
        }
        let erased: Vec<_> = self
            .get_strokes()
            .into_iter()
            .filter(|s| s.hit_test_segment(from, to, radius))
            .collect();
        self.remove_strokes(&erased);
        Ok(erased)
    }
    /// Removes these exact stroke instances. Serialized IDs may occur in
    /// different documents, so an unrelated instance with the same ID does
    /// not remove the stroke held by this collection.
    pub fn remove_strokes(&self, strokes: &[Rc<InkStroke>]) {
        if strokes.is_empty() {
            return;
        }
        let removed = {
            let mut state = self.state.borrow_mut();
            let ids: HashSet<_> = strokes
                .iter()
                .filter(|stroke| {
                    state
                        .by_id
                        .get(&stroke.id())
                        .is_some_and(|held| Rc::ptr_eq(held, stroke))
                })
                .map(|stroke| stroke.id())
                .collect();
            let mut removed = Vec::with_capacity(ids.len());
            state.strokes.retain(|s| !ids.contains(&s.id()));
            for id in ids {
                if let Some(stroke) = state.by_id.remove(&id) {
                    state.total_points -= stroke.ink_points().len();
                    state.geometry_vertices -= stroke.geometry_vertex_bound();
                }
                if let Some(subscription) = state.subscriptions.remove(&id) {
                    removed.push(subscription);
                }
            }
            removed
        };
        if removed.is_empty() {
            return;
        }
        for subscription in removed {
            subscription.dispose();
        }
        self.notify();
    }

    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.changed);
        Disposable::create(move || {
            if let Some(list) = weak.upgrade() {
                list.remove(token);
            }
        })
    }
    fn notify(&self) {
        if self.batch_depth.get() != 0 {
            self.pending_change.set(true);
            return;
        }
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
    }

    /// Saves the versioned native FerroUI ink format (not Windows ISF).
    pub fn save(&self) -> Result<Vec<u8>, InkError> {
        persistence::encode(&self.get_strokes(), InkPersistenceLimits::DEFAULT)
    }
    pub fn save_with_limits(&self, limits: InkPersistenceLimits) -> Result<Vec<u8>, InkError> {
        persistence::encode(&self.get_strokes(), limits)
    }
    /// Attaches an OS interoperability adapter. Drawing remains portable;
    /// native persistence and clipboard operations use this service only.
    pub fn set_platform_services(&self, value: Option<Rc<dyn IInkPlatformServices>>) {
        // An adapter's destructor can release subscriptions or reenter the
        // collection. Publish the replacement and release the borrow first.
        let previous = self.platform_services.replace(value);
        drop(previous);
    }
    pub fn platform_services(&self) -> Option<Rc<dyn IInkPlatformServices>> {
        self.platform_services.borrow().clone()
    }
    pub fn save_async(&self) -> InkServiceFuture<Vec<u8>> {
        self.save_with_format_async(InkPersistenceFormat::Portable)
    }
    pub fn save_with_format_async(
        &self,
        format: InkPersistenceFormat,
    ) -> InkServiceFuture<Vec<u8>> {
        let strokes = self.get_strokes();
        if let Err(error) = validate_imported_strokes(&strokes) {
            return Box::pin(async move { Err(error) });
        }
        // Freeze values at invocation, before an executor can yield to edits.
        // Native adapters likewise snapshot before returning their future.
        match format {
            InkPersistenceFormat::Portable => {
                let result = persistence::encode(&strokes, InkPersistenceLimits::DEFAULT);
                Box::pin(async move { result })
            }
            _ => match self.platform_services() {
                Some(service) => service.save(strokes, format),
                None => Box::pin(async { Err(InkError::UnsupportedFormat) }),
            },
        }
    }
    /// Replaces this collection only after the complete document validates.
    pub fn load(&self, data: &[u8]) -> Result<(), InkError> {
        self.load_with_limits(data, InkPersistenceLimits::DEFAULT)
    }
    pub fn load_with_limits(
        &self,
        data: &[u8],
        limits: InkPersistenceLimits,
    ) -> Result<(), InkError> {
        let strokes = persistence::decode(data, limits)?;
        self.replace_document(strokes)
    }
    /// Auto-detects native portable ink; other representations are delegated to
    /// the attached OS service. The old document survives all decode failures.
    pub fn load_async(&self, data: Vec<u8>) -> InkServiceFuture<()> {
        let this = self.weak_self.upgrade();
        let service = self.platform_services();
        Box::pin(async move {
            if data.len() > InkPersistenceLimits::DEFAULT.max_bytes {
                return Err(InkError::LimitExceeded);
            }
            let strokes = if data.starts_with(b"FERROINK") {
                persistence::decode(&data, InkPersistenceLimits::DEFAULT)?
            } else {
                let service = service.ok_or(InkError::UnsupportedFormat)?;
                service.load(data).await?
            };
            let this = this.ok_or(InkError::InvalidOperation(
                "The ink collection is no longer available",
            ))?;
            this.replace_document(strokes)
        })
    }

    /// Produces a portable clipboard payload. Platform clipboard adapters own
    /// transport; this method never accesses a process-global clipboard.
    pub fn copy_selected(&self) -> Result<Vec<u8>, InkError> {
        let selected: Vec<_> = self
            .get_strokes()
            .into_iter()
            .filter(|s| s.selected())
            .collect();
        persistence::encode(&selected, InkPersistenceLimits::DEFAULT)
    }
    /// Imports clipboard data with fresh stroke IDs and places its bounding
    /// rectangle at `position`. Existing selection is cleared on success only.
    pub fn paste(&self, data: &[u8], position: Point) -> Result<Rect, InkError> {
        if !valid_position(position) {
            return Err(InkError::InvalidPoint);
        }
        let imported = persistence::decode(data, InkPersistenceLimits::DEFAULT)?;
        validate_imported_strokes(&imported)?;
        if imported.is_empty() {
            return Ok(Rect::default());
        }
        let old_bounds = bounds(imported.iter().map(|s| s.bounding_rect()));
        let delta =
            Matrix::create_translation(position.x - old_bounds.x, position.y - old_bounds.y);
        let mut copies = Vec::with_capacity(imported.len());
        for s in imported {
            let copy = s.clone_stroke()?;
            copy.set_point_transform(copy.point_transform() * delta)?;
            copy.set_selected(true);
            copies.push(copy);
        }
        self.commit_paste(copies)
    }
    pub fn can_paste(data: &[u8]) -> bool {
        persistence::decode(data, InkPersistenceLimits::DEFAULT)
            .is_ok_and(|strokes| !strokes.is_empty() && validate_imported_strokes(&strokes).is_ok())
    }
    pub fn copy_selected_to_clipboard(&self) -> Result<(), InkError> {
        let service = self
            .platform_services()
            .ok_or(InkError::UnsupportedFormat)?;
        let selected: Vec<_> = self
            .get_strokes()
            .into_iter()
            .filter(|s| s.selected())
            .collect();
        service.copy_to_clipboard(&selected)
    }
    pub fn can_paste_from_clipboard(&self) -> Result<bool, InkError> {
        match self.platform_services() {
            Some(service) => service.can_paste(),
            None => Ok(false),
        }
    }
    pub fn paste_from_clipboard(&self, position: Point) -> Result<Rect, InkError> {
        if !valid_position(position) {
            return Err(InkError::InvalidPoint);
        }
        let service = self
            .platform_services()
            .ok_or(InkError::UnsupportedFormat)?;
        let imported = service.paste(position)?;
        validate_imported_strokes(&imported)?;
        if imported.is_empty() {
            return Ok(Rect::default());
        }
        // The adapter performs positioning in its coordinate system; cloned
        // identities prevent collisions with existing copies of the same ink.
        let copies = imported
            .into_iter()
            .map(|stroke| {
                let copy = stroke.clone_stroke()?;
                copy.set_selected(true);
                Ok(copy)
            })
            .collect::<Result<Vec<_>, InkError>>()?;
        self.commit_paste(copies)
    }
    fn commit_paste(&self, copies: Vec<Rc<InkStroke>>) -> Result<Rect, InkError> {
        let result = bounds(copies.iter().map(|s| s.bounding_rect()));
        let previous = self.get_strokes();
        let _batch = ChangeBatch::new(self);
        // This is the only fallible mutation. Complete admission before a
        // selected stroke can notify application code. A reentrant clear or
        // edit then observes, and can intentionally change, the pasted ink.
        self.add_strokes(copies)?;
        for stroke in previous {
            stroke.set_selected(false);
        }
        Ok(result)
    }

    fn replace_document(&self, strokes: Vec<Rc<InkStroke>>) -> Result<(), InkError> {
        validate_imported_strokes(&strokes)?;
        let mut next = ContainerState {
            by_id: HashMap::with_capacity(strokes.len()),
            subscriptions: HashMap::with_capacity(strokes.len()),
            ..ContainerState::default()
        };
        for stroke in &strokes {
            let weak = self.weak_self.clone();
            let subscription = stroke.changed(move || {
                if let Some(this) = weak.upgrade() {
                    this.notify();
                }
            });
            next.total_points += stroke.ink_points().len();
            next.geometry_vertices += stroke.geometry_vertex_bound();
            next.by_id.insert(stroke.id(), stroke.clone());
            next.subscriptions.insert(stroke.id(), subscription);
        }
        next.strokes = strokes;
        let _batch = ChangeBatch::new(self);
        // Commit the complete document before releasing old objects. A stroke
        // event handler can own a resource whose destructor reenters us; it
        // must observe the replacement, never a temporarily empty document.
        let previous = self.state.replace(next);
        for subscription in previous.subscriptions.values() {
            subscription.dispose();
        }
        drop(previous);
        self.notify();
        Ok(())
    }
}

fn validate_imported_strokes(strokes: &[Rc<InkStroke>]) -> Result<(), InkError> {
    let limits = InkPersistenceLimits::DEFAULT;
    if strokes.len() > limits.max_strokes {
        return Err(InkError::LimitExceeded);
    }
    let mut seen = HashSet::with_capacity(strokes.len());
    let mut total = 0usize;
    let mut vertices = 0usize;
    for stroke in strokes {
        if !seen.insert(stroke.id()) {
            return Err(InkError::DuplicateStrokeId(stroke.id()));
        }
        total = total
            .checked_add(stroke.ink_points().len())
            .ok_or(InkError::LimitExceeded)?;
        vertices = vertices
            .checked_add(stroke.geometry_vertex_bound())
            .ok_or(InkError::LimitExceeded)?;
        if stroke.ink_points().len() > limits.max_points_per_stroke
            || total > limits.max_total_points
            || vertices > InkStrokeContainer::MAX_RENDERED_VERTICES
        {
            return Err(InkError::LimitExceeded);
        }
    }
    Ok(())
}

impl Drop for InkStrokeContainer {
    fn drop(&mut self) {
        for subscription in self.state.get_mut().subscriptions.values() {
            subscription.dispose();
        }
    }
}

struct ChangeBatch<'a>(&'a InkStrokeContainer);
impl<'a> ChangeBatch<'a> {
    fn new(container: &'a InkStrokeContainer) -> Self {
        container.batch_depth.set(container.batch_depth.get() + 1);
        Self(container)
    }
}
impl Drop for ChangeBatch<'_> {
    fn drop(&mut self) {
        self.0.batch_depth.set(self.0.batch_depth.get() - 1);
        if self.0.batch_depth.get() == 0 && self.0.pending_change.replace(false) {
            self.0.notify();
        }
    }
}

fn bounds(items: impl Iterator<Item = Rect>) -> Rect {
    items
        .filter(|rect| !rect.is_empty())
        .reduce(union_rect)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::super::{ink_drawing_attributes::InkDrawingAttributes, ink_point::InkPoint};
    use super::*;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    fn ready<T>(mut future: InkServiceFuture<T>) -> Result<T, InkError> {
        let mut context = Context::from_waker(Waker::noop());
        match Future::poll(future.as_mut(), &mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("test service unexpectedly yielded"),
        }
    }

    struct TestPlatform {
        save_format: Cell<Option<InkPersistenceFormat>>,
        saved_count: Cell<usize>,
        pasted: RefCell<Vec<Rc<InkStroke>>>,
        failure: Cell<bool>,
        on_drop: Option<Box<dyn Fn()>>,
    }
    impl TestPlatform {
        fn new() -> Rc<Self> {
            Rc::new(Self {
                save_format: Cell::new(None),
                saved_count: Cell::new(0),
                pasted: RefCell::new(Vec::new()),
                failure: Cell::new(false),
                on_drop: None,
            })
        }
    }
    impl Drop for TestPlatform {
        fn drop(&mut self) {
            if let Some(callback) = self.on_drop.take() {
                callback();
            }
        }
    }
    impl IInkPlatformServices for TestPlatform {
        fn save(
            &self,
            strokes: Vec<Rc<InkStroke>>,
            format: InkPersistenceFormat,
        ) -> InkServiceFuture<Vec<u8>> {
            self.save_format.set(Some(format));
            self.saved_count.set(strokes.len());
            Box::pin(async { Ok(vec![1, 2, 3]) })
        }
        fn load(&self, _data: Vec<u8>) -> InkServiceFuture<Vec<Rc<InkStroke>>> {
            let result = if self.failure.get() {
                Err(InkError::Platform("native load failed".into()))
            } else {
                Ok(self.pasted.borrow().clone())
            };
            Box::pin(async move { result })
        }
        fn copy_to_clipboard(&self, strokes: &[Rc<InkStroke>]) -> Result<(), InkError> {
            self.saved_count.set(strokes.len());
            Ok(())
        }
        fn can_paste(&self) -> Result<bool, InkError> {
            Ok(!self.pasted.borrow().is_empty())
        }
        fn paste(&self, _position: Point) -> Result<Vec<Rc<InkStroke>>, InkError> {
            Ok(self.pasted.borrow().clone())
        }
    }
    fn stroke(x: f64) -> Rc<InkStroke> {
        InkStroke::new(
            vec![
                // The nominal pressure, which draws the configured size.
                InkPoint::new(Point::new(x, 0.0), 0.5).unwrap(),
                InkPoint::new(Point::new(x + 10.0, 0.0), 0.5).unwrap(),
            ],
            InkDrawingAttributes::default(),
        )
        .unwrap()
    }
    #[test]
    fn duplicate_batch_is_atomic() {
        let c = InkStrokeContainer::new();
        let a = stroke(0.0);
        c.add_stroke(a.clone()).unwrap();
        assert!(c.add_strokes([stroke(20.0), a]).is_err());
        assert_eq!(c.len(), 1);
    }
    #[test]
    fn document_geometry_budget_rejects_add_and_load_atomically() {
        let imported: Vec<_> = (0..5)
            .map(|_| {
                InkStroke::new(
                    (0..2048)
                        .map(|index| {
                            InkPoint::new(Point::new(index as f64 * 32.0, 0.0), 1.0).unwrap()
                        })
                        .collect(),
                    Default::default(),
                )
                .unwrap()
            })
            .collect();
        let container = InkStrokeContainer::new();
        let original = stroke(0.0);
        container.add_stroke(original.clone()).unwrap();
        assert_eq!(
            container.add_strokes(imported.clone()),
            Err(InkError::LimitExceeded)
        );
        let data = persistence::encode(&imported, InkPersistenceLimits::DEFAULT).unwrap();
        assert!(!InkStrokeContainer::can_paste(&data));
        assert_eq!(
            container.paste(&data, Point::default()),
            Err(InkError::LimitExceeded)
        );
        assert_eq!(container.load(&data), Err(InkError::LimitExceeded));
        assert_eq!(container.len(), 1);
        assert!(Rc::ptr_eq(&container.get_strokes()[0], &original));
    }
    #[test]
    fn changed_handlers_can_reenter_collection() {
        let c = InkStrokeContainer::new();
        let weak = Rc::downgrade(&c);
        c.changed(move || {
            if let Some(c) = weak.upgrade() {
                if c.len() == 1 {
                    c.clear();
                }
            }
        });
        c.add_stroke(stroke(0.0)).unwrap();
        assert!(c.is_empty());
    }
    #[test]
    fn selection_and_move_emit_one_notification_each() {
        let c = InkStrokeContainer::new();
        c.add_strokes([stroke(0.0), stroke(20.0)]).unwrap();
        let count = Rc::new(Cell::new(0));
        let count2 = count.clone();
        c.changed(move || count2.set(count2.get() + 1));
        c.select_with_line(Point::new(-5.0, 0.0), Point::new(40.0, 0.0));
        assert_eq!(count.get(), 1);
        c.move_selected(Point::new(5.0, 0.0)).unwrap();
        assert_eq!(count.get(), 2);
        assert_eq!(c.bounding_rect().x, 4.0);
    }
    #[test]
    fn failed_load_preserves_existing_strokes() {
        let c = InkStrokeContainer::new();
        let s = stroke(0.0);
        c.add_stroke(s.clone()).unwrap();
        assert!(c.load(b"not ink").is_err());
        assert!(Rc::ptr_eq(&c.get_strokes()[0], &s));
    }
    #[test]
    fn erased_strokes_stop_invalidating_container() {
        let c = InkStrokeContainer::new();
        let s = stroke(0.0);
        c.add_stroke(s.clone()).unwrap();
        let removed = c
            .erase(Point::new(5.0, -10.0), Point::new(5.0, 10.0), 0.1)
            .unwrap();
        assert_eq!(removed.len(), 1);
        assert!(c.is_empty());
        let count = Rc::new(Cell::new(0));
        let count2 = count.clone();
        c.changed(move || count2.set(count2.get() + 1));
        s.set_selected(true);
        assert_eq!(count.get(), 0);
    }
    #[test]
    fn removal_does_not_confuse_instances_from_different_documents() {
        let original = InkStrokeContainer::new();
        let stroke = stroke(0.0);
        original.add_stroke(stroke.clone()).unwrap();
        let other = InkStrokeContainer::new();
        other.load(&original.save().unwrap()).unwrap();
        let restored = other.get_strokes().remove(0);
        assert_eq!(restored.id(), stroke.id());
        assert!(!Rc::ptr_eq(&restored, &stroke));
        let notifications = Rc::new(Cell::new(0));
        let observed = notifications.clone();
        original.changed(move || observed.set(observed.get() + 1));

        original.remove_strokes(&[restored]);
        assert_eq!(original.len(), 1);
        assert_eq!(notifications.get(), 0);
        original.remove_strokes(&[stroke]);
        assert!(original.is_empty());
        assert_eq!(notifications.get(), 1);
    }

    #[test]
    fn replacing_an_adapter_releases_it_without_a_collection_borrow() {
        let container = InkStrokeContainer::new();
        let observed = Rc::new(Cell::new(false));
        let flag = observed.clone();
        let weak = Rc::downgrade(&container);
        let mut platform = TestPlatform::new();
        Rc::get_mut(&mut platform).unwrap().on_drop = Some(Box::new(move || {
            let container = weak.upgrade().unwrap();
            assert!(container.platform_services().is_none());
            container.set_platform_services(Some(TestPlatform::new()));
            flag.set(true);
        }));
        container.set_platform_services(Some(platform));
        container.set_platform_services(None);
        assert!(observed.get());
        assert!(container.platform_services().is_some());
    }

    #[test]
    fn load_commits_before_releasing_the_previous_document() {
        struct InsertOnDrop {
            container: Weak<InkStrokeContainer>,
            stroke: Rc<InkStroke>,
            result: Rc<RefCell<Option<Result<(), InkError>>>>,
        }
        impl Drop for InsertOnDrop {
            fn drop(&mut self) {
                if let Some(container) = self.container.upgrade() {
                    self.result
                        .replace(Some(container.add_stroke(self.stroke.clone())));
                }
            }
        }

        let container = InkStrokeContainer::new();
        let replacement = stroke(50.0);
        let data =
            persistence::encode(&[replacement.clone()], InkPersistenceLimits::DEFAULT).unwrap();
        let result = Rc::new(RefCell::new(None));
        let guard = InsertOnDrop {
            container: Rc::downgrade(&container),
            stroke: replacement.clone(),
            result: result.clone(),
        };
        let previous = stroke(0.0);
        previous.changed(move || {
            let _ = &guard;
        });
        container.add_stroke(previous).unwrap();

        container.load(&data).unwrap();
        assert_eq!(container.len(), 1);
        let held = container.get_stroke_by_id(replacement.id()).unwrap();
        assert!(!Rc::ptr_eq(&held, &replacement));
        assert_eq!(
            *result.borrow(),
            Some(Err(InkError::DuplicateStrokeId(replacement.id())))
        );
    }
    #[test]
    fn paste_allocates_ids_and_places_visible_bounds() {
        let c = InkStrokeContainer::new();
        let s = stroke(0.0);
        s.set_selected(true);
        c.add_stroke(s.clone()).unwrap();
        let data = c.copy_selected().unwrap();
        let r = c.paste(&data, Point::new(50.0, 60.0)).unwrap();
        assert_eq!(r.position(), Point::new(50.0, 60.0));
        assert_eq!(c.len(), 2);
        assert_ne!(c.get_strokes()[0].id(), c.get_strokes()[1].id());
        assert!(!s.selected());
    }
    #[test]
    fn invisible_strokes_do_not_shift_visible_bounds_or_paste_placement() {
        let container = InkStrokeContainer::new();
        let visible = stroke(100.0);
        let invisible = InkStroke::new(
            vec![InkPoint::new(Point::default(), 0.0).unwrap()],
            Default::default(),
        )
        .unwrap();
        visible.set_selected(true);
        invisible.set_selected(true);
        container.add_strokes([invisible, visible.clone()]).unwrap();
        assert_eq!(container.bounding_rect(), visible.bounding_rect());
        let result = container
            .paste(&container.copy_selected().unwrap(), Point::new(50.0, 60.0))
            .unwrap();
        assert_eq!(result.position(), Point::new(50.0, 60.0));
        assert_eq!(result.size(), visible.bounding_rect().size());
    }
    #[test]
    fn paste_selection_callbacks_observe_the_committed_document() {
        for system_clipboard in [false, true] {
            let container = InkStrokeContainer::new();
            let original = stroke(0.0);
            original.set_selected(true);
            container.add_stroke(original.clone()).unwrap();
            let data = container.copy_selected().unwrap();
            let platform = TestPlatform::new();
            platform.pasted.replace(vec![original.clone()]);
            container.set_platform_services(Some(platform));
            let observed_count = Rc::new(Cell::new(0));
            let count = observed_count.clone();
            let weak = Rc::downgrade(&container);
            original.changed(move || {
                if let Some(container) = weak.upgrade() {
                    count.set(container.len());
                    // Application code may intentionally clear the entire
                    // newly pasted selection in response to this event.
                    container.clear();
                }
            });

            if system_clipboard {
                container.paste_from_clipboard(Point::default()).unwrap();
            } else {
                container.paste(&data, Point::default()).unwrap();
            }
            assert_eq!(observed_count.get(), 2);
            assert!(container.is_empty());
        }
    }
    #[test]
    fn portable_async_io_does_not_require_a_platform_service() {
        let c = InkStrokeContainer::new();
        c.add_stroke(stroke(0.0)).unwrap();
        let data = ready(c.save_async()).unwrap();
        let copy = InkStrokeContainer::new();
        ready(copy.load_async(data)).unwrap();
        assert_eq!(copy.len(), 1);
        assert_eq!(
            ready(c.save_with_format_async(InkPersistenceFormat::Isf)),
            Err(InkError::UnsupportedFormat)
        );
        assert!(!c.can_paste_from_clipboard().unwrap());
    }
    #[test]
    fn async_save_captures_attributes_before_the_future_is_polled() {
        let container = InkStrokeContainer::new();
        let original = stroke(0.0);
        let attributes = original.drawing_attributes();
        container.add_stroke(original.clone()).unwrap();
        let save = container.save_async();
        let mut edited = attributes;
        edited.size.width = 20.0;
        original.set_drawing_attributes(edited).unwrap();
        container.clear();
        let restored = InkStrokeContainer::new();
        restored.load(&ready(save).unwrap()).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored.get_strokes()[0].drawing_attributes(), attributes);
    }
    #[test]
    fn native_io_delegates_explicit_format_and_preserves_failed_load() {
        let c = InkStrokeContainer::new();
        let original = stroke(0.0);
        c.add_stroke(original.clone()).unwrap();
        let platform = TestPlatform::new();
        c.set_platform_services(Some(platform.clone()));
        assert_eq!(
            ready(c.save_with_format_async(InkPersistenceFormat::GifWithEmbeddedIsf)).unwrap(),
            vec![1, 2, 3]
        );
        assert_eq!(
            platform.save_format.get(),
            Some(InkPersistenceFormat::GifWithEmbeddedIsf)
        );
        platform.failure.set(true);
        assert!(ready(c.load_async(vec![0])).is_err());
        assert!(Rc::ptr_eq(&c.get_strokes()[0], &original));
        platform.failure.set(false);
        platform
            .pasted
            .replace(vec![original.clone(), original.clone()]);
        assert!(matches!(
            ready(c.load_async(vec![0])),
            Err(InkError::DuplicateStrokeId(_))
        ));
        assert!(Rc::ptr_eq(&c.get_strokes()[0], &original));
    }
    #[test]
    fn system_clipboard_uses_only_selection_and_pastes_new_ids() {
        let c = InkStrokeContainer::new();
        let a = stroke(0.0);
        let b = stroke(20.0);
        a.set_selected(true);
        c.add_strokes([a.clone(), b]).unwrap();
        let platform = TestPlatform::new();
        platform.pasted.replace(vec![a.clone()]);
        c.set_platform_services(Some(platform.clone()));
        c.copy_selected_to_clipboard().unwrap();
        assert_eq!(platform.saved_count.get(), 1);
        assert!(c.can_paste_from_clipboard().unwrap());
        c.paste_from_clipboard(Point::default()).unwrap();
        assert_eq!(c.len(), 3);
        assert!(!a.selected());
        assert_ne!(c.get_strokes()[2].id(), a.id());
        assert!(c.get_strokes()[2].selected());
    }
}
