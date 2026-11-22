use super::Phase;
use crate::rendering::IPresentationSource;
use crate::{Ref, Visual, WeakRef};
use rustc_hash::FxHashMap;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

const MAX_ROWS: usize = 1024;
pub(crate) const MAX_DURATION: Duration = Duration::from_secs(30);

crate::ferro_thread_local! {
    static ACTIVE: RefCell<Option<Weak<Capture>>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Metric {
    pub calls: u64,
    pub inclusive: Duration,
    pub own: Duration,
    pub longest_own: Duration,
}

#[derive(Clone)]
pub struct Row {
    pub name: &'static str,
    pub is_component: bool,
    pub visual: Option<WeakRef<Visual>>,
    pub metrics: [Metric; Phase::ALL.len()],
}

pub struct Snapshot {
    pub elapsed: Duration,
    pub recording: bool,
    pub omitted_calls: u64,
    pub rows: Vec<Row>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Identity {
    Control(usize),
    Component(usize),
}

struct Frame {
    started: Instant,
    nested: Duration,
    destination: Destination,
    phase: Phase,
}

enum Destination {
    Row(usize),
    Limit,
    OtherWindow,
}

#[derive(Default)]
struct Records {
    index: FxHashMap<Identity, usize>,
    rows: Vec<Row>,
    // A weak reference keeps an expired instance's address reserved until this
    // capture is retired. It never keeps the application object alive.
    components: Vec<Weak<dyn Any>>,
    stack: Vec<Frame>,
    omitted_calls: u64,
    frozen: bool,
}

/// One bounded capture on the current UI thread. Layout and view hooks never
/// hold a borrow of the recorder while running application code.
pub struct Capture {
    source: Weak<dyn IPresentationSource>,
    started: Instant,
    deadline: Instant,
    stopped: Cell<Option<Instant>>,
    records: RefCell<Records>,
}

impl Capture {
    pub fn start(
        source: Rc<dyn IPresentationSource>,
        duration: Duration,
    ) -> Result<Rc<Self>, &'static str> {
        if cfg!(target_arch = "wasm32") {
            return Err("UI profiling requires a native runtime");
        }
        if duration.is_zero() || duration > MAX_DURATION {
            return Err("Choose a profile duration of at most 30 seconds");
        }
        let started = Instant::now();
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active
                .as_ref()
                .and_then(Weak::upgrade)
                .is_some_and(|capture| capture.recording_at(started))
            {
                return Err("A UI profile is already recording");
            }
            let capture = Rc::new(Self {
                source: Rc::downgrade(&source),
                started,
                deadline: started + duration,
                stopped: Cell::new(None),
                records: RefCell::new(Records::default()),
            });
            *active = Some(Rc::downgrade(&capture));
            Ok(capture)
        })
    }

    pub fn stop(&self) {
        if self.stopped.get().is_none() {
            self.stopped.set(Some(Instant::now().min(self.deadline)));
            self.records.borrow_mut().frozen = true;
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let now = Instant::now();
        let recording = self.recording_at(now);
        let mut records = self.records.borrow_mut();
        records.frozen |= !recording;
        Snapshot {
            elapsed: now.min(self.end()).saturating_duration_since(self.started),
            recording,
            omitted_calls: records.omitted_calls,
            rows: records.rows.clone(),
        }
    }

    fn end(&self) -> Instant {
        self.stopped.get().unwrap_or(self.deadline)
    }

    fn recording_at(&self, now: Instant) -> bool {
        self.stopped.get().is_none() && now < self.deadline
    }

    /// Checks the window's presentation source, including its internal visual host.
    pub fn contains(&self, visual: &Visual) -> bool {
        visual
            .presentation_source()
            .is_some_and(|source| std::ptr::addr_eq(self.source.as_ptr(), Rc::as_ptr(&source)))
    }

    fn begin(
        self: Rc<Self>,
        identity: Identity,
        phase: Phase,
        make_row: impl FnOnce(&mut Records) -> Row,
    ) -> Span {
        let depth = {
            let mut records = self.records.borrow_mut();
            let row = records.index.get(&identity).copied().or_else(|| {
                if records.rows.len() == MAX_ROWS {
                    return None;
                }
                let row = make_row(&mut records);
                let index = records.rows.len();
                records.rows.push(row);
                records.index.insert(identity, index);
                Some(index)
            });
            let depth = records.stack.len();
            records.stack.push(Frame {
                started: Instant::now(),
                nested: Duration::ZERO,
                destination: row.map_or(Destination::Limit, Destination::Row),
                phase,
            });
            depth
        };
        Span {
            capture: self,
            depth,
        }
    }

    fn exclude_nested(self: Rc<Self>, phase: Phase) -> Option<Span> {
        let depth = {
            let mut records = self.records.borrow_mut();
            let depth = records.stack.len();
            if depth == 0 {
                return None;
            }
            records.stack.push(Frame {
                started: Instant::now(),
                nested: Duration::ZERO,
                destination: Destination::OtherWindow,
                phase,
            });
            depth
        };
        Some(Span {
            capture: self,
            depth,
        })
    }
}

/// The execution of a single synchronous UI operation.
pub struct Span {
    capture: Rc<Capture>,
    depth: usize,
}

impl Span {
    /// Associates the recorded component with the control it just committed.
    pub fn set_visual(&self, output: Option<Ref<Visual>>) {
        let Some(output) = output.filter(|view| self.capture.contains(view)) else {
            return;
        };
        let mut records = self.capture.records.borrow_mut();
        if !records.frozen {
            if let Destination::Row(row) = records.stack[self.depth].destination {
                records.rows[row].visual = Some(output.downgrade());
            }
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let ended = Instant::now().min(self.capture.end());
        let mut records = self.capture.records.borrow_mut();
        debug_assert_eq!(records.stack.len(), self.depth + 1);
        let frame = records.stack.pop().expect("profile scope");
        let elapsed = ended.saturating_duration_since(frame.started);
        if let Some(parent) = records.stack.last_mut() {
            parent.nested += elapsed;
        }
        if records.frozen {
            return;
        }
        match frame.destination {
            Destination::Row(row) => {
                let metric = &mut records.rows[row].metrics[frame.phase as usize];
                let own = elapsed.saturating_sub(frame.nested);
                metric.calls += 1;
                metric.inclusive += elapsed;
                metric.own += own;
                metric.longest_own = metric.longest_own.max(own);
            }
            Destination::Limit => records.omitted_calls += 1,
            Destination::OtherWindow => {}
        }
    }
}

fn active() -> Option<Rc<Capture>> {
    let capture = ACTIVE.with(|active| active.borrow().as_ref().and_then(Weak::upgrade))?;
    let now = Instant::now();
    if capture.recording_at(now) {
        Some(capture)
    } else {
        ACTIVE.with(|active| active.borrow_mut().take());
        None
    }
}

pub(super) fn layout(control: &Visual, phase: Phase) -> Option<Span> {
    let capture = active()?;
    if !capture.contains(control) {
        return capture.exclude_nested(phase);
    }
    let identity = Identity::Control(control as *const Visual as usize);
    Some(capture.begin(identity, phase, |_| Row {
        name: control.get_type().name(),
        is_component: false,
        visual: Some(control.to_ref().downgrade()),
        metrics: Default::default(),
    }))
}

pub(super) fn component<T: Any>(
    instance: &Rc<T>,
    name: &'static str,
    phase: Phase,
    views: impl FnOnce() -> (Option<Ref<Visual>>, Option<Ref<Visual>>),
) -> Option<Span> {
    let capture = active()?;
    let (output, host) = views();
    let output = output.filter(|view| capture.contains(view));
    if output.is_none() && !host.is_some_and(|host| capture.contains(&host)) {
        return capture.exclude_nested(phase);
    }
    let identity = Identity::Component(Rc::as_ptr(instance) as usize);
    let span = capture.begin(identity, phase, |records| {
        let weak: Weak<dyn Any> = Rc::<T>::downgrade(instance);
        records.components.push(weak);
        Row {
            name,
            is_component: true,
            visual: None,
            metrics: Default::default(),
        }
    });
    span.set_visual(output);
    Some(span)
}
