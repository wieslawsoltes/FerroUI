//! `Controls/PagerControl.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/PagerControl/PagerControl.xaml (the implicit style of PagerControl)
//!   controls/dev/PagerControl/PagerControl_themeresources.xaml (PagerControlPageNavigationButtonStyle,
//!   PagerControlNumberPanelButtonStyle; its resources are Resources/PagerControl_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's PagerControl is WinUI's PagerControl (a WinUI control in ferroui-controls,
//!   docs/porting/winui-controls.md, G2). The parts keep WinUI's names.
//! - The ItemsRepeater NumberPanelItemsRepeater with a horizontal StackLayout (Spacing 5) is an
//!   ItemsControl with a horizontal StackPanel of the same spacing: the control finds the element of
//!   a page with ContainerFromIndex, where WinUI asks the repeater with TryGetElement.
//! - The visual states are the pseudo-classes the control sets. PagerDisplayModeStates:
//!   NumberBoxVisible is :numberbox, ComboBoxVisible :combobox, NumberPanelVisible :numberpanel.
//!   InfiniteItemsDisplayMode: InfiniteItems is :infinite. The groups of the buttons:
//!   FirstPageButtonCollapsed is :firstcollapsed, FirstPageButtonHidden :firsthidden,
//!   FirstPageButtonDisabled :firstdisabled, and the same for the previous, next and last buttons
//!   (NextPageButtonHidden and LastPageButtonHidden are states of the groups of the enabled states in
//!   WinUI, so the control sets them in place of :nextdisabled and :lastdisabled). The Visible and
//!   Enabled states set nothing. Visibility setters are IsVisible.
//! - The Style properties FirstButtonStyle, PreviousButtonStyle, NextButtonStyle and LastButtonStyle
//!   are control themes (Theme of the buttons).
//! - The NumberBox Maximum binding to NumberOfPages is left out: the control sets the maximum when
//!   the number of pages changes, as WinUI's control does too (infinity while it is not known).
//! - The tooltips of the navigation buttons (their automation names, ToolTipService.ToolTip with a
//!   binding to the attached property) and the RepositionThemeTransition of the indicator are left
//!   out.
//! - PagerControlPageNavigationButtonStyle shows its Content (a glyph of the symbol font) in a
//!   FontIcon at the FontSize and FontFamily of the button: a ContentPresenter named Content in the
//!   font of the button, which the pager sets to SymbolThemeFontFamily. MirroredWhenRightToLeft is
//!   left out. The CommonStates are :pointerover, :pressed and :disabled. WinUI applies the default
//!   style of Button beneath these keyed styles (docs/porting/winui-theme.md, section 4.2).
//! - UseSystemFocusVisuals is the focus adorner (section 4.6).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::automation::peers::AutomationLandmarkType;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::PagerControlTemplateSettings;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ComboBox;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::NumberBox;
use ferroui_controls::PagerControl;
use ferroui_controls::Panel;
use ferroui_controls::RowDefinitions;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(
        "PagerControlPageNavigationButtonStyle",
        pager_control_page_navigation_button_style,
    );
    resources.control_theme(
        "PagerControlNumberPanelButtonStyle",
        pager_control_number_panel_button_style,
    );
    // The implicit style of PagerControl (it has no key in WinUI).
    resources.control_theme(type_key::<PagerControl>(), pager_control_theme);
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

/// The focus adorner of `pager_control_page_navigation_button_style`.
fn pager_control_page_navigation_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
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

/// The template of `Button` of `pager_control_page_navigation_button_style`.
fn pager_control_page_navigation_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(ContentPresenter::new())
                .name("Content")?
                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                .template_bind(
                    ContentPresenter::font_size_property(),
                    TemplatedControl::font_size_property(),
                )
                .template_bind(
                    ContentPresenter::font_family_property(),
                    TemplatedControl::font_family_property(),
                )
                .template_bind(
                    ContentPresenter::foreground_property(),
                    TemplatedControl::foreground_property(),
                )
                .template_bind(
                    Layoutable::horizontal_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    Layoutable::vertical_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                ),
        )
        .control())
}

/// `PagerControlPageNavigationButtonStyle`: a control theme of `Button`.
fn pager_control_page_navigation_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(Layoutable::width_property(), 40.0);
    theme.set(Layoutable::height_property(), 40.0);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.static_resource(TemplatedControl::corner_radius_property(), "ControlCornerRadius")?;
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            pager_control_page_navigation_button_style_focus_adorner,
        )),
    );
    theme.template(pager_control_page_navigation_button_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PagerControlPageNavigationButtonBackgroundPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "PagerControlPageNavigationButtonForegroundPointerOver",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PagerControlPageNavigationButtonBackgroundPressed",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "PagerControlPageNavigationButtonForegroundPressed",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PagerControlPageNavigationButtonBackgroundDisabled",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<ContentPresenter>().name("Content"), |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "PagerControlPageNavigationButtonForegroundDisabled",
            );
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The template of `Button` of `pager_control_number_panel_button_style`.
fn pager_control_number_panel_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("ContentPresenter")?
        .template_bind(
            ContentPresenter::background_property(),
            TemplatedControl::background_property(),
        )
        .template_bind(
            ContentPresenter::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .template_bind(
            ContentPresenter::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .template_bind(
            ContentPresenter::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .template_bind(
            ContentPresenter::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .template_bind(
            ContentPresenter::padding_property(),
            TemplatedControl::padding_property(),
        )
        .template_bind(
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .control())
}

/// `PagerControlNumberPanelButtonStyle`: a control theme of `Button`.
fn pager_control_number_panel_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.static_resource(Layoutable::min_width_property(), "PagerControlNumberPanelButtonWidth")?;
    theme.set(Layoutable::min_height_property(), 20.0);
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.template(pager_control_number_panel_button_style_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "PagerControlPageNavigationButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "PagerControlPageNavigationButtonForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "PagerControlPageNavigationButtonBackgroundPressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "PagerControlPageNavigationButtonForegroundPressed",
            );
            Ok(())
        },
    )?;
    // Disabled
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "PagerControlPageNavigationButtonBackgroundDisabled",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "PagerControlPageNavigationButtonForegroundDisabled",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// The items panel of `template`.
fn template_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(StackPanel::new())
        .set(StackPanel::orientation_property(), Orientation::Horizontal)
        .set(StackPanel::spacing_property(), 5.0)
        .end()
        .upcast())
}

/// The template of `PagerControl` of `pager_control_theme`.
fn pager_control_theme_template(t: &Template<PagerControl>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("RootGrid")?
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .with(|e| {
            e.set_column_definitions(literal(ColumnDefinitions::parse("*,Auto,*"))?);
            Ok(())
        })?
        .with(|e| {
            e.set_row_definitions(literal(RowDefinitions::parse("Auto"))?);
            Ok(())
        })?
        .children([
            t.make(StackPanel::new())
                .set(StackPanel::orientation_property(), Orientation::Horizontal)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                .children([
                    t.make(Button::new())
                        .name("FirstPageButton")?
                        .template_bind(
                            StyledElement::theme_property(),
                            PagerControl::first_button_style_property(),
                        )
                        .dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily")
                        .set(
                            ContentControl::content_property(),
                            Some(value("".to_string()).expect("a value")),
                        )
                        .control(),
                    t.make(Button::new())
                        .name("PreviousPageButton")?
                        .template_bind(
                            StyledElement::theme_property(),
                            PagerControl::previous_button_style_property(),
                        )
                        .dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily")
                        .set(
                            ContentControl::content_property(),
                            Some(value("".to_string()).expect("a value")),
                        )
                        .control(),
                ])
                .control(),
            t.make(StackPanel::new())
                .name("BoxPanels")?
                .set(Visual::is_visible_property(), false)
                .set(StackPanel::orientation_property(), Orientation::Horizontal)
                .set(Grid::column_property(), 1)
                // NumberBox Display
                // ComboBox Display
                .children([
                    t.make(TextBlock::new())
                        .template_bind(TextBlock::text_property(), PagerControl::prefix_text_property())
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(Layoutable::margin_property(), Thickness::new(5.0, 0.0, 5.0, 0.0))
                        .control(),
                    t.make(NumberBox::new())
                        .name("NumberBoxDisplay")?
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(NumberBox::minimum_property(), 1.0)
                        .set(NumberBox::small_change_property(), 1.0)
                        .set(NumberBox::large_change_property(), 5.0)
                        .set(Visual::is_visible_property(), false)
                        .control(),
                    t.make(ComboBox::new())
                        .name("ComboBoxDisplay")?
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .bind(
                            ItemsControl::items_source_property(),
                            path()
                                .templated_parent()
                                .prop(PagerControl::template_settings_property())
                                .prop(PagerControlTemplateSettings::pages_property())
                                .binding(),
                        )
                        .set(Visual::is_visible_property(), false)
                        .control(),
                    t.make(TextBlock::new())
                        .name("SuffixTextLabel")?
                        .template_bind(TextBlock::text_property(), PagerControl::suffix_text_property())
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(Layoutable::margin_property(), Thickness::new(5.0, 0.0, 0.0, 0.0))
                        .control(),
                    t.make(TextBlock::new())
                        .name("TotalNumberOfPagesLabel")?
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(Layoutable::margin_property(), Thickness::new(5.0, 0.0, 5.0, 0.0))
                        .bind(
                            TextBlock::text_property(),
                            path()
                                .templated_parent()
                                .prop(PagerControl::number_of_pages_property())
                                .binding(),
                        )
                        .control(),
                ])
                .control(),
            t.make(ItemsControl::new())
                .name("NumberPanelItemsRepeater")?
                .bind(
                    ItemsControl::items_source_property(),
                    path()
                        .templated_parent()
                        .prop(PagerControl::template_settings_property())
                        .prop(PagerControlTemplateSettings::number_panel_items_property())
                        .binding(),
                )
                .set(Grid::column_property(), 1)
                .set(Visual::is_visible_property(), false)
                .set(
                    ItemsControl::items_panel_property(),
                    items_panel(t.scope(), template_items_panel),
                )
                .control(),
            t.make(Rectangle::new())
                .name("NumberPanelCurrentPageIndicator")?
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                .set(Grid::column_property(), 1)
                .set(Visual::is_visible_property(), false)
                .dynamic(Shape::fill_property(), "PagerControlSelectionIndicatorForeground")
                .set(Rectangle::radius_x_property(), 1.0)
                .set(Rectangle::radius_y_property(), 1.0)
                .set(Layoutable::height_property(), 2.0)
                .control(),
            t.make(StackPanel::new())
                .set(StackPanel::orientation_property(), Orientation::Horizontal)
                .set(Grid::column_property(), 2)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                .children([
                    t.make(Button::new())
                        .name("NextPageButton")?
                        .template_bind(
                            StyledElement::theme_property(),
                            PagerControl::next_button_style_property(),
                        )
                        .dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily")
                        .set(
                            ContentControl::content_property(),
                            Some(value("".to_string()).expect("a value")),
                        )
                        .control(),
                    t.make(Button::new())
                        .name("LastPageButton")?
                        .template_bind(
                            StyledElement::theme_property(),
                            PagerControl::last_button_style_property(),
                        )
                        .dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily")
                        .set(
                            ContentControl::content_property(),
                            Some(value("".to_string()).expect("a value")),
                        )
                        .control(),
                ])
                .control(),
        ])
        .control())
}

/// The default control theme of `PagerControl`.
fn pager_control_theme(theme: &StyleBuilder) -> Result {
    theme.target::<PagerControl>();
    theme.set(
        AutomationProperties::landmark_type_property(),
        Some(AutomationLandmarkType::Navigation),
    );
    theme.static_resource(
        PagerControl::first_button_style_property(),
        "PagerControlPageNavigationButtonStyle",
    )?;
    theme.static_resource(
        PagerControl::previous_button_style_property(),
        "PagerControlPageNavigationButtonStyle",
    )?;
    theme.static_resource(
        PagerControl::next_button_style_property(),
        "PagerControlPageNavigationButtonStyle",
    )?;
    theme.static_resource(
        PagerControl::last_button_style_property(),
        "PagerControlPageNavigationButtonStyle",
    )?;
    theme.template(pager_control_theme_template);
    // FirstPageButtonVisibilityStates and FirstPageButtonIsEnabledStates
    theme.style(
        nesting()
            .class(":firstcollapsed")
            .template()
            .of::<Button>()
            .name("FirstPageButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":firsthidden")
            .template()
            .of::<Button>()
            .name("FirstPageButton"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            style.set(InputElement::is_enabled_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":firstdisabled")
            .template()
            .of::<Button>()
            .name("FirstPageButton"),
        |style| {
            style.set(InputElement::is_enabled_property(), false);
            Ok(())
        },
    )?;
    // PreviousPageButtonVisibilityStates and PreviousPageButtonIsEnabledStates
    theme.style(
        nesting()
            .class(":previouscollapsed")
            .template()
            .of::<Button>()
            .name("PreviousPageButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":previoushidden")
            .template()
            .of::<Button>()
            .name("PreviousPageButton"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            style.set(InputElement::is_enabled_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":previousdisabled")
            .template()
            .of::<Button>()
            .name("PreviousPageButton"),
        |style| {
            style.set(InputElement::is_enabled_property(), false);
            Ok(())
        },
    )?;
    // NextPageButtonVisibilityStates and NextPageButtonIsEnabledStates
    theme.style(
        nesting()
            .class(":nextcollapsed")
            .template()
            .of::<Button>()
            .name("NextPageButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":nextdisabled")
            .template()
            .of::<Button>()
            .name("NextPageButton"),
        |style| {
            style.set(InputElement::is_enabled_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":nexthidden")
            .template()
            .of::<Button>()
            .name("NextPageButton"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            style.set(InputElement::is_enabled_property(), false);
            Ok(())
        },
    )?;
    // LastPageButtonStates and the enabled states of the last button
    theme.style(
        nesting()
            .class(":lastcollapsed")
            .template()
            .of::<Button>()
            .name("LastPageButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":lastdisabled")
            .template()
            .of::<Button>()
            .name("LastPageButton"),
        |style| {
            style.set(InputElement::is_enabled_property(), false);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":lasthidden")
            .template()
            .of::<Button>()
            .name("LastPageButton"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            style.set(InputElement::is_enabled_property(), false);
            Ok(())
        },
    )?;
    // PagerDisplayModeStates
    theme.style(nesting().class(":numberbox"), |style| {
        style.style(nesting().template().of::<StackPanel>().name("BoxPanels"), |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        })?;
        style.style(
            nesting().template().of::<NumberBox>().name("NumberBoxDisplay"),
            |style| {
                style.set(Visual::is_visible_property(), true);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(nesting().class(":combobox"), |style| {
        style.style(nesting().template().of::<StackPanel>().name("BoxPanels"), |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        })?;
        style.style(nesting().template().of::<ComboBox>().name("ComboBoxDisplay"), |style| {
            style.set(InputElement::is_enabled_property(), true);
            style.set(Visual::is_visible_property(), true);
            Ok(())
        })?;
        Ok(())
    })?;
    theme.style(nesting().class(":numberpanel"), |style| {
        style.style(nesting().template().of::<StackPanel>().name("BoxPanels"), |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ItemsControl>()
                .name("NumberPanelItemsRepeater"),
            |style| {
                style.set(Visual::is_visible_property(), true);
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<Rectangle>()
                .name("NumberPanelCurrentPageIndicator"),
            |style| {
                style.set(Visual::is_visible_property(), true);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // InfiniteItemsDisplayMode
    theme.style(nesting().class(":infinite"), |style| {
        style.style(
            nesting().template().of::<TextBlock>().name("SuffixTextLabel"),
            |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<TextBlock>().name("TotalNumberOfPagesLabel"),
            |style| {
                style.set(Visual::is_visible_property(), false);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Button>().name("LastPageButton"), |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}
