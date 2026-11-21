use super::*;
use ferroui_base::media::FlowDirection;

#[test]
fn runtime_trace_keeps_the_local_base_and_current_value_identity() {
    run(|| {
        let (window, button, _, mut tester) = fixture();
        tester.permissions = Permissions::default();
        let id = tester.node_id(&button);
        let property = Visual::opacity_property();
        button.set_value(property, 0.8);
        let animation = button
            .set_value_with_priority(property, 0.25, BindingPriority::Animation)
            .unwrap();
        let args = json!({"node_id": id, "property": "Opacity"});
        let trace = tester.execute("get_property_trace", &args).unwrap();
        assert_eq!(trace["effective"], "0.25");
        assert_eq!(trace["contributors"][0]["kind"], "animation");
        assert_eq!(trace["contributors"][0]["state"], "effective");
        let local = trace["contributors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["kind"] == "local")
            .unwrap();
        assert_eq!(local["state"], "base");
        assert_eq!(local["value"], "0.8");
        animation.dispose();
        button.set_current_value(property, 0.62);
        let trace = tester.execute("get_property_trace", &args).unwrap();
        assert_eq!(trace["effective"], "0.62");
        assert_eq!(trace["contributors"][0]["kind"], "currentValue");
        assert_eq!(trace["contributors"][0]["state"], "effective");
        assert_eq!(button.opacity(), 0.62);
        window.close();
    });
}

#[test]
fn inherited_trace_identifies_the_nearest_contributing_ancestor() {
    run(|| {
        let parent = StackPanel::new();
        parent.set_name(Some("ReadingOrder".into()));
        let intermediate = StackPanel::new();
        let child = Border::new();
        intermediate
            .children()
            .add(child.clone().upcast::<Control>());
        parent.children().add(intermediate.upcast::<Control>());
        parent.set_flow_direction(FlowDirection::RightToLeft);
        let property = property_name(Visual::flow_direction_property().as_property());
        let trace = Inspector::property_trace(&child, &property).unwrap();
        assert_eq!(
            trace.contributors[0].kind,
            ferroui_base::diagnostics::ValueContributorKind::Inherited
        );
        assert_eq!(
            trace.contributors[0].label,
            "Inherited from StackPanel#ReadingOrder"
        );
        assert_eq!(
            trace.contributors[0].source.as_ref(),
            Some(&parent.upcast::<FerroObject>())
        );
    });
}
