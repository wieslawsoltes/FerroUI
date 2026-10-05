//! Classes and helpers shared by the binding tests.
//!
//! The control library is a separate crate that does not have these controls
//! yet, so the tests use minimal classes with the same properties in place
//! of the controls used by the tests these were ported from.

use ferroui_base::controls::{INameScope, NameScope, NameScopeRef};
use ferroui_base::data::{BindingError, BindingValueType};
use ferroui_base::input::{
    FocusManager, ICommand, IInputRoot, InputElement, InputElementImpl,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, ILayoutRoot, LayoutManager, Layoutable, LayoutableImpl};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::rendering::{
    IHitTester, IPresentationSource, IRenderer, ManagedHitTester, RendererDiagnostics, SceneInvalidatedEventArgs,
};
use ferroui_base::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub fn s(value: &str) -> String {
    value.to_string()
}

/// An untyped value.
pub fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
    Rc::new(value)
}

/// An untyped string value.
pub fn bs(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_string()))
}

/// The string held by an untyped value, if it holds one.
pub fn as_string(value: &Option<BoxedValue>) -> Option<String> {
    value.as_ref().and_then(|v| v.downcast_ref::<String>().cloned())
}

/// An object of the class hierarchy as a binding source.
pub fn object_source<T: ObjectType + Upcast<FerroObject>>(object: &Ref<T>) -> Option<BoxedValue> {
    Some(Rc::new(object.clone().upcast::<FerroObject>()))
}

/// Asserts that `f` panics: the equivalent of asserting that an exception is
/// thrown.
#[track_caller]
pub fn assert_panics(f: impl FnOnce()) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    assert!(result.is_err(), "expected a panic");
}

// --- controls -------------------------------------------------------------

macro_rules! control_class {
    ($(#[$meta:meta])* $name:ident : $base:ident [$($property:ident),*]) => {
        $(#[$meta])*
        #[repr(C)]
        pub struct $name {
            base: $base,
        }

        ferroui_base::ferro_class!($name: $base);
        ferroui_base::ferro_impl_classes!(
            $name: FerroObjectImpl,
            StyledElementImpl,
            VisualImpl,
            LayoutableImpl,
            InteractiveImpl,
            InputElementImpl
        );

        impl $name {
            pub fn construct() -> Self {
                Self { base: $base::construct() }
            }

            /// Registers the properties of the class and its base classes
            /// (the equivalent of the static initialisation of the class).
            pub fn class_init() {
                $base::class_init();
                $(Self::$property();)*
                ferroui_base::data::core::ValueTypes::register_object::<$name>();
            }

            pub fn new() -> Ref<Self> {
                Self::class_init();
                instantiate(Self::construct())
            }
        }
    };
}
pub(crate) use control_class;

/// The base of the test controls: an input element with a tag.
#[repr(C)]
pub struct Control {
    base: InputElement,
}

ferro_class!(Control: InputElement);
ferro_impl_classes!(
    Control: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl
);

impl Control {
    pub fn construct() -> Self {
        Self { base: InputElement::construct() }
    }

    pub fn class_init() {
        Self::tag_property();
        ferroui_base::data::core::ValueTypes::register_object::<Control>();
    }

    pub fn new() -> Ref<Self> {
        Self::class_init();
        instantiate(Self::construct())
    }

    ferro_property!(pub fn tag_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<Control, _>("Tag", None)
    });

    pub fn tag(&self) -> Option<BoxedValue> {
        self.get_value(Self::tag_property())
    }

    pub fn set_tag(&self, value: Option<BoxedValue>) {
        self.set_value(Self::tag_property(), value)
    }
}

/// Makes `child` a logical and visual child of `parent`.
pub fn add_child<P, T>(parent: &Ref<P>, child: &Ref<T>)
where
    P: ObjectType + Upcast<Visual>,
    T: ObjectType + Upcast<StyledElement> + Upcast<Visual>,
{
    let parent: Ref<Visual> = parent.clone().upcast();
    parent.logical_children().add(child.clone().upcast());
    parent.visual_children().add(child.clone().upcast());
}

/// Removes `child` from the logical and visual children of `parent`.
pub fn remove_child<P, T>(parent: &Ref<P>, child: &Ref<T>)
where
    P: ObjectType + Upcast<Visual>,
    T: ObjectType + Upcast<StyledElement> + Upcast<Visual>,
{
    let parent: Ref<Visual> = parent.clone().upcast();
    parent.logical_children().remove(&child.clone().upcast());
    parent.visual_children().remove(&child.clone().upcast());
}

/// A control with a single child.
#[repr(C)]
pub struct Decorator {
    base: Control,
    child: RefCell<Option<Ref<Control>>>,
}

ferro_class!(Decorator: Control);
ferro_impl_classes!(
    Decorator: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl
);

impl Decorator {
    pub fn construct() -> Self {
        Self { base: Control::construct(), child: RefCell::new(None) }
    }

    pub fn class_init() {
        Control::class_init();
        ferroui_base::data::core::ValueTypes::register_object::<Decorator>();
    }

    pub fn new() -> Ref<Self> {
        Self::class_init();
        instantiate(Self::construct())
    }

    pub fn with_child<T: ObjectType + Upcast<Control>>(child: &Ref<T>) -> Ref<Self> {
        let result = Self::new();
        result.set_child(Some(child.clone().upcast()));
        result
    }

    pub fn child(&self) -> Option<Ref<Control>> {
        self.child.borrow().clone()
    }

    pub fn set_child(&self, value: Option<Ref<Control>>) {
        let old = self.child.replace(value.clone());
        let this: Ref<Visual> = self.to_ref().upcast();
        if let Some(old) = old {
            this.logical_children().remove(&old.clone().upcast());
            this.visual_children().remove(&old.upcast());
        }
        if let Some(new) = value {
            this.logical_children().add(new.clone().upcast());
            this.visual_children().add(new.upcast());
        }
    }
}

control_class!(
    /// A decorator subclass.
    Border: Decorator []
);

control_class!(
    /// A control with any number of children.
    Panel: Control []
);

impl Panel {
    pub fn add<T: ObjectType + Upcast<StyledElement> + Upcast<Visual>>(&self, child: &Ref<T>) {
        add_child(&self.to_ref(), child);
    }
}

control_class!(StackPanel: Panel []);
control_class!(Canvas: Panel []);

control_class!(
    /// A control that shows text.
    TextBlock: Control [text_property]
);

impl TextBlock {
    ferro_property!(pub fn text_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<TextBlock, _>("Text", None)
    });

    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_string))
    }
}

control_class!(
    /// A control that edits text: the text property binds two-way by default
    /// and has data validation enabled.
    TextBox: Control [text_property]
);

impl TextBox {
    ferro_property!(pub fn text_property() -> StyledProperty<Option<String>> {
        TextBlock::text_property().add_owner_with::<TextBox>(
            StyledPropertyMetadata::new(None)
                .with_default_binding_mode(ferroui_base::data::BindingMode::TwoWay)
                .with_enable_data_validation(true),
        )
    });

    pub fn new_focusable() -> Ref<Self> {
        let result = Self::new();
        result.set_value(InputElement::focusable_property(), true);
        result
    }

    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_string))
    }
}

control_class!(
    /// A control with content.
    ContentControl: Control [content_property]
);

impl ContentControl {
    ferro_property!(pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<ContentControl, _>("Content", None)
    });

    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value)
    }
}

control_class!(
    /// A content control with a command.
    Button: ContentControl [command_property]
);

impl Button {
    ferro_property!(pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
        FerroProperty::register::<Button, _>("Command", None)
    });

    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    /// Clicks the button: executes its command, if it has one that can be
    /// executed.
    pub fn click(&self) {
        if let Some(command) = self.command() {
            if command.can_execute(None) {
                command.execute(None);
            }
        }
    }
}

control_class!(
    /// The part of a content control's template that shows its content.
    ContentPresenter: Control [content_property, max_lines_property]
);

impl ContentPresenter {
    ferro_property!(pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<ContentPresenter, _>("Content", None)
    });

    ferro_property!(pub fn max_lines_property() -> StyledProperty<i32> {
        FerroProperty::register::<ContentPresenter, _>("MaxLines", 0)
    });

    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value)
    }

    pub fn max_lines(&self) -> i32 {
        self.get_value(Self::max_lines_property())
    }
}

control_class!(
    /// A control with a numeric value.
    ProgressBar: Control [value_property]
);

impl ProgressBar {
    ferro_property!(pub fn value_property() -> StyledProperty<f64> {
        FerroProperty::register::<ProgressBar, _>("Value", 0.0)
    });

    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }
}

control_class!(
    /// A control that shows a collection of items.
    ItemsControl: Control [items_source_property]
);

impl ItemsControl {
    ferro_property!(pub fn items_source_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<ItemsControl, _>("ItemsSource", None)
    });

    pub fn items_source(&self) -> Option<BoxedValue> {
        self.get_value(Self::items_source_property())
    }
}

/// Applies a template to a templated control the way a templated control
/// does: the template child was built (and its bindings were created) before
/// it gets its templated parent, its logical parent and its visual parent.
pub fn apply_template<P, T>(parent: &Ref<P>, child: &Ref<T>)
where
    P: ObjectType + Upcast<Visual> + Upcast<FerroObject>,
    T: ObjectType + Upcast<StyledElement> + Upcast<Visual>,
{
    let element: Ref<StyledElement> = child.clone().upcast();
    element.set_templated_parent(Some(parent.clone().upcast::<FerroObject>()));
    let visual: Ref<Visual> = parent.clone().upcast();
    element.set_parent(Some(visual.clone().upcast::<StyledElement>()));
    visual.visual_children().add(child.clone().upcast());
}

// --- root -------------------------------------------------------------------

struct TestRenderer;

impl IRenderer for TestRenderer {
    fn diagnostics(&self) -> Rc<RendererDiagnostics> {
        RendererDiagnostics::new()
    }
    fn scene_invalidated(&self, _handler: Rc<dyn Fn(&SceneInvalidatedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
    fn try_get_render_interface_feature(&self, _feature_type: std::any::TypeId) -> Option<Rc<dyn std::any::Any>> {
        None
    }
    fn add_dirty(&self, _visual: &Visual) {}
    fn recalculate_children(&self, _visual: &Visual) {}
    fn resized(&self, _size: Size) {}
    fn paint(&self, _rect: Rect) {}
    fn start(&self) {}
    fn stop(&self) {}
    fn dispose(&self) {}
}

/// The presentation source of a [`TestRoot`]: makes it the root of a visual
/// tree, a layout root and an input root with a focus manager.
struct TestSource {
    this: Weak<TestSource>,
    root: WeakRef<TestRoot>,
    renderer: Rc<TestRenderer>,
    layout_manager: RefCell<Option<Rc<LayoutManager>>>,
    focus_manager: Rc<FocusManager>,
}

impl TestSource {
    fn new(root: &Ref<TestRoot>) -> Rc<Self> {
        let source = Rc::new_cyclic(|this: &Weak<TestSource>| TestSource {
            this: this.clone(),
            root: root.downgrade(),
            renderer: Rc::new(TestRenderer),
            layout_manager: RefCell::new(None),
            focus_manager: FocusManager::new(),
        });
        let as_layout_root: Rc<dyn ILayoutRoot> = source.clone();
        *source.layout_manager.borrow_mut() = Some(LayoutManager::new(Rc::downgrade(&as_layout_root)));
        source
    }

    fn root(&self) -> Ref<TestRoot> {
        self.root.upgrade().expect("the root is alive")
    }
}

impl IPresentationSource for TestSource {
    fn root_visual(&self) -> Option<Ref<Visual>> {
        self.root.upgrade().map(Ref::upcast)
    }
    fn render_scaling(&self) -> f64 {
        1.0
    }
    fn renderer(&self) -> Rc<dyn IRenderer> {
        self.renderer.clone()
    }
    fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
        self.this.upgrade().unwrap()
    }
    fn hit_tester(&self) -> Rc<dyn IHitTester> {
        Rc::new(ManagedHitTester::new())
    }
    fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.this.upgrade().unwrap()
    }
    fn client_size(&self) -> Size {
        Size::new(100.0, 100.0)
    }
}

impl IInputRoot for TestSource {
    fn focus_manager(&self) -> Option<Rc<FocusManager>> {
        Some(self.focus_manager.clone())
    }
    fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
        None
    }
    fn set_pointer_over_element(&self, _value: Option<Ref<InputElement>>) {}
    fn cursor_element(&self) -> Option<Ref<InputElement>> {
        None
    }
    fn set_cursor_element(&self, _value: Option<Ref<InputElement>>) {}
    fn root_element(&self) -> Ref<InputElement> {
        self.root().upcast()
    }
    fn focus_root(&self) -> Ref<InputElement> {
        self.root().upcast()
    }
    fn pointer_over_invalidated(&self) {}
}

impl ILayoutRoot for TestSource {
    fn layout_scaling(&self) -> f64 {
        1.0
    }
    fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.layout_manager.borrow().clone().unwrap()
    }
    fn root_visual(&self) -> Ref<Layoutable> {
        self.root().upcast()
    }
}

/// A decorator that is the root of a logical and a visual tree.
#[repr(C)]
pub struct TestRoot {
    base: Decorator,
}

ferro_class!(TestRoot: Decorator);
ferro_impl_classes!(TestRoot: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

impl StyledElementImpl for TestRoot {
    fn is_logical_root(_this: &Self) -> bool {
        true
    }
}

impl TestRoot {
    pub fn new() -> Ref<Self> {
        Decorator::class_init();
        ferroui_base::data::core::ValueTypes::register_object::<TestRoot>();
        let root = instantiate(Self { base: Decorator::construct() });
        let source = TestSource::new(&root);
        root.set_presentation_source_for_root_visual(Some(source));
        root
    }

    pub fn with_child<T: ObjectType + Upcast<Control>>(child: &Ref<T>) -> Ref<Self> {
        let root = Self::new();
        root.set_child(Some(child.clone().upcast()));
        root
    }

    /// Applies the styles of the tree to the elements that are not styled
    /// yet (the part of an initial layout pass that the tests rely on).
    pub fn execute_initial_layout_pass(&self) {
        fn visit(element: &Ref<StyledElement>) {
            element.apply_styling();
            for child in element.logical_children().snapshot().iter() {
                visit(child);
            }
        }
        visit(&self.to_ref().upcast());
    }

    /// Registers the names of the descendants of the root in the name scope
    /// of the root, creating the name scope if needed.
    pub fn register_children_names(&self) {
        let scope = match NameScope::get_name_scope(self) {
            Some(scope) => scope,
            None => {
                let scope = NameScopeRef::new(NameScope::new());
                NameScope::set_name_scope(self, Some(scope.clone()));
                scope
            }
        };

        fn visit(scope: &NameScopeRef, element: &Ref<StyledElement>) {
            for child in element.logical_children().snapshot().iter() {
                if let Some(name) = child.name() {
                    if scope.find(&name).is_none() {
                        scope.register(&name, child.clone().upcast());
                    }
                }
                visit(scope, child);
            }
        }
        visit(&scope, &self.to_ref().upcast());
    }

    /// The name scope of the root, as bindings refer to it.
    pub fn name_scope(&self) -> Option<Weak<dyn INameScope>> {
        NameScope::get_name_scope(self).map(|scope| Rc::downgrade(&scope.0))
    }
}

// --- data validation ---------------------------------------------------------

/// Records the data validation error of a property, as a control with
/// validated properties does.
#[derive(Default)]
pub struct DataValidationState {
    error: RefCell<Option<BindingError>>,
}

impl DataValidationState {
    pub fn update(&self, state: BindingValueType, error: Option<&BindingError>) {
        let is_error = state == BindingValueType::DATA_VALIDATION_ERROR
            || state == BindingValueType::DATA_VALIDATION_ERROR_WITH_FALLBACK;
        self.error.replace(if is_error { error.cloned() } else { None });
    }

    pub fn error(&self) -> Option<BindingError> {
        self.error.borrow().clone()
    }
}

/// A shared counter.
#[derive(Clone, Default)]
pub struct Counter(Rc<Cell<i32>>);

impl Counter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment(&self) {
        self.0.set(self.0.get() + 1);
    }

    pub fn get(&self) -> i32 {
        self.0.get()
    }
}

// --- logging -------------------------------------------------------------------

use ferroui_base::logging::{ILogSink, LogEventLevel, Logger};

/// A recorded log event.
#[derive(Clone, Debug)]
pub struct LogMessage {
    pub level: LogEventLevel,
    pub area: String,
    /// The address of the object the event originates from.
    pub source: Option<*const ()>,
    pub message_template: String,
    pub property_values: Vec<String>,
}

/// A log sink that records the events logged on the calling thread while it
/// is installed.
pub struct TestLogSink {
    minimum_level: LogEventLevel,
    messages: RefCell<Vec<LogMessage>>,
    previous: RefCell<Option<Option<Rc<dyn ILogSink>>>>,
}

impl TestLogSink {
    /// Installs a sink that records the events of at least `minimum_level`.
    pub fn start(minimum_level: LogEventLevel) -> Rc<Self> {
        let sink = Rc::new(Self { minimum_level, messages: RefCell::new(Vec::new()), previous: RefCell::new(None) });
        let previous = Logger::set_thread_sink(Some(sink.clone()));
        sink.previous.replace(Some(previous));
        sink
    }

    /// Uninstalls the sink and returns the recorded events.
    pub fn stop(&self) -> Vec<LogMessage> {
        if let Some(previous) = self.previous.borrow_mut().take() {
            Logger::set_thread_sink(previous);
        }
        self.messages.borrow().clone()
    }
}

impl ILogSink for TestLogSink {
    fn is_enabled(&self, level: LogEventLevel, _area: &str) -> bool {
        level >= self.minimum_level
    }

    fn log(&self, level: LogEventLevel, area: &str, source: Option<&dyn std::any::Any>, message_template: &str) {
        self.log_with_values(level, area, source, message_template, &[]);
    }

    fn log_with_values(
        &self,
        level: LogEventLevel,
        area: &str,
        source: Option<&dyn std::any::Any>,
        message_template: &str,
        property_values: &[&dyn std::fmt::Display],
    ) {
        if level >= self.minimum_level {
            self.messages.borrow_mut().push(LogMessage {
                level,
                area: area.to_string(),
                source: source.map(|s| s as *const dyn std::any::Any as *const ()),
                message_template: message_template.to_string(),
                property_values: property_values.iter().map(|v| v.to_string()).collect(),
            });
        }
    }
}

// --- binding expressions as observables ------------------------------------------

use ferroui_base::data::core::expression_nodes::ExpressionNode;
use ferroui_base::data::core::parsers::{BindingExpressionGrammar, ExpressionNodeFactory, TypeResolver};
use ferroui_base::data::core::{BindingExpression, BindingExpressionOptions, ExpressionParseException};
use ferroui_base::reactive::{AnonymousObserver, IObservable};

/// Parses a binding path into expression nodes.
pub fn parse_nodes(
    path: &str,
    type_resolver: Option<&TypeResolver>,
) -> Result<Vec<Rc<dyn ExpressionNode>>, ExpressionParseException> {
    let (ast, _source_mode) = BindingExpressionGrammar::parse(path)?;
    let mut nodes = Vec::new();
    ExpressionNodeFactory::create_from_ast(&ast, type_resolver, None, &mut nodes)?;
    Ok(nodes)
}

/// Builds a binding expression over `source` from a binding path.
pub fn build_expression(
    source: Option<BoxedValue>,
    path: &str,
    type_resolver: Option<&TypeResolver>,
    enable_data_validation: bool,
) -> Result<Rc<BindingExpression>, ExpressionParseException> {
    let nodes = parse_nodes(path, type_resolver)?;
    Ok(BindingExpression::new(
        source,
        nodes,
        BindingExpressionOptions { enable_data_validation, ..BindingExpressionOptions::default() },
    ))
}

/// The first value an observable produces when subscribed to (the
/// subscription is disposed again afterwards).
#[track_caller]
pub fn take_one(observable: &dyn IObservable<Option<BoxedValue>>) -> Option<BoxedValue> {
    let result = Rc::new(RefCell::new(None));
    let recorded = result.clone();
    let subscription = observable.subscribe(Rc::new(AnonymousObserver::new(move |value| {
        let mut slot = recorded.borrow_mut();
        if slot.is_none() {
            *slot = Some(value);
        }
    })));
    subscription.dispose();
    let value = result.borrow_mut().take();
    value.expect("the observable produced a value")
}

/// Subscribes to an observable, recording the values it produces.
pub fn record(
    observable: &dyn IObservable<Option<BoxedValue>>,
) -> (Rc<RefCell<Vec<Option<BoxedValue>>>>, Rc<dyn IDisposable>) {
    let result = Rc::new(RefCell::new(Vec::new()));
    let recorded = result.clone();
    let subscription =
        observable.subscribe(Rc::new(AnonymousObserver::new(move |value| recorded.borrow_mut().push(value))));
    (result, subscription)
}
