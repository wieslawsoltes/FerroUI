//! The corpus of the emitter's differential harness: small documents, one feature each.
//!
//! `DOCUMENTS` is the input of the emitter (`generated.rs` is its output for the
//! eligible ones). `EXPECTED_ELIGIBLE` lists the documents that must be eligible (a
//! regression of the emitter's coverage fails the harness); `EXPECTED_NOT_ELIGIBLE`
//! the ones that must not be (bindings, styles with selectors, templates, resources,
//! collection adds and direct properties are outside this increment). A document in
//! neither list is only reported.

/// `(name, xaml)`: the name is the path of the document below the root URI.
pub const DOCUMENTS: &[(&str, &str)] = &[
    ("border_empty.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'></Border>"),
    ("border_padding.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Padding='1,2,3,4'></Border>"),
    ("border_padding_uniform.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Padding='5'></Border>"),
    ("border_thickness_and_radius.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' BorderThickness='2' CornerRadius='3,4,5,6'></Border>"),
    ("border_margin.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Margin='10,20'></Border>"),
    ("border_size.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Width='100' Height='50.5'></Border>"),
    ("border_min_width.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' MinWidth='12'></Border>"),
    ("border_opacity.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Opacity='0.25'></Border>"),
    ("border_is_visible.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' IsVisible='False'></Border>"),
    ("border_clip_to_bounds.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' ClipToBounds='True'></Border>"),
    ("border_z_index.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' ZIndex='3'></Border>"),
    ("border_horizontal_alignment.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' HorizontalAlignment='Center'></Border>"),
    ("border_vertical_alignment.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' VerticalAlignment='Bottom'></Border>"),
    ("border_layout_rounding.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' UseLayoutRounding='False'></Border>"),
    ("border_input_flags.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' IsEnabled='False' Focusable='True' IsHitTestVisible='False'></Border>"),
    ("border_child.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><TextBlock/></Border>"),
    ("border_child_with_text.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Padding='4'><TextBlock Text='Hello'/></Border>"),
    ("border_nested.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border Margin='1'><Border Opacity='0.5'><TextBlock Text='Deep'/></Border></Border></Border>"),
    ("text_block_text.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Text='Hi &quot;there&quot; \\ you'></TextBlock>"),
    ("text_block_empty_text.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Text=''></TextBlock>"),
    ("text_block_font_size.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' FontSize='20'></TextBlock>"),
    ("stack_panel_properties.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Orientation='Horizontal' Spacing='4'></StackPanel>"),
    ("dock_panel_last_child_fill.xaml", "<DockPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' LastChildFill='False'></DockPanel>"),
    ("button_content_text.xaml", "<Button xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Content='Click'></Button>"),
    ("user_control_content_element.xaml", "<UserControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border Width='8'/></UserControl>"),
    ("control_tag.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Tag='marker'></Border>"),
    ("attached_grid_position.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><TextBlock Grid.Row='1' Grid.Column='2'/></Border>"),
    ("attached_dock.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><TextBlock DockPanel.Dock='Right'/></Border>"),
    ("attached_canvas_left.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><TextBlock Canvas.Left='10.5'/></Border>"),
    ("x_null_child.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Child='{x:Null}'></Border>"),
    ("x_static_enum.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' HorizontalAlignment='{x:Static HorizontalAlignment.Right}'></Border>"),
    ("border_many_properties.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Width='200' Height='100' Margin='1,2,3,4' Padding='8' Opacity='0.75' HorizontalAlignment='Left' VerticalAlignment='Top' ZIndex='7'><TextBlock Text='All' IsVisible='True'/></Border>"),
    ("panel_children.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border/><TextBlock Text='Two'/></StackPanel>"),
    ("named_element.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Name='root'></Border>"),
    ("name_property.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><TextBlock Name='inner'/></Border>"),
    ("binding.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Text='{Binding Name}'></TextBlock>"),
    ("style_with_selector.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Styles><Style Selector='TextBlock'><Setter Property='Opacity' Value='0.5'/></Style></Border.Styles></Border>"),
    ("control_template.xaml", "<Button xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Button.Template><ControlTemplate><Border/></ControlTemplate></Button.Template></Button>"),
    ("brush_from_text.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Background='Red'></Border>"),
    ("resources.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Resources><x:Double x:Key='Size'>4</x:Double></Border.Resources></Border>"),
];

/// The documents that must be eligible for emission.
pub const EXPECTED_ELIGIBLE: &[&str] = &[
    "border_empty.xaml",
    "border_padding.xaml",
    "border_padding_uniform.xaml",
    "border_thickness_and_radius.xaml",
    "border_margin.xaml",
    "border_size.xaml",
    "border_min_width.xaml",
    "border_opacity.xaml",
    "border_is_visible.xaml",
    "border_clip_to_bounds.xaml",
    "border_z_index.xaml",
    "border_horizontal_alignment.xaml",
    "border_vertical_alignment.xaml",
    "border_layout_rounding.xaml",
    "border_input_flags.xaml",
    "border_child.xaml",
    "border_child_with_text.xaml",
    "border_nested.xaml",
    "text_block_text.xaml",
    "stack_panel_properties.xaml",
    "dock_panel_last_child_fill.xaml",
    "border_many_properties.xaml",
];

/// The documents that must not be eligible for emission.
pub const EXPECTED_NOT_ELIGIBLE: &[&str] = &[
    "panel_children.xaml",
    "named_element.xaml",
    "name_property.xaml",
    "binding.xaml",
    "style_with_selector.xaml",
    "control_template.xaml",
    "resources.xaml",
];
