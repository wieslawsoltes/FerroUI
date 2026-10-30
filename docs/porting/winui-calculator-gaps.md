# The framework gaps of the Windows Calculator port

This page is the inventory, the design and the plan of the framework features that the port of the
Windows Calculator to FerroUI found missing, in the order the owner set:

1. **XAML-02**: `VisualStateManager`, `AdaptiveTrigger`, `StateTrigger` and the `StateTriggerBase`
   extension point (the Calculator derives its own `ControlSizeTrigger` from it);
2. **CONTROL-02**: `ListView` (with `GridView`, the controls of `ListViewBase`) and `SwipeControl`;
3. `Pivot`;
4. **XAML-03**: system high contrast: the HighContrast variant of the WinUI theme selected from the
   contrast setting of the system, with the system colours, updated live, on Windows, macOS
   ("Increase contrast") and Linux.

The identifiers are those of the Calculator's tracker (`composition-gaps-tracker.md` of the
port's notes). The other items of that tracker (materials, composition, NavigationView, the
application work) are covered elsewhere: Reveal and the materials by #325, `ThemeShadow` by #326,
NavigationView by the W-series of `winui-controls.md`.

Nothing here is implemented yet. Each item lands as a series of small pull requests from `main`
(section 8), each with its tests, its rows in the deviation registers and a line in
`CHANGELOG.md` under "Unreleased".

## 1. Sources and revisions

| What | Revision |
|---|---|
| FerroUI `main`, the base of the inventory | `622412be` (2026-10-24, after #313 and #328) |
| WinUI source of truth | microsoft-ui-xaml, branch `winui3/main`, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (MIT, read-only), the pin of every WinUI port of the repository (O13) |
| Windows Calculator, for the requirements | microsoft/calculator `79e1ac53bba690cdf930e5dcd6ed9dbc4cbfb128`, the revision of the port |
| Open pull requests inventoried | #269 Repeater `d36d1f5d`; #283 Inking `62e6ab21`; #325 materials `17e128ee`; #326 shadows `d6d5370d`; #327 core gaps `653fc7a8`; the W-series of #286 (`winui-controls.md`): #329 W11 `36209c8f`, #330 W1 `5b834e59`, #331 W2 `57e10200`, #332 W3 `247aa580`, #333 W4 `77c4ff7c`, #334 W5 `0f2f9550`, #335 W6 `21851317`; the integration plan of #286 (owner decisions of 2026-10-24, recorded in `winui-controls.md` section 2) |

The WinUI sources, by item (paths relative to the checkout; `LIB` = `dxaml/xcp/dxaml/lib/`,
`CS` = `controls/dev/CommonStyles/`, `GX` = `dxaml/xcp/dxaml/themes/generic.xaml`):

- **Visual states:** the IDL `dxaml/xcp/dxaml/idl/winrt/core/microsoft.ui.xaml.coretypes.idl`
  and `coretypes2.idl`; `LIB/VisualStateManager_Partial.cpp`, `VisualStateGroup_Partial.cpp`,
  `StateTriggerBase_Partial.cpp`; the engine `dxaml/xcp/components/vsm/` (`CVisualStateManager2.cpp`,
  `VisualStateManagerActuator.cpp`, `StandardVisualStateManagerDataSource.cpp`,
  `OptimizedVisualStateManagerDataSource.cpp`, `VisualStateSetterHelper.cpp`,
  `DynamicTransitionStoryboardGenerator.cpp`, `VisualTransitionTableOptimizedLookup.cpp`);
  `dxaml/xcp/core/core/elements/VisualStateManager.cpp`, `StateTriggerBase.cpp`,
  `StateTrigger.cpp`, `AdaptiveTrigger.cpp`; the trigger qualifiers
  `dxaml/xcp/components/qualifiers/` (`VariantMap.h`, `MinWidthQualifier.cpp`,
  `ExtensibleQualifier.cpp`, `QualifierFactory.cpp`); the deferred loading
  `dxaml/xcp/components/deferral/VisualStateGroupCollectionCustomWriter.cpp`.
- **Lists:** the metadata `dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Controls.cs`
  (lines 6336 to 7482) and `...Primitives.cs`; `LIB/ListViewBase_Partial*.cpp` (eight files),
  `ListViewBaseItem_Partial.cpp`, `ListView_Partial.cpp`, `GridView_Partial.cpp`,
  `ItemsStackPanel_Partial.cpp`, `ItemsWrapGrid_Partial.cpp`, the chrome
  `dxaml/xcp/core/core/elements/ListViewBaseItemChrome.cpp`; the styles GX 9284 (ListView), 9348
  (GridView), `CS/ListViewItem_themeresources.xaml`, `CS/GridViewItem_themeresources.xaml`; the
  peers `LIB/ListViewBaseAutomationPeer`, `ListViewAutomationPeer`, `ListViewItemAutomationPeer`,
  `ListViewItemDataAutomationPeer`, `ListViewBaseItemAutomationPeer` (and the GridView ones);
  `controls/dev/ItemsView`, `controls/dev/ItemContainer`.
- **Swipe:** `controls/dev/SwipeControl/` (`SwipeControl.cpp`, `SwipeItem.cpp`, `SwipeItems.cpp`,
  `SwipeControl.idl`, `SwipeControl.xaml`, `SwipeControl_themeresources.xaml`).
- **Pivot:** the metadata `dxaml/xcp/tools/XCPTypesAutoGen/Modules/Phone/Phone.cs` and
  `Primitives.cs`; `dxaml/phone/lib/Pivot_Partial.cpp`, `Pivot_Automation_Partial.cpp`,
  `Pivot_SlideInEffect_Partial.cpp`, `PivotHeaderManager.cpp`, `PivotHeaderItem.cpp`,
  `PivotHeaderPanel_Partial.cpp`, `PivotPanel_Partial.cpp`, `PivotItem_Partial.cpp`;
  `dxaml/xcp/components/pivot/PivotStateMachine.cpp`; `CS/Pivot_themeresources.xaml`; the peers
  `PivotAutomationPeer`, `PivotItemAutomationPeer`, `PivotItemDataAutomationPeer`.
- **High contrast:** `dxaml/xcp/components/theminginterop/SystemThemingInterop.cpp`
  (`GetSystemHighContrastTheme`, `GetSystemColor`), `dxaml/xcp/components/theming/FrameworkTheming.cpp`
  (`OnThemeChanged`, `GetTheme`, `RebuildColorAndBrushResources`),
  `dxaml/xcp/core/core/elements/Resources.cpp` (`EnsureActiveThemeDictionary`),
  `dxaml/xcp/core/dll/xcpcore.cpp` (`UpdateColorAndBrushResources`, `NotifyThemeChange`),
  `LIB/JupiterWindow.cpp` (`RegisterUISettingsEvents`).

## 2. Standing decisions that bind the design

| Rule | Consequence here |
|---|---|
| The upstream-ported public API and behaviour do not change; WinUI additions are additive or opt-in (O10, section 6 of `winui-controls.md`) | No VSM code runs for an element that has no visual state groups; upstream styling, pseudo-classes and control themes behave as today. Every WinUI member on an upstream-ported type is opt-in, with a test that its default keeps upstream and a row in `DEVIATIONS.md`, "WinUI additions to upstream controls". |
| WinUI controls go into `ferroui-controls` unless they depend on a feature crate (O1) | VisualStateManager, the storyboards, ListView, GridView, SwipeControl and Pivot go into `ferroui-controls`. ItemsView and ItemContainer depend on ItemsRepeater, so they go into `ferroui-controls-repeater` (#269), if they are ported (decision D3). |
| Templates are XAML control themes of the WinUI theme; no Fluent or Simple templates for now (O2, O2b) | The new templated controls get documents in `src/FerroUI.Themes.WinUI/Controls/` and are listed in the shared WinUI-only list of the Fluent and Simple theme tests (`src/FerroUI.Controls/testing/winui_only.rs`, `WINUI_ONLY_CONTROLS`, added by W4, #333; kept in sync with the table of `winui-controls.md` section 5.1). |
| Generated files are regenerated, never edited (O7) | `compiled_xaml.rs` and `compiled_xaml.xamlmeta` of the WinUI theme, the markup type tables (`scripts/generate_markup_types.py`), the view builders (`scripts/generate_view_builders.py`) and the theme's tracking (`scripts/winui-theme/status.py`) are written by their scripts, after a rebase on `main`. |
| The public API only grows within 0.4 (O9: `cargo-semver-checks` against 0.4.0 and the snapshot of property defaults, W11) | All members below are additions; no default of a registered property changes. |
| Naming | The upstream name never appears in code; `ferro_thread_local!` for thread-local state. |

## 3. What the Calculator needs

Counted in the 30 XAML documents and the C# sources of `src/Calculator` at the Calculator's
revision:

| Feature | Uses |
|---|---|
| `VisualStateGroup` / `VisualState` | 75 / 289, in 23 documents (`App.xaml` 82 states, `Calculator.xaml` 39, `DateCalculator.xaml` 26, `UnitConverter.xaml` 20, the keypads) |
| `Setter Target="Element.Property"` | 1,094; the properties: `FontSize` 536, `IsEnabled` 96, `Style` 89, `Visibility` 54, `MinHeight` 37, `Foreground` 37, `Background` 36, `Height` 27, `Opacity` 21, `Width` 19, `MinWidth` 18, `BorderBrush` 17, `ButtonStyle` 13, `Margin` 10; attached targets `(Grid.Column)` 7, `(Grid.ColumnSpan)` 5, `(Grid.Row)` 4, `(Grid.RowSpan)` 3; `(muxc:AnimatedIcon.State)` 12 |
| `VisualState.StateTriggers` | 55: `AdaptiveTrigger` 46, the Calculator's `ControlSizeTrigger` 8 (`Source="{x:Bind Root}"`, `MinWidth`, `MinHeight`), `AspectRatioTrigger` 1 |
| `Storyboard` | 115: `ObjectAnimationUsingKeyFrames` 526, `DoubleAnimationUsingKeyFrames` 84, `DoubleAnimation` 13; target properties `Foreground` 81, `Background` 73, `BorderBrush` 59, `Opacity` 31, `Visibility` 14, `ScaleX`/`ScaleY` 22, `(UIElement.RenderTransform).(CompositeTransform.ScaleX/Y)` 6 |
| `VisualTransition` | 24 (`From="Pressed" To="Normal"`, the Pivot header and calendar states) |
| Theme animations in states | `PointerDownThemeAnimation` 3, `PointerUpThemeAnimation` 2, `RepositionThemeAnimation` 4, `SplitOpen/CloseThemeAnimation` 2 |
| `VisualStateManager.GoToState` from code | 31 calls in 16 files, on `this` (pages and user controls), on a `Memory` control and on keypad buttons (`GoToState(button, "Pressed")` and `"Normal"` from the keyboard shortcut manager, so the button's own template states are driven from outside) |
| `ListView` / `GridView` | 13 / 5 (history, memory, the unit lists; the `GridView`s are in graphing mode, APP-01) |
| `SwipeControl` / `SwipeItem` | 5 / 4 (delete from history and memory) |
| `Pivot` / `PivotItem` | 11 / 4 (the history and memory dock) |
| High contrast | `AccessibilitySettings.HighContrastChanged`, `Application.Current.Resources.ThemeDictionaries["HighContrast"]`, `HighContrastAdjustment = None`, a `GrapherHighContrast` state |

WinUI's own templates use no triggers at all: neither `GX` nor `controls/dev` has a
`StateTriggers`, an `AdaptiveTrigger` or a `StateTrigger` (only tests do). Triggers are an
application feature; the Calculator is a heavy user.

## 4. Inventory

Status of each WinUI member in FerroUI today: **has** (a counterpart with the WinUI contract),
**partial** (a counterpart that differs, or covers part), **missing**. "PRs" names an open pull
request that changes the status; none of the PRs of section 1 adds any of these types.

### 4.1 Visual state manager (XAML-02)

| WinUI type and members | Behaviour in WinUI | FerroUI `main` | PRs | Status |
|---|---|---|---|---|
| `VisualStateManager`: `static GoToState(Control, String, bool useTransitions) -> bool`; `static GetVisualStateGroups(FrameworkElement)` (the attached `VisualStateGroups`, created on demand); `static Get/SetCustomVisualStateManager`, `CustomVisualStateManagerProperty`; overridable `GoToStateCore(Control, FrameworkElement templateRoot, String, VisualStateGroup, VisualState, bool)`; protected `RaiseCurrentStateChanging/Changed` | The groups sit on `Control.ImplementationRoot`, the control's first visual child (the template root; for a `UserControl` or a `Page`, the root of the content). Names are matched case-sensitively across all groups. An unknown name, or no groups, returns `false` and does nothing. A custom manager on the root gets `GoToStateCore` whether the state exists or not. Already in the state: `useTransitions=false` completes running storyboards and re-applies the setters; either way `true` | Missing. Visual states of the WinUI theme are nested styles on pseudo-classes (theme DEVIATIONS G-1 to G-3). | none. #286's WinUI importer rejects VSM markup with the diagnostic "VisualStateManager requires a semantic implementation" (`winui/migration.rs`), which W9 carries | missing |
| `VisualStateGroup` (content property `States`): `Name`, `States`, `Transitions`, `CurrentState`; events `CurrentStateChanging`, `CurrentStateChanged` (`VisualStateChangedEventHandler`) | One current state per group, null at first. `Changing` when a change begins, `Changed` when it completes (after the transition), both queued, the sender is the template root; an interrupted transition raises no `Changed` | Missing | none | missing |
| `VisualState` (content property `Storyboard`): `Name`, `Storyboard`, `Setters` (`SetterBaseCollection`), `StateTriggers` (`IVector<StateTriggerBase>`) | The setters and the storyboard apply while the state is current | Missing | none | missing |
| `Setter.Target` (`TargetPropertyPath`: `Path`, `Target`; markup `Element.Property` or `Element.(Owner.Property)`) | Applied with `SetAnimatedValue`: in the animation layer, above local values, which it does not overwrite; removed with `ClearAnimatedValue`; refreshed on a theme change and on re-entering the tree. `Setter.Property` in a visual state is a parse error | FerroUI's `Setter` (`src/FerroUI.Base/styling/setter.rs`) has `Property` and `Value` only, resolved against the target type of its style; `BindingPriority::Animation` exists (`src/FerroUI.Base/data/binding_priority.rs`) and `set_value_untyped(.., priority)` returns a removable handle | none | partial: the value layer exists, the targeting does not |
| `VisualTransition` (content property `Storyboard`): `From`, `To`, `GeneratedDuration`, `GeneratedEasingFunction`, `Storyboard` | Chosen by score: +1 when `From` names the current state, +2 when `To` names the target; a transition that names another state of the group is out; else the default transition (no `From`, no `To`); the first wins a tie. Generated animations for the `Double`, `Point` and `Color` targets of the new state (to) and of the old state (back to the base value), with the full duration only for independent animations (opacity, transforms), zero for the rest. New setters and the state storyboard wait for the transition to complete. With system animations off, transitions jump to their end | Missing. Property `Transitions` (`DoubleTransition`, `BrushTransition`, ...) are FerroUI's counterpart in the theme (G-3); `IPlatformSettings::animations_enabled()` exists (#322) | none | missing |
| `VisualStateChangedEventArgs`: `OldState`, `NewState`, `Control` | | Missing | none | missing |
| `StateTriggerBase`: protected constructor, protected `SetActive(bool)` | The extension point of custom triggers (the Calculator's `ControlSizeTrigger` and `AspectRatioTrigger`) | Missing | none | missing |
| `StateTrigger`: `IsActive` | | Missing | none | missing |
| `AdaptiveTrigger`: `MinWindowWidth`, `MinWindowHeight` (default -1) | Compares the size of the window (the XamlRoot), not of the element. Both -1: never active | Missing. Upstream's container queries (`src/FerroUI.Base/styling/container_query.rs`, `<ContainerQuery>` in styles) select on the size of a container; they are styles, not visual states | none | missing |
| Trigger evaluation | Per group: the triggers of a state are OR-ed; the winner is a qualified `StateTrigger` or custom trigger before any `AdaptiveTrigger`, then the larger `MinWindowWidth`, then the larger `MinWindowHeight`; evaluated after the template is applied, on entering the live tree, on a window size change and when a trigger changes; applied only when the winner changes (a manual `GoToState` holds until then); nothing qualified resets the group to the null state; trigger changes always use transitions | Missing | none | missing |
| Markup: the attached collection `VisualStateManager.VisualStateGroups`; `Storyboard.TargetName` in the namescope of the template (or of the page); deferred construction of states (`VisualStateGroupCollectionCustomWriter`) | | The markup compiler and the run-time loader share the XamlX pipeline (`src/Markup/FerroUI.Markup.Xaml.Loader`); attached collection properties created by their getter work (the Behaviors port's `Interaction.Behaviors`, #298). No VSM types, no `Setter.Target` transform | none | missing |
| Automation peers | None relate to visual states | | | n/a |

The storyboard family that visual states use (`Microsoft.UI.Xaml.Media.Animation`), as far as
templates and the Calculator use it:

| WinUI type and members | FerroUI `main` | Status |
|---|---|---|
| `Timeline`: `BeginTime`, `Duration`, `AutoReverse`, `RepeatBehavior`, `FillBehavior`, `SpeedRatio`, `Completed` | `Animation` (`src/FerroUI.Base/animation/animation.rs`): `Delay`, `Duration`, `PlaybackDirection`, `IterationCount`, `FillMode`, `SpeedRatio`, a task that completes; per-target, not per-property | partial |
| `Storyboard`: children, attached `TargetName`, `TargetProperty`; `Begin`, `Stop`, `Pause`, `Resume`, `Seek`, `SeekAlignedToLastTick`, `SkipToFill`, `GetCurrentState`, `GetCurrentTime` | none (a FerroUI `Animation` runs on one target given in code) | missing |
| `DoubleAnimation`, `ColorAnimation`, `PointAnimation` (`From`, `To`, `By`, `EasingFunction`, `EnableDependentAnimation`) | key frames of `Animation` with the animators of double, colour and point | partial |
| `ObjectAnimationUsingKeyFrames` + `DiscreteObjectKeyFrame`; `Double/Color/PointAnimationUsingKeyFrames` + `Discrete`, `Linear`, `Spline`, `Easing` key frames | `KeyFrame` with `Cue` and setters; `KeySpline` (`src/FerroUI.Base/animation/key_spline.rs`, the same `"x1,y1,x2,y2"` syntax) | partial |
| `EasingFunctionBase` with `EasingMode` (`CubicEase`, `QuadraticEase`, `ExponentialEase`, `SineEase`, `BackEase`, `BounceEase`, `CircleEase`, `ElasticEase`, `PowerEase`, `QuarticEase`, `QuinticEase`) | one class per mode (`CubicEaseOut`, `SineEaseInOut`, ...; `src/FerroUI.Base/animation/easings`) | partial (other names) |
| `RepeatBehavior`, `FillBehavior`, `Duration` (`Automatic`, `Forever`) | `IterationCount`, `FillMode`, `TimeSpan` | partial |
| Theme animations (`PointerDownThemeAnimation`, `FadeInThemeAnimation`, `RepositionThemeAnimation`, ...) | none (theme G-22) | missing (not in this plan) |

### 4.2 ListView and GridView (CONTROL-02)

| WinUI type and members | FerroUI `main` | PRs | Status |
|---|---|---|---|
| `ListViewBase : Selector`: `SelectionMode` (`ListViewSelectionMode`: `None`, `Single`, `Multiple`, `Extended`), `SelectedItems`, `SelectedRanges`, `IsItemClickEnabled`, `ItemClick` (`ItemClickEventArgs.ClickedItem`), `ScrollIntoView(item)`, `ScrollIntoView(item, ScrollIntoViewAlignment)`, `SelectAll`, `SelectRange`, `DeselectRange`, `Header`/`HeaderTemplate`/`HeaderTransitions`, `Footer`/`FooterTemplate`/`FooterTransitions`, `ContainerContentChanging`, `ChoosingItemContainer`, `ChoosingGroupHeaderContainer`, `IsMultiSelectCheckBoxEnabled`, `SingleSelectionFollowsFocus`, `ShowsScrollingPlaceholders`, `CanDragItems`, `CanReorderItems`, `ReorderMode`, `DragItemsStarting`/`Completed`, `IncrementalLoadingTrigger`/`Threshold`, `DataFetchSize`, `LoadMoreItemsAsync`, `IsSwipeEnabled` (a no-op in WinUI 3, `ListViewBase_Partial.cpp:165`), semantic zoom (`ISemanticZoomInformation`), `PrepareConnectedAnimation`, `TryStartConnectedAnimationAsync`, `SetDesiredContainerUpdateDuration` | `SelectingItemsControl` (`primitives/selecting_items_control.rs`) is FerroUI's `Selector`: selected index, item, items and value, the selection model, `SelectionMode` flags (`Single`, `Multiple`, `Toggle`, `AlwaysSelected`; no `None`), `AutoScrollToSelectedItem`, `WrapSelection`, the virtual `update_selection_from_event`. `ListBox` adds `SelectAll`/`UnselectAll` and the `PART_ScrollViewer` and `PART_ItemsPresenter` parts. `ItemsControl::scroll_into_view(index)` and `scroll_into_view_item` (no alignment). `ConnectedAnimationService` exists. No item click, no header or footer, no drag reorder, no incremental loading, no semantic zoom | #327 fixes `SelectionModel::select_all` without items | partial (selection and virtualization), missing (the ListViewBase surface) |
| `ListViewBaseItem : SelectorItem`, `ListViewItem`, `GridViewItem`; `ListViewItemTemplateSettings`, `GridViewItemTemplateSettings` (`DragItemsCount`) | `ListBoxItem` (`list_box_item.rs`): `IsSelected`, `:pressed`, `:selected`, pointer and key selection through its owner | | partial |
| `ListViewItemPresenter` (about 60 brush and layout properties; the chrome is native code, `ListViewBaseItemChrome.cpp`, 6.4k lines), `GridViewItemPresenter`, the check and indicator modes | The WinUI theme draws the chrome with elements in keyed `ListBoxItem` themes (`Controls/ListBoxItem.xaml`: `DefaultListViewItemStyle`, `ListViewItemExpanded`; theme DEVIATIONS LBI-6 to LBI-9) | | partial (in the theme, for `ListBoxItem`) |
| `ItemsStackPanel` (`Orientation`, `CacheLength`, `First/LastVisibleIndex`, `First/LastCacheIndex`, `ItemsUpdatingScrollMode`, `GroupPadding`, `GroupHeaderPlacement`, `AreStickyGroupHeadersEnabled`) on `ModernCollectionBasePanel` (18.5k lines) | `VirtualizingStackPanel` (1.8k lines): `Orientation`, `CacheLength`, `FirstRealizedIndex`, `LastRealizedIndex`, recycling by `RecycleKey`, snap points, `scroll_into_view(index)` | | partial |
| `ItemsWrapGrid` (`MaximumRowsOrColumns`, `ItemWidth`, `ItemHeight`, `Orientation`, ...) | `WrapPanel` (not virtualizing). `UniformGridLayout` in #269 is virtualizing, on ItemsRepeater | #269 | partial |
| `ListViewHeaderItem`, `GridViewHeaderItem` (groups) | none (no grouped collections in `ItemsControl`) | | missing |
| Styles: `ListView` (GX 9284: `IsSwipeEnabled`, `ItemContainerTransitions` AddDelete, Content, Reorder, Entrance; `ItemsStackPanel`; `Border > ScrollViewer > ItemsPresenter` with header and footer), `GridView` (GX 9348), `DefaultListViewItemStyle` (CS, a single `ListViewItemPresenter`), `ListViewItemExpanded` (CS, elements with 42 states), `DefaultGridViewItemStyle` | `ListBox.xaml`, `ListBoxItem.xaml` of the WinUI theme; ListView and GridView waived (G-23, `waivers.json`) | | missing (ListView, GridView); partial (items) |
| Peers: `ListViewBaseAutomationPeer` (DropTarget), `ListViewAutomationPeer`, `GridViewAutomationPeer` (List; `OnCreateItemAutomationPeer`), `ListViewItemDataAutomationPeer`, `GridViewItemDataAutomationPeer` (ScrollItem, through `ItemAutomationPeer` and `SelectorItemAutomationPeer`: ListItem, SelectionItem, VirtualizedItem), `ListViewBaseItemAutomationPeer` (ListItem, Drag, Invoke), `ListViewItemAutomationPeer`, `GridViewItemAutomationPeer` | `ListBoxAutomationPeer`, `ListItemAutomationPeer`, `SelectingItemsControlAutomationPeer`, `ItemsControlAutomationPeer` | W1 (#330) completes the UIA range value, unrelated | partial |
| `ItemsView` (`controls/dev/ItemsView`: `ItemsSource`, `ItemTemplate` (an `IElementFactory` whose root must be an `ItemContainer`), `Layout`, `SelectionMode` (`ItemsViewSelectionMode`), `IsItemInvokedEnabled`, `ItemInvoked`, `SelectionChanged`, `ScrollView`, `CurrentItemIndex`, `StartBringItemIntoView`, `Select`/`Deselect`/`SelectAll`/`InvertSelection`), `ItemContainer` (`Child`, `IsSelected`, `CanUserSelect`, `CanUserInvoke`, `MultiSelectMode`, `ItemInvoked`), `ItemsViewAutomationPeer`, `ItemContainerAutomationPeer` | none; `ScrollView` (the WinUI scroller it hosts) is also missing | #269 brings `ItemsRepeater`, the layouts, `ItemCollectionTransitionProvider` and the hierarchical `SelectionModel`; not ItemsView, ItemContainer or ScrollView | missing |

The size of the WinUI family, for scale: about 51k lines for ListViewBase, its items, the chrome,
`ModernCollectionBasePanel` and the layout strategies, plus the 5.3k of `Selector` and 4k of
`ItemsControl`. What a Calculator list needs is a small part of it: `SelectionMode="None"`,
`IsItemClickEnabled` with `ItemClick`, `ItemTemplate`, `ItemContainerStyle`,
`ItemContainerTransitions`, `ScrollIntoView`, phase 0 of `ContainerContentChanging`, a vertical
virtualizing stack, and the List, ListItem and Invoke patterns.

### 4.3 SwipeControl (CONTROL-02)

| WinUI type and members | Behaviour in WinUI | FerroUI `main` | PRs | Status |
|---|---|---|---|---|
| `SwipeControl : ContentControl`: `LeftItems`, `RightItems`, `TopItems`, `BottomItems`, `Close()` | Composition: a `VisualInteractionSource` (redirection `CapableTouchpadOnly`) found on the nearest `ListViewItemPresenter` within five parents, else the control; an `InteractionTracker` with inertia resting values (open to the size of the items when past the threshold, `min(size - 1, 100)`, else closed); expression animations for the content translation, the execute mode and the clip; redirected for touch only; one control open at a time (a thread-local last control); closes on a tap outside, a key, a change of the XamlRoot; mixing horizontal and vertical items throws; no handler for a parent scrolling | `InteractionTracker` and `VisualInteractionSource` with `try_redirect_for_manipulation` (any pointer type) in `src/FerroUI.Base/rendering/composition/interactions/`; `ExpressionAnimation`; `ElementCompositionPreview` in `src/FerroUI.Controls/hosting`; `SwipeGestureRecognizer` in Base | #326 extends the manipulation property set of the scroll viewer (zoom); nothing for swipe | missing |
| `SwipeItems : IVector<SwipeItem>`: `Mode` (`SwipeMode`: `Reveal`, `Execute`; execute allows one item) | | none | | missing |
| `SwipeItem`: `Text`, `IconSource`, `Background`, `Foreground`, `Command`, `CommandParameter`, `BehaviorOnInvoked` (`SwipeBehaviorOnInvoked`: `Auto`, `Close`, `RemainOpen`), `Invoked` (`SwipeItemInvokedEventArgs.SwipeControl`) | Drawn as `AppBarButton`s with `SwipeItemStyle`; `InvokeSwipe` raises `Invoked`, runs the command, closes for `Auto` and `Close` | `AppBarButton` exists (command bar); `IconSource` is missing on `main` | **W4 (#333)** adds `IconSource`, `FontIconSource`, `SymbolIconSource`, `PathIconSource` | missing (needs W4) |
| Template `SwipeControl.xaml`, resources `SwipeControl_themeresources.xaml` (`SwipeItemBackground`, `SwipeItemPreThresholdExecute*`, `SwipeItemPostThresholdExecute*`, `SwipeItemStyle`) | | waived (G-23, `waivers.json`) | | missing |
| Automation peer | none in WinUI | | | n/a |

### 4.4 Pivot

| WinUI type and members | Behaviour in WinUI | FerroUI `main` | Status |
|---|---|---|---|
| `Pivot : ItemsControl`: `Title`, `TitleTemplate`, `LeftHeader`, `LeftHeaderTemplate`, `RightHeader`, `RightHeaderTemplate`, `HeaderTemplate`, `SelectedIndex`, `SelectedItem`, `IsLocked`, `IsHeaderItemsCarouselEnabled`, `HeaderFocusVisualPlacement` (`ItemHeaders`, `SelectedItemHeader`); events `SelectionChanged`, `PivotItemLoading`, `PivotItemLoaded`, `PivotItemUnloading`, `PivotItemUnloaded` (`PivotItemEventArgs.Item`); attached `SlideInAnimationGroup` (`PivotSlideInAnimationGroup`) | Items side by side in a `ScrollViewer` whose panel is 1000 times the item width, re-centred by a state machine (`PivotStateMachine`) so that a flick wraps around; with the header carousel off, or when the headers fit, static headers and no wrapping; the headers rotate so that the selected one comes first, with a parallax tied to the scroll offset (`PivotHeaderManager`, `PivotCurveGenerator`); previous and next buttons on pointer devices; content slides in by group | `Carousel` (`carousel.rs`, on `SelectingItemsControl`): `IsSwipeEnabled`, `IsSwiping`, `ViewportFraction`, `PageTransition`, `Next`, `Previous`, with `VirtualizingCarouselPanel` (swipe with velocity and rubber band, wrapping by `WrapSelection`); `TabControl`/`TabItem`; `PipsPager` | missing (parts exist) |
| `PivotItem : ContentControl`: `Header` | | `TabItem` (a headered content control of another control) | missing |
| `PivotHeaderItem : ContentControl`, `PivotHeaderPanel : Canvas`, `PivotPanel : Panel` | Header items generated by the Pivot (not item containers); states `Disabled`, `Unselected`, `UnselectedLocked`, `Selected`, `(Un)SelectedPointerOver`, `(Un)SelectedPressed` | none | missing |
| Style `Pivot` (CS `Pivot_themeresources.xaml` 207: parts `RootElement`, `TitleContentControl`, `LeftHeaderPresenter`, `RightHeaderPresenter`, `ScrollViewer`, `Panel`, `PivotLayoutElement`, `Header`, `StaticHeader`, `HeaderClipper`, `FocusFollower`, `PreviousButton`, `NextButton`, `PivotItemPresenter`; states `Orientation`, `NavigationButtonsVisibility`), `PivotItem`, `PivotHeaderItem`; about 69 resources | | waived (G-23; the scroll viewer template of Pivot is theme SCV-9) | missing |
| Peers: `PivotAutomationPeer` (Tab; Selection, Scroll), `PivotItemAutomationPeer` (TabItem), `PivotItemDataAutomationPeer` (SelectionItem, ScrollItem, VirtualizedItem) | | `SelectingItemsControlAutomationPeer`, `ListItemAutomationPeer`; no tab peers | missing |

### 4.5 System high contrast (XAML-03)

| WinUI behaviour | FerroUI `main` | PRs | Status |
|---|---|---|---|
| Detection: `SystemParametersInfo(SPI_GETHIGHCONTRAST)` and `HCF_HIGHCONTRASTON`; the scheme classified as Black, White or Custom from `GetSysColor(COLOR_WINDOW)` and `COLOR_WINDOWTEXT` | `PlatformColorValues::contrast_preference` (`NoPreference`, `High`) reported by Windows (`IAccessibilitySettings.HighContrast`, `win32_platform_settings.rs`), the browser (`prefers-contrast: more`), iOS (`accessibilityContrast`) and Android (`high_text_contrast_enabled`). macOS compares `effectiveAppearance.name` with the high contrast appearance names (`native/FerroUI.Native/src/OSX/PlatformSettings.mm`), which "Increase contrast" does not select, so it probably never reports `High`; `accessibilityDisplayShouldIncreaseContrast` is not read. Linux reads `color-scheme` and `accent-color` of `org.freedesktop.appearance`, not `contrast` | #325 reads the contrast preference for the Reveal fallback (`reveal_brush.rs`) and adds `IsHighContrast` to `SystemBackdropConfiguration` | partial |
| Selection: with high contrast on, the theme dictionary `HighContrastBlack`/`HighContrastWhite`/`HighContrastCustom`, falling back to `HighContrast` (the only key WinUI's themes define), whatever `RequestedTheme` says (a subtree's `RequestedTheme` only picks the White or Black key); `ActualThemeChanged` is suppressed while high contrast stays on | `ThemeVariant::update_actual_theme_variant` (`src/FerroUI.Base/styling/theme_variant.rs`) uses the requested variant, else Light or Dark of the platform; `contrast_preference` is read nowhere in Base, Controls or the themes. The WinUI theme's `HighContrast` dictionaries (61, keyed `{x:Static winui:WinUITheme.HighContrast}`, a variant inheriting Dark) apply only when an application requests the variant (theme G-5) | | missing |
| System colours: 15 `SystemColor*Color` resources and brushes built from `GetSysColor` (`FrameworkTheming::RebuildColorAndBrushResources`), in a dictionary of the framework; the WinUI theme names 8 of them (Window, WindowText, ButtonFace, ButtonText, Highlight, HighlightText, Hotlight, GrayText) about 1,150 times | `Resources/SystemResources.xaml` of the WinUI theme: fixed values, Windows defaults in `Default`/`Light`, High Contrast Black in `HighContrast` (G-6); no backend reports system colours, only the accent (`Accents/system_accent_colors.rs` follows `color_values_changed` for the accent) | | missing |
| Live update: `UISettings.ColorValuesChanged` (and the island's `ThemeChanged`) re-reads theme, contrast and accent, rebuilds the system colours and re-resolves every `ThemeResource` | `IPlatformSettings::color_values_changed`; Windows re-reads on `WM_SETTINGCHANGE` (`ImmersiveColorSet`, `WindowsThemeElement`); macOS by KVO on `NSApp.effectiveAppearance`; Linux on the portal's `SettingChanged` for its two keys; `{DynamicResource}` re-resolves on resource changes | | partial |
| `FrameworkElement.HighContrastChanged`; `Application/FrameworkElement.HighContrastAdjustment` (the text backplate of `TextBlock`, `RichTextBlock`, `TextBox`) | none | | missing (not in this plan, section 7.5) |

## 5. Design: visual states

### 5.1 Principles

- **A separate value layer, not a second styling system.** Visual states apply their values at
  `BindingPriority::Animation`, through the removable handles of the property store, exactly the
  layer WinUI applies them in (`SetAnimatedValue`, above local values, which stay untouched).
  Styles, control themes, pseudo-classes and their priorities are not changed: below an applied
  visual state the styles keep computing their values, and they show again when the state is left.
- **Nothing runs without groups.** Every entry point returns at once for an element without
  `VisualStateManager.VisualStateGroups`. No upstream template has any, so upstream behaviour,
  performance and allocations are unchanged.
- **FerroUI names in markup.** The VSM is a FerroUI feature with WinUI's object model; markup
  names FerroUI types and properties (`Target="Panel.IsVisible" Value="False"`, `Theme`, not
  `Style`). Translating WinUI's own names (`Visibility`, `UIElement.Opacity`, `Style`,
  `FrameworkElement`) is the job of the WinUI importer (`ferroui-markup-xaml-winui`, W9), which
  then maps visual state markup instead of rejecting it.

### 5.2 Types and location

All in `ferroui-controls` (O1; the manager needs `Control`, `TopLevel` and templates), in a module
`visual_state` registered under the namespace `FerroUI.Controls`, except the two styling types
that `Setter` refers to, which go to `ferroui-base` (`styling/target_property_path.rs`):

| Type | Shape |
|---|---|
| `VisualStateManager` | `ferro_class!` with the virtual `go_to_state_core(control, template_root, state_name, group, state, use_transitions) -> bool`; statics `go_to_state(control: &Control, state_name: &str, use_transitions: bool) -> bool`, `get_visual_state_groups(element) -> VisualStateGroupCollection` (creates the collection, as `Interaction::get_behaviors` does), `visual_state_groups_property()`, `get/set_custom_visual_state_manager`, `custom_visual_state_manager_property()`; `raise_current_state_changing/changed` for derived managers |
| `VisualStateGroupCollection` | a `FerroList<VisualStateGroup>` with the namescope it was loaded in (5.4) |
| `VisualStateGroup` | `Name`, `States` (content), `Transitions`, `CurrentState`; events `current_state_changing`, `current_state_changed` |
| `VisualState` | `Name`, `Setters` (a `SetterBaseCollection`), `Storyboard` (content), `StateTriggers` |
| `VisualTransition` | `From`, `To`, `GeneratedDuration` (`Option<TimeSpan>`), `GeneratedEasingFunction` (`Easing`), `Storyboard` (content) |
| `VisualStateChangedEventArgs` | `old_state`, `new_state`, `control` |
| `StateTriggerBase` | `ferro_class!` with `set_active(&self, bool)`, public in Rust and documented as the call of derived triggers (WinUI's protected member); the trigger keeps a weak link to the groups it belongs to and asks them to re-evaluate |
| `StateTrigger` | `IsActive` |
| `AdaptiveTrigger` | `MinWindowWidth`, `MinWindowHeight` (default -1.0, as WinUI) |
| `Setter.Target` (Base) | an opt-in member of the upstream-ported `Setter`: `target() -> Option<TargetPropertyPath>`, default `None`, which keeps upstream. A style that holds a setter with a target is an error of the compiler, as `Setter.Property` in a visual state is an error in WinUI. A row in "WinUI additions to upstream controls" with the default test |
| `TargetPropertyPath` (Base) | the target name, or the target object, and the resolved property (plain or attached) |

`Name` of the group, the state and the transition is their run-time name (`x:Name` sets it, as
WinUI's `x:Name` does).

### 5.3 `GoToState`

`VisualStateManager::go_to_state(control, name, use_transitions)`:

1. **The root.** WinUI's root is the control's first visual child: the template root, or the
   content of a `UserControl`. In FerroUI a `UserControl` is templated (its template root is the
   content presenter's border), so the root is the template root when it has groups, else the
   `Content` of a content control when that is a control with groups (the user control and page
   pattern of the Calculator: `<UserControl><Grid><VisualStateManager.VisualStateGroups>`).
   Recorded as a Representation deviation: the same groups are found.
2. No root, or no groups: `false`. A custom manager on the root: its `go_to_state_core`, whether
   the state exists or not.
3. Find the state by exact name across the groups (`false` if none).
4. Already current: with `use_transitions == false`, complete running storyboards and apply the
   setters again; `true`.
5. Otherwise change the group's state (5.5) and return `true`.

`go_to_state` may be called before the template is applied (it returns `false`, as WinUI);
controls call it from `on_apply_template` with `use_transitions == false`, as WinUI's
`UpdateVisualState(false)`.

### 5.4 Names: setter targets and storyboard targets

- The `VisualStateGroupCollection` keeps the namescope of the document or template that loaded it
  (the `INameScope` of the markup's service provider). Targets are resolved in it when a state is
  first entered, so a name declared after the groups resolves (forward references are the rule:
  the groups are the first child of the root). That is the template namescope for a template and
  the page namescope for a user control, as in WinUI.
- Groups built in Rust (no namescope captured) resolve in `NameScope::find_name_scope` of the
  root.
- A target that does not resolve: the setter or animation is skipped and a warning is logged
  (WinUI raises an error at run time); the compiler reports it as an error when it can see the
  namescope (5.9).

### 5.5 Changing state

The order is WinUI's `VisualStateManagerActuator::ChangeVisualState`:

1. Raise `CurrentStateChanging` (queued on the dispatcher, sender the root).
2. With `use_transitions`, choose the transition (5.6). A transition is zero-length when it has no
   `GeneratedDuration` (or zero) and no storyboard (or one of zero length).
3. Zero-length: apply the new setters, start the new state's storyboard, stop the old storyboard,
   remove the old setters whose property the new state does not set; set `CurrentState`; raise
   `CurrentStateChanged`.
4. With a transition: start the transition's storyboard and the generated one; when all have
   completed, do step 3. Interrupting it (another `GoToState`) stops the transition and raises no
   `Changed` for it.
5. With `IPlatformSettings::animations_enabled() == false`, transitions jump to their end (WinUI's
   behaviour with system animations off).

Setters are applied with `set_value_untyped(property, value, BindingPriority::Animation)` and
removed by disposing the returned handle; a setter whose value is a binding or a resource
reference (`{DynamicResource}`, `{Binding}`, `{TemplateBinding}`) is bound at the same priority.
A storyboard that targets a property a setter of the same state sets wins while it runs, as in
WinUI. Leaving a state stops its storyboard and drops its values at once (WinUI does not animate
back unless a transition says so).

### 5.6 Transitions

`VisualTransition` selection is WinUI's score (`VisualTransitionTableOptimizedLookup`): +1 for a
`From` equal to the current state, +2 for a `To` equal to the target state, a transition naming
another existing state of the group is excluded, the default transition (no `From`, no `To`)
when nothing scores, the first of equal scores. The generated storyboard
(`DynamicTransitionStoryboardGenerator`) animates, over `GeneratedDuration` with
`GeneratedEasingFunction`, the double, colour and point values that the target state's animations
and setters set (to their values) and that the old state set but the new one does not (back to the
base value), leaving out what the transition's own storyboard animates. As in WinUI, only
independent animations (opacity, render transform and translation values) get the full duration;
the others get zero.

### 5.7 State triggers

- When the groups are attached to a visual tree, after the template is applied, when the
  `ClientSize` of the `TopLevel` changes and when a trigger calls `set_active` (or a property of
  `StateTrigger`/`AdaptiveTrigger` changes), each group with triggers is evaluated.
- The winner: among the states with a qualified trigger (several triggers of a state are OR-ed),
  a `StateTrigger` or custom trigger before any `AdaptiveTrigger`; among adaptive triggers the
  larger `MinWindowWidth`, then the larger `MinWindowHeight`; then the order of the source (WinUI's
  order there is unspecified, its sort is not stable). An `AdaptiveTrigger` with both values -1
  never qualifies.
- The group changes state only when the winner changes, always with transitions (WinUI's
  `m_isInitialEvaluation` is never set, so its trigger changes always use them). No winner resets
  the group to no state (its values are removed). A manual `GoToState` holds until the winner
  next changes.
- `AdaptiveTrigger` compares with the client size of the `TopLevel` (WinUI's XamlRoot size), not
  the size of the element.
- **The extension point.** A custom trigger derives from `StateTriggerBase` in Rust
  (`ferro_class!(ControlSizeTrigger: StateTriggerBase)`), registers its properties with
  `ferro_properties!` and calls `set_active`. The Calculator's `ControlSizeTrigger` ports one to one:
  `Source` (a control, `[ResolveByName]` so markup can write `Source="Root"` for the
  `{x:Bind Root}` of the source), `MinWidth`, `MinHeight` (default -1), the `SizeChanged`
  subscription and the same rule. A test of the crate ports it as the example of the extension
  point, and the catalog demo uses it.

### 5.8 Storyboards

The storyboard family of section 4.1 is ported as new types in `ferroui-controls`, module
`animation::storyboard`, namespace `FerroUI.Animation` (no name conflicts: only `KeySpline`
exists, and it is reused): `Timeline`, `Storyboard` (with the attached `TargetName` and
`TargetProperty`), `DoubleAnimation`, `ColorAnimation`, `PointAnimation`,
`ObjectAnimationUsingKeyFrames`, `DoubleAnimationUsingKeyFrames`, `ColorAnimationUsingKeyFrames`,
`PointAnimationUsingKeyFrames`, the `Discrete`, `Linear`, `Spline` and `Easing` key frames,
`EasingFunctionBase` with `EasingMode` and the eleven WinUI easing functions (as `IEasing`
implementations, so they also fit FerroUI's transitions), `RepeatBehavior`, `FillBehavior`,
`Duration`.

- **On FerroUI's clocks.** A storyboard runs its timelines on the global clock of
  `src/FerroUI.Base/animation`, each animation writing its property at
  `BindingPriority::Animation` with the existing animators of double, colour and point (and a
  discrete object setter); `Completed`, `Begin`, `Stop`, `Pause`, `Resume`, `Seek` and
  `SkipToFill` map to the instance's clock. `FillBehavior.HoldEnd` holds the last value until the
  storyboard stops; `Stop` drops the values.
- **Property paths.** `TargetProperty` is a FerroUI property name (`Opacity`, `Background`),
  an attached property (`(Grid.Row)`), or a path through sub-objects
  (`(Visual.RenderTransform).(ScaleTransform.ScaleX)`, resolved at start by walking the values).
  FerroUI has no `CompositeTransform`; the Calculator's six paths through it become
  `ScaleTransform` paths in its markup (an application adaptation, not a framework type).
- **Not in scope:** theme animations (`PointerDownThemeAnimation` and the others, theme G-22),
  `Storyboard.TargetName` across namescopes, dependent animation throttling
  (`EnableDependentAnimation` is accepted and has no effect: FerroUI animates every property on the
  UI thread).

### 5.9 Markup: compiler and run-time loader

The compiler (which writes `compiled_xaml.rs`) and the run-time loader share the XamlX pipeline of
`src/Markup/FerroUI.Markup.Xaml.Loader`, so one set of transformers serves both:

- `VisualStateManager.VisualStateGroups` is an attached collection property whose getter creates
  the collection; property element syntax adds the groups to it, as `<Interaction.Behaviors>`
  does. The loader passes its namescope to the collection (5.4).
- A new transformer runs before `FerroXamlIlSetterTransformer`: for a `Setter` with `Target` in
  `VisualState.Setters` it parses `Element.Property` or `Element.(Owner.Property)`, finds the
  element of that `x:Name` in the same namescope of the document (the enclosing template, or the
  document root), takes its type as the target type, resolves the property and converts the value
  as for a style setter (literals, `{StaticResource}`, `{DynamicResource}`, bindings), and writes
  the `TargetPropertyPath`. An unknown element or property is a diagnostic with a code of
  `ferro_xaml_diagnostic_codes.rs`; a `Setter` with `Target` outside a visual state is an error.
- The same resolution types the values of key frames: `Storyboard.TargetName` and
  `Storyboard.TargetProperty` are resolved where the compiler can see the element, so
  `DiscreteObjectKeyFrame Value="False"` becomes a boolean for `IsVisible`. Where it cannot (a path
  through an object built in code), the value is converted at run time from its text by the
  property's type, and a failure is logged.
- Generated files: the type tables (`scripts/generate_markup_types.py`), the `rust_paths.rs` of the
  crates and the view builders (`scripts/generate_view_builders.py`) are regenerated; no theme's
  `compiled_xaml.rs` changes in the visual state PRs (no theme document uses them).
- **Deferred states (later):** WinUI builds the objects of a state only when the state is first
  entered (`VisualStateGroupCollectionCustomWriter`). The compiler can emit each state's setters
  and storyboard as a deferred builder, like a template. Not needed for applications; required
  before templates of many-instance controls use visual states (section 5.11).
- Hot reload (#313): a reloaded document builds new groups; the old ones drop their values when
  their root leaves the tree (as WinUI completes every group when its collection leaves the tree).

### 5.10 Controls that drive states: the bridge for upstream-ported controls

A WinUI template with visual state markup on a FerroUI control works only if the control calls
`GoToState`. The WinUI-only controls of this plan do (their ported C++ calls it, or the
pseudo-class equivalent, decision D2). The upstream-ported controls never do: a Calculator button
template with `CommonStates` (and its `GoToState(button, "Pressed")` from the keyboard manager)
would never leave `Normal` on its own.

The bridge (decision D1): a table of state mappers in `ferroui-controls`, one per control family,
each a port of the `ChangeVisualState` of the WinUI control (`ButtonBase`, `ToggleButton`,
`CheckBox`, `RadioButton`, `ListBoxItem`, `ListViewItem`, `TextBox`, `ComboBox`, ...), that turns
the control's state (its pseudo-classes and properties) into `GoToState` calls. The manager
installs a control's mapper when it applies a template whose root has groups (and only then), and
re-runs it on the control's pseudo-class changes (`Classes::add_listener`) with transitions. A
control without groups in its template is untouched; a mapper never changes pseudo-classes.

### 5.11 Can WinUI templates keep their visual state markup, and should the WinUI theme migrate?

**They can**, once VSM-1 to VSM-4 (section 8) and, for upstream-ported controls, the bridge are
in: the groups, states, setters, storyboards and transitions of a WinUI template then work as
written, with FerroUI property names. What still needs adapting is everything else theme
DEVIATIONS records for templates: property names and types (`Visibility` to `IsVisible`, the
`ScrollViewer` modes), panels with borders (G-17), template-local resources (G-18), `x:Load`,
theme animations (G-22), the native `ListViewItemPresenter`, the focus visuals (G-14).

**Decided (D2): no migration of the existing control themes in 0.4.**

- The current themes of about 100 controls are calibrated against native WinUI in the Windows VM
  (`winui-theme-gallery.md`); a migration is a rewrite of every control document and of
  `compiled_xaml.rs`, with a new round of VM comparison, for little visible gain: the values of
  a state are the same either way (G-1, G-2).
- What visual states would add is fidelity of timing: begin times and storyboard animations that
  the property transitions cannot express (G-3), and the transitions of `VisualTransition`. Those
  differences are small and local.
- Cost: every instance of a template builds its groups; until the deferred states of 5.9 exist,
  a list of 1,000 items with `ListViewItemExpanded`'s 42 states builds 42,000 state objects.
- So: new WinUI-only controls of this plan keep the pseudo-class style of the theme (consistency
  with the W-series and the theme tests). After VSM-5, a pilot ports one control (ToggleSwitch,
  whose WinUI states have begin times) both ways in a branch, measures template build time,
  `compiled_xaml.rs` size and the VM comparison, and the owner decides about migrating, control
  family by control family, for 0.5.

## 6. Design: ListView, GridView and SwipeControl

### 6.1 What they build on (decision D3)

Decided (D3): **both, in sequence**.

- **ListView and GridView in `ferroui-controls`, on `SelectingItemsControl` and the existing
  virtualizing panels.** WinUI's `ListViewBase` derives from `Selector`, whose counterpart is
  `SelectingItemsControl`; FerroUI's `VirtualizingStackPanel` is the counterpart of
  `ItemsStackPanel` (without grouping), with recycling and `scroll_into_view` already there. This
  is what the Calculator uses, and it needs no feature crate and no wait for #269.
- **ItemsView and ItemContainer later, in `ferroui-controls-repeater`,** after #269 is merged:
  they are WinUI's newer list built on `ItemsRepeater`, `SelectionModel` and `ScrollView`
  (missing too). The Calculator does not use them; they are listed as a follow-up (LV-4), not a
  dependency.

### 6.2 Types

| Type | Base | Notes |
|---|---|---|
| `ListViewBase` | `SelectingItemsControl` | `SelectionMode` of type `ListViewSelectionMode` (its own property: `SelectingItemsControl`'s `SelectionMode` flags stay unregistered on it, so the name resolves to one property; the mode is translated to the flags: `Single`, `Multiple` and `Extended` to `Single`, `Multiple \| Toggle` and `Multiple`, `None` to `Single` with selection from input turned off), `IsItemClickEnabled`, `ItemClick`, `SelectedRanges` (from the selection model's ranges), `SelectAll`, `SelectRange`, `DeselectRange`, `ScrollIntoView(item, ScrollIntoViewAlignment)`, `Header`/`HeaderTemplate`/`Footer`/`FooterTemplate` (presented by the template), `ContainerContentChanging` (phase 0, `InRecycleQueue`, `Handled`), `ChoosingItemContainer`, `SingleSelectionFollowsFocus`, `IsMultiSelectCheckBoxEnabled`, `ShowsScrollingPlaceholders` (accepted, no placeholders). The interaction is a port of `ListViewBase_Partial_Interaction.cpp` (`OnItemPrimaryInteractionGesture`, `OnItemClick`, `OnKeyDown`) in an override of `update_selection_from_event` |
| `ListView`, `GridView` | `ListViewBase` | default panels `ItemsStackPanel` and `ItemsWrapGrid` |
| `ListViewBaseItem` | `ListBoxItem` | WinUI's `ListViewBaseItem : SelectorItem`; FerroUI's `ListBoxItem` is its `SelectorItem`. Deriving from it keeps the keyed `ListBoxItem` themes of the WinUI theme applicable (decision D4) and reuses its selection plumbing |
| `ListViewItem`, `GridViewItem` | `ListViewBaseItem` | with `TemplateSettings` (`DragItemsCount`, 0) |
| `ItemsStackPanel` | `VirtualizingStackPanel` | adds `FirstVisibleIndex`, `LastVisibleIndex`, `FirstCacheIndex`, `LastCacheIndex`, `ItemsUpdatingScrollMode` |
| `ItemsWrapGrid` | `WrapPanel` | `MaximumRowsOrColumns`, `ItemWidth`, `ItemHeight`, `Orientation`; not virtualizing at first (decision D5) |
| `ItemClickEventArgs`, `ContainerContentChangingEventArgs`, `ChoosingItemContainerEventArgs`, `ListViewSelectionMode`, `ScrollIntoViewAlignment`, `ItemsUpdatingScrollMode`, `ListViewReorderMode`, `IncrementalLoadingTrigger` | | the enums with all their WinUI values; the members that use the reorder and incremental loading values are accepted and recorded as missing |
| Peers `ListViewBaseAutomationPeer`, `ListViewAutomationPeer`, `GridViewAutomationPeer`, `ListViewBaseItemAutomationPeer`, `ListViewItemAutomationPeer`, `GridViewItemAutomationPeer` | `SelectingItemsControlAutomationPeer`, `ListItemAutomationPeer` | List and ListItem control types, SelectionItem, Invoke when `IsItemClickEnabled`, ScrollItem |

Left out and recorded: drag and reorder (`CanDragItems`, `CanReorderItems`, the drag events, 2.4k
lines of WinUI), incremental loading, semantic zoom, grouping and sticky headers, the connected
animation helpers (FerroUI's `ConnectedAnimationService` serves them from application code), the
scrolling placeholders.

### 6.3 Templates and resources (WinUI theme)

- `Controls/ListView.xaml` and `Controls/GridView.xaml` from GX 9284 and 9348:
  `Border > ScrollViewer > ItemsPresenter` with the header and footer; the item container
  transitions (AddDelete, Content, Reorder, Entrance) as the theme's G-22 rule for theme
  transitions, or recorded as missing.
- The items: `DefaultListViewItemStyle` becomes the implicit theme of `ListViewItem`. Both keyed
  themes stay targeted at `ListBoxItem` (so an application that uses them on `ListBoxItem` keeps
  working, and `ListViewItem` gets them by inheritance); LBI-8 ("FerroUI has no ListView") is
  updated. The chrome stays drawn with elements (decision D6).
- The ListView and GridView documents leave `waivers.json`; G-23 drops them; both controls and
  their items join the WinUI-only list of the Fluent and Simple theme tests.
- `compiled_xaml.rs` of the WinUI theme is regenerated after a rebase on `main` (W4 to W6 change
  it too).

### 6.4 SwipeControl

A port of `controls/dev/SwipeControl` on FerroUI's composition interactions, which have the same
model as WinUI's:

- `SwipeControl : ContentControl`, `SwipeItems` (a `FerroList<SwipeItem>` with `Mode` and the
  execute-mode checks), `SwipeItem` (`IconSource` from W4), `SwipeItemInvokedEventArgs`,
  `SwipeMode`, `SwipeBehaviorOnInvoked`.
- The interaction: a `VisualInteractionSource` on the nearest `ListViewItem` within five parents,
  else the control (WinUI looks for the `ListViewItemPresenter`, which FerroUI draws as the item's
  template), an `InteractionTracker` with the inertia resting values of
  `ConfigurePositionInertiaRestingValues`, expression animations of the content translation and
  the clip, the threshold (`c_ThresholdValue = 100`) and `c_MinimumCloseVelocity`, the idle-state
  invocation of execute mode, the single open control (`ferro_thread_local!`), the dismissal
  handlers (a tap outside, a key, a size change of the top level).
- Input (decision D7): as WinUI, redirected for touch (and pen, which FerroUI's pointers report
  like touch on mobile); a precision touchpad's horizontal pan where the platform reports touchpad
  gestures; no mouse drag.
- Template and resources from `SwipeControl.xaml` and `SwipeControl_themeresources.xaml`;
  `SwipeItemStyle` on `AppBarButton`. No peer (WinUI has none).

## 7. Design: Pivot and high contrast

### 7.1 Pivot (decision D8: the full WinUI structure)

The owner chose the structure of WinUI over a `Carousel` base. `Pivot` derives from
`SelectingItemsControl`, in `ferroui-controls`, and is ported structurally from
`dxaml/phone/lib` and `dxaml/xcp/components/pivot`:

- **`Pivot : SelectingItemsControl`.** WinUI's Pivot is an `ItemsControl` with its own
  `SelectedIndex` and `SelectedItem`. `SelectingItemsControl` is FerroUI's base with those members,
  with `AlwaysSelected` semantics while there are items. The other members are those of section
  4.4: `Title`, `TitleTemplate`, `LeftHeader(Template)`, `RightHeader(Template)`, `HeaderTemplate`,
  `IsLocked`, `IsHeaderItemsCarouselEnabled`, `HeaderFocusVisualPlacement`, the attached
  `SlideInAnimationGroup` and the item events (`PivotItemLoading` to `PivotItemUnloaded`).
- **`PivotPanel`** (`PivotPanel_Partial.cpp`), the items panel. It arranges the items side by side
  in a `ScrollViewer` (the `ScrollViewer` part of the template). With wrapping on, the extent is
  `GetPivotPanelMultiplier()` (1000) times the viewport. Without wrapping, it is the item count.
- **The state machine** (`PivotStateMachine.cpp`), ported with its states and events: Idle,
  InNonInertial, InFinishTransition, the fly-in and fly-out states, ProgrammaticInertial,
  LockedPending and the rest. It drives the scroll viewer's offsets, the re-centring at the mid
  offset (`GetMidpointViewportOffset`, `PositiveMod`), the snap to an item, and the
  selection-changed and items-changed handling. Its inputs come from `ScrollViewer`'s
  `ScrollChanged` and from the pan of its content. Where WinUI hooks DManip (`OnViewChanging`,
  `OnViewChanged`, the DManip state callbacks), the port takes the scroll viewer's offset changes and
  the end of its inertia. The mandatory single snap points come from `PivotPanel`
  (`IScrollSnapPointsInfo`, which FerroUI's scroll content presenter honours).
- **The headers:** `PivotHeaderManager` (header items generated by the Pivot, the rotation that
  puts the selected header first, the "ghost" item of the wrapped left header, static headers when
  the carousel is off or the headers fit), `PivotHeaderPanel : Canvas`, `PivotHeaderItem`, and the
  parallax of `ApplyParallax`. In WinUI the parallax is a secondary content relationship of the
  scroll viewer with the curves of `PivotCurveGenerator`; in the port it is the same curves applied to
  the header transform on each offset change of the scroll viewer (a recorded deviation: UI thread,
  not compositor).
- **The rest:** the previous and next buttons, keyboard handling (`OnKeyDown`, the header keys),
  the fly-in and fly-out animations, the slide-in of `Pivot_SlideInEffect_Partial.cpp`, `IsLocked`
  (`LockedPending`, `UnselectedLocked`), the automation helpers of `Pivot_Automation_Partial.cpp`.
- **Peers:** `PivotAutomationPeer` (Tab; Selection, Scroll), `PivotItemAutomationPeer` (TabItem),
  `PivotItemDataAutomationPeer` (SelectionItem, ScrollItem, VirtualizedItem).
- **Templates:** `Pivot`, `PivotItem` and `PivotHeaderItem` from `CS/Pivot_themeresources.xaml`,
  with the scroll viewer template of Pivot (theme SCV-9). The controls join the WinUI-only list, and
  the waiver is removed.

The size is about 12.5k lines of WinUI. PV-1 is therefore split into three pull requests (section
8).

### 7.2 High contrast: the base library

Additive members of `ferroui-base`:

- `PlatformColorValues::system_colors() -> Option<SystemColors>` and `with_system_colors(..)`;
  `SystemColors` (`window`, `window_text`, `button_face`, `button_text`, `highlight`,
  `highlight_text`, `hotlight`, `gray_text`) with a constructor and builder methods, so that later
  colours are additions too. `None` on every platform that does not report them; the struct's
  equality keeps `color_values_changed` from firing for an unchanged scheme.
- The high contrast variant of a theme: `ThemeVariant::set_high_contrast_variant(Option<ThemeVariant>)`
  (per UI thread, `ferro_thread_local!`), set by a theme that has one. With a variant set and the
  platform's `contrast_preference() == High`, `update_actual_theme_variant` resolves a theme
  variant root to that variant (decision D10: also when the root requests Light or Dark, as in
  WinUI). Without a variant set (Fluent, Simple, upstream applications) nothing changes.
- The application, the top levels and every `ThemeVariantScope` re-run the resolution on
  `color_values_changed` (the application does already; the rest follows the inherited actual
  variant).

### 7.3 High contrast: the WinUI theme

- `WinUITheme.UseSystemHighContrast` (`bool`, default `true`, decision D9): while the theme is in
  the styles of the application it sets `WinUITheme.HighContrast` as the high contrast variant.
- `Accents/SystemColors` (beside `SystemAccentColors`): a resource provider of the eight
  `SystemColor*Color` keys of the `HighContrast` dictionary, from `PlatformColorValues::system_colors()`,
  following `color_values_changed`. Without system colours: the High Contrast Black values (today's
  fixed ones) when the platform variant is dark, High Contrast White when it is light. The
  `HighContrast` entries of `Resources/SystemResources.xaml` move to the provider; `Default` and
  `Light` keep their fixed values (as WinUI's own dictionaries).
- The `{DynamicResource}` references of the theme re-resolve when the provider changes, so the
  colours update live; G-5 and G-6 are corrected, and the window frame follows the luminance of
  `SystemColorWindowColor`.

### 7.4 High contrast: the platforms

| Platform | Contrast | System colours | Change | Validation |
|---|---|---|---|---|
| Windows (`src/Windows/FerroUI.Win32/win32_platform_settings.rs`) | `SPI_GETHIGHCONTRAST` with `HCF_HIGHCONTRASTON` (as WinUI), the `IAccessibilitySettings` reading kept as the fallback | `GetSysColor` of `COLOR_WINDOW`, `COLOR_WINDOWTEXT`, `COLOR_BTNFACE`, `COLOR_BTNTEXT`, `COLOR_HIGHLIGHT`, `COLOR_HIGHLIGHTTEXT`, `COLOR_HOTLIGHT`, `COLOR_GRAYTEXT`, while high contrast is on | `WM_SETTINGCHANGE` with `SPI_SETHIGHCONTRAST`, and `WM_SYSCOLORCHANGE` | Windows VM: the four contrast themes, switching while running |
| macOS (`native/FerroUI.Native/src/OSX/PlatformSettings.mm`, `src/FerroUI.Native/frn.idl`, `native_platform_settings.rs`) | `NSWorkspace.accessibilityDisplayShouldIncreaseContrast` ("Increase contrast"), as well as the high contrast appearance names | the dynamic `NSColor`s under the effective appearance: `windowBackgroundColor`, `labelColor`, `controlColor`, `controlTextColor`, `selectedContentBackgroundColor`, `alternateSelectedControlTextColor`, `linkColor`, `disabledControlTextColor` (decision D11) | an observer of `NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification` beside the appearance KVO | locally, with the owner switching the setting (an agent does not change system settings); the native interface gains two methods at the end of `IFrnPlatformSettings` |
| Linux (`src/FerroUI.FreeDesktop/dbus_platform_settings.rs`) | the portal key `contrast` of `org.freedesktop.appearance` (1 is high) | none (the portal has none): the fixed palettes of 7.3 | `SettingChanged` for `contrast` | the fake portal of the crate's tests; the Linux VM with GNOME's high contrast switch |
| Browser, iOS, Android | already report the preference; the browser can read the system colours of `forced-colors` from CSS system colours (follow-up HC-5) | | already | |

### 7.5 Not in this plan

`FrameworkElement.HighContrastChanged` and `HighContrastAdjustment` (the text backplate): an
application reads the preference from the platform settings (the Calculator's
`AccessibilitySettings` use maps to it). `HighContrastAdjustment = None` is the FerroUI default.

## 8. Pull requests, in order

One small PR at a time from current `main`, each with tests, deviation rows, a `CHANGELOG.md`
line, a ControlCatalog page (with macOS screenshots of the app window) where there is something
to see. Efforts are working days of one agent, including review rounds.

| PR | Content | Depends on | Decision | Windows VM | Effort |
|---|---|---|---|---|---|
| P0 | This page | | | no | done |
| **VSM-1** | `VisualStateManager`, `VisualStateGroup(Collection)`, `VisualState`, `VisualStateChangedEventArgs`, `Setter.Target` and `TargetPropertyPath` (Base), `GoToState` (5.3), names (5.4), setters at animation priority (5.5), the events; the markup transformer of `Setter.Target` in the compiler and the loader (5.9); tests (unit, compiled and run-time markup, precedence over local values and styles, user control and template roots, unknown states); catalog page "Visual states" with `GoToState` buttons | | none | no | 4 |
| **VSM-2** | `StateTriggerBase`, `StateTrigger`, `AdaptiveTrigger`, the evaluation (5.7); a ported `ControlSizeTrigger` as the test of the extension point; catalog demo resizing with adaptive and control-size triggers | VSM-1 | none | no | 2 |
| **VSM-3** | The storyboard family (5.8), standalone and in states; `TargetName`/`TargetProperty` in markup with typed key frame values | VSM-1 | none | no | 5 |
| **VSM-4** | `VisualTransition`, generated transitions, `animations_enabled` (5.6) | VSM-3 | none | no | 3 |
| VSM-5 | The state mappers of upstream-ported controls (5.10): button family, list items, text box first; catalog page with a WinUI button template written with visual states | VSM-4 | D1 | no | 3 |
| LV-1 | `ListViewBase`, `ListView`, `ListViewBaseItem`, `ListViewItem`, `ItemsStackPanel`, the event args and enums, the peers; WinUI theme documents; WinUI-only list; catalog page "ListView" (history-like list with item click, `SelectionMode` switch) | W4 (#333, the WinUI-only list); VSM-1 not needed | D3, D4, D6 | yes: the List/ListItem/Invoke tree with Inspect, the item visuals against WinUI | 5 |
| LV-2 | `GridView`, `GridViewItem`, `ItemsWrapGrid`, their peers and templates | LV-1 | D5 | yes (visuals) | 3 |
| SW-1 | `SwipeControl`, `SwipeItems`, `SwipeItem` and the rest of 6.4; templates and resources; catalog page (a list with reveal and execute items) | LV-1, W4 | D7 | yes (visuals; touch is checked with injected touch in headless tests, the VM has no touch screen) | 4 |
| PV-1 | `Pivot`, `PivotItem`, `PivotPanel` with the scroll viewer and the state machine (7.1), selection, keyboard, `IsLocked`, the item events; peers | W4 | D8 | | 7 |
| PV-2 | The headers: `PivotHeaderManager`, `PivotHeaderPanel`, `PivotHeaderItem`, static headers, the parallax, the previous and next buttons, the fly-in, fly-out and slide-in animations | PV-1 | D8 | | 5 |
| PV-3 | The WinUI theme documents of Pivot, PivotItem and PivotHeaderItem; WinUI-only list; catalog page | PV-2 | D8 | yes: the header carousel and flick against WinUI, the Tab peers | 3 |
| HC-1 | Base: `SystemColors`, the high contrast variant (7.2); WinUI theme: `UseSystemHighContrast`, `Accents/SystemColors` (7.3); headless tests with simulated platform settings; catalog shows the variant | | D9, D10 | no | 2 |
| HC-2 | Windows: `SPI_GETHIGHCONTRAST`, `GetSysColor`, the messages | HC-1; after #325 (it changes the same file) | | **yes** | 2 |
| HC-3 | macOS: "Increase contrast", the `NSColor`s, the notification | HC-1; after #325 (same files) | D11 | no (macOS; the owner toggles the setting) | 2 |
| HC-4 | Linux: the portal's `contrast` | HC-1 | | no (Linux VM to confirm with GNOME) | 1 |
| Follow-ups | LV-3 drag and reorder; LV-4 ItemsView and ItemContainer on #269; deferred states (5.9); the theme migration pilot (5.11); HC-5 browser system colours; `ListViewItemPresenter` as an element (D6) | | | | |

Order of work: VSM-1 to VSM-5 first. LV-1, SW-1 and PV-1 wait for W4 (#333). They change the
WinUI theme, so each is rebased on `main` and regenerates `compiled_xaml.rs` and the theme
tracking instead of merging them. HC-2 and HC-3 follow #325, which changes the same platform
settings files.

The W-series' WinUI importer (W9) maps visual state markup once VSM-1 to VSM-4 are on `main`,
instead of rejecting it; that change belongs to W9.

## 9. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| Two value layers (styles and visual states) on the same property | A state's value hides a style trigger the author expected to win | It is WinUI's precedence (animated values over everything but other animations); documented in `xaml.md` with an example; tests of the precedence |
| Animation-priority values and FerroUI transitions | A property transition does not animate a visual state setter (transitions observe base values) | As WinUI: setters snap, storyboards and visual transitions animate; tested |
| Template namescope not stored on the template root | Names of a template would not resolve from the root | The collection keeps the namescope of its load (5.4); tests for templates, user controls and groups built in Rust |
| Cost of groups per template instance | Lists with stateful item templates build many objects | Nothing changes for themes (5.11); deferred states before any many-instance template uses them |
| `SelectionMode` on `ListViewBase` beside the upstream flags | Ambiguous property resolution in markup and styles | `ListViewBase` registers its own `SelectionMode` and does not add the base's as an owner; tests of markup resolution and of the snapshot of property defaults (W11) |
| `compiled_xaml.rs` regenerated by many PRs (W4 to W10, #325, #326, #313 and this plan) | Merge conflicts | Never merged by hand: rebase on `main`, regenerate, push; land the theme PRs of this plan one at a time |
| Swipe without a touch screen | The VM and the Mac cannot test the gesture by hand | Headless tests with injected touch; the reveal and execute states shown in the catalog by a test hook, not by a non-WinUI API |
| macOS "Increase contrast" is not a palette change like Windows high contrast | The HighContrast variant looks unlike other macOS apps | Decision D11; the colours come from the system, so they follow its contrast |
| Platform files shared with #325 | Conflicts in the Windows and macOS platform settings | HC-2 and HC-3 after #325 |

## 10. Decisions taken (owner, 2026-10-24)

| # | Decision |
|---|---|
| D1 | Upstream-ported controls drive visual states (VSM-5). This is opt-in per template: a mapper is active only when the template root declares visual state groups. |
| D2 | No migration of the WinUI theme in 0.4: the pseudo-class control themes stay, including those of the new controls of this plan. After VSM-5, a pilot ports ToggleSwitch both ways, and the owner decides per control family for 0.5. |
| D3 | Both, in sequence: ListView and GridView now in `ferroui-controls`, on `SelectingItemsControl` and `VirtualizingStackPanel`; ItemsView and ItemContainer later in `ferroui-controls-repeater`, after #269. |
| D4 | `ListViewBaseItem` derives from `ListBoxItem`. |
| D5 | GridView's `ItemsWrapGrid` derives from `WrapPanel`. It does not virtualize, and that is recorded as a deviation. |
| D6 | The item chrome is drawn with elements (the theme's existing chrome). |
| D7 | SwipeControl takes touch and touchpad input only, as WinUI does; no mouse drag. |
| D8 | **The full WinUI structure** for Pivot: `Pivot : SelectingItemsControl` with WinUI's `PivotPanel`, its scroll viewer and the Pivot state machine, ported structurally, with PivotItem, PivotHeaderItem, PivotHeaderPanel and the peers (section 7.1). The extra effort is accepted (PV-1 to PV-3). |
| D9 | The WinUI theme follows the system's high contrast by default (`UseSystemHighContrast = true`). |
| D10 | System high contrast overrides an application's requested Light or Dark variant, as in WinUI. |
| D11 | macOS "Increase contrast" maps to the HighContrast variant, with the system `NSColor` colours. |

## 11. The questions put to the owner



The questions as they were put, with the recommendations; the answers are in section 10.

| # | Question | Options | Recommendation |
|---|---|---|---|
| D1 | Should upstream-ported controls drive visual states, so that WinUI templates with visual state markup work on them (the Calculator's button templates, `GoToState(button, "Pressed")`)? | A: state mappers per control family, active only when the template root has groups (5.10); B: no, only WinUI-only controls and application code call `GoToState` | **A**, as VSM-5 |
| D2 | Migrate the WinUI theme's control themes to visual state markup? | A: no migration in 0.4; new controls of this plan use pseudo-classes; a pilot after VSM-5 informs a per-family decision for 0.5; B: migrate now; C: new WinUI-only controls (ListView items, SwipeControl, Pivot) use visual state markup now, existing themes stay | **A** (5.11) |
| D3 | What ListView builds on | A: `SelectingItemsControl` and the core virtualizing panels, in core; B: ItemsView on ItemsRepeater (wait for #269); C: both, A now and ItemsView/ItemContainer later in the repeater crate | **C** (6.1) |
| D4 | `ListViewBaseItem : ListBoxItem` (keeps the keyed `ListBoxItem` themes `DefaultListViewItemStyle` and `ListViewItemExpanded` working for both) or `: ContentControl` as WinUI's `SelectorItem` (the keyed themes then move to `ListViewItem`, a change for applications that use them on `ListBoxItem`) | A: `ListBoxItem`; B: `ContentControl` | **A** |
| D5 | `ItemsWrapGrid` of GridView | A: on `WrapPanel`, not virtualizing, recorded; B: a new virtualizing wrap panel now; C: GridView on `UniformGridLayout` of #269 | **A** (the Calculator's GridViews are in graphing mode; B as a follow-up if needed) |
| D6 | The list item chrome | A: elements in the template (the theme's existing chrome); B: port `ListViewItemPresenter` as an element that draws the chrome (6.4k lines of WinUI) | **A**; B later if applications retemplate with `ListViewItemPresenter` |
| D7 | Input of SwipeControl | A: WinUI's (touch, plus touchpad pans where reported); B: also mouse drag, as an opt-in property (not WinUI) | **A** |
| D8 | The base of Pivot | A: `Pivot : Carousel`; B: `Pivot : SelectingItemsControl` with a ported `PivotPanel`, scroll viewer and state machine | **A** (the owner chose B) |
| D9 | Does the WinUI theme follow the system's high contrast by default? | A: yes, `UseSystemHighContrast = true`, as WinUI; B: opt-in, default `false` | **A** (the theme is a FerroUI addition; WinUI applications get high contrast without code) |
| D10 | Does system high contrast override a requested Light or Dark variant? | A: yes, as WinUI (the Calculator's own Light/Dark switch then has no effect while high contrast is on); B: only roots that follow the system (`Default`) | **A** |
| D11 | macOS: map "Increase contrast" to the HighContrast variant with the `NSColor` system colours of 7.4? | A: yes, as asked; B: only the macOS high contrast appearances | **A** |
