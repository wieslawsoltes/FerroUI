//! Owner-thread tracing, profiling and breakpoints at QML declaration boundaries.
use super::*;
use std::cell::RefCell;
use std::time::Duration;
use web_time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Kind {
    Binding,
    Method,
    Handler,
}

/// A declaration in the original QML input. Spans are half-open UTF-8 byte ranges.
/// Host-created bindings can have no source span, and in-memory documents can have no URL.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Site {
    pub source: Option<String>,
    pub span: Option<ir::SourceSpan>,
    pub kind: Kind,
    pub name: String,
}
impl Site {
    pub(crate) fn new(
        model: &Instance,
        owner: ObjectId,
        scope: Option<u32>,
        kind: Kind,
        name: &str,
        span: Option<ir::SourceSpan>,
    ) -> Self {
        Self {
            source: model
                .node(owner)
                .and_then(|node| {
                    scope
                        .and_then(|scope| {
                            node.source_layers
                                .iter()
                                .find(|layer| layer.scope == Some(scope))
                                .map(|layer| layer.url.as_str())
                                .or_else(|| model.declaration_key(scope))
                        })
                        .or_else(|| node.scoped_url(scope))
                })
                .map(str::to_owned),
            span,
            kind,
            name: name.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Breakpoint {
    /// Stop in the declaration whose original span contains this byte offset.
    Source {
        url: String,
        offset: usize,
    },
    Member {
        kind: Kind,
        name: String,
    },
}
impl Breakpoint {
    fn matches(&self, site: &Site) -> bool {
        match self {
            Self::Source { url, offset } => {
                site.source.as_ref() == Some(url)
                    && site
                        .span
                        .is_some_and(|span| (span.start..span.end).contains(offset))
            }
            Self::Member { kind, name } => site.kind == *kind && site.name == *name,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Resume {
    Continue,
    StepInto,
    StepOver,
    StepOut,
    /// Fail the current transaction and run its usual rollback.
    Abort,
}

pub struct Frame<'a> {
    pub site: &'a Site,
    pub receiver: ObjectId,
    /// Outermost instrumented frame has depth zero.
    pub depth: usize,
    /// The current transaction's object graph, including tentative values.
    pub instance: &'a Instance,
    pub parameters: &'a [Parameter],
    pub arguments: &'a [Value],
}

#[derive(Clone, Copy, Debug)]
pub enum Phase<'a> {
    Enter,
    Exit {
        elapsed: Duration,
        error: Option<&'a str>,
    },
}

pub struct Event<'a> {
    pub site: &'a Site,
    pub receiver: ObjectId,
    pub depth: usize,
    pub phase: Phase<'a>,
}

#[derive(Clone, Default, Debug)]
pub struct Metrics {
    pub calls: u64,
    pub failures: u64,
    /// Wall time including nested QML work and guest execution.
    pub inclusive: Duration,
    /// Wall time spent outside nested instrumented declarations.
    pub exclusive: Duration,
    pub longest: Duration,
}

type TraceHook = Rc<dyn Fn(&Event<'_>)>;
type PauseHook = Rc<dyn Fn(&Frame<'_>) -> Resume>;

#[derive(Default)]
struct State {
    trace: Option<TraceHook>,
    pause: Option<PauseHook>,
    breaks: Vec<Breakpoint>,
    next_depth: Option<usize>,
    profile: BTreeMap<Site, Metrics>,
    child_times: Vec<Duration>,
    hook_time: Duration,
}

/// A cloneable handle. Profiling begins when this handle is attached to an instance.
/// Configure the pause callback before setting a breakpoint or requesting a step.
#[derive(Clone, Default)]
pub struct Debugger(Rc<RefCell<State>>);

impl Debugger {
    /// Receive paired entry and exit notifications, including failures.
    pub fn on_trace(&self, hook: impl Fn(&Event<'_>) + 'static) {
        self.0.borrow_mut().trace = Some(Rc::new(hook));
    }

    /// Execution waits for the callback to return a resume command. A frontend
    /// may inspect the frame and wait for a command from another thread here.
    pub fn on_pause(&self, hook: impl Fn(&Frame<'_>) -> Resume + 'static) {
        self.0.borrow_mut().pause = Some(Rc::new(hook));
    }

    pub fn set_breakpoints(&self, breakpoints: impl IntoIterator<Item = Breakpoint>) {
        self.0.borrow_mut().breaks = breakpoints.into_iter().collect();
    }

    pub fn pause_next(&self) {
        self.0.borrow_mut().next_depth = Some(usize::MAX);
    }

    pub fn profile(&self) -> BTreeMap<Site, Metrics> {
        self.0.borrow().profile.clone()
    }

    /// Drain completed measurements. Frames still running appear on their exit.
    pub fn take_profile(&self) -> BTreeMap<Site, Metrics> {
        std::mem::take(&mut self.0.borrow_mut().profile)
    }

    pub(crate) fn enter(
        &self,
        instance: &Instance,
        receiver: ObjectId,
        site: Site,
        parameters: &[Parameter],
        arguments: &[Value],
    ) -> Call {
        let (depth, trace, pause) = {
            let mut state = self.0.borrow_mut();
            let depth = state.child_times.len();
            state.child_times.push(Duration::ZERO);
            let stop = state.next_depth.is_some_and(|target| depth <= target)
                || state.breaks.iter().any(|b| b.matches(&site));
            (
                depth,
                state.trace.clone(),
                state.pause.clone().filter(|_| stop),
            )
        };
        let before = Instant::now();
        if let Some(trace) = trace {
            trace(&Event {
                site: &site,
                receiver,
                depth,
                phase: Phase::Enter,
            });
        }
        let mut abort = false;
        if let Some(pause) = pause {
            let command = pause(&Frame {
                site: &site,
                receiver,
                depth,
                instance,
                parameters,
                arguments,
            });
            abort = command == Resume::Abort;
            self.0.borrow_mut().next_depth = match command {
                Resume::Continue | Resume::Abort => None,
                Resume::StepInto => Some(usize::MAX),
                Resume::StepOver => Some(depth),
                Resume::StepOut => depth.checked_sub(1),
            };
        }
        let mut state = self.0.borrow_mut();
        state.hook_time += before.elapsed();
        Call {
            debugger: self.clone(),
            receiver,
            site,
            depth,
            start: Instant::now(),
            hook_time: state.hook_time,
            abort,
        }
    }
}

pub(crate) struct Call {
    debugger: Debugger,
    receiver: ObjectId,
    site: Site,
    depth: usize,
    start: Instant,
    hook_time: Duration,
    abort: bool,
}
impl Call {
    pub(crate) fn execute<T>(self, op: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
        let result = if self.abort {
            Err(Error::new("execution aborted by QML debugger"))
        } else {
            op()
        };
        let (elapsed, trace) = {
            let mut state = self.debugger.0.borrow_mut();
            let elapsed = self
                .start
                .elapsed()
                .saturating_sub(state.hook_time - self.hook_time);
            let nested = state.child_times.pop().expect("active diagnostic frame");
            if let Some(parent) = state.child_times.last_mut() {
                *parent += elapsed;
            }
            let record = state.profile.entry(self.site.clone()).or_default();
            record.calls += 1;
            record.failures += u64::from(result.is_err());
            record.longest = record.longest.max(elapsed);
            record.inclusive += elapsed;
            record.exclusive += elapsed.saturating_sub(nested);
            (elapsed, state.trace.clone())
        };
        if let Some(trace) = trace {
            let before = Instant::now();
            trace(&Event {
                site: &self.site,
                receiver: self.receiver,
                depth: self.depth,
                phase: Phase::Exit {
                    elapsed,
                    error: result.as_ref().err().map(|e| e.message.as_str()),
                },
            });
            self.debugger.0.borrow_mut().hook_time += before.elapsed();
        }
        result
    }
}

impl Instance {
    /// Attach or detach at a host transaction boundary.
    pub fn set_debugger(&mut self, debugger: Option<Debugger>) {
        self.diagnostics = debugger;
    }

    /// Attach before initial bindings and lifecycle handlers run.
    pub fn with_debugger(
        source: &impl ComponentSource,
        registry: &TypeRegistry,
        initial: BTreeMap<String, Value>,
        engine: Option<Rc<dyn ScriptEngine>>,
        debugger: Debugger,
    ) -> Result<Self, Error> {
        Self::construct(None, source, registry, initial, engine, |instance| {
            instance.set_debugger(Some(debugger))
        })
    }
}
