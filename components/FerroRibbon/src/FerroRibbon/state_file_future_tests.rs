//! The actual ribbon's persistence path resumes through the native dispatcher.
use crate::runtime_sample::Mounted;
use ferroribbon_core::{RibbonDensity, RibbonVisibilityMode, state::RibbonStateSerializer};
use ferroui_base::threading::Dispatcher;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[test]
fn file_futures_restore_real_ribbon_state_on_the_native_dispatcher() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.set_selected_tab_id(Some("insert".into()));
        m.ribbon.set_density(RibbonDensity::Touch);
        m.ribbon
            .set_visibility_mode(RibbonVisibilityMode::PanelTitles);
        m.settle();
        let path = std::env::temp_dir().join(format!(
            "native-ribbon-state-future-{}.json",
            std::process::id()
        ));
        let snapshot = m.ribbon.get_state().unwrap();
        let save = RibbonStateSerializer::save_async_default(Some(&snapshot), Some(&path));
        drop(snapshot);
        m.ribbon.set_selected_tab_id(Some("home".into()));
        m.ribbon.set_density(RibbonDensity::Compact);
        m.ribbon
            .set_visibility_mode(RibbonVisibilityMode::AlwaysShow);
        let dispatcher = Dispatcher::ui_thread();
        let owner_thread = std::thread::current().id();
        let ribbon = m.ribbon.clone();
        let completed = Rc::new(RefCell::new(None));
        let output = completed.clone();
        let input_path = path.clone();
        drop(dispatcher.start_local(async move {
            let outcome = async move {
                save.await.map_err(|e| e.to_string())?;
                let state = RibbonStateSerializer::load_async_default(Some(&input_path))
                    .await
                    .map_err(|e| e.to_string())?;
                assert_eq!(std::thread::current().id(), owner_thread);
                ribbon.apply_saved_state(state)?;
                Ok::<(), String>(())
            }
            .await;
            output.replace(Some(outcome));
        }));
        let deadline = Instant::now() + Duration::from_secs(5);
        while completed.borrow().is_none() && Instant::now() < deadline {
            dispatcher.run_jobs(None);
            std::thread::sleep(Duration::from_millis(1));
        }
        completed
            .borrow_mut()
            .take()
            .expect("persistence future did not complete")
            .unwrap();
        m.settle();
        assert_eq!(
            m.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("insert")
        );
        assert_eq!(m.ribbon.density(), RibbonDensity::Touch);
        assert_eq!(
            m.ribbon.visibility_mode(),
            RibbonVisibilityMode::PanelTitles
        );
        std::fs::remove_file(path).unwrap();
    });
}
