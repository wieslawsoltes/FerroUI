// The reference tests use a rectangle shape as the scaled child; any control
// with an explicit size shows the same behaviour, so a border is used here.
use crate::primitives::{TemplatedControl, TemplatedControlImpl};
use crate::{Border, Canvas, Control, ControlImpl, Viewbox};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{FerroObjectImpl, StyledElementImpl, VisualImpl};
use ferroui_base::media::{Stretch, StretchDirection};
use ferroui_base::{Matrix, Point, Rect, Ref, Size, StyledElement, Vector};

fn sized<T>(control: Ref<T>, width: f64, height: f64) -> Ref<T>
where
    T: ferroui_base::ObjectType + ferroui_base::Upcast<ferroui_base::layout::Layoutable>,
{
    {
        let layoutable: &ferroui_base::layout::Layoutable = (*control).upcast();
        layoutable.set_width(width);
        layoutable.set_height(height);
    }
    control
}

fn viewbox_with_rectangle() -> Ref<Viewbox> {
    let target = Viewbox::new();
    target.set_child(sized(Border::new(), 100.0, 50.0));
    target
}

fn try_get_scale(viewbox: &Viewbox) -> Option<Vector> {
    let matrix = viewbox.internal_transform()?.value();

    Some(Matrix::try_decompose_transform(matrix).map(|decomposed| decomposed.scale).unwrap_or_default())
}

#[test]
fn viewbox_stretch_uniform_child() {
    let target = viewbox_with_rectangle();

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(200.0, 100.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 2.0);
    assert_eq!(scale.y, 2.0);
}

#[test]
fn viewbox_stretch_none_child() {
    let target = viewbox_with_rectangle();
    target.set_stretch(Stretch::None);

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(100.0, 50.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 1.0);
    assert_eq!(scale.y, 1.0);
}

#[test]
fn viewbox_stretch_fill_child() {
    let target = viewbox_with_rectangle();
    target.set_stretch(Stretch::Fill);

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(200.0, 200.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 2.0);
    assert_eq!(scale.y, 4.0);
}

#[test]
fn viewbox_stretch_uniform_to_fill_child() {
    let target = viewbox_with_rectangle();
    target.set_stretch(Stretch::UniformToFill);

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(200.0, 200.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 4.0);
    assert_eq!(scale.y, 4.0);
}

#[test]
fn viewbox_stretch_uniform_child_with_unrestricted_width() {
    let target = viewbox_with_rectangle();

    target.measure(Size::new(f64::INFINITY, 200.0));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(400.0, 200.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 4.0);
    assert_eq!(scale.y, 4.0);
}

#[test]
fn viewbox_stretch_uniform_child_with_unrestricted_height() {
    let target = viewbox_with_rectangle();

    target.measure(Size::new(200.0, f64::INFINITY));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(200.0, 100.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 2.0);
    assert_eq!(scale.y, 2.0);
}

/// (child width, child height, viewbox width, viewbox height, expected width,
/// expected height, expected scale)
type SizeAndScaleCase = (f64, f64, f64, f64, f64, f64, f64);

fn check_size_and_scale(stretch_direction: StretchDirection, cases: &[SizeAndScaleCase]) {
    for &(child_width, child_height, viewbox_width, viewbox_height, expected_width, expected_height, expected_scale) in
        cases
    {
        let target = Viewbox::new();
        target.set_child(sized(Control::new(), child_width, child_height));
        target.set_stretch_direction(stretch_direction);

        target.measure(Size::new(viewbox_width, viewbox_height));
        target.arrange(Rect::from_position_size(Point::default(), target.desired_size()));

        assert_eq!(target.desired_size(), Size::new(expected_width, expected_height));

        let scale = try_get_scale(&target).unwrap();
        assert_eq!(scale.x, expected_scale);
        assert_eq!(scale.y, expected_scale);
    }
}

#[test]
fn viewbox_should_return_correct_size_and_scale_stretch_direction_down_only() {
    check_size_and_scale(
        StretchDirection::DownOnly,
        &[
            (50.0, 100.0, 50.0, 100.0, 50.0, 100.0, 1.0),
            (50.0, 100.0, 150.0, 150.0, 50.0, 100.0, 1.0),
            (50.0, 100.0, 25.0, 50.0, 25.0, 50.0, 0.5),
        ],
    );
}

#[test]
fn viewbox_should_return_correct_size_and_scale_stretch_direction_up_only() {
    check_size_and_scale(
        StretchDirection::UpOnly,
        &[
            (50.0, 100.0, 50.0, 100.0, 50.0, 100.0, 1.0),
            (50.0, 100.0, 25.0, 50.0, 25.0, 50.0, 1.0),
            (50.0, 100.0, 150.0, 150.0, 75.0, 150.0, 1.5),
        ],
    );
}

#[test]
fn child_should_be_logical_child_of_viewbox() {
    let target = Viewbox::new();

    assert!(target.logical_children().is_empty());

    let child = Canvas::new();
    target.set_child(&child);

    assert_eq!(target.logical_children().to_vec(), vec![child.clone().upcast::<StyledElement>()]);
    assert_eq!(child.parent().unwrap(), target);

    target.set_child(None);

    assert!(target.logical_children().is_empty());
    assert!(child.parent().is_none());
}

#[test]
fn changing_child_should_invalidate_layout() {
    let target = Viewbox::new();

    target.set_child(sized(Canvas::new(), 100.0, 100.0));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
    assert_eq!(target.desired_size(), Size::new(100.0, 100.0));

    target.set_child(sized(Canvas::new(), 200.0, 200.0));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
    assert_eq!(target.desired_size(), Size::new(200.0, 200.0));
}

#[test]
fn child_is_hosted_in_a_container_visual() {
    let target = Viewbox::new();
    let child = Canvas::new();

    target.set_child(&child);

    assert_eq!(target.visual_children().count(), 1);
    let container = target.visual_children().get(0);
    assert_eq!(child.visual_parent().unwrap(), container);
    assert_eq!(container.visual_parent().unwrap(), target);

    target.set_child(None);

    assert!(child.visual_parent().is_none());
    assert_eq!(target.visual_children().count(), 1);
}

/// The data of `child_data_context_binding_works` (an anonymous object in
/// the reference).
struct FooData {
    foo: String,
}

ferroui_base::ferro_model!(FooData, |b| b
    .read_only::<ferroui_base::data::core::Value<String>>("Foo", |data| data.foo.clone()));

#[test]
fn child_data_context_binding_works() {
    use crate::test_support::string_of;
    use ferroui_base::data::model::Model;
    use ferroui_base::data::ReflectionBinding;

    let data = Model::new_model(FooData { foo: "foo".to_string() });

    let child = Canvas::new();
    child.bind_binding(StyledElement::data_context_property().as_property(), &*ReflectionBinding::new("Foo"));
    let target = Viewbox::new();
    target.set_data_context(Some(data));
    target.set_child(&child);

    assert_eq!(Some("foo".to_string()), target.child().unwrap().data_context().as_ref().and_then(string_of));
}

/// The `TestTemplatedControl` of the reference: a templated control with a
/// property that holds a control.
#[repr(C)]
struct TestTemplatedControl {
    base: TemplatedControl,
}

ferroui_base::ferro_class!(TestTemplatedControl: TemplatedControl);
ferroui_base::ferro_impl_classes!(
    TestTemplatedControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl TestTemplatedControl {
    ferroui_base::ferro_property!(
        fn my_control_property() -> ferroui_base::StyledProperty<Option<Ref<Control>>> {
            ferroui_base::FerroProperty::register::<TestTemplatedControl, _>("MyControl", None)
        }
    );

    fn new() -> Ref<Self> {
        ferroui_base::instantiate(Self { base: TemplatedControl::construct() })
    }

    fn set_my_control(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::my_control_property(), value)
    }
}

#[test]
fn content_presented_in_viewbox_should_be_reparented_when_template_changes() {
    use crate::presenters::ContentPresenter;
    use crate::templates::{FuncControlTemplate, IControlTemplate};
    use crate::test_support::{test_scope, TestRoot};
    use ferroui_base::data::TemplateBinding;
    use ferroui_base::layout::ILayoutManager;
    use std::rc::Rc;

    // Issue #9551: a template containing Viewbox > ContentPresenter presenting a control
    // that outlives the template. Swapping the template must disconnect the presented
    // control so the new template's presenter can adopt it.
    let _scope = test_scope();

    fn create_template() -> Option<Rc<dyn IControlTemplate>> {
        Some(FuncControlTemplate::for_type::<TestTemplatedControl>(|_, _| {
            let presenter = ContentPresenter::new();
            presenter.bind_binding(
                ContentPresenter::content_property().as_property(),
                &TemplateBinding::new(TestTemplatedControl::my_control_property().as_property()),
            );
            let viewbox = Viewbox::new();
            viewbox.set_child(presenter);
            viewbox.upcast()
        }))
    }

    let child = Canvas::new();
    let target = TestTemplatedControl::new();
    target.set_my_control(Some(child.clone().upcast()));
    target.set_template(create_template());

    let root = TestRoot::with_child(&target);
    root.execute_initial_layout_pass();

    let old_presenter = child.visual_parent().and_then(|parent| parent.cast::<ContentPresenter>());
    let old_presenter = old_presenter.expect("a content presenter");

    target.set_template(create_template());
    target.apply_template();
    root.layout_manager().execute_layout_pass();

    let new_presenter = child.visual_parent().and_then(|parent| parent.cast::<ContentPresenter>());
    let new_presenter = new_presenter.expect("a content presenter");
    assert!(!old_presenter.ptr_eq(&new_presenter));
    assert!(child.get_visual_root().is_some_and(|visual_root| visual_root.ptr_eq(&root)));
}

