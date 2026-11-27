//! The Color page: the six sections of the source over the brushes of the WinUI theme.

use crate::color_sections::{ColorBrushInfo, ColorSection, COLOR_SECTIONS};
use ferroui_base::media::{IBrush, SolidColorBrush, TextWrapping};
use ferroui_base::{BoxedValue, CornerRadius, Ref, StyledProperty, Thickness};
use ferroui_controls::primitives::{SelectingItemsControl, TabStrip};
use ferroui_controls::{Border, Control, StackPanel, TextBlock, WrapPanel};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id != "Color" {
        return;
    }
    let strip = root.get_control::<TabStrip>("ColorSections");
    strip.set_selected_index(0);
    show_selected(root);
    let weak = root.downgrade();
    let _ = strip.property_changed(move |change| {
        if change.property() == SelectingItemsControl::selected_index_property().as_property() {
            if let Some(root) = weak.upgrade() {
                show_selected(&root);
            }
        }
    });
}

/// Binds the brush of `property` to a resource of the theme, so that it follows the theme.
fn bind_theme_brush(swatch: &Border, property: &'static StyledProperty<Option<Rc<dyn IBrush>>>, key: &str) {
    let resource = DynamicResourceExtension::with_resource_key(Some(Rc::new(key.to_owned()) as BoxedValue));
    swatch.bind_binding(property.as_property(), &*resource.as_binding());
}

/// The color of a solid brush, as `#AARRGGBB`.
fn value_text(brush: &Rc<dyn IBrush>) -> String {
    match brush.as_object().and_then(|object| object.to_ref().cast::<SolidColorBrush>()) {
        Some(solid) => format!("#{:08X}", solid.color().to_uint32()),
        None => "Composition brush".to_owned(),
    }
}

fn tile(info: &ColorBrushInfo) -> Ref<StackPanel> {
    let swatch = Border::new();
    swatch.set_height(72.0);
    swatch.set_corner_radius(CornerRadius::uniform(4.0));
    swatch.set_border_thickness(Thickness::uniform(1.0));
    bind_theme_brush(&swatch, Border::border_brush_property(), "CardStrokeColorDefaultBrush");
    let value = text("Not defined by this theme", 0.7);
    if let Some(key) = info.key {
        // The value follows the brush the theme gives the swatch.
        let weak = value.downgrade();
        let _ = swatch.property_changed(move |change| {
            if change.property() == Border::background_property().as_property() {
                if let (Some(value), Some(sender)) = (weak.upgrade(), change.sender().downcast_ref::<Border>()) {
                    value.set_text(sender.background().as_ref().map(value_text).as_deref());
                }
            }
        });
        bind_theme_brush(&swatch, Border::background_property(), key);
    }

    let panel = StackPanel::new();
    panel.set_width(240.0);
    panel.set_spacing(4.0);
    panel.set_name(Some(info.name.to_owned()));
    panel.children().add(&swatch);
    panel.children().add(&text(info.name, 1.0));
    panel.children().add(&text(info.usage, 0.7));
    panel.children().add(&value);
    panel
}

fn text(value: &str, opacity: f64) -> Ref<TextBlock> {
    let block = TextBlock::new();
    block.set_text(Some(value));
    block.set_text_wrapping(TextWrapping::Wrap);
    block.set_opacity(opacity);
    block
}

fn show(root: &Control, section: &ColorSection) {
    let host = root.get_control::<StackPanel>("ColorGroups");
    host.children().clear();
    for group in section.groups {
        let heading = text(group.name, 1.0);
        heading.set_font_size(20.0);
        let tiles = WrapPanel::new();
        tiles.set_name(Some(format!("{} tiles", group.name)));
        tiles.set_item_spacing(12.0);
        tiles.set_line_spacing(12.0);
        for info in group.brushes {
            tiles.children().add(&tile(info));
        }
        host.children().add(&heading);
        host.children().add(&text(group.description, 0.7));
        host.children().add(&tiles);
    }
}

fn show_selected(root: &Ref<Control>) {
    let index = root.get_control::<TabStrip>("ColorSections").selected_index();
    if let Some(section) = usize::try_from(index).ok().and_then(|index| COLOR_SECTIONS.get(index)) {
        show(root, section);
    }
}
