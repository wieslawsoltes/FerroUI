//! `Controls/PersonPicture.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/PersonPicture/PersonPicture.xaml (the implicit style of PersonPicture and
//!   DefaultPersonPictureStyle; the resources are Resources/PersonPicture_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's PersonPicture is WinUI's PersonPicture (a WinUI control in ferroui-controls,
//!   docs/porting/winui-controls.md, G2). The parts keep WinUI's names.
//! - The visual states are the pseudo-classes the control sets. CommonStates: Photo is :photo,
//!   NoPhotoOrInitials :nophotoorinitials, Group :group (Initials sets nothing). BadgeStates:
//!   BadgeWithoutImageSource is :badge, BadgeWithImageSource :badgeimage (NoBadge sets nothing).
//!   Visibility setters are IsVisible.
//! - The ImageBrush BadgeImageBrush, which WinUI declares in the setter of BadgingEllipse.Fill of
//!   BadgeWithImageSource, belongs to the control, which sets it as the Fill of BadgingEllipse; the
//!   state shows the ellipse (its Opacity).
//! - StrokeThickness of the outer ellipse is the resource PersonPictureEllipseStrokeThickness,
//!   which WinUI's style sets as BorderThickness and the template binds: FerroUI's BorderThickness is
//!   a Thickness, and StrokeThickness a number.
//! - The Text and FontFamily of InitialsTextBlock, which WinUI's template binds and the states
//!   NoPhotoOrInitials and Group replace, are setters of a style of the part, so that the setters of
//!   the states win over them.
//! - x:DeferLoadStrategy, IsTextScaleFactorEnabled, IsTabStop and the AccessibilityView of the text
//!   parts are left out.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::FlowDirection;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextLineBounds;
use ferroui_base::Visual;
use ferroui_controls::automation::AccessibilityView;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::PersonPictureTemplateSettings;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Control;
use ferroui_controls::FontIcon;
use ferroui_controls::Grid;
use ferroui_controls::PersonPicture;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultPersonPictureStyle", default_person_picture_style);
    // The implicit style of PersonPicture (BasedOn DefaultPersonPictureStyle in WinUI).
    resources.control_theme(type_key::<PersonPicture>(), person_picture_theme);
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

/// The template of `PersonPicture` of `default_person_picture_style`.
fn default_person_picture_style_template(t: &Template<PersonPicture>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("RootGrid")?
        .children([
            t.make(Ellipse::new())
                .template_bind(Shape::fill_property(), TemplatedControl::background_property())
                .template_bind(Shape::stroke_property(), TemplatedControl::border_brush_property())
                .dynamic(
                    Shape::stroke_thickness_property(),
                    "PersonPictureEllipseStrokeThickness",
                )
                .template_bind(Layoutable::width_property(), Layoutable::width_property())
                .template_bind(Layoutable::height_property(), Layoutable::height_property())
                .control(),
            t.make(TextBlock::new())
                .name("InitialsTextBlock")?
                .set(TextBlock::font_size_property(), 40.0)
                .template_bind(
                    TextBlock::foreground_property(),
                    TemplatedControl::foreground_property(),
                )
                .template_bind(
                    TextBlock::font_weight_property(),
                    TemplatedControl::font_weight_property(),
                )
                .set(TextBlock::text_line_bounds_property(), TextLineBounds::Tight)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .control(),
            t.make(Ellipse::new())
                .name("PersonPictureEllipse")?
                .template_bind(Layoutable::width_property(), Layoutable::width_property())
                .template_bind(Layoutable::height_property(), Layoutable::height_property())
                .set(Visual::flow_direction_property(), FlowDirection::LeftToRight)
                .control(),
            t.make(Grid::new())
                .name("BadgeGrid")?
                .set(Visual::is_visible_property(), false)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                .dynamic(Layoutable::margin_property(), "PersonPictureBadgeGridMargin")
                // Default font family to "Segoe MDL2 Assets"
                .children([
                    t.make(Ellipse::new())
                        .name("BadgingBackgroundEllipse")?
                        .dynamic(Visual::opacity_property(), "PersonPictureEllipseBadgeStrokeOpacity")
                        .dynamic(Shape::fill_property(), "PersonPictureEllipseBadgeFillThemeBrush")
                        .dynamic(Shape::stroke_property(), "PersonPictureEllipseBadgeStrokeThemeBrush")
                        .dynamic(
                            Shape::stroke_thickness_property(),
                            "PersonPictureEllipseBadgeStrokeThickness",
                        )
                        .control(),
                    t.make(Ellipse::new())
                        .name("BadgingEllipse")?
                        .set(Visual::opacity_property(), 0.0)
                        .set(Visual::flow_direction_property(), FlowDirection::LeftToRight)
                        .control(),
                    t.make(TextBlock::new())
                        .name("BadgeNumberTextBlock")?
                        .dynamic(
                            TextBlock::foreground_property(),
                            "PersonPictureEllipseBadgeForegroundThemeBrush",
                        )
                        .template_bind(
                            TextBlock::font_family_property(),
                            TemplatedControl::font_family_property(),
                        )
                        .template_bind(
                            TextBlock::font_weight_property(),
                            TemplatedControl::font_weight_property(),
                        )
                        .set(TextBlock::text_line_bounds_property(), TextLineBounds::Tight)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                        .control(),
                    t.make(FontIcon::new())
                        .name("BadgeGlyphIcon")?
                        .dynamic(
                            TemplatedControl::foreground_property(),
                            "PersonPictureEllipseBadgeForegroundThemeBrush",
                        )
                        .dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily")
                        .template_bind(
                            TemplatedControl::font_weight_property(),
                            TemplatedControl::font_weight_property(),
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                        .control(),
                ])
                .control(),
        ])
        .control())
}

/// `DefaultPersonPictureStyle`: a control theme of `PersonPicture`.
fn default_person_picture_style(theme: &StyleBuilder) -> Result {
    theme.target::<PersonPicture>();
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "PersonPictureForegroundThemeBrush",
    );
    theme.dynamic(
        TemplatedControl::background_property(),
        "PersonPictureEllipseFillThemeBrush",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "PersonPictureEllipseFillStrokeBrush",
    );
    theme.set(Layoutable::width_property(), 96.0);
    theme.set(Layoutable::height_property(), 96.0);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::SemiBold);
    theme.set(
        AutomationProperties::accessibility_view_property(),
        AccessibilityView::Raw,
    );
    theme.template(default_person_picture_style_template);
    // The text and the font of the initials, which the states replace: a style, so that the
    // setters of the states win over it (WinUI's template binds them).
    theme.style(
        nesting().template().of::<TextBlock>().name("InitialsTextBlock"),
        |style| {
            style.bind(
                TextBlock::font_family_property(),
                path()
                    .templated_parent()
                    .prop(TemplatedControl::font_family_property())
                    .binding(),
            );
            style.bind(
                TextBlock::text_property(),
                path()
                    .templated_parent()
                    .prop(PersonPicture::template_settings_property())
                    .prop(PersonPictureTemplateSettings::actual_initials_property())
                    .binding(),
            );
            Ok(())
        },
    )?;
    // CommonStates: a photo is available for display.
    theme.style(
        nesting()
            .class(":photo")
            .template()
            .of::<Ellipse>()
            .name("PersonPictureEllipse"),
        |style| {
            style.bind(
                Shape::fill_property(),
                path()
                    .templated_parent()
                    .prop(PersonPicture::template_settings_property())
                    .prop(PersonPictureTemplateSettings::actual_image_brush_property())
                    .binding(),
            );
            Ok(())
        },
    )?;
    // No photo and no initials: the placeholder symbol of a contact.
    theme.style(
        nesting()
            .class(":nophotoorinitials")
            .template()
            .of::<TextBlock>()
            .name("InitialsTextBlock"),
        |style| {
            style.dynamic(TextBlock::font_family_property(), "SymbolThemeFontFamily");
            style.set(TextBlock::text_property(), Some(String::from("")));
            Ok(())
        },
    )?;
    // Group mode: the placeholder symbol of people.
    theme.style(
        nesting()
            .class(":group")
            .template()
            .of::<TextBlock>()
            .name("InitialsTextBlock"),
        |style| {
            style.dynamic(TextBlock::font_family_property(), "SymbolThemeFontFamily");
            style.set(TextBlock::text_property(), Some(String::from("")));
            Ok(())
        },
    )?;
    // BadgeStates
    theme.style(
        nesting().class(":badge").template().of::<Grid>().name("BadgeGrid"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    theme.style(nesting().class(":badgeimage"), |style| {
        style.style(nesting().template().of::<Grid>().name("BadgeGrid"), |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        })?;
        style.style(nesting().template().of::<Ellipse>().name("BadgingEllipse"), |style| {
            style.dynamic(
                Visual::opacity_property(),
                "PersonPictureEllipseBadgeImageSourceStrokeOpacity",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `PersonPicture`.
fn person_picture_theme(theme: &StyleBuilder) -> Result {
    theme.target::<PersonPicture>();
    theme.based_on("DefaultPersonPictureStyle")?;
    Ok(())
}
