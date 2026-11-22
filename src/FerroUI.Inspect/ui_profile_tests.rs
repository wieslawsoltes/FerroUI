use super::*;
use ferroui_base::diagnostics::ui_profile::{self, Phase};

#[test]
fn ui_profile_is_window_scoped_and_excludes_nested_time() {
    run(|| {
        let (window, button, _, mut tester) = fixture();
        tester.permissions = Permissions::default();
        let other = Window::new();
        let other_button = Button::new();
        other_button.set_name(Some("OtherWindowButton".into()));
        other.set_content(Some(Control::boxed(other_button.clone())));
        other.show();
        tester.inspector.register_root(&other);
        let window_id = tester.node_id(&window);
        let node_id = tester.node_id(&button);
        let args = json!({"window_id":window_id,"duration_ms":20_000});
        let start = tester.execute("start_ui_profile", &args).unwrap();
        let handle = json!({"profile_id":start["id"]});
        assert_eq!(start["recording"], true);
        assert!(tester.execute("start_ui_profile", &args).is_err());

        let component = Rc::new(());
        let view = || (Some(button.clone().upcast::<Visual>()), None);
        let outer =
            ui_profile::component(&component, "ObservedComponent", Phase::Render, view).unwrap();
        let inner =
            ui_profile::component(&component, "ObservedComponent", Phase::Reconcile, view).unwrap();
        button.invalidate_measure();
        button.measure(ferroui_base::Size::new(225.0, 75.0));
        other_button.invalidate_measure();
        other_button.measure(ferroui_base::Size::new(225.0, 75.0));
        drop(inner);
        drop(outer);
        let complete = tester.execute("stop_ui_profile", &handle).unwrap();
        assert_eq!(complete["recording"], false);
        let rows = complete["rows"].as_array().unwrap();
        let component = rows.iter().find(|row| row["kind"] == "component").unwrap();
        assert_eq!(component["node_id"], node_id);
        assert_eq!(component["label"], "ObservedComponent");
        let render = &component["phases"][0];
        let reconcile = &component["phases"][1];
        assert_eq!(render["phase"], "render");
        assert_eq!(reconcile["phase"], "reconcile");
        let value = |stat: &Value, key: &str| stat[key].as_f64().unwrap();
        assert!(
            value(render, "total_ms") - value(render, "self_ms")
                >= value(reconcile, "total_ms") - 0.000_001
        );
        assert!(rows
            .iter()
            .any(|row| row["kind"] == "control" && row["node_id"] == node_id));
        assert!(!rows.iter().any(|row| row["name"] == "OtherWindowButton"));

        let latest = tester.execute("start_ui_profile", &args).unwrap();
        assert_ne!(latest["id"], start["id"]);
        assert!(tester.execute("stop_ui_profile", &handle).is_err());
        window.close();
        let closed = tester
            .execute("get_ui_profile", &json!({"profile_id":latest["id"]}))
            .unwrap();
        assert_eq!(closed["window_open"], false);
        assert_eq!(closed["recording"], false);
        other.close();
    });
}

#[test]
fn ui_profile_tools_bound_durations_and_require_handles_for_stop() {
    assert!(validate(
        "start_ui_profile",
        &json!({"window_id":2,"duration_ms":30_000})
    )
    .is_ok());
    assert!(validate(
        "start_ui_profile",
        &json!({"window_id":2,"duration_ms":30_001})
    )
    .is_err());
    assert!(validate("start_ui_profile", &json!({"window_id":2,"duration_ms":-1})).is_err());
    assert!(validate("stop_ui_profile", &json!({})).is_err());
}
