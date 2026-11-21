import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

export const styleView = `<Window xmlns="https://github.com/ferroui" xmlns:app="using:TodoWorkbench.Design" Width="440" Height="240">
  <Window.Styles><StyleInclude Source="Styles/ReviewStyle.xaml"/></Window.Styles>
  <Border Padding="24" Background="#FFF2F6FD">
    <StackPanel Spacing="12">
      <app:PreviewBadge Classes="review" Text="Customer review"/>
      <TextBlock Theme="{StaticResource ReviewMessage}" Text="Ready for your team"/>
    </StackPanel>
  </Border>
</Window>`;

export const authoredStyles = `<Styles xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:app="using:TodoWorkbench.Design">
  <Styles.Resources><ResourceDictionary>
    <ResourceDictionary.MergedDictionaries><ResourceInclude Source="Palette.xaml"/></ResourceDictionary.MergedDictionaries>
    <ControlTheme x:Key="ReviewMessage" TargetType="TextBlock" BasedOn="{StaticResource StandardMessage}">
      <Setter Property="FontSize" Value="16"/>
    </ControlTheme>
  </ResourceDictionary></Styles.Resources>
  <!-- Keep this declaration and its setters together. 🧭 -->
  <Style Selector="app|PreviewBadge.review">
    <Setter Property="Opacity" Value="1"/>
    <Setter Property="Foreground" Value="#FF233F67"/>
  </Style>
</Styles>`.replaceAll('\n', '\r\n');

const palette = `<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <SolidColorBrush x:Key="ReviewAccent" Color="#FF247D60"/>
  <StaticResource x:Key="ReviewAccentAlias" ResourceKey="ReviewAccent"/>
  <ControlTheme x:Key="StandardMessage" TargetType="TextBlock"><Setter Property="Foreground" Value="#FF233F67"/></ControlTheme>
  <ControlTheme x:Key="ApprovedMessage" TargetType="TextBlock"><Setter Property="Foreground" Value="#FF247D60"/></ControlTheme>
  <ControlTheme x:Key="WrongControl" TargetType="Border"/>
</ResourceDictionary>`;

export async function writeHostedStyleFixture(root: string): Promise<void> {
    await mkdir(join(root, 'Styles'), { recursive: true });
    await Promise.all([
        writeFile(join(root, 'Review.xaml'), styleView),
        writeFile(join(root, 'Styles', 'ReviewStyle.xaml'), authoredStyles),
        writeFile(join(root, 'Styles', 'Palette.xaml'), palette),
        writeFile(join(root, 'review.xamlmeta'), JSON.stringify({
            format: 2, name: 'todo-workbench', crate_name: 'todo_workbench',
            types: [{ name: 'PreviewBadge', namespace: 'TodoWorkbench.Design', kind: 'class',
                rust_path: '::todo_workbench::app_preview::PreviewBadge', object_model: true,
                base: '::ferroui_controls::TextBlock' }],
        })),
    ]);
}
