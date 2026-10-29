use crate::{catch_reload, PreparedReload, ReloadError};
use ferroui_base::controls::{resource_preview::ResourcePreviewScope, IResourceNode};
use std::{cell::RefCell, rc::Rc};

type Candidate = (Rc<dyn IResourceNode>, Box<dyn PreparedReload>);
enum State {
    Pending,
    Preparing,
    Ready(Rc<dyn IResourceNode>, Option<Box<dyn PreparedReload>>),
    Failed(ReloadError),
}
struct Preparation {
    original: Rc<dyn IResourceNode>,
    create: Box<dyn Fn() -> Result<Candidate, ReloadError>>,
    state: RefCell<State>,
}
/// Lazily prepares one resource contribution, shared by all candidate consumers.
/// Register all contributions before resolving any so dependencies are independent
/// of target registration order. Cycles and ambiguous ownership fail preparation.
#[derive(Clone)]
pub struct ResourcePreparation(Rc<Preparation>);
impl ResourcePreparation {
    pub fn new(
        original: Rc<dyn IResourceNode>,
        create: impl Fn() -> Result<Candidate, ReloadError> + 'static,
    ) -> Self {
        Self(Rc::new(Preparation {
            original,
            create: Box::new(create),
            state: RefCell::new(State::Pending),
        }))
    }
    pub(crate) fn register(&self, scope: &ResourcePreviewScope) -> Result<(), ReloadError> {
        let candidate = self.clone();
        if scope.insert(
            &*self.0.original,
            Rc::new(move || {
                candidate
                    .resolve()
                    .unwrap_or_else(|error| std::panic::panic_any(error))
            }),
        ) {
            Ok(())
        } else {
            Err(ReloadError::RestartRequired(
                "Two adapters own the same resource contribution".into(),
            ))
        }
    }
    fn resolve(&self) -> Result<Rc<dyn IResourceNode>, ReloadError> {
        match &*self.0.state.borrow() {
            State::Ready(node, _) => return Ok(node.clone()),
            State::Failed(error) => return Err(error.clone()),
            State::Preparing => {
                return Err(ReloadError::Failed(
                    "Cyclic resource candidate dependency".into(),
                ))
            }
            State::Pending => {}
        }
        *self.0.state.borrow_mut() = State::Preparing;
        match catch_reload(|| (self.0.create)()) {
            Ok((node, change)) => {
                *self.0.state.borrow_mut() = State::Ready(node.clone(), Some(change));
                Ok(node)
            }
            Err(error) => {
                *self.0.state.borrow_mut() = State::Failed(error.clone());
                Err(error)
            }
        }
    }
    pub(crate) fn take_change(&self) -> Result<Box<dyn PreparedReload>, ReloadError> {
        self.resolve()?;
        match &mut *self.0.state.borrow_mut() {
            State::Ready(_, change) => change
                .take()
                .ok_or_else(|| ReloadError::Failed("Resource candidate consumed twice".into())),
            _ => unreachable!("resolved candidate"),
        }
    }
}
