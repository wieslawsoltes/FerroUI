use super::{AutomationPeer, ControlAutomationPeer};
use crate::automation::{AutomationElementIdentifiers, AutomationProperties, AutomationPropertyChangedEventArgs};
use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Button, ContentControl, Control, Decorator, Panel};
use ferroui_base::data::TemplateBinding;
use ferroui_base::{AnyValue, Rect, Ref, Size, Visual};
use std::cell::RefCell;
use std::rc::Rc;

fn create_peer(control: &Control) -> Ref<AutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(control)
}

fn owner_of(peer: &Ref<AutomationPeer>) -> Ref<Visual> {
    peer.cast::<ControlAutomationPeer>().expect("a control automation peer").owner().upcast()
}

fn panel_with_two_borders() -> Ref<Panel> {
    let panel = Panel::new();
    panel.children().add(Border::new());
    panel.children().add(Border::new());
    panel
}

fn record(peer: &Ref<AutomationPeer>) -> Rc<RefCell<Vec<AutomationPropertyChangedEventArgs>>> {
    let raised = Rc::new(RefCell::new(Vec::new()));
    let sink = raised.clone();
    peer.property_changed(move |e| sink.borrow_mut().push(e.clone()));
    raised
}

fn string_value(e: &AutomationPropertyChangedEventArgs) -> Option<String> {
    let value: &dyn AnyValue = &**e.new_value()?;
    value.downcast_ref::<String>().cloned()
}

mod children {
    use super::*;

    #[test]
    fn creates_children_for_controls_in_visual_tree() {
        let _scope = test_scope();
        let panel = panel_with_two_borders();

        let target = create_peer(&panel);

        assert_eq!(
            panel.visual_children().to_vec(),
            target.get_children().iter().map(owner_of).collect::<Vec<_>>()
        );
    }

    #[test]
    fn creates_children_when_controls_attached_to_visual_tree() {
        let _scope = test_scope();
        let content_control = ContentControl::new();
        content_control.set_template(Some(FuncControlTemplate::for_type::<ContentControl>(|_, scope| {
            let presenter = ContentPresenter::new();
            presenter.set_name(Some("PART_ContentPresenter".to_string()));
            let property = ContentControl::content_property().as_property();
            presenter.bind_binding(property, &TemplateBinding::new(property));
            presenter.register_in_name_scope(&**scope).upcast()
        })));
        content_control.set_content(Some(Control::boxed(Border::new())));

        let target = create_peer(&content_control);

        assert!(target.get_children().is_empty());

        content_control.measure(Size::INFINITY);

        assert_eq!(1, target.get_children().len());
    }

    #[test]
    fn updates_children_when_visual_children_added() {
        let _scope = test_scope();
        let panel = panel_with_two_borders();

        let target = create_peer(&panel);
        let children = target.get_children();

        assert_eq!(2, children.len());

        panel.children().add(Decorator::new());

        let children = target.get_children();
        assert_eq!(3, children.len());
    }

    #[test]
    fn updates_children_when_visual_children_removed() {
        let _scope = test_scope();
        let panel = panel_with_two_borders();

        let target = create_peer(&panel);
        let children = target.get_children();

        assert_eq!(2, children.len());

        panel.children().remove_at(1);

        let children = target.get_children();
        assert_eq!(1, children.len());
    }

    #[test]
    fn updates_children_when_visibility_changes_from_visible_to_invisible() {
        let _scope = test_scope();
        let panel = panel_with_two_borders();

        let target = create_peer(&panel);
        let children = target.get_children();

        assert_eq!(2, children.len());

        panel.children().get(1).set_is_visible(false);
        let children = target.get_children();
        assert_eq!(1, children.len());

        panel.children().get(1).set_is_visible(true);
        let children = target.get_children();
        assert_eq!(2, children.len());
    }

    #[test]
    fn updates_children_when_visibility_changes_from_invisible_to_visible() {
        let _scope = test_scope();
        let panel = Panel::new();
        panel.children().add(Border::new());
        let hidden = Border::new();
        hidden.set_is_visible(false);
        panel.children().add(hidden);

        let target = create_peer(&panel);
        let children = target.get_children();
        assert_eq!(1, children.len());

        panel.children().get(1).set_is_visible(true);
        let children = target.get_children();
        assert_eq!(2, children.len());
    }
}

mod parent {
    use super::*;

    #[test]
    fn connects_peer_to_tree_when_get_parent_called() {
        let _scope = test_scope();
        let border = Border::new();
        let inner = Decorator::new();
        inner.set_child(border.clone());
        let tree = Decorator::new();
        tree.set_child(inner);

        // We're accessing Border directly without going via its ancestors. Because the tree
        // is built lazily, ensure that calling GetParent causes the ancestor tree to be built.
        let target = create_peer(&border);

        let parent_peer = target.get_parent().expect("a parent peer");
        assert!(parent_peer.is::<ControlAutomationPeer>());
        assert_eq!(border.get_visual_parent().unwrap(), owner_of(&parent_peer));
        drop(tree);
    }

    #[test]
    fn parent_updated_when_moved_to_separate_visual_tree() {
        let _scope = test_scope();
        let border = Border::new();
        let root1 = Decorator::new();
        root1.set_child(border.clone());
        let root2 = Decorator::new();
        let target = create_peer(&border);

        let parent_peer = target.get_parent().expect("a parent peer");
        assert!(parent_peer.is::<ControlAutomationPeer>());
        assert_eq!(root1.clone().upcast::<Visual>(), owner_of(&parent_peer));

        root1.set_child(None);

        assert!(target.get_parent().is_none());

        root2.set_child(border.clone());

        let parent_peer = target.get_parent().expect("a parent peer");
        assert!(parent_peer.is::<ControlAutomationPeer>());
        assert_eq!(root2.clone().upcast::<Visual>(), owner_of(&parent_peer));
    }
}

mod automation_id_notifications {
    use super::*;

    #[test]
    fn raises_property_changed_when_automation_id_set() {
        let _scope = test_scope();
        let button = Button::new();
        let peer = create_peer(&button);
        let raised = record(&peer);

        AutomationProperties::set_automation_id(&button, Some("btn1"));

        let raised = raised.borrow();
        assert_eq!(1, raised.len());
        assert!(std::ptr::eq(AutomationElementIdentifiers::automation_id_property(), raised[0].property()));
        assert_eq!(Some("btn1".to_string()), string_value(&raised[0]));
    }

    #[test]
    fn automation_id_overrides_name_fallback_in_notification() {
        let _scope = test_scope();
        // Name="fallback" is set before styles apply (the only valid window),
        // then AutomationId is set at runtime and the notification should reflect
        // the explicit AutomationId, not the Name fallback.
        let button = Button::new();
        button.set_name(Some("fallback".to_string()));
        let peer = create_peer(&button);
        let raised = record(&peer);

        AutomationProperties::set_automation_id(&button, Some("explicit"));

        let raised = raised.borrow();
        assert_eq!(1, raised.len());
        assert!(std::ptr::eq(AutomationElementIdentifiers::automation_id_property(), raised[0].property()));
        assert_eq!(Some("explicit".to_string()), string_value(&raised[0]));
    }
}

mod to_screen {
    use super::*;

    #[test]
    fn converts_rect_when_attached_to_rooted_tree() {
        let _scope = test_scope();
        let border = Border::new();
        let _root = TestRoot::with_child(border.clone());
        let peer = create_peer(&border);

        assert_eq!(Some(Rect::new(10.0, 20.0, 30.0, 40.0)), peer.to_screen(Rect::new(10.0, 20.0, 30.0, 40.0)));
    }

    #[test]
    fn returns_null_when_detached() {
        let _scope = test_scope();
        let border = Border::new();
        let root = TestRoot::with_child(border.clone());
        let peer = create_peer(&border);

        root.set_child(None);

        assert_eq!(None, peer.to_screen(Rect::new(10.0, 20.0, 30.0, 40.0)));
    }
}
