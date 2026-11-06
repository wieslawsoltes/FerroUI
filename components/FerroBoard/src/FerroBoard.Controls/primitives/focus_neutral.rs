//! Source FocusNeutral tree walk with native inherited interaction-focus suppression.
//! Known Uno FrameworkElement-only families keep their independent tab-stop values.
//! Panel enumeration matches Uno's materialized snapshot, including callbacks that mutate children.
//! Complete type-hierarchy, owner mutation and property-store parity remain separately audited.
use ferroui_controls::{
    Border, ContentControl, Control, Image, Panel, TextBlock, Viewbox,
    presenters::{ContentPresenter, ItemsPresenter},
    primitives::Popup,
    shapes::Shape,
};
pub(crate) struct FocusNeutral;
impl FocusNeutral {
    pub fn apply(element: &Control) -> &Control {
        Self::walk(element, 0);
        element
    }
    #[cfg(test)]
    fn apply_optional(element: Option<&Control>) -> Option<&Control> {
        if let Some(element) = element {
            Self::walk(element, 0);
        }
        element
    }
    fn walk(element: &Control, depth: usize) {
        if depth > 64 {
            return;
        }
        element.set_allow_focus_on_interaction(false);
        // FerroUI places these FrameworkElement counterparts under Control. The source
        // `element is Control` check excludes each family, including derived panels/shapes.
        if element.downcast_ref::<Panel>().is_none()
            && element.downcast_ref::<Border>().is_none()
            && element.downcast_ref::<Image>().is_none()
            && element.downcast_ref::<TextBlock>().is_none()
            && element.downcast_ref::<Shape>().is_none()
            && element.downcast_ref::<ContentPresenter>().is_none()
            && element.downcast_ref::<ItemsPresenter>().is_none()
            && element.downcast_ref::<Viewbox>().is_none()
            && element.downcast_ref::<Popup>().is_none()
        {
            element.set_is_tab_stop(false);
        }
        if let Some(panel) = element.downcast_ref::<Panel>() {
            // Uno UIElementCollection enumerates MaterializableList.Materialized, not the
            // live backing list. A callback can remove/replace/add children without changing
            // this invocation's remaining visits; a recursive Apply takes a fresh snapshot.
            for child in panel.children().snapshot().iter() {
                Self::walk(child, depth + 1);
            }
        } else if let Some(border) = element.downcast_ref::<Border>() {
            if let Some(child) = border.child() {
                Self::walk(&child, depth + 1);
            }
        } else if let Some(content) = element.downcast_ref::<ContentControl>() {
            // Includes native ScrollViewer/UserControl, the same source content branches.
            if let Some(child) = content.content().as_ref().and_then(Control::from_boxed) {
                Self::walk(&child, depth + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{Ref, input::InputElement};
    use ferroui_controls::{Button, Grid, ScrollViewer, UserControl};
    use serde_json::{Value, json};
    #[test]
    fn source_presenter_viewbox_and_popup_roles_preserve_tab_and_do_not_walk_content() {
        use ferroui_controls::presenters::ScrollContentPresenter;
        crate::test_support::run(|| {
            let expected: Vec<Value> =
                include_str!("../../../tests/fixtures/focus-framework-roles.jsonl")
                    .lines()
                    .map(|line| serde_json::from_str(line).unwrap())
                    .collect();
            let mut actual = Vec::new();
            for initial in [false, true] {
                let elements: Vec<(&str, Ref<Control>)> = vec![
                    ("ContentPresenter", ContentPresenter::new().upcast()),
                    (
                        "ScrollContentPresenter",
                        ScrollContentPresenter::new().upcast(),
                    ),
                    ("ItemsPresenter", ItemsPresenter::new().upcast()),
                    ("Viewbox", Viewbox::new().upcast()),
                    ("Popup", Popup::new().upcast()),
                    ("ContentControl", ContentControl::new().upcast()),
                    ("Button", Button::new().upcast()),
                    ("UserControl", UserControl::new().upcast()),
                    ("ScrollViewer", ScrollViewer::new().upcast()),
                ];
                for (name, element) in elements {
                    element.set_is_tab_stop(initial);
                    let same = std::ptr::eq(FocusNeutral::apply(&element), &*element);
                    actual.push(json!({"tag":"type","type":name,"initial":initial,"tab":element.is_tab_stop(),"local":element.get_base_value(InputElement::allow_focus_on_interaction_property()),"same":same}));
                }
            }
            for name in [
                "ContentPresenter",
                "ScrollContentPresenter",
                "Viewbox",
                "Popup",
            ] {
                let child = Control::new();
                child.set_is_tab_stop(true);
                let root: Ref<Control> = match name {
                    "ContentPresenter" => {
                        let p = ContentPresenter::new();
                        p.set_content(Some(Control::boxed(child.clone())));
                        p.upcast()
                    }
                    "ScrollContentPresenter" => {
                        let p = ScrollContentPresenter::new();
                        p.set_content(Some(Control::boxed(child.clone())));
                        p.upcast()
                    }
                    "Viewbox" => {
                        let p = Viewbox::new();
                        p.set_child(child.clone());
                        p.upcast()
                    }
                    "Popup" => {
                        let p = Popup::new();
                        p.set_child(child.clone());
                        p.upcast()
                    }
                    _ => unreachable!(),
                };
                root.set_is_tab_stop(true);
                let same = std::ptr::eq(FocusNeutral::apply(&root), &*root);
                actual.push(json!({"tag":"unwalked-content","type":name,"tab":root.is_tab_stop(),"local":root.get_base_value(InputElement::allow_focus_on_interaction_property()),"childTab":child.is_tab_stop(),"childLocal":child.get_base_value(InputElement::allow_focus_on_interaction_property()),"same":same}));
            }
            assert_eq!(actual, expected);
            assert_eq!(actual.len(), 22);
        });
    }
    #[test]
    fn source_materialized_panel_walk_survives_mutation_reentrancy_and_propagates_callbacks() {
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
            sync::Arc,
        };
        crate::test_support::run(|| {
            let source: Vec<Value> = include_str!("../../../tests/fixtures/focus-traversal.jsonl")
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            assert_eq!(source.len(), 30);
            for expected in source {
                let action = expected["action"].as_str().unwrap().to_owned();
                let trigger = expected["trigger"].as_u64().unwrap() as usize;
                let panel = Grid::new();
                panel.set_is_tab_stop(true);
                let nodes = vec![Control::new(), Control::new(), Control::new()];
                for node in &nodes {
                    node.set_is_tab_stop(true);
                }
                panel.children().add(nodes[0].clone());
                panel.children().add(nodes[1].clone());
                let trace = Rc::new(RefCell::new(Vec::<&str>::new()));
                let done = Rc::new(Cell::new(false));
                let failure = Arc::new(());
                let mut subscriptions = Vec::new();
                for index in 0..nodes.len() {
                    let panel = panel.clone();
                    let all = nodes.clone();
                    let action = action.clone();
                    let trace = trace.clone();
                    let done = done.clone();
                    let failure = failure.clone();
                    subscriptions.push(nodes[index].property_changed(move |e| {
                        if e.property() != InputElement::is_tab_stop_property().as_property() {
                            return;
                        }
                        trace.borrow_mut().push(["a", "b", "c"][index]);
                        if index != trigger || done.replace(true) {
                            return;
                        }
                        match action.as_str() {
                            "add" => panel.children().add(all[2].clone()),
                            "insert" => panel.children().insert(0, all[2].clone()),
                            "remove-current" => {
                                assert!(panel.children().remove(all[index].clone()));
                            }
                            "remove-other" => {
                                assert!(panel.children().remove(all[1 - index].clone()));
                            }
                            "clear" => panel.children().clear(),
                            "replace-current" => panel.children().set(index, all[2].clone()),
                            "replace-other" => panel.children().set(1 - index, all[2].clone()),
                            "move" => panel.children().move_item(0, 1),
                            "no-op-remove" => {
                                assert!(!panel.children().remove(all[2].clone()));
                            }
                            "same-replace" => panel.children().set(index, all[index].clone()),
                            "same-move" => panel.children().move_item(index, index),
                            "reenter" => {
                                FocusNeutral::apply(&panel);
                            }
                            "add-and-reenter" => {
                                panel.children().add(all[2].clone());
                                FocusNeutral::apply(&panel);
                            }
                            "throw-before" => std::panic::resume_unwind(Box::new(failure.clone())),
                            "throw-after" => {
                                panel.children().clear();
                                std::panic::resume_unwind(Box::new(failure.clone()));
                            }
                            other => panic!("Unknown source action {other}"),
                        }
                    }));
                }
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    FocusNeutral::apply(&panel)
                }));
                let (returned, threw, same_error) = match result {
                    Ok(value) => {
                        let base: &Control = &panel;
                        (std::ptr::eq(value, base), false, false)
                    }
                    Err(value) => (
                        false,
                        true,
                        Arc::ptr_eq(
                            value
                                .downcast_ref::<Arc<()>>()
                                .expect("Unexpected callback failure"),
                            &failure,
                        ),
                    ),
                };
                for subscription in subscriptions {
                    subscription.dispose();
                }
                let names = ["a", "b", "c"];
                let children: Vec<_> = panel
                    .children()
                    .snapshot()
                    .iter()
                    .map(|child| names[nodes.iter().position(|node| node == child).unwrap()])
                    .collect();
                let mut flags = vec![
                    json!({"name":"root","local":panel.get_base_value(InputElement::allow_focus_on_interaction_property()),"tab":panel.is_tab_stop()}),
                ];
                flags.extend(nodes.iter().enumerate().map(|(i,node)|json!({"name":names[i],"local":node.get_base_value(InputElement::allow_focus_on_interaction_property()),"tab":node.is_tab_stop()})));
                let actual = json!({"action":action,"trigger":trigger,"returned":returned,"threw":threw,"sameError":same_error,"children":children,"trace":*trace.borrow(),"nodes":flags});
                assert_eq!(actual, expected);
            }
        });
    }
    #[test]
    fn source_tab_stop_type_gate_preserves_framework_element_values_and_descendants() {
        use ferroui_controls::{
            StackPanel,
            shapes::{Ellipse, Polyline, Rectangle},
        };
        crate::test_support::run(|| {
            let mut actual = Vec::new();
            for initial in [false, true] {
                let elements: Vec<(&str, Ref<Control>)> = vec![
                    ("Panel", Panel::new().upcast()),
                    ("Grid", Grid::new().upcast()),
                    ("StackPanel", StackPanel::new().upcast()),
                    ("Border", Border::new().upcast()),
                    ("Image", Image::new().upcast()),
                    ("TextBlock", TextBlock::new().upcast()),
                    ("Rectangle", Rectangle::new().upcast()),
                    ("Ellipse", Ellipse::new().upcast()),
                    ("Polyline", Polyline::new().upcast()),
                    ("Control", Control::new()),
                    ("Button", Button::new().upcast()),
                    ("ContentControl", ContentControl::new().upcast()),
                    ("ScrollViewer", ScrollViewer::new().upcast()),
                    ("UserControl", UserControl::new().upcast()),
                ];
                for (name, element) in elements {
                    element.set_is_tab_stop(initial);
                    let same = std::ptr::eq(FocusNeutral::apply(&element), &*element);
                    actual.push(json!({"tag":"type","type":name,"initial":initial,"tab":element.is_tab_stop(),"allow":element.allow_focus_on_interaction(),"same":same}));
                }
            }
            let panel = Grid::new();
            let border = Border::new();
            let content = ContentControl::new();
            let button = Button::new();
            content.set_content(Some(Control::boxed(button.clone())));
            border.set_child(content.clone());
            panel.children().add(border.clone());
            let elements: Vec<(&str, Ref<Control>)> = vec![
                ("Grid", panel.clone().upcast()),
                ("Border", border.upcast()),
                ("ContentControl", content.upcast()),
                ("Button", button.upcast()),
            ];
            for (_, element) in &elements {
                element.set_is_tab_stop(true);
            }
            FocusNeutral::apply(&panel);
            for (name, element) in elements {
                actual.push(json!({"tag":"nested","type":name,"initial":true,"tab":element.is_tab_stop(),"allow":element.allow_focus_on_interaction(),"same":true}));
            }
            let expected: Vec<Value> =
                include_str!("../../../tests/fixtures/focus-tab-policy.jsonl")
                    .lines()
                    .map(|line| serde_json::from_str(line).unwrap())
                    .collect();
            assert_eq!(actual, expected);
            assert_eq!(actual.len(), 32);
        });
    }
    fn row(name: &str, control: &Control, is_source_control: bool) -> Value {
        json!({"name":name,"local":control.get_base_value(InputElement::allow_focus_on_interaction_property()),"allow":control.allow_focus_on_interaction(),"tab":if is_source_control {Some(control.is_tab_stop())}else{None}})
    }
    #[test]
    fn source_focus_neutral_tree_flags_depth_boundary_and_original_return_identity() {
        crate::test_support::run(|| {
            let source: Vec<Value> =
                include_str!("../../../tests/fixtures/focus-interaction.jsonl")
                    .lines()
                    .map(|v| serde_json::from_str(v).unwrap())
                    .filter(|v: &Value| v["tag"] == "walk")
                    .collect();
            let root = Grid::new();
            let border = Border::new();
            let a = Button::new();
            border.set_child(a.clone());
            let scroll = ScrollViewer::new();
            let user = UserControl::new();
            let content = ContentControl::new();
            let b = Button::new();
            content.set_content(Some(Control::boxed(b.clone())));
            user.set_content(Some(Control::boxed(content.clone())));
            scroll.set_content(Some(Control::boxed(user.clone())));
            root.children().add(border.clone());
            root.children().add(scroll.clone());
            let returned = FocusNeutral::apply(&root);
            let base: &Control = &root;
            let same = std::ptr::eq(returned, base);
            let actual = json!({"tag":"walk","scenario":"branches","same":same,"nodes":[row("panel",&root,false),row("border",&border,false),row("a",&a,true),row("scroll",&scroll,true),row("user",&user,true),row("content",&content,true),row("b",&b,true)]});
            assert_eq!(
                actual,
                *source.iter().find(|v| v["scenario"] == "branches").unwrap()
            );
            let chain: Vec<Ref<Border>> = (0..68).map(|_| Border::new()).collect();
            for i in 1..chain.len() {
                chain[i - 1].set_child(chain[i].clone());
            }
            chain[65].set_allow_focus_on_interaction(true);
            FocusNeutral::apply(&chain[0]);
            let actual = json!({"tag":"walk","scenario":"depth","same":true,"nodes":chain.iter().enumerate().map(|(i,c)|row(&format!("d{i}"),c,false)).collect::<Vec<_>>()});
            assert_eq!(
                actual,
                *source.iter().find(|v| v["scenario"] == "depth").unwrap()
            );
            assert!(FocusNeutral::apply_optional(None).is_none());
            assert_eq!(
                json!({"tag":"walk","scenario":"null","same":true,"nodes":[]}),
                *source.iter().find(|v| v["scenario"] == "null").unwrap()
            );
        });
    }
}
