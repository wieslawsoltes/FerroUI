use ferroui_base::{
    Ref,
    threading::{DispatcherPriority, DispatcherTimer},
};
use ferroui_controls::{ContentControl, Window};
use ferroui_hot_reload::{ComponentReloadTarget, ReloadManifest, ReloadStatus, ReloadTarget};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

const VIEW: &str = "ferres://Installed.Runtime/Status.xaml";
const SOURCE: &str = include_str!("Status.xaml");

pub(super) fn watch(window: &Ref<Window>, host: Ref<ContentControl>, receipts: &Path) {
    ferroui_controls::register_types();
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Status.xaml");
    let manifest = ReloadManifest::parse(
        &serde_json::json!({
            "version": 1,
            "documents": [{ "uri": VIEW, "source_path": source_path, "source": SOURCE }],
        })
        .to_string(),
    )
    .expect("read the installed test markup manifest");
    let target: Rc<dyn ReloadTarget> = Rc::new(ComponentReloadTarget::content_control(
        &host,
        VIEW.into(),
        SOURCE.into(),
        manifest.configuration(None),
    ));
    let watcher = manifest.watcher(Duration::from_millis(40));
    let mut session = manifest.session().expect("register the XAML baseline");
    session.register_target(&target);
    session.submit(VIEW.into(), SOURCE.into());
    assert_eq!(
        session.apply_pending().unwrap().status,
        ReloadStatus::Applied
    );
    ferroui_hot_reload::notify_editor_on_open(window);

    let receipt = receipts.join(format!("{}-xaml.txt", std::process::id()));
    let session = std::cell::RefCell::new(session);
    let weak_window = window.downgrade();
    let timer = DispatcherTimer::with_callback(
        Duration::from_millis(40),
        DispatcherPriority::BACKGROUND,
        move |timer| {
            let Some(window) = weak_window.upgrade() else {
                timer.stop();
                return;
            };
            let _target = &target;
            let mut session = session.borrow_mut();
            if let Some(batch) = watcher.changes.try_iter().last() {
                session.submit_files(batch);
            }
            if let Some(outcome) = session.apply_pending() {
                ferroui_hot_reload::record_frame_timing(&window, &outcome);
                std::fs::write(&receipt, format!("{:?}", outcome.status)).unwrap();
            }
        },
    );
    window.closed(move || timer.stop()).detach();
}
