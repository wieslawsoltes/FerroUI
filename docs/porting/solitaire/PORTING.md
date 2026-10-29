# Porting notes

Source: [AvaloniaUI/Solitaire](https://github.com/AvaloniaUI/Solitaire/tree/82fa813c35ac120931b1568ce24d1bb1cc68043c), commit `82fa813c35ac120931b1568ce24d1bb1cc68043c`.

Target: [FerroUI](https://github.com/wieslawsoltes/FerroUI), published `0.4.0-preview.1` crates. This version was verified using Cargo's registry, rather than assuming the version shown in the repository README is published.

The implementation follows the framework's [porting guide](https://github.com/wieslawsoltes/FerroUI/blob/main/docs/porting/PORTING-GUIDE.md) and [XAML guide](https://github.com/wieslawsoltes/FerroUI/blob/main/docs/porting/xaml.md): `Application` subclass and desktop lifetime, `Ref<T>` control handles, a `#[repr(C)]` custom card class using `ferro_class!`, native layout, XAML and routed pointer capture. There is no webview, C# runtime, emulated UI toolkit or flattened board bitmap.

## Source mapping

| Source | Rust port |
| --- | --- |
| `App.axaml.cs`, desktop `Program.cs` | `samples/Solitaire/src/app.rs`, `samples/Solitaire/src/main.rs` |
| `Models/CardSuit`, `CardType`, `CardColour`, `Difficulty`, `DrawMode` | `samples/Solitaire/src/models/playing_card.rs`; rank and suit replace the 52-value card-type enum |
| `ViewModels/CardGameViewModel.cs` | `samples/Solitaire/src/view_models/card_game_view_model.rs`; shared deck, state, commands, history and victory handling |
| `ViewModels/Pages/KlondikeSolitaireViewModel.cs` | `samples/Solitaire/src/view_models/pages/klondike_solitaire_view_model.rs` and shared state/deal operations |
| `ViewModels/Pages/FreeCellSolitaireViewModel.cs` | `samples/Solitaire/src/view_models/pages/free_cell_solitaire_view_model.rs` and shared state/deal operations |
| `ViewModels/Pages/SpiderSolitaireViewModel.cs` | `samples/Solitaire/src/view_models/pages/spider_solitaire_view_model.rs` and shared state/deal operations |
| `ViewModels/CasinoViewModel.cs` | `samples/Solitaire/src/view_models/casino_view_model.rs` |
| `ViewModels/Pages/SettingsViewModel.cs` | `SettingsState` and settings commands in casino model/view |
| `ViewModels/Pages/StatisticsViewModel.cs`, `GameStatisticsViewModel.cs` | `GameStatistics` and statistics page in casino model/view |
| `Models/PersistentState.cs`, runtime storage provider | `samples/Solitaire/src/models/persistent_state.rs` |
| `Views/CasinoView.axaml` | `samples/Solitaire/src/views/CasinoView.xaml`, `samples/Solitaire/src/views/casino_view.rs` |
| Title, settings and statistics XAML pages | corresponding `samples/Solitaire/src/views/pages/*.xaml` documents |
| Three solitaire XAML pages and their code-behind | shared `CardGameView.xaml`; game-specific placements in `CardStackPlacementControl` |
| `Controls/PlayingCard.cs`, `Styles/PlayingCard.axaml` | `samples/Solitaire/src/controls/playing_card.rs`; a FerroUI Border subclass containing a native Image |
| `Controls/CardStackPlacementControl.cs`, `OffsetMode.cs` | `samples/Solitaire/src/controls/card_stack_placement_control.rs`; native Canvas placements and compressed vertical stacking |
| `Behaviors/CardFieldBehavior.cs` | `samples/Solitaire/src/behaviors/card_field_behavior.rs`; routed pointer input with capture |
| `Styles/CardsStyle.axaml` | `samples/Solitaire/src/styles/CardsStyle.xaml`; original DrawingImages, changed framework namespace and branding resource key |
| `Styles/Button.axaml`, `TrayContainer.axaml` | `samples/Solitaire/src/styles/Button.xaml`, `samples/Solitaire/src/styles/Animations.xaml`, `samples/Solitaire/src/controls/tray_container.rs` and page tray Borders |
| `Assets/*.ttf` | original embedded fonts under `samples/Solitaire/src/assets/`, registered as a native font collection |

Small related source classes are consolidated, and identical game commands/pile state are shared rather than copying them three times. Rust value snapshots replace the original mutable card view-model references and collection notification batches. Page XAML retains native controls; explicit named-part command wiring replaces CommunityToolkit-generated C# bindings. Pages are loaded through FerroUI's supported run-time XAML loader, not compiled XAML in a build script.

## Game semantics

Retained: seven Klondike tableaus, fixed hearts/clubs/diamonds/spades foundations, draw one/three, stock recycling, alternating descending builds and kings in empty columns; eight FreeCell tableaus, four single-card cells, any rank in empty columns and the source's `free cells + 1` run capacity; ten Spider tableaus, 54 initial cards, five ten-card deals, same-suit movable runs, mixed-suit builds, complete king-to-ace collection, 500 initial points, minus one per move and plus 100 per completed run. Stock turns do not increment the move counter, matching the source. Klondike and FreeCell foundation moves score 10, waste-to-tableau scores 5, and Klondike foundation-to-tableau costs 15.

Intentional correctness fixes:

- A foundation receives only one exposed card, never the rest of a tableau run.
- Every pair in a FreeCell movable run must alternate colour as well as descend in rank.
- Changing Spider difficulty builds 104 cards with the selected suit distribution for the next deal.
- Undo restores complete board snapshots, including face states, empty-source moves, stock draws/recycling and automatic Spider collection.
- Win statistics are recorded once per deal, including across restart, undo and reload. Started deals count as played; best scores/times/moves are winning records. Time played accumulates only while playing.
- Save files are versioned, validated and atomically replaced. They include unfinished games and undo history as well as settings/statistics.

Additional conveniences: hints, replaying a deal, click-to-move, keyboard shortcuts and confirmation before replacing or restarting a deal.

## Visual and platform scope

The original vector card art and fonts are embedded unchanged. The green radial background, white title artwork, gold conic-gradient buttons, dark trays and source page navigation are retained. Placement and toolbar sizing are adapted to a single scalable field; statistics appear together on one page. Empty pile outlines use a subdued green border.

Card controls are retained by card ID for the lifetime of a game view, including covered stock/foundation cards and across new deals. Updates change native `Canvas.Left`, `Canvas.Top`, face artwork and `ZIndex`; they do not clear the Canvas or replace the card controls. Position transitions use the source's 350 ms `CubicEaseInOut`; depth uses its `1,0,1,0` spline. Dragging suspends transitions, and release restores them so valid drops and rejected drops animate from the pointer's final position. Hit testing reads the effective animated coordinates.

The presentation scheduler uses cancellable `DispatcherTimer` steps with the source's timing: Klondike/FreeCell wait 600 ms and deal every 75 ms; Spider waits 1000 ms, deals every 50 ms, reveals column tops and stocks the remaining cards every 50 ms. Klondike draw/recycle steps are 175 ms apart and automatic foundation moves are 75 ms apart. Spider's stock row starts moving together, matching the source's synchronous `DoDealCards`. A final 350 ms settling interval lets the last card finish moving.

`Animations.xaml` restores the 2.5-second quadratic startup fade on the persistent `Panel.mainContent`, 300 ms tray translation and repeating 60-second alternating gold-gradient animation with the original gradient stops. Replacing a navigation page or changing a setting does not restart the fade. Tray entry uses the source `downDeck` / `animateUp` states; the game command tray lowers when the victory dialog opens.

Rules and undo commit atomically before presentation begins. Partial visual deals are never saved. Mutating game input and the game clock pause while the presentation runs; Home remains available. Navigation and window close cancel the pending timer and invalidate its callbacks. Selecting a game from the title always starts a new animated deal, matching the source title commands, even if a saved snapshot for that game exists. Desktop game arguments use the same entry path. The UI thread never blocks to perform game animations.

The source's noise texture, static 3D card-back transform and controller/TV directional navigation remain unported. The source card template switches front/back visibility; it does not declare an animated flip angle, so this port also switches the face artwork when a card is revealed.

This deliverable includes the native desktop entry point for macOS, Windows and Linux. macOS was built and smoke-tested here. Windows and Linux are configured in CI but were not executed locally. The rules and persistence can be tested without the UI (`--no-default-features`). The source application's browser, Android, iOS, tvOS and visionOS hosts are not included; those need platform entry points and a storage provider appropriate to each target. The remaining host and visual differences above are explicit scope limits.

## Verification

- Rule/persistence tests verify deterministic deals, all Spider difficulties, moves, reveals, scoring, stock order, victory, conservation during sequences of legal moves, reverse undo and restart.
- The native headless UI test loads every XAML page with the real Skia backend and checks command wiring, settings, game replacement, dialogs, undo, saving, pointer capture, drag-and-drop and click-to-move through the Viewbox.
- Presentation tests verify the source delays, card conservation at every deal/draw step, final committed boards and undo semantics.
- The animation integration test drives FerroUI with a manual `IClock` and renders Skia frames before, during and after a move. It checks native interpolation of positions, startup opacity (including no restart on navigation), tray translation and gradients, card identity across move/undo/restart/new game, immediate dragging, animated return, navigation/close cancellation, and title-button deals for all three games with and without saved games.
- Rendered title, settings, three games and statistics are exported by that test when `FERRO_SOLITAIRE_CAPTURE_DIR` is set; representative captures are under `images/`.
- An unattended macOS desktop launch verifies that the native platform opens the application and the dispatcher closes it normally.
