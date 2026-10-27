# Design-time values and sample data

The **Design Data** canvas command opens a staged editor for the selected authored control or
template part. With no selection it edits the document root. It supports existing override
rows, adding an arbitrary property, object XAML, design width/height and generated sample items.
Apply writes one document transaction; Cancel, Escape and light dismissal discard the draft.

An override changes only the design namespace. The application binding, literal, property
element and comments remain in source. Uncheck an override to remove it. **Show design-time
values** switches the preview between the design values and application values without changing
source or undo history. A collaboration viewer can switch that preview but cannot author values.

```xml
<UserControl xmlns="https://github.com/ferroui"
             xmlns:d="http://schemas.microsoft.com/expression/blend/2008"
             xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
             mc:Ignorable="d"
             d:DesignWidth="760" d:DesignHeight="480">
  <StackPanel Spacing="12" Margin="24">
    <TextBlock Text="{Binding Title}" d:Text="Account overview" FontSize="24" />
    <Button Content="{Binding ActionLabel}" d:Content="Save changes" />
    <d:TextBlock Text="This note exists only in the designer." />
  </StackPanel>
</UserControl>
```

The **Preview Environment** editor's **Use design size** action reads the root's design width
and height. User-selected environment sizes are retained while ordinary properties are edited.

## Source semantics

The implementation resolves the design namespace URI, so aliases such as `preview:Text` work.
An application namespace that happens to use the prefix `d` remains an application namespace.
The source planner chooses an available prefix and adds `mc:Ignorable` without removing existing
compatibility tokens. Scalar and object forms of one design property replace each other
atomically. Conflicting effective overrides, such as both `d:Width` and `d:DesignWidth`, report a
preview diagnostic.

Generic design attributes override their application property in the temporary loading source.
Design property elements are supported as well:

```xml
<ContentControl Content="{Binding CurrentView}"
                xmlns="https://github.com/ferroui"
                xmlns:d="http://schemas.microsoft.com/expression/blend/2008"
                xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
                mc:Ignorable="d">
  <d:ContentControl.Content>
    <Border Padding="16" Background="#FFEAF2FF">
      <TextBlock Text="Sample view" />
    </Border>
  </d:ContentControl.Content>
</ContentControl>
```

The existing `Design.Width`, `Design.Height` and `Design.DataContext` attached forms also apply
to the preview. `Design.PreviewWith` and design-style configuration continue through the native
framework's existing design hooks. Design-only controls use the framework namespace; application
controls can use ordinary registered types inside a design property element.

The preview composes source maps across the Window projection, definition scope and design-data
projection. A rendered template part still identifies its original authored node even when
an override changes source length or removes a runtime subtree. Generated sample records and
synthetic preview hosts do not become authored objects.

## Generated sample records

Select an items control and enable **Generate sample items from template bindings**, or author:

```xml
<ItemsControl xmlns="https://github.com/ferroui"
              xmlns:d="http://schemas.microsoft.com/expression/blend/2008"
              xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
              mc:Ignorable="d"
              ItemsSource="{Binding ActualItems}"
              d:ItemsSource="{d:SampleData ItemCount=5}">
  <ItemsControl.ItemTemplate>
    <DataTemplate>
      <StackPanel Orientation="Horizontal" Spacing="12">
        <TextBlock Text="{Binding Person.FirstName}" />
        <CheckBox IsChecked="{Binding IsSelected}" />
      </StackPanel>
    </DataTemplate>
  </ItemsControl.ItemTemplate>
</ItemsControl>
```

`ItemCount` defaults to five and accepts 0–200. A preview graph has a total budget of 4,096 generated
records, including nested sample collections. Reaching the budget produces a notice and truncates
remaining generated collections. The temporary records are immutable and deterministic:

- Text uses the binding member name and a one-based row number.
- Registered numeric target properties receive multiples of ten.
- Registered Boolean target properties alternate true and false.
- Dot-separated paths create nested records, up to sixteen segments and 128 fields per record.
- Without named binding fields, rows are ordinary sample strings.

Inline item templates and visible local `StaticResource` item templates contribute binding
fields. `d:DataContext="{d:SampleData}"` creates one record for descendant bindings. Bindings with
explicit `Source`, `ElementName` or `RelativeSource` keep those sources and do not contribute
sample fields. Complex paths, indexers and converter-specific input types should use explicit
design values or an authored design object.

Generated records use a dedicated property-access plugin, which matches only the internal sample
record type. Their preview bindings use `ReflectionBinding`, including when the application uses
compiled bindings; converters and the other binding arguments are retained. The authored
compiled-binding source is unchanged. Sample generation does not construct an application
viewmodel or call its setters. Explicit object XAML still uses normal registered loader types.

The native graph receives samples after loading. Every realized instance is matched through
source provenance; a sibling sharing its type, name or source line does not inherit an unrelated
sample request. Newly realized template instances are initialized on runtime-map refresh, while
already initialized instances retain temporary interaction state. The native top-level preview
uses the same sample-data path as the artboard.

## Embedding and extension APIs

`plan_design_time_edits(document, commands)` produces source edits without a native UI. It stages
the complete batch on a private document before returning edits. `DesignTimeEdit` supports
`SetProperty`, `SetObject` and `SampleItems`; a `None` value resets the design override.
`DesignerSession::apply_design_time_edits` applies the batch through the usual writable-session
transaction boundary.

`DesignTimeEditor` is a reusable native control. Its model carries the owner, existing values,
writability and current preview state; its host callback accepts or rejects one staged batch.
`DesignTimeSnapshot` captures URI, revision, AST, selection, generation and definition-scope epoch.
The workspace checks this token again before accepting a draft.

`PreviewHostOptions::design_time_data` defaults to true for designer hosts. Registered root
factories receive the exact authored document in `PreviewRequest::source`; the optional
`PreviewRequest::preview_source` contains the projected loading form. `populate` uses that form
while still populating the application's chosen native root. The normal application XAML loader
and build-time compiler are unchanged.

## References and verification

Microsoft's [design-time data workflow](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/xaml-designtime-data?view=visualstudio)
motivates design-namespace attributes, object values and placeholder controls. Its
[sample-data workflow](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/xaml-design-time-sample-data?view=visualstudio)
motivates bounded `SampleData` generation inferred from template bindings.

The focused `design_time` regressions cover namespace aliases and collisions, atomic source edits,
exact application-source retention, Unicode source mapping, property elements, generated records,
compiled-binding preview isolation, same-line siblings, native-window parity, permission checks,
stale drafts and registered application-root factories.
