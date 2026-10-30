use super::*;
use std::cell::{Cell, RefCell};

const URI: &str = "ferres://runtime-contract/View.xaml";
const BEFORE: &str = "<Panel><Button Name=\"Before\"/></Panel>";
const AFTER: &str = "<Panel><Button Name=\"After\"/><TextBlock Name=\"New\"/></Panel>";

struct Target {
    live: Rc<RefCell<String>>,
    fail_prepare: Rc<Cell<bool>>,
    fail_commit: Rc<Cell<bool>>,
}
struct Change {
    live: Rc<RefCell<String>>,
    before: String,
    after: String,
    fail_commit: bool,
}
impl ReloadTarget for Target {
    fn documents(&self) -> Vec<String> {
        vec![URI.into()]
    }
    fn prepare(&self, sources: Rc<Sources>) -> Result<Box<dyn PreparedReload>, ReloadError> {
        if self.fail_prepare.get() {
            return Err(ReloadError::Failed("prepare failed".into()));
        }
        Ok(Box::new(Change {
            live: self.live.clone(),
            before: self.live.borrow().clone(),
            after: sources[URI].clone(),
            fail_commit: self.fail_commit.get(),
        }))
    }
}
impl PreparedReload for Change {
    fn commit(&mut self) -> Result<(), ReloadError> {
        *self.live.borrow_mut() = self.after.clone();
        if self.fail_commit {
            Err(ReloadError::Failed("commit failed".into()))
        } else {
            Ok(())
        }
    }
    fn rollback(&mut self) {
        *self.live.borrow_mut() = self.before.clone();
    }
}

fn target(session: &mut HotReloadSession) -> (Rc<Target>, Rc<dyn ReloadTarget>) {
    let target = Rc::new(Target {
        live: Rc::new(RefCell::new(BEFORE.into())),
        fail_prepare: Rc::new(Cell::new(false)),
        fail_commit: Rc::new(Cell::new(false)),
    });
    let registered: Rc<dyn ReloadTarget> = target.clone();
    session.register_target(&registered);
    (target, registered)
}

#[test]
fn protected_sessions_reject_changed_names_before_preparing() {
    let mut session = HotReloadSession::new();
    session
        .register_document(URI.into(), BEFORE.into())
        .unwrap();
    let (target, _registered) = target(&mut session);
    target.fail_prepare.set(true);
    session.submit(URI.into(), AFTER.into());
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::RestartRequired
    );
    assert_eq!(&*target.live.borrow(), BEFORE);
    assert_eq!(session.sources()[URI], BEFORE);
}

#[test]
fn runtime_sessions_accept_structural_edits_after_atomic_commit() {
    let mut session = HotReloadSession::for_runtime_documents();
    session
        .register_document(URI.into(), BEFORE.into())
        .unwrap();
    let (target, _registered) = target(&mut session);
    session.submit(URI.into(), AFTER.into());
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    assert_eq!(&*target.live.borrow(), AFTER);
    assert_eq!(session.sources()[URI], AFTER);
    assert_eq!(
        session.contracts[URI],
        DocumentContract::parse(AFTER).unwrap()
    );
}

#[test]
fn runtime_contracts_and_sources_roll_back_failed_prepare_commit_and_finalize() {
    let mut session = HotReloadSession::for_runtime_documents();
    session
        .register_document(URI.into(), BEFORE.into())
        .unwrap();
    let (target, _registered) = target(&mut session);
    for failure in 0..3 {
        target.fail_prepare.set(failure == 0);
        target.fail_commit.set(failure == 1);
        session.submit(URI.into(), AFTER.into());
        let outcome = session
            .apply_pending_with(|| {
                if failure == 2 {
                    Err(ReloadError::Failed("finalize failed".into()))
                } else {
                    Ok(())
                }
            })
            .unwrap();
        assert_eq!(outcome.status, ReloadStatus::Failed);
        assert_eq!(&*target.live.borrow(), BEFORE);
        assert_eq!(session.sources()[URI], BEFORE);
        assert_eq!(
            session.contracts[URI],
            DocumentContract::parse(BEFORE).unwrap()
        );
    }
    session.submit(URI.into(), AFTER.into());
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
}

#[test]
fn runtime_sessions_cannot_install_compiled_class_overrides() {
    let mut session = HotReloadSession::for_runtime_documents();
    session
        .register_document(URI.into(), BEFORE.into())
        .unwrap();
    let error = session
        .bind_class(
            ferroui_controls::ContentControl::TYPE,
            URI.into(),
            ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::new(),
        )
        .unwrap_err();
    assert!(matches!(error, ReloadError::Failed(message) if message.contains("compiled class")));
}
