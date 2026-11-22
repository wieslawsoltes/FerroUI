//! Typed bindings to the properties of a view model.

use crate::generated::builders::*;
use crate::reactive::Owner;
use crate::{IntoElement, ModelProperty, ReadOnlyModelProperty};
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

struct Person {
    name: RefCell<String>,
    age: Cell<f64>,
    property_changed: Event<str>,
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for Person {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl Person {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(name.to_string()), age: Cell::new(30.0), property_changed: Event::default() })
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: String) {
        if *self.name.borrow() != value {
            *self.name.borrow_mut() = value;
            self.property_changed.raise("Name");
            self.property_changed.raise("Greeting");
        }
    }

    fn greeting(&self) -> String {
        format!("Hello, {}", self.name.borrow())
    }

    fn age(&self) -> f64 {
        self.age.get()
    }

    fn set_age(&self, value: f64) {
        if self.age.get() != value {
            self.age.set(value);
            self.property_changed.raise("Age");
        }
    }
}

const NAME: ModelProperty<Person, String> = ModelProperty::new("Name", Person::name, Person::set_name);
const AGE: ModelProperty<Person, f64> = ModelProperty::new("Age", Person::age, Person::set_age);
const GREETING: ReadOnlyModelProperty<Person, String> = ReadOnlyModelProperty::new("Greeting", Person::greeting);

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
            .child(text_block().text(GREETING.one_way()))
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
            .child(text_box().text(NAME.two_way()))
            .child(slider().maximum(100.0).value(AGE.two_way()))
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
            .child(text_block().text(NAME.one_time()))
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
            .child(text_box().text(NAME.one_way_to_source()))
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
                TextBlock { text: GREETING.one_way() }
            }
        };
        element.mount()
    });
    let window = window_with(&root);
    assert_eq!(find::<TextBlock>(&root).text().as_deref(), Some("Hello, Ada"));
    window.close();
    owner.dispose();
}
