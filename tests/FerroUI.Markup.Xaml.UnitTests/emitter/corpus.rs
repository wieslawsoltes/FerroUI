//! The corpus of the emitter's differential harness: small documents, one feature each.
//!
//! `DOCUMENTS` is the input of the emitter (`generated.rs` is its output for the
//! eligible ones). `EXPECTED_ELIGIBLE` lists the documents that must be eligible (a
//! regression of the emitter's coverage fails the harness); `EXPECTED_NOT_ELIGIBLE`
//! the ones that must not be (none today; types without a recorded Rust path and anything
//! the emitter does not cover would be). A document in neither list is only reported. A
//! document whose load fails is compared by the exception type and the message of the error
//! (`duplicate_name.xaml`, `multiline_duplicate_name.xaml`, `end_init_failure.xaml`, whose
//! control fails in `EndInit`, `static_resource_missing.xaml`, the documents whose type
//! converter fails or yields a value of another type, or finds no asset). A document whose
//! load panics (a value a validator rejects) cannot be in the corpus: the harness does not
//! catch panics.

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
    ("text_block_font_style_weight.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' FontStyle='Italic' FontWeight='Bold' FontStretch='Condensed'></TextBlock>"),
    ("text_block_text_layout.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' TextAlignment='Right' TextWrapping='Wrap' MaxLines='3' LineHeight='18.5' LetterSpacing='0.5'></TextBlock>"),
    ("text_block_padding_font.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Padding='1,2' FontSize='11.5' BaselineOffset='2'></TextBlock>"),
    ("text_block_unicode_text.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Text='caf\u{e9} \u{1f600} tab&#9;end'></TextBlock>"),
    ("text_block_multiline_text.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Text='first&#10;second'></TextBlock>"),
    ("border_exponent_numbers.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Opacity='1e-3' Width='1.5E2'></Border>"),
    ("border_negative_margin.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Margin='-4,0,-2.5,1'></Border>"),
    ("border_auto_size.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Width='NaN' MaxWidth='Infinity' MinHeight='0'></Border>"),
    ("content_control_alignment.xaml", "<ContentControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' HorizontalContentAlignment='Stretch' VerticalContentAlignment='Center'></ContentControl>"),
    ("content_control_element.xaml", "<ContentControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border Padding='2'/></ContentControl>"),
    ("button_content_element.xaml", "<Button xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><TextBlock Text='Inner'/></Button>"),
    ("button_content_property_element.xaml", "<Button xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Button.Content><Border Width='3'/></Button.Content></Button>"),
    ("content_tag_null.xaml", "<ContentControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Tag='{x:Null}' Content='{x:Null}'></ContentControl>"),
    ("content_x_static_enum.xaml", "<ContentControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Content='{x:Static Dock.Bottom}'></ContentControl>"),
    ("attached_dock_x_static.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><TextBlock DockPanel.Dock='{x:Static Dock.Left}'/></Border>"),
    ("attached_grid_spans.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border Grid.RowSpan='2' Grid.ColumnSpan='3' Grid.IsSharedSizeScope='True'/></Border>"),
    ("named_nested_elements.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Name='outer'><Border Name='middle'><TextBlock x:Name='inner' Text='x'/></Border></Border>"),
    ("named_content.xaml", "<UserControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Name='host'><Border x:Name='content'/></UserControl>"),
    ("duplicate_name.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Name='same'><Border Name='same'/></Border>"),
    ("name_and_x_name.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Name='a'><TextBlock x:Name='b' Tag='t'/></Border>"),
    ("deep_nesting.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Decorator><Border><Viewbox><Border><TextBlock Text='deep'/></Border></Viewbox></Border></Decorator></Border>"),
    ("layout_transform_control.xaml", "<LayoutTransformControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border/></LayoutTransformControl>"),
    ("user_control_properties.xaml", "<UserControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Width='320' Height='200' Padding='5' Opacity='0.9'><TextBlock Text='Body' Margin='2'/></UserControl>"),
    ("multiline_document.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'\n        Padding='2'>\n  <Border x:Name='first'\n          Margin='1'>\n    <TextBlock Name='second' Text='two'/>\n  </Border>\n</Border>\n"),
    ("multiline_duplicate_name.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n  <Border x:Name='same'>\n    <Border\n        x:Name='same'/>\n  </Border>\n</Border>\n"),
    ("end_init_failure.xaml", "<Border xmlns='https://github.com/ferroui'\n        xmlns:t='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'>\n  <t:FailingEndInit Tag='x'/>\n</Border>\n"),
    ("grid_definitions_elements.xaml", "<Grid xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Grid.RowDefinitions><RowDefinition Height='Auto'/><RowDefinition Height='2*'/></Grid.RowDefinitions><Grid.ColumnDefinitions><ColumnDefinition Width='100'/></Grid.ColumnDefinitions><Border Grid.Row='1'/></Grid>"),
    ("grid_definitions_text.xaml", "<Grid xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' RowDefinitions='Auto,*' ColumnDefinitions='*,2*,40'></Grid>"),
    ("canvas_children.xaml", "<Canvas xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border Canvas.Left='1' Canvas.Top='2' Width='3'/><TextBlock Canvas.Right='4' Canvas.Bottom='5'/></Canvas>"),
    ("dock_panel_children.xaml", "<DockPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border DockPanel.Dock='Top' Height='10'/><TextBlock Text='Fill'/></DockPanel>"),
    ("nested_panels.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Spacing='2'><StackPanel Orientation='Horizontal'><Border/><Border/></StackPanel><Grid><TextBlock Text='g'/></Grid></StackPanel>"),
    ("classes_text.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Classes='one two'></Border>"),
    ("text_block_inlines.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Run Text='a'/><Run Text='b' FontWeight='Bold'/><LineBreak/></TextBlock>"),
    ("text_block_text_content.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>plain text</TextBlock>"),
    ("items_control_items.xaml", "<ItemsControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border/><TextBlock Text='i'/></ItemsControl>"),
    ("font_weight_number.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' FontWeight='600'></TextBlock>"),
    ("user_control_named_children.xaml", "<UserControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><StackPanel x:Name='panel'><Border x:Name='first'/><Border Name='second'/></StackPanel></UserControl>"),
    ("panel_children_x_null_tag.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border Tag='{x:Null}'/><Border Tag='3'/></StackPanel>"),
    ("direct_selected_index.xaml", "<ListBox xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' SelectedIndex='-1'></ListBox>"),
    ("direct_selected_item_null.xaml", "<ListBox xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' SelectedItem='{x:Null}'></ListBox>"),
    ("direct_selected_item_text.xaml", "<ComboBox xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' SelectedItem='text'></ComboBox>"),
    ("binding_element_name.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><TextBlock x:Name='source' Text='x'/><TextBlock Text='{Binding #source.Text}'/></StackPanel>"),
    ("binding_data_context.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' DataContext='hello'><TextBlock Text='{Binding}'/></StackPanel>"),
    ("binding_mode.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' DataContext='hello'><TextBlock Text='{Binding Mode=OneTime}'/><TextBlock Text='{Binding Path=., Mode=OneWay, FallbackValue=none}'/></StackPanel>"),
    ("dynamic_resource_unresolved.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Background='{DynamicResource AccentBrush}'></Border>"),
    ("type_extension.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Tag='{x:Type Button}'></Border>"),
    ("static_resource_local.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Resources><SolidColorBrush x:Key='Accent' Color='Red'/></Border.Resources><Border Background='{StaticResource Accent}'/></Border>"),
    ("dynamic_resource_local.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Resources><SolidColorBrush x:Key='Accent' Color='Red'/></Border.Resources><Border Background='{DynamicResource Accent}'/></Border>"),
    ("static_resource_missing.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Background='{StaticResource Missing}'></Border>"),
    ("x_static_converter.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Tag='{x:Static ObjectConverters.IsNotNull}'></TextBlock>"),
    ("x_static_text_trimming.xaml", "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' TextTrimming='{x:Static TextTrimming.CharacterEllipsis}'></TextBlock>"),
    ("resources_many.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Resources><x:Double x:Key='A'>1</x:Double><x:Double x:Key='B'>2</x:Double><SolidColorBrush x:Key='C' Color='Blue'/></Border.Resources></Border>"),
    ("resource_dictionary_root.xaml", "<ResourceDictionary xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><x:Double x:Key='A'>1</x:Double><SolidColorBrush x:Key='B' Color='Green'/><x:String x:Key='C'>text</x:String></ResourceDictionary>"),
    ("transitions.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Transitions><Transitions><DoubleTransition Property='Opacity' Duration='0:0:0.2'/></Transitions></Border.Transitions></Border>"),
    ("control_theme_resources.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Resources><SolidColorBrush x:Key='Accent' Color='Red'/><ControlTheme x:Key='Base' TargetType='Button'><Setter Property='Opacity' Value='0.5'/></ControlTheme><ControlTheme x:Key='Derived' TargetType='Button' BasedOn='{StaticResource Base}'><Setter Property='Background' Value='{StaticResource Accent}'/><Setter Property='Foreground' Value='{DynamicResource Accent}'/></ControlTheme></Border.Resources></Border>"),
    ("compiled_binding_element.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:CompileBindings='True'><TextBlock x:Name='source' Text='x'/><TextBlock Text='{Binding #source.Text}'/></StackPanel>"),
    ("compiled_binding_data_type.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><StackPanel.DataContext><TextBlock Text='context'/></StackPanel.DataContext><TextBlock x:DataType='TextBlock' Text='{CompiledBinding Text}'/></StackPanel>"),
    ("compiled_binding_parent.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Tag='t'><TextBlock Text='{CompiledBinding $parent[Border].Tag}'/></Border>"),
    ("compiled_binding_plain_property.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><StackPanel.DataContext><ResourceDictionary><x:Double x:Key='A'>1</x:Double><x:Double x:Key='B'>2</x:Double></ResourceDictionary></StackPanel.DataContext><TextBlock x:DataType='ResourceDictionary' Text='{CompiledBinding Count}'/><TextBlock x:DataType='ResourceDictionary' Tag='{CompiledBinding Count}'/></StackPanel>"),
    ("on_platform.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><TextBlock Text='{OnPlatform Linux=lin, Default=def}'/><TextBlock Text='{OnPlatform Windows=win, Default=def}'/><Border Opacity='{OnPlatform 0.5, Linux=0.25}'/></StackPanel>"),
    // No branch is taken on a desktop test run and there is no default: `default(T)` of the
    // property type (the zero member of an enumeration, zero of a number).
    ("on_platform_without_default.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border HorizontalAlignment='{OnPlatform Android=Left}'/><Border Width='{OnPlatform Android=10}'/></StackPanel>"),
    ("font_family.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><TextBlock FontFamily='Arial'/><TextBlock FontFamily='Arial, Consolas'/></StackPanel>"),
    ("flags_value.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' xmlns:c='using:FerroUI.Controls.Converters'><Border.Resources><c:CornerRadiusFilterConverter x:Key='Filter' Filter='TopLeft, BottomRight'/></Border.Resources></Border>"),
    ("control_template_parts.xaml", "<Button xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Background='Red'><Button.Template><ControlTemplate><Border Name='PART_Border' Background='{TemplateBinding Background}'><ContentPresenter Name='PART_ContentPresenter' Content='{TemplateBinding Content}'/></Border></ControlTemplate></Button.Template></Button>"),
    ("data_template.xaml", "<ItemsControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><ItemsControl.ItemTemplate><DataTemplate><TextBlock Text='{Binding}'/></DataTemplate></ItemsControl.ItemTemplate></ItemsControl>"),
    ("control_theme_template.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Resources><ControlTheme x:Key='Theme' TargetType='Button'><Setter Property='Template'><ControlTemplate><Border Name='PART_Root'><ContentPresenter Content='{TemplateBinding Content}'/></Border></ControlTemplate></Setter></ControlTheme></Border.Resources></Border>"),
    ("style_selectors.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Name='Root'><Border.Styles><Style Selector='Button.primary:pointerover'><Setter Property='Opacity' Value='0.5'/></Style><Style Selector='#Root > TextBlock'><Setter Property='Opacity' Value='0.5'/></Style><Style Selector='StackPanel Border'><Setter Property='Opacity' Value='0.5'/></Style><Style Selector='Button /template/ ContentPresenter'><Setter Property='Opacity' Value='0.5'/></Style><Style Selector='TextBlock:not(.muted)'><Setter Property='Opacity' Value='0.5'/></Style><Style Selector='ListBoxItem:nth-child(2n+1)'><Setter Property='Opacity' Value='0.5'/></Style><Style Selector='TextBlock[IsVisible=True]'><Setter Property='Opacity' Value='0.5'/></Style><Style Selector='Border[(Grid.Row)=1]'><Setter Property='Opacity' Value='0.5'/></Style><Style Selector='TextBlock, Border.card'><Setter Property='Opacity' Value='0.5'/></Style><Style Selector=':is(Control)'><Setter Property='Opacity' Value='0.5'/></Style></Border.Styles></Border>"),
    ("style_nested.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Styles><Style Selector='Button'><Setter Property='Opacity' Value='0.25'/><Style Selector='^:pressed'><Setter Property='Opacity' Value='0.75'/></Style><Style Selector='^ /template/ ContentPresenter'><Setter Property='Margin' Value='2'/></Style></Style></Border.Styles></Border>"),
    ("style_resources.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Resources><SolidColorBrush x:Key='Accent' Color='Blue'/></Border.Resources><Border.Styles><Style Selector='Border.accent'><Setter Property='Background' Value='{StaticResource Accent}'/><Setter Property='BorderBrush' Value='{DynamicResource Accent}'/></Style></Border.Styles></Border>"),
    ("type_converter.xaml", "<StackPanel xmlns='https://github.com/ferroui'\n            xmlns:t='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'>\n  <t:Captioned Caption='plain'/>\n  <Border><t:Captioned Caption='@parent'/></Border>\n  <t:Captioned Caption='none'/>\n</StackPanel>\n"),
    ("type_converter_failure.xaml", "<Border xmlns='https://github.com/ferroui'\n        xmlns:t='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'>\n  <t:Captioned\n      Caption='!text'/>\n</Border>\n"),
    ("type_converter_cast_failure.xaml", "<Border xmlns='https://github.com/ferroui'\n        xmlns:t='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'>\n  <t:Captioned Caption='number'/>\n</Border>\n"),
    ("type_converter_base_uri.xaml", "<Border xmlns='https://github.com/ferroui'\n        xmlns:t='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'>\n  <t:Captioned Caption='/name'/>\n</Border>\n"),
    ("image_source_missing_asset.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n  <Image Source='ferres://FerroUI.Markup.Xaml.UnitTests/Assets/missing.png'/>\n</Border>\n"),
    ("image_brush_source_missing_asset.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Border.Background><ImageBrush Source='ferres://FerroUI.Markup.Xaml.UnitTests/Assets/missing.png' Stretch='Fill'/></Border.Background></Border>"),
    ("image_source_rooted.xaml", "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n  <Image Source='/Assets/missing.png'/>\n</Border>\n"),
    ("items_source_typed_list.xaml", "<ItemsControl xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'\n              xmlns:t='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'\n              x:DataType='t:Table' ItemsSource='{CompiledBinding Rows}'>\n  <ItemsControl.ItemTemplate>\n    <DataTemplate>\n      <TextBlock Text='{CompiledBinding Name}'/>\n    </DataTemplate>\n  </ItemsControl.ItemTemplate>\n</ItemsControl>\n"),
    ("class_value.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n  <Border Classes='first' Classes.accent='True' Classes.muted='False'/>\n  <Border Classes.accent='False'/>\n</StackPanel>\n"),
    ("class_binding.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n  <CheckBox Name='Toggle' IsChecked='True'/>\n  <Border Classes.accent='{Binding #Toggle.IsChecked}'/>\n  <Border Classes.other='{ReflectionBinding IsChecked, ElementName=Toggle}'/>\n</StackPanel>\n"),
    ("list_text_held_collection.xaml", "<Canvas xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n  <Polygon Points='75,0 120,120 0,45'/>\n  <Polyline Points='0,0 65,0'/>\n  <Slider Ticks='0,20,25,40'/>\n  <Grid RowDefinitions='Auto,*'/>\n</Canvas>\n"),
    ("type_extension_object.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n  <Border Tag='{x:Type x:Object}'/>\n  <ContentControl Content='text'>\n    <ContentControl.ContentTemplate>\n      <DataTemplate x:DataType='x:Object'>\n        <TextBlock Text='{Binding}'/>\n      </DataTemplate>\n    </ContentControl.ContentTemplate>\n  </ContentControl>\n</StackPanel>\n"),
    ("compiled_binding_unnamed_types.xaml", "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'\n            xmlns:t='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'\n            x:DataType='t:Row'>\n  <Border Tag='{CompiledBinding Stamp}'/>\n  <Border Tag='{CompiledBinding Length}'/>\n  <TextBlock Text='{CompiledBinding Name}'/>\n</StackPanel>\n"),
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
    "text_block_empty_text.xaml",
    "text_block_font_size.xaml",
    "stack_panel_properties.xaml",
    "dock_panel_last_child_fill.xaml",
    "button_content_text.xaml",
    "user_control_content_element.xaml",
    "control_tag.xaml",
    "attached_grid_position.xaml",
    "attached_dock.xaml",
    "attached_canvas_left.xaml",
    "x_null_child.xaml",
    "x_static_enum.xaml",
    "border_many_properties.xaml",
    "panel_children.xaml",
    "named_element.xaml",
    "name_property.xaml",
    "brush_from_text.xaml",
    "text_block_font_style_weight.xaml",
    "text_block_text_layout.xaml",
    "text_block_padding_font.xaml",
    "text_block_unicode_text.xaml",
    "text_block_multiline_text.xaml",
    "border_exponent_numbers.xaml",
    "border_negative_margin.xaml",
    "border_auto_size.xaml",
    "content_control_alignment.xaml",
    "content_control_element.xaml",
    "button_content_element.xaml",
    "button_content_property_element.xaml",
    "content_tag_null.xaml",
    "content_x_static_enum.xaml",
    "attached_dock_x_static.xaml",
    "attached_grid_spans.xaml",
    "named_nested_elements.xaml",
    "named_content.xaml",
    "duplicate_name.xaml",
    "name_and_x_name.xaml",
    "deep_nesting.xaml",
    "layout_transform_control.xaml",
    "user_control_properties.xaml",
    "multiline_document.xaml",
    "multiline_duplicate_name.xaml",
    "end_init_failure.xaml",
    "grid_definitions_elements.xaml",
    "grid_definitions_text.xaml",
    "canvas_children.xaml",
    "dock_panel_children.xaml",
    "nested_panels.xaml",
    "classes_text.xaml",
    "text_block_inlines.xaml",
    "text_block_text_content.xaml",
    "items_control_items.xaml",
    "font_weight_number.xaml",
    "user_control_named_children.xaml",
    "panel_children_x_null_tag.xaml",
    "direct_selected_index.xaml",
    "direct_selected_item_null.xaml",
    "direct_selected_item_text.xaml",
    "binding.xaml",
    "binding_element_name.xaml",
    "binding_data_context.xaml",
    "binding_mode.xaml",
    "dynamic_resource_unresolved.xaml",
    "type_extension.xaml",
    "static_resource_missing.xaml",
    "resources.xaml",
    "static_resource_local.xaml",
    "dynamic_resource_local.xaml",
    "x_static_converter.xaml",
    "x_static_text_trimming.xaml",
    "resources_many.xaml",
    "resource_dictionary_root.xaml",
    "transitions.xaml",
    "control_theme_resources.xaml",
    "compiled_binding_element.xaml",
    "compiled_binding_data_type.xaml",
    "compiled_binding_parent.xaml",
    "compiled_binding_plain_property.xaml",
    "on_platform.xaml",
    "on_platform_without_default.xaml",
    "font_family.xaml",
    "flags_value.xaml",
    "control_template.xaml",
    "control_template_parts.xaml",
    "data_template.xaml",
    "control_theme_template.xaml",
    "style_with_selector.xaml",
    "style_selectors.xaml",
    "style_nested.xaml",
    "style_resources.xaml",
    "type_converter.xaml",
    "type_converter_failure.xaml",
    "type_converter_cast_failure.xaml",
    "type_converter_base_uri.xaml",
    "image_source_missing_asset.xaml",
    "image_brush_source_missing_asset.xaml",
    "image_source_rooted.xaml",
    "items_source_typed_list.xaml",
    "class_value.xaml",
    "class_binding.xaml",
    "list_text_held_collection.xaml",
    "type_extension_object.xaml",
    "compiled_binding_unnamed_types.xaml",
];

/// The documents whose values depend on the base URI of the document or on a service of
/// the application (a text a type converter converts with the base URI of its context; the
/// path of an asset, which the asset loader opens). Generated code states the URI of its
/// document; the run-time loader is given the same URI for them, in an application with an
/// asset loader, in a test of their own
/// (`documents_that_read_their_base_uri_build_equal_object_trees`).
pub const BASE_URI_DOCUMENTS: &[&str] =
    &["type_converter_base_uri.xaml", "image_source_missing_asset.xaml", "image_brush_source_missing_asset.xaml", "image_source_rooted.xaml"];

/// The documents that must not be eligible for emission.
pub const EXPECTED_NOT_ELIGIBLE: &[&str] = &[];

/// A document of a class of this crate (`x:Class`): the code-behind forms of the corpus.
pub struct ClassDocument {
    /// The module of `generated_handlers` that holds the emitter's output for the document.
    pub module: &'static str,
    /// The path of the document below the root URI.
    pub name: &'static str,
    /// The full name of the class the document populates.
    pub class: &'static str,
    pub xaml: &'static str,
}

/// The documents with a class: a method of the class named in markup as the handler of
/// an event or as the value of a property of a delegate type. Each is compiled as the
/// document of its class (`rust_emitter::generate_class_file_with`) by both hosts of the
/// emitter; the output is `generated_handlers/<module>.rs`, checked in.
pub const CLASS_DOCUMENTS: &[ClassDocument] = &[
    // A routed event of the element, and a routed event of another class (an attached event).
    ClassDocument {
        module: "routed_event",
        name: "Handlers/RoutedEvent.xaml",
        class: "FerroUI.Markup.Xaml.UnitTests.Xaml.MyButton",
        xaml: "<Button xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Class='FerroUI.Markup.Xaml.UnitTests.Xaml.MyButton' Click='OnClick' InputElement.Tapped='OnTapped'/>",
    },
    // A routed event of a child handled on the root, and an event whose arguments are of a
    // class of their own (`RoutedEvent<TappedEventArgs>`) handled on an element below.
    ClassDocument {
        module: "attached_event",
        name: "Handlers/AttachedEvent.xaml",
        class: "FerroUI.Markup.Xaml.UnitTests.Xaml.MyPanel",
        xaml: "<Panel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Class='FerroUI.Markup.Xaml.UnitTests.Xaml.MyPanel' Button.Click='OnClick'><Grid DoubleTapped='OnTapped'><Button Name='target'/></Grid></Panel>",
    },
    // Events that are not routed events: one of the class of the document, whose handler
    // takes the arguments untyped (wider parameters than the delegate passes), and the
    // property-changed event of an element, whose handler takes them exactly.
    ClassDocument {
        module: "plain_event",
        name: "Handlers/PlainEvent.xaml",
        class: "FerroUI.Markup.Xaml.UnitTests.Xaml.MyHost",
        xaml: "<Panel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Class='FerroUI.Markup.Xaml.UnitTests.Xaml.MyHost' Opening='OnOpening'><Border Name='target' PropertyChanged='OnPropertyChanged'/></Panel>",
    },
    // A method named as the value of a property of a delegate type (an attached property
    // and a plain one), and a handler with wider parameters on the event of a flyout.
    ClassDocument {
        module: "delegate_property",
        name: "Handlers/DelegateProperty.xaml",
        class: "FerroUI.Markup.Xaml.UnitTests.Xaml.MyHost",
        xaml: "<Panel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Class='FerroUI.Markup.Xaml.UnitTests.Xaml.MyHost'><Border Name='target' ToolTip.Placement='Custom' ToolTip.CustomPopupPlacementCallback='OnCustomPlacement'/><Button Name='button'><Button.Flyout><Flyout Placement='Custom' CustomPopupPlacementCallback='OnCustomPlacement' Opening='OnOpening'/></Button.Flyout></Button></Panel>",
    },
    // A handler named inside a template: the delegate is created by the build of the
    // deferred content, over the root object its context has from the document.
    ClassDocument {
        module: "template_event",
        name: "Handlers/TemplateEvent.xaml",
        class: "FerroUI.Markup.Xaml.UnitTests.Xaml.MyPanel",
        xaml: "<Panel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Class='FerroUI.Markup.Xaml.UnitTests.Xaml.MyPanel'><Button Name='target'><Button.Template><ControlTemplate><Border Name='part' Tapped='OnTapped'/></ControlTemplate></Button.Template></Button></Panel>",
    },
];

/// A document that names a method no method of its root object answers to: the class the
/// document is compiled for (nothing for a document without one) and how the diagnostic
/// of the compile must start (the method and the member it is named for).
pub struct MissingMethodDocument {
    pub name: &'static str,
    pub class: Option<&'static str>,
    pub xaml: &'static str,
    pub diagnostic: &'static str,
}

/// The documents that name a method that is not found: neither host compiles them, and
/// both report the same diagnostic, at the place of the name in the document.
pub const MISSING_METHOD_DOCUMENTS: &[MissingMethodDocument] = &[
    // No method of the class has the name.
    MissingMethodDocument {
        name: "Handlers/UnknownMethod.xaml",
        class: Some("FerroUI.Markup.Xaml.UnitTests.Xaml.MyButton"),
        xaml: "<Button xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Class='FerroUI.Markup.Xaml.UnitTests.Xaml.MyButton' Click='NotFound'/>",
        diagnostic: "No method `NotFound` for `Click` (System.EventHandler`1[FerroUI.Interactivity.RoutedEventArgs]): the class `FerroUI.Markup.Xaml.UnitTests.Xaml.MyButton` of the document has no method of that name that the delegate can call",
    },
    // A method of the name exists, with parameters the delegate cannot call it with.
    MissingMethodDocument {
        name: "Handlers/OtherParameters.xaml",
        class: Some("FerroUI.Markup.Xaml.UnitTests.Xaml.MyHost"),
        xaml: "<Panel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Class='FerroUI.Markup.Xaml.UnitTests.Xaml.MyHost'><Button Click='OnCustomPlacement'/></Panel>",
        diagnostic: "No method `OnCustomPlacement` for `Click` (System.EventHandler`1[FerroUI.Interactivity.RoutedEventArgs]): the class `FerroUI.Markup.Xaml.UnitTests.Xaml.MyHost` of the document has no method of that name that the delegate can call",
    },
    // A property of a delegate type.
    MissingMethodDocument {
        name: "Handlers/UnknownMethodOfProperty.xaml",
        class: Some("FerroUI.Markup.Xaml.UnitTests.Xaml.MyHost"),
        xaml: "<Panel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Class='FerroUI.Markup.Xaml.UnitTests.Xaml.MyHost'><Border ToolTip.CustomPopupPlacementCallback='NotFound'/></Panel>",
        diagnostic: "No method `NotFound` for `CustomPopupPlacementCallback` (FerroUI.Controls.Primitives.PopupPositioning.CustomPopupPlacementCallback): the class `FerroUI.Markup.Xaml.UnitTests.Xaml.MyHost` of the document has no method of that name that the delegate can call",
    },
    // A document without a class: its root object is a button, which has no such method.
    MissingMethodDocument {
        name: "Handlers/WithoutClass.xaml",
        class: None,
        xaml: "<Button xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Click='OnClick'/>",
        diagnostic: "No method `OnClick` for `Click` (System.EventHandler`1[FerroUI.Interactivity.RoutedEventArgs]): the document has no class (`x:Class`), and the type of its root object has no method of that name that the delegate can call",
    },
];
