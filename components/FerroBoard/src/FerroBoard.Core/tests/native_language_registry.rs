use ferroboard_core::{engine::*, text::*};
use std::{
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::Duration,
};

#[test]
fn registry_growth_and_replacement_preserve_language_order_without_loading_factories() {
    let models = LanguageModels::new();
    let mut expected = vec![Utf16Text::from("en")];
    for index in 0..1024 {
        let tag = format!("x{index:04}");
        models.register_dictionary(&Utf16Text::from(format!("{tag}-initial").as_str()), None);
        expected.push(Utf16Text::from(tag.as_str()));
    }
    let snapshot = models.languages();
    let replacement = Arc::new(WordDictionary::new("replacement".into()));
    for index in (0..1024).step_by(7) {
        let tag = Utf16Text::from(format!("X{index:04}_replacement").as_str());
        models.register_dictionary(&tag, Some(replacement.clone()));
        assert!(!models.is_loaded(&tag));
        assert!(Arc::ptr_eq(
            &models.get_dictionary(&tag).unwrap().unwrap(),
            &replacement
        ));
    }
    assert_eq!(models.languages(), expected);
    assert_eq!(snapshot, expected);
    assert!(!models.is_loaded(&"en".into()));
    models.register_dictionary(&"last".into(), None);
    expected.push("last".into());
    assert_eq!(models.languages(), expected);
    assert_eq!(snapshot.len(), 1025);
}

#[test]
fn native_user_snapshots_keep_first_spelling_order_and_original_handles_after_growth() {
    let models = LanguageModels::new();
    let first = models.get_user_dictionary(&"Σ-first".into());
    assert!(first.ptr_eq(&models.get_user_dictionary(&"ς-alias".into())));
    let early = models.user_dictionaries();
    let mut expected = vec![Utf16Text::from("σ")];
    for index in 0..256 {
        let tag = format!("u{index:04}");
        models.get_user_dictionary(&Utf16Text::from(format!("{tag}-region").as_str()));
        expected.push(Utf16Text::from(tag.as_str()));
    }
    let snapshot = models.user_dictionaries();
    assert_eq!(snapshot.len(), 257);
    assert_eq!(
        snapshot
            .iter()
            .map(|(tag, _)| tag.clone())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        early.iter().map(|(tag, _)| tag.clone()).collect::<Vec<_>>(),
        vec![Utf16Text::from("σ")]
    );
    assert!(snapshot.get(&"Σ".into()).unwrap().ptr_eq(&first));
    assert!(snapshot.get(&"σ-region".into()).is_none());
    first.learn(&"shared".into(), None);
    assert_eq!(
        early.get(&"ς".into()).unwrap().get_count(&"shared".into()),
        1
    );
    let owner = Arc::downgrade(&models);
    drop(first);
    drop(models);
    assert!(owner.upgrade().is_some());
    drop(snapshot);
    assert!(owner.upgrade().is_some());
    drop(early);
    assert!(owner.upgrade().is_none());
}

#[test]
fn replacing_an_inflight_factory_keeps_the_old_result_and_new_registration_independent() {
    let models = LanguageModels::new();
    let old = Arc::new(WordDictionary::new("old".into()));
    let new = Arc::new(WordDictionary::new("new".into()));
    let (entered_tx, entered) = mpsc::channel();
    let (release_tx, release) = mpsc::channel();
    let release = Mutex::new(release);
    let value = old.clone();
    models
        .register_factory(
            &"xx-first".into(),
            Some(Arc::new(move || {
                entered_tx.send(()).unwrap();
                release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
                Ok(Some(value.clone()))
            })),
        )
        .unwrap();
    let worker = {
        let models = models.clone();
        std::thread::spawn(move || models.get_dictionary(&"XX-worker".into()).unwrap().unwrap())
    };
    entered.recv_timeout(Duration::from_secs(10)).unwrap();
    models.register_dictionary(&"XX-replaced".into(), Some(new.clone()));
    assert_eq!(
        models.languages(),
        vec![Utf16Text::from("en"), Utf16Text::from("xx")]
    );
    assert!(!models.is_loaded(&"xx".into()));
    assert!(Arc::ptr_eq(
        &models.get_dictionary(&"xx".into()).unwrap().unwrap(),
        &new
    ));
    release_tx.send(()).unwrap();
    assert!(Arc::ptr_eq(&worker.join().unwrap(), &old));
    assert!(Arc::ptr_eq(
        &models.get_dictionary(&"xx".into()).unwrap().unwrap(),
        &new
    ));
}

struct DropProbe {
    owner: Weak<LanguageModels>,
    completed_outside_lock: Arc<AtomicBool>,
    worker: Arc<Mutex<Option<JoinHandle<()>>>>,
}
impl Drop for DropProbe {
    fn drop(&mut self) {
        let owner = self.owner.upgrade().unwrap();
        let (done_tx, done) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            owner.register_dictionary(&"from-drop".into(), None);
            done_tx.send(()).unwrap();
        });
        *self.worker.lock().unwrap() = Some(worker);
        self.completed_outside_lock.store(
            done.recv_timeout(Duration::from_secs(5)).is_ok(),
            Ordering::SeqCst,
        );
    }
}
#[test]
fn replaced_factory_destructors_run_after_registry_borrows_and_lock_are_released() {
    let models = LanguageModels::new();
    let completed = Arc::new(AtomicBool::new(false));
    let worker = Arc::new(Mutex::new(None));
    let probe = DropProbe {
        owner: Arc::downgrade(&models),
        completed_outside_lock: completed.clone(),
        worker: worker.clone(),
    };
    models
        .register_factory(
            &"xx".into(),
            Some(Arc::new(move || {
                let _keep_until_factory_drop = &probe;
                Ok(None)
            })),
        )
        .unwrap();
    models.register_dictionary(&"XX-replacement".into(), None);
    worker.lock().unwrap().take().unwrap().join().unwrap();
    assert!(
        completed.load(Ordering::SeqCst),
        "user destructor must not run under the registry lock"
    );
    assert_eq!(
        models.languages(),
        vec![
            Utf16Text::from("en"),
            Utf16Text::from("xx"),
            Utf16Text::from("from")
        ]
    );
}
