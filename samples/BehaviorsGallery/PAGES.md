# The pages of the behaviors gallery

The gallery is the port of the sample application of Xaml.Behaviors
(`samples/BehaviorsTestApplication` of the reference, commit `45387e32`): 234 pages, one per tab
item of its main view, in the same order. This file lists every page with its state, says how the
crate is laid out, and how a page is ported.

Run the gallery with `cargo run -p behaviors-gallery`. The smoke run shows every page in turn and
exits, with a failure when a page cannot be created:

```text
FERROUI_SMOKE_PAGES=all cargo run -p behaviors-gallery
FERROUI_SMOKE_PAGES=ported FERROUI_SMOKE_PAGE_MS=500 cargo run -p behaviors-gallery
FERROUI_SMOKE_PAGES="CallMethodAction,TimerTriggerView" cargo run -p behaviors-gallery
```

`FERROUI_SMOKE_PAGES` is `all`, `ported`, or headers and view names separated by commas;
`FERROUI_SMOKE_PAGE_MS` is the time each page is shown (150 ms by default). The options of the
other samples work too (`FERROUI_SMOKE_EXIT_MS`, `FERROUI_SMOKE_SCREENSHOT`).

## Layout of the crate

| Path | What it is |
|---|---|
| `lib.rs`, `app.rs`, `App.xaml`, `SideBar.xaml` | the application: resources, the Fluent theme, the styles of the side bar |
| `program.rs`, `smoke.rs` | the desktop entry point and the page-cycling smoke run |
| `markup.rs`, `register_types.rs` | the table of the documents and classes, the namespaces and the registration of the types |
| `Views/` | `MainWindow`, `MainView` (the search box and the tab control of the pages), `SampleView`; `Windows/` holds the windows of two pages |
| `ViewModels/` | `MainWindowViewModel` (the data context every page inherits), `ItemViewModel`, `SampleViewModel`, the drag and drop view models, and the helpers `command.rs` and `items.rs` |
| `Models/`, `Controls/`, `Converters/`, `Dialogs/`, `Behaviors/` | the shared models, controls (`SingleSelectionTabControl`, `CustomControl`, `EditableItem`), the converter, the dialog and the shared drop handlers |
| `Animations/`, `Renderers/`, `Triggers/` | the types of single pages, ported with their pages |
| `Assets/` | the pictures of the reference |
| `pages/` | one module (`<page>.rs`) and one document (`<Page>.xaml`) per page, the table of the pages (`pages/mod.rs`) and the placeholder of a page not ported yet (`pages/placeholder.rs`) |
| `tests/` | the generated per-document tests, the comparison of the compiled markup with the run-time loader, the checks of the page table and the behaviors of some pages |

The namespaces are the ones of the reference with the name of the gallery
(`BehaviorsGallery.ViewModels`, `BehaviorsGallery.Views.Pages`, ...). `BehaviorsGallery`,
`BehaviorsGallery.Controls`, `BehaviorsGallery.Converters` and `BehaviorsGallery.Behaviors` are in
the default XML namespace (`https://github.com/ferroui`), as the reference states for its own, so
documents name their types without a prefix; the behavior crates are in it too.

What a single page uses (its view model, a behavior of the sample, a renderer, a window) is ported
with the page: its module line in the folder (`ViewModels/mod.rs`, `Behaviors/mod.rs`, ...) is
written as a comment naming its owner page, and the page registers the type in its own entry of the
page table. What several pages share is ported already and registered by the shared tables.

## How to port a page

1. Convert the page with the helper of the port (outside the repository):

   ```text
   python3 -I /Volumes/1TB-macOS/behaviors-port-notes/convert_gallery_page.py <worktree> <ViewName> --apply
   ```

   It writes `pages/<ViewName>.xaml` (the document of the reference with the names of the port:
   the XML namespace `https://github.com/ferroui`, `BehaviorsGallery` for the namespaces of the
   sample, `FerroUI.Xaml.Interactions.*` for the namespaces of the behaviors, `.xaml`,
   `ferres://`, `FerroProperty`), writes `pages/<view_name>.rs` (the class of the page and its entry
   of the table) when it does not exist, changes the two lines of the page in `pages/mod.rs`, and
   prints the code-behind of the reference when it does more than load the document, and every
   line that still names the upstream framework.
2. The two lines of `pages/mod.rs` (the helper changes them; by hand it is the same):
   - the module line: `// pub mod <view_name>;` becomes `pub mod <view_name>;`;
   - the table line: `PageInfo::placeholder("<Header>", "Views/Pages/<ViewName>"),` becomes
     `<view_name>::PAGE,`.

   No other line of a shared file changes, except the module line of a type the page owns (step 4).
3. The page module (`pages/<view_name>.rs`), as the ported pages show (`call_method_action_view.rs`
   is the plain form):
   - the class: `#[repr(C)] pub struct <ViewName> { base: UserControl }`,
     `user_control_class!(<ViewName>)`, `ferro_class_info!(<ViewName> { new: <ViewName>::new })`,
     `xaml_class!(<ViewName>, "/pages/<ViewName>.xaml")`, and `new()` that calls
     `initialize_component()`;
   - `pub(crate) const TYPES: &[&TypeInfo]`: the class of the page and every class only the page
     uses (a behavior of the sample);
   - `pub(crate) const PAGE: PageInfo`: the header (the header of the tab item of the reference),
     the reference file, `create`, `TYPES`, the classes with a document (`&X::XAML_CLASS`), the
     markup types only the page uses (`<X as MarkupTyped>::MARKUP`) and the function that registers
     their value types (`ValueTypes::register_reference::<X>()`, casts to contracts), or
     `no_value_types`.
4. The code-behind:
   - an override of the reference (`OnPointerPressed`, ...) is the implementation of the trait of
     its level, written instead of the one `user_control_class!` writes: declare the class with
     `ferro_class!` and `ferro_impl_classes!` without that trait, and implement it by hand
     (`event_trigger_behavior_view.rs`);
   - `DataContext = new XViewModel()` is `this.set_data_context(Some(XViewModel::new() as BoxedValue))`
     after `initialize_component()` (`drag_drop_commands_view.rs`);
   - an event handler a document names (`Click="OnClick"`) is a method in the `markup` part of
     `ferro_class_info!` (`fn OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) => ..`);
   - `this.FindControl<T>("Name")` is `this.find_control::<T>("Name")`.
5. A view model only the page uses: remove the comment marker of its line in `ViewModels/mod.rs`,
   port the file there, and register it in the `PAGE` of the page (`markup_types` and
   `register_value_types`), as `drag_drop_commands_view.rs` does for its view model.
6. View models follow `ViewModels/`:
   - the struct is declared outside a macro (the scanner of the build must read it: a type a
     document names needs a public path), holds `base: ViewModelBase`, and
     `reactive_object!(X, base)` makes it a reactive object;
   - a `[Reactive]` property of the reference is a `RefCell<T>` field with its accessors written by
     `super::reactive_accessors!(X { field, set_field, "Field": T; })`;
   - a command (`ReactiveCommand.Create(..)`) is a `Command` of `ViewModels/command.rs`, held as
     `Rc<dyn ICommand>`;
   - an `ObservableCollection<T>` of view models is an `ItemList` of `ViewModels/items.rs` (a
     notifying list of untyped values, which the item actions and the drop handlers change), exposed
     to markup as `ItemsSource`;
   - the type is declared to markup with `ferro_markup_type!` (properties, methods for
     `CallMethodAction`, `notify_property_changed`), and implements `PartialEq` by identity.
7. Build and run (only the orchestrator builds; a page agent writes code it believes compiles):
   `cargo build -p behaviors-gallery`, `cargo test -p behaviors-gallery`, and
   `FERROUI_SMOKE_PAGES=<Header> cargo run -p behaviors-gallery`. The build compiles every
   document under `pages/`; a refusal names the document and the line.
8. Update the row of the page in the table below: the state `ported`, and what the notes need
   (a deviation, a part left out). A deviation in code or markup is written at its place as
   `Deviation (DEVIATIONS.md, Behaviors gallery): <why>`.

Rules: no name of the upstream framework anywhere under `samples/` (file names, documents,
strings, comments; the text of a page that names it says FerroUI), `ferro_thread_local!` and never
`thread_local!`, no emojis.

## Groups for parallel work

The placeholders are split into three contiguous ranges of the table with about the same number of
reference lines (document and code-behind). The two lists of `pages/mod.rs` carry a comment at the
start of each group, so that the changes of two groups never touch neighbouring lines. Pages that
share a type of a single group stay in one group (the editable lists, the animation builder pages,
the execute command pages).

| Group | Pages | Placeholders | Reference lines |
|---|---|---|---|
| A | 1 to 86 | 73 | 4255 |
| B | 87 to 144 | 56 | 4451 |
| C | 145 to 234 | 89 | 4490 |

## Known blockers in the behavior crates

- `ferroui-xaml-interactions-reactiveui` is not a dependency of the gallery yet: its type model has
  a cast registration the scanner of the build cannot read (the generic `register_cast` of
  `register_interaction_casts` in `src/Behaviors/FerroUI.Xaml.Interactions.ReactiveUI/interaction_trigger_behavior.rs`),
  and with it the compiler of the documents refuses every question about casts. Pages:
  `InteractionTriggerBehaviorView`, `ReactiveNavigationView` (and `ObservableTriggerBehaviorView`
  once it needs the crate).
- `IDropHandler` and `IDragHandler` have no markup metadata
  (`src/Behaviors/FerroUI.Xaml.Interactions.DragAndDrop/markup_types/mod.rs`, which registers their
  nullable forms only), so a drop handler declared as a resource and assigned with
  `Handler="{StaticResource ..}"` is refused ("has no metadata"). Pages: `TypedDragBehaviorView`,
  `DragAndDropView`, `EditableDraggableListBoxView`, `EditableDragTreeViewView`,
  `FileDropHandlerView`.

## The pages

The groups, the reference lines (the document and its code-behind) and the notes of each page. A
note `own view model` or `owns` names a type only that page uses, whose module line is a comment
until the page is ported.

| # | Header | Reference file | Group | Lines | State | Notes |
|---|---|---|---|---|---|---|
| 1 | SelectAllOnFocusBehavior | `Views/Pages/SelectAllOnFocusBehaviorView` | A | 40 | ported |  |
| 2 | FocusNextElementAction | `Views/Pages/FocusNextElementActionView` | A | 38 | ported |  |
| 3 | WindowDragMoveBehavior | `Views/Pages/WindowDragMoveBehaviorView` | A | 46 | ported | code-behind: opens the window `Views/Windows/WindowDemo` (WindowDragMoveBehavior, WindowAction) as a dialog of the window of the page |
| 4 | TiltEffectBehavior | `Views/Pages/TiltEffectBehaviorView` | A | 53 | ported |  |
| 5 | OrbitEffectBehavior | `Views/Pages/OrbitEffectBehaviorView` | A | 59 | ported |  |
| 6 | ParallaxBehavior | `Views/Pages/ParallaxBehaviorView` | A | 70 | ported |  |
| 7 | ConditionalAction | `Views/Pages/ConditionalActionView` | A | 51 | ported |  |
| 8 | SwitchCaseAction | `Views/Pages/SwitchCaseActionView` | A | 67 | ported |  |
| 9 | SwitchCaseBehavior | `Views/Pages/SwitchCaseBehaviorView` | A | 64 | ported |  |
| 10 | ConditionalBehavior | `Views/Pages/ConditionalBehaviorView` | A | 41 | ported |  |
| 11 | LoseFocusOnEnterBehavior | `Views/Pages/LoseFocusOnEnterBehaviorView` | A | 34 | ported |  |
| 12 | AutoScrollToBottomBehavior | `Views/Pages/AutoScrollToBottomBehaviorView` | A | 57 | ported | code-behind: the items of the list box and the handler of the Add button |
| 13 | CallMethodAction | `Views/Pages/CallMethodActionView` | A | 63 | ported | EventTriggerBehavior (Click) with CallMethodAction and ChangePropertyAction on the view model; tested (`tests/page_behaviors.rs`) |
| 14 | Debounce & Throttle | `Views/Pages/DebounceThrottleActionView` | A | 74 | ported | code-behind: `IncrementThrottleCount`, a markup method the throttled CallMethodAction calls |
| 15 | ObservableStreamBehavior | `Views/Pages/ObservableStreamBehaviorView` | A | 61 | ported |  |
| 16 | AsyncActionGroup | `Views/Pages/AsyncActionGroupView` | A | 56 | ported |  |
| 17 | FocusTrapBehavior | `Views/Pages/FocusTrapBehaviorView` | A | 48 | ported |  |
| 18 | ScreenReaderAnnounceAction | `Views/Pages/ScreenReaderAnnounceActionView` | A | 49 | ported |  |
| 19 | GlobalHotkeyBehavior | `Views/Pages/GlobalHotkeyBehaviorView` | A | 43 | ported |  |
| 20 | VisualDebugBehavior | `Views/Pages/VisualDebugBehaviorView` | A | 49 | ported |  |
| 21 | LogAction | `Views/Pages/LogActionView` | A | 62 | ported |  |
| 22 | NetworkStatusTrigger | `Views/Pages/NetworkStatusTriggerView` | A | 50 | ported |  |
| 23 | ClipboardMonitorBehavior | `Views/Pages/ClipboardMonitorBehaviorView` | A | 52 | ported | code-behind: the Paste button reads the text of the clipboard of the top level in a job of the dispatcher |
| 24 | ChangePropertyAction | `Views/Pages/ChangePropertyActionView` | A | 55 | ported | ChangePropertyAction with a dynamic resource value; tested |
| 25 | DataTriggerBehavior | `Views/Pages/DataTriggerBehaviorView` | A | 64 | ported | DataTriggerBehavior with comparison conditions on an element binding |
| 26 | DataTriggerBehavior Advanced | `Views/Pages/DataTriggerBehaviorAdvancedView` | A | 51 | ported |  |
| 27 | MultiDataTriggerBehavior | `Views/Pages/MultiDataTriggerBehaviorView` | A | 87 | ported |  |
| 28 | BindingTriggerBehavior | `Views/Pages/BindingTriggerBehaviorView` | A | 64 | ported |  |
| 29 | ValueChangedTriggerBehavior | `Views/Pages/ValueChangedTriggerBehaviorView` | A | 40 | ported | ValueChangedTriggerBehavior on a stream binding (`Values^`) |
| 30 | PropertyChangedTrigger | `Views/Pages/PropertyChangedTriggerView` | A | 39 | ported |  |
| 31 | EventTriggerBehavior | `Views/Pages/EventTriggerBehaviorView` | A | 56 | ported | code-behind: `OnPointerPressed` override (`InputElementImpl`); shows `Views/SampleView` |
| 32 | TimerTrigger | `Views/Pages/TimerTriggerView` | A | 45 | ported | TimerTrigger with CallMethodAction |
| 33 | TaskCompletedTrigger | `Views/Pages/TaskCompletedTriggerView` | A | 45 | ported |  |
| 34 | LoadedTrigger | `Views/Pages/LoadedTriggerView` | A | 39 | ported |  |
| 35 | InvokeCommandAction | `Views/Pages/InvokeCommandActionView` | A | 62 | ported | InvokeCommandAction; deviation: no `Unit.Default` command parameter; tested |
| 36 | AutoFocusBehavior | `Views/Pages/AutoFocusBehaviorView` | A | 34 | ported |  |
| 37 | AutoSelectBehavior | `Views/Pages/AutoSelectBehaviorView` | A | 34 | ported |  |
| 38 | TextBoxValidationBehavior | `Views/Pages/TextBoxValidationBehaviorView` | A | 43 | ported |  |
| 39 | NumericUpDownValidationBehavior | `Views/Pages/NumericUpDownValidationBehaviorView` | A | 46 | ported | deviation: the rules without `x:TypeArguments` (markup cannot give a class of the object model type arguments; the rules of the port are untyped) |
| 40 | NotNullValidationRule | `Views/Pages/NotNullValidationRuleView` | A | 45 | ported | deviation: the rule without `x:TypeArguments` (markup cannot give a class of the object model type arguments; the rules of the port are untyped) |
| 41 | PropertyValidationBehavior | `Views/Pages/PropertyValidationBehaviorView` | A | 56 | ported | deviation: no `x:TypeArguments="TextBox, sys:String"` (markup cannot give a class of the object model type arguments); the behavior markup creates validates the property whatever the type of its value |
| 42 | SliderValidationBehavior | `Views/Pages/SliderValidationBehaviorView` | A | 60 | ported | deviation: the rule without `x:TypeArguments` (markup cannot give a class of the object model type arguments; the rules of the port are untyped) |
| 43 | DatePickerValidationBehavior | `Views/Pages/DatePickerValidationBehaviorView` | A | 47 | ported | deviation: the rule without `x:TypeArguments` (markup cannot give a class of the object model type arguments; the rules of the port are untyped) |
| 44 | ComboBoxValidationBehavior | `Views/Pages/ComboBoxValidationBehaviorView` | A | 46 | ported | deviation: the rule without `x:TypeArguments` (markup cannot give a class of the object model type arguments; the rules of the port are untyped) |
| 45 | FocusControlBehavior | `Views/Pages/FocusControlBehaviorView` | A | 37 | ported |  |
| 46 | FocusOnAttachedBehavior | `Views/Pages/FocusOnAttachedBehaviorView` | A | 34 | ported | a behavior of the Custom crate |
| 47 | FocusOnAttachedToVisualTreeBehavior | `Views/Pages/FocusOnAttachedToVisualTreeBehaviorView` | A | 34 | ported |  |
| 48 | FocusOnPointerMovedBehavior | `Views/Pages/FocusOnPointerMovedBehaviorView` | A | 34 | ported |  |
| 49 | FocusOnPointerPressedBehavior | `Views/Pages/FocusOnPointerPressedBehaviorView` | A | 34 | ported |  |
| 50 | FocusOnVisibleBehavior | `Views/Pages/FocusOnVisibleBehaviorView` | A | 38 | ported |  |
| 51 | FocusSelectedItemBehavior | `Views/Pages/FocusSelectedItemBehaviorView` | A | 34 | ported |  |
| 52 | RoutedEventTriggerBehavior | `Views/Pages/RoutedEventTriggerBehaviorView` | A | 61 | ported |  |
| 53 | ChangeFerroPropertyAction | `Views/Pages/ChangeFerroPropertyActionView` | A | 59 | ported | the reference names the view and its text after the upstream framework (`ChangeFerroPropertyAction` here) |
| 54 | ViewModel Property Actions | `Views/Pages/ViewModelPropertyActionsView` | A | 66 | ported |  |
| 55 | Custom Action | `Views/Pages/CustomActionView` | A | 67 | ported | ShowPointerPositionBehavior and PopupAction (custom action) |
| 56 | Custom Behavior | `Views/Pages/CustomBehaviorView` | A | 68 | ported | DragControlBehavior (custom behavior), also through a style setter with a BehaviorCollectionTemplate |
| 57 | ButtonClickEventTriggerBehavior | `Views/Pages/ButtonClickEventTriggerBehaviorView` | A | 53 | ported |  |
| 58 | ClickEventTrigger | `Views/Pages/ClickEventTriggerView` | A | 336 | ported |  |
| 59 | Advanced Behavior | `Views/Pages/AdvancedView` | A | 66 | ported |  |
| 60 | Add/RemoveClassAction | `Views/Pages/AddRemoveClassActionView` | A | 64 | ported | AddClassAction and RemoveClassAction from data triggers on the bounds |
| 61 | RemoveElementAction | `Views/Pages/RemoveElementActionView` | A | 45 | ported |  |
| 62 | RemoveItemInListBoxAction | `Views/Pages/RemoveItemInListBoxActionView` | A | 45 | ported |  |
| 63 | RemoveItemInItemsControlAction | `Views/Pages/RemoveItemInItemsControlActionView` | A | 45 | ported |  |
| 64 | AddItemToItemsControlAction | `Views/Pages/AddItemToItemsControlActionView` | A | 64 | ported | the item of the template is created by the markup constructor of `ItemViewModel` (`x:Arguments`) |
| 65 | InsertItemToItemsControlAction | `Views/Pages/InsertItemToItemsControlActionView` | A | 60 | ported | the item of the template is created by the markup constructor of `ItemViewModel` (`x:Arguments`) |
| 66 | ObjectTemplate | `Views/Pages/ObjectTemplateView` | A | 82 | ported | the items of the templates are created by the markup constructor of `ItemViewModel` (`x:Arguments`) |
| 67 | ClearItemsControlAction | `Views/Pages/ClearItemsControlActionView` | A | 45 | ported |  |
| 68 | AddRangeAction | `Views/Pages/AddRangeActionView` | A | 72 | ported | deviation: the items are in an `ArrayList`, not a `List<ItemViewModel>` (the compiled markup cannot add a view model to a typed list of the runtime library) |
| 69 | RemoveRangeAction | `Views/Pages/RemoveRangeActionView` | A | 45 | ported | deviation: the page registers the conversion of the selected items of a list box (`SelectedItemsList`) to the items of the action (`ItemsSource`), which the framework lacks |
| 70 | MoveItemInItemsControlAction | `Views/Pages/MoveItemInItemsControlActionView` | A | 56 | ported |  |
| 71 | MoveElementToPanelAction | `Views/Pages/MoveElementActionsView` | A | 76 | ported |  |
| 72 | CollectionChangedTrigger | `Views/Pages/CollectionChangedTriggerBehaviorView` | A | 55 | ported |  |
| 73 | ItemsControl Container Triggers | `Views/Pages/ItemsControlContainerTriggersView` | A | 106 | ported |  |
| 74 | Remove Items Sample | `Views/Pages/RemoveItemsSampleView` | A | 61 | ported |  |
| 75 | AdaptiveBehavior | `Views/Pages/AdaptiveBehaviorView` | A | 155 | ported | the Responsive crate; the shared `ClassesToStringConverter` as a markup extension |
| 76 | AspectRatioBehavior | `Views/Pages/AspectRatioBehaviorView` | A | 76 | ported | uses the shared `ClassesToStringConverter` |
| 77 | SplitView | `Views/Pages/SplitViewStateBehaviorView` | A | 75 | ported |  |
| 78 | EditableListBox | `Views/Pages/EditableListBoxView` | A | 44 | ported | uses the shared `EditableItem` control |
| 79 | EditableDraggableListBox | `Views/Pages/EditableDraggableListBoxView` | A | 83 | ported | own data context (`DragAndDropSampleViewModel`); the drop handler is a resource of a style (`Handler="{StaticResource ..}"`); uses the shared `EditableItem` control |
| 80 | EditableListBox DoubleTap | `Views/Pages/EditableDoubleTappedListBoxView` | A | 44 | ported | uses the shared `EditableItem` control |
| 81 | EditableTree | `Views/Pages/EditableTreeViewView` | A | 45 | ported | uses the shared `EditableItem` control |
| 82 | EditableTree DoubleTap | `Views/Pages/EditableDoubleTappedTreeViewView` | A | 44 | ported | uses the shared `EditableItem` control |
| 83 | Editable Drag TreeView | `Views/Pages/EditableDragTreeViewView` | A | 67 | ported | own data context (`DragAndDropSampleViewModel`); the drop handler is a resource; includes the styles of the drag and drop crate (`ferres://FerroUI.Xaml.Interactions.DragAndDrop/Styles.xaml`) |
| 84 | Sliding Animation | `Views/Pages/SlidingAnimationView` | A | 77 | ported |  |
| 85 | Fade Animation | `Views/Pages/FadeAnimationView` | A | 49 | ported | attached properties of the Custom crate (`custom:FadeAnimation`): an animation page |
| 86 | Scale Animation | `Views/Pages/ScaleAnimationView` | A | 91 | ported |  |
| 87 | Rotate Animation | `Views/Pages/RotateAnimationView` | B | 105 | ported | attached properties of the Custom crate (`custom:RotateAnimation`): an animation page |
| 88 | Entrance Animations | `Views/Pages/EntranceAnimationsView` | B | 198 | ported | attached properties of the Custom crate (`EntranceAnimations`): an animation page |
| 89 | Attention Animations | `Views/Pages/AttentionAnimationsView` | B | 117 | ported | attached properties of the Custom crate (`AttentionAnimations`): an animation page |
| 90 | Exit Animations | `Views/Pages/ExitAnimationsView` | B | 195 | ported | attached properties of the Custom crate (`ExitAnimations`): an animation page |
| 91 | Framer Motion Animations | `Views/Pages/FramerMotionAnimationsView` | B | 157 | ported | attached properties of the Custom crate (`FramerMotionAnimations`): an animation page |
| 92 | Special Animations | `Views/Pages/SpecialAnimationsView` | B | 34 | ported | attached properties of the Custom crate (`SpecialAnimations`) |
| 93 | Animation Behaviors | `Views/Pages/AnimationBehaviorView` | B | 69 | ported | PlayAnimationBehavior and BeginAnimationAction with key frame animations |
| 94 | Animation Completed Trigger | `Views/Pages/AnimationCompletedTriggerView` | B | 49 | ported | AnimationCompletedTrigger with ChangePropertyAction |
| 95 | StartAnimationAction | `Views/Pages/StartAnimationActionView` | B | 50 | ported | StartAnimationAction from a click |
| 96 | AnimationBuilder | `Views/Pages/AnimationBuilderView` | B | 40 | ported | RunAnimationTrigger with the builder of the view model; owns `ViewModels/custom_animator_view_model.rs` and `Animations/custom_string_animation_builder.rs`, `Animations/custom_string_animator.rs` (registered by this page); deviations: the builder is a plain type handed out as the builder handle `AnimationBuilderRef`, the animator interpolates the optional text of a text block |
| 97 | StartBuiltAnimationAction | `Views/Pages/StartBuiltAnimationActionView` | B | 41 | ported | StartBuiltAnimationAction with the builder of the view model (`CustomAnimatorViewModel`, registered by AnimationBuilder) |
| 98 | RunAnimationTrigger | `Views/Pages/RunAnimationTriggerView` | B | 40 | ported | RunAnimationTrigger with the builder of the view model (`CustomAnimatorViewModel`, registered by AnimationBuilder) |
| 99 | FluidMoveBehavior | `Views/Pages/FluidMoveBehaviorView` | B | 51 | ported | FluidMoveBehavior; own view model (`ViewModels/fluid_move_behavior_view_model.rs`; the shuffle uses a small clock-seeded generator, no random number crate) |
| 100 | SelectingItemsControlBehavior | `Views/Pages/SelectingItemsControlBehaviorView` | B | 40 | ported | SelectingItemsControlBehavior.EnableSelectionAnimation on a tab control |
| 101 | ListBox Selection Behaviors | `Views/Pages/ListBoxSelectionBehaviorsView` | B | 103 | ported | ListBoxSelectAll, ListBoxUnselectAll, SelectListBoxItemOnPointerMoved and TextBoxSelectAllText behaviors |
| 102 | BehaviorCollectionTemplate | `Views/Pages/BehaviorCollectionTemplateView` | B | 70 | ported | BehaviorCollectionTemplate in a style, ChangeFerroPropertyAction, PopupAction |
| 103 | ActionCollectionTemplate | `Views/Pages/ActionCollectionTemplateView` | B | 90 | ported | BehaviorCollectionTemplate and ActionCollectionTemplate in styles, ChangeFerroPropertyAction, PopupAction |
| 104 | TemplateBinding | `Views/Pages/TemplateBindingView` | B | 30 | ported | the shared `Controls/CustomControl` (its styles are in `App.xaml`) |
| 105 | OneTime Binding | `Views/Pages/OneTimeBinding` | B | 55 | ported | one-time bindings in a carousel with InvokeCommandAction |
| 106 | Pointer Triggers | `Views/Pages/PointerTriggersView` | B | 51 | ported | pointer triggers with InvokeCommandAction and PointerEventArgsConverter; own view model (`ViewModels/pointer_triggers_view_model.rs`) |
| 107 | KeyGestureTrigger | `Views/Pages/KeyGestureTriggerView` | B | 40 | ported | KeyGestureTrigger; own view model (`ViewModels/key_gesture_trigger_view_model.rs`) |
| 108 | ExecuteCommand Behaviors | `Views/Pages/ExecuteCommandBehaviorsView` | B | 176 | ported | the execute command behaviors; own view model (`ViewModels/execute_command_behaviors_view_model.rs`, shared with ButtonExecuteCommandOnKeyDownBehavior) |
| 109 | ButtonExecuteCommandOnKeyDownBehavior | `Views/Pages/ButtonExecuteCommandOnKeyDownBehaviorView` | B | 36 | ported | ButtonExecuteCommandOnKeyDownBehavior (`ExecuteCommandBehaviorsViewModel`, registered by ExecuteCommand Behaviors) |
| 110 | Storage Provider | `Views/Pages/StorageProviderView` | B | 203 | ported | the picker behaviors and actions; code-behind: the `Pick` handler of the folder picker behavior is subscribed in the constructor (deviation: `Pick` is an event of the Rust API, the button of the behavior is named `OpenFoldersBehaviorButton`); deviation: `StorageProvider="{Binding $parent[TopLevel].StorageProvider}"` left out (the top level does not declare its storage provider to markup; the default is the same provider) |
| 111 | Picker CanExecuteCommand | `Views/Pages/PickerCanExecuteCommandView` | B | 188 | ported | picker behaviors with can-execute commands; deviation: `StorageProvider="{Binding $parent[TopLevel].StorageProvider}"` left out (as Storage Provider) |
| 112 | Clipboard | `Views/Pages/ClipboardView` | B | 52 | ported | the clipboard text actions; the text names FerroUI |
| 113 | GetClipboardDataAction | `Views/Pages/GetClipboardDataActionView` | B | 38 | ported | GetClipboardDataAction |
| 114 | GetClipboardFormatsAction | `Views/Pages/GetClipboardFormatsActionView` | B | 38 | ported | GetClipboardFormatsAction |
| 115 | SetClipboardDataObjectAction | `Views/Pages/SetClipboardDataObjectActionView` | B | 38 | ported | SetClipboardDataObjectAction |
| 116 | FocusControlAction | `Views/Pages/FocusControlActionView` | B | 39 | ported | FocusControlAction |
| 117 | Hide/ShowControlAction | `Views/Pages/HideShowControlActionView` | B | 46 | ported | HideControlAction and ShowControlAction |
| 118 | ShowHideFlyoutAction | `Views/Pages/ShowHideFlyoutActionView` | B | 48 | ported | ShowFlyoutAction and HideFlyoutAction |
| 119 | ShowContextMenuAction | `Views/Pages/ShowContextMenuActionView` | B | 42 | ported | ShowContextMenuAction |
| 120 | ContextDialog | `Views/Pages/ContextDialogView` | B | 60 | ported | ContextDialogBehavior with its show and hide actions and its opened and closed triggers |
| 121 | ShowHidePopupAction | `Views/Pages/ShowHidePopupActionView` | B | 47 | ported | ShowPopupAction and HidePopupAction |
| 122 | ShowOn Event Behaviors | `Views/Pages/ShowOnEventBehaviorsView` | B | 138 | ported | ShowOnTapped, ShowOnDoubleTapped and ShowOnKeyDown behaviors |
| 123 | SetEnabledAction | `Views/Pages/SetEnabledActionView` | B | 44 | ported | SetEnabledAction |
| 124 | ShowNotificationAction | `Views/Pages/ShowNotificationActionView` | B | 49 | ported | NotificationManagerBehavior and ShowNotificationAction; deviation: the `Notification` of the framework is not declared to markup, the code-behind sets the notification of the action |
| 125 | Dialog Actions | `Views/Pages/DialogActionView` | B | 52 | ported | the dialog actions with the shared `Dialogs/SimpleDialog` |
| 126 | File Drop Handler | `Views/Pages/FileDropHandlerView` | B | 55 | ported | ContextDropBehavior with a drop handler resource, FilesDropBehavior, TextDropBehavior; owns `Behaviors/files_drop_handler.rs` (registered by this page) |
| 127 | Upload File | `Views/Pages/UploadFileView` | B | 102 | ported | ButtonUploadFileBehavior, UploadFileAction and UploadCompletedTrigger |
| 128 | ContentControlFilesDropBehavior | `Views/Pages/ContentControlFilesDropBehaviorView` | B | 51 | ported | ContentControlFilesDropBehavior |
| 129 | Drag and Drop | `Views/Pages/DragAndDropView` | B | 426 | ported | the drag and drop behaviors of lists and trees (context, managed with preview, reorder, item drag, nudge); owns `Behaviors/managed_items_list_box_drop_handler.rs`, `Behaviors/nodes_list_box_drop_handler.rs` (registered by this page); deviation: the DataGrid styles and tab are left out (no DataGrid, no DataGrid drag and drop crate), the styles of the crate have its port name; the shared `DragAndDropSampleViewModel` holds `SelectedTreeNodes` as the selected items list of the controls, which the two-way binding of the tree view needs |
| 130 | DragDrop Commands | `Views/Pages/DragDropCommandsView` | B | 50 | ported | drag and drop: DragDropCommandsBehavior and ContextDragBehavior; own view model (`ViewModels/drag_drop_commands_view_model.rs`, registered by the page) |
| 131 | Files Preview | `Views/Pages/FilesPreviewView` | B | 51 | ported | FilesPreviewBehavior and AddPreviewFilesAction on DragOverEventTrigger |
| 132 | TypedDragBehavior | `Views/Pages/TypedDragBehaviorView` | B | 66 | ported | TypedDragBehavior with the shared `DragAndDropSampleViewModel` and `ItemsListBoxDropHandler` |
| 133 | Draggable | `Views/Pages/DraggableView` | B | 232 | ported | the draggable behaviors; own view model (`ViewModels/draggable_view_model.rs`); owns `Styles/DraggableCustomStyles.xaml`, included by its resource URI; the text names FerroUI |
| 134 | Mouse Drag Behaviors | `Views/Pages/MouseDragBehaviorView` | B | 69 | ported | mouse drag behaviors; code-behind: the multi drag behavior of the first rectangle also moves the two others |
| 135 | Canvas Drag Behavior | `Views/Pages/CanvasDragBehaviorView` | B | 72 | ported | CanvasDragBehavior |
| 136 | Auto Scroll During Drag | `Views/Pages/AutoScrollDuringDragBehaviorView` | B | 45 | ported | AutoScrollDuringDragBehavior |
| 137 | Drag Between Panels | `Views/Pages/DragBetweenPanelsView` | B | 54 | ported | PanelDragBehavior and PanelDropBehavior with drag event triggers |
| 138 | Cursor Behavior | `Views/Pages/CursorView` | B | 67 | ported | the cursor behaviors and actions; own view model (`ViewModels/cursor_view_model.rs`, with `StandardCursorModel`); deviations: the standard cursor types are listed by the view model (the enumeration has no public list of values), the item template states its data type |
| 139 | Drawn Cursor | `Views/Pages/DrawnCursorView` | B | 51 | ported | SetCursorFromProvider behavior and action; own view model (`ViewModels/drawn_cursor_view_model.rs`) |
| 140 | IfElseTrigger | `Views/Pages/IfElseTriggerView` | B | 49 | ported | IfElseTrigger |
| 141 | ExecuteScriptAction | `Views/Pages/ExecuteScriptActionView` | B | 53 | placeholder | NOT PORTED: ExecuteScriptAction needs a C# script run time, which the port has not (waived) |
| 142 | Observable Trigger | `Views/ReactiveUI/ObservableTriggerBehaviorView` | B | 46 | ported | ObservableTriggerBehavior with `Observable="{Binding Values}"` (the stream value of the view model) and ChangePropertyAction; deviation: no `x:TypeArguments="x:Int32"` (the trigger is not generic, its value is untyped) |
| 143 | InteractionTriggerBehavior | `Views/ReactiveUI/InteractionTriggerBehaviorView` | B | 45 | ported | InteractionTriggerBehavior; own view model (`ViewModels/ReactiveUI/interaction_trigger_behavior_view_model.rs`, namespace `BehaviorsGallery.ViewModels.ReactiveUI`); deviation: no x:TypeArguments (the behavior is not generic) |
| 144 | Flyout Binding | `Views/Pages/FlyoutBindingView` | B | 48 | ported | InvokeCommandAction in a flyout bound to the data context |
| 145 | KeyTrigger | `Views/Pages/KeyTriggerView` | C | 49 | ported |  |
| 146 | ToolTip Helpers | `Views/Pages/ToolTipHelpersView` | C | 62 | ported |  |
| 147 | WriteableBitmap | `Views/Pages/WriteableBitmapView` | C | 123 | ported | owns `Renderers/sample_writeable_bitmap_renderer.rs` (registered by the page, cast to `IWriteableBitmapRenderer`) |
| 148 | RenderTargetBitmap | `Views/Pages/RenderTargetBitmapView` | C | 67 | ported | owns `Renderers/rotating_square_renderer.rs` and `Renderers/solid_square_renderer.rs` (registered by the page, cast to their render target contracts); deviation: the code-behind sets the `Target` of the trigger and of the action (markup resolves the names to the behaviors but does not assign them to the render host contract) |
| 149 | LaunchUriOrFileAction | `Views/Pages/LaunchUriOrFileActionView` | C | 39 | ported | the address is the FerroUI repository (the reference names the site of its framework) |
| 150 | Reactive Navigation | `Views/ReactiveUI/ReactiveNavigationView` | C | 84 | ported | own view models (`ViewModels/ReactiveUI/`: `ReactiveNavigationViewModel`, `HomePageViewModel`, `DetailPageViewModel`) and the views of its pages (`pages/HomePageView.xaml`, `pages/DetailPageView.xaml`); deviations: the view models are in the namespace `BehaviorsGallery.ViewModels.ReactiveUI`, the root of the documents is `ReactiveUserControlBase`, and the generic `NavigateToAction` and `NavigateToAndResetAction` (no type arguments in markup) are added by the code-behind; the gallery depends on `ferroui-splat` (`Locator.CurrentMutable`) |
| 151 | Transitions Behavior | `Views/Pages/TransitionsBehaviorView` | C | 60 | ported |  |
| 152 | Transitions Actions | `Views/Pages/TransitionsActionsView` | C | 75 | ported |  |
| 153 | Transitions Trigger | `Views/Pages/TransitionsChangedTriggerView` | C | 75 | ported |  |
| 154 | Notifications | `Views/Pages/NotificationsView` | C | 70 | ported |  |
| 155 | Carousel Navigation | `Views/Pages/CarouselNavigationView` | C | 56 | ported |  |
| 156 | CarouselSelectionChangedTrigger | `Views/Pages/CarouselSelectionChangedTriggerView` | C | 42 | ported |  |
| 157 | AutoCompleteBox | `Views/Pages/AutoCompleteBoxView` | C | 49 | ported |  |
| 158 | FocusAutoCompleteBoxTextBoxBehavior | `Views/Pages/FocusAutoCompleteBoxTextBoxBehaviorView` | C | 36 | ported |  |
| 159 | TabControl Navigation | `Views/Pages/TabControlNavigationView` | C | 60 | ported |  |
| 160 | TabControl Selection Trigger | `Views/Pages/TabControlSelectionChangedTriggerView` | C | 42 | ported |  |
| 161 | Theme Variant | `Views/Pages/ThemeVariantView` | C | 58 | ported | code-behind: the static `Pink` variant (`ThemeVariantView::pink()`) |
| 162 | Screens | `Views/Pages/ScreenView` | C | 86 | ported |  |
| 163 | BindPointerOverBehavior | `Views/Pages/BindPointerOverBehaviorView` | C | 35 | ported |  |
| 164 | BindTagToVisualRootDataContextBehavior | `Views/Pages/BindTagToVisualRootDataContextBehaviorView` | C | 45 | ported | deviation: the items of the view model have no indexer for markup (`Items[0]`), so the code-behind sets the data context of the text block to the first item |
| 165 | BoundsObserverBehavior | `Views/Pages/BoundsObserverBehaviorView` | C | 39 | ported |  |
| 166 | HideAttachedFlyoutBehavior | `Views/Pages/HideAttachedFlyoutBehaviorView` | C | 48 | ported |  |
| 167 | ButtonHideFlyoutBehavior | `Views/Pages/ButtonHideFlyoutBehaviorView` | C | 48 | ported |  |
| 168 | ButtonHideFlyoutOnClickBehavior | `Views/Pages/ButtonHideFlyoutOnClickBehaviorView` | C | 45 | ported |  |
| 169 | HideOnKeyPressedBehavior | `Views/Pages/HideOnKeyPressedBehaviorView` | C | 40 | ported |  |
| 170 | HideOnLostFocusBehavior | `Views/Pages/HideOnLostFocusBehaviorView` | C | 40 | ported |  |
| 171 | SizeChangedTrigger | `Views/Pages/SizeChangedTriggerView` | C | 40 | ported |  |
| 172 | FadeInBehavior | `Views/Pages/FadeInBehaviorView` | C | 34 | ported |  |
| 173 | DelayedLoadBehavior | `Views/Pages/DelayedLoadBehaviorView` | C | 34 | ported |  |
| 174 | DelayedLoadTrigger | `Views/Pages/DelayedLoadTriggerView` | C | 38 | ported |  |
| 175 | DelayedShowControlAction | `Views/Pages/DelayedShowControlActionView` | C | 38 | ported |  |
| 176 | AnimateOnAttachedBehavior | `Views/Pages/AnimateOnAttachedBehaviorView` | C | 45 | ported |  |
| 177 | Async Loading | `Views/Pages/AsyncLoadingView` | C | 44 | ported |  |
| 178 | ThemeVariantTrigger | `Views/Pages/ThemeVariantTriggerView` | C | 57 | ported |  |
| 179 | ActualThemeVariantChangedTrigger | `Views/Pages/ActualThemeVariantChangedTriggerView` | C | 47 | ported |  |
| 180 | ToggleIsExpandedOnDoubleTappedBehavior | `Views/Pages/ToggleIsExpandedOnDoubleTappedBehaviorView` | C | 50 | ported |  |
| 181 | TreeView Filter | `Views/Pages/TreeViewFilterView` | C | 57 | ported |  |
| 182 | ViewportBehavior | `Views/Pages/ViewportBehaviorView` | C | 47 | ported |  |
| 183 | ScrollChangedTrigger | `Views/Pages/ScrollChangedTriggerView` | C | 44 | ported |  |
| 184 | ScrollViewerOffsetBehavior | `Views/Pages/ScrollViewerOffsetBehaviorView` | C | 43 | ported |  |
| 185 | ScrollToOffsetAction | `Views/Pages/ScrollToOffsetActionView` | C | 44 | ported |  |
| 186 | Scroll Behaviors | `Views/Pages/ScrollBehaviorsView` | C | 116 | ported |  |
| 187 | Window Interactions | `Views/Pages/WindowInteractionsView` | C | 48 | ported | owns `Views/Windows/SampleWindow` (registered by the page) |
| 188 | Events Behaviors | `Views/Pages/EventsBehaviorsView` | C | 34 | ported |  |
| 189 | Focus Event Triggers | `Views/Pages/FocusEventTriggersView` | C | 37 | ported |  |
| 190 | Key Event Triggers | `Views/Pages/KeyEventTriggersView` | C | 43 | ported |  |
| 191 | Pointer Event Triggers | `Views/Pages/PointerEventTriggersView` | C | 52 | ported | the triggers of the Events crate |
| 192 | PointerEventsTrigger | `Views/Pages/PointerEventsTriggerView` | C | 69 | ported | code-behind: the page is its own data context (`Events`, `OnPointerEvent`) |
| 193 | DragLeaveEventTrigger | `Views/Pages/DragLeaveEventTriggerView` | C | 77 | ported | code-behind: the page is its own data context (`Events`, `OnDragLeave`), `OnStartDrag` starts the drag |
| 194 | Gesture Event Triggers | `Views/Pages/GestureEventTriggersView` | C | 46 | ported |  |
| 195 | Gesture Advanced Triggers | `Views/Pages/GestureAdvancedTriggersView` | C | 82 | ported | code-behind: the page is its own data context (`Events`, `OnPinch`, `OnPinchEnded`, `Clear`) |
| 196 | Pull Gesture Triggers | `Views/Pages/PullGestureTriggersView` | C | 82 | ported | code-behind: the page is its own data context (`Events`, `OnPull`, `OnPullEnded`, `Clear`) |
| 197 | Scroll Gesture Triggers | `Views/Pages/ScrollGestureTriggersView` | C | 78 | ported | code-behind: the page is its own data context (`Events`, `OnScroll`, `OnScrollEnded`, `OnScrollInertia`, `Clear`) |
| 198 | TouchPad Gesture Triggers | `Views/Pages/TouchPadGestureTriggersView` | C | 78 | ported | code-behind: the page is its own data context (`Events`, `OnMagnify`, `OnRotate`, `OnSwipe`, `Clear`) |
| 199 | Popup Event Triggers | `Views/Pages/PopupEventTriggersView` | C | 50 | ported |  |
| 200 | Focus Triggers | `Views/Pages/FocusTriggersView` | C | 37 | ported |  |
| 201 | Key Input Triggers | `Views/Pages/KeyInputTriggersView` | C | 43 | ported |  |
| 202 | Pointer Extra Triggers | `Views/Pages/PointerExtraTriggersView` | C | 43 | ported |  |
| 203 | Gesture Triggers | `Views/Pages/GestureTriggersView` | C | 57 | ported |  |
| 204 | Icon Sample | `Views/Pages/IconView` | C | 49 | ported | own view model (`ViewModels/icon_view_model.rs`, registered by the page) |
| 205 | Automation | `Views/Pages/AutomationView` | C | 57 | ported |  |
| 206 | ActualThemeVariantChangedBehavior | `Views/Pages/ActualThemeVariantChangedBehaviorView` | C | 45 | ported | owns `Behaviors/actual_theme_variant_changed_message_behavior.rs` (registered by the page) |
| 207 | AttachedToLogicalTreeBehavior | `Views/Pages/AttachedToLogicalTreeBehaviorView` | C | 38 | ported | owns `Behaviors/attached_to_logical_tree_message_behavior.rs` (registered by the page) |
| 208 | AttachedToLogicalTreeTrigger | `Views/Pages/AttachedToLogicalTreeTriggerView` | C | 35 | ported |  |
| 209 | AttachedToVisualTreeTrigger | `Views/Pages/AttachedToVisualTreeTriggerView` | C | 35 | ported |  |
| 210 | BindingBehavior | `Views/Pages/BindingBehaviorView` | C | 38 | ported |  |
| 211 | DataContextChangedBehavior | `Views/Pages/DataContextChangedBehaviorView` | C | 38 | ported | owns `Behaviors/data_context_changed_message_behavior.rs` (registered by the page) |
| 212 | DataContextChangedTrigger | `Views/Pages/DataContextChangedTriggerView` | C | 35 | ported |  |
| 213 | DetachedFromLogicalTreeTrigger | `Views/Pages/DetachedFromLogicalTreeTriggerView` | C | 49 | ported |  |
| 214 | DetachedFromVisualTreeTrigger | `Views/Pages/DetachedFromVisualTreeTriggerView` | C | 49 | ported |  |
| 215 | DisposingBehavior | `Views/Pages/DisposingBehaviorView` | C | 39 | ported | owns `Behaviors/disposing_message_behavior.rs` (registered by the page) |
| 216 | DisposingTrigger | `Views/Pages/DisposingTriggerView` | C | 41 | ported | owns `Triggers/pointer_pressed_disposing_trigger.rs` (registered by the page) |
| 217 | DisposableAction | `Views/Pages/DisposableActionView` | C | 39 | ported | owns `Behaviors/disposable_action_behavior.rs` (registered by the page) |
| 218 | InitializedBehavior | `Views/Pages/InitializedBehaviorView` | C | 39 | ported | owns `Behaviors/initialized_message_behavior.rs` (registered by the page) |
| 219 | InitializedTrigger | `Views/Pages/InitializedTriggerView` | C | 40 | ported |  |
| 220 | InvokeCommandActionBase | `Views/Pages/InvokeCommandActionBaseView` | C | 36 | ported |  |
| 221 | LoadedBehavior | `Views/Pages/LoadedBehaviorView` | C | 37 | ported | owns `Behaviors/loaded_message_behavior.rs` (registered by the page) |
| 222 | ResourcesChangedBehavior | `Views/Pages/ResourcesChangedBehaviorView` | C | 46 | ported | owns `Behaviors/resources_changed_message_behavior.rs` (registered by the page); code-behind: the button sets the resource `Color` of the border |
| 223 | ResourcesChangedTrigger | `Views/Pages/ResourcesChangedTriggerView` | C | 48 | ported | code-behind: the button sets the resource `Color` of the border |
| 224 | RoutedEventTriggerBase | `Views/Pages/RoutedEventTriggerBaseView` | C | 40 | ported |  |
| 225 | UnloadedTrigger | `Views/Pages/UnloadedTriggerView` | C | 49 | ported |  |
| 226 | FileSystem | `Views/Pages/FileSystemView` | C | 93 | ported | own view model (`ViewModels/file_system_view_model.rs`); the gallery enables the feature `file-system` of `ferroui-xaml-interactions`; the folder and the text name FerroUI |
| 227 | Network | `Views/Pages/NetworkView` | C | 70 | ported | own view model (`ViewModels/network_view_model.rs`); the gallery enables the feature `network` of `ferroui-xaml-interactions` |
| 228 | ToggleClassAction | `Views/Pages/ToggleClassActionView` | C | 40 | ported |  |
| 229 | LaunchUriAction | `Views/Pages/LaunchUriActionView` | C | 34 | ported | the button opens the FerroUI repository (the reference names the site of its framework) |
| 230 | ScreenshotAction | `Views/Pages/ScreenshotActionView` | C | 34 | ported |  |
| 231 | ScrollToControlAction | `Views/Pages/ScrollToControlActionView` | C | 40 | ported |  |
| 232 | ConsoleBeepAction | `Views/Pages/ConsoleBeepActionView` | C | 34 | ported |  |
| 233 | BreakAction | `Views/Pages/BreakActionView` | C | 34 | ported |  |
| 234 | InactivityTrigger | `Views/Pages/InactivityTriggerView` | C | 34 | ported |  |
