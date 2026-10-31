//! Reusable native workspace for editing FerroUI XAML.
//!
//! [`DesignerSession`] is the UI-thread transaction boundary used by source editing,
//! visual tools and automation. [`DesignerWorkspace`] is an optional embeddable view
//! of that session. Neither starts a window, server or background transport.
mod affine_manipulation;
mod animation_runtime;
mod animation_session;
mod asset_toolbox;
mod chrome;
mod canvas_guides;
mod canvas_guide_overlay;
mod canvas_guides_editor;
mod collaboration;
mod component_browser;
mod component_session;
mod definition_manipulation;
mod definition_projection;
mod definition_session;
mod design_time;
mod native_types;
mod design_time_samples;
mod editors;
mod element_names;
mod focus_order;
mod focus_order_editor;
mod grid_track_editor;
mod icons;
mod interaction_session;
mod layer_state;
mod layout_helper_renderer;
mod preview_environment;
mod preview_host;
mod preview_prototype;
mod preview_prototype_editor;
mod preview_prototype_player;
mod preview_prototype_view;
#[cfg(test)]
mod preview_prototype_ui_tests;
mod preview_stages;
mod preview_stage_export;
mod preview_stage_matrix;
mod preview_stage_matrix_editor;
#[cfg(test)]
mod preview_stage_matrix_ui_tests;
mod preview_stage_scenarios;
mod preview_stage_editor;
mod preview_stage_view;
mod preview_source;
mod property_color_editor;
mod property_controls;
mod property_inspector_view;
mod property_literals;
mod property_transfer;
mod property_transfer_editor;
mod resource_editor;
mod runtime;
mod selection_layout;
mod session;
pub mod sharing;
mod surface;
mod timeline_lane;
mod viewport;
mod workspace;

pub use asset_toolbox::{
    AssetIcon, AssetToolbox, AssetViewMode, DesignerAsset, default_designer_assets,
};
pub use chrome::designer_chrome_style;
pub use canvas_guides::{CanvasGuide, CanvasGuideAxis, CanvasGuideSettings};
pub use canvas_guides_editor::CanvasGuidesEditor;
pub use workspace::{CanvasGuideEditSnapshot, CanvasGuideViewSnapshot};
pub use component_browser::{ComponentLibrary, ComponentLibraryFilter, ComponentLibraryItem};
pub use component_session::ComponentEditPreview;
pub use definition_projection::{DefinitionPreviewOptions, DefinitionScope};
pub use design_time::{
    DesignTimeEdit, DesignTimeEditor, DesignTimeEditorModel, DesignTimeProperty,
    DesignTimeSnapshot, plan_design_time_edits,
};
pub use editors::{
    PropertyBindingDraft, PropertyBindingEditor, PropertyBindingOption, PropertyBindingSource,
    PropertyEditorContext, PropertyEditorFactory, PropertyEditorKind, PropertyEditorMetadata,
    PropertyLiteralValidator, PropertyObjectDraft, PropertyResourceChoice, PropertyValueCommit,
    PropertyValueEdit, PropertyValueSource, PropertyVectorEditor, binding_expression,
    create_property_editor, resolve_property_editor_metadata,
};
pub use ferroui_designer_core::{Document, Edit, NodeId};
pub use element_names::plan_element_names;
pub use ferroui_designer_tools as tools;
pub use focus_order::{
    FocusLabelAssociation, FocusLabelTarget, FocusOrderCatalog, FocusOrderEdit, FocusOrderItem,
    FocusOrderScope, FocusOrderSnapshot, navigation_mode_name,
};
pub use focus_order_editor::FocusOrderEditor;
pub use grid_track_editor::GridTrackEditor;
pub use layer_state::{LayerCommand, LayerEditSnapshot, LayerState, LayerViewSnapshot};
pub use preview_environment::{
    PREVIEW_SIZE_PRESETS, PreviewEnvironment, PreviewEnvironmentEditor, PreviewFlowDirection,
    PreviewSizePreset, PreviewTheme,
};
pub use preview_host::{
    PreviewFactory, PreviewHostOptions, PreviewRequest, ResourcePreviewFactory,
};
pub use preview_prototype::{
    PreviewPrototype, PreviewPrototypeAction, PreviewPrototypeActionManifest, PreviewPrototypeFlow,
    PreviewPrototypeFlowManifest, PreviewPrototypeLink, PreviewPrototypeLinkManifest,
    PreviewPrototypeManifest, PreviewPrototypeOverlay, PreviewPrototypeOverlayOptions,
    PreviewPrototypeOverlayPlacement, PreviewPrototypeSession, MAX_PREVIEW_PROTOTYPE_FLOWS,
    MAX_PREVIEW_PROTOTYPE_HISTORY, MAX_PREVIEW_PROTOTYPE_LINKS, MAX_PREVIEW_PROTOTYPE_OVERLAYS,
    MAX_PREVIEW_PROTOTYPE_OVERLAY_OFFSET,
};
pub use preview_prototype_editor::{
    PreviewPrototypeElementChoice, PreviewPrototypeFlowEditor, PreviewPrototypeLinkEditor,
};
pub use preview_prototype_player::{
    PreviewPrototypeHotspot, PreviewPrototypePlayerAction, PreviewPrototypePlayerLayer,
    PreviewPrototypePlayerView,
};
pub use preview_prototype_view::{PreviewPrototypeView, PreviewPrototypeViewAction};
pub use preview_stages::{
    MAX_PREVIEW_STAGES, PreviewSourceAnchor, PreviewSourceSegment, PreviewStageBoard,
    PreviewStageConfiguration, PreviewStageId, PreviewStageInfo, PreviewStageManifest,
    PreviewStageLayout, PreviewStageSize, PreviewStageMode, PreviewStageSelection, PreviewStageSelectionToken, PreviewStageSnapshot,
    PreviewStageSource, PreviewStageState, PreviewStageStatus, PreviewStageTarget,
};
pub use preview_stage_editor::PreviewStageEditor;
pub use preview_stage_export::{
    capture_preview_stage_png, PreviewStageExportOptions, PreviewStageImage,
    MAX_PREVIEW_STAGE_EXPORT_BYTES, MAX_PREVIEW_STAGE_EXPORT_PIXELS,
};
pub use preview_stage_matrix::{PreviewStageMatrix, PreviewStageMatrixSize, PreviewStageStateAxis};
pub use preview_stage_matrix_editor::PreviewStageMatrixEditor;
pub use preview_stage_view::{PreviewStageAction, PreviewStageComparison, PreviewStageView};
pub use property_color_editor::PropertyColorEditor;
pub use property_controls::{
    PropertyComponentsEditor, PropertyComponentsKind, PropertyNumberOptions,
    PropertyNumberScrubber, property_value_data_format,
};
pub use property_inspector_view::{
    PropertyInspectorFilter, PropertyInspectorSort, PropertyInspectorView,
};
pub use property_transfer::{
    CopiedProperty, CopiedPropertyKind, PropertyClipboard, PropertyPasteChange,
    PropertyPastePreview, PropertyTransferIssue,
};
pub use property_transfer_editor::PropertyTransferEditor;
pub use resource_editor::{
    DefinitionCreationKind, DefinitionEditorCommand, DefinitionPreviewSettings,
    DefinitionPropertyField, DefinitionReference, ResourceDefinitionEditor,
    ResourceDefinitionModel,
};
pub use runtime::{PreviewStatus, RuntimeNode};
pub use selection_layout::{LayoutMatch, LayoutReset};
pub use session::DesignerSession;
pub use surface::{DesignGesture, DesignSurface};
pub use timeline_lane::{EasingPreview, TimelineLane};
pub use viewport::{CanvasViewport, CanvasWheelMode};
pub use workspace::shell::{NavigationPane, WorkspaceViewMode};
pub use workspace::{
    CanvasLayerChoice, CanvasSelectionSnapshot, DesignerDefinitionAsset, DesignerWorkspace,
    OutlineCommand, OutlineMode, PreviewPrototypePlaybackInfo,
};

/// The self-contained example opened by Designer Studio and usable by hosts.
pub const STARTER_XAML: &str = include_str!("Starter.xaml");

#[cfg(test)]
mod layer_state_tests;
#[cfg(test)]
mod native_extensions_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod workspace_authoring_tests;
mod workspace_layout_helpers;
#[cfg(test)]
mod workspace_shell_tests;
#[cfg(test)]
mod workspace_navigation_tests;
#[cfg(test)]
mod workspace_guides_tests;
#[cfg(test)]
mod workspace_navigation_file_tests;
mod workspace_tools;

pub use layout_helper_renderer::{
    DragVisualGhost, LayoutHelperRenderState, hit_test_layout_handle, hover_layout_handle,
    render_layout_helpers, render_layout_helpers_with_state, render_manipulation_metrics,
};
pub use workspace_layout_helpers::{LayoutEditPreview, LayoutHelperSnapshot};

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(all(test, feature = "render-smoke"))]
mod render_tests;

#[cfg(test)]
mod animation_integration_tests;
