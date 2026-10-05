//! The text selection canvas test of the reference test suite, followed by
//! additional tests of the touch selection handles (marked as additional;
//! their expectations follow the reference implementation).

use super::{SelectionHandleType, TemplatedControlImpl, TextSelectionHandle, TextSelectionHandleCanvas, Thumb, VisualLayerManager};
use crate::mouse_test_helper::MouseTestHelper;
use crate::platform::ITopLevelImpl;
use crate::presenters::{ContentPresenter, TextPresenter};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::{MockImplKind, MockWindowImpl, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::text_box_tests_input::TouchTestHelper;
use crate::{
    Border, ContentControl, ContentControlImpl, Control, ControlImpl, ScrollViewer, StackPanel,
    TextBox, TopLevel, TopLevelImpl,
};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{InputElement, InputElementImpl, Key, KeyEventArgs, VectorEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, LayoutableImpl, Orientation};
use ferroui_base::media::text_formatting::testing::{line_height, TextTestScope};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroLocator, FerroObjectImpl, LocatorExtensions, Point, Ref,
    StyledElementImpl, Vector, VisualImpl,
};
use std::rc::Rc;

/// The top-level of the tests.
#[repr(C)]
struct TestTopLevel {
    base: TopLevel,
}

ferro_class!(TestTopLevel: TopLevel);
ferro_impl_classes!(
    TestTopLevel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl
);

impl TestTopLevel {
    fn new(platform_impl: Rc<dyn ITopLevelImpl>) -> Ref<Self> {
        instantiate(Self { base: TopLevel::construct(platform_impl) })
    }
}

/// The test application and the text services it runs with.
struct TestApplication {
    // Dropped in this order: the application ends before the text services.
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

/// The services of the reference test: text services, a keyboard device, a
/// keyboard navigation handler, an input manager and an asset loader.
fn start() -> TestApplication {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(TestServices::real_focus().with_render_interface(render_interface));
    TestApplication { _app: app, _text: text }
}

fn create_top_level() -> Ref<TestTopLevel> {
    let top_level = TestTopLevel::new(MockWindowImpl::bare(MockImplKind::TopLevel));
    top_level.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let content = ContentControl::content_property().as_property();
        presenter.bind_binding(content, &TemplateBinding::new(content));
        presenter.register_in_name_scope(&**scope).upcast()
    })));
    top_level
}

/// A text box with the template of the reference text box tests, which
/// binds the text and (both ways) the caret index of the presenter, or,
/// when `themed`, with a template that binds the presenter as the reference
/// themes do: the selection too, and the caret index one way.
fn create_text_box(text: &str, themed: bool) -> Ref<TextBox> {
    let text_box = TextBox::new();
    text_box.set_template(if themed {
        crate::text_box_tests::create_themed_template()
    } else {
        crate::text_box_tests::create_template()
    });
    text_box.set_text(Some(text));
    text_box.set_width(150.0);
    text_box
}

/// The tree of the reference test: a top level whose content is a visual
/// layer manager with the text selector layer, holding a border, a scroll
/// viewer and a stack of text boxes. `first` is the first of them.
struct Tree {
    top_level: Ref<TestTopLevel>,
    scroll_viewer: Ref<ScrollViewer>,
    first: Ref<TextBox>,
}

fn create_tree(first_text: &str, themed: bool) -> Tree {
    let root_border = Border::new();
    root_border.set_width(200.0);
    root_border.set_height(600.0);

    let visual_layer_manager = VisualLayerManager::new();
    visual_layer_manager.set_child(&root_border);
    visual_layer_manager.set_enable_text_selector_layer(true);

    let top_level = create_top_level();
    top_level.set_content(Some(Control::boxed(visual_layer_manager)));

    let first = create_text_box(first_text, themed);

    let panel = StackPanel::new();
    panel.set_orientation(Orientation::Vertical);
    panel.set_spacing(20.0);
    panel.children().add(first.clone());
    for _ in 0..23 {
        panel.children().add(create_text_box("Test", themed));
    }

    let scroll_viewer = ScrollViewer::new();
    scroll_viewer
        .set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(crate::scroll_viewer_tests::create_template)));
    scroll_viewer.set_content(Some(Control::boxed(panel)));

    root_border.set_child(&scroll_viewer);

    top_level.layout_manager().execute_initial_layout_pass();

    Tree { top_level, scroll_viewer, first }
}

impl Tree {
    fn layout(&self) {
        self.top_level.layout_manager().execute_layout_pass();
    }

    fn presenter(&self) -> Ref<TextPresenter> {
        self.first.find_descendant_of_type::<TextPresenter>(false).expect("the text box has a presenter")
    }

    fn canvas(&self) -> Ref<TextSelectionHandleCanvas> {
        self.presenter().text_selection_handle_canvas().expect("the presenter has a handle canvas")
    }
}

#[test]
fn text_selection_handle_moves_with_scrolling() {
    let _app = start();
    let touch_helper = TouchTestHelper::new();
    let tree = create_tree("Test", false);

    // A tap: down and up.
    touch_helper.down(&tree.first, Point::default());
    touch_helper.up(&tree.first, Point::default());

    tree.layout();

    let presenter = tree.presenter();
    let canvas = presenter.text_selection_handle_canvas();

    assert!(canvas.is_some());
    let canvas = canvas.unwrap();

    let handle = canvas.children().snapshot().first().and_then(|child| child.cast::<TextSelectionHandle>());

    assert!(handle.is_some());
    let handle = handle.unwrap();

    // The handle is at the bottom of the caret: the reference test expects
    // 14.5 with the line height of its font; the line of the test font here
    // is 13.2 high.
    let bottom = line_height(12.0).ceil() - 0.5;
    assert_eq!(13.5, bottom);
    assert_eq!(Point::new(25.5, bottom), handle.get_top_left());

    tree.scroll_viewer.set_offset(Vector::new(0.0, 50.0));

    tree.layout();

    assert_eq!(Point::new(25.5, bottom - 50.0), handle.get_top_left());
}

// --- additional tests ---

fn raise_key_down(target: &InputElement, key: Key) {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(InputElement::key_down_event()));
    target.raise_event(&e);
}

/// Selects the word at `position` of the presenter with a double tap of a
/// touch contact on the presenter.
fn touch_double_tap(tree: &Tree, position: Point) -> TouchTestHelper {
    let touch = TouchTestHelper::new();
    let presenter = tree.presenter();
    touch.down(&presenter, position);
    touch.up(&presenter, position);
    touch.down(&presenter, position);
    touch.up(&presenter, position);
    tree.layout();
    touch
}

/// The x coordinate, in the top level, of the leading edge of the character
/// at `index` of the presenter.
fn character_x(tree: &Tree, index: i32) -> f64 {
    let presenter = tree.presenter();
    let rect = presenter.text_layout().hit_test_text_position(index);
    presenter.translate_point(rect.top_left(), &tree.top_level).expect("the presenter is in the top level").x
}

fn raise_drag(handle: &TextSelectionHandle, event: &'static ferroui_base::interactivity::RoutedEvent<VectorEventArgs>, vector: Vector) {
    let mut e = VectorEventArgs::new();
    e.set_routed_event(Some(event));
    e.vector = vector;
    handle.raise_event(&e);
}

/// Drags `handle` so that its indicator ends just after the leading edge of
/// the character at `index`.
fn drag_to_character(tree: &Tree, handle: &TextSelectionHandle, index: i32) {
    let delta = Vector::new(character_x(tree, index) + 1.0 - handle.indicator_position().x, 0.0);
    raise_drag(handle, Thumb::drag_started_event(), Vector::default());
    raise_drag(handle, Thumb::drag_delta_event(), delta);
    raise_drag(handle, Thumb::drag_completed_event(), delta);
}

/// Additional: a touch selection shows the handles; a key press on the
/// presenter hides them and leaves the touch mode.
#[test]
fn handles_are_shown_on_touch_selection_and_hidden_on_key_input() {
    let _app = start();
    let tree = create_tree("12 12345678", true);

    let _touch = touch_double_tap(&tree, Point::new(50.0, 0.0));

    assert_eq!((3, 11), (tree.first.selection_start(), tree.first.selection_end()));
    let canvas = tree.canvas();
    assert!(canvas.is_in_touch_mode());
    assert!(canvas.show_handles());
    assert!(canvas.is_visible());
    assert!(canvas.visual_parent().is_some_and(|parent| parent.is::<super::TextSelectorLayer>()));

    raise_key_down(&tree.presenter(), Key::Left);

    assert!(!canvas.is_in_touch_mode());
    assert!(!canvas.show_handles());
    assert!(!canvas.is_visible());
    assert!(canvas.handles().iter().all(|handle| !handle.is_visible()));
}

/// Additional: a press of the left mouse button hides the handles and
/// leaves the touch mode.
#[test]
fn handles_are_hidden_on_mouse_input() {
    let _app = start();
    let tree = create_tree("12 12345678", true);

    let _touch = touch_double_tap(&tree, Point::new(50.0, 0.0));

    let canvas = tree.canvas();
    assert!(canvas.show_handles());

    let mouse = MouseTestHelper::new();
    mouse.down(&tree.presenter());

    assert!(!canvas.is_in_touch_mode());
    assert!(!canvas.show_handles());
    assert!(!canvas.is_visible());
    assert!(canvas.handles().iter().all(|handle| !handle.is_visible()));
}

/// Additional: the handles keep their roles after a touch selection: the
/// first child is the caret handle, then the start and the end handle, at
/// the two ends of the selection.
#[test]
fn selection_handles_are_placed_at_the_ends_of_the_selection() {
    let _app = start();
    let tree = create_tree("12 12345678", true);

    let _touch = touch_double_tap(&tree, Point::new(50.0, 0.0));

    let [caret, start, end] = tree.canvas().handles();
    assert_eq!(SelectionHandleType::Caret, caret.selection_handle_type());
    assert_eq!(SelectionHandleType::Start, start.selection_handle_type());
    assert_eq!(SelectionHandleType::End, end.selection_handle_type());

    assert_eq!(character_x(&tree, 3), start.indicator_position().x);
    assert_eq!(character_x(&tree, 11), end.indicator_position().x);
    assert_eq!(start.indicator_position().y, end.indicator_position().y);
}

/// Additional: dragging the end handle moves the end of the selection and
/// the caret of the presenter.
#[test]
fn dragging_the_end_handle_moves_the_selection_end() {
    let _app = start();
    let tree = create_tree("12 12345678", true);

    let _touch = touch_double_tap(&tree, Point::new(50.0, 0.0));
    let [_, start, end] = tree.canvas().handles();

    drag_to_character(&tree, &end, 6);

    assert_eq!((3, 6), (tree.first.selection_start(), tree.first.selection_end()));
    assert_eq!(6, tree.presenter().caret_index());
    assert!(!end.is_dragging());
    assert_eq!(SelectionHandleType::Start, start.selection_handle_type());
    assert_eq!(SelectionHandleType::End, end.selection_handle_type());
}

/// Additional: dragging the end handle before the start of the selection
/// swaps the bounds of the selection (on the platforms that wrap around).
#[test]
fn dragging_the_end_handle_over_the_start_swaps_the_selection_bounds() {
    let _app = start();
    let tree = create_tree("12 12345678", true);

    let _touch = touch_double_tap(&tree, Point::new(50.0, 0.0));
    let [_, start, end] = tree.canvas().handles();

    // While the drag is in progress the other handle takes the other role.
    let delta = Vector::new(character_x(&tree, 1) + 1.0 - end.indicator_position().x, 0.0);
    raise_drag(&end, Thumb::drag_started_event(), Vector::default());
    raise_drag(&end, Thumb::drag_delta_event(), delta);

    assert_eq!((1, 3), (tree.first.selection_start(), tree.first.selection_end()));
    assert!(end.is_dragging());
    assert_eq!(SelectionHandleType::End, start.selection_handle_type());

    // When it completes the handles take their roles back.
    raise_drag(&end, Thumb::drag_completed_event(), delta);

    assert_eq!((1, 3), (tree.first.selection_start(), tree.first.selection_end()));
    assert_eq!(SelectionHandleType::Start, start.selection_handle_type());
    assert_eq!(SelectionHandleType::End, end.selection_handle_type());
}

/// Additional: dragging the caret handle moves the caret and collapses the
/// selection to it.
#[test]
fn dragging_the_caret_handle_moves_the_caret() {
    let _app = start();
    let touch = TouchTestHelper::new();
    let tree = create_tree("12 12345678", true);

    let presenter = tree.presenter();
    touch.down(&presenter, Point::default());
    touch.up(&presenter, Point::default());
    tree.layout();

    let [caret, ..] = tree.canvas().handles();
    assert_eq!(0, tree.first.caret_index());

    drag_to_character(&tree, &caret, 5);

    assert_eq!(5, tree.first.caret_index());
    assert_eq!((5, 5), (tree.first.selection_start(), tree.first.selection_end()));
    assert_eq!(character_x(&tree, 5), caret.indicator_position().x - 0.5);
}

/// Additional: the selection handles follow the text when it is scrolled.
#[test]
fn selection_handles_move_with_scrolling() {
    let _app = start();
    let tree = create_tree("12 12345678", true);

    let _touch = touch_double_tap(&tree, Point::new(50.0, 0.0));
    let [_, start, end] = tree.canvas().handles();
    let (start_position, end_position) = (start.get_top_left(), end.get_top_left());

    tree.scroll_viewer.set_offset(Vector::new(0.0, 50.0));
    tree.layout();

    assert_eq!(start_position.with_y(start_position.y - 50.0), start.get_top_left());
    assert_eq!(end_position.with_y(end_position.y - 50.0), end.get_top_left());
}
