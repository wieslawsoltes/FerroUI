//! Typed bindings to the properties of a view model.

use crate::generated::builders::*;
use crate::reactive::Owner;
use crate::{IntoElement, ModelProperty, ViewModel};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, Ref};
use ferroui_controls::primitives::RangeBase;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, Slider, TextBlock, TextBox, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(ViewModel)]
#[computed(greeting: String, depends_on(name))]
struct Person {
    #[property]
    name: RefCell<String>,
    #[property]
    age: Cell<f64>,
    property_changed: Event<str>,
}

impl Person {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(name.to_string()), age: Cell::new(30.0), property_changed: Event::default() })
    }

    fn greeting(&self) -> String {
        format!("Hello, {}", self.name.borrow())
    }
}

/// A model with setters of its own, which raises its own names.
struct Existing {
    title: RefCell<String>,
    property_changed: Event<str>,
}

impl PartialEq for Existing {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for Existing {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl Existing {
    fn title(&self) -> String {
        self.title.borrow().clone()
    }

    fn set_title(&self, value: String) {
        *self.title.borrow_mut() = value;
        self.property_changed.raise("Title");
    }
}

const TITLE: ModelProperty<Existing, String> =
    ModelProperty::with_raised_name("Title", Existing::title, Existing::set_title);

struct App {
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

fn app() -> App {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(TestServices::styled_window().with_render_interface(render_interface));
    App { _app: app, _text: text }
}

fn window_with(content: &Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(300.0);
    window.set_height(200.0);
    window.set_content(Some(Control::boxed(content)));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
}

fn find<T: ferroui_base::ObjectType>(root: &Ref<Control>) -> Ref<T> {
    root.get_visual_descendants().find_map(|visual| visual.cast::<T>()).expect("the control")
}

#[test]
fn one_way_follows_the_model() {
    let _app = app();
    let person = Person::new("Ada");
    let owner = Owner::new_root();
    let root = owner.with(|| {
        stack_panel()
            .data_context(person.clone() as BoxedValue)
            .child(text_block().text(Person::GREETING.one_way()))
            .into_element()
            .mount()
    });
    let window = window_with(&root);
    let text = find::<TextBlock>(&root);
    assert_eq!(text.text().as_deref(), Some("Hello, Ada"));
    person.set_name("Grace".to_string());
    assert_eq!(text.text().as_deref(), Some("Hello, Grace"));
    window.close();
    owner.dispose();
}

#[test]
fn two_way_writes_a_change_of_the_control_to_the_model() {
    let _app = app();
    let person = Person::new("Ada");
    let owner = Owner::new_root();
    let root = owner.with(|| {
        stack_panel()
            .data_context(person.clone() as BoxedValue)
            .child(text_box().text(Person::NAME.two_way()))
            .child(slider().maximum(100.0).value(Person::AGE.two_way()))
            .into_element()
            .mount()
    });
    let window = window_with(&root);
    let (text, age) = (find::<TextBox>(&root), find::<Slider>(&root));
    assert_eq!(text.text().as_deref(), Some("Ada"));
    assert_eq!(age.value(), 30.0);

    text.set_text(Some("Grace"));
    assert_eq!(person.name(), "Grace");
    age.set_value(RangeBase::value_property(), 40.0);
    assert_eq!(person.age(), 40.0);

    person.set_name("Lin".to_string());
    assert_eq!(text.text().as_deref(), Some("Lin"));
    person.set_age(50.0);
    assert_eq!(age.value(), 50.0);
    window.close();
    owner.dispose();
}

#[test]
fn one_time_takes_the_value_once() {
    let _app = app();
    let person = Person::new("Ada");
    let owner = Owner::new_root();
    let root = owner.with(|| {
        stack_panel()
            .data_context(person.clone() as BoxedValue)
            .child(text_block().text(Person::NAME.one_time()))
            .into_element()
            .mount()
    });
    let window = window_with(&root);
    let text = find::<TextBlock>(&root);
    assert_eq!(text.text().as_deref(), Some("Ada"));
    person.set_name("Grace".to_string());
    assert_eq!(text.text().as_deref(), Some("Ada"));
    window.close();
    owner.dispose();
}

#[test]
fn one_way_to_source_writes_only_to_the_model() {
    let _app = app();
    let person = Person::new("Ada");
    let owner = Owner::new_root();
    let root = owner.with(|| {
        stack_panel()
            .data_context(person.clone() as BoxedValue)
            .child(text_box().text(Person::NAME.one_way_to_source()))
            .into_element()
            .mount()
    });
    let window = window_with(&root);
    let text = find::<TextBox>(&root);
    text.set_text(Some("Grace"));
    assert_eq!(person.name(), "Grace");
    person.set_name("Lin".to_string());
    assert_eq!(text.text().as_deref(), Some("Grace"));
    window.close();
    owner.dispose();
}

#[test]
fn rsx_takes_a_typed_binding() {
    let _app = app();
    let person = Person::new("Ada");
    let owner = Owner::new_root();
    let root = owner.with(|| {
        let context = person.clone() as BoxedValue;
        let element: crate::Element = crate::rsx! {
            StackPanel { data_context: context,
                TextBlock { text: Person::GREETING.one_way() }
            }
        };
        element.mount()
    });
    let window = window_with(&root);
    assert_eq!(find::<TextBlock>(&root).text().as_deref(), Some("Hello, Ada"));
    window.close();
    owner.dispose();
}

#[test]
fn the_declaration_gives_the_name_the_setter_raises() {
    assert_eq!(Person::NAME.name(), "name");
    assert_eq!(Person::AGE.name(), "age");
    assert_eq!(Person::GREETING.name(), "greeting");
    let person = Person::new("Ada");
    let raised = Rc::new(RefCell::new(Vec::new()));
    let seen = raised.clone();
    let _token = person.property_changed().add(Rc::new(move |name: &str| seen.borrow_mut().push(name.to_string())));
    person.set_name("Ada".to_string());
    assert!(raised.borrow().is_empty(), "the same value raises nothing");
    person.set_name("Grace".to_string());
    assert_eq!(*raised.borrow(), ["name", "greeting"], "a change raises the property and what depends on it");
    person.set_age(31.0);
    assert_eq!(raised.borrow().last().map(String::as_str), Some("age"));
    assert!(*person == *person.clone() && *person != *Person::new("Grace"), "a model compares by identity");
}

#[test]
fn a_model_with_its_own_setters_binds_with_the_name_it_raises() {
    let _app = app();
    let model = Rc::new(Existing { title: RefCell::new("first".to_string()), property_changed: Event::default() });
    let owner = Owner::new_root();
    let root = owner.with(|| {
        stack_panel()
            .data_context(model.clone() as BoxedValue)
            .child(text_block().text(TITLE.one_way()))
            .into_element()
            .mount()
    });
    let window = window_with(&root);
    let text = find::<TextBlock>(&root);
    assert_eq!(text.text().as_deref(), Some("first"));
    model.set_title("second".to_string());
    assert_eq!(text.text().as_deref(), Some("second"));
    window.close();
    owner.dispose();
}
