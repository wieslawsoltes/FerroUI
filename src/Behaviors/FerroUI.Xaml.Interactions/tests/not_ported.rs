//! The test files of `tests/Xaml.Behaviors.Interactions.UnitTests` of Xaml.Behaviors that test the
//! types of other projects of the reference: the test project of the reference holds the tests of
//! the draggable, drag and drop and custom behaviors too. They are ported with the crates of those
//! projects (docs/porting/behaviors.md, stages 3 and 4), in their own test modules: every test
//! below has its port there (the two item drag reorder tests, which the reference skips, run
//! there). This crate does not depend on those crates; the custom behaviors are a development
//! dependency only for the logical tree lifetime tests of `Core/`, which test the actions of this
//! crate.
//!
//! 78 test methods; the test names below keep the names of the reference, except the name of its
//! framework, which three of them contain and which is replaced by `Framework` or left out
//! (`ValidationRules_ExposeConfigurationAs..Properties`, two names of
//! `ManagedContextDropBehaviorTests.cs`).

// Core/ItemDragBehaviorTests.cs (the item drag behaviors of the crate `ferroui-xaml-interactions-draggable`, stage 3):
// Not ported: ItemDragBehavior_Reorders_Vertical - tests a type of another project; ported with its crate.
// Not ported: ItemDragBehavior_Reorders_Horizontal - tests a type of another project; ported with its crate.
// Not ported: ListReorderDragBehavior_CanStartAgainAfterVirtualizedItemMoves - tests a type of another project; ported with its crate.
// Not ported: ListReorderDragBehavior_CanStartAfterListReattaches - tests a type of another project; ported with its crate.

// Custom/AnimationAdapterTests.cs (the animation behaviors of the crate `ferroui-xaml-interactions-custom`, stage 4):
// Not ported: TransitionsBehavior_AppliesAndRestoresTransitions - tests a type of another project; ported with its crate.
// Not ported: RuntimeLoader_ResolvesStandaloneAnimationsAndBehaviorAdapters - tests a type of another project; ported with its crate.
// Not ported: TransitionsChangedTrigger_ExecutesForSharedTransitionObservation - tests a type of another project; ported with its crate.
// Not ported: FluidMoveBehavior_AnimatesChangedChildPositionThroughSharedPrimitive - tests a type of another project; ported with its crate.
// Not ported: ParallaxBehavior_TracksScrollOffsetThroughSharedPrimitive - tests a type of another project; ported with its crate.
// Not ported: AnimationActions_PreserveTargetsResultsAndBuilderPrecedence - tests a type of another project; ported with its crate.
// Not ported: AnimateOnAttachedBehavior_UsesBuilderWhenAnimationIsMissing - tests a type of another project; ported with its crate.
// Not ported: PlayAnimationBehavior_RunsConfiguredAnimationOnAssociatedVisual - tests a type of another project; ported with its crate.
// Not ported: AnimationCompletedTrigger_ExecutesImmediatelyWithoutAnimation - tests a type of another project; ported with its crate.
// Not ported: RunAnimationTrigger_UsesBuilderAndExecutesAfterCompletion - tests a type of another project; ported with its crate.

// Custom/AttachedToVisualTreeLifecycleTests.cs (the custom behaviors, stage 4):
// Not ported: PointerEnteredTrigger_ExecutesOnceAfterVisualTreeReattach - tests a type of another project; ported with its crate.
// Not ported: AttachedToVisualTreeTriggerBase_DisposesForEachVisualTreeLifetime - tests a type of another project; ported with its crate.
// Not ported: AttachedToVisualTreeBehavior_DisposesForEachVisualTreeLifetime - tests a type of another project; ported with its crate.

// Custom/ClickEventTriggerTests.cs (the click trigger of the custom behaviors, stage 4):
// Not ported: ClickEventTrigger_SaveFilePickerAction_Cancel_DoesNotExecuteCommand - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_ButtonWithPickerAction_Cancel_DoesNotInvokeNativeButtonCommand - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_PickerAction_UseCommandCanExecuteForIsEnabled_DisablesAssociatedPanel - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_AsyncActionGroup_DetachesChildCommandObservers - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_RoutingStrategies_PropertyChange_RewiresHandlers - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_ReleaseMode_DoesNotFireWhenReleasedOutside - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_ReleaseMode_DoesNotLeakPressedStateAcrossInteractions - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_PressMode_FiresOnPointerDown - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_KeyModifiers_FilterBlocksAndAllows - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_SourceControl_UsesExternalSourceControl - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_HandledEventsToo_DefaultFalse_AndPropertyChange_RewiresHandlers_ForButtonAndTextBoxSources - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_HandledEventsToo_AlsoControlsKeyboardHandlers - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_HandleEventFalse_Button_PreservesNativeClick - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_HandleEventFalse_TextBox_PreservesKeyBubbling - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_HandleEventFalse_TextBox_PointerClick_FiresTriggerAndClickEvent - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_HandleEventFalse_TextBox_AllowsPointerDragSelection - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_SpaceHandling_WorksForPressAndReleaseModes - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_IsDefault_RootEnter_TriggersAndDoesNotDuplicate - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_IsCancel_RootEscape_TriggersAndDoesNotDuplicate - tests a type of another project; ported with its crate.
// Not ported: ClickEventTrigger_Flyout_TogglesOpenAndClose - tests a type of another project; ported with its crate.

// Custom/DetachedFromVisualTreeTriggerTests.cs (the custom behaviors, stage 4):
// Not ported: SwitchingTabs_ExecutesBoundDetachedCommand - tests a type of another project; ported with its crate.

// Custom/ExecuteCommandBehaviorBaseTests.cs (the custom behaviors, stage 4):
// Not ported: CanExecuteCommand_UpdatesWhenCommandRaisesCanExecuteChanged - tests a type of another project; ported with its crate.
// Not ported: CanExecuteCommand_UsesCurrentCommandParameter - tests a type of another project; ported with its crate.
// Not ported: CanExecuteCommand_RewiresSubscriptionsWhenCommandChangesAndDetaches - tests a type of another project; ported with its crate.

// Custom/FocusNextElementActionTests.cs (the custom behaviors, stage 4):
// Not ported: Execute_Uses_Framework_TabNavigation_Rules - tests a type of another project; ported with its crate.
// Not ported: Execute_DoesNotRaise_Tab_KeyDown_Handlers - tests a type of another project; ported with its crate.
// Not ported: Execute_Skips_None_TabNavigation_Children - tests a type of another project; ported with its crate.
// Not ported: Execute_Uses_TabOnceActiveElement_For_Once_Containers - tests a type of another project; ported with its crate.
// Not ported: Execute_Respects_Local_TabNavigation_Subtree_Order - tests a type of another project; ported with its crate.

// Custom/FocusTrapBehaviorTests.cs (the custom behaviors, stage 4):
// Not ported: Tab_Inside_Cycle_Group_Uses_Framework_Navigation - tests a type of another project; ported with its crate.

// Custom/InlineEditBehaviorTests.cs (the custom behaviors, stage 4):
// Not ported: DoubleTapped_DisplayControl_Begins_Edit - tests a type of another project; ported with its crate.
// Not ported: DoubleTapped_AssociatedObject_Begins_Edit - tests a type of another project; ported with its crate.
// Not ported: DoubleTapped_EditControl_DoesNotRestartEdit - tests a type of another project; ported with its crate.

// Custom/InvokeButtonClickActionTests.cs (the custom behaviors, stage 4):
// Not ported: Execute_Invokes_Button_Command_With_Parameter - tests a type of another project; ported with its crate.
// Not ported: Execute_Respects_Handled_Click_Event - tests a type of another project; ported with its crate.
// Not ported: Execute_Does_Not_Invoke_Disabled_Button - tests a type of another project; ported with its crate.

// Custom/RoutedEventTriggerBehaviorTests.cs (the custom behaviors, stage 4):
// Not ported: WindowClosedEvent_ExecutesBoundCommand - tests a type of another project; ported with its crate.
// Not ported: ControlRoutedEvent_UnsubscribesWhenControlLeavesVisualTree - tests a type of another project; ported with its crate.

// Custom/ToolTipTriggerTests.cs (the tool tip triggers of the custom behaviors, stage 4):
// Not ported: ToolTipOpeningTrigger_ExecutesActionsForDirectEvent - tests a type of another project; ported with its crate.
// Not ported: ToolTipClosingTrigger_ExecutesActionsForDirectEvent - tests a type of another project; ported with its crate.
// Not ported: EventTriggerBehavior_ExecutesActionsForToolTipOpeningEvent - tests a type of another project; ported with its crate.
// Not ported: EventTriggerBehavior_ExecutesActionsForToolTipClosingEvent - tests a type of another project; ported with its crate.

// Custom/ValidationRuleTests.cs (the validation rules of the custom behaviors, stage 4):
// Not ported: BoundRangeChanges_RevalidateAssociatedProperty - tests a type of another project; ported with its crate.
// Not ported: RuleCollectionChanges_UpdateSubscriptionsAndValidation - tests a type of another project; ported with its crate.
// Not ported: ValidationRules_ExposeConfigurationAsFrameworkProperties - tests a type of another project; ported with its crate.

// Custom/XmlnsDefinitionRuntimeXamlTests.cs (the namespaces of the custom behaviors, stage 4):
// Not ported: Runtime_Loader_Resolves_Explicit_Behavior_Assemblies_Alongside_Application_XmlnsDefinitions - tests a type of another project; ported with its crate.

// DragAndDrop/ContextDragBehaviorTests.cs (the crate `ferroui-xaml-interactions-draganddrop`, stage 3):
// Not ported: PointerPressed_Is_Not_Handled_For_Selected_Item_Content - tests a type of another project; ported with its crate.
// Not ported: Escape_Cancels_Drag - tests a type of another project; ported with its crate.
// Not ported: VirtualizedListBoxItem_ReplacementTransfersContextDragLifecycle - tests a type of another project; ported with its crate.

// DragAndDrop/ContextDropBehaviorTests.cs (the crate `ferroui-xaml-interactions-draganddrop`, stage 3):
// Not ported: DropHandler_Receives_Context_Data - tests a type of another project; ported with its crate.

// DragAndDrop/DragDropCommandsBehaviorTests.cs (the crate `ferroui-xaml-interactions-draganddrop`, stage 3):
// Not ported: CanExecuteCommandProperties_UpdateWhenCommandsRaiseCanExecuteChanged - tests a type of another project; ported with its crate.
// Not ported: CanExecuteCommandProperties_RewireSubscriptionsWhenCommandsChangeAndDetach - tests a type of another project; ported with its crate.
// Not ported: CanExecuteCommandProperties_UseNullWhenEventArgsAreNotPassed - tests a type of another project; ported with its crate.
// Not ported: CanExecuteCommandProperties_DeferWhenDragEventArgsAreUnavailable - tests a type of another project; ported with its crate.
// Not ported: CanExecuteCommandProperties_UseCanExecuteCommandParameterWhenEventArgsArePassed - tests a type of another project; ported with its crate.
// Not ported: PassEventArgsToCommand_UpdatesCanExecuteParameterAvailability - tests a type of another project; ported with its crate.

// DragAndDrop/ManagedContextDropBehaviorTests.cs (the crate `ferroui-xaml-interactions-draganddrop`, stage 3):
// Not ported: ManagedDataTransferExtensions_Return_Original_Payload - tests a type of another project; ported with its crate.
// Not ported: ManagedDataTransferExtensions_Keep_Text_Payload_Compatible_With_Text_APIs - tests a type of another project; ported with its crate.
// Not ported: ManagedContextDropBehavior_CreateDataTransfer_Preserves_Payload_In_DragEventArgs_DataTransfer - tests a type of another project; ported with its crate.
// Not ported: ManagedDataTransferExtensions_Keep_ByteArray_Payload_Compatible_With_Byte_APIs - tests a type of another project; ported with its crate.

// DragAndDrop/PanelDragBehaviorTests.cs (the crate `ferroui-xaml-interactions-draganddrop`, stage 3):
// Not ported: DragRectangle_Moves_Between_Panels - tests a type of another project; ported with its crate.
