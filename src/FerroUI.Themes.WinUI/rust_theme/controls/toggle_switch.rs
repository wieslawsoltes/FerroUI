//! `Controls/ToggleSwitch.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/ToggleSwitch_themeresources.xaml (the styles; its resources are
//!   Resources/ToggleSwitch_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The header of WinUI's ToggleSwitch is the Content of FerroUI's: HeaderContentPresenter is
//!   PART_ContentPresenter, shown while there is content (the control shows it when it has a
//!   Header). OffContentPresenter and OnContentPresenter are PART_OffContentPresenter and
//!   PART_OnContentPresenter.
//! - The knob: WinUI moves the Grid SwitchKnob with a TranslateTransform (X = 20 in the state On,
//!   RepositionThemeAnimation between the states). FerroUI's switch moves the panel
//!   PART_MovingKnobs on the Canvas PART_SwitchKnob (Canvas.Left), so SwitchKnob is the canvas,
//!   the knobs are in PART_MovingKnobs, and the reposition animation is the KnobTransitions of the
//!   control: a DoubleTransition of Canvas.Left with the timing of the reposition theme animation
//!   of Windows (367 ms, spline 0.1,0.9,0.2,1), which the framework does not define in the source.
//! - The visual states: CommonStates are the pseudo-classes :pointerover, :pressed and :disabled
//!   (Normal sets the base values); ToggleStates On and Off are :checked and :unchecked, and
//!   ContentStates OnContent and OffContent the same. The colour animations of OuterBorder and
//!   SwitchAreaGrid (ControlFasterAnimationDuration, 83 ms) and the opacity animations of the toggle
//!   states (83 ms) are brush and opacity transitions; the sizes of the knobs
//!   (ControlFasterAnimationDuration with ControlFastOutSlowInKeySpline) are DoubleTransitions of
//!   83 ms with the spline 0,0,0,1, also for Disabled (250 ms in the source). The state Dragging and
//!   the visual transitions from and to it are left out: FerroUI's :dragging keeps the look of
//!   the toggle state.
//! - The root Grid draws the background and the border: a Border around the Grid. SwitchAreaGrid
//!   has a corner radius: a Border of the same name. Row heights of resources
//!   (ToggleSwitchPreContentMargin, ToggleSwitchPostContentMargin, 10) are written as values.
//! - Control.IsTemplateFocusTarget of SwitchAreaGrid is TemplatedControl.IsTemplateFocusTarget,
//!   and UseSystemFocusVisuals with FocusVisualMargin="-7,-3,-7,-3" is the FocusAdorner.
//!   ManipulationMode has no counterpart (the control handles the drag).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::Animatable;
use ferroui_base::animation::BrushTransition;
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::SignedDuration;
use ferroui_base::animation::TransitionBase;
use ferroui_base::animation::Transitions;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::TextWrapping;
use ferroui_base::CornerRadius;
use ferroui_base::RelativePoint;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Canvas;
use ferroui_controls::ColumnDefinition;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::GridLength;
use ferroui_controls::RowDefinition;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ToggleSwitch;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultToggleSwitchStyle", default_toggle_switch_style);
    resources.control_theme(type_key::<ToggleSwitch>(), toggle_switch_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`: the document included
/// (`ResourceInclude`) rather than merged.
#[allow(dead_code)]
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The focus adorner of `default_toggle_switch_style`.
fn default_toggle_switch_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::new(-7.0, -3.0, -7.0, -3.0))
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "SystemControlFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(
                    Border::border_brush_property(),
                    "SystemControlFocusVisualSecondaryBrush",
                )
                .dynamic(Border::corner_radius_property(), "ControlCornerRadius"),
        )
        .control())
}

/// The template of `ToggleSwitch` of `default_toggle_switch_style`.
fn default_toggle_switch_style_template(t: &Template<ToggleSwitch>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Grid::new())
                .with(|e| {
                    e.set_row_definitions(literal(RowDefinitions::parse("Auto,*"))?);
                    Ok(())
                })?
                .children([
                    t.make(ContentPresenter::new())
                        .name("PART_ContentPresenter")?
                        .set(Grid::row_property(), 0)
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(
                            ContentPresenter::content_template_property(),
                            ContentControl::content_template_property(),
                        )
                        .dynamic(ContentPresenter::foreground_property(), "ToggleSwitchHeaderForeground")
                        .set(InputElement::is_hit_test_visible_property(), false)
                        .dynamic(Layoutable::margin_property(), "ToggleSwitchTopHeaderMargin")
                        .set(ContentPresenter::text_wrapping_property(), TextWrapping::Wrap)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                        .set(ContentPresenter::recognizes_access_key_property(), true)
                        .control(),
                    t.make(Grid::new())
                        .set(Grid::row_property(), 1)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                        .with(|e| {
                            let items = Grid::row_definitions(e);
                            items.add(
                                t.make(RowDefinition::new())
                                    .set(RowDefinition::height_property(), literal(GridLength::parse("10"))?)
                                    .end()
                                    .into(),
                            );
                            items.add(
                                t.make(RowDefinition::new())
                                    .set(RowDefinition::height_property(), GridLength::auto())
                                    .end()
                                    .into(),
                            );
                            items.add(
                                t.make(RowDefinition::new())
                                    .set(RowDefinition::height_property(), literal(GridLength::parse("10"))?)
                                    .end()
                                    .into(),
                            );
                            Ok(())
                        })?
                        .with(|e| {
                            let items = Grid::column_definitions(e);
                            items.add(
                                t.make(ColumnDefinition::new())
                                    .set(ColumnDefinition::width_property(), GridLength::auto())
                                    .end()
                                    .into(),
                            );
                            items.add(
                                t.make(ColumnDefinition::new())
                                    .set(ColumnDefinition::width_property(), literal(GridLength::parse("12"))?)
                                    .set(ColumnDefinition::max_width_property(), 12.0)
                                    .end()
                                    .into(),
                            );
                            items.add(
                                t.make(ColumnDefinition::new())
                                    .set(ColumnDefinition::width_property(), GridLength::auto())
                                    .end()
                                    .into(),
                            );
                            Ok(())
                        })?
                        .children([
                            t.make(Border::new())
                                .name("SwitchAreaGrid")?
                                .set(Grid::row_span_property(), 3)
                                .set(Grid::column_span_property(), 3)
                                .set(Layoutable::margin_property(), Thickness::symmetric(0.0, 5.0))
                                .template_bind(
                                    Border::corner_radius_property(),
                                    TemplatedControl::corner_radius_property(),
                                )
                                .set(TemplatedControl::is_template_focus_target_property(), true)
                                .dynamic(Border::background_property(), "ToggleSwitchContainerBackground")
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(BrushTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Border::background_property().property()),
                                                )
                                                .set(
                                                    TransitionBase::duration_property(),
                                                    SignedDuration::from_milliseconds(83.0),
                                                )
                                                .end()
                                                .into(),
                                        );
                                        object
                                    }),
                                )
                                .control(),
                            t.make(ContentPresenter::new())
                                .name("PART_OffContentPresenter")?
                                .set(Grid::row_span_property(), 3)
                                .set(Grid::column_property(), 2)
                                .set(Visual::opacity_property(), 0.0)
                                .template_bind(
                                    ContentPresenter::foreground_property(),
                                    TemplatedControl::foreground_property(),
                                )
                                .set(InputElement::is_hit_test_visible_property(), false)
                                .template_bind(
                                    ContentPresenter::content_property(),
                                    ToggleSwitch::off_content_property(),
                                )
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    ToggleSwitch::off_content_template_property(),
                                )
                                .template_bind(
                                    Layoutable::horizontal_alignment_property(),
                                    ContentControl::horizontal_content_alignment_property(),
                                )
                                .template_bind(
                                    Layoutable::vertical_alignment_property(),
                                    ContentControl::vertical_content_alignment_property(),
                                )
                                .control(),
                            t.make(ContentPresenter::new())
                                .name("PART_OnContentPresenter")?
                                .set(Grid::row_span_property(), 3)
                                .set(Grid::column_property(), 2)
                                .set(Visual::opacity_property(), 0.0)
                                .template_bind(
                                    ContentPresenter::foreground_property(),
                                    TemplatedControl::foreground_property(),
                                )
                                .set(InputElement::is_hit_test_visible_property(), false)
                                .template_bind(
                                    ContentPresenter::content_property(),
                                    ToggleSwitch::on_content_property(),
                                )
                                .template_bind(
                                    ContentPresenter::content_template_property(),
                                    ToggleSwitch::on_content_template_property(),
                                )
                                .template_bind(
                                    Layoutable::horizontal_alignment_property(),
                                    ContentControl::horizontal_content_alignment_property(),
                                )
                                .template_bind(
                                    Layoutable::vertical_alignment_property(),
                                    ContentControl::vertical_content_alignment_property(),
                                )
                                .control(),
                            t.make(Rectangle::new())
                                .name("OuterBorder")?
                                .set(Grid::row_property(), 1)
                                .set(Layoutable::height_property(), 20.0)
                                .set(Layoutable::width_property(), 40.0)
                                .set(Rectangle::radius_x_property(), 10.0)
                                .set(Rectangle::radius_y_property(), 10.0)
                                .dynamic(Shape::fill_property(), "ToggleSwitchFillOff")
                                .dynamic(Shape::stroke_property(), "ToggleSwitchStrokeOff")
                                .dynamic(
                                    Shape::stroke_thickness_property(),
                                    "ToggleSwitchOuterBorderStrokeThickness",
                                )
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(BrushTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Shape::fill_property().property()),
                                                )
                                                .set(
                                                    TransitionBase::duration_property(),
                                                    SignedDuration::from_milliseconds(83.0),
                                                )
                                                .end()
                                                .into(),
                                        );
                                        object.add(
                                            t.make(BrushTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Shape::stroke_property().property()),
                                                )
                                                .set(
                                                    TransitionBase::duration_property(),
                                                    SignedDuration::from_milliseconds(83.0),
                                                )
                                                .end()
                                                .into(),
                                        );
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Visual::opacity_property().property()),
                                                )
                                                .set(
                                                    TransitionBase::duration_property(),
                                                    SignedDuration::from_milliseconds(83.0),
                                                )
                                                .end()
                                                .into(),
                                        );
                                        object
                                    }),
                                )
                                .control(),
                            t.make(Rectangle::new())
                                .name("SwitchKnobBounds")?
                                .set(Grid::row_property(), 1)
                                .set(Layoutable::height_property(), 20.0)
                                .set(Layoutable::width_property(), 40.0)
                                .set(Rectangle::radius_x_property(), 10.0)
                                .set(Rectangle::radius_y_property(), 10.0)
                                .dynamic(Shape::fill_property(), "ToggleSwitchFillOn")
                                .dynamic(Shape::stroke_property(), "ToggleSwitchStrokeOn")
                                .dynamic(Shape::stroke_thickness_property(), "ToggleSwitchOnStrokeThickness")
                                .set(Visual::opacity_property(), 0.0)
                                .set(
                                    Animatable::transitions_property(),
                                    Some({
                                        let object = Transitions::new();
                                        object.add(
                                            t.make(DoubleTransition::new())
                                                .set(
                                                    TransitionBase::property_property(),
                                                    Some(Visual::opacity_property().property()),
                                                )
                                                .set(
                                                    TransitionBase::duration_property(),
                                                    SignedDuration::from_milliseconds(83.0),
                                                )
                                                .end()
                                                .into(),
                                        );
                                        object
                                    }),
                                )
                                .control(),
                            t.make(Canvas::new())
                                .name("PART_SwitchKnob")?
                                .set(Grid::row_property(), 1)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                                .set(Layoutable::width_property(), 20.0)
                                .set(Layoutable::height_property(), 20.0)
                                .children([t
                                    .make(Grid::new())
                                    .name("PART_MovingKnobs")?
                                    .set(Layoutable::width_property(), 20.0)
                                    .set(Layoutable::height_property(), 20.0)
                                    .children([
                                        t.make(Border::new())
                                            .name("SwitchKnobOn")?
                                            .dynamic(Border::background_property(), "ToggleSwitchKnobFillOn")
                                            .dynamic(Border::border_brush_property(), "ToggleSwitchKnobStrokeOn")
                                            .set(
                                                Border::background_sizing_property(),
                                                BackgroundSizing::OuterBorderEdge,
                                            )
                                            .set(Layoutable::width_property(), 12.0)
                                            .set(Layoutable::height_property(), 12.0)
                                            .set(Border::corner_radius_property(), CornerRadius::uniform(7.0))
                                            .set(Visual::opacity_property(), 0.0)
                                            .set(
                                                Layoutable::horizontal_alignment_property(),
                                                HorizontalAlignment::Center,
                                            )
                                            .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 1.0, 0.0))
                                            .set(
                                                Visual::render_transform_origin_property(),
                                                literal(RelativePoint::parse("50%,50%"))?,
                                            )
                                            .set(
                                                Animatable::transitions_property(),
                                                Some({
                                                    let object = Transitions::new();
                                                    object.add(
                                                        t.make(DoubleTransition::new())
                                                            .set(
                                                                TransitionBase::property_property(),
                                                                Some(Layoutable::width_property().property()),
                                                            )
                                                            .set(
                                                                TransitionBase::duration_property(),
                                                                SignedDuration::from_milliseconds(83.0),
                                                            )
                                                            .set(
                                                                TransitionBase::easing_property(),
                                                                literal(Easing::parse("0,0,0,1"))?,
                                                            )
                                                            .end()
                                                            .into(),
                                                    );
                                                    object.add(
                                                        t.make(DoubleTransition::new())
                                                            .set(
                                                                TransitionBase::property_property(),
                                                                Some(Layoutable::height_property().property()),
                                                            )
                                                            .set(
                                                                TransitionBase::duration_property(),
                                                                SignedDuration::from_milliseconds(83.0),
                                                            )
                                                            .set(
                                                                TransitionBase::easing_property(),
                                                                literal(Easing::parse("0,0,0,1"))?,
                                                            )
                                                            .end()
                                                            .into(),
                                                    );
                                                    object.add(
                                                        t.make(DoubleTransition::new())
                                                            .set(
                                                                TransitionBase::property_property(),
                                                                Some(Visual::opacity_property().property()),
                                                            )
                                                            .set(
                                                                TransitionBase::duration_property(),
                                                                SignedDuration::from_milliseconds(83.0),
                                                            )
                                                            .end()
                                                            .into(),
                                                    );
                                                    object
                                                }),
                                            )
                                            .control(),
                                        t.make(Rectangle::new())
                                            .name("SwitchKnobOff")?
                                            .dynamic(Shape::fill_property(), "ToggleSwitchKnobFillOff")
                                            .set(Layoutable::width_property(), 12.0)
                                            .set(Layoutable::height_property(), 12.0)
                                            .set(Rectangle::radius_x_property(), 7.0)
                                            .set(Rectangle::radius_y_property(), 7.0)
                                            .set(
                                                Layoutable::horizontal_alignment_property(),
                                                HorizontalAlignment::Center,
                                            )
                                            .set(Layoutable::margin_property(), Thickness::new(-1.0, 0.0, 0.0, 0.0))
                                            .set(
                                                Visual::render_transform_origin_property(),
                                                literal(RelativePoint::parse("50%,50%"))?,
                                            )
                                            .set(
                                                Animatable::transitions_property(),
                                                Some({
                                                    let object = Transitions::new();
                                                    object.add(
                                                        t.make(DoubleTransition::new())
                                                            .set(
                                                                TransitionBase::property_property(),
                                                                Some(Layoutable::width_property().property()),
                                                            )
                                                            .set(
                                                                TransitionBase::duration_property(),
                                                                SignedDuration::from_milliseconds(83.0),
                                                            )
                                                            .set(
                                                                TransitionBase::easing_property(),
                                                                literal(Easing::parse("0,0,0,1"))?,
                                                            )
                                                            .end()
                                                            .into(),
                                                    );
                                                    object.add(
                                                        t.make(DoubleTransition::new())
                                                            .set(
                                                                TransitionBase::property_property(),
                                                                Some(Layoutable::height_property().property()),
                                                            )
                                                            .set(
                                                                TransitionBase::duration_property(),
                                                                SignedDuration::from_milliseconds(83.0),
                                                            )
                                                            .set(
                                                                TransitionBase::easing_property(),
                                                                literal(Easing::parse("0,0,0,1"))?,
                                                            )
                                                            .end()
                                                            .into(),
                                                    );
                                                    object.add(
                                                        t.make(DoubleTransition::new())
                                                            .set(
                                                                TransitionBase::property_property(),
                                                                Some(Visual::opacity_property().property()),
                                                            )
                                                            .set(
                                                                TransitionBase::duration_property(),
                                                                SignedDuration::from_milliseconds(83.0),
                                                            )
                                                            .end()
                                                            .into(),
                                                    );
                                                    object
                                                }),
                                            )
                                            .control(),
                                    ])
                                    .control()])
                                .control(),
                        ])
                        .control(),
                ]),
        )
        .control())
}

/// `DefaultToggleSwitchStyle`: a control theme of `ToggleSwitch`.
fn default_toggle_switch_style(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleSwitch>();
    theme.dynamic(TemplatedControl::foreground_property(), "ToggleSwitchContentForeground");
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Left,
    );
    // Not in the style: the default of a WinUI control.
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), default_toggle_switch_style_focus_adorner)),
    );
    theme.static_resource(Layoutable::min_width_property(), "ToggleSwitchThemeMinWidth")?;
    theme.static_resource(TemplatedControl::corner_radius_property(), "ControlCornerRadius")?;
    // RepositionThemeAnimation of SwitchKnob between the states Off and On.
    theme.set_value(
        ToggleSwitch::knob_transitions_property(),
        value({
            let object = Transitions::new();
            object.add(
                theme
                    .scope()
                    .make(DoubleTransition::new())
                    .set(
                        TransitionBase::property_property(),
                        Some(Canvas::left_property().property()),
                    )
                    .set(
                        TransitionBase::duration_property(),
                        SignedDuration::from_milliseconds(367.0),
                    )
                    .set(
                        TransitionBase::easing_property(),
                        literal(Easing::parse("0.1,0.9,0.2,1"))?,
                    )
                    .end()
                    .into(),
            );
            object
        }),
    );
    theme.template(default_toggle_switch_style_template);
    // The header is shown while there is one.
    theme.style(
        nesting()
            .class(":empty")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Rectangle>().name("OuterBorder"), |style| {
            style.dynamic(Shape::stroke_property(), "ToggleSwitchStrokeOffPointerOver");
            style.dynamic(Shape::fill_property(), "ToggleSwitchFillOffPointerOver");
            Ok(())
        })?;
        style.style(nesting().template().of::<Rectangle>().name("SwitchKnobOff"), |style| {
            style.dynamic(Shape::fill_property(), "ToggleSwitchKnobFillOffPointerOver");
            style.set(Layoutable::width_property(), 14.0);
            style.set(Layoutable::height_property(), 14.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("SwitchKnobOn"), |style| {
            style.dynamic(Border::background_property(), "ToggleSwitchKnobFillOnPointerOver");
            style.set(Layoutable::width_property(), 14.0);
            style.set(Layoutable::height_property(), 14.0);
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("SwitchKnobBounds"),
            |style| {
                style.dynamic(Shape::fill_property(), "ToggleSwitchFillOnPointerOver");
                style.dynamic(Shape::stroke_property(), "ToggleSwitchStrokeOnPointerOver");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("SwitchAreaGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "ToggleSwitchContainerBackgroundPointerOver",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Rectangle>().name("OuterBorder"), |style| {
            style.dynamic(Shape::stroke_property(), "ToggleSwitchStrokeOffPressed");
            style.dynamic(Shape::fill_property(), "ToggleSwitchFillOffPressed");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("SwitchKnobBounds"),
            |style| {
                style.dynamic(Shape::fill_property(), "ToggleSwitchFillOnPressed");
                style.dynamic(Shape::stroke_property(), "ToggleSwitchStrokeOnPressed");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Rectangle>().name("SwitchKnobOff"), |style| {
            style.dynamic(Shape::fill_property(), "ToggleSwitchKnobFillOffPressed");
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
            style.set(Layoutable::margin_property(), Thickness::new(3.0, 0.0, 0.0, 0.0));
            style.set(Layoutable::width_property(), 17.0);
            style.set(Layoutable::height_property(), 14.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("SwitchKnobOn"), |style| {
            style.dynamic(Border::background_property(), "ToggleSwitchKnobFillOnPressed");
            style.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right);
            style.set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 3.0, 0.0));
            style.set(Layoutable::width_property(), 17.0);
            style.set(Layoutable::height_property(), 14.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("SwitchAreaGrid"), |style| {
            style.dynamic(Border::background_property(), "ToggleSwitchContainerBackgroundPressed");
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleSwitchHeaderForegroundDisabled",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_OffContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleSwitchContentForegroundDisabled",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_OnContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "ToggleSwitchContentForegroundDisabled",
                );
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Rectangle>().name("OuterBorder"), |style| {
            style.dynamic(Shape::stroke_property(), "ToggleSwitchStrokeOffDisabled");
            style.dynamic(Shape::fill_property(), "ToggleSwitchFillOffDisabled");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Rectangle>().name("SwitchKnobBounds"),
            |style| {
                style.dynamic(Shape::fill_property(), "ToggleSwitchFillOnDisabled");
                style.dynamic(Shape::stroke_property(), "ToggleSwitchStrokeOnDisabled");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Rectangle>().name("SwitchKnobOff"), |style| {
            style.dynamic(Shape::fill_property(), "ToggleSwitchKnobFillOffDisabled");
            style.set(Layoutable::width_property(), 12.0);
            style.set(Layoutable::height_property(), 12.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("SwitchKnobOn"), |style| {
            style.dynamic(Border::background_property(), "ToggleSwitchKnobFillOnDisabled");
            style.set(Layoutable::width_property(), 12.0);
            style.set(Layoutable::height_property(), 12.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("SwitchAreaGrid"), |style| {
            style.dynamic(Border::background_property(), "ToggleSwitchContainerBackgroundDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    // On (ToggleStates) and OnContent (ContentStates)
    theme.style(nesting().class(":checked"), |style| {
        style.style(
            nesting().template().of::<Rectangle>().name("SwitchKnobBounds"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Rectangle>().name("OuterBorder"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("SwitchKnobOn"), |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        })?;
        style.style(nesting().template().of::<Rectangle>().name("SwitchKnobOff"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_OnContentPresenter"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
                style.set(InputElement::is_hit_test_visible_property(), true);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // OffContent (ContentStates); Off sets the base values.
    theme.style(
        nesting()
            .class(":unchecked")
            .template()
            .of::<ContentPresenter>()
            .name("PART_OffContentPresenter"),
        |style| {
            style.set(Visual::opacity_property(), 1.0);
            style.set(InputElement::is_hit_test_visible_property(), true);
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `ToggleSwitch`.
fn toggle_switch_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleSwitch>();
    theme.based_on("DefaultToggleSwitchStyle")?;
    Ok(())
}
