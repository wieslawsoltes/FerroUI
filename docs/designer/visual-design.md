# Designer visual language

The native workspace uses a shared visual system for its shell, inspectors, assets,
layout controls and motion tools. The application being designed keeps its own theme.

## Reference study and decisions

The earlier workspace exposed useful features but gave nearly every command the same
presentation: a small generic button, a text field or another horizontal toolbar. The
property inspector resembled a database grid; repeated type names narrowed the outline;
timeline controls competed with the tracks. Reducing heights alone did not establish a
clear hierarchy.

The redesign applies the following patterns to FerroUI's native authoring model:

| Reference | Useful pattern | FerroUI implementation |
| --- | --- | --- |
| [Microsoft Blend workspace](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/creating-a-ui-by-using-blend-for-visual-studio?view=visualstudio) | Object organization and animation composition are closely connected; drawing tools and searchable assets have distinct places. | Stable layers dock, a compact canvas tool shelf, searchable asset categories, and target/property track headers aligned with the timeline ruler. |
| [Figma UI3 design process](https://www.figma.com/blog/our-approach-to-designing-ui3/) | Stable docked panels preserve working speed; controls need recognizable affordances and labels. | Resizable docks, one application bar, quiet mode tabs, vector icons with tooltips, inset fields and consistent focus outlines. |
| [Figma property-panel redesign](https://www.figma.com/blog/behind-our-redesign-ui3/) | Prioritize the current selection and group related layout controls; use field backgrounds and explicit dropdown affordances. | Named selection header, contextual panel editor, distinct property sections, visual alignment/flow controls, linked spacing fields and staged color editing. |
| [Figma layers and assets](https://help.figma.com/hc/en-us/articles/360039831974-Explore-the-navigation-bar-and-left-sidebar) | Semantic layer icons, nested hierarchy, collapse while retaining the selection, and searchable reusable assets. | One virtualized hierarchy column, complete type details in tooltips, native type-ahead, selection-preserving Collapse layers and category browsing. |
| [Penpot assets](https://help.penpot.app/user-guide/design-systems/assets/) | Grid browsing supports visual recognition; list browsing supports names and density. Search, filtering and drag-to-canvas are continuous workflows. | Functional Grid/List asset modes share the same catalog, search, categories, keyboard activation and native drag callbacks. |

These are interaction patterns adapted to a native XAML designer. Source, visual and
logical trees remain separate inspection modes. Animation editing still writes native
XAML through the transaction engine, and the property editor still respects expressions,
mixed values, custom factories and effective runtime values.

## Hierarchy and workspace

The application bar holds file/history commands, Design/Split/XAML modes, preview and
sharing. The left dock holds Layers, Assets and Project. The right dock switches between
Design, States and Behaviors; authoring panels use the available inspector height instead
of opening beneath an already crowded property grid.

The canvas has a subdued document tab strip and size/theme/zoom bar. Its tool shelf holds
Select, Interact, Arrange and Focus canvas. Select and Interact have `V` and `I` shortcuts
outside text entry. Focus canvas (`Ctrl+Shift+F`, or `Cmd+Shift+F`) temporarily hides the
docks and timeline. Restoring the workspace retains resized widths, the prior timeline
visibility, panel selections, pending source and scroll state.

Layers use kind icons and authored names. Unnamed types and metadata have quieter text.
Selection and native drag insertion feedback remain visible without spreadsheet headers
or vertical rules. Collapse layers retains the ancestor paths of every selected item.

## Tokens and control rules

`DesignerChrome.xaml` is the shared style source. `designer_chrome_style()` loads it for
an embeddable host. Mark only editor containers with `designer-chrome`. All named color
resources use the `Designer` prefix; no application-theme resource is replaced.

| Role | Resource | Value |
| --- | --- | --- |
| Canvas | `DesignerCanvasBrush` | `#17181C` |
| Application chrome | `DesignerChromeBrush` | `#202126` |
| Dock / section surface | `DesignerPanelBrush` | `#25262B` |
| Inset field | `DesignerFieldBrush` | `#303138` |
| Hover surface | `DesignerHoverBrush` | `#393B44` |
| Divider / outline | `DesignerBorderBrush` | `#3B3D46` |
| Primary text | `DesignerTextBrush` | `#F2F3F5` |
| Secondary text | `DesignerSecondaryBrush` | `#A9ADBA` |
| Supporting text | `DesignerMutedBrush` | `#979CAA` |
| Selection / keyboard focus | `DesignerAccentBrush` | `#79A8FF` |
| Selection surface | `DesignerSelectionBrush` | `#2D4061` |
| Motion authoring | `DesignerMotionBrush` | `#B6A2F6` |
| Recording | `DesignerRecordingBrush` | `#F48B95` |

Body text is 12 px; section labels and supporting text use 11 px. Inspector inputs are
26 px high, toolbar targets are usually 28–32 px, and layer rows are 28 px. Field corners
use 4 px; the canvas shelf uses 8 px. Eight- and twelve-pixel gutters establish consistent
spacing. Supporting text has a calculated contrast ratio of 5.50:1 on the panel surface
and 4.72:1 on the field surface. State meaning also appears in labels, geometry and
tooltips, rather than depending on color alone.

Inputs use an even outline, a restrained hover border and a full blue keyboard-focus
border. Reset appears for authored values. Alignment uses vector controls. Uniform
spacing can use one linked value; nonuniform and mixed values retain explicit components.
Solid colors use a swatch and native color view. Spectrum changes remain a local draft
until Apply, so a continuous color drag produces one authored transaction. Expressions
and richer brushes retain their source representation.

## Motion presentation

The animation dock has a single transport bar. Program choice and management live at the
left; playback, recording and key creation form a compact central group; the editable
clock and settings remain at the right. Target and property labels use separate lines.

The ruler and selected-key editor have fixed hosts, with only tracks scrolling between
them. A blue playhead identifies time; violet diamonds and authored-range rails identify
animation data. A sampled easing preview shows the actual engine curve, including
overshoot, and clears when a draft is invalid. It is a view of staged easing; source
changes still require Apply. Multi-key movement, duplication and deletion keep atomic
validation and undo behavior.

## Native evidence

The screenshots under [images](images/) come from the real workspace, using WinUI control
templates, HarfBuzz text shaping and the Vello CPU renderer. They are generated by the
`render-smoke` tests, including compact and full-size workspaces. Visual review is paired
with behavioral tests for focus restoration, selection, layer virtualization, native drag
contracts, staged color editing and timeline authoring.

See [native-controls.md](native-controls.md) for embedding and extension APIs.
