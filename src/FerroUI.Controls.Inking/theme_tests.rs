//! The WinUI styles of the toolbar (`Themes/WinUI.xaml`): every toolbar control has its control
//! theme, applies its template with the parts it requires and takes the values its theme sets;
//! and the layout of the themed toolbar. Ported with the documents from the tests of the WinUI
//! theme (`src/FerroUI.Themes.WinUI/tests/control_templates.rs`).

use ferroui_base::controls::{NameScopeRef, ResourceKey};
use ferroui_base::layout::Layoutable;
use ferroui_base::media::IBrush;
use ferroui_base::metadata::{from_markup_value, MarkupAttribute, MarkupAttributeValue};
use ferroui_base::styling::{ControlTheme, IStyle, Setter, SetterValue, StyleBase};
use ferroui_base::{CornerRadius, FerroObject, Ref, Thickness, TypeInfo};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_controls::{Application, Control, Window};
use ferroui_markup_xaml::styling::StyleInclude;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_themes_winui::WinUITheme;
use std::cell::RefCell;
use std::rc::Rc;

/// An application with the WinUI theme and the WinUI styles of the inking controls.
fn start_themed_application() -> UnitTestApplicationScope {
    crate::register_types();
    let services = TestServices::styled_window()
        .with_global_clock(Rc::new(MockGlobalClock::new()))
        .with_theme(|| WinUITheme::new().as_style());
    let scope = UnitTestApplication::start(services);
    FerroRuntimeXamlLoader::register();
    let styles = StyleInclude::new(None);
    styles.set_source(Some(Uri::new("ferres://FerroUI.Controls.Inking/Themes/WinUI.xaml", UriKind::Absolute).expect("a URI")));
    Application::current().expect("an application").styles().add(styles as Rc<dyn IStyle>);
    scope
}

/// The global clock of the themed application: the transitions of the control themes need one.
/// It never pulses, so a transition keeps its start value.
pub struct MockGlobalClock {
    subject: ferroui_base::reactive::LightweightSubject<ferroui_base::animation::TimeSpan>,
    play_state: std::cell::Cell<ferroui_base::animation::PlayState>,
}

impl MockGlobalClock {
    pub fn new() -> Self {
        Self {
            subject: ferroui_base::reactive::LightweightSubject::new(),
            play_state: std::cell::Cell::new(ferroui_base::animation::PlayState::Run),
        }
    }
}

impl ferroui_base::reactive::IObservable<ferroui_base::animation::TimeSpan> for MockGlobalClock {
    fn subscribe(
        &self,
        observer: Rc<dyn ferroui_base::reactive::IObserver<ferroui_base::animation::TimeSpan>>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl ferroui_base::animation::IClock for MockGlobalClock {
    fn play_state(&self) -> ferroui_base::animation::PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: ferroui_base::animation::PlayState) {
        self.play_state.set(value)
    }
}

impl ferroui_base::animation::IGlobalClock for MockGlobalClock {}

/// A style (when it is a style and not a collection of styles) and its
/// nested styles (`StyleExtensions.EnumerateStyles`).
pub fn enumerate_styles(style: &Rc<dyn IStyle>) -> Vec<Ref<StyleBase>> {
    let mut styles = Vec::new();
    enumerate_styles_into(style, &mut styles);
    styles
}
fn enumerate_styles_into(style: &Rc<dyn IStyle>, styles: &mut Vec<Ref<StyleBase>>) {
    if let Some(style_base) = style.as_object().and_then(|o| o.downcast_ref::<StyleBase>()) {
        styles.push(style_base.to_ref());
    }
    for child in style.children().iter() {
        enumerate_styles_into(child, styles);
    }
}

/// The template parts a class declares itself (`inherit: false`) and the
/// required ones of the classes it derives from.
pub fn requested_parts(target_type: &'static TypeInfo) -> Vec<&'static MarkupAttribute> {
    fn parts(type_: &'static TypeInfo) -> impl Iterator<Item = &'static MarkupAttribute> {
        type_.markup().into_iter().flat_map(|markup| markup.attributes.iter()).filter(|a| a.name == "TemplatePart")
    }
    fn part_name(attribute: &MarkupAttribute) -> &'static str {
        match attribute.arguments.first() {
            Some(MarkupAttributeValue::Str(name)) => name,
            _ => panic!("a template part without a name"),
        }
    }

    let mut requested: Vec<&'static MarkupAttribute> = parts(target_type).collect();
    let mut base = target_type.base_type();
    while let Some(type_) = base {
        for part in parts(type_) {
            let required = matches!(part.property("IsRequired"), Some(MarkupAttributeValue::Bool(true)));
            if required && !requested.iter().any(|known| part_name(known) == part_name(part)) {
                requested.push(part);
            }
        }
        base = type_.base_type();
    }
    requested
}

/// Whether the class of `object`, or a class it derives from, is the class
/// whose handle type is named `handle_type` (`..::Ref<module::Class>`). A
/// part declared with a contract instead of a class is accepted as it is.
pub fn is_instance_of(object: &Ref<FerroObject>, handle_type: &str) -> bool {
    if !handle_type.contains("Ref<") {
        return true;
    }
    let mut current = Some(object.get_type());
    while let Some(type_) = current {
        if handle_type.ends_with(&format!("Ref<{}::{}>", type_.module_path(), type_.name())) {
            return true;
        }
        current = type_.base_type();
    }
    false
}

fn control_type(name: &str) -> &'static TypeInfo {
    let mut found = TypeInfo::registered_types().into_iter().filter(|type_| {
        type_.name() == name
            && (type_.module_path().starts_with("ferroui_controls_inking"))
    });
    let type_ = found.next().unwrap_or_else(|| panic!("{name} is not a class of the inking crate"));
    assert!(found.next().is_none(), "{name} is ambiguous");
    type_
}

/// The control theme the application has for `type_`.
fn control_theme(type_: &'static TypeInfo) -> Ref<ControlTheme> {
    let application = Application::current().expect("an application");
    let theme = application.try_get_resource(&ResourceKey::Type(type_), None).flatten();
    from_markup_value::<Ref<ControlTheme>>(&theme).unwrap_or_else(|| panic!("the theme has no control theme for {}", type_.name()))
}

/// The parts of `type_` the theme must define.
fn expected_parts(type_: &'static TypeInfo) -> Vec<(&'static str, Option<&'static str>)> {
    // AutoCompleteBox has the optional part PART_SelectionAdapter, which no template of FerroUI's
    // themes defines (as the tests of the Fluent theme skip it).
    let skipped: &[&str] = if type_.name() == "AutoCompleteBox" { &["PART_SelectionAdapter"] } else { &[] };
    requested_parts(type_)
        .into_iter()
        .filter_map(|part| {
            let Some(MarkupAttributeValue::Str(name)) = part.arguments.first() else { panic!("a part without a name") };
            let type_name = match part.arguments.get(1) {
                Some(MarkupAttributeValue::Type(part_type)) => Some(part_type().name()),
                _ => None,
            };
            (!skipped.contains(name)).then_some((*name, type_name))
        })
        .collect()
}

/// The setters of a control theme and of the themes it is based on, the theme's own first: the
/// first setter of a property is the one in effect.
fn setters_of_chain(theme: &Ref<ControlTheme>) -> Vec<(&'static str, SetterValue)> {
    let mut setters = Vec::new();
    let mut current = Some(theme.clone());
    while let Some(theme) = current {
        for setter in theme.setters().snapshot().iter() {
            let Some(setter) = setter.as_any().and_then(|any| any.downcast_ref::<Setter>()) else { continue };
            let (Some(property), Some(value)) = (setter.property(), setter.value()) else { continue };
            if !setters.iter().any(|(name, _): &(&'static str, SetterValue)| *name == property.name()) {
                setters.push((property.name(), value));
            }
        }
        current = theme.based_on();
    }
    setters
}

/// The properties the nested styles of a control theme and of its bases set.
fn set_by_nested_styles(theme: &Ref<ControlTheme>) -> Vec<&'static str> {
    let mut names = Vec::new();
    let mut current = Some(theme.clone());
    while let Some(theme) = current {
        names.extend(
            enumerate_styles(&theme.clone().into())
                .iter()
                .skip(1)
                .flat_map(|style| style.setters().snapshot().iter().cloned().collect::<Vec<_>>())
                .filter_map(|setter| setter.as_any()?.downcast_ref::<Setter>()?.property().map(|property| property.name())),
        );
        current = theme.based_on();
    }
    names
}

fn check_control(name: &str) {
    let _app = start_themed_application();
    let type_ = control_type(name);
    let theme = control_theme(type_);

    let instance = type_.create_instance().unwrap_or_else(|| panic!("{name} has no default constructor"));
    let control = instance.cast::<Control>().unwrap_or_else(|| panic!("{name} is not a control"));
    let templated = control.cast::<TemplatedControl>();

    let name_scope: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    if let Some(templated) = &templated {
        let name_scope = name_scope.clone();
        let _ = templated.template_applied(move |_, e| *name_scope.borrow_mut() = Some(e.name_scope().clone()));
    }

    let window = Window::new();
    window.set_content(Some(Control::boxed(&control)));
    window.show();

    // The control theme of the theme is the one the control uses.
    assert!(theme.target_type().is_some_and(|target| std::ptr::eq(target, type_)));

    let setters = setters_of_chain(&theme);
    let setter = |property: &str| setters.iter().find(|(name, _)| *name == property).map(|(_, value)| value.clone());

    // The template of the theme has been applied (for ScrollBar, the template of the nested style
    // of its orientation).
    if setter("Template").is_some() || name == "ScrollBar" {
        let templated = templated.as_ref().unwrap_or_else(|| panic!("{name} has a template but is not templated"));
        assert!(templated.template().is_some(), "{name}: no template");
        assert_eq!(1, control.visual_children_count(), "{name}: the template was not applied");
        assert!(name_scope.borrow().is_some(), "{name}: TemplateApplied was not raised");
    }

    // The template parts.
    for (part, part_type) in expected_parts(type_) {
        let name_scope = name_scope.borrow();
        let name_scope = name_scope.as_ref().unwrap_or_else(|| panic!("{name}: no template name scope"));
        let found = name_scope.find(part).unwrap_or_else(|| panic!("{name}: the part {part} is not in the template"));
        if let Some(part_type) = part_type {
            assert!(
                is_instance_of(&found, part_type),
                "{name}: the part {part} is a {}, not a {part_type}",
                found.get_type().name()
            );
        }
    }

    // The setters of the theme.
    let Some(templated) = templated else { return };
    let brushes: [(&str, Option<Rc<dyn IBrush>>); 3] = [
        ("Background", templated.background()),
        ("BorderBrush", templated.border_brush()),
        ("Foreground", templated.foreground()),
    ];
    for (property, brush) in brushes {
        if matches!(setter(property), Some(SetterValue::BindingBase(_))) {
            assert!(brush.is_some(), "{name}: {property} is set by the theme from a resource but is not resolved");
        }
    }
    let nested = set_by_nested_styles(&theme);
    let value_of = |property: &str| match setter(property) {
        Some(SetterValue::Value(value)) if !nested.contains(&property) => Some(Some(value)),
        _ => None,
    };
    if let Some(expected) = value_of("Padding").and_then(|value| from_markup_value::<Thickness>(&value)) {
        assert_eq!(expected, templated.padding(), "{name}: Padding");
    }
    if let Some(expected) = value_of("BorderThickness").and_then(|value| from_markup_value::<Thickness>(&value)) {
        assert_eq!(expected, templated.border_thickness(), "{name}: BorderThickness");
    }
    if let Some(expected) = value_of("CornerRadius").and_then(|value| from_markup_value::<CornerRadius>(&value)) {
        assert_eq!(expected, templated.corner_radius(), "{name}: CornerRadius");
    }
    let layoutable: &Layoutable = &templated;
    if let Some(expected) = value_of("MinWidth").and_then(|value| from_markup_value::<f64>(&value)) {
        assert_eq!(expected, layoutable.min_width(), "{name}: MinWidth");
    }
    if let Some(expected) = value_of("MinHeight").and_then(|value| from_markup_value::<f64>(&value)) {
        assert_eq!(expected, layoutable.min_height(), "{name}: MinHeight");
    }
}

macro_rules! control_tests {
    ($($test:ident => $name:literal;)*) => {
        $(
            #[test]
            fn $test() {
                check_control($name);
            }
        )*
    };
}

control_tests! {
    ink_toolbar => "InkToolbar";
    ink_toolbar_tool_button => "InkToolbarToolButton";
    ink_toolbar_pen_button => "InkToolbarPenButton";
    ink_toolbar_ballpoint_pen_button => "InkToolbarBallpointPenButton";
    ink_toolbar_pencil_button => "InkToolbarPencilButton";
    ink_toolbar_highlighter_button => "InkToolbarHighlighterButton";
    ink_toolbar_custom_pen_button => "InkToolbarCustomPenButton";
    ink_toolbar_eraser_button => "InkToolbarEraserButton";
    ink_toolbar_custom_tool_button => "InkToolbarCustomToolButton";
    ink_toolbar_toggle_button => "InkToolbarToggleButton";
    ink_toolbar_custom_toggle_button => "InkToolbarCustomToggleButton";
    ink_toolbar_ruler_button => "InkToolbarRulerButton";
    ink_toolbar_menu_button => "InkToolbarMenuButton";
    ink_toolbar_stencil_button => "InkToolbarStencilButton";
    ink_toolbar_flyout_item => "InkToolbarFlyoutItem";
    ink_toolbar_stroke_width_slider => "InkToolbarStrokeWidthSlider";
    ink_toolbar_pen_configuration_control => "InkToolbarPenConfigurationControl";
}

/// The panel-backed ink controls retain the source Control border/padding contract and
/// account for it once, even when their child templates and margins come from this theme.
#[test]
fn ink_toolbar_border_and_padding_offset_themed_children() {
    use crate::{InkToolbar, InkToolbarCustomToolButton, InkToolbarInitialControls,
        InkToolbarPenConfigurationControl};
    use ferroui_controls::StackPanel;
    let _app = start_themed_application();
    let host = StackPanel::new();
    let toolbar = InkToolbar::new();
    toolbar.set_initial_controls(InkToolbarInitialControls::None);
    toolbar.set_width(200.0);
    toolbar.set_height(80.0);
    toolbar.set_padding(Thickness::new(6.0, 8.0, 10.0, 12.0));
    toolbar.set_border_thickness(Thickness::new(1.0, 2.0, 3.0, 4.0));
    toolbar.set_corner_radius(CornerRadius::uniform(7.0));
    let tool = InkToolbarCustomToolButton::new();
    tool.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Top);
    toolbar.children().add(&tool);
    host.children().add(&toolbar);
    let configuration = InkToolbarPenConfigurationControl::new();
    configuration.set_padding(toolbar.padding());
    configuration.set_border_thickness(toolbar.border_thickness());
    host.children().add(&configuration);
    let window = Window::new();
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    assert!(tool.template().is_some());
    assert_eq!(tool.bounds().x, 7.0 + tool.margin().left);
    assert_eq!(tool.bounds().y, 10.0 + tool.margin().top);
    let first = configuration.children().get(0);
    assert_eq!(first.bounds().x, 7.0 + first.margin().left);
    assert_eq!(first.bounds().y, 10.0 + first.margin().top);
    window.close();
}

/// The populated stencil is in the same realized toolbar as its pens and has an actual
/// vector glyph for both stencil kinds, preserving the source's blank-icon regression.
#[test]
fn ink_toolbar_populated_stencil_has_laid_out_ruler_and_protractor_geometry() {
    use ferroui_base::Visual;
    use ferroui_controls::shapes::Path;
    use crate::{InkToolbar, InkToolbarBallpointPenButton, InkToolbarStencilButton,
        InkToolbarStencilKind};
    let _app = start_themed_application();
    let toolbar = InkToolbar::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&toolbar)));
    window.show();
    window.update_layout();
    let visual: &Visual = &toolbar;
    assert!(visual.get_visual_descendants().any(|child| child.cast::<InkToolbarBallpointPenButton>().is_some()));
    let stencil = visual.get_visual_descendants()
        .find_map(|child| child.cast::<InkToolbarStencilButton>())
        .expect("the stencil is realized beside the pens");
    assert!(stencil.bounds().width > 0.0 && stencil.bounds().height > 0.0);
    let visual: &Visual = &stencil;
    let glyph = visual.get_visual_descendants().filter_map(|child| child.cast::<Path>())
        .find(|path| path.name().as_deref() == Some("ContentGlyph"))
        .expect("the stencil's content glyph");
    assert!(glyph.bounds().width > 0.0 && glyph.bounds().height > 0.0);
    let ruler = glyph.data().expect("the ruler geometry");
    assert!(ruler.bounds().width > 0.0 && ruler.bounds().height > 0.0);
    stencil.set_selected_stencil(InkToolbarStencilKind::Protractor);
    window.update_layout();
    let protractor = glyph.data().expect("the protractor geometry");
    assert_ne!(ruler, protractor);
    assert!(protractor.bounds().width > 0.0 && protractor.bounds().height > 0.0);
    window.close();
}

/// Template-local defaults must not mask the selected/placement styles.
#[test]
fn ink_toolbar_extension_glyph_tracks_selection_and_every_flyout_edge() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_base::media::Geometry;
    use ferroui_base::Visual;
    use ferroui_controls::shapes::Path;
    use crate::{InkToolbar, InkToolbarButtonFlyoutPlacement as Placement, InkToolbarTool};
    let _app = start_themed_application();
    let toolbar = InkToolbar::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&toolbar)));
    window.show();
    window.update_layout();
    let pen = toolbar.get_tool_button(InkToolbarTool::BallpointPen).unwrap();
    let visual: &Visual = &pen;
    let glyph = visual.get_visual_descendants().filter_map(|child| child.cast::<Path>())
        .find(|path| path.name().as_deref() == Some("ExtensionGlyph"))
        .expect("the pen's configuration glyph");
    assert_eq!(glyph.opacity(), 1.0);
    assert!(glyph.data().is_some(), "the selected pen has a real configuration arrow");
    assert!(glyph.bounds().width > 0.0 && glyph.bounds().height > 0.0);
    let outline = visual.get_visual_descendants().filter_map(|child| child.cast::<Path>())
        .find(|path| path.name().as_deref() == Some("ContentGlyph"))
        .expect("the pen outline");
    let fill = visual.get_visual_descendants().filter_map(|child| child.cast::<Path>())
        .find(|path| path.name().as_deref() == Some("ContentFillGlyph"))
        .expect("the selected ink color glyph");
    assert!(outline.fill().is_some(), "presenter content resolves the button foreground");
    let selected_brush = pen.cast::<crate::InkToolbarPenButton>().unwrap()
        .selected_brush().expect("the selected ink brush");
    assert!(fill.fill().is_some_and(|brush| Rc::ptr_eq(&brush, &selected_brush)),
        "presenter content resolves the selected ink color");
    let flyout = pen.flyout().unwrap();
    let activate = || {
        let mut key = KeyEventArgs::new();
        key.key = Key::Enter;
        key.set_routed_event(Some(InputElement::key_down_event()));
        pen.raise_event(&key);
        assert!(key.handled());
    };
    activate();
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(flyout.is_open());
    assert!(pen.classes().contains(":flyout-open"));
    assert_eq!(glyph.opacity(), 0.0);
    activate();
    assert!(!flyout.is_open());
    assert!(!pen.classes().contains(":flyout-open"));
    assert_eq!(glyph.opacity(), 1.0);
    for (placement, resource, horizontal, vertical, width, height) in [
        (Placement::Top, "InkToolbarArrowUp", HorizontalAlignment::Center, VerticalAlignment::Top, 8.0, 5.0),
        (Placement::Left, "InkToolbarArrowLeft", HorizontalAlignment::Left, VerticalAlignment::Center, 5.0, 8.0),
        (Placement::Right, "InkToolbarArrowRight", HorizontalAlignment::Right, VerticalAlignment::Center, 5.0, 8.0),
        (Placement::Bottom, "InkToolbarArrowDown", HorizontalAlignment::Center, VerticalAlignment::Bottom, 8.0, 5.0),
        (Placement::Auto, "InkToolbarArrowDown", HorizontalAlignment::Center, VerticalAlignment::Bottom, 8.0, 5.0),
    ] {
        toolbar.set_button_flyout_placement(placement);
        window.update_layout();
        let value = Application::current().unwrap().try_get_resource(&ResourceKey::from(resource), None).flatten();
        let expected = from_markup_value::<Ref<Geometry>>(&value).expect("the arrow resource");
        assert_eq!(glyph.data(), Some(expected));
        assert_eq!(glyph.horizontal_alignment(), horizontal);
        assert_eq!(glyph.vertical_alignment(), vertical);
        assert_eq!(glyph.width(), width);
        assert_eq!(glyph.height(), height);
    }
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Pencil));
    assert_eq!(glyph.opacity(), 0.0);
    window.close();
}
