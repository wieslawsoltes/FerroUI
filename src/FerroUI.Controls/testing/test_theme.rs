use crate::embedding::EmbeddableControlRoot;
use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::{Border, ContentControl, Panel};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::TemplateBinding;
use ferroui_base::styling::{styles_as_style, ControlTheme, IStyle, Setter, Styles};
use ferroui_base::{FerroProperty, ObjectType, Ref};
use std::rc::Rc;

/// The control template the test theme gives to top-levels: the
/// transparency fallback border, a background border and the content
/// presenter (the template of the reference simple theme, minus the parts
/// that are not ported), as used for windows.
pub fn top_level_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let panel = Panel::new();

        let transparency_fallback = Border::new();
        transparency_fallback.set_name(Some("PART_TransparencyFallback".to_string()));
        transparency_fallback.set_is_hit_test_visible(false);
        panel.children().add(transparency_fallback.clone().register_in_name_scope(&**ns));

        let background = Border::new();
        background.set_is_hit_test_visible(false);
        background.bind_binding(
            Border::background_property().as_property(),
            &TemplateBinding::new(TemplatedControl::background_property().as_property()),
        );
        panel.children().add(background);

        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            presenter.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(
            ContentPresenter::horizontal_content_alignment_property().as_property(),
            ContentControl::horizontal_content_alignment_property().as_property(),
        );
        bind(
            ContentPresenter::vertical_content_alignment_property().as_property(),
            ContentControl::vertical_content_alignment_property().as_property(),
        );
        bind(ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        bind(
            ContentPresenter::content_template_property().as_property(),
            ContentControl::content_template_property().as_property(),
        );
        bind(ferroui_base::layout::Layoutable::margin_property().as_property(), TemplatedControl::padding_property().as_property());
        let layer_manager = crate::primitives::VisualLayerManager::new();
        layer_manager.set_name(Some("PART_VisualLayerManager".to_string()));
        layer_manager.set_child(presenter.register_in_name_scope(&**ns));
        panel.children().add(layer_manager.register_in_name_scope(&**ns));

        panel.upcast()
    })
}

/// The control template the test theme gives to the embeddable control
/// root, nested as in the reference simple theme: the layer manager sits
/// inside the background border.
pub fn embeddable_control_root_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let panel = Panel::new();

        let transparency_fallback = Border::new();
        transparency_fallback.set_name(Some("PART_TransparencyFallback".to_string()));
        transparency_fallback.set_is_hit_test_visible(false);
        panel.children().add(transparency_fallback.register_in_name_scope(&**ns));

        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            presenter.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(ferroui_base::layout::Layoutable::margin_property().as_property(), TemplatedControl::padding_property().as_property());
        bind(ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        bind(
            ContentPresenter::content_template_property().as_property(),
            ContentControl::content_template_property().as_property(),
        );

        let layer_manager = crate::primitives::VisualLayerManager::new();
        layer_manager.set_name(Some("PART_VisualLayerManager".to_string()));
        layer_manager.set_child(presenter.register_in_name_scope(&**ns));

        let background = Border::new();
        background.bind_binding(
            Border::background_property().as_property(),
            &TemplateBinding::new(TemplatedControl::background_property().as_property()),
        );
        background.set_child(layer_manager.register_in_name_scope(&**ns));
        panel.children().add(background);

        panel.upcast()
    })
}

/// The layout transform control of the popup host templates, with its
/// layout transform bound to the `Transform` property of the host, around
/// `child`.
fn popup_host_layout_transform(
    transform_property: &'static FerroProperty,
    child: Ref<crate::Control>,
) -> Ref<crate::LayoutTransformControl> {
    use ferroui_base::data::converters::{FuncValueConverter, IValueConverter};
    use ferroui_base::media::{ITransform, Transform};

    let layout_transform = crate::LayoutTransformControl::new();
    let converter: Rc<dyn IValueConverter> = Rc::new(FuncValueConverter::new(
        |transform: Option<Ref<Transform>>| -> Option<Rc<dyn ITransform>> { transform.map(Into::into) },
    ));
    let binding = TemplateBinding::new(transform_property);
    binding.set_converter(Some(converter));
    layout_transform
        .bind_binding(crate::LayoutTransformControl::layout_transform_property().as_property(), &binding);
    layout_transform.set_child(child);
    layout_transform
}

/// The layer manager of the popup host templates around the content
/// presenter.
fn popup_host_layer_manager(ns: &ferroui_base::controls::NameScopeRef) -> Ref<crate::primitives::VisualLayerManager> {
    let presenter = ContentPresenter::new();
    presenter.set_name(Some("PART_ContentPresenter".to_string()));
    let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
        presenter.bind_binding(target, &TemplateBinding::new(source));
    };
    bind(ContentPresenter::padding_property().as_property(), TemplatedControl::padding_property().as_property());
    bind(ContentPresenter::background_property().as_property(), TemplatedControl::background_property().as_property());
    bind(ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
    bind(
        ContentPresenter::content_template_property().as_property(),
        ContentControl::content_template_property().as_property(),
    );

    let layer_manager = crate::primitives::VisualLayerManager::new();
    layer_manager.set_name(Some("PART_VisualLayerManager".to_string()));
    layer_manager.set_child(presenter.register_in_name_scope(&**ns));
    layer_manager.register_in_name_scope(&**ns)
}

/// The control template the test theme gives to popup roots (the template
/// of the reference simple theme).
pub fn popup_root_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let panel = Panel::new();

        let transparency_fallback = Border::new();
        transparency_fallback.set_name(Some("PART_TransparencyFallback".to_string()));
        transparency_fallback.set_is_hit_test_visible(false);
        panel.children().add(transparency_fallback.register_in_name_scope(&**ns));
        panel.children().add(popup_host_layer_manager(ns));

        popup_host_layout_transform(crate::primitives::PopupRoot::transform_property().as_property(), panel.upcast())
            .upcast()
    })
}

/// The control template the test theme gives to overlay popup hosts (the
/// template of the reference simple theme).
pub fn overlay_popup_host_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        popup_host_layout_transform(
            crate::primitives::OverlayPopupHost::transform_property().as_property(),
            popup_host_layer_manager(ns).upcast(),
        )
        .upcast()
    })
}

/// Adds the control themes of the popup hosts.
fn add_popup_host_themes(styles: &Ref<Styles>) {
    use crate::primitives::{OverlayPopupHost, PopupRoot};
    use crate::{TopLevel, WindowTransparencyLevel, WindowTransparencyLevelCollection};

    let popup_root_theme = ControlTheme::with_setters(
        PopupRoot::TYPE,
        [
            Setter::new(TemplatedControl::background_property(), None),
            Setter::new(
                TopLevel::transparency_level_hint_property(),
                WindowTransparencyLevelCollection::new(vec![WindowTransparencyLevel::transparent()]),
            ),
            Setter::new(TemplatedControl::template_property(), Some(popup_root_template())),
        ],
    );
    styles.resources().add_value(ResourceKey::Type(PopupRoot::TYPE), popup_root_theme);

    add_template_theme::<OverlayPopupHost>(styles, overlay_popup_host_template());
}

/// A decorations template built by a function (the code counterpart of the
/// markup decorations template of the reference).
pub struct FuncWindowDrawnDecorationsTemplate {
    build: Box<dyn Fn() -> crate::templates::TemplateResult<Ref<crate::chrome::WindowDrawnDecorationsContent>>>,
}

impl FuncWindowDrawnDecorationsTemplate {
    /// Creates a decorations template from the function that builds the
    /// content and its name scope.
    pub fn new(
        build: impl Fn() -> crate::templates::TemplateResult<Ref<crate::chrome::WindowDrawnDecorationsContent>> + 'static,
    ) -> Rc<dyn crate::chrome::IWindowDrawnDecorationsTemplate> {
        Rc::new(Self { build: Box::new(build) })
    }

    /// Creates a decorations template that always answers `content`.
    pub fn from_content(
        content: &Ref<crate::chrome::WindowDrawnDecorationsContent>,
    ) -> Rc<dyn crate::chrome::IWindowDrawnDecorationsTemplate> {
        let content = content.clone();
        Self::new(move || {
            crate::templates::TemplateResult::new(
                content.clone(),
                ferroui_base::controls::NameScopeRef::new(ferroui_base::controls::NameScope::new()),
            )
        })
    }
}

impl ferroui_base::styling::ITemplate for FuncWindowDrawnDecorationsTemplate {
    fn build(&self) -> ferroui_base::BoxedValue {
        Rc::new((self.build)().deconstruct().0)
    }
}

impl crate::chrome::IWindowDrawnDecorationsTemplate for FuncWindowDrawnDecorationsTemplate {
    fn build_typed(&self) -> crate::templates::TemplateResult<Ref<crate::chrome::WindowDrawnDecorationsContent>> {
        (self.build)()
    }
}

/// A control theme for the drawn decorations whose only setter is the
/// template.
pub fn decorations_template_theme(
    template: Rc<dyn crate::chrome::IWindowDrawnDecorationsTemplate>,
) -> Ref<ControlTheme> {
    use crate::chrome::WindowDrawnDecorations;

    ControlTheme::with_setters(
        WindowDrawnDecorations::TYPE,
        [Setter::new(WindowDrawnDecorations::template_property(), Some(template))],
    )
}

/// The control theme of the caption buttons of the drawn decorations (the
/// caption button theme of the reference simple theme, minus the resource
/// brushes and the pointer-over and pressed backgrounds that use them).
fn caption_button_theme() -> Ref<ControlTheme> {
    use crate::chrome::WindowDecorationProperties;
    use ferroui_base::input::WindowDecorationsElementRole;
    use ferroui_base::layout::{Layoutable, VerticalAlignment};
    use ferroui_base::media::{Brushes, IBrush};

    let template = FuncControlTemplate::new(|_, ns| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let transparent: Option<Rc<dyn IBrush>> = Some(Brushes::transparent());
        presenter.set_background(transparent);
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        presenter.register_in_name_scope(&**ns).upcast()
    });

    ControlTheme::with_setters(
        crate::Button::TYPE,
        [
            Setter::new(Layoutable::width_property(), 45.0),
            Setter::new(Layoutable::height_property(), 30.0),
            Setter::new(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch),
            Setter::new(
                WindowDecorationProperties::element_role_property(),
                WindowDecorationsElementRole::DecorationsElement,
            ),
            Setter::new(TemplatedControl::template_property(), Some(template)),
        ],
    )
}

/// The decorations template of the test theme: the template of the
/// reference simple theme, minus the parts that are not ported (the glyphs
/// of the caption buttons, the title texts, the box shadow and the resource
/// brushes).
pub fn window_drawn_decorations_template() -> Rc<dyn crate::chrome::IWindowDrawnDecorationsTemplate> {
    use crate::chrome::{WindowDecorationProperties, WindowDrawnDecorations, WindowDrawnDecorationsContent};
    use crate::StackPanel;
    use ferroui_base::controls::{NameScope, NameScopeRef};
    use ferroui_base::input::WindowDecorationsElementRole;
    use ferroui_base::layout::{HorizontalAlignment, Layoutable, Orientation, VerticalAlignment};
    use ferroui_base::media::{Brushes, IBrush};
    use ferroui_base::Visual;

    FuncWindowDrawnDecorationsTemplate::new(|| {
        let ns = NameScopeRef::new(NameScope::new());
        let button_theme = caption_button_theme();
        let caption_button = |name: &str, role: WindowDecorationsElementRole| {
            let button = crate::Button::new();
            button.set_name(Some(name.to_string()));
            button.set_theme(button_theme.clone());
            WindowDecorationProperties::set_element_role(&button, role);
            button.register_in_name_scope(&*ns)
        };
        let transparent = || -> Option<Rc<dyn IBrush>> { Some(Brushes::transparent()) };
        let template_binding = |source: &'static FerroProperty| TemplateBinding::new(source);
        let title_bar_height = WindowDrawnDecorations::title_bar_height_property().as_property();
        let has_title_bar = WindowDrawnDecorations::has_title_bar_property().as_property();

        // Underlay
        let underlay = Panel::new();
        underlay.set_name(Some("PART_UnderlayWrapper".to_string()));

        let window_border = Border::new();
        window_border.set_name(Some("PART_WindowBorder".to_string()));
        window_border.set_background(transparent());
        window_border.bind_binding(
            Border::border_thickness_property().as_property(),
            &template_binding(WindowDrawnDecorations::frame_thickness_property().as_property()),
        );
        window_border.set_is_hit_test_visible(false);
        underlay.children().add(window_border.register_in_name_scope(&*ns));

        // Titlebar: background and drag area live in underlay
        let title_bar = Panel::new();
        title_bar.set_name(Some(WindowDrawnDecorations::PART_TITLE_BAR.to_string()));
        title_bar.set_vertical_alignment(VerticalAlignment::Top);
        title_bar.bind_binding(Layoutable::height_property().as_property(), &template_binding(title_bar_height));
        title_bar.set_background(transparent());
        title_bar.bind_binding(Visual::is_visible_property().as_property(), &template_binding(has_title_bar));
        WindowDecorationProperties::set_element_role(&title_bar, WindowDecorationsElementRole::TitleBar);
        underlay.children().add(title_bar.register_in_name_scope(&*ns));

        // Overlay
        let overlay = Panel::new();
        overlay.set_name(Some("PART_OverlayWrapper".to_string()));

        let title_text_panel = Panel::new();
        title_text_panel.set_name(Some("PART_TitleTextPanel".to_string()));
        title_text_panel.set_vertical_alignment(VerticalAlignment::Top);
        title_text_panel
            .bind_binding(Layoutable::height_property().as_property(), &template_binding(title_bar_height));
        title_text_panel.set_is_hit_test_visible(false);
        title_text_panel.bind_binding(Visual::is_visible_property().as_property(), &template_binding(has_title_bar));
        overlay.children().add(title_text_panel.register_in_name_scope(&*ns));

        let overlay_panel = StackPanel::new();
        overlay_panel.set_name(Some("PART_OverlayPanel".to_string()));
        overlay_panel.set_vertical_alignment(VerticalAlignment::Top);
        overlay_panel.set_horizontal_alignment(HorizontalAlignment::Right);
        overlay_panel.bind_binding(Layoutable::height_property().as_property(), &template_binding(title_bar_height));
        overlay_panel.set_orientation(Orientation::Horizontal);
        overlay_panel.set_spacing(2.0);
        overlay_panel.bind_binding(Visual::is_visible_property().as_property(), &template_binding(has_title_bar));
        overlay_panel.children().add(caption_button(
            WindowDrawnDecorations::PART_FULL_SCREEN_BUTTON,
            WindowDecorationsElementRole::FullScreenButton,
        ));
        overlay_panel.children().add(caption_button(
            WindowDrawnDecorations::PART_MINIMIZE_BUTTON,
            WindowDecorationsElementRole::MinimizeButton,
        ));
        overlay_panel.children().add(caption_button(
            WindowDrawnDecorations::PART_MAXIMIZE_BUTTON,
            WindowDecorationsElementRole::MaximizeButton,
        ));
        overlay_panel
            .children()
            .add(caption_button(WindowDrawnDecorations::PART_CLOSE_BUTTON, WindowDecorationsElementRole::CloseButton));
        overlay.children().add(overlay_panel.register_in_name_scope(&*ns));

        // Fullscreen popover: shown on hover at top edge in fullscreen mode
        let popover = Panel::new();
        popover.set_height(30.0);
        popover.set_vertical_alignment(VerticalAlignment::Top);
        popover.set_background(transparent());
        WindowDecorationProperties::set_element_role(&popover, WindowDecorationsElementRole::DecorationsElement);

        let popover_panel = StackPanel::new();
        popover_panel.set_horizontal_alignment(HorizontalAlignment::Right);
        popover_panel.set_orientation(Orientation::Horizontal);
        popover_panel.set_spacing(2.0);
        popover_panel.children().add(caption_button(
            WindowDrawnDecorations::PART_POPOVER_FULL_SCREEN_BUTTON,
            WindowDecorationsElementRole::FullScreenButton,
        ));
        popover_panel.children().add(caption_button(
            WindowDrawnDecorations::PART_POPOVER_CLOSE_BUTTON,
            WindowDecorationsElementRole::CloseButton,
        ));
        popover.children().add(popover_panel);

        let content = WindowDrawnDecorationsContent::new();
        content.set_underlay(underlay.register_in_name_scope(&*ns).upcast::<crate::Control>());
        content.set_overlay(overlay.register_in_name_scope(&*ns).upcast::<crate::Control>());
        content.set_fullscreen_popover(popover.upcast::<crate::Control>());

        crate::templates::TemplateResult::new(content, ns)
    })
}

/// Adds the control theme of the drawn decorations: the theme of the
/// reference simple theme over [`window_drawn_decorations_template`].
fn add_window_drawn_decorations_theme(styles: &Ref<Styles>) {
    use crate::chrome::WindowDrawnDecorations;
    use crate::StackPanel;
    use ferroui_base::data::BindingBase;
    use ferroui_base::layout::Layoutable;
    use ferroui_base::styling::{Selector, Selectors, Style};
    use ferroui_base::{Thickness, Visual};

    let theme = ControlTheme::with_setters(
        WindowDrawnDecorations::TYPE,
        [
            Setter::new(WindowDrawnDecorations::default_title_bar_height_property(), 30.0),
            Setter::new(WindowDrawnDecorations::default_frame_thickness_property(), Thickness::uniform(1.0)),
            Setter::new(WindowDrawnDecorations::default_shadow_thickness_property(), Thickness::uniform(8.0)),
            Setter::new(WindowDrawnDecorations::template_property(), Some(window_drawn_decorations_template())),
        ],
    );

    let add = |selector: Selector, setter: Rc<Setter>| {
        theme.add_style(Style::with_setters(selector, [setter]));
    };
    let with_class = |name: &str| Selectors::nesting(None).class(name).template();
    let without_class = |name: &str| Selectors::nesting(None).not_with(|x| Selectors::class(x, name)).template();
    let shadow_margin = || {
        let binding: Rc<dyn BindingBase> =
            TemplateBinding::new(WindowDrawnDecorations::shadow_thickness_property().as_property());
        Setter::new_binding_base(Layoutable::margin_property().as_property(), binding)
    };
    let margin = |margin: Thickness| Setter::new(Layoutable::margin_property(), margin);
    let hidden = || Setter::new(Visual::is_visible_property(), false);

    // Shadow: inset border
    add(with_class(":has-shadow").of_type::<Panel>().name("PART_UnderlayWrapper"), shadow_margin());
    add(with_class(":has-shadow").of_type::<Panel>().name("PART_OverlayWrapper"), shadow_margin());

    // Border: inset titlebar and buttons inside frame
    add(
        with_class(":has-border").of_type::<Panel>().name("PART_TitleTextPanel"),
        margin(Thickness::new(1.0, 1.0, 1.0, 0.0)),
    );
    add(with_class(":has-border").of_type::<Panel>().name("PART_TitleBar"), margin(Thickness::new(1.0, 1.0, 1.0, 0.0)));
    add(
        with_class(":has-border").of_type::<StackPanel>().name("PART_OverlayPanel"),
        margin(Thickness::new(0.0, 1.0, 1.0, 0.0)),
    );

    // Disabled buttons
    add(
        Selectors::nesting(None).template().of_type::<crate::Button>().class(":disabled"),
        Setter::new(Visual::opacity_property(), 0.2),
    );

    // Hide title bar elements when the platform does not support the action
    // or when they aren't requested
    for (class, part) in [
        (":has-minimize", WindowDrawnDecorations::PART_MINIMIZE_BUTTON),
        (":has-maximize", WindowDrawnDecorations::PART_MAXIMIZE_BUTTON),
        (":has-fullscreen", WindowDrawnDecorations::PART_FULL_SCREEN_BUTTON),
        (":has-fullscreen", WindowDrawnDecorations::PART_POPOVER_FULL_SCREEN_BUTTON),
        (":has-close", WindowDrawnDecorations::PART_CLOSE_BUTTON),
        (":has-close", WindowDrawnDecorations::PART_POPOVER_CLOSE_BUTTON),
    ] {
        add(without_class(class).of_type::<crate::Button>().name(part), hidden());
    }
    add(without_class(":has-title").of_type::<Panel>().name("PART_TitleTextPanel"), hidden());

    // Fullscreen: hide overlay and titlebar (popover takes over)
    add(with_class(":fullscreen").of_type::<Panel>().name("PART_TitleTextPanel"), hidden());
    add(with_class(":fullscreen").of_type::<StackPanel>().name("PART_OverlayPanel"), hidden());
    add(with_class(":fullscreen").of_type::<Panel>().name("PART_TitleBar"), hidden());

    styles.resources().add_value(ResourceKey::Type(WindowDrawnDecorations::TYPE), theme);
}

/// Adds a control theme for `T` whose only setter is the template.
pub fn add_template_theme<T: ObjectType>(styles: &Ref<Styles>, template: Rc<dyn IControlTemplate>) {
    let theme = ControlTheme::with_setters(T::TYPE, [Setter::new(TemplatedControl::template_property(), Some(template))]);
    styles.resources().add_value(ResourceKey::Type(T::TYPE), theme);
}

/// Creates the theme of the windowing tests: control themes for the
/// top-level classes, standing in for the simple theme of the reference
/// test suite.
pub fn create_test_theme() -> Rc<dyn IStyle> {
    let styles = Styles::new();
    add_template_theme::<EmbeddableControlRoot>(&styles, embeddable_control_root_template());
    add_template_theme::<crate::Window>(&styles, top_level_template());
    add_popup_host_themes(&styles);
    add_window_drawn_decorations_theme(&styles);
    add_tool_tip_theme(&styles);
    add_flyout_presenter_theme(&styles);
    add_menu_themes(&styles);
    add_context_menu_theme(&styles);
    super::test_theme_split_view::add_split_view_themes(&styles);
    super::test_theme_command_bar::add_command_bar_themes(&styles);
    super::test_theme_notifications::add_notifications_themes(&styles);
    styles_as_style(&styles)
}

/// The control template the test theme gives to tooltips (the template of
/// the reference simple theme).
pub fn tool_tip_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            presenter.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(ContentPresenter::padding_property().as_property(), TemplatedControl::padding_property().as_property());
        bind(ContentPresenter::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(
            ContentPresenter::border_brush_property().as_property(),
            TemplatedControl::border_brush_property().as_property(),
        );
        bind(
            ContentPresenter::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        bind(
            ContentPresenter::content_template_property().as_property(),
            ContentControl::content_template_property().as_property(),
        );
        bind(
            ContentPresenter::corner_radius_property().as_property(),
            TemplatedControl::corner_radius_property().as_property(),
        );
        presenter.register_in_name_scope(&**ns).upcast()
    })
}

/// Adds the control theme of tooltips: the theme of the reference simple
/// theme, minus the setters whose values are theme resources (background,
/// border brush and border thickness).
fn add_tool_tip_theme(styles: &Ref<Styles>) {
    use crate::ToolTip;
    use ferroui_base::Thickness;

    let theme = ControlTheme::with_setters(
        ToolTip::TYPE,
        [
            Setter::new(TemplatedControl::padding_property(), Thickness::new(4.0, 2.0, 4.0, 2.0)),
            Setter::new(TemplatedControl::template_property(), Some(tool_tip_template())),
        ],
    );
    styles.resources().add_value(ResourceKey::Type(ToolTip::TYPE), theme);
}

/// The control template the test theme gives to flyout presenters: the
/// template of the reference simple theme, minus the scroll viewer between
/// the border and the content presenter (the test theme has no theme for
/// it).
pub fn flyout_presenter_template() -> Rc<dyn IControlTemplate> {
    use ferroui_base::layout::{HorizontalAlignment, Layoutable, VerticalAlignment};

    FuncControlTemplate::new(|_, ns| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            presenter.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(Layoutable::margin_property().as_property(), TemplatedControl::padding_property().as_property());
        bind(
            Layoutable::horizontal_alignment_property().as_property(),
            ContentControl::horizontal_content_alignment_property().as_property(),
        );
        bind(
            Layoutable::vertical_alignment_property().as_property(),
            ContentControl::vertical_content_alignment_property().as_property(),
        );
        presenter.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        presenter.set_vertical_content_alignment(VerticalAlignment::Stretch);
        bind(ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        bind(
            ContentPresenter::content_template_property().as_property(),
            ContentControl::content_template_property().as_property(),
        );

        let layout_root = Border::new();
        layout_root.set_name(Some("LayoutRoot".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            layout_root.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(Border::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(Border::border_brush_property().as_property(), TemplatedControl::border_brush_property().as_property());
        bind(
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(Border::corner_radius_property().as_property(), TemplatedControl::corner_radius_property().as_property());
        layout_root.set_child(presenter.register_in_name_scope(&**ns));

        layout_root.register_in_name_scope(&**ns).upcast()
    })
}

/// Adds the control theme of the flyout presenter: the theme of the
/// reference simple theme over [`flyout_presenter_template`], minus the
/// resource brushes and the scroll bar visibilities.
fn add_flyout_presenter_theme(styles: &Ref<Styles>) {
    use crate::FlyoutPresenter;
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_base::Thickness;

    let theme = ControlTheme::with_setters(
        FlyoutPresenter::TYPE,
        [
            Setter::new(ContentControl::horizontal_content_alignment_property(), HorizontalAlignment::Stretch),
            Setter::new(ContentControl::vertical_content_alignment_property(), VerticalAlignment::Stretch),
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(1.0)),
            Setter::new(TemplatedControl::padding_property(), Thickness::uniform(4.0)),
            Setter::new(TemplatedControl::template_property(), Some(flyout_presenter_template())),
        ],
    );
    styles.resources().add_value(ResourceKey::Type(FlyoutPresenter::TYPE), theme);
}

/// The control template the test theme gives to separators (the template of
/// the reference simple theme).
pub fn separator_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, _| {
        let border = Border::new();
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            border.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(Border::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(Border::border_brush_property().as_property(), TemplatedControl::border_brush_property().as_property());
        bind(
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(Border::corner_radius_property().as_property(), TemplatedControl::corner_radius_property().as_property());
        border.upcast()
    })
}

/// The root border of the menu item templates, with the bindings of the
/// reference simple theme.
fn menu_item_root_border() -> Ref<Border> {
    let root = Border::new();
    root.set_name(Some("root".to_string()));
    let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
        root.bind_binding(target, &TemplateBinding::new(source));
    };
    bind(Border::background_property().as_property(), TemplatedControl::background_property().as_property());
    bind(Border::border_brush_property().as_property(), TemplatedControl::border_brush_property().as_property());
    bind(Border::border_thickness_property().as_property(), TemplatedControl::border_thickness_property().as_property());
    bind(Border::corner_radius_property().as_property(), TemplatedControl::corner_radius_property().as_property());
    root
}

/// The header presenter and the popup with the items presenter that the
/// menu item templates of the reference simple theme share. The popup is
/// light-dismissable and placed as given.
fn menu_item_template_parts(
    ns: &ferroui_base::controls::NameScopeRef,
    is_light_dismiss_enabled: bool,
    placement: crate::PlacementMode,
) -> (Ref<ContentPresenter>, Ref<crate::primitives::Popup>) {
    use crate::presenters::ItemsPresenter;
    use crate::primitives::{HeaderedSelectingItemsControl, Popup};
    use crate::{Grid, ItemsControl, MenuItem};
    use ferroui_base::data::BindingMode;
    use ferroui_base::layout::Layoutable;
    use ferroui_base::Thickness;

    let header = ContentPresenter::new();
    header.set_name(Some("PART_HeaderPresenter".to_string()));
    let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
        header.bind_binding(target, &TemplateBinding::new(source));
    };
    bind(Layoutable::margin_property().as_property(), TemplatedControl::padding_property().as_property());
    bind(
        ContentPresenter::content_property().as_property(),
        HeaderedSelectingItemsControl::header_property().as_property(),
    );
    bind(
        ContentPresenter::content_template_property().as_property(),
        HeaderedSelectingItemsControl::header_template_property().as_property(),
    );

    let items_presenter = ItemsPresenter::new();
    items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
    items_presenter.set_margin(Thickness::uniform(2.0));
    Grid::set_is_shared_size_scope(&items_presenter, true);
    items_presenter.bind_binding(
        ItemsPresenter::items_panel_property().as_property(),
        &TemplateBinding::new(ItemsControl::items_panel_property().as_property()),
    );

    let popup_border = Border::new();
    popup_border.bind_binding(
        Border::border_thickness_property().as_property(),
        &TemplateBinding::new(TemplatedControl::border_thickness_property().as_property()),
    );
    popup_border.set_child(items_presenter.register_in_name_scope(&**ns));

    let popup = Popup::new();
    popup.set_name(Some("PART_Popup".to_string()));
    popup.set_is_light_dismiss_enabled(is_light_dismiss_enabled);
    popup.bind_binding(
        Popup::is_open_property().as_property(),
        &TemplateBinding::new(MenuItem::is_sub_menu_open_property().as_property()).with_mode(BindingMode::TwoWay),
    );
    popup.set_placement(placement);
    popup.set_child(popup_border);

    (header.register_in_name_scope(&**ns), popup.register_in_name_scope(&**ns))
}

/// The control template the test theme gives to menu items: the template of
/// the reference simple theme, minus the icon and toggle icon presenters,
/// the input gesture text, the arrow, the data template that shows text
/// headers as access text and the scroll viewer around the items presenter
/// (the test theme has no theme for it); the grid is a panel.
pub fn menu_item_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let (header, popup) = menu_item_template_parts(ns, false, crate::PlacementMode::RightEdgeAlignedTop);

        let panel = Panel::new();
        panel.children().add(header);
        panel.children().add(popup);

        let root = menu_item_root_border();
        root.set_child(panel);
        root.register_in_name_scope(&**ns).upcast()
    })
}

/// The control template of top-level menu items (the template of the item
/// container theme of menus in the reference simple theme), minus the data
/// template that shows text headers as access text, the scroll viewer
/// around the items presenter and the binding of the element under the
/// overlay that receives input to the menu.
fn top_level_menu_item_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let (header, popup) = menu_item_template_parts(ns, true, crate::PlacementMode::BottomEdgeAlignedLeft);

        let panel = Panel::new();
        panel.children().add(header);
        panel.children().add(popup);

        let root = menu_item_root_border();
        root.set_child(panel);
        root.register_in_name_scope(&**ns).upcast()
    })
}

/// The control template the test theme gives to menus (the template of the
/// reference simple theme).
pub fn menu_template() -> Rc<dyn IControlTemplate> {
    use crate::presenters::ItemsPresenter;
    use crate::ItemsControl;
    use ferroui_base::input::{KeyboardNavigation, KeyboardNavigationMode};

    FuncControlTemplate::new(|_, ns| {
        let items_presenter = ItemsPresenter::new();
        items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        items_presenter.bind_binding(
            ItemsPresenter::items_panel_property().as_property(),
            &TemplateBinding::new(ItemsControl::items_panel_property().as_property()),
        );
        KeyboardNavigation::set_tab_navigation(&items_presenter, KeyboardNavigationMode::Continue);

        let border = Border::new();
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            border.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(crate::Decorator::padding_property().as_property(), TemplatedControl::padding_property().as_property());
        bind(Border::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(Border::border_brush_property().as_property(), TemplatedControl::border_brush_property().as_property());
        bind(
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(Border::corner_radius_property().as_property(), TemplatedControl::corner_radius_property().as_property());
        border.set_child(items_presenter.register_in_name_scope(&**ns));
        border.upcast()
    })
}

/// The control theme of the scroll viewer inside menus (the menu scroll
/// viewer theme of the reference simple theme, minus the two scroll
/// buttons): a scroll content presenter named `PART_ContentPresenter` whose
/// content is NOT bound in the template. The presenter binds its content to
/// the scroll viewer itself, when it is attached to a visual tree; a menu
/// that is measured before it is shown therefore realizes no items.
fn menu_scroll_viewer_theme() -> Ref<ControlTheme> {
    use crate::presenters::ScrollContentPresenter;
    use crate::ScrollViewer;
    use ferroui_base::layout::Layoutable;

    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(|_, ns| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_binding(
            Layoutable::margin_property().as_property(),
            &TemplateBinding::new(TemplatedControl::padding_property().as_property()),
        );
        presenter.register_in_name_scope(&**ns).upcast()
    });

    ControlTheme::with_setters(ScrollViewer::TYPE, [Setter::new(TemplatedControl::template_property(), Some(template))])
}

/// The control template the test theme gives to menu flyout presenters: the
/// template of the reference simple theme (a border around a scroll viewer
/// that hosts the items presenter).
pub fn menu_flyout_presenter_template() -> Rc<dyn IControlTemplate> {
    use crate::presenters::ItemsPresenter;
    use crate::{Grid, ItemsControl};
    use ferroui_base::input::{KeyboardNavigation, KeyboardNavigationMode};
    use ferroui_base::layout::Layoutable;

    FuncControlTemplate::new(|_, ns| {
        let items_presenter = ItemsPresenter::new();
        items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        items_presenter.bind_binding(
            Layoutable::margin_property().as_property(),
            &TemplateBinding::new(TemplatedControl::padding_property().as_property()),
        );
        Grid::set_is_shared_size_scope(&items_presenter, true);
        items_presenter.bind_binding(
            ItemsPresenter::items_panel_property().as_property(),
            &TemplateBinding::new(ItemsControl::items_panel_property().as_property()),
        );
        KeyboardNavigation::set_tab_navigation(&items_presenter, KeyboardNavigationMode::Continue);

        let layout_root = Border::new();
        layout_root.set_name(Some("LayoutRoot".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            layout_root.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(Border::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(Border::border_brush_property().as_property(), TemplatedControl::border_brush_property().as_property());
        bind(
            Border::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(Border::corner_radius_property().as_property(), TemplatedControl::corner_radius_property().as_property());
        let scroll_viewer = crate::ScrollViewer::new();
        scroll_viewer.set_theme(menu_scroll_viewer_theme());
        scroll_viewer.set_content(Some(crate::Control::boxed(items_presenter.register_in_name_scope(&**ns))));
        layout_root.set_child(scroll_viewer);

        layout_root.register_in_name_scope(&**ns).upcast()
    })
}

/// Adds the control themes of menus, menu items, menu flyout presenters and
/// separators: the themes of the reference simple theme over the templates
/// above, minus the resource brushes, the scroll bar visibilities and the
/// styles for the selected, empty, disabled, toggle and checked states.
fn add_menu_themes(styles: &Ref<Styles>) {
    use crate::{ItemsControl, Menu, MenuFlyoutPresenter, MenuItem, Separator};
    use ferroui_base::input::InputElement;
    use ferroui_base::layout::Layoutable;
    use ferroui_base::media::{Brushes, IBrush};
    use ferroui_base::styling::{Selectors, Style};
    use ferroui_base::Thickness;

    let transparent = || -> Option<Rc<dyn IBrush>> { Some(Brushes::transparent()) };

    let separator_theme = ControlTheme::with_setters(
        Separator::TYPE,
        [
            Setter::new(InputElement::focusable_property(), false),
            Setter::new(Layoutable::margin_property(), Thickness::new(29.0, 1.0, 0.0, 1.0)),
            Setter::new(Layoutable::height_property(), 1.0),
            Setter::new(TemplatedControl::template_property(), Some(separator_template())),
        ],
    );
    styles.resources().add_value(ResourceKey::Type(Separator::TYPE), separator_theme);

    let menu_item_theme = ControlTheme::with_setters(
        MenuItem::TYPE,
        [
            Setter::new(TemplatedControl::background_property(), transparent()),
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(1.0)),
            Setter::new(TemplatedControl::padding_property(), Thickness::new(6.0, 0.0, 6.0, 0.0)),
            Setter::new(TemplatedControl::template_property(), Some(menu_item_template())),
        ],
    );
    let separator_item_template: Rc<dyn IControlTemplate> =
        FuncControlTemplate::new(|_, _| Separator::new().upcast());
    menu_item_theme.add_style(Style::with_setters(
        Selectors::nesting(None).class(":separator"),
        [Setter::new(TemplatedControl::template_property(), Some(separator_item_template))],
    ));
    styles.resources().add_value(ResourceKey::Type(MenuItem::TYPE), menu_item_theme);

    let top_level_menu_item_theme = ControlTheme::with_setters(
        MenuItem::TYPE,
        [
            Setter::new(TemplatedControl::background_property(), transparent()),
            Setter::new(TemplatedControl::padding_property(), Thickness::new(6.0, 0.0, 6.0, 0.0)),
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(1.0)),
            Setter::new(TemplatedControl::template_property(), Some(top_level_menu_item_template())),
        ],
    );
    let menu_theme = ControlTheme::with_setters(
        Menu::TYPE,
        [
            Setter::new(ItemsControl::item_container_theme_property(), Some(top_level_menu_item_theme)),
            Setter::new(TemplatedControl::template_property(), Some(menu_template())),
        ],
    );
    styles.resources().add_value(ResourceKey::Type(Menu::TYPE), menu_theme);

    let menu_flyout_presenter_theme = ControlTheme::with_setters(
        MenuFlyoutPresenter::TYPE,
        [
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(1.0)),
            Setter::new(TemplatedControl::template_property(), Some(menu_flyout_presenter_template())),
        ],
    );
    styles.resources().add_value(ResourceKey::Type(MenuFlyoutPresenter::TYPE), menu_flyout_presenter_theme);
}

/// The control template the test theme gives to context menus: the template
/// of the reference simple theme (a border around the items presenter),
/// minus the scroll viewer between the two (the test theme has no theme for
/// it). Without the scroll viewer it is the template of menus.
pub fn context_menu_template() -> Rc<dyn IControlTemplate> {
    menu_template()
}

/// Adds the control theme of context menus: the theme of the reference
/// simple theme over [`context_menu_template`], minus the resource brushes
/// and the text properties.
fn add_context_menu_theme(styles: &Ref<Styles>) {
    use crate::ContextMenu;
    use ferroui_base::Thickness;

    let theme = ControlTheme::with_setters(
        ContextMenu::TYPE,
        [
            Setter::new(TemplatedControl::border_thickness_property(), Thickness::uniform(1.0)),
            Setter::new(ferroui_base::input::InputElement::focusable_property(), true),
            Setter::new(TemplatedControl::padding_property(), Thickness::new(4.0, 2.0, 4.0, 2.0)),
            Setter::new(TemplatedControl::template_property(), Some(context_menu_template())),
        ],
    );
    styles.resources().add_value(ResourceKey::Type(ContextMenu::TYPE), theme);
}
