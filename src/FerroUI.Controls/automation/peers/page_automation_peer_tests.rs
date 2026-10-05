use super::{
    AutomationControlType, AutomationPeer, CarouselPageAutomationPeer, ContentPageAutomationPeer,
    ControlAutomationPeer, NavigationPageAutomationPeer, TabbedPageAutomationPeer,
};
use crate::test_support::{boxed_str, test_scope};
use crate::{CarouselPage, ContentPage, NavigationPage, Page, TabbedPage};
use ferroui_base::{BoxedValue, Ref};
use std::rc::Rc;

mod content_page_peer {
    use super::*;

    fn create_peer(page: &ContentPage) -> Ref<ContentPageAutomationPeer> {
        ControlAutomationPeer::create_peer_for_element(page).cast().expect("a content page automation peer")
    }

    #[test]
    fn creates_content_page_automation_peer() {
        let _scope = test_scope();
        let page = ContentPage::new();
        let peer: Ref<AutomationPeer> = ControlAutomationPeer::create_peer_for_element(&page);

        assert!(std::ptr::eq(peer.get_type(), ContentPageAutomationPeer::TYPE));
    }

    #[test]
    fn control_type_is_pane() {
        let _scope = test_scope();
        let page = ContentPage::new();
        let peer = create_peer(&page);

        assert_eq!(AutomationControlType::Pane, peer.get_automation_control_type());
    }

    #[test]
    fn name_returns_string_header() {
        let _scope = test_scope();
        let page = ContentPage::new();
        page.set_header(boxed_str("Settings"));
        let peer = create_peer(&page);

        assert_eq!("Settings", peer.get_name());
    }

    #[test]
    fn name_returns_to_string_for_non_string_header() {
        let _scope = test_scope();
        let page = ContentPage::new();
        page.set_header(Some(Rc::new(42_i32) as BoxedValue));
        let peer = create_peer(&page);

        assert_eq!("42", peer.get_name());
    }

    #[test]
    fn name_is_empty_when_no_header() {
        let _scope = test_scope();
        let page = ContentPage::new();
        let peer = create_peer(&page);

        assert!(peer.get_name().is_empty());
    }
}

mod tabbed_page_peer {
    use super::*;

    fn create_peer(page: &TabbedPage) -> Ref<TabbedPageAutomationPeer> {
        ControlAutomationPeer::create_peer_for_element(page).cast().expect("a tabbed page automation peer")
    }

    #[test]
    fn creates_tabbed_page_automation_peer() {
        let _scope = test_scope();
        let page = TabbedPage::new();
        let peer: Ref<AutomationPeer> = ControlAutomationPeer::create_peer_for_element(&page);

        assert!(std::ptr::eq(peer.get_type(), TabbedPageAutomationPeer::TYPE));
    }

    #[test]
    fn control_type_is_pane() {
        let _scope = test_scope();
        let page = TabbedPage::new();
        let peer = create_peer(&page);

        assert_eq!(AutomationControlType::Pane, peer.get_automation_control_type());
    }

    #[test]
    fn name_returns_string_header() {
        let _scope = test_scope();
        let page = TabbedPage::new();
        page.set_header(boxed_str("Main"));
        let peer = create_peer(&page);

        assert_eq!("Main", peer.get_name());
    }

    #[test]
    fn name_returns_to_string_for_non_string_header() {
        let _scope = test_scope();
        let page = TabbedPage::new();
        page.set_header(Some(Rc::new(42_i32) as BoxedValue));
        let peer = create_peer(&page);

        assert_eq!("42", peer.get_name());
    }

    #[test]
    fn name_is_empty_when_no_header() {
        let _scope = test_scope();
        let page = TabbedPage::new();
        let peer = create_peer(&page);

        assert!(peer.get_name().is_empty());
    }
}

mod navigation_page_peer {
    use super::*;

    fn create_peer(page: &NavigationPage) -> Ref<NavigationPageAutomationPeer> {
        ControlAutomationPeer::create_peer_for_element(page).cast().expect("a navigation page automation peer")
    }

    fn content_page(header: &str) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_header(boxed_str(header));
        page
    }

    #[test]
    fn creates_navigation_page_automation_peer() {
        let _scope = test_scope();
        let page = NavigationPage::new();
        let peer: Ref<AutomationPeer> = ControlAutomationPeer::create_peer_for_element(&page);

        assert!(std::ptr::eq(peer.get_type(), NavigationPageAutomationPeer::TYPE));
    }

    #[test]
    fn control_type_is_pane() {
        let _scope = test_scope();
        let page = NavigationPage::new();
        let peer = create_peer(&page);

        assert_eq!(AutomationControlType::Pane, peer.get_automation_control_type());
    }

    #[test]
    fn name_returns_own_header_when_set() {
        let _scope = test_scope();
        let page = NavigationPage::new();
        page.set_header(boxed_str("Navigation"));
        let peer = create_peer(&page);

        assert_eq!("Navigation", peer.get_name());
    }

    #[test]
    fn name_returns_to_string_for_non_string_header() {
        let _scope = test_scope();
        let page = NavigationPage::new();
        page.set_header(Some(Rc::new(42_i32) as BoxedValue));
        let peer = create_peer(&page);

        assert_eq!("42", peer.get_name());
    }

    #[test]
    fn name_prioritizes_own_header_over_current_page_header() {
        let _scope = test_scope();
        let inner = content_page("Details");
        let page = NavigationPage::new();
        page.set_header(boxed_str("Navigation"));
        page.set_current_value(Page::current_page_property(), Some(inner.upcast::<Page>()));

        let peer = create_peer(&page);

        assert_eq!("Navigation", peer.get_name());
    }

    #[test]
    fn name_falls_back_to_current_page_header() {
        let _scope = test_scope();
        let inner = content_page("Details");
        let page = NavigationPage::new();
        page.set_current_value(Page::current_page_property(), Some(inner.upcast::<Page>()));

        let peer = create_peer(&page);

        assert_eq!("Details", peer.get_name());
    }

    #[test]
    fn name_is_empty_when_no_header_and_no_current_page() {
        let _scope = test_scope();
        let page = NavigationPage::new();
        let peer = create_peer(&page);

        assert!(peer.get_name().is_empty());
    }
}

mod carousel_page_peer {
    use super::*;

    fn create_peer(page: &CarouselPage) -> Ref<CarouselPageAutomationPeer> {
        ControlAutomationPeer::create_peer_for_element(page).cast().expect("a carousel page automation peer")
    }

    #[test]
    fn creates_carousel_page_automation_peer() {
        let _scope = test_scope();
        let page = CarouselPage::new();
        let peer: Ref<AutomationPeer> = ControlAutomationPeer::create_peer_for_element(&page);

        assert!(std::ptr::eq(peer.get_type(), CarouselPageAutomationPeer::TYPE));
    }

    #[test]
    fn control_type_is_pane() {
        let _scope = test_scope();
        let page = CarouselPage::new();
        let peer = create_peer(&page);

        assert_eq!(AutomationControlType::Pane, peer.get_automation_control_type());
    }

    #[test]
    fn name_returns_string_header() {
        let _scope = test_scope();
        let page = CarouselPage::new();
        page.set_header(boxed_str("Photos"));
        let peer = create_peer(&page);

        assert_eq!("Photos", peer.get_name());
    }

    #[test]
    fn name_returns_to_string_for_non_string_header() {
        let _scope = test_scope();
        let page = CarouselPage::new();
        page.set_header(Some(Rc::new(7_i32) as BoxedValue));
        let peer = create_peer(&page);

        assert_eq!("7", peer.get_name());
    }

    #[test]
    fn name_is_empty_when_no_header() {
        let _scope = test_scope();
        let page = CarouselPage::new();
        let peer = create_peer(&page);

        assert!(peer.get_name().is_empty());
    }
}
