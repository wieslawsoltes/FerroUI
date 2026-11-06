//! Compares only source-written assignments; extra native padding/decoration nodes are mapped
//! explicitly. This is not a WinUI/native defaults, focus, validation or renderer oracle.
#![allow(dead_code)]
use crate::{
    primitives::{TouchButton, UniformWrapPanel},
    themes::KeyboardTheme,
    views::{
        ClipboardPanelView, EditingPanelView, EmojiPanelView, PaddedGrid, SmartbarView,
        SubtypePickerView,
    },
};
use ferroui_base::{CornerRadius, Ref, Thickness, media::IBrush};
use ferroui_controls::{
    Border, Control, Grid, ScrollViewer, StackPanel, TextBlock, automation::AutomationProperties,
};
use serde_json::{Value, json};
use std::rc::Rc;
pub fn bits(v: f64) -> Value {
    json!(format!("{:016x}", v.to_bits()))
}
pub fn float(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn text(v: Option<String>) -> Value {
    v.map_or(Value::Null, |s| json!(s.encode_utf16().collect::<Vec<_>>()))
}
fn brush(v: Option<Rc<dyn IBrush>>) -> Value {
    v.map_or(Value::Null, |b| {
        json!(KeyboardTheme::to_hex(
            b.as_solid_color_brush().unwrap().color()
        ))
    })
}
fn thick(v: Thickness) -> Value {
    json!([bits(v.left), bits(v.top), bits(v.right), bits(v.bottom)])
}
fn corner(v: CornerRadius) -> Value {
    json!([
        bits(v.top_left),
        bits(v.top_right),
        bits(v.bottom_right),
        bits(v.bottom_left)
    ])
}
fn property(c: &Control, name: &str) -> Value {
    let button = c.downcast_ref::<TouchButton>();
    let border = c.downcast_ref::<Border>();
    let grid = c.downcast_ref::<Grid>();
    let label = c.downcast_ref::<TextBlock>();
    let path = c.downcast_ref::<ferroui_controls::shapes::Path>();
    match name {
        "Width" => bits(c.width()),
        "Height" => bits(c.height()),
        "MaxWidth" => bits(c.max_width()),
        "MinWidth" => bits(c.min_width()),
        "MaxLines" => json!(label.unwrap().max_lines()),
        "Row" => json!(Grid::get_row(c)),
        "RowSpan" => json!(Grid::get_row_span(c)),
        "Left" => bits(ferroui_controls::Canvas::get_left(c)),
        "Top" => bits(ferroui_controls::Canvas::get_top(c)),
        "ColumnSpan" => json!(Grid::get_column_span(c)),
        "IsHitTestVisible" => json!(c.is_hit_test_visible()),
        "Visibility" => json!(if c.is_visible() {
            "Visible"
        } else {
            "Collapsed"
        }),
        "HorizontalAlignment" => json!(format!("{:?}", c.horizontal_alignment())),
        "VerticalAlignment" => json!(format!("{:?}", c.vertical_alignment())),
        "Margin" => thick(c.margin()),
        "Column" => json!(Grid::get_column(c)),
        "CornerRadius" => corner(
            button
                .map(|b| b.corner_radius())
                .unwrap_or_else(|| border.unwrap().corner_radius()),
        ),
        "Padding" => thick(
            if let Some(editing) = c.downcast_ref::<EditingPanelView>() {
                editing.padding()
            } else if let Some(padded) = c.downcast_ref::<PaddedGrid>() {
                padded.padding()
            } else if let Some(button) = button {
                button.padding()
            } else {
                border.unwrap().padding()
            },
        ),
        "Background" => brush(if let Some(b) = border {
            b.background()
        } else {
            grid.unwrap().background()
        }),
        "BorderBrush" => brush(
            button
                .map(|b| b.border_brush())
                .unwrap_or_else(|| border.unwrap().border_brush()),
        ),
        "BorderThickness" => thick(
            button
                .map(|b| b.border_thickness())
                .unwrap_or_else(|| border.unwrap().border_thickness()),
        ),
        "Foreground" => brush(label.unwrap().foreground()),
        "FontSize" => bits(label.unwrap().font_size()),
        "FontFamily" => json!(label.unwrap().font_family().name()),
        "FontWeight" => json!(label.unwrap().font_weight().0),
        "Text" => text(label.unwrap().text()),
        "TextAlignment" => json!(format!("{:?}", label.unwrap().text_alignment())),
        "TextWrapping" => json!(format!("{:?}", label.unwrap().text_wrapping())),
        "TextTrimming" => json!(label.unwrap().text_trimming().to_string()),
        "AutomationName" => text(AutomationProperties::get_name(c)),
        "Fill" => brush(path.unwrap().fill()),
        "Stretch" => json!(format!("{:?}", path.unwrap().stretch())),
        _ => panic!("unmapped property {name}"),
    }
}
fn children(c: &Control) -> Vec<Ref<Control>> {
    if let Some(view) = c.downcast_ref::<EditingPanelView>() {
        return view.content_grid().children().snapshot().to_vec();
    }
    if let Some(view) = c.downcast_ref::<PaddedGrid>() {
        return view.content_grid().children().snapshot().to_vec();
    }
    if let Some(b) = c.downcast_ref::<TouchButton>() {
        return vec![b.content_grid().upcast()];
    }
    if let Some(p) = c.downcast_ref::<ferroui_controls::Panel>() {
        return p.children().snapshot().to_vec();
    }
    if let Some(b) = c.downcast_ref::<Border>() {
        return b.child().into_iter().collect();
    }
    if let Some(content) = c.downcast_ref::<ferroui_controls::ContentControl>() {
        return content
            .content()
            .as_ref()
            .and_then(Control::from_boxed)
            .into_iter()
            .collect();
    }
    vec![]
}
fn kind(c: &Control) -> &'static str {
    if c.downcast_ref::<SmartbarView>().is_some() {
        "SmartbarView"
    } else if c.downcast_ref::<EmojiPanelView>().is_some() {
        "EmojiPanelView"
    } else if c.downcast_ref::<ClipboardPanelView>().is_some() {
        "ClipboardPanelView"
    } else if c.downcast_ref::<EditingPanelView>().is_some() {
        "EditingPanelView"
    } else if c.downcast_ref::<SubtypePickerView>().is_some() {
        "SubtypePickerView"
    } else if c.downcast_ref::<UniformWrapPanel>().is_some() {
        "UniformWrapPanel"
    } else if c.downcast_ref::<TouchButton>().is_some() {
        "TouchButton"
    } else if c.downcast_ref::<ferroui_controls::Canvas>().is_some() {
        "Canvas"
    } else if c.downcast_ref::<Grid>().is_some() {
        "Grid"
    } else if c.downcast_ref::<StackPanel>().is_some() {
        "StackPanel"
    } else if c.downcast_ref::<ScrollViewer>().is_some() {
        "ScrollViewer"
    } else if c.downcast_ref::<Border>().is_some() {
        "Border"
    } else if c.downcast_ref::<TextBlock>().is_some() {
        "TextBlock"
    } else if c.downcast_ref::<ferroui_controls::shapes::Path>().is_some() {
        "Path"
    } else {
        panic!("unknown tree node")
    }
}
pub fn compare(c: &Control, source: &Value) {
    assert_eq!(kind(c), source["type"]);
    for (name, value) in source["properties"].as_object().unwrap() {
        assert_eq!(&property(c, name), value, "{}.{}", kind(c), name);
    }
    let grid = if let Some(view) = c.downcast_ref::<EditingPanelView>() {
        Some(view.content_grid())
    } else if let Some(view) = c.downcast_ref::<PaddedGrid>() {
        Some(view.content_grid())
    } else {
        c.to_ref().downcast::<Grid>().ok()
    };
    if let Some(grid) = grid {
        if let Some(rows) = source["rows"].as_array() {
            let native = grid.row_definitions().snapshot();
            assert_eq!(native.len(), rows.len());
            for (n, s) in native.iter().zip(rows) {
                assert_eq!(bits(n.height().value()), s["value"]);
                assert_eq!(
                    format!("{:?}", n.height().grid_unit_type()),
                    s["unit"].as_str().unwrap()
                );
            }
        }
        let columns = grid.column_definitions().snapshot();
        let source = source["columns"].as_array().unwrap();
        assert_eq!(columns.len(), source.len());
        for (c, s) in columns.iter().zip(source) {
            assert_eq!(bits(c.width().value()), s["value"]);
            assert_eq!(
                format!("{:?}", c.width().grid_unit_type()),
                s["unit"].as_str().unwrap()
            );
        }
    }
    if let Some(button) = c.downcast_ref::<TouchButton>()
        && source["repeat"].is_object()
    {
        assert_eq!(
            json!(button.repeat_delay().map(|t| t.as_nanos() / 100)),
            source["repeat"]["delay"]
        );
        assert_eq!(
            json!(button.repeat_interval().as_nanos() / 100),
            source["repeat"]["interval"]
        );
        assert_eq!(
            json!(button.long_press_delay().as_nanos() / 100),
            source["repeat"]["longPress"]
        );
    }
    if let Some(panel) = c.downcast_ref::<UniformWrapPanel>() {
        assert_eq!(bits(panel.item_width()), source["cells"]["width"]);
        assert_eq!(bits(panel.item_height()), source["cells"]["height"]);
        assert_eq!(json!(panel.max_columns()), source["cells"]["max"]);
    }
    let children = children(c);
    let expected = source["children"].as_array().unwrap();
    assert_eq!(children.len(), expected.len(), "{} child count", kind(c));
    for (c, s) in children.iter().zip(expected) {
        compare(c, s);
    }
}
