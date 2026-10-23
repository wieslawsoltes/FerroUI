//! Integration tests call the native COM text pattern over actual FerroUI rich
//! controls. Attribute cases mirror WinUI's native TextBlock/RichTextBlock UIA
//! tests; threading, identity, lifetime, and Unicode cases exercise this bridge.

use super::automation_node::{AutomationNode, SendProvider, ELEMENT_NOT_AVAILABLE};
use super::interop::{
    IInvokeProvider, IRawElementProviderSimple, ITextChildProvider, ITextEditProvider, ITextProvider, ITextProvider2,
    ITextRangeProvider, IUiaCoreProviderApi, StructureChangeType, SupportedTextSelection,
    TextEditChangeType, TextPatternRangeEndpoint, TextUnit, UiaCoreProviderApi, UiaCoreTypesApi, UiaEventId,
    UiaPatternId, UiaPoint, UiaPropertyId,
};
use super::marshalling::{
    free_bstr, ptr_to_string_bstr, string_to_bstr, ComVariant, SafeArrayElement,
    SafeArrayMarshaller, SafeArrayRef, VariantValue,
};
use super::text_range_node::TextRangeNode;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{
    Color, FlowDirection, FontFamily, FontWeight, IBrush, TextDecoration, TextDecorationCollection,
    TextDecorationLocation,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::FerroLocator;
use ferroui_controls::automation::peers::ControlAutomationPeer;
use ferroui_controls::documents::{
    CharacterFormat, Hyperlink, InlineUIContainer, Paragraph, RichTextDocument, Run,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, Control, RichEditBox, RichTextBlock, Window};
use ferroui_microcom::{ComPtr, HResult, IUnknown};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

struct TextEditRecord {
    change_type: TextEditChangeType,
    changed_data: Vec<String>,
    identity: usize,
    thread: std::thread::ThreadId,
}

#[derive(Default)]
struct Recorder {
    events: RefCell<Vec<i32>>,
    text_edits: RefCell<Vec<TextEditRecord>>,
    text_edit_result: Cell<i32>,
    text_edit_callback: RefCell<Option<Rc<dyn Fn()>>>,
}

impl IUiaCoreProviderApi for Recorder {
    fn uia_clients_are_listening(&self) -> bool {
        true
    }
    fn uia_return_raw_element_provider(
        &self,
        _: isize,
        _: usize,
        _: isize,
        _: Option<&IRawElementProviderSimple>,
    ) -> isize {
        0
    }
    fn uia_host_provider_from_hwnd(
        &self,
        _: isize,
    ) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
        Ok(None)
    }
    fn uia_raise_automation_event(&self, _: Option<&IRawElementProviderSimple>, event: i32) -> i32 {
        self.events.borrow_mut().push(event);
        0
    }
    fn uia_raise_text_edit_text_changed_event(
        &self,
        provider: Option<&IRawElementProviderSimple>,
        change_type: TextEditChangeType,
        changed_data: SafeArrayRef,
    ) -> i32 {
        assert!(!changed_data.is_null());
        // SAFETY: UiaCoreProviderApi lends a one-dimensional VT_BSTR array.
        // Inspect its ABI shape and copy strings without freeing borrowed data.
        let data = unsafe {
            let descriptor = &*changed_data.as_ptr();
            assert_eq!(descriptor.c_dims, 1);
            assert_eq!(descriptor.cb_elements as usize, std::mem::size_of::<usize>());
            assert_eq!(descriptor.rgsabound[0].l_lbound, 0);
            SafeArrayMarshaller::<String>::convert_to_managed(changed_data).unwrap().unwrap()
        };
        let identity = super::text_pattern_marshalling::identity_of(provider.unwrap()).unwrap();
        self.text_edits.borrow_mut().push(TextEditRecord {
            change_type,
            changed_data: data,
            identity: identity.as_ptr() as usize,
            thread: std::thread::current().id(),
        });
        self.events.borrow_mut().push(UiaEventId::TextEdit_TextChanged.0);
        let callback = self.text_edit_callback.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
        self.text_edit_result.get()
    }

    fn uia_raise_automation_property_changed_event(
        &self,
        _: Option<&IRawElementProviderSimple>,
        _: i32,
        _: Option<&VariantValue>,
        _: Option<&VariantValue>,
    ) -> i32 {
        0
    }
    fn uia_raise_structure_changed_event(
        &self,
        _: Option<&IRawElementProviderSimple>,
        _: StructureChangeType,
        _: Option<&[i32]>,
    ) -> i32 {
        0
    }
    fn uia_disconnect_provider(&self, _: Option<&IRawElementProviderSimple>) -> i32 {
        0
    }
}

struct RecordingScope(Rc<Recorder>, Rc<dyn IDisposable>);
impl RecordingScope {
    fn new() -> Self {
        let scope = FerroLocator::enter_scope();
        super::Win32AutomationEventSink::register();
        let recorder = Rc::new(Recorder::default());
        UiaCoreProviderApi::set_for_thread(Some(recorder.clone()));
        Self(recorder, scope)
    }
    fn take(&self) -> Vec<i32> {
        self.0.events.take()
    }
}
impl Drop for RecordingScope {
    fn drop(&mut self) {
        UiaCoreProviderApi::set_for_thread(None);
        self.1.dispose();
    }
}

fn native_node(control: &Control) -> ComPtr<IRawElementProviderSimple> {
    let peer = ControlAutomationPeer::create_peer_for_element(control);
    AutomationNode::get_or_create(Some(&peer)).unwrap()
}

fn on_worker<R: Send + 'static>(call: impl FnOnce() -> R + Send + 'static) -> R {
    let worker = std::thread::spawn(call);
    let deadline = Instant::now() + Duration::from_secs(20);
    while !worker.is_finished() {
        Dispatcher::ui_thread().run_jobs(None);
        assert!(Instant::now() < deadline, "native text call did not return");
        std::thread::sleep(Duration::from_millis(1));
    }
    let result = worker.join().expect("native text call on worker thread");
    // Final COM release on the worker queues disposal of its portable ranges.
    Dispatcher::ui_thread().run_jobs(None);
    result
}

fn text(range: &ITextRangeProvider, maximum: i32) -> String {
    let value = range.get_text(maximum).unwrap();
    // SAFETY: the native range returned an owned BSTR, released exactly once.
    unsafe {
        let result = ptr_to_string_bstr(value).unwrap();
        free_bstr(value);
        result
    }
}

fn attribute(range: &ITextRangeProvider, id: i32) -> VariantValue {
    let mut value = range.get_attribute_value(id).unwrap();
    // SAFETY: the native range returned an owned VARIANT. `as_object` copies
    // its value before the original's native allocation is released.
    unsafe {
        let result = value.as_object().unwrap().unwrap();
        value.dispose();
        result
    }
}

fn array<T: SafeArrayElement>(value: SafeArrayRef) -> Vec<T> {
    // SAFETY: each caller provides the array of the element type returned by
    // that UIA method, then releases the owned array once after copying it.
    unsafe {
        let result = SafeArrayMarshaller::<T>::convert_to_managed(value)
            .unwrap()
            .unwrap();
        SafeArrayMarshaller::<T>::free(value);
        result
    }
}

#[test]
fn native_text_pattern_marshals_unicode_ranges_and_selection_from_worker_thread() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _recording = RecordingScope::new();
    let editor = RichEditBox::new();
    editor.set_text("a😀 b\0c\nlast").unwrap();
    let node = native_node(&editor);
    let identity = node.cast::<IUnknown>().unwrap().as_ptr() as usize;
    let before = TextRangeNode::count();
    let node = SendProvider(node);
    on_worker(move || {
        let advertised = node
            .get()
            .get_pattern_provider(UiaPatternId::Text.0)
            .unwrap()
            .unwrap();
        assert_eq!(advertised.as_ptr() as usize, identity);
        let provider = advertised.cast::<ITextProvider>().unwrap();
        assert_eq!(
            provider.get_supported_text_selection(),
            Ok(SupportedTextSelection::Single)
        );
        let document = provider.get_document_range().unwrap().unwrap();
        assert_eq!(text(&document, -1), "a😀 b\0c\nlast");
        assert_eq!(text(&document, 2), "a");
        assert_eq!(text(&document, 3), "a😀");
        assert_eq!(document.get_text(-2).map(|_| ()), Err(HResult::INVALIDARG));
        let clone = ITextRangeProvider::clone(&document).unwrap().unwrap();
        assert_eq!(document.compare(Some(&clone)), Ok(1));
        assert_eq!(document.compare(None), Err(HResult::INVALIDARG));
        assert_eq!(
            document.expand_to_enclosing_unit(TextUnit(9)),
            Err(HResult::INVALIDARG)
        );

        let needle = string_to_bstr("😀 b\0c");
        // SAFETY: needle is an owned, live BSTR for this synchronous call.
        let found = unsafe { document.find_text(needle, 0, 0) }
            .unwrap()
            .unwrap();
        // SAFETY: this test owns the BSTR passed as a borrowed search input.
        unsafe { free_bstr(needle) };
        assert_eq!(text(&found, -1), "😀 b\0c");
        assert_eq!(
            found.compare_endpoints(
                TextPatternRangeEndpoint::Start,
                Some(&document),
                TextPatternRangeEndpoint::Start
            ),
            Ok(1)
        );
        found.select().unwrap();
        let selected = array::<ComPtr<ITextRangeProvider>>(provider.get_selection().unwrap());
        assert_eq!(selected.len(), 1);
        assert_eq!(text(&selected[0], -1), "😀 b\0c");
        found.add_to_selection().unwrap();
        assert_eq!(
            found.move_endpoint_by_unit(TextPatternRangeEndpoint::Start, TextUnit::Character, 1),
            Ok(1)
        );
        assert_eq!(text(&found, -1), " b\0c");
        found
            .move_endpoint_by_range(
                TextPatternRangeEndpoint::Start,
                Some(&document),
                TextPatternRangeEndpoint::Start,
            )
            .unwrap();
        found.expand_to_enclosing_unit(TextUnit::Character).unwrap();
        assert_eq!(text(&found, -1), "a");
        assert_eq!(found.move_(TextUnit::Character, 1), Ok(1));
        assert_eq!(text(&found, -1), "😀");
        found.select().unwrap();
    });
    assert_eq!(editor.selected_text(), "😀");
    assert_eq!(TextRangeNode::count(), before);
}

#[test]
fn native_text_attributes_use_native_reserved_values_and_colorref() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _recording = RecordingScope::new();
    let editor = RichEditBox::new();
    editor.set_text("plain bold").unwrap();
    editor
        .document()
        .apply_format(
            6..10,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                foreground: Some(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                    0x12, 0x34, 0x56,
                ))) as Rc<dyn IBrush>)),
                ..CharacterFormat::default()
            },
        )
        .unwrap();
    let provider = native_node(&editor).cast::<ITextProvider>().unwrap();
    let document = provider.get_document_range().unwrap().unwrap();
    let VariantValue::Unknown(mixed) = attribute(&document, 40007) else {
        panic!("mixed font weight")
    };
    assert_eq!(
        mixed.cast::<IUnknown>().unwrap(),
        UiaCoreTypesApi::reserved_mixed_attribute_value()
            .unwrap()
            .cast::<IUnknown>()
            .unwrap()
    );
    let VariantValue::Unknown(unsupported) = attribute(&document, 49999) else {
        panic!("unsupported attribute")
    };
    assert_eq!(
        unsupported.cast::<IUnknown>().unwrap(),
        UiaCoreTypesApi::reserved_not_supported_value()
            .unwrap()
            .cast::<IUnknown>()
            .unwrap()
    );
    assert_ne!(mixed, unsupported);
    let mut weight = ComVariant::create(Some(&VariantValue::I4(700))).unwrap();
    let bold = document.find_attribute(40007, weight, 0).unwrap().unwrap();
    // SAFETY: the input VARIANT belongs to this test, not to the callee.
    unsafe { weight.dispose() };
    assert_eq!(text(&bold, -1), "bold");
    assert_eq!(attribute(&bold, 40008), VariantValue::I4(0x0056_3412));
    assert_eq!(attribute(&document, 40015), VariantValue::Bool(false));
    editor.set_is_read_only(true);
    assert_eq!(attribute(&document, 40015), VariantValue::Bool(true));
    let mut color = ComVariant::create(Some(&VariantValue::I4(0x0056_3412))).unwrap();
    let colored = document.find_attribute(40008, color, 0).unwrap().unwrap();
    unsafe { color.dispose() };
    assert_eq!(text(&colored, -1), "bold");
}

#[test]
fn native_text_geometry_children_and_hit_testing_use_real_document_layout() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _text = TextTestScope::new();
    let _recording = RecordingScope::new();
    let view = RichTextBlock::new();
    view.set_font_family(FontFamily::new("Test Sans"));
    view.set_font_size(16.0);
    let paragraph = Paragraph::with_text("Hello ");
    let child = Button::new();
    child.set_width(40.0);
    child.set_height(20.0);
    paragraph
        .inlines()
        .add(InlineUIContainer::with_child(&child));
    paragraph.inlines().add_text(" world");
    view.blocks().add(paragraph);
    let window = Window::new();
    window.set_width(320.0);
    window.set_height(200.0);
    window.set_content(Some(Control::boxed(&view)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    let child_node = native_node(&child);
    let provider = native_node(&view).cast::<ITextProvider>().unwrap();
    let document = provider.get_document_range().unwrap().unwrap();
    let children = array::<ComPtr<IRawElementProviderSimple>>(document.get_children().unwrap());
    assert_eq!(children.len(), 1);
    assert_eq!(
        children[0].cast::<IUnknown>().unwrap(),
        child_node.cast::<IUnknown>().unwrap()
    );
    let embedded = provider
        .range_from_child(Some(&child_node))
        .unwrap()
        .unwrap();
    assert_eq!(text(&embedded, -1), "\u{fffc}");
    assert_eq!(
        embedded
            .get_enclosing_element()
            .unwrap()
            .unwrap()
            .cast::<IUnknown>()
            .unwrap(),
        child_node.cast::<IUnknown>().unwrap()
    );
    assert_eq!(
        provider.range_from_child(None).map(|_| ()),
        Err(HResult::INVALIDARG)
    );

    let rectangles = array::<f64>(document.get_bounding_rectangles().unwrap());
    assert!(!rectangles.is_empty());
    assert_eq!(rectangles.len() % 4, 0);
    assert!(rectangles
        .chunks_exact(4)
        .all(|rect| rect[2] > 0.0 && rect[3] > 0.0));
    let peer = ControlAutomationPeer::create_peer_for_element(&view);
    let portable = peer
        .get_provider::<dyn ferroui_controls::automation::provider::ITextProvider>()
        .unwrap();
    let expected = portable
        .document_range()
        .unwrap()
        .get_bounding_rectangles()
        .unwrap()
        .into_iter()
        .map(|rect| peer.to_screen(rect).unwrap())
        .flat_map(|rect| [rect.x, rect.y, rect.width, rect.height])
        .collect::<Vec<_>>();
    assert_eq!(rectangles, expected);
    let hit = provider
        .range_from_point(UiaPoint {
            x: rectangles[0] + 1.0,
            y: rectangles[1] + 1.0,
        })
        .unwrap()
        .unwrap();
    hit.expand_to_enclosing_unit(TextUnit::Character).unwrap();
    assert_eq!(text(&hit, -1), "H");
    assert!(provider
        .range_from_point(UiaPoint {
            x: f64::NAN,
            y: 0.0
        })
        .is_err());
    let visible = array::<ComPtr<ITextRangeProvider>>(provider.get_visible_ranges().unwrap());
    assert!(!visible.is_empty());
    assert_eq!(text(&visible[0], -1), view.text());
    document.scroll_into_view(1).unwrap();
    window.close();
}

#[test]
fn native_text_ranges_follow_edits_and_reject_replacement_disconnection_and_foreign_owner() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _recording = RecordingScope::new();
    let editor = RichEditBox::new();
    editor.set_text("old document").unwrap();
    let node = native_node(&editor);
    let provider = node.cast::<ITextProvider>().unwrap();
    let original = provider.get_document_range().unwrap().unwrap();
    editor.document().replace_text(0..0, "new ").unwrap();
    assert_eq!(text(&original, -1), "new old document");

    let foreign_owner = RichEditBox::new();
    foreign_owner.set_text("elsewhere").unwrap();
    let foreign = native_node(&foreign_owner)
        .cast::<ITextProvider>()
        .unwrap()
        .get_document_range()
        .unwrap()
        .unwrap();
    assert_eq!(original.compare(Some(&foreign)), Err(HResult::INVALIDARG));
    assert_eq!(
        original.move_endpoint_by_range(
            TextPatternRangeEndpoint::Start,
            Some(&foreign),
            TextPatternRangeEndpoint::End
        ),
        Err(HResult::INVALIDARG)
    );
    editor.set_document(RichTextDocument::from_text("replacement"));
    assert_eq!(
        original.get_text(-1).map(|_| ()),
        Err(ELEMENT_NOT_AVAILABLE)
    );
    let fresh = provider.get_document_range().unwrap().unwrap();
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    AutomationNode::release(&peer);
    assert_eq!(fresh.get_text(-1).map(|_| ()), Err(ELEMENT_NOT_AVAILABLE));

    let next = native_node(&editor)
        .cast::<ITextProvider>()
        .unwrap()
        .get_document_range()
        .unwrap()
        .unwrap();
    let weak = editor.downgrade();
    drop(editor);
    assert!(weak.upgrade().is_none());
    assert_eq!(next.get_text(-1).map(|_| ()), Err(ELEMENT_NOT_AVAILABLE));
}

#[test]
fn native_text_events_observe_edits_formatting_selection_and_document_replacement() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let recording = RecordingScope::new();
    let editor = RichEditBox::new();
    editor.set_text("text").unwrap();
    let node = native_node(&editor);
    recording.take();
    editor.document().replace_text(0..1, "T").unwrap();
    assert!(recording.take().contains(&UiaEventId::Text_TextChanged.0));
    editor
        .document()
        .apply_format(
            0..4,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..CharacterFormat::default()
            },
        )
        .unwrap();
    assert!(recording.take().contains(&UiaEventId::Text_TextChanged.0));
    editor.select(1, 3);
    assert!(recording
        .take()
        .contains(&UiaEventId::Text_TextSelectionChanged.0));
    let old = editor.document();
    editor.set_document(RichTextDocument::from_text("other"));
    assert!(recording.take().contains(&UiaEventId::Text_TextChanged.0));
    old.replace_text(0..1, "t").unwrap();
    assert!(!recording.take().contains(&UiaEventId::Text_TextChanged.0));
    let mut available = node
        .get_property_value(UiaPropertyId::IsTextPatternAvailable.0)
        .unwrap();
    unsafe {
        assert_eq!(
            available.as_object().unwrap(),
            Some(VariantValue::Bool(true))
        );
        available.dispose();
    }
    let selection = node
        .cast::<ITextProvider>()
        .unwrap()
        .get_selection()
        .unwrap();
    let selected = array::<ComPtr<ITextRangeProvider>>(selection);
    selected[0].remove_from_selection().unwrap();
    assert_eq!(editor.selection_start(), editor.selection_end());
}

#[test]
fn native_text2_inherits_text_and_marshals_caret_bool_on_the_callers_thread() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _recording = RecordingScope::new();
    let editor = RichEditBox::new();
    editor.set_text("a😀b").unwrap();
    editor.select(4, 1);
    let node = SendProvider(native_node(&editor));
    on_worker(move || {
        let advertised = node
            .get()
            .get_pattern_provider(UiaPatternId::Text2.0)
            .unwrap()
            .unwrap();
        let provider = advertised.cast::<ITextProvider2>().unwrap();
        let text_provider = provider.cast::<ITextProvider>().unwrap();
        let all = text_provider.get_document_range().unwrap().unwrap();
        assert_eq!(text(&all, -1), "a😀b");
        let mut active = [0x1234_5678i32, -1, 0x3456_7890];
        // SAFETY: the middle entry is writable BOOL storage. The adjacent
        // sentinels verify that the COM bridge writes exactly one native BOOL.
        let caret = unsafe { provider.get_caret_range(&mut active[1]) }
            .unwrap()
            .unwrap();
        assert_eq!(active, [0x1234_5678, 0, 0x3456_7890]);
        assert_eq!(text(&caret, -1), "");
        assert_eq!(
            caret.compare_endpoints(
                TextPatternRangeEndpoint::Start,
                Some(&all),
                TextPatternRangeEndpoint::Start
            ),
            Ok(1)
        );
        // SAFETY: null is an explicitly supported validation input.
        assert_eq!(
            unsafe { provider.get_caret_range(std::ptr::null_mut()) }.map(|_| ()),
            Err(HResult::INVALIDARG)
        );
        assert_eq!(
            provider.range_from_annotation(None).map(|_| ()),
            Err(HResult::INVALIDARG)
        );
        assert_eq!(
            provider.range_from_annotation(Some(node.get())).map(|_| ()),
            Err(HResult::INVALIDARG)
        );
        let mut available = node
            .get()
            .get_property_value(UiaPropertyId::IsTextPattern2Available.0)
            .unwrap();
        // SAFETY: this call owns the returned VARIANT until it is disposed.
        unsafe {
            assert_eq!(
                available.as_object().unwrap(),
                Some(VariantValue::Bool(true))
            );
            available.dispose();
        }
    });
    assert_eq!((editor.selection_start(), editor.selection_end()), (4, 1));
}

#[test]
fn native_hyperlink_text_child_has_canonical_container_ranges_and_dispatches_invoke() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _recording = RecordingScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("before ");
    let link = Hyperlink::with_text("link😀", None);
    let invocations = Rc::new(Cell::new(0));
    let count = invocations.clone();
    link.click(move |args| {
        count.set(count.get() + 1);
        args.set_handled(true);
    });
    paragraph.add_child(link);
    paragraph.add_child_text(" after");
    view.blocks().add(paragraph);
    let node = SendProvider(native_node(&view));
    on_worker(move || {
        let text_provider = node.get().cast::<ITextProvider>().unwrap();
        let all = text_provider.get_document_range().unwrap().unwrap();
        let children = array::<ComPtr<IRawElementProviderSimple>>(all.get_children().unwrap());
        assert_eq!(children.len(), 1);
        let child = &children[0];
        assert!(child
            .get_pattern_provider(UiaPatternId::TextChild.0)
            .unwrap()
            .is_some());
        let child_text = child.cast::<ITextChildProvider>().unwrap();
        let container = child_text.get_text_container().unwrap().unwrap();
        assert_eq!(
            container.cast::<IUnknown>().unwrap(),
            node.get().cast::<IUnknown>().unwrap()
        );
        let range = child_text.get_text_range().unwrap().unwrap();
        assert_eq!(text(&range, -1), "link😀");
        assert_eq!(
            range.compare_endpoints(
                TextPatternRangeEndpoint::Start,
                Some(&all),
                TextPatternRangeEndpoint::Start
            ),
            Ok(7)
        );
        let from_child = text_provider
            .range_from_child(Some(child))
            .unwrap()
            .unwrap();
        assert_eq!(from_child.compare(Some(&range)), Ok(1));
        let enclosing = range.get_enclosing_element().unwrap().unwrap();
        assert_eq!(
            enclosing.cast::<IUnknown>().unwrap(),
            child.cast::<IUnknown>().unwrap()
        );
        assert!(
            array::<ComPtr<IRawElementProviderSimple>>(range.get_children().unwrap()).is_empty()
        );
        child.cast::<IInvokeProvider>().unwrap().invoke().unwrap();
        let mut available = child
            .get_property_value(UiaPropertyId::IsTextChildPatternAvailable.0)
            .unwrap();
        // SAFETY: the returned owned VARIANT is copied, then disposed once.
        unsafe {
            assert_eq!(
                available.as_object().unwrap(),
                Some(VariantValue::Bool(true))
            );
            available.dispose();
        }
    });
    assert_eq!(invocations.get(), 1);
}

#[test]
fn native_text_child_range_survives_child_removal_but_not_document_replacement() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _recording = RecordingScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    paragraph.add_child(Hyperlink::with_text("temporary", None));
    view.blocks().add(paragraph.clone());
    let text_provider = native_node(&view).cast::<ITextProvider>().unwrap();
    let all = text_provider.get_document_range().unwrap().unwrap();
    assert_eq!(
        all.get_enclosing_element()
            .unwrap()
            .unwrap()
            .cast::<IUnknown>()
            .unwrap(),
        text_provider.cast::<IUnknown>().unwrap()
    );
    let children = array::<ComPtr<IRawElementProviderSimple>>(all.get_children().unwrap());
    let child = children[0].cast::<ITextChildProvider>().unwrap();
    let range = child.get_text_range().unwrap().unwrap();
    assert!(array::<ComPtr<IRawElementProviderSimple>>(range.get_children().unwrap()).is_empty());
    paragraph.inlines().clear();
    assert_eq!(text(&range, -1), "");
    assert_eq!(
        child.get_text_range().map(|_| ()),
        Err(ELEMENT_NOT_AVAILABLE)
    );
    view.set_document(RichTextDocument::from_text("replacement"));
    assert_eq!(range.get_text(-1).map(|_| ()), Err(ELEMENT_NOT_AVAILABLE));
}

#[test]
fn native_extra_text_attributes_preserve_colorref_and_find_color_input_types() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _recording = RecordingScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    paragraph.set_flow_direction(FlowDirection::RightToLeft);
    let run = Run::with_text(Some("overline"));
    let overline = TextDecoration::new();
    overline.set_location(TextDecorationLocation::Overline);
    overline.set_stroke(Some(Rc::new(ImmutableSolidColorBrush::new(
        Color::from_rgb(0x12, 0x34, 0x56),
    ))));
    run.set_text_decorations(Some(TextDecorationCollection::from_items([overline])));
    paragraph.add_child(run);
    view.blocks().add(paragraph);
    let provider = native_node(&view).cast::<ITextProvider>().unwrap();
    let all = provider.get_document_range().unwrap().unwrap();
    assert_eq!(attribute(&all, 40023), VariantValue::I4(0x0056_3412));
    assert_eq!(attribute(&all, 40024), VariantValue::I4(1));
    assert_eq!(attribute(&all, 40028), VariantValue::I4(1));
    let mut color = ComVariant::create(Some(&VariantValue::UI4(0x0056_3412))).unwrap();
    let found = all.find_attribute(40023, color, 0).unwrap().unwrap();
    // SAFETY: the input VARIANT belongs to this test and is released once.
    unsafe {
        color.dispose();
    }
    assert_eq!(text(&found, -1), "overline");
}

#[test]
fn native_text_edit_inherits_text_shares_text2_identity_and_dispatches_live_composition_ranges() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _recording = RecordingScope::new();
    let editor = RichEditBox::new();
    editor.set_text("a😀b tail").unwrap();
    editor.select(1, 4);
    let node = native_node(&editor);
    let identity = node.cast::<IUnknown>().unwrap().as_ptr() as usize;
    let edit = node.cast::<ITextEditProvider>().unwrap();
    assert!(edit.get_active_composition().unwrap().is_none());
    editor.set_preedit_text(Some("候😀補"), Some(1)).unwrap();
    let selection = editor.editor().selection();
    let revision = editor.document().revision();
    let node = SendProvider(node);
    on_worker(move || {
        let advertised = node.get().get_pattern_provider(UiaPatternId::TextEdit.0).unwrap().unwrap();
        assert_eq!(advertised.as_ptr() as usize, identity);
        let edit = advertised.cast::<ITextEditProvider>().unwrap();
        let text_provider = edit.cast::<ITextProvider>().unwrap();
        let text2 = edit.cast::<ITextProvider2>().unwrap();
        assert_eq!(text_provider.cast::<IUnknown>().unwrap().as_ptr() as usize, identity);
        assert_eq!(text2.cast::<IUnknown>().unwrap().as_ptr() as usize, identity);
        let document = edit.get_document_range().unwrap().unwrap();
        assert_eq!(text(&document, -1), "a候😀補 tail");
        let composition = edit.get_active_composition().unwrap().unwrap();
        assert_eq!(text(&composition, -1), "候😀補");
        assert_eq!(composition.compare_endpoints(TextPatternRangeEndpoint::Start, Some(&document), TextPatternRangeEndpoint::Start), Ok(1));
        assert_eq!(composition.compare_endpoints(TextPatternRangeEndpoint::End, Some(&document), TextPatternRangeEndpoint::Start), Ok(5));
        assert_eq!(composition.get_enclosing_element().unwrap().unwrap().cast::<IUnknown>().unwrap().as_ptr() as usize, identity);
        let mut active = -1;
        // SAFETY: caller-owned writable BOOL storage remains on this thread.
        let caret = unsafe { text2.get_caret_range(&mut active) }.unwrap().unwrap();
        assert_eq!(caret.compare_endpoints(TextPatternRangeEndpoint::Start, Some(&document), TextPatternRangeEndpoint::Start), Ok(2));
        assert!(edit.get_conversion_target().unwrap().is_none());
        let mut available = node.get().get_property_value(UiaPropertyId::IsTextEditPatternAvailable.0).unwrap();
        // SAFETY: the call returned an owned VARIANT, released once after copy.
        unsafe {
            assert_eq!(available.as_object().unwrap(), Some(VariantValue::Bool(true)));
            available.dispose();
        }
    });
    assert_eq!(editor.editor().selection(), selection);
    assert_eq!(editor.document().revision(), revision);

    let view = RichTextBlock::new();
    let node = native_node(&view);
    assert!(node.get_pattern_provider(UiaPatternId::TextEdit.0).unwrap().is_none());
    let mut available = node.get_property_value(UiaPropertyId::IsTextEditPatternAvailable.0).unwrap();
    // SAFETY: this call owns its native VARIANT until disposed below.
    unsafe {
        assert_eq!(available.as_object().unwrap(), Some(VariantValue::Bool(false)));
        available.dispose();
    }
}

#[test]
fn native_text_edit_range_lifetime_follows_owner_and_document_not_composition() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _recording = RecordingScope::new();
    let (provider, retained_range, model) = {
        let editor = RichEditBox::new();
        editor.set_text("before").unwrap();
        let node = native_node(&editor);
        let provider = node.cast::<ITextEditProvider>().unwrap();
        editor.set_preedit_text(Some("preview"), Some(2)).unwrap();
        let range = provider.get_active_composition().unwrap().unwrap();
        editor.cancel_composition().unwrap();
        assert!(provider.get_active_composition().unwrap().is_none());
        assert!(range.get_text(-1).is_ok_and(|value| {
            // SAFETY: the range returned an owned BSTR; its text is irrelevant
            // here because live endpoints follow the model's cancellation.
            unsafe { free_bstr(value) };
            true
        }));
        editor.set_document(RichTextDocument::from_text("replacement"));
        assert_eq!(range.get_text(-1).map(|_| ()), Err(ELEMENT_NOT_AVAILABLE));
        editor.set_preedit_text(Some("live"), None).unwrap();
        let retained_range = provider.get_active_composition().unwrap().unwrap();
        (provider, retained_range, editor.editor())
    };
    assert_eq!(provider.get_active_composition().map(|_| ()), Err(ELEMENT_NOT_AVAILABLE));
    assert_eq!(provider.get_conversion_target().map(|_| ()), Err(ELEMENT_NOT_AVAILABLE));
    assert_eq!(retained_range.get_text(-1).map(|_| ()), Err(ELEMENT_NOT_AVAILABLE));
    assert!(model.is_composing());
}

#[test]
fn native_text_edit_events_marshal_unicode_deltas_and_empty_finalization_on_ui_thread() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let recording = RecordingScope::new();
    let editor = RichEditBox::new();
    let node = native_node(&editor);
    let identity = node.cast::<IUnknown>().unwrap().as_ptr() as usize;
    let thread = std::thread::current().id();
    editor.set_preedit_text(Some("a😀\0z"), Some(1)).unwrap();
    editor.set_preedit_text(Some("a🦀\0z"), Some(1)).unwrap();
    editor.set_preedit_text(Some("a🦀\0z"), Some(3)).unwrap();
    editor.commit_composition("完成\0😀").unwrap();
    editor.select(0, editor.document().text_length());
    editor.set_preedit_text(Some("cancel"), None).unwrap();
    editor.cancel_composition().unwrap();
    let expected = [
        (TextEditChangeType::Composition, "a😀\0z"),
        (TextEditChangeType::Composition, "🦀"),
        (TextEditChangeType::CompositionFinalized, "完成\0😀"),
        (TextEditChangeType::Composition, "cancel"),
        (TextEditChangeType::CompositionFinalized, ""),
    ];
    let events = recording.0.text_edits.borrow();
    assert_eq!(events.len(), expected.len());
    for (event, (cause, value)) in events.iter().zip(expected) {
        assert_eq!(event.change_type, cause);
        assert_eq!(event.changed_data, vec![value.to_owned()]);
        assert_eq!(event.identity, identity);
        assert_eq!(event.thread, thread);
    }
    assert!(!recording.0.events.borrow().contains(&UiaEventId::TextEdit_ConversionTargetChanged.0));
}

#[test]
fn native_text_edit_payload_validation_preserves_embedded_nul_and_return_codes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let recording = RecordingScope::new();
    let editor = RichEditBox::new();
    let node = native_node(&editor);
    let data = vec!["候\0😀".to_owned(), String::new()];
    assert_eq!(UiaCoreProviderApi::uia_raise_text_edit_text_changed_event(Some(&node), TextEditChangeType::None, &data), 0);
    assert_eq!(UiaCoreProviderApi::uia_raise_text_edit_text_changed_event(Some(&node), TextEditChangeType(-1), &data), ferroui_microcom::E_INVALIDARG as i32);
    assert!(recording.0.text_edits.borrow().is_empty());
    recording.0.text_edit_result.set(ferroui_microcom::E_FAIL as i32);
    assert_eq!(UiaCoreProviderApi::uia_raise_text_edit_text_changed_event(Some(&node), TextEditChangeType::Composition, &data), ferroui_microcom::E_FAIL as i32);
    assert_eq!(recording.0.text_edits.borrow().len(), 1);
    // The native call has returned and its borrowed SAFEARRAY has been freed.
    // The recorder retains only copied Rust strings, including embedded NUL.
    assert_eq!(recording.0.text_edits.borrow()[0].changed_data, data);
}

#[test]
fn native_public_text_edit_event_lazily_creates_a_provider_without_a_text_pattern() {
    use ferroui_controls::automation::provider::TextEditChangeType as Cause;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let recording = RecordingScope::new();
    let button = Button::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&button);
    let before = AutomationNode::count();
    peer.raise_text_edit_text_changed_event(Cause::None, vec!["ignored".into()]);
    assert_eq!(AutomationNode::count(), before);
    assert!(recording.0.text_edits.borrow().is_empty());
    peer.raise_text_edit_text_changed_event(
        Cause::AutoCorrect,
        vec!["候\0😀".into(), String::new()],
    );
    let events = recording.0.text_edits.borrow();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].changed_data, ["候\0😀", ""]);
    assert_eq!(events[0].change_type, TextEditChangeType::AutoCorrect);
    assert_eq!(
        AutomationNode::peer_from_identity(events[0].identity),
        Some(peer.clone())
    );
    let native = AutomationNode::get_or_create(Some(&peer)).unwrap();
    assert_eq!(
        events[0].identity,
        native.cast::<IUnknown>().unwrap().as_ptr() as usize
    );
    assert!(native
        .get_pattern_provider(UiaPatternId::TextEdit.0)
        .unwrap()
        .is_none());
    assert_eq!(events[0].thread, std::thread::current().id());
}

#[test]
fn native_public_text_edit_events_source_has_one_hop_and_uses_the_target_identity() {
    use ferroui_controls::automation::provider::TextEditChangeType as Cause;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let recording = RecordingScope::new();
    let a = Button::new();
    let b = Button::new();
    let c = Button::new();
    let original = ControlAutomationPeer::create_peer_for_element(&a);
    let source = ControlAutomationPeer::create_peer_for_element(&b);
    let further = ControlAutomationPeer::create_peer_for_element(&c);
    original.set_events_source(Some(source.clone()));
    source.set_events_source(Some(further));
    original.raise_text_edit_text_changed_event(Cause::Composition, vec!["one hop".into()]);
    let events = recording.0.text_edits.borrow();
    assert_eq!(events.len(), 1);
    assert_eq!(
        AutomationNode::peer_from_identity(events[0].identity),
        Some(source)
    );
}

#[test]
fn native_rich_first_composition_creates_both_peer_and_provider() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let recording = RecordingScope::new();
    let editor = RichEditBox::new();
    assert!(ControlAutomationPeer::from_element(&editor).is_none());
    editor.set_preedit_text(Some("first"), None).unwrap();
    let peer = ControlAutomationPeer::from_element(&editor).unwrap();
    let first_identity = recording.0.text_edits.borrow()[0].identity;
    assert_eq!(AutomationNode::peer_from_identity(first_identity), Some(peer));
    editor.set_preedit_text(Some("second"), None).unwrap();
    editor.cancel_composition().unwrap();
    let events = recording.0.text_edits.borrow();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].changed_data, ["first"]);
    assert!(events.iter().all(|event| event.identity == first_identity));
    assert_eq!(events[2].change_type, TextEditChangeType::CompositionFinalized);
}

#[test]
fn native_rich_peer_first_composition_activates_text_edit_delivery_without_duplicates() {
    use ferroui_controls::automation::provider::ITextEditProvider as PortableTextEdit;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let recording = RecordingScope::new();
    let editor = RichEditBox::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    let provider = peer.get_provider::<dyn PortableTextEdit>().unwrap();
    assert!(provider.raises_text_edit_events_through_peer());
    let portable = Rc::new(RefCell::new(Vec::new()));
    let observed = portable.clone();
    let subscription = provider
        .text_edit_text_changed(Rc::new(move |event| {
            observed.borrow_mut().push(event.clone());
        }))
        .unwrap();
    editor.set_preedit_text(Some("候補"), None).unwrap();
    // The first event creates the COM node during event delivery. Its new
    // subscriptions must not forward the current or subsequent cause twice.
    let first_identity = recording.0.text_edits.borrow()[0].identity;
    assert_eq!(
        AutomationNode::peer_from_identity(first_identity),
        Some(peer)
    );
    editor.set_preedit_text(Some("候補文字"), None).unwrap();
    editor.commit_composition("確定").unwrap();
    assert_eq!(recording.0.text_edits.borrow().len(), 3);
    assert_eq!(portable.borrow().len(), 3);
    assert!(recording
        .0
        .text_edits
        .borrow()
        .iter()
        .all(|event| event.identity == first_identity));
    assert_eq!(
        recording.0.text_edits.borrow()[2].change_type,
        TextEditChangeType::CompositionFinalized
    );
    subscription.dispose();
}

#[test]
fn native_rich_peer_event_route_rewires_documents_and_does_not_retain_the_owner() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let recording = RecordingScope::new();
    let (detached_model, retained_model, weak_peer) = {
        let editor = RichEditBox::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&editor);
        let old = editor.editor();
        editor.set_document(RichTextDocument::from_text("new document"));
        old.set_preedit("detached", None).unwrap();
        assert!(recording.0.text_edits.borrow().is_empty());
        editor.set_preedit_text(Some("live"), None).unwrap();
        assert_eq!(recording.0.text_edits.borrow().len(), 1);
        (old, editor.editor(), peer.downgrade())
    };
    assert!(weak_peer.upgrade().is_none());
    let count = recording.0.text_edits.borrow().len();
    detached_model.set_preedit("still detached", None).unwrap();
    retained_model.set_preedit("owner gone", None).unwrap();
    assert_eq!(recording.0.text_edits.borrow().len(), count);
}

#[test]
fn native_public_text_edit_can_reactivate_a_released_node_without_old_subscriptions() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let recording = RecordingScope::new();
    let editor = RichEditBox::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    editor.set_preedit_text(Some("first"), None).unwrap();
    let retired = recording.0.text_edits.borrow()[0].identity;
    let retained_node = AutomationNode::get_or_create(Some(&peer)).unwrap();
    AutomationNode::release(&peer);
    editor.set_preedit_text(Some("second"), None).unwrap();
    editor.cancel_composition().unwrap();
    let events = recording.0.text_edits.borrow();
    assert_eq!(events.len(), 3);
    assert_ne!(events[1].identity, retired);
    assert_eq!(events[1].identity, events[2].identity);
    assert_eq!(
        retained_node
            .get_property_value(UiaPropertyId::Name.0)
            .map(|_| ()),
        Err(ELEMENT_NOT_AVAILABLE)
    );
}

#[test]
fn native_public_text_edit_reentrant_release_and_source_change_preserve_each_event() {
    use ferroui_controls::automation::provider::TextEditChangeType as Cause;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let recording = RecordingScope::new();
    let owner = Button::new();
    let b = Button::new();
    let c = Button::new();
    let original = ControlAutomationPeer::create_peer_for_element(&owner);
    let source = ControlAutomationPeer::create_peer_for_element(&b);
    let next = ControlAutomationPeer::create_peer_for_element(&c);
    original.set_events_source(Some(source.clone()));
    let expected = AutomationNode::get_or_create(Some(&source)).unwrap();
    let old_identity = expected.cast::<IUnknown>().unwrap().as_ptr() as usize;
    let entered = Rc::new(Cell::new(false));
    let original_weak = original.downgrade();
    let source_weak = source.downgrade();
    let next_weak = next.downgrade();
    *recording.0.text_edit_callback.borrow_mut() = Some(Rc::new(move || {
        if !entered.replace(true) {
            AutomationNode::release(&source_weak.upgrade().unwrap());
            let peer = original_weak.upgrade().unwrap();
            peer.set_events_source(next_weak.upgrade());
            peer.raise_text_edit_text_changed_event(
                Cause::CompositionFinalized,
                vec!["nested".into()],
            );
        }
    }));
    original.raise_text_edit_text_changed_event(Cause::Composition, vec!["outer".into()]);
    let events = recording.0.text_edits.borrow();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].identity, old_identity);
    assert_eq!(events[0].changed_data, ["outer"]);
    assert_eq!(
        AutomationNode::peer_from_identity(events[1].identity),
        Some(next)
    );
    assert_eq!(events[1].changed_data, ["nested"]);
}
