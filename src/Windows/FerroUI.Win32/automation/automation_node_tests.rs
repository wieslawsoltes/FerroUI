//! The port of `AutomationNodeTests` of the reference's integration tests
//! of Windows (the first two tests: they test a type that is internal to
//! the backend, so they are tests of the crate), and tests of the port of
//! the nodes over a window of controls.
//!
//! The nodes are called through their COM interfaces, as a client calls
//! them. The tests run on every host: the windowing platform is the one of
//! the unit tests of the controls, the automation core is a recorder, and
//! the values cross through the automation library of the system on
//! Windows and through its stand-in elsewhere.

use super::automation_node::{AutomationNode, NodeKind, SendProvider, ELEMENT_NOT_AVAILABLE, ELEMENT_NOT_ENABLED, NOT_SUPPORTED};
use super::interop::{
    IExpandCollapseProvider, IInvokeProvider, IRangeValueProvider, IRawElementProviderFragment,
    IRawElementProviderFragmentRoot, IRawElementProviderSimple, IRawElementProviderSimple2, IScrollItemProvider,
    ISelectionItemProvider, IToggleProvider, IUiaCoreProviderApi, IValueProvider, NavigateDirection, ProviderOptions,
    StructureChangeType, UiaControlTypeId, UiaCoreProviderApi, UiaEventId, UiaPatternId, UiaPropertyId,
};
use super::marshalling::{free_bstr, ptr_to_string_bstr, SafeArrayMarshaller, VariantArray, VariantValue};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::provider::ToggleState;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, CheckBox, Control, MenuItem, MenuItemToggleType, Slider, StackPanel, TextBox, Window};
use ferroui_microcom::{ComObject, ComPtr, HResult, IUnknown, E_NOINTERFACE};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Runs `call` on another thread, as the automation core calls a
/// provider, while this thread runs the jobs of its dispatcher.
fn on_another_thread<R: Send + 'static>(call: impl FnOnce() -> R + Send + 'static) -> R {
    let thread = std::thread::spawn(call);
    let deadline = Instant::now() + Duration::from_secs(20);
    while !thread.is_finished() {
        Dispatcher::ui_thread().run_jobs(None);
        assert!(Instant::now() < deadline, "the call did not return within twenty seconds");
        std::thread::sleep(Duration::from_millis(1));
    }
    thread.join().expect("the call on the other thread")
}

fn create_menu_item_node(is_checked: bool) -> (Ref<MenuItem>, ComPtr<IRawElementProviderSimple>) {
    let menu_item = MenuItem::new();
    menu_item.set_toggle_type(MenuItemToggleType::CheckBox);
    menu_item.set_is_checked(is_checked);
    let peer = ControlAutomationPeer::create_peer_for_element(&menu_item);
    let node = AutomationNode::get_or_create(Some(&peer)).expect("a node");
    (menu_item, node)
}

#[test]
fn toggle_pattern_is_available_for_checkable_menu_item_from_another_thread() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (_menu_item, node) = create_menu_item_node(false);

    let identity = node.cast::<IUnknown>().unwrap().as_ptr() as usize;
    let node = SendProvider(node);
    let provider = on_another_thread(move || {
        let provider = node.get().get_pattern_provider(UiaPatternId::Toggle.0).unwrap();
        provider.map(|provider| provider.as_ptr() as usize)
    });

    assert_eq!(Some(identity), provider);
}

#[test]
fn toggle_state_of_checkable_menu_item_can_be_read_from_another_thread() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (_menu_item, node) = create_menu_item_node(true);

    let node = SendProvider(node);
    let state = on_another_thread(move || node.get().cast::<IToggleProvider>().unwrap().get_toggle_state().unwrap());

    assert_eq!(ToggleState::On as i32, state);
}

// The tests below are not from the reference.

/// What stands in for the automation core: it writes down what the nodes
/// raise, with the number of the node.
#[derive(Default)]
struct Recorder {
    calls: RefCell<Vec<String>>,
}

fn number(provider: Option<&IRawElementProviderSimple>) -> String {
    match provider {
        Some(provider) => match value(provider, UiaPropertyId::RuntimeId) {
            Some(VariantValue::Array(VariantArray::I4(id))) => format!("#{}", id[1]),
            other => panic!("the runtime identifier is {other:?}"),
        },
        None => "none".to_owned(),
    }
}

impl IUiaCoreProviderApi for Recorder {
    fn uia_clients_are_listening(&self) -> bool {
        true
    }

    fn uia_return_raw_element_provider(&self, hwnd: isize, _: usize, _: isize, el: Option<&IRawElementProviderSimple>) -> isize {
        self.calls.borrow_mut().push(format!("return {hwnd} {}", number(el)));
        1
    }

    fn uia_host_provider_from_hwnd(&self, hwnd: isize) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
        self.calls.borrow_mut().push(format!("host {hwnd}"));
        Ok(None)
    }

    fn uia_raise_automation_event(&self, provider: Option<&IRawElementProviderSimple>, id: i32) -> i32 {
        self.calls.borrow_mut().push(format!("event {} {:?}", number(provider), UiaEventId(id)));
        0
    }

    fn uia_raise_automation_property_changed_event(
        &self,
        provider: Option<&IRawElementProviderSimple>,
        id: i32,
        old_value: Option<&VariantValue>,
        new_value: Option<&VariantValue>,
    ) -> i32 {
        self.calls.borrow_mut().push(format!(
            "property {} {:?} {old_value:?} -> {new_value:?}",
            number(provider),
            UiaPropertyId(id)
        ));
        0
    }

    fn uia_raise_structure_changed_event(
        &self,
        provider: Option<&IRawElementProviderSimple>,
        structure_change_type: StructureChangeType,
        runtime_id: Option<&[i32]>,
    ) -> i32 {
        self.calls.borrow_mut().push(format!("structure {} {structure_change_type:?} {runtime_id:?}", number(provider)));
        0
    }

    fn uia_disconnect_provider(&self, provider: Option<&IRawElementProviderSimple>) -> i32 {
        self.calls.borrow_mut().push(format!("disconnect {}", number(provider)));
        0
    }
}

struct Ui {
    window: Ref<Window>,
    panel: Ref<StackPanel>,
    button: Ref<Button>,
    text_box: Ref<TextBox>,
    check_box: Ref<CheckBox>,
    slider: Ref<Slider>,
    clicks: Rc<Cell<u32>>,
    recorder: Rc<Recorder>,
    root: ComPtr<IRawElementProviderSimple>,
}

impl Drop for Ui {
    fn drop(&mut self) {
        UiaCoreProviderApi::set_for_thread(None);
    }
}

fn ui() -> Ui {
    let recorder = Rc::new(Recorder::default());
    UiaCoreProviderApi::set_for_thread(Some(recorder.clone()));

    let button = Button::new();
    button.set_content(Some(Rc::new("Save".to_string())));
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    button.click(move |_, _| counter.set(counter.get() + 1));

    let text_box = TextBox::new();
    text_box.set_text(Some("Hello world"));

    let check_box = CheckBox::new();
    check_box.set_content(Some(Rc::new("Agree".to_string())));

    let slider = Slider::new();
    slider.set_minimum(0.0);
    slider.set_maximum(100.0);
    slider.set_range_value(25.0);

    let panel = StackPanel::new();
    panel.children().add(&button);
    panel.children().add(&text_box);
    panel.children().add(&check_box);
    panel.children().add(&slider);

    let window = Window::new();
    window.set_title(Some("Accessible window".to_string()));
    window.set_content(Some(Control::boxed(&panel)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();

    let peer = ControlAutomationPeer::create_peer_for_element(&window);
    let root = AutomationNode::get_or_create(Some(&peer)).expect("the node of the window");
    Ui { window, panel, button, text_box, check_box, slider, clicks, recorder, root }
}

impl Ui {
    fn node_of(&self, control: &Control) -> ComPtr<IRawElementProviderSimple> {
        // The nodes of the tree are made by walking it, as a client does.
        let _ = descendants(&self.root);
        let peer = ControlAutomationPeer::create_peer_for_element(control);
        AutomationNode::get_or_create(Some(&peer)).expect("the node of the control")
    }

    fn take_calls(&self) -> Vec<String> {
        self.recorder.calls.take()
    }
}

/// The value of a property, read through the interface and its variant.
fn value(node: &IRawElementProviderSimple, property: UiaPropertyId) -> Option<VariantValue> {
    let mut variant = node.get_property_value(property.0).unwrap();
    // SAFETY: the variant the node made, which this function owns.
    unsafe {
        let value = variant.as_object().unwrap();
        variant.dispose();
        value
    }
}

fn text(node: &IRawElementProviderSimple, property: UiaPropertyId) -> Option<String> {
    match value(node, property) {
        Some(VariantValue::Bstr(text)) => Some(text),
        None => None,
        other => panic!("{property:?} is {other:?}"),
    }
}

fn flag(node: &IRawElementProviderSimple, property: UiaPropertyId) -> bool {
    match value(node, property) {
        Some(VariantValue::Bool(flag)) => flag,
        other => panic!("{property:?} is {other:?}"),
    }
}

fn control_type(node: &IRawElementProviderSimple) -> UiaControlTypeId {
    match value(node, UiaPropertyId::ControlType) {
        Some(VariantValue::I4(control_type)) => UiaControlTypeId(control_type),
        other => panic!("the control type is {other:?}"),
    }
}

fn navigate(node: &IRawElementProviderSimple, direction: NavigateDirection) -> Option<ComPtr<IRawElementProviderSimple>> {
    let fragment = ComPtr::from_ref(node).cast::<IRawElementProviderFragment>().unwrap();
    fragment.navigate(direction).unwrap().map(|found| found.cast::<IRawElementProviderSimple>().unwrap())
}

fn children(node: &IRawElementProviderSimple) -> Vec<ComPtr<IRawElementProviderSimple>> {
    let mut result = Vec::new();
    let mut child = navigate(node, NavigateDirection::FirstChild);
    while let Some(current) = child {
        child = navigate(&current, NavigateDirection::NextSibling);
        result.push(current);
    }
    result
}

/// The nodes below `node`, depth first.
fn descendants(node: &IRawElementProviderSimple) -> Vec<ComPtr<IRawElementProviderSimple>> {
    let mut result = Vec::new();
    for child in children(node) {
        let below = descendants(&child);
        result.push(child);
        result.extend(below);
    }
    result
}

fn find(root: &IRawElementProviderSimple, wanted: UiaControlTypeId) -> ComPtr<IRawElementProviderSimple> {
    descendants(root)
        .into_iter()
        .find(|node| control_type(node) == wanted)
        .unwrap_or_else(|| panic!("the tree has no {wanted:?}"))
}

fn has_pattern(node: &IRawElementProviderSimple, pattern: UiaPatternId) -> bool {
    node.get_pattern_provider(pattern.0).unwrap().is_some()
}

#[test]
fn the_node_of_a_window_is_a_fragment_root_and_a_node_of_a_control_is_not() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let ui = ui();

    // SAFETY: the object of a node, whose value is the node.
    let node = unsafe { ComObject::<AutomationNode>::value(ui.root.as_ptr().cast()) };
    assert_eq!(NodeKind::Root, node.kind());
    assert!(node.peer().is_some());
    assert_eq!(ProviderOptions::ServerSideProvider, ui.root.get_provider_options().unwrap());

    let fragment_root = ui.root.cast::<IRawElementProviderFragmentRoot>().unwrap();
    let fragment = ui.root.cast::<IRawElementProviderFragment>().unwrap();
    // The root of a root is the root itself, and it has no parent.
    let own_root = fragment.get_fragment_root().unwrap().expect("the root of the root");
    assert_eq!(fragment_root.as_ptr(), own_root.as_ptr());
    assert!(navigate(&ui.root, NavigateDirection::Parent).is_none());
    assert_eq!(UiaControlTypeId::Window, control_type(&ui.root));
    assert_eq!(Some("Accessible window".to_owned()), text(&ui.root, UiaPropertyId::Name));

    let button = find(&ui.root, UiaControlTypeId::Button);
    // SAFETY: as above.
    assert_eq!(NodeKind::Node, unsafe { ComObject::<AutomationNode>::value(button.as_ptr().cast()) }.kind());
    assert_eq!(Err(HResult(E_NOINTERFACE)), button.cast::<IRawElementProviderFragmentRoot>().map(|_| ()));
    let root_of_button = button.cast::<IRawElementProviderFragment>().unwrap().get_fragment_root().unwrap();
    assert_eq!(Some(fragment_root.as_ptr()), root_of_button.map(|root| root.as_ptr()));
    assert!(button.get_host_raw_element_provider().unwrap().is_none());

    // Every interface of the class is one object.
    let identity = button.cast::<IUnknown>().unwrap().as_ptr();
    assert_eq!(identity, button.cast::<IRawElementProviderSimple2>().unwrap().cast::<IUnknown>().unwrap().as_ptr());
    assert_eq!(identity, button.cast::<IInvokeProvider>().unwrap().cast::<IUnknown>().unwrap().as_ptr());
    assert_eq!(identity, button.cast::<IScrollItemProvider>().unwrap().cast::<IUnknown>().unwrap().as_ptr());
    assert!(button.cast::<IExpandCollapseProvider>().is_ok());
    assert!(button.cast::<ISelectionItemProvider>().is_ok());
}

#[test]
fn the_tree_is_walked_in_every_direction_and_a_peer_has_one_node() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let ui = ui();

    let all = descendants(&ui.root);
    let types: Vec<UiaControlTypeId> = all.iter().map(|node| control_type(node)).collect();
    for wanted in [UiaControlTypeId::Button, UiaControlTypeId::Edit, UiaControlTypeId::CheckBox, UiaControlTypeId::Slider] {
        assert_eq!(1, types.iter().filter(|control_type| **control_type == wanted).count(), "{wanted:?} in {types:?}");
    }

    // Walked again, the nodes are the same objects with the same numbers.
    let again = descendants(&ui.root);
    assert_eq!(
        all.iter().map(|node| node.as_ptr()).collect::<Vec<_>>(),
        again.iter().map(|node| node.as_ptr()).collect::<Vec<_>>()
    );
    let mut numbers: Vec<String> = all.iter().map(|node| number(Some(node))).collect();
    numbers.sort();
    numbers.dedup();
    assert_eq!(all.len(), numbers.len());

    for node in &all {
        // The parent of a node has the node among its children, between
        // its siblings.
        let parent = navigate(node, NavigateDirection::Parent).expect("a parent");
        let siblings = children(&parent);
        let index = siblings.iter().position(|sibling| sibling.as_ptr() == node.as_ptr()).expect("the node among them");
        let previous = navigate(node, NavigateDirection::PreviousSibling).map(|node| node.as_ptr());
        let next = navigate(node, NavigateDirection::NextSibling).map(|node| node.as_ptr());
        assert_eq!(index.checked_sub(1).map(|index| siblings[index].as_ptr()), previous);
        assert_eq!(siblings.get(index + 1).map(|sibling| sibling.as_ptr()), next);
        let last = navigate(&parent, NavigateDirection::LastChild).expect("a last child");
        assert_eq!(siblings.last().map(|sibling| sibling.as_ptr()), Some(last.as_ptr()));

        // The runtime identifier through the fragment is the one of the
        // property.
        let fragment = node.cast::<IRawElementProviderFragment>().unwrap();
        let array = fragment.get_runtime_id().unwrap();
        // SAFETY: an array of integers the node made, which this test owns.
        let id = unsafe {
            let id = SafeArrayMarshaller::<i32>::convert_to_managed(array).unwrap().unwrap();
            SafeArrayMarshaller::<i32>::free(array);
            id
        };
        assert_eq!(3, id[0]);
        assert_eq!(number(Some(node)), format!("#{}", id[1]));
        assert!(fragment.get_embedded_fragment_roots().unwrap().is_null());
    }

    // The same walk from another thread, while this one runs its jobs.
    let root = SendProvider(ui.root.clone());
    let count = on_another_thread(move || descendants(root.get()).len());
    assert_eq!(all.len(), count);
}

#[test]
fn the_properties_of_the_controls() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let ui = ui();

    let button = find(&ui.root, UiaControlTypeId::Button);
    assert_eq!(Some("Save".to_owned()), text(&button, UiaPropertyId::Name));
    assert_eq!(Some("Button".to_owned()), text(&button, UiaPropertyId::ClassName));
    assert_eq!(Some("button".to_owned()), text(&button, UiaPropertyId::LocalizedControlType));
    assert_eq!(Some("FerroUI".to_owned()), text(&button, UiaPropertyId::FrameworkId));
    assert!(flag(&button, UiaPropertyId::IsEnabled));
    assert!(flag(&button, UiaPropertyId::IsKeyboardFocusable));
    assert!(flag(&button, UiaPropertyId::IsControlElement));
    assert!(flag(&button, UiaPropertyId::IsContentElement));
    assert!(!flag(&button, UiaPropertyId::HasKeyboardFocus));
    assert!(!flag(&button, UiaPropertyId::IsOffscreen));
    assert_eq!(Some(VariantValue::I4(std::process::id() as i32)), value(&button, UiaPropertyId::ProcessId));
    assert_eq!(Some(VariantValue::I4(80050)), value(&button, UiaPropertyId::HeadingLevel));
    assert_eq!(Some(VariantValue::I4(0)), value(&button, UiaPropertyId::LiveSetting));
    assert!(matches!(value(&button, UiaPropertyId::Culture), Some(VariantValue::I4(_))));
    // What the peer does not have, and what the node does not answer (the
    // core asks the pattern for the properties of a pattern).
    assert_eq!(None, value(&button, UiaPropertyId::AcceleratorKey));
    assert_eq!(None, value(&button, UiaPropertyId::ItemStatus));
    assert_eq!(None, value(&button, UiaPropertyId::LandmarkType));
    assert_eq!(None, value(&button, UiaPropertyId::LocalizedLandmarkType));
    assert_eq!(None, value(&button, UiaPropertyId::ToggleToggleState));
    assert_eq!(None, value(&button, UiaPropertyId::NativeWindowHandle));

    // The clickable point is the centre of the bounding rectangle.
    let rect = button.cast::<IRawElementProviderFragment>().unwrap().get_bounding_rectangle().unwrap();
    assert_eq!(
        Some(VariantValue::Array(VariantArray::R8(vec![rect.left + rect.width / 2.0, rect.top + rect.height / 2.0]))),
        value(&button, UiaPropertyId::ClickablePoint)
    );

    ui.button.set_is_enabled(false);
    assert!(!flag(&button, UiaPropertyId::IsEnabled));

    let edit = find(&ui.root, UiaControlTypeId::Edit);
    assert_eq!(Some("TextBox".to_owned()), text(&edit, UiaPropertyId::ClassName));
    assert!(flag(&edit, UiaPropertyId::IsKeyboardFocusable));
    let check_box = find(&ui.root, UiaControlTypeId::CheckBox);
    assert_eq!(Some("Agree".to_owned()), text(&check_box, UiaPropertyId::Name));
    assert_eq!(Some("check box".to_owned()), text(&check_box, UiaPropertyId::LocalizedControlType));
    let slider = find(&ui.root, UiaControlTypeId::Slider);
    assert_eq!(Some("Slider".to_owned()), text(&slider, UiaPropertyId::ClassName));
}

#[test]
fn the_patterns_a_peer_provides_are_the_ones_a_client_gets() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let ui = ui();

    let button = find(&ui.root, UiaControlTypeId::Button);
    let edit = find(&ui.root, UiaControlTypeId::Edit);
    let check_box = find(&ui.root, UiaControlTypeId::CheckBox);
    let slider = find(&ui.root, UiaControlTypeId::Slider);

    assert!(has_pattern(&button, UiaPatternId::Invoke));
    assert!(!has_pattern(&button, UiaPatternId::Toggle));
    assert!(!has_pattern(&button, UiaPatternId::Value));
    assert!(has_pattern(&check_box, UiaPatternId::Toggle));
    assert!(!has_pattern(&check_box, UiaPatternId::Invoke));
    assert!(has_pattern(&edit, UiaPatternId::Value));
    assert!(!has_pattern(&edit, UiaPatternId::RangeValue));
    assert!(has_pattern(&slider, UiaPatternId::RangeValue));
    // Every node brings itself into view; no node has a pattern the
    // reference does not answer.
    for node in [&button, &edit, &check_box, &slider] {
        assert!(has_pattern(node, UiaPatternId::ScrollItem));
        assert!(!has_pattern(node, UiaPatternId::Text));
        assert!(!has_pattern(node, UiaPatternId::Window));
    }
    // The provider of a pattern is the node.
    assert_eq!(
        Some(button.cast::<IUnknown>().unwrap().as_ptr()),
        button.get_pattern_provider(UiaPatternId::Invoke.0).unwrap().map(|provider| provider.as_ptr())
    );

    // Invoke.
    button.cast::<IInvokeProvider>().unwrap().invoke().unwrap();
    assert_eq!(1, ui.clicks.get());
    // A pattern the peer does not provide fails as in the reference.
    assert_eq!(Err(NOT_SUPPORTED), check_box.cast::<IInvokeProvider>().unwrap().invoke());
    assert_eq!(Err(NOT_SUPPORTED), button.cast::<IToggleProvider>().unwrap().get_toggle_state());

    // Toggle, with the event of the property.
    let toggle = check_box.cast::<IToggleProvider>().unwrap();
    assert_eq!(ToggleState::Off as i32, toggle.get_toggle_state().unwrap());
    ui.take_calls();
    toggle.toggle().unwrap();
    assert_eq!(Some(true), ui.check_box.is_checked());
    assert_eq!(ToggleState::On as i32, toggle.get_toggle_state().unwrap());
    assert_eq!(
        vec![format!("property {} ToggleToggleState Some(I4(0)) -> Some(I4(1))", number(Some(&check_box)))],
        ui.take_calls()
    );

    // Value.
    let value_provider = edit.cast::<IValueProvider>().unwrap();
    let read = |provider: &IValueProvider| {
        let bstr = provider.get_value().unwrap();
        // SAFETY: the string the node made, which this test owns.
        unsafe {
            let text = ptr_to_string_bstr(bstr);
            free_bstr(bstr);
            text
        }
    };
    assert_eq!(Some("Hello world".to_owned()), read(&value_provider));
    assert_eq!(0, value_provider.get_is_read_only().unwrap());
    let typed: Vec<u16> = "typed by a client".encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: a null-terminated string that lives through the call.
    unsafe { value_provider.set_value(typed.as_ptr()) }.unwrap();
    assert_eq!(Some("typed by a client"), ui.text_box.text().as_deref());
    assert_eq!(Some("typed by a client".to_owned()), read(&value_provider));

    // Range value.
    let range = slider.cast::<IRangeValueProvider>().unwrap();
    assert_eq!(25.0, range.get_value().unwrap());
    assert_eq!(0.0, range.get_minimum().unwrap());
    assert_eq!(100.0, range.get_maximum().unwrap());
    assert_eq!(0, range.get_is_read_only().unwrap());
    assert_eq!((1.0, 1.0), (range.get_small_change().unwrap(), range.get_large_change().unwrap()));
    ui.take_calls();
    range.set_value(60.0).unwrap();
    assert_eq!(60.0, ui.slider.value());
    assert_eq!(
        vec![format!("property {} RangeValueValue Some(R8(25.0)) -> Some(R8(60.0))", number(Some(&slider)))],
        ui.take_calls()
    );

    // An element that is not enabled has the result code of the system.
    ui.button.set_is_enabled(false);
    assert_eq!(Err(ELEMENT_NOT_ENABLED), button.cast::<IInvokeProvider>().unwrap().invoke());
    assert_eq!(1, ui.clicks.get());

    // The patterns from another thread.
    let (check_box, slider) = (SendProvider(check_box), SendProvider(slider));
    let (state, value) = on_another_thread(move || {
        let toggle = check_box.get().cast::<IToggleProvider>().unwrap();
        toggle.toggle().unwrap();
        let range = slider.get().cast::<IRangeValueProvider>().unwrap();
        range.set_value(10.0).unwrap();
        (toggle.get_toggle_state().unwrap(), range.get_value().unwrap())
    });
    // A check box without a third state goes from checked to unchecked.
    assert_eq!((ToggleState::Off as i32, 10.0), (state, value));
    assert_eq!(Some(false), ui.check_box.is_checked());
    assert_eq!(10.0, ui.slider.value());
}

#[test]
fn the_events_of_the_peers_are_raised_for_their_nodes() {
    // The services of a window that takes the focus: a window peer follows
    // the keyboard device.
    let focusable = TestServices::focusable_window();
    let _app = UnitTestApplication::start(TestServices {
        keyboard_device: focusable.keyboard_device,
        keyboard_navigation: focusable.keyboard_navigation,
        input_manager: focusable.input_manager,
        ..TestServices::styled_window()
    });
    let ui = ui();
    let panel = ui.node_of(&ui.panel);
    let edit = ui.node_of(&ui.text_box);
    ui.take_calls();

    // A change of the children of a peer invalidates the children of its
    // node.
    let late = Button::new();
    late.set_content(Some(Rc::new("Late".to_string())));
    ui.panel.children().add(&late);
    ui.window.layout_manager().execute_layout_pass();
    Dispatcher::ui_thread().run_jobs(None);
    let calls = ui.take_calls();
    assert!(
        calls.contains(&format!("structure {} ChildrenInvalidated None", number(Some(&panel)))),
        "the calls: {calls:?}"
    );
    assert!(children(&panel).iter().any(|child| text(child, UiaPropertyId::Name).as_deref() == Some("Late")));

    // The focus of the window is raised for the node of the element.
    ui.text_box.focus();
    Dispatcher::ui_thread().run_jobs(None);
    let calls = ui.take_calls();
    assert!(
        calls.contains(&format!("event {} AutomationFocusChanged", number(Some(&edit)))),
        "the calls: {calls:?}"
    );
    assert!(flag(&edit, UiaPropertyId::HasKeyboardFocus));
    let focus = ui.root.cast::<IRawElementProviderFragmentRoot>().unwrap().get_focus().unwrap().expect("the focus");
    assert_eq!(edit.cast::<IRawElementProviderFragment>().unwrap().as_ptr(), focus.as_ptr());

    // A node sets the focus.
    let button = ui.node_of(&ui.button);
    button.cast::<IRawElementProviderFragment>().unwrap().set_focus().unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(flag(&button, UiaPropertyId::HasKeyboardFocus));
    assert!(ui.take_calls().contains(&format!("event {} AutomationFocusChanged", number(Some(&button)))));
}

#[test]
fn a_node_whose_peer_is_forgotten_answers_that_its_element_is_not_available() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let ui = ui();
    let button = ui.node_of(&ui.button);
    let peer: Ref<AutomationPeer> = ControlAutomationPeer::create_peer_for_element(&ui.button);
    let before = AutomationNode::count();

    AutomationNode::release(&peer);

    assert_eq!(before - 1, AutomationNode::count());
    assert_eq!(Err(ELEMENT_NOT_AVAILABLE), button.get_property_value(UiaPropertyId::Name.0).map(|_| ()));
    assert_eq!(Err(ELEMENT_NOT_AVAILABLE), button.cast::<IInvokeProvider>().unwrap().invoke());
    assert_eq!(0, ui.clicks.get());
    // What does not ask the peer still answers.
    assert_eq!(ProviderOptions::ServerSideProvider, button.get_provider_options().unwrap());
    assert!(matches!(value(&button, UiaPropertyId::RuntimeId), Some(VariantValue::Array(VariantArray::I4(_)))));
    // The peer gets a new node, with a number of its own.
    let again = ui.node_of(&ui.button);
    assert_ne!(button.as_ptr(), again.as_ptr());
    assert_ne!(number(Some(&button)), number(Some(&again)));
    assert_eq!(Some("Save".to_owned()), text(&again, UiaPropertyId::Name));
}

// Not in the reference, whose peer holds its control: the peer of a control that is gone is
// still held (by the children of its parent peer, as when a page is replaced), and a client asks
// its node for the fragment root, its neighbours, its bounds and its properties.
#[test]
fn the_node_of_a_peer_whose_control_is_gone_answers_that_its_element_is_not_available() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let border = ferroui_controls::Border::new();
    let peer: Ref<AutomationPeer> = ControlAutomationPeer::create_peer_for_element(&border);
    let node = AutomationNode::get_or_create(Some(&peer)).expect("a node");
    let fragment = node.cast::<IRawElementProviderFragment>().unwrap();
    assert!(fragment.get_bounding_rectangle().is_ok());

    drop(border);
    assert!(!peer.is_element_available());

    assert_eq!(Err(ELEMENT_NOT_AVAILABLE), fragment.get_fragment_root().map(|_| ()));
    assert_eq!(Err(ELEMENT_NOT_AVAILABLE), fragment.navigate(NavigateDirection::Parent).map(|_| ()));
    assert_eq!(Err(ELEMENT_NOT_AVAILABLE), fragment.navigate(NavigateDirection::FirstChild).map(|_| ()));
    assert_eq!(Err(ELEMENT_NOT_AVAILABLE), fragment.get_bounding_rectangle().map(|_| ()));
    assert_eq!(Err(ELEMENT_NOT_AVAILABLE), fragment.set_focus());
    assert_eq!(Err(ELEMENT_NOT_AVAILABLE), node.get_property_value(UiaPropertyId::Name.0).map(|_| ()));
    assert_eq!(Err(ELEMENT_NOT_AVAILABLE), node.get_pattern_provider(UiaPatternId::Invoke.0).map(|_| ()));
    // What does not ask the peer still answers.
    assert_eq!(ProviderOptions::ServerSideProvider, node.get_provider_options().unwrap());
}

#[test]
fn the_control_types_the_landmarks_and_the_heading_levels_of_the_system() {
    use ferroui_controls::automation::peers::{AutomationControlType, AutomationLandmarkType};
    use ferroui_controls::automation::ExpandCollapseState;

    assert_eq!(UiaControlTypeId::Group, AutomationNode::to_uia_control_type(AutomationControlType::None));
    assert_eq!(UiaControlTypeId::ListItem, AutomationNode::to_uia_control_type(AutomationControlType::ComboBoxItem));
    assert_eq!(UiaControlTypeId::Group, AutomationNode::to_uia_control_type(AutomationControlType::Expander));
    assert_eq!(UiaControlTypeId::Pane, AutomationNode::to_uia_control_type(AutomationControlType::ScrollViewer));
    assert_eq!(UiaControlTypeId::Separator, AutomationNode::to_uia_control_type(AutomationControlType::Separator));
    assert_eq!(UiaControlTypeId::Edit, AutomationNode::to_uia_control_type(AutomationControlType::Edit));

    assert_eq!(None, AutomationNode::to_uia_landmark_type(None));
    assert_eq!(Some(80000), AutomationNode::to_uia_landmark_type(Some(AutomationLandmarkType::Banner)).map(|x| x.0));
    assert_eq!(Some(80002), AutomationNode::to_uia_landmark_type(Some(AutomationLandmarkType::Main)).map(|x| x.0));
    assert_eq!(Some(80004), AutomationNode::to_uia_landmark_type(Some(AutomationLandmarkType::Search)).map(|x| x.0));
    assert_eq!(
        Some("content information"),
        AutomationNode::to_uia_localized_landmark_type(Some(AutomationLandmarkType::ContentInfo))
    );
    assert_eq!(None, AutomationNode::to_uia_localized_landmark_type(Some(AutomationLandmarkType::Main)));

    assert_eq!(80050, AutomationNode::to_uia_heading_level(0).0);
    assert_eq!(80051, AutomationNode::to_uia_heading_level(1).0);
    assert_eq!(80059, AutomationNode::to_uia_heading_level(9).0);
    assert_eq!(80050, AutomationNode::to_uia_heading_level(10).0);

    // The two states the framework declares in the other order.
    assert_eq!(2, AutomationNode::to_uia_expand_collapse_state(ExpandCollapseState::PartiallyExpanded));
    assert_eq!(3, AutomationNode::to_uia_expand_collapse_state(ExpandCollapseState::LeafNode));
    assert_eq!(0, AutomationNode::to_uia_expand_collapse_state(ExpandCollapseState::Collapsed));
    assert_eq!(1, AutomationNode::to_uia_expand_collapse_state(ExpandCollapseState::Expanded));
}

/// The window of the accessibility run of the example (`win32_window
/// --a11y`): controls without a theme below a window that has the template
/// of a top level, with their sizes set and their names given as
/// automation names. What the run expects of them holds here, on every
/// host, before it is asked of the system.
#[test]
fn controls_without_a_theme_have_what_the_smoke_run_reads() {
    use ferroui_controls::automation::AutomationProperties;
    use ferroui_controls::testing::top_level_template;

    let focusable = TestServices::focusable_window();
    let _app = UnitTestApplication::start(TestServices {
        keyboard_device: focusable.keyboard_device,
        keyboard_navigation: focusable.keyboard_navigation,
        input_manager: focusable.input_manager,
        theme: None,
        ..TestServices::styled_window()
    });
    let recorder = Rc::new(Recorder::default());
    UiaCoreProviderApi::set_for_thread(Some(recorder.clone()));

    let button = Button::new();
    button.set_content(Some(Rc::new("Invoke me".to_string())));
    AutomationProperties::set_name(&button, Some("Invoke me"));
    let text_box = TextBox::new();
    text_box.set_text(Some("Hello"));
    let check_box = CheckBox::new();
    AutomationProperties::set_name(&check_box, Some("Check me"));
    let slider = Slider::new();
    slider.set_minimum(0.0);
    slider.set_maximum(100.0);
    slider.set_range_value(25.0);
    let panel = StackPanel::new();
    for control in [&*button as &Control, &text_box, &check_box, &slider] {
        control.set_width(200.0);
        control.set_height(32.0);
    }
    panel.children().add(&button);
    panel.children().add(&text_box);
    panel.children().add(&check_box);
    panel.children().add(&slider);
    let window = Window::new();
    window.set_template(Some(top_level_template()));
    window.set_title(Some("Smoke".to_string()));
    window.set_width(480.0);
    window.set_height(360.0);
    window.set_content(Some(Control::boxed(&panel)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    window.activate();

    let peer = ControlAutomationPeer::create_peer_for_element(&window);
    let root = AutomationNode::get_or_create(Some(&peer)).expect("the node of the window");
    assert_eq!(UiaControlTypeId::Window, control_type(&root));
    assert_eq!(Some("Smoke".to_owned()), text(&root, UiaPropertyId::Name));

    let rect = |node: &IRawElementProviderSimple| {
        let rect = ComPtr::from_ref(node).cast::<IRawElementProviderFragment>().unwrap().get_bounding_rectangle().unwrap();
        (rect.width, rect.height)
    };
    let found_button = find(&root, UiaControlTypeId::Button);
    assert_eq!(Some("Invoke me".to_owned()), text(&found_button, UiaPropertyId::Name));
    let edit = find(&root, UiaControlTypeId::Edit);
    let found_check_box = find(&root, UiaControlTypeId::CheckBox);
    assert_eq!(Some("Check me".to_owned()), text(&found_check_box, UiaPropertyId::Name));
    let found_slider = find(&root, UiaControlTypeId::Slider);
    for node in [&found_button, &edit, &found_check_box, &found_slider] {
        assert!(flag(node, UiaPropertyId::IsEnabled));
        assert!(flag(node, UiaPropertyId::IsKeyboardFocusable), "{:?}", control_type(node));
        assert_eq!((200.0, 32.0), rect(node), "{:?}", control_type(node));
    }
    let patterns = |node: &IRawElementProviderSimple| {
        [UiaPatternId::Invoke, UiaPatternId::Toggle, UiaPatternId::Value, UiaPatternId::RangeValue]
            .map(|pattern| has_pattern(node, pattern))
    };
    assert_eq!([true, false, false, false], patterns(&found_button));
    assert_eq!([false, true, false, false], patterns(&found_check_box));
    assert_eq!([false, false, true, false], patterns(&edit));
    assert!(patterns(&found_slider)[3] && !patterns(&found_slider)[0]);

    let toggle = found_check_box.cast::<IToggleProvider>().unwrap();
    assert_eq!(0, toggle.get_toggle_state().unwrap());
    toggle.toggle().unwrap();
    assert_eq!(1, toggle.get_toggle_state().unwrap());
    let range = found_slider.cast::<IRangeValueProvider>().unwrap();
    assert_eq!(25.0, range.get_value().unwrap());
    range.set_value(60.0).unwrap();
    assert_eq!(60.0, slider.value());

    // The focus through the node, and its event.
    recorder.calls.take();
    edit.cast::<IRawElementProviderFragment>().unwrap().set_focus().unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(flag(&edit, UiaPropertyId::HasKeyboardFocus));
    let calls = recorder.calls.take();
    assert!(calls.contains(&format!("event {} AutomationFocusChanged", number(Some(&edit)))), "the calls: {calls:?}");
    UiaCoreProviderApi::set_for_thread(None);
}
