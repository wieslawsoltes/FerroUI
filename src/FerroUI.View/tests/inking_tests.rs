//! Inking must work through the public builders and the runtime XAML model.
use crate::generated::builders::*;
use crate::generated::inking::*;
use crate::IntoElement;
use ferroui_base::input::PointerType;
use ferroui_base::{Point, Size};
use ferroui_controls::Control;
use ferroui_controls_inking::{
    CoreInputDeviceTypes, InkCanvas, InkDrawingAttributes, InkInputProcessingMode, InkPoint,
    InkPointerEventArgs, InkPointerPhase, InkPresenter, InkToolbar, InkToolbarBallpointPenButton,
    InkToolbarInitialControls, InkToolbarTool,
};
use std::rc::Rc;

#[test]
fn inking_builders_construct_and_target_real_controls() {
    let presenter = InkPresenter::new();
    let (toolbar, owner) = ink_toolbar()
        .initial_controls(InkToolbarInitialControls::PensOnly)
        .target_ink_presenter(Some(presenter.clone()))
        .into_element()
        .mount_root();
    let toolbar = toolbar.cast::<InkToolbar>().unwrap();
    assert_eq!(toolbar.children().count(), 3);
    assert!(Rc::ptr_eq(
        &toolbar.target_ink_presenter().unwrap(),
        &presenter
    ));
    let pen = toolbar
        .get_tool_button(InkToolbarTool::BallpointPen)
        .unwrap();
    assert!(pen.cast::<InkToolbarBallpointPenButton>().is_some());
    assert_eq!(
        presenter.input_processing_configuration().mode(),
        InkInputProcessingMode::Inking
    );
    let (canvas, canvas_owner) = ink_canvas()
        .width(480.0)
        .height(320.0)
        .into_element()
        .mount_root();
    assert_eq!(canvas.cast::<InkCanvas>().unwrap().width(), 480.0);
    owner.dispose();
    canvas_owner.dispose();
}

#[test]
fn inking_xaml_populates_stable_plain_services_and_custom_children() {
    ferroui_controls::register_types();
    ferroui_controls_inking::register_types();
    ferroui_markup_xaml::register_types();
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
    let loaded = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
        r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
          <StackPanel.Resources>
            <InkPresenter x:Key="presenter" InputDeviceTypes="Pen, Mouse" HighContrastAdjustment="UseOriginalColors" />
          </StackPanel.Resources>
          <InkCanvas x:Name="surface" />
          <InkToolbar x:Name="tools" InitialControls="None" TargetInkPresenter="{StaticResource presenter}">
            <InkToolbarBallpointPenButton SelectedStrokeWidth="7" />
            <InkToolbarEraserButton />
          </InkToolbar>
        </StackPanel>"#,
        None,
    ).unwrap_or_else(|error| panic!("{error}"));
    let root = Control::from_boxed(&loaded).unwrap();
    let canvas = root.get_control::<InkCanvas>("surface");
    let toolbar = root.get_control::<InkToolbar>("tools");
    let presenter = toolbar.target_ink_presenter().unwrap();
    assert_eq!(
        presenter.input_device_types(),
        CoreInputDeviceTypes::PEN | CoreInputDeviceTypes::MOUSE
    );
    assert_eq!(toolbar.children().count(), 2);
    toolbar.set_target_ink_canvas(Some(canvas));
    // As in WinUI, a custom child becomes the initial tool during layout.
    toolbar.measure(Size::new(480.0, 64.0));
    assert_eq!(presenter.copy_default_drawing_attributes().size.width, 7.0);
    let input = InkPointerEventArgs::new(
        1,
        PointerType::Mouse,
        InkPoint::new(Point::new(10.0, 20.0), 0.5).unwrap(),
    );
    presenter
        .process_pointer(InkPointerPhase::Pressed, &input)
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Released, &input)
        .unwrap();
    assert_eq!(presenter.stroke_container().get_strokes().len(), 1);
    assert_ne!(
        presenter.copy_default_drawing_attributes(),
        InkDrawingAttributes::default()
    );
}
