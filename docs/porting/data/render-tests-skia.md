# Render tests: measured errors, Skia backend

Written from `results.tsv` of a run of `cargo test -p ferroui-render-tests` on 2026-10-10 (macOS, Apple M3 Pro) by `scripts/render_tests_report.py`. The error is the root mean square error of upstream's comparison; the allowed error is 0.022. See `docs/porting/render-tests.md`.

| Test | Immediate | Composited | Other output | Result |
|---|---|---|---|---|
| `CrossTests/Media/DrawingContext/Transform_Should_Work_As_Expected` |  | 0.0002 |  | pass |
| `CrossTests/Media/Geometry/Should_Render_Geometry_With_Strokeless_Lines` |  | 0.0098 |  | pass |
| `CrossTests/Media/Geometry/Should_Render_PolyBezierSegment_With_Strokeless_Lines` |  | 0.0057 |  | pass |
| `CrossTests/Media/Geometry/Should_Render_PolyLineSegment_With_Strokeless_Lines` |  | 0.0000 |  | pass |
| `CrossTests/Media/Geometry/Should_Render_Stream_Geometry` |  | 0.0096 |  | pass |
| `CrossTests/Media/ImageScaling/Downscaling_With_HighQuality_Should_Be_Antialiased` |  | 0.0194 |  | pass |
| `CrossTests/Media/ImageScaling/Upscaling_With_HighQuality_Should_Be_Antialiased` |  | 0.0023 |  | pass |
| `CrossTests/Media/RadialGradientBrush/Transform_Should_Work_As_Expected` |  | 0.0039 |  | pass |
| `CrossTests/Media/RelativeTransformBrush/Drawing_Brush_Relative_Transform_Should_Work_As_Expected` |  | 0.0157 |  | pass |
| `CrossTests/Media/RelativeTransformBrush/Image_Brush_Relative_Transform_Should_Work_As_Expected` |  | 0.0109 |  | pass |
| `CrossTests/Media/RelativeTransformBrush/Linear_Gradient_Relative_Transform_Should_Work_As_Expected` |  | 0.0000 |  | pass |
| `CrossTests/Media/RelativeTransformBrush/Radial_Gradient_Relative_Transform_Should_Work_As_Expected` |  | 0.0000 |  | pass |
| `CrossTests/Media/RelativeTransformBrush/Relative_Transform_Should_Apply_Before_Transform` |  | 0.0000 |  | pass |
| `CrossTests/Media/RelativeTransformBrush/Relative_Transform_Should_Follow_The_Bounds_Of_Each_Fill` |  | 0.0000 |  | pass |
| `CrossTests/Media/RelativeTransformBrush/Solid_Color_Relative_Transform_Should_Be_Ignored` |  | 0.0000 |  | pass |
| `CrossTests/Media/RelativeTransformBrush/Tiled_Drawing_Brush_Relative_Transform_Should_Work_As_Expected` |  | 0.0169 |  | pass |
| `CrossTests/Media/RelativeTransformBrush/Tiled_Image_Brush_Relative_And_Absolute_Transform_Should_Work_As_Expected` |  | 0.0109 |  | pass |
| `CrossTests/Media/TileBrushes/Should_Render_Aligned_TileBrush` |  | 0.0065 |  | pass |
| `CrossTests/Media/TileBrushes/Should_Render_Drawing_Brush_With_Transform_On_An_Offset_Fill` |  | 0.0000 |  | pass |
| `CrossTests/Media/TileBrushes/Should_Render_Image_Brush_With_Transform_On_An_Offset_Fill` |  | 0.0010 |  | pass |
| `CrossTests/Media/TileBrushes/Should_Render_Scaled_TileBrush` |  | 0.0099 |  | pass |
| `CrossTests/Media/TileBrushes/Should_Render_TileBrush_With_TileMode_None` |  | 0.0000 |  | pass |
| `CrossTests/Media/TileBrushes/Should_Render_With_Transform` |  | 0.0000 |  | pass |
| `CrossTests/Media/TileBrushes/Simple_Checkboard_Pattern_Is_Rendered_Identically` |  | 0.0000 |  | pass |
| `Skia/BugRepros/Sibling_Visuals_With_Opacity_Should_Not_Affect_Each_Other` | 0.0007 | 0.0007 |  | pass |
| `Skia/Composition/DirectFb/Should_Only_Update_Clipped_Rects_When_Retained_Fb_Is_Advertised_advertized-False_initial` |  | 0.0000 |  | pass |
| `Skia/Composition/DirectFb/Should_Only_Update_Clipped_Rects_When_Retained_Fb_Is_Advertised_advertized-False_updated` |  | 0.0000 |  | pass |
| `Skia/Composition/DirectFb/Should_Only_Update_Clipped_Rects_When_Retained_Fb_Is_Advertised_advertized-True_initial` |  | 0.0000 |  | pass |
| `Skia/Composition/DirectFb/Should_Only_Update_Clipped_Rects_When_Retained_Fb_Is_Advertised_advertized-True_updated` |  | 0.0000 |  | pass |
| `Skia/Controls/Adorner/Focus_Adorner_Is_Properly_Clipped_Clip_False` |  | 0.0000 |  | pass |
| `Skia/Controls/Adorner/Focus_Adorner_Is_Properly_Clipped_Clip_True` |  | 0.0000 |  | pass |
| `Skia/Controls/Border/Border_1px_Border` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Border/Border_2px_Border` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Border/Border_Brush_Offsets_Content` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Border/Border_Clips_To_Round_Bounds_NonUniform` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Border/Border_Clips_To_Round_Bounds_Uniform` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Border/Border_Fill` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Border/Border_Margin_Offsets_Content` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Border/Border_Nested_Rotate` | 0.0027 | 0.0027 |  | pass |
| `Skia/Controls/Border/Border_NonUniform_CornerRadius` | 0.0020 | 0.0020 |  | pass |
| `Skia/Controls/Border/Border_Padding_Offsets_Content` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Border/Border_Stretches_Content_Horizontally` | 0.0195 | 0.0195 |  | pass |
| `Skia/Controls/Border/Border_Stretches_Content_Vertically` | 0.0195 | 0.0195 |  | pass |
| `Skia/Controls/Border/Border_Uniform_CornerRadius` | 0.0022 | 0.0022 |  | pass |
| `Skia/Controls/Carousel/Carousel_ViewportFraction_MiddleItemSelected_ShowsSidePeeks` |  | 0.0008 |  | pass |
| `Skia/Controls/CarouselPage/CarouselPage_Blue_Page` |  | 0.0018 |  | pass |
| `Skia/Controls/CarouselPage/CarouselPage_CustomBackground` |  | 0.0034 |  | pass |
| `Skia/Controls/CarouselPage/CarouselPage_Green_Page` |  | 0.0020 |  | pass |
| `Skia/Controls/CarouselPage/CarouselPage_Red_Page` |  | 0.0022 |  | pass |
| `Skia/Controls/CarouselPage/CarouselPage_ThreePages_FirstSelected` |  | 0.0018 |  | pass |
| `Skia/Controls/CarouselPage/CarouselPage_ThreePages_SecondSelected` |  | 0.0020 |  | pass |
| `Skia/Controls/CommandBar/CommandBar_Compact_LabelCollapsed` |  | 0.0038 |  | pass |
| `Skia/Controls/CommandBar/CommandBar_Default_PrimaryCommands` |  | 0.0034 |  | pass |
| `Skia/Controls/ContentPage/ContentPage_Default_Content` |  | 0.0000 |  | pass |
| `Skia/Controls/ContentPage/ContentPage_WithTopAndBottomCommandBars` |  | 0.0018 |  | pass |
| `Skia/Controls/CustomRender/Clip` | 0.0023 | 0.0023 |  | pass |
| `Skia/Controls/CustomRender/Clip_With_Transform` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/CustomRender/GeometryClip` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/CustomRender/GeometryClip_With_Transform` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/CustomRender/Opacity` | 0.0034 | 0.0034 |  | pass |
| `Skia/Controls/CustomRender/OpacityMask` | 0.0026 | 0.0026 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_BottomPlacement_CompactInline_Closed_ShowsRail` |  | 0.0003 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_BottomPlacement_CompactOverlay_Closed_ShowsRail` |  | 0.0003 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_BottomPlacement_Open` |  | 0.0000 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_Closed_ShowsTopBar` |  | 0.0021 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_LeftPlacement_CompactInline_Closed_ShowsRail` |  | 0.0003 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_LeftPlacement_CompactInline_Open_PanePushesContent` |  | 0.0001 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_LeftPlacement_CompactOverlay_Closed_ShowsRail` |  | 0.0003 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_LeftPlacement_CompactOverlay_Open_PaneOverlaysContent` |  | 0.0001 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_Locked_NoTopBar` |  | 0.0000 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_Open_ShowsDrawerPane` |  | 0.0000 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_RightPlacement_CompactInline_Closed_ShowsRail` |  | 0.0003 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_RightPlacement_CompactOverlay_Closed_ShowsRail` |  | 0.0003 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_RightPlacement_Open` |  | 0.0020 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_Split_Open_ShowsBothPanes` |  | 0.0001 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_TopPlacement_CompactInline_Closed_ShowsRail` |  | 0.0003 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_TopPlacement_CompactOverlay_Closed_ShowsRail` |  | 0.0003 |  | pass |
| `Skia/Controls/DrawerPage/DrawerPage_TopPlacement_Open` |  | 0.0000 |  | pass |
| `Skia/Controls/Image/Image_Rotated_EdgeMode_Aliased` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Image/Image_Rotated_EdgeMode_Antialias` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Image/Image_Rotated_EdgeMode_Unspecified` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Image/Image_Stretch_Fill` | 0.0002 | 0.0002 |  | pass |
| `Skia/Controls/Image/Image_Stretch_None` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Image/Image_Stretch_Uniform` | 0.0002 | 0.0002 |  | pass |
| `Skia/Controls/Image/Image_Stretch_UniformToFill` | 0.0003 | 0.0003 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Color` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_ColorBurn` | 0.0002 | 0.0002 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_ColorDodge` | 0.0002 | 0.0002 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Darken` | 0.0002 | 0.0002 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Difference` | 0.0005 | 0.0005 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Exclusion` | 0.0023 | 0.0023 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_HardLight` | 0.0010 | 0.0010 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Hue` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Lighten` | 0.0003 | 0.0003 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Luminosity` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Multiply` | 0.0011 | 0.0011 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Nothing` | 0.0013 | 0.0013 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Overlay` | 0.0010 | 0.0010 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Plus` | 0.0013 | 0.0013 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Saturation` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_Screen` | 0.0015 | 0.0015 |  | pass |
| `Skia/Controls/Image/blend/Image_Blend_SoftLight` | 0.0003 | 0.0003 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_Destination` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_DestinationAtop` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_DestinationIn` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_DestinationOut` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_DestinationOver` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_Source` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_SourceAtop` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_SourceIn` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_SourceOut` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_SourceOver` | 0.0001 | 0.0001 |  | pass |
| `Skia/Controls/Image/composition/Image_Blend_Xor` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/NavigationPage/NavigationPage_CustomBarBackground` |  | 0.0000 |  | pass |
| `Skia/Controls/NavigationPage/NavigationPage_SinglePage_ShowsNavBar` |  | 0.0000 |  | pass |
| `Skia/Controls/NavigationPage/NavigationPage_TwoPages_ShowsBackButton` |  | 0.0007 |  | pass |
| `Skia/Controls/PipsPager/PipsPager_Default` |  | 0.0000 |  | pass |
| `Skia/Controls/PipsPager/PipsPager_Preselected_Index` |  | 0.0000 |  | pass |
| `Skia/Controls/TabbedPage/TabbedPage_BottomPlacement` |  | 0.0001 |  | pass |
| `Skia/Controls/TabbedPage/TabbedPage_CustomBarBackground` |  | 0.0000 |  | pass |
| `Skia/Controls/TabbedPage/TabbedPage_CustomTabColors` |  | 0.0000 |  | pass |
| `Skia/Controls/TabbedPage/TabbedPage_Default_TopPlacement_FirstTabSelected` |  | 0.0001 |  | pass |
| `Skia/Controls/TabbedPage/TabbedPage_LeftPlacement` |  | 0.0000 |  | pass |
| `Skia/Controls/TabbedPage/TabbedPage_RightPlacement` |  | 0.0000 |  | pass |
| `Skia/Controls/TabbedPage/TabbedPage_TopPlacement_SecondTabSelected` |  | 0.0001 |  | pass |
| `Skia/Controls/TabbedPage/TabbedPage_TwoTabs` |  | 0.0001 |  | pass |
| `Skia/Controls/TabbedPage/TabbedPage_WithIcons_BottomPlacement` |  | 0.0145 |  | pass |
| `Skia/Controls/TabbedPage/TabbedPage_WithIcons_TopPlacement` |  | 0.0145 |  | pass |
| `Skia/Controls/TextBox/Placeholder_With_Blue_Foreground` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/TextBox/Placeholder_With_Default_Foreground` | 0.0000 | 0.0000 |  | pass |
| `Skia/Controls/TextBox/Placeholder_With_Red_Foreground` | 0.0000 | 0.0000 |  | pass |
| `Skia/GeometryClipping/Geometry_Clip_Clips_Path` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Bgr24_Normal` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Bgr24_Writeable` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Bgr24_WriteableInitialized` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_BlackWhite_Normal` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_BlackWhite_Writeable` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_BlackWhite_WriteableInitialized` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray16_Normal` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray16_Writeable` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray16_WriteableInitialized` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray2_Normal` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray2_Writeable` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray2_WriteableInitialized` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray32Float_Normal` |  |  | 0.0065 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray32Float_Writeable` |  |  | 0.0065 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray32Float_WriteableInitialized` |  |  | 0.0065 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray4_Normal` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray4_Writeable` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray4_WriteableInitialized` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray8_Normal` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray8_Writeable` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Gray8_WriteableInitialized` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Prgba64_Normal` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Prgba64_Writeable` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Prgba64_WriteableInitialized` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Rgb24_Normal` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Rgb24_Writeable` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Rgb24_WriteableInitialized` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Rgba64_Normal` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Rgba64_Writeable` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/BitmapsShouldSupportTranscoders_Lenna_Rgba64_WriteableInitialized` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/FramebufferRenderResultsShouldBeUsableAsBitmap_Bgra8888` |  |  | 0.0008 | pass |
| `Skia/Media/Bitmap/FramebufferRenderResultsShouldBeUsableAsBitmap_Rgb565` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/FramebufferRenderResultsShouldBeUsableAsBitmap_Rgba8888` |  |  | 0.0008 | pass |
| `Skia/Media/Bitmap/WriteableBitmapShouldBeUsable_Bgra8888` |  |  | 0.0000 | pass |
| `Skia/Media/Bitmap/WriteableBitmapShouldBeUsable_Rgba8888` |  |  | 0.0000 | pass |
| `Skia/Media/BoxShadow/BoxShadowShouldBeRenderedEvenWithNullBrushAndPen` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/CombinedGeometry/Geometry1_Transform` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/CombinedGeometry/GeometryCombineMode_Exclude` | 0.0053 | 0.0053 |  | pass |
| `Skia/Media/CombinedGeometry/GeometryCombineMode_Intersect` | 0.0051 | 0.0051 |  | pass |
| `Skia/Media/CombinedGeometry/GeometryCombineMode_Union` | 0.0109 | 0.0109 |  | pass |
| `Skia/Media/CombinedGeometry/GeometryCombineMode_Xor` | 0.0119 | 0.0119 |  | pass |
| `Skia/Media/ConicGradientBrush/ConicGradientBrushIsProperlyMapped_Absolute` | 0.0024 | 0.0024 |  | pass |
| `Skia/Media/ConicGradientBrush/ConicGradientBrushIsProperlyMapped_Relative` | 0.0043 | 0.0043 |  | pass |
| `Skia/Media/ConicGradientBrush/ConicGradientBrush_DrawingContext` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ConicGradientBrush/ConicGradientBrush_RedBlue` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ConicGradientBrush/ConicGradientBrush_RedBlue_Center` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ConicGradientBrush/ConicGradientBrush_RedBlue_Center_and_Rotation` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ConicGradientBrush/ConicGradientBrush_RedBlue_Rotation` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ConicGradientBrush/ConicGradientBrush_RedBlue_SoftEdge` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ConicGradientBrush/ConicGradientBrush_Transform_Applies_After_Angle` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ConicGradientBrush/ConicGradientBrush_Umbrella` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/DrawingBrush/DrawingBrushIsProperlyScaled` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/DrawingBrush/DrawingBrushIsProperlyTiled` | 0.0007 | 0.0007 |  | pass |
| `Skia/Media/DrawingBrush/DrawingBrushIsProperlyUpscaled` | 0.0003 | 0.0003 |  | pass |
| `Skia/Media/DrawingContent/DrawingBrush_Reflects_Replaced_Inner_Brush` |  | 0.0000 |  | pass |
| `Skia/Media/DrawingContent/DrawingImage_Reflects_Replaced_Inner_Brush` |  | 0.0000 |  | pass |
| `Skia/Media/DrawingContext/Should_Render_DrawingGroup_With_Effect` | 0.0001 | 0.0001 |  | pass |
| `Skia/Media/DrawingContext/Should_Render_LinesAndText` |  | 0.0207 |  | pass |
| `Skia/Media/Effects/CachedSiblingFollowedByEffect` |  | 0.0000 |  | pass |
| `Skia/Media/Effects/DropShadowEffect` |  | 0.0062 |  | pass |
| `Skia/Media/Effects/EffectFollowedByNonEffect` |  | 0.0000 |  | pass |
| `Skia/Media/GeometryGroup/Child_Transform` | 0.0001 | 0.0001 |  | pass |
| `Skia/Media/GeometryGroup/FillRule_Stroke_EvenOdd` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/GeometryGroup/FillRule_Stroke_NonZero` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/GlyphOutline/Should_Render_InterVariable_At_Default` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/GlyphOutline/Should_Render_Inter_Composite_Glyph` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/GlyphOutline/Should_Render_Inter_Latin_Glyph` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/GlyphOutline/Should_Render_MiSans_CJK_Glyph` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/GlyphOutline/Should_Render_PointMatched_Composite_Glyph` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/GlyphRun/Should_Render_GlyphRun_Geometry` | 0.0101 | 0.0101 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_Fill_NoTile` | 0.0052 | 0.0052 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_Is_Properly_Mapped_Absolute` | 0.0054 | 0.0054 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_Is_Properly_Mapped_Relative` | 0.0038 | 0.0038 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NoStretch_FlipXY_TopLeftDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NoStretch_FlipX_TopLeftDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NoStretch_FlipY_TopLeftDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NoStretch_NoTile_Alignment_BottomRight` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NoStretch_NoTile_Alignment_Center` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NoStretch_NoTile_Alignment_TopLeft` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NoStretch_NoTile_BottomRightQuarterDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NoStretch_NoTile_BottomRightQuarterSource` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NoStretch_NoTile_BottomRightQuarterSource_BottomRightQuarterDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NoStretch_Tile_BottomRightQuarterSource_CenterQuarterDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_NullSource` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_Should_Render_With_Transform` | 0.0170 | 0.0170 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_Should_Render_With_TransformOrigin` | 0.0087 | 0.0087 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_Tile_Fill` | 0.0023 | 0.0023 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_Tile_Small_Image` | 0.0001 | 0.0001 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_Tile_Small_Image_With_Transform` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_Tile_UniformToFill` | 0.0012 | 0.0012 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_UniformToFill_NoTile` | 0.0044 | 0.0044 |  | pass |
| `Skia/Media/ImageBrush/ImageBrush_Uniform_NoTile` | 0.0039 | 0.0039 |  | pass |
| `Skia/Media/ImageDrawing/ImageDrawing_BottomRight` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/ImageDrawing/ImageDrawing_Fill` | 0.0004 | 0.0004 |  | pass |
| `Skia/Media/ImageDrawing/ImageDrawing_Viewbox` | 0.0047 | 0.0047 |  | pass |
| `Skia/Media/ImageDrawing/Should_Render_DrawingBrushTransform` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/LinearGradientBrush/LinearGradientBrushIsProperlyMapped_Absolute` | 0.0037 | 0.0037 |  | pass |
| `Skia/Media/LinearGradientBrush/LinearGradientBrushIsProperlyMapped_Relative` | 0.0036 | 0.0036 |  | pass |
| `Skia/Media/LinearGradientBrush/LinearGradientBrush_DrawingContext` | 0.0002 | 0.0002 |  | pass |
| `Skia/Media/LinearGradientBrush/LinearGradientBrush_RedBlue_Horizontal_Fill` | 0.0013 | 0.0013 |  | pass |
| `Skia/Media/LinearGradientBrush/LinearGradientBrush_RedBlue_Vertical_Fill` | 0.0013 | 0.0013 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_DrawingContext` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_Is_Properly_Mapped_Absolute_CenterOrigin` | 0.0028 | 0.0028 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_Is_Properly_Mapped_Absolute_MovedOrigin` | 0.0042 | 0.0042 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_Is_Properly_Mapped_Relative_CenterOrigin` | 0.0040 | 0.0040 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_Is_Properly_Mapped_Relative_MovedOrigin` | 0.0059 | 0.0059 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_Partial_Cover` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_RedBlue` | 0.0016 | 0.0016 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_RedBlue_Offset_Inside` | 0.0016 | 0.0016 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_RedBlue_Offset_Outside` | 0.0016 | 0.0016 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_RedGreenBlue_Offset_Inside` | 0.0016 | 0.0016 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_RedGreenBlue_Offset_Outside` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_With_Different_Radius_Is_Properly_Rotated_CenterOrigin` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/RadialGradientBrush/RadialGradientBrush_With_Different_Radius_Is_Properly_Rotated_MovedOrigin` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/RelativeTransformBrush/Drawing_Brush_Rotated_In_Unit_Space` | 0.0001 | 0.0001 |  | pass |
| `Skia/Media/RelativeTransformBrush/Linear_Gradient_Rotated_In_Unit_Space` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/RelativeTransformBrush/Radial_Gradient_Rotated_In_Unit_Space` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/RelativeTransformBrush/Relative_Transform_Applies_Before_Absolute` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/RelativeTransformBrush/Shared_Brush_On_Two_Different_Bounds` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/RenderTargetBitmap/RenderTargetBitmap_DropShadowEffect` | 0.0048 | 0.0048 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Checkerboard_144_Dpi` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Checkerboard_192_Dpi` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Checkerboard_96_Dpi` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Fill_NoTile` | 0.0081 | 0.0081 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Grip_144_Dpi` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Grip_192_Dpi` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Grip_96_Dpi` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_InTree_Visual` | 0.0166 | 0.0166 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Is_Properly_Mapped_Absolute` | 0.0054 | 0.0054 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Is_Properly_Mapped_Relative` | 0.0038 | 0.0038 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_NoStretch_FlipXY_TopLeftDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_NoStretch_FlipX_TopLeftDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_NoStretch_FlipY_TopLeftDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_NoStretch_NoTile_Alignment_BottomRight` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_NoStretch_NoTile_Alignment_Center` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_NoStretch_NoTile_Alignment_TopLeft` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_NoStretch_NoTile_BottomRightQuarterDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_NoStretch_NoTile_BottomRightQuarterSource` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_NoStretch_NoTile_BottomRightQuarterSource_BottomRightQuarterDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_NoStretch_Tile_BottomRightQuarterSource_CenterQuarterDest` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Should_Be_Usable_As_Opacity_Mask` | 0.0000 | 0.0000 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_UniformToFill_NoTile` | 0.0123 | 0.0123 |  | pass |
| `Skia/Media/VisualBrush/VisualBrush_Uniform_NoTile` | 0.0080 | 0.0080 |  | pass |
| `Skia/OpacityMask/Opacity_Mask_Masks_Element` | 0.0013 | 0.0013 |  | pass |
| `Skia/OpacityMask/RenderTransform_Applies_To_Opacity_Mask` | 0.0059 | 0.0059 |  | pass |
| `Skia/SVGPath/SVGPath` | 0.0069 | 0.0069 |  | pass |
| `Skia/Shapes/Ellipse/Circle_1px_Stroke` | 0.0074 | 0.0074 |  | pass |
| `Skia/Shapes/Ellipse/Should_Render_Circle_Aliased` | 0.0043 | 0.0043 |  | pass |
| `Skia/Shapes/Ellipse/Should_Render_Circle_Antialiased` | 0.0003 | 0.0003 |  | pass |
| `Skia/Shapes/Line/Line_1px_Stroke` | 0.0122 | 0.0122 |  | pass |
| `Skia/Shapes/Line/Line_1px_Stroke_Reversed` | 0.0122 | 0.0122 |  | pass |
| `Skia/Shapes/Line/Line_1px_Stroke_Vertical` | 0.0001 | 0.0001 |  | pass |
| `Skia/Shapes/Line/Lines_With_DashArray` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Path/Arc_Absolute` | 0.0072 | 0.0072 |  | pass |
| `Skia/Shapes/Path/Arc_Relative` | 0.0072 | 0.0072 |  | pass |
| `Skia/Shapes/Path/BeginFigure_IsFilled_Is_Respected` | 0.0025 | 0.0025 |  | pass |
| `Skia/Shapes/Path/CubicBezier_Absolute` | 0.0078 | 0.0078 |  | pass |
| `Skia/Shapes/Path/CubicBezier_Relative` | 0.0078 | 0.0078 |  | pass |
| `Skia/Shapes/Path/GetWidenedPathGeometry_Line` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Path/GetWidenedPathGeometry_Line_Dash` | 0.0181 | 0.0181 |  | pass |
| `Skia/Shapes/Path/HorizontalLine_Absolute` | 0.0002 | 0.0002 |  | pass |
| `Skia/Shapes/Path/HorizontalLine_Relative` | 0.0002 | 0.0002 |  | pass |
| `Skia/Shapes/Path/Line_Absolute` | 0.0164 | 0.0164 |  | pass |
| `Skia/Shapes/Path/Line_Relative` | 0.0199 | 0.0199 |  | pass |
| `Skia/Shapes/Path/Path_100px_Triangle_Centered` | 0.0011 | 0.0011 |  | pass |
| `Skia/Shapes/Path/Path_Expander_With_Border` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Path/Path_Tick_Scaled` | 0.0130 | 0.0130 |  | pass |
| `Skia/Shapes/Path/Path_Tick_Scaled_Stroke_8px` | 0.0055 | 0.0055 |  | pass |
| `Skia/Shapes/Path/Path_With_PenLineCap` | 0.0037 | 0.0037 |  | pass |
| `Skia/Shapes/Path/Path_With_Rotated_Geometry` | 0.0004 | 0.0004 |  | pass |
| `Skia/Shapes/Path/VerticalLine_Absolute` | 0.0002 | 0.0002 |  | pass |
| `Skia/Shapes/Path/VerticalLine_Relative` | 0.0002 | 0.0002 |  | pass |
| `Skia/Shapes/Polygon/Polygon_1px_Stroke` | 0.0083 | 0.0083 |  | pass |
| `Skia/Shapes/Polygon/Polygon_FillRule_EvenOdd` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Polygon/Polygon_FillRule_NonZero` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Polygon/Polygon_NonUniformFill` | 0.0050 | 0.0050 |  | pass |
| `Skia/Shapes/Polyline/Polyline_10px_Stroke_PenLineJoin` | 0.0038 | 0.0038 |  | pass |
| `Skia/Shapes/Polyline/Polyline_1px_Stroke` | 0.0052 | 0.0052 |  | pass |
| `Skia/Shapes/Polyline/Polyline_FillRule_EvenOdd` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Polyline/Polyline_FillRule_NoFill` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Polyline/Polyline_FillRule_NonZero` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Rectangle/Rectangle_0px_Stroke` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Rectangle/Rectangle_1px_Stroke` | 0.0013 | 0.0013 |  | pass |
| `Skia/Shapes/Rectangle/Rectangle_2px_Stroke` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Rectangle/Rectangle_Stroke_Fill` | 0.0000 | 0.0000 |  | pass |
| `Skia/Shapes/Rectangle/Rectangle_Stroke_Fill_ClipToBounds` | 0.0000 | 0.0000 |  | pass |
