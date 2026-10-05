//! Port of `Xaml/RelativePanelTests.cs`.

use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{ItemsSource, RelativePanel, ScrollViewer, Window};

use crate::support::app::styled_window_application;
use crate::support::loader::load_as;

/// The stand-in of the anonymous object `new { DataExample = .. }`.
struct AnonymousDataExample {
    data_example: ItemsSource,
}

/// The stand-in of the anonymous object `new { Id = .. }`.
struct AnonymousId {
    id: String,
}

crate::test_identity_eq!(AnonymousDataExample, AnonymousId);

ferro_markup_type!(class AnonymousDataExample {
    this: Rc<AnonymousDataExample>,
    handles: [AnonymousDataExample, Rc<AnonymousDataExample>, Option<Rc<AnonymousDataExample>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    properties: [
        DataExample: ItemsSource { get: |this: &Rc<AnonymousDataExample>| this.data_example.clone() },
    ],
});

ferro_markup_type!(class AnonymousId {
    this: Rc<AnonymousId>,
    handles: [AnonymousId, Rc<AnonymousId>, Option<Rc<AnonymousId>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    properties: [
        Id: String { get: |this: &Rc<AnonymousId>| this.id.clone() },
    ],
});

/// Makes the stand-ins of the anonymous objects known to bindings.
fn register_anonymous_objects() {
    MarkupType::register_all(&[
        <AnonymousDataExample as MarkupTyped>::MARKUP,
        <AnonymousId as MarkupTyped>::MARKUP,
    ]);
    ValueTypes::register_reference::<AnonymousDataExample>();
    ValueTypes::register_reference::<AnonymousId>();
}

#[test]
#[ignore = "themes: needs the Simple theme (upstream runs this test under it)"]
fn scroll_viewer_viewport_small_than_bounds() {
    let _app = styled_window_application();
    register_anonymous_objects();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'
        Height='800'
        Width='1000'
>
  <RelativePanel x:Name="TestRelativePanel">
    <Panel
          x:Name="Area1"
          RelativePanel.AlignTopWithPanel="True"
          RelativePanel.AlignLeftWithPanel="True"
          RelativePanel.AlignRightWithPanel="True"
          Height="100"
          Background="LightSkyBlue">
      <TextBlock
          Text="Area1"
          HorizontalAlignment="Center"
          VerticalAlignment="Center"
          />
      <!-- <Button Click="Button_OnClick">Second</Button> -->
    </Panel>
    <Panel
      x:Name="Area2"
      RelativePanel.Below="Area1"
      RelativePanel.AlignLeftWithPanel="True"
      RelativePanel.AlignBottomWithPanel="True"
      Background="DeepSkyBlue"
        Width="100"
      >
      <TextBlock
        Text="Area2"
        HorizontalAlignment="Center"
        VerticalAlignment="Center"
        Height="100"
      />
    </Panel>
    <ScrollViewer
        x:Name="TestArea"
        Background="Aqua"
        RelativePanel.Below="Area1"
        RelativePanel.RightOf="Area2"
        RelativePanel.AlignRightWithPanel="True"
        RelativePanel.AlignBottomWithPanel="True"
        HorizontalScrollBarVisibility="Visible"
        Margin="0 0 0 0"
        >
      <ItemsControl
        ItemsSource="{Binding DataExample}"
        >
        <ItemsControl.ItemsPanel>
          <ItemsPanelTemplate>
            <StackPanel></StackPanel>
          </ItemsPanelTemplate>
        </ItemsControl.ItemsPanel>
        <ItemsControl.ItemTemplate>
          <DataTemplate>
            <StackPanel Orientation="Horizontal">
              <TextBlock Width="100" Text="{Binding Id}"></TextBlock>
              <TextBlock Width="100" Text="{Binding Id}"></TextBlock>
              <TextBlock Width="100" Text="{Binding Id}"></TextBlock>
              <TextBlock Width="100" Text="{Binding Id}"></TextBlock>
              <TextBlock Width="100" Text="{Binding Id}"></TextBlock>
              <TextBlock Width="100" Text="{Binding Id}"></TextBlock>
              <TextBlock Width="100" Text="{Binding Id}"></TextBlock>
              <TextBlock Width="100" Text="{Binding Id}"></TextBlock>
              <TextBlock Width="100" Text="{Binding Id}"></TextBlock>
              <TextBlock Width="100" Text="{Binding Id}"></TextBlock>
            </StackPanel>
          </DataTemplate>
        </ItemsControl.ItemTemplate>
      </ItemsControl>
    </ScrollViewer>
  </RelativePanel>
</Window>"#;
    let window = load_as::<Ref<Window>>(xaml);
    {
        let panel = window.get_control::<RelativePanel>("TestRelativePanel");

        let data_example = ItemsSource::from_items((1001..1101).map(|e| {
            let item: BoxedValue = Rc::new(AnonymousId { id: format!("{e}") });
            Some(item)
        }));
        let data_context: BoxedValue = Rc::new(AnonymousDataExample { data_example });
        panel.set_data_context(Some(data_context));
    }
    window.apply_template();
    window.show();

    let sv = window.get_control::<ScrollViewer>("TestArea");
    assert!(sv.viewport().width < sv.bounds().width);
    assert!(sv.viewport().height < sv.bounds().height);
}
