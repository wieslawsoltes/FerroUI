//! Actual native automation provider/inheritance/selection identity contracts.
use crate::controls::*;
use ferroui_controls::{
    Control,
    automation::{
        AutomationProperties,
        peers::{AutomationControlType, ControlAutomationPeer},
        provider::{IInvokeProvider, ISelectionItemProvider, ISelectionProvider},
    },
};
#[test]
fn standalone_header_retains_its_public_tab_and_releases_without_a_cycle() {
    crate::group_tests::run(|| {
        let tab = RibbonTab::new();
        tab.set_header(Some("Detached".into()));
        tab.set_id(Some("detached".into()));
        let weak_tab = tab.downgrade();
        let header = RibbonTabHeader::new(tab);
        let weak_header = header.downgrade();
        assert!(weak_tab.upgrade().is_some(), "A public header owns its Tab");
        assert_eq!(header.tab().header().as_deref(), Some("Detached"));
        let peer = ControlAutomationPeer::create_peer_for_element(&header);
        assert_eq!(peer.get_name(), "Detached");
        assert_eq!(
            peer.get_automation_id().as_deref(),
            Some("RibbonTab_detached")
        );
        header.select();
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/tab-header.json")).unwrap();
        let source = reference["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "standalone")
            .unwrap();
        assert_eq!(
            serde_json::json!({
                "name":"standalone","tabAlive":weak_tab.upgrade().is_some(),
                "same":header.tab()==weak_tab.upgrade().unwrap(),
                "label":header.tab().header(),
                "content":header.content().unwrap().downcast_ref::<String>().unwrap(),
                "automationName":peer.get_name(),"automationId":peer.get_automation_id(),
                "headerCacheNull":header.tab().header_element().is_none()
            }),
            *source
        );
        drop(peer);
        drop(header);
        assert!(weak_header.upgrade().is_none());
        assert!(weak_tab.upgrade().is_none());
    });
}
#[test]
fn header_constructor_null_empty_and_unicode_labels_match_unchanged_source() {
    crate::group_tests::run(|| {
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/tab-header.json")).unwrap();
        for case in reference["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "constructor")
        {
            let label = case["label"].as_str().map(str::to_owned);
            let id = case["id"].as_str().map(str::to_owned);
            let element_name = case["elementName"].as_str().map(str::to_owned);
            let tab = RibbonTab::new();
            tab.set_header(label.clone());
            tab.set_id(id.clone());
            tab.set_name(element_name.clone());
            let header = RibbonTabHeader::new(tab.clone());
            let peer = ControlAutomationPeer::create_peer_for_element(&header);
            assert_eq!(
                serde_json::json!({"name":"constructor","label":label,"id":id,"elementName":element_name,
                "content":header.content().as_ref().and_then(|v|v.downcast_ref::<String>()).cloned(),
                "automationName":peer.get_name(),"automationId":peer.get_automation_id(),"same":header.tab()==tab}),
                *case
            );
        }
    });
}
#[test]
fn generated_header_cache_keeps_live_identity_and_custom_state_without_retaining_removed_tabs() {
    crate::group_tests::run(|| {
        use std::rc::Rc;
        let root = Ribbon::new();
        let home = RibbonTab::new();
        home.set_header(Some("Home".into()));
        root.tabs().add(Some(home));
        let context = RibbonContextualTabGroup::new();
        context.set_id(Some("tools".into()));
        context.set_is_visible(true);
        root.contextual_groups().add(Some(context.clone()));
        let tab = RibbonTab::new();
        tab.set_contextual_group_id(Some("tools".into()));
        tab.set_header(Some("Tools".into()));
        root.tabs().add(Some(tab.clone()));
        let window = crate::ribbon_tests::mount(&root);
        let header = tab.header_element().unwrap();
        header.set_tag(Some(Rc::new("custom state".to_string())));
        let weak_header = header.downgrade();
        drop(header);
        // No caller retains a header across these rebuilds: the ribbon owns
        // generated presentations even while a contextual tab is hidden.
        tab.set_header(Some("Renamed".into()));
        assert!(
            tab.header_element()
                .unwrap()
                .ptr_eq(&weak_header.upgrade().unwrap())
        );
        context.set_is_visible(false);
        assert!(!root.tab_headers().iter().any(|h| h.tab() == tab));
        assert!(weak_header.upgrade().is_some());
        context.set_is_visible(true);
        let header = tab.header_element().unwrap();
        assert!(header.ptr_eq(&weak_header.upgrade().unwrap()));
        assert_eq!(
            header.tag().unwrap().downcast_ref::<String>().unwrap(),
            "custom state"
        );
        assert_eq!(header.tab(), tab);
        let weak_tab = tab.downgrade();
        root.tabs().remove(&Some(tab.clone()));
        drop(tab);
        assert!(
            weak_tab.upgrade().is_some(),
            "An external header owns the removed Tab"
        );
        assert_eq!(header.tab().header().as_deref(), Some("Renamed"));
        drop(header);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(
            weak_header.upgrade().is_none(),
            "The ribbon cache must prune removed entries"
        );
        assert!(weak_tab.upgrade().is_none());
        window.close();
        window.set_content(None);
    });
}
#[test]
fn generated_header_and_tab_release_when_the_ribbon_owner_is_dropped() {
    crate::group_tests::run(|| {
        let (weak_root, weak_tab, weak_header) = {
            let root = Ribbon::new();
            let tab = RibbonTab::new();
            tab.set_header(Some("Home".into()));
            root.tabs().add(Some(tab.clone()));
            let window = crate::ribbon_tests::mount(&root);
            let header = tab.header_element().unwrap();
            let refs = (root.downgrade(), tab.downgrade(), header.downgrade());
            window.close();
            window.set_content(None);
            refs
        };
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(weak_root.upgrade().is_none());
        assert!(weak_header.upgrade().is_none());
        assert!(weak_tab.upgrade().is_none());
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/tab-header.json")).unwrap();
        let source = reference["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "unrootedPair")
            .unwrap();
        assert_eq!(
            serde_json::json!({"name":"unrootedPair","tabReleased":weak_tab.upgrade().is_none(),"headerReleased":weak_header.upgrade().is_none()}),
            *source
        );
    });
}
#[test]
fn removed_header_cleanup_can_reenter_tab_collection_after_cache_is_published() {
    crate::group_tests::run(|| {
        use std::{cell::Cell, rc::Rc};
        struct Cleanup(Box<dyn Fn()>);
        impl PartialEq for Cleanup {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::eq(self, other)
            }
        }
        impl Drop for Cleanup {
            fn drop(&mut self) {
                (self.0)();
            }
        }
        let root = Ribbon::new();
        // Leave keyboard focus on an unrelated live tab. A focused element can
        // be retained by the native focus service after it leaves the strip.
        let home = RibbonTab::new();
        home.set_header(Some("Home".into()));
        root.tabs().add(Some(home));
        let tab = RibbonTab::new();
        tab.set_header(Some("Retiring".into()));
        root.tabs().add(Some(tab.clone()));
        let window = crate::ribbon_tests::mount(&root);
        let header = tab.header_element().unwrap();
        let weak_header = header.downgrade();
        let weak_root = root.downgrade();
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        header.set_tag(Some(Rc::new(Cleanup(Box::new(move || {
            observed.set(observed.get() + 1);
            let root = weak_root.upgrade().unwrap();
            let replacement = RibbonTab::new();
            replacement.set_header(Some("Replacement".into()));
            root.tabs().add(Some(replacement));
        })))));
        drop(header);
        root.tabs().remove(&Some(tab.clone()));
        // Native template detach work can retain a presentation until its
        // dispatcher cleanup has run; exercise application reentry from there.
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(weak_header.upgrade().is_none());
        assert_eq!(calls.get(), 1);
        let replacement = root.tabs().get(1).unwrap();
        assert_eq!(replacement.header().as_deref(), Some("Replacement"));
        assert_eq!(replacement.header_element().unwrap().tab(), replacement);
        window.close();
        window.set_content(None);
    });
}
#[test]
fn source_group_name_precedence_role_class_and_tab_generic_peer() {
    crate::group_tests::run(|| {
        let g = RibbonGroup::new();
        AutomationProperties::set_name(&g, Some("fallback"));
        let p = ControlAutomationPeer::create_peer_for_element(&g);
        assert!(p.is::<RibbonGroupAutomationPeer>());
        assert_eq!(
            p.get_automation_control_type(),
            AutomationControlType::Group
        );
        assert_eq!(p.get_class_name(), "RibbonGroup");
        assert_eq!(p.get_name(), "fallback");
        g.set_header(Some("".into()));
        assert_eq!(p.get_name(), "");
        g.set_header(Some("Caption".into()));
        assert_eq!(p.get_name(), "Caption");
        let t = RibbonTab::new();
        let p = ControlAutomationPeer::create_peer_for_element(&t);
        assert!(p.is::<ControlAutomationPeer>());
    });
}
#[test]
fn source_header_selection_and_inherited_invoke_use_actual_root_peer() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let first = RibbonTab::new();
        first.set_header(Some("Home".into()));
        first.set_id(Some("home".into()));
        let second = RibbonTab::new();
        second.set_header(Some("Insert".into()));
        r.tabs().add(Some(first.clone()));
        r.tabs().add(Some(second.clone()));
        let w = crate::ribbon_tests::mount(&r);
        let root = ControlAutomationPeer::create_peer_for_element(&r);
        assert!(root.is::<RibbonAutomationPeer>());
        assert_eq!(
            root.get_automation_control_type(),
            AutomationControlType::Tab
        );
        assert_eq!(root.get_class_name(), "Ribbon");
        let rp = root.get_provider::<dyn ISelectionProvider>().unwrap();
        assert!(!rp.can_select_multiple());
        assert!(rp.is_selection_required());
        let h = first.header_element().unwrap();
        let p = ControlAutomationPeer::create_peer_for_element(&h);
        let next = second.header_element().unwrap();
        let next_peer = ControlAutomationPeer::create_peer_for_element(&next);
        assert!(p.is_keyboard_focusable());
        assert!(!next_peer.is_keyboard_focusable());
        p.set_focus();
        assert!(h.is_focused());
        next_peer.set_focus();
        assert!(h.is_focused());
        assert!(!next.is_focused());
        let selection = p.get_provider::<dyn ISelectionItemProvider>().unwrap();
        assert!(p.is::<RibbonTabHeaderAutomationPeer>());
        assert_eq!(
            p.get_automation_control_type(),
            AutomationControlType::TabItem
        );
        assert_eq!(p.get_class_name(), "RibbonTabHeader");
        assert_eq!(p.get_name(), "Home");
        assert_eq!(p.get_automation_id().as_deref(), Some("RibbonTab_home"));
        assert_eq!(selection.selection_container_peer(), Some(root.clone()));
        assert_eq!(selection.selection_container().unwrap().peer(), root);
        r.select_tab(second.clone());
        assert!(!p.is_keyboard_focusable());
        assert!(next_peer.is_keyboard_focusable());
        next_peer.set_focus();
        assert!(next.is_focused());
        selection.select().unwrap();
        assert_eq!(r.selected_tab(), Some(first.clone()));
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/tab-header.json")).unwrap();
        let source = reference["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "select")
            .unwrap();
        assert_eq!(
            serde_json::json!({"name":"select","same":r.selected_tab()==Some(h.tab()),"label":h.content().unwrap().downcast_ref::<String>().unwrap()}),
            *source
        );
        assert!(selection.is_selected());
        assert!(ferroui_base::input::KeyboardNavigation::get_is_tab_stop(&h));
        assert!(!ferroui_base::input::KeyboardNavigation::get_is_tab_stop(
            &second.header_element().unwrap()
        ));
        assert_eq!(rp.get_selection(), vec![p.clone()]);
        selection.remove_from_selection().unwrap();
        assert_eq!(r.selected_tab(), Some(first.clone()));
        r.select_tab(second.clone());
        selection.add_to_selection().unwrap();
        assert_eq!(r.selected_tab(), Some(first.clone()));
        r.select_tab(second);
        p.get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        assert_eq!(r.selected_tab(), Some(first.clone()));
        first.set_header(Some("Localized".into()));
        assert_eq!(p.get_name(), "Localized");
        // Source SelectionItem methods do not synthesize an enabled check; SelectTab owns selection rules.
        h.set_is_enabled(false);
        assert!(!p.is_keyboard_focusable());
        selection.select().unwrap();
        assert_eq!(r.selected_tab(), Some(first));
        w.close();
        w.set_content(None);
    });
}
#[test]
fn source_selection_lookup_does_not_construct_missing_peers() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let t = RibbonTab::new();
        r.tabs().add(Some(t.clone()));
        let h = RibbonTabHeader::new(t.clone());
        let p = RibbonTabHeaderAutomationPeer::new(&h);
        assert!(p.selection_container().is_none());
        let root = RibbonAutomationPeer::new(&r);
        assert!(root.get_selection().is_empty());
        drop(root);
        drop(p);
        drop(h);
        // Header.Tab is weak in the native ownership model; CLR strong rooting remains an explicit gap.
        let h = RibbonTabHeader::new(t);
        let weak = h.downgrade();
        let p = ControlAutomationPeer::create_peer_for_element(&h);
        drop(p);
        drop(h);
        assert!(weak.upgrade().is_none());
        assert!(ControlAutomationPeer::from_element(&r).is_none());
        let _ = Control::boxed(r);
    });
}
