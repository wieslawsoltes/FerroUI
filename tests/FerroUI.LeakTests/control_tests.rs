//! Port of the control tests of the reference leak tests: a control (and
//! what belongs to it) is freed after it left the tree of a window.
//!
//! Every scenario runs in a block that returns the weak handles
//! ([`Tracked`]) of what it created, as the local function of a reference
//! test returns weak references; the handles of the scenario are dropped
//! with the block. The window of a scenario is shown and not closed, as in
//! the reference: its platform implementation keeps it alive, so what is
//! asserted is that the window does not keep what was taken out of it.
//!
//! The reference clears the calls its mock of the platform implementation
//! recorded before it collects, because the mock holds the arguments of the
//! calls; the mock of these tests records calls without their objects, so
//! nothing is cleared.

use crate::leak::{assert_all_freed, Tracked};
use crate::services::{collect_garbage, start_control_test};
use ferroui_base::collections::FerroList;
use ferroui_base::controls::{NameScope, NameScopeExtensions, NameScopeRef};
use ferroui_base::data::core::Maybe;
use ferroui_base::data::model::Model;
use ferroui_base::data::{ReflectionBinding, TemplateBinding};
use ferroui_base::input::{InputElement, Key, KeyGesture, KeyModifiers};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::{EllipseGeometry, ITransform, Points, RotateTransform};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_model, AnyValue, BoxedValue, FerroLocator, PixelRect, PixelSize, Point, Rect, Ref, Thickness, Visual,
};
use ferroui_controls::platform::{IScreenImpl, ITopLevelImpl, IWindowImpl, IWindowingPlatform};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::shapes::{Path, Polyline};
use ferroui_controls::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, FuncTreeDataTemplate, IControlTemplate,
    IDataTemplate,
};
use ferroui_controls::testing::{mock_screen, MockImplKind, MockScreenImpl, MockWindowImpl, MockWindowingPlatform, NullRenderer};
use ferroui_controls::{
    Button, Canvas, ContentControl, ContextMenu, Control, Decorator, Dock, DockPanel, Flyout, HotKeyManager, ItemsControl,
    ItemsSource, LayoutTransformControl, ListBox, ListBoxItem, Menu, MenuItem, ScrollViewer, SizeToContent, Slider,
    TabControl, TabItem, TextBlock, TextBox, ToolTip, TreeView, Window,
};
use std::cell::RefCell;
use std::rc::Rc;

// --- the data of the tests --------------------------------------------------

struct Node {
    name: RefCell<Option<String>>,
    children: Vec<Rc<Node>>,
}

ferro_model!(Node, |b| b.property::<Maybe<String>>("Name", |o| o.name(), |o, v| *o.name.borrow_mut() = v));

impl Node {
    fn new(name: Option<&str>, children: Vec<Rc<Node>>) -> Rc<Node> {
        Model::new_model(Node { name: RefCell::new(name.map(str::to_string)), children })
    }

    fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }
}

// --- helpers ----------------------------------------------------------------

/// The child of the content presenter of a window: the content of the
/// window, once a layout pass applied the templates.
fn presenter_child(window: &Window) -> Option<Ref<Control>> {
    window.presenter().expect("the template of the window has a content presenter").child()
}

/// The text an untyped value holds.
fn text_of(value: Option<BoxedValue>) -> Option<String> {
    let value = value?;
    let value: &dyn AnyValue = &*value;
    value.downcast_ref::<String>().cloned().or_else(|| value.downcast_ref::<Option<String>>().cloned().flatten())
}

fn boxed_text(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()))
}

/// A text block whose text is the content of its templated parent.
fn text_block_bound_to_the_content() -> Ref<TextBlock> {
    let text_block = TextBlock::new();
    text_block.bind_binding(
        TextBlock::text_property().as_property(),
        &*TemplateBinding::new(ContentControl::content_property().as_property()),
    );
    text_block
}

fn create_window_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &*TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

// --- tests ------------------------------------------------------------------

#[test]
fn canvas_is_freed() {
    let _app = start_control_test();

    let canvas = {
        let canvas = Canvas::new();
        let window = Window::new();
        window.set_content(Some(Control::boxed(&canvas)));

        window.show();

        // Do a layout and make sure that the canvas gets added to the visual tree.
        window.layout_manager().execute_initial_layout_pass();
        assert!(presenter_child(&window).is_some_and(|child| child.is::<Canvas>()));

        // Clear the content and ensure the canvas is removed.
        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the canvas", &canvas)
    };

    collect_garbage();

    canvas.assert_freed();
}

#[test]
fn named_canvas_is_freed() {
    let _app = start_control_test();

    let canvas = {
        let scope = NameScopeRef::new(NameScope::new());
        let canvas = Canvas::new();
        canvas.set_name(Some("foo".to_string()));
        let window = Window::new();
        window.set_content(Some(Control::boxed(canvas.clone().register_in_name_scope(&*scope))));
        NameScope::set_name_scope(&window, Some(scope.clone()));

        window.show();

        // Do a layout and make sure that the canvas gets added to the visual tree.
        window.layout_manager().execute_initial_layout_pass();
        assert!(NameScopeExtensions::find::<Canvas>(&window, "foo").is_some());
        assert!(presenter_child(&window).is_some_and(|child| child.is::<Canvas>()));

        // Clear the content and ensure the canvas is removed.
        window.set_content(None);
        NameScope::set_name_scope(&window, None);

        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the canvas", &canvas)
    };

    collect_garbage();

    canvas.assert_freed();
}

#[test]
fn scroll_viewer_with_content_is_freed() {
    let _app = start_control_test();

    let canvas = {
        let canvas = Canvas::new();
        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_content(Some(Control::boxed(&canvas)));
        let window = Window::new();
        window.set_content(Some(Control::boxed(&scroll_viewer)));

        window.show();

        // Do a layout and make sure that the scroll viewer gets added to the
        // visual tree and its template applied.
        window.layout_manager().execute_initial_layout_pass();
        let child = presenter_child(&window).and_then(|child| child.cast::<ScrollViewer>());
        let child = child.expect("the content of the window is the scroll viewer");
        let presenter = child.presenter().expect("the template of the scroll viewer has a content presenter");
        assert!(presenter.child().is_some_and(|child| child.is::<Canvas>()));

        // Clear the content and ensure the scroll viewer is removed.
        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the canvas", &canvas)
    };

    collect_garbage();

    canvas.assert_freed();
}

#[test]
fn text_box_is_freed() {
    let _app = start_control_test();

    let text_box = {
        let text_box = TextBox::new();
        let window = Window::new();
        window.set_content(Some(Control::boxed(&text_box)));

        window.show();

        // Do a layout and make sure that the text box gets added to the
        // visual tree and its template applied.
        window.layout_manager().execute_initial_layout_pass();
        let child = presenter_child(&window).expect("the window has its content");
        assert!(child.is::<TextBox>());
        assert!(!child.get_visual_children().is_empty());
        drop(child);

        // Clear the content and ensure the text box is removed.
        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the text box", &text_box)
    };

    collect_garbage();

    text_box.assert_freed();
}

#[test]
fn text_box_with_xaml_binding_is_freed() {
    let _app = start_control_test();

    let tracked = {
        let node = Node::new(Some("foo"), Vec::new());
        let window = Window::new();
        window.set_data_context(Some(node.clone() as BoxedValue));
        window.set_content(Some(Control::boxed(TextBox::new())));

        let binding = ReflectionBinding::new("Name");

        let text_box = window.content().and_then(|content| Control::from_boxed(&content)).and_then(|c| c.cast::<TextBox>());
        let text_box = text_box.expect("the content of the window is the text box");
        text_box.bind_binding(TextBox::text_property().as_property(), &*binding);

        window.show();

        // Do a layout and make sure that the text box gets added to the
        // visual tree and its text set.
        window.layout_manager().execute_initial_layout_pass();
        let child = presenter_child(&window).and_then(|child| child.cast::<TextBox>());
        assert_eq!(Some("foo".to_string()), child.expect("the content of the window is the text box").text());

        // Clear the content and the data context and ensure the text box is removed.
        window.set_content(None);
        window.set_data_context(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        [Tracked::shared("the data context", &node), Tracked::object("the text box", &text_box)]
    };

    collect_garbage();

    assert_all_freed(&tracked);
}

// Finding L001 (README.md), open: nine listeners of the classes of the text box are left after
// the text box is removed from the window, where upstream asserts none
// (`Assert.Equal(0, textBox.Classes.ListenerCount)`). Not investigated to its cause.
#[test]
#[ignore = "finding L001: the classes of a text box keep 9 listeners after it leaves the tree"]
fn text_box_class_listeners_are_freed() {
    let _app = start_control_test();

    let text_box = TextBox::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&text_box)));

    window.show();

    // Do a layout and make sure that the text box gets added to the visual
    // tree and its template applied.
    window.layout_manager().execute_initial_layout_pass();
    assert!(presenter_child(&window).is_some_and(|child| child == text_box));

    // Get the border from the template of the text box.
    let _border = text_box.get_template_descendants().into_iter().find(|x| {
        x.cast::<Control>().is_some_and(|control| control.name().as_deref() == Some("border"))
    });

    // The text box has subscriptions to its classes from the default theme.
    assert_ne!(0, text_box.classes().listener_count());

    // Clear the content and ensure the text box is removed.
    window.set_content(None);
    window.layout_manager().execute_layout_pass();
    assert!(presenter_child(&window).is_none());

    // Check that the text box has no subscriptions to its classes.
    assert_eq!(0, text_box.classes().listener_count());
}

#[test]
fn tree_view_is_freed() {
    let _app = start_control_test();

    let tree_view = {
        let nodes = [Node::new(None, vec![Node::new(None, Vec::new())])];

        let target = TreeView::new();
        let template: Rc<dyn IDataTemplate> = FuncTreeDataTemplate::for_type::<Node>(
            |x, _| {
                let text_block = TextBlock::new();
                text_block.set_text(x.name().as_deref());
                Some(text_block.upcast())
            },
            |x| Rc::new(Some(ItemsSource::from_items(x.children.iter().map(|child| Some(child.clone() as BoxedValue))))),
        );
        target.data_templates().add(template);
        target.set_items_source(Some(ItemsSource::from_items(nodes.iter().map(|node| Some(node.clone() as BoxedValue)))));

        let window = Window::new();
        window.set_content(Some(Control::boxed(&target)));

        window.show();

        // Do a layout and make sure that the tree view items get realized.
        window.layout_manager().execute_initial_layout_pass();
        assert_eq!(1, target.get_realized_containers().len());

        // Clear the content and ensure the tree view is removed.
        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the tree view", &target)
    };

    collect_garbage();

    tree_view.assert_freed();
}

#[test]
fn slider_is_freed() {
    let _app = start_control_test();

    let slider = {
        let slider = Slider::new();
        let window = Window::new();
        window.set_content(Some(Control::boxed(&slider)));

        window.show();

        // Do a layout and make sure that the slider gets added to the visual tree.
        window.layout_manager().execute_initial_layout_pass();
        assert!(presenter_child(&window).is_some_and(|child| child.is::<Slider>()));

        // Clear the content and ensure the slider is removed.
        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the slider", &slider)
    };

    collect_garbage();

    slider.assert_freed();
}

#[test]
fn tab_item_is_freed() {
    let _app = start_control_test();

    let tab_item = {
        let tab_item = TabItem::new();
        let tab_control = TabControl::new();
        tab_control.set_items_source(Some(ItemsSource::from_items([Some(Control::boxed(&tab_item))])));
        let window = Window::new();
        window.set_content(Some(Control::boxed(tab_control)));

        window.show();

        // Do a layout and make sure that the tab control and the tab item get
        // added to the visual tree.
        window.layout_manager().execute_initial_layout_pass();
        let tab_control = presenter_child(&window).and_then(|child| child.cast::<TabControl>());
        let tab_control = tab_control.expect("the content of the window is the tab control");
        let panel = tab_control.presenter().and_then(|presenter| presenter.panel());
        let panel = panel.expect("the items presenter of the tab control has its panel");
        assert!(panel.children().get(0).is::<TabItem>());

        // Clear the items and ensure the tab item is removed.
        tab_control.set_items_source(None);
        window.layout_manager().execute_layout_pass();
        assert_eq!(0, panel.children().count());

        Tracked::object("the tab item", &tab_item)
    };

    collect_garbage();

    tab_item.assert_freed();
}

/// The reference asks the compositing renderer of the window, after the
/// window closed, whether it is disposed. The top-levels of the test
/// services render through the null renderer (the dummy compositor of the
/// reference), which is asked instead; it is taken from the window before
/// the window closes and lets go of its presentation source.
#[test]
fn renderer_is_disposed() {
    let _app = start_control_test();

    let screen1 = mock_screen(
        1.75,
        PixelRect::from_size(PixelSize::new(1920, 1080)),
        PixelRect::from_size(PixelSize::new(1920, 966)),
        true,
    );
    let screens: Rc<dyn IScreenImpl> = MockScreenImpl::new(vec![screen1]);

    // A window implementation without behaviour, except that disposing it
    // reports that it closed.
    let window_impl = MockWindowImpl::bare(MockImplKind::Window);
    window_impl.setup_feature::<dyn IScreenImpl>(screens);
    window_impl.setup_dispose(|window_impl| {
        if let Some(closed) = ITopLevelImpl::closed(window_impl) {
            closed();
        }
    });

    let platform: Rc<dyn IWindowingPlatform> =
        MockWindowingPlatform::with_window_impl(move || window_impl.clone() as Rc<dyn IWindowImpl>);
    FerroLocator::current_mutable().bind::<dyn IWindowingPlatform>().to_constant(platform);

    let window = Window::new();
    window.set_content(Some(Control::boxed(Button::new())));
    window.show();
    let renderer = window.renderer();
    window.close();

    let renderer = renderer.as_any().downcast_ref::<NullRenderer>().expect("the renderer of the tests is the null renderer");
    assert!(renderer.is_disposed());
}

#[test]
fn control_with_style_render_transform_is_freed() {
    // Issue #3545 of the reference.
    let _app = start_control_test();

    let canvas = {
        let window = Window::new();
        let transform: Rc<dyn ITransform> = RotateTransform::with_angle(45.0).into();
        window.styles().add(Style::with_setters(
            Selectors::of_type::<Canvas>(),
            [Setter::new(Visual::render_transform_property(), Some(transform))],
        ));
        window.set_content(Some(Control::boxed(Canvas::new())));

        window.show();

        // Do a layout and make sure that the canvas gets added to the visual
        // tree with its render transform.
        window.layout_manager().execute_initial_layout_pass();
        let canvas = presenter_child(&window).and_then(|child| child.cast::<Canvas>());
        let canvas = canvas.expect("the content of the window is the canvas");
        let render_transform = canvas.render_transform().expect("the style set the render transform");
        assert!(render_transform.as_object().is_some_and(|transform| transform.to_ref().is::<RotateTransform>()));
        drop(render_transform);

        // Clear the content and ensure the canvas is removed.
        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the canvas", &canvas)
    };

    collect_garbage();

    canvas.assert_freed();
}

#[test]
fn attached_context_menu_is_freed() {
    let _app = start_control_test();

    fn attach_show_and_detach_context_menu(control: &Ref<Control>) -> [Tracked; 3] {
        let menu_item1 = MenuItem::new();
        menu_item1.set_header(boxed_text("Foo"));
        let menu_item2 = MenuItem::new();
        menu_item2.set_header(boxed_text("Foo"));
        let context_menu = ContextMenu::new();
        context_menu.items().add(Some(Control::boxed(&menu_item1)));
        context_menu.items().add(Some(Control::boxed(&menu_item2)));

        control.set_context_menu(&context_menu);
        context_menu.open_at(Some(control));
        context_menu.close();
        control.set_context_menu(None);

        [
            Tracked::object("the first menu item", &menu_item1),
            Tracked::object("the second menu item", &menu_item2),
            Tracked::object("the context menu", &context_menu),
        ]
    }

    let window = Window::new();
    window.set_focusable(true);
    window.show();

    assert_eq!(Some(window.clone().upcast::<InputElement>()), window.focus_manager().get_focused_element());

    let tracked = attach_show_and_detach_context_menu(&window.clone().upcast());

    collect_garbage();

    assert_all_freed(&tracked);
}

#[test]
fn attached_control_from_context_menu_is_freed() {
    let _app = start_control_test();

    let context_menu = ContextMenu::new();

    let text_block = {
        let text_block = TextBlock::new();
        text_block.set_context_menu(&context_menu);
        let window = Window::new();
        window.set_content(Some(Control::boxed(&text_block)));

        window.show();

        // Do a layout and make sure that the text block gets added to the visual tree.
        window.layout_manager().execute_initial_layout_pass();
        assert!(presenter_child(&window).is_some_and(|child| child.is::<TextBlock>()));

        // Clear the content and ensure the text block is removed.
        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the text block", &text_block)
    };

    collect_garbage();

    text_block.assert_freed();
    drop(context_menu);
}

#[test]
fn standalone_context_menu_is_freed() {
    let _app = start_control_test();

    fn build_and_show_context_menu(control: &Ref<Control>) -> [Tracked; 3] {
        let menu_item1 = MenuItem::new();
        menu_item1.set_header(boxed_text("Foo"));
        let menu_item2 = MenuItem::new();
        menu_item2.set_header(boxed_text("Foo"));
        let context_menu = ContextMenu::new();
        context_menu.items().add(Some(Control::boxed(&menu_item1)));
        context_menu.items().add(Some(Control::boxed(&menu_item2)));

        context_menu.open_at(Some(control));
        context_menu.close();

        [
            Tracked::object("the first menu item", &menu_item1),
            Tracked::object("the second menu item", &menu_item2),
            Tracked::object("the context menu", &context_menu),
        ]
    }

    let window = Window::new();
    window.set_focusable(true);
    window.show();

    assert_eq!(Some(window.clone().upcast::<InputElement>()), window.focus_manager().get_focused_element());

    let first = build_and_show_context_menu(&window.clone().upcast());
    let second = build_and_show_context_menu(&window.clone().upcast());

    collect_garbage();

    let tracked: Vec<Tracked> = first.into_iter().chain(second).collect();
    assert_all_freed(&tracked);
}

#[test]
fn path_is_freed() {
    let _app = start_control_test();

    let geometry = EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0));

    let path = {
        let path = Path::new();
        path.set_data(&geometry);
        let window = Window::new();
        window.set_content(Some(Control::boxed(&path)));

        window.show();

        window.layout_manager().execute_initial_layout_pass();
        assert!(presenter_child(&window).is_some_and(|child| child.is::<Path>()));

        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the path", &path)
    };

    collect_garbage();

    path.assert_freed();
    drop(geometry);
}

#[test]
fn polyline_with_observable_collection_points_binding_is_freed() {
    let _app = start_control_test();

    let observable_collection = Points::from_items([Point::new(0.0, 0.0)]);

    let polyline = {
        let polyline = Polyline::new();
        polyline.set_points(Some(observable_collection.clone()));
        let window = Window::new();
        window.set_content(Some(Control::boxed(&polyline)));

        window.show();

        window.layout_manager().execute_initial_layout_pass();
        assert!(presenter_child(&window).is_some_and(|child| child.is::<Polyline>()));

        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the polyline", &polyline)
    };

    collect_garbage();

    polyline.assert_freed();

    // The collection is kept alive: a resource that outlives the control.
    drop(observable_collection);
}

#[test]
fn element_name_binding_in_data_template_is_freed() {
    let _app = start_control_test();

    // The height of the content of a list box item, with its padding.
    const LIST_BOX_ITEM_HEIGHT: f64 = 12.0;
    const TEXT_BOX_HEIGHT: f64 = 25.0;

    let items = Rc::new(FerroList::from_items(0..10));
    let ns = NameScopeRef::new(NameScope::new());

    let weak_canvases: Rc<RefCell<Vec<Tracked>>> = Rc::new(RefCell::new(Vec::new()));

    let window = Window::new();
    NameScope::set_name_scope(&window, Some(ns.clone()));
    window.set_width(100.0);
    window.set_height((items.count() as f64 * LIST_BOX_ITEM_HEIGHT) + TEXT_BOX_HEIGHT);

    let tb = TextBox::new();
    tb.set_name(Some("tb".to_string()));
    tb.set_text(Some("foo"));
    tb.set_height(TEXT_BOX_HEIGHT);
    DockPanel::set_dock(&tb, Dock::Top);

    let lb = ListBox::new();
    lb.set_items_source(Some(items.clone().into()));
    let item_template: Rc<dyn IDataTemplate> = {
        let weak_canvases = weak_canvases.clone();
        let weak_ns = Rc::downgrade(&ns.0);
        FuncDataTemplate::for_type::<i32>(
            move |_, _| {
                let canvas = Canvas::new();
                canvas.set_width(10.0);
                canvas.set_height(10.0);
                let binding = ReflectionBinding::new("Text");
                binding.set_element_name(Some("tb".to_string()));
                binding.set_name_scope(Some(weak_ns.clone()));
                canvas.bind_binding(Control::tag_property().as_property(), &*binding);
                weak_canvases.borrow_mut().push(Tracked::object("a canvas of the item template", &canvas));
                Some(canvas.upcast())
            },
            false,
        )
    };
    lb.set_item_template(Some(item_template));
    lb.set_padding(Thickness::uniform(0.0));

    let dock_panel = DockPanel::new();
    dock_panel.children().add(tb.clone());
    dock_panel.children().add(lb.clone());
    window.set_content(Some(Control::boxed(dock_panel)));

    tb.clone().register_in_name_scope(&*ns);

    window.show();
    window.layout_manager().execute_initial_layout_pass();

    let assert_initial_item_state = || {
        let item0 = lb.get_realized_containers().into_iter().next().and_then(|item| item.cast::<ListBoxItem>());
        let item0 = item0.expect("the first container is a list box item");
        let canvas0 = item0.presenter().and_then(|presenter| presenter.child()).and_then(|child| child.cast::<Canvas>());
        let canvas0 = canvas0.expect("the content of the first item is a canvas of the template");
        assert_eq!(Some("foo".to_string()), text_of(canvas0.tag()));
    };

    assert_eq!(10, lb.get_realized_containers().len());
    assert_initial_item_state();

    items.clear();
    window.layout_manager().execute_layout_pass();

    assert!(lb.get_realized_containers().is_empty());
    assert_eq!(10, weak_canvases.borrow().len());

    collect_garbage();

    assert_all_freed(&weak_canvases.borrow());
}

#[test]
fn hot_key_manager_should_release_reference_when_control_detached() {
    let _app = start_control_test();

    let button = {
        let gesture1 = KeyGesture::new(Key::A, KeyModifiers::CONTROL);
        let tl = Window::new();
        tl.set_content(Some(Control::boxed(ItemsControl::new())));

        tl.show();

        let button = Button::new();
        tl.set_content(Some(Control::boxed(&button)));
        tl.set_template(Some(create_window_template()));
        tl.apply_template();
        tl.presenter().expect("the template of the window has a content presenter").apply_template();
        HotKeyManager::set_hot_key(&button, Some(gesture1));

        // Detach the button from the logical tree, so there is no reference to it.
        tl.set_content(None);
        tl.apply_template();

        Tracked::object("the button", &button)
    };

    collect_garbage();

    button.assert_freed();
}

#[test]
fn hot_key_manager_should_release_reference_when_control_in_item_template_detached() {
    let _app = start_control_test();

    let weak_buttons = {
        let gesture1 = KeyGesture::new(Key::A, KeyModifiers::CONTROL);

        let tl = Window::new();
        tl.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
        tl.set_is_visible(true);
        let lm = tl.layout_manager();
        tl.show();

        let key_gestures = Rc::new(FerroList::from_items([gesture1]));
        let weak_buttons: Rc<RefCell<Vec<Tracked>>> = Rc::new(RefCell::new(Vec::new()));
        let list_box = ListBox::new();
        list_box.set_width(100.0);
        list_box.set_height(100.0);
        // A button with a binding to the key gesture in the template, added
        // to the list of the weak handles.
        let item_template: Rc<dyn IDataTemplate> = {
            let weak_buttons = weak_buttons.clone();
            FuncDataTemplate::for_type::<KeyGesture>(
                move |key_gesture, _| {
                    let button = Button::new();
                    button.set_data_context(Some(Rc::new(*key_gesture) as BoxedValue));
                    button.bind_binding(Button::hot_key_property().as_property(), &*ReflectionBinding::new(""));
                    weak_buttons.borrow_mut().push(Tracked::object("a button of the item template", &button));
                    Some(button.upcast())
                },
                false,
            )
        };
        list_box.set_item_template(Some(item_template));
        // Add the list box and render it.
        tl.set_content(Some(Control::boxed(&list_box)));
        lm.execute_initial_layout_pass();
        list_box.set_items_source(Some(key_gestures.clone().into()));
        lm.execute_layout_pass();

        // Let the button detach when clearing the source items.
        key_gestures.clear();
        lm.execute_layout_pass();

        // Add it again to double check, and render.
        key_gestures.add(gesture1);
        lm.execute_layout_pass();

        key_gestures.clear();
        lm.execute_layout_pass();

        weak_buttons
    };

    assert!(!weak_buttons.borrow().is_empty());

    collect_garbage();

    assert_all_freed(&weak_buttons.borrow());
}

#[test]
fn tool_tip_is_freed() {
    let _app = start_control_test();

    let tracked = {
        let window = Window::new();
        // The text block the template builds: the template is built once.
        let text_block: Rc<RefCell<Option<Tracked>>> = Rc::new(RefCell::new(None));
        let source = Button::new();
        let template: Rc<dyn IControlTemplate> = {
            let text_block = text_block.clone();
            FuncControlTemplate::for_type::<Button>(move |_, _| {
                assert!(text_block.borrow().is_none());
                let tip = text_block_bound_to_the_content();
                *text_block.borrow_mut() = Some(Tracked::object("the text block of the tooltip", &tip));
                let decorator = Decorator::new();
                ToolTip::set_tip(&decorator, Some(Control::boxed(tip)));
                decorator.upcast()
            })
        };
        source.set_template(Some(template));

        window.set_content(Some(Control::boxed(&source)));
        window.show();

        let children = source.get_visual_children();
        assert_eq!(1, children.len());
        let template_child = children[0].cast::<Decorator>().expect("the template of the button is the decorator");
        drop(children);
        ToolTip::set_is_open(&template_child, true);
        let tool_tip = template_child.get_value(ToolTip::tool_tip_property());
        let tool_tip = tool_tip.expect("an open tooltip is the value of the tooltip property");

        ToolTip::set_is_open(&template_child, false);

        // Detach the button from the logical tree, so there is no reference to it.
        window.set_content(None);

        let text_block = text_block.borrow_mut().take().expect("the template was built");
        [Tracked::object("the tooltip", &tool_tip), text_block]
    };

    collect_garbage();

    assert_all_freed(&tracked);
}

#[test]
fn flyout_is_freed() {
    let _app = start_control_test();

    let tracked = {
        let window = Window::new();
        let source = Button::new();
        let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<Button>(|_, _| {
            let flyout = Flyout::new();
            flyout.set_content(Some(Control::boxed(text_block_bound_to_the_content())));
            let button = Button::new();
            button.set_flyout(&flyout);
            button.upcast()
        });
        source.set_template(Some(template));

        window.set_content(Some(Control::boxed(&source)));
        window.show();

        let children = source.get_visual_children();
        assert_eq!(1, children.len());
        let template_child = children[0].cast::<Button>().expect("the template of the button is the inner button");
        drop(children);
        let flyout = template_child.flyout().and_then(|flyout| flyout.cast::<Flyout>());
        let flyout = flyout.expect("the inner button has the flyout of the template");
        let text_block =
            flyout.content().and_then(|content| Control::from_boxed(&content)).and_then(|c| c.cast::<TextBlock>());
        let text_block = text_block.expect("the content of the flyout is the text block");

        flyout.show_at(&template_child);

        flyout.hide();

        // Detach the button from the logical tree, so there is no reference to it.
        window.set_content(None);

        [Tracked::object("the flyout", &flyout), Tracked::object("the text block of the flyout", &text_block)]
    };

    collect_garbage();

    assert_all_freed(&tracked);
}

#[test]
fn layout_transform_control_is_freed() {
    let _app = start_control_test();

    let transform = RotateTransform::with_angle(90.0);

    let tracked = {
        let canvas = Canvas::new();
        let layout_transform_control = LayoutTransformControl::new();
        layout_transform_control.set_layout_transform(Some((&transform).into()));
        layout_transform_control.set_child(&canvas);
        let window = Window::new();
        window.set_content(Some(Control::boxed(&layout_transform_control)));

        window.show();

        // Do a layout and make sure that the control gets added to the visual tree.
        window.layout_manager().execute_initial_layout_pass();
        let child = presenter_child(&window).expect("the window has its content");
        assert!(child.is::<LayoutTransformControl>());
        assert!(!child.get_visual_children().is_empty());
        drop(child);

        // Clear the content and ensure the control is removed.
        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        [Tracked::object("the layout transform control", &layout_transform_control), Tracked::object("the canvas", &canvas)]
    };

    collect_garbage();

    assert_all_freed(&tracked);

    // The transform is kept alive: a resource that outlives the control.
    drop(transform);
}

#[test]
fn menu_is_freed() {
    let _app = start_control_test();

    let window = Window::new();

    let menu = {
        let menu = Menu::new();
        window.set_content(Some(Control::boxed(&menu)));

        window.show();

        // Do a layout and make sure that the menu gets added to the visual tree.
        window.layout_manager().execute_initial_layout_pass();
        let child = presenter_child(&window).expect("the window has its content");
        assert!(child.is::<Menu>());
        assert!(!child.get_visual_children().is_empty());
        drop(child);

        // Clear the content and ensure the menu is removed.
        window.set_content(None);
        window.layout_manager().execute_layout_pass();
        assert!(presenter_child(&window).is_none());

        Tracked::object("the menu", &menu)
    };

    collect_garbage();

    menu.assert_freed();

    drop(window);
}
