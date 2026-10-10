# VirtualizationDemo: gaps

What the port of the sample found in the framework.

```sh
cargo test -p virtualization-demo --lib
```

Status: 5 documents (`App.xaml`, `MainWindow.xaml`, `Views/ChatPageView.xaml`, `Views/ExpanderPageView.xaml`, `Views/PlaygroundPageView.xaml`), all converted without an edit, all compiled by the build, all load with the run-time loader too and show their class; 3 pages, all selected, laid out, scrolled and rendered through the compositor with Skia by `tests/shell.rs`. `Assets/chat.json` is the upstream file, unchanged.

## Gaps of the framework

None is open, and the sample needed no fix of its own. The list boxes of the three pages virtualize, recycle their containers and follow their view models (`tests/shell.rs`). Two fixes the BindingDemo sample needed are in the path of this sample too (`samples/BindingDemo/GAPS.md`): the identity of a list a binding delivers (B009) and the change notifications of a typed list (B010).

## Bindings that report an error

`tests/shell.rs` (`the_bindings_of_the_window_report_the_accepted_errors`) visits every page and compares the errors the bindings report with its list (`ACCEPTED`); the desktop host prints the same three reports when it starts. The managed original makes each of them: the template of the hamburger menu binds members of the selected item of the menu (`SelectedItem.(ScrollViewer.HorizontalScrollBarVisibility)`, `SelectedItem.(ScrollViewer.VerticalScrollBarVisibility)`, `SelectedItem.(TabItem.Header)`), the menu has no selected item while it sorts its pages when it is loaded, and a null in the middle of a path is an error of a binding. The pages of the sample report nothing.

## Differences of the port that are not gaps

- **The chat file is an embedded asset** (was V001). `ChatFile.Load(Path.Combine("Assets", "chat.json"))` opens a file of the directory the program runs in, where the build of the managed sample copies it (`CopyToOutputDirectory`). A build of a sample here copies nothing next to the program, and a program started with `cargo run` runs in the directory it is started from: the build of the crate embeds the files below `Assets/` (`build.rs`), and `ChatFile::load` takes the path as the path of the embedded asset.
- **The JSON reader** (was V002). `JsonSerializer.Deserialize<ChatFile>(s, options)` with `PropertyNameCaseInsensitive = true` is the serializer of the runtime library; the framework has no JSON reader (neither has the upstream framework), and no crate the sample may depend on reads JSON. `Models/chat.rs` has the small reader the file needs (objects, arrays, texts with their escapes, numbers, `true`, `false`, `null`) and builds the two types by hand: the names in any case, a text for a text, and the time of a message through the parser of `DateTimeOffset` of the framework (a time without an offset is a local time, as the serializer reads it).
- **Typed lists** (was V003). The item templates of the chat and of the expanders state no data type: the compiler takes it from the items of the list box. Each observable collection of the sample is the notifying list of its items with a declaration of that instantiation (`ferro_markup_list!`: `ChatMessageList`, `ExpanderItemList`, `PlaygroundItemList`), from which the compiler learns the element type.
- **Named elements** (was V004). The handler of the timer of the playground reads two named elements through the fields the build of the managed sample generates (`list`, `itemCount`); here it finds them by their names in the name scope of the view (`get_control`), at every tick.
- **Two constructors of one parameter** (was V005): `PlaygroundItemViewModel(int index)` and `PlaygroundItemViewModel(string? header)` are both declared in the markup metadata of the class; the build accepts them.
- The handler of the timer of the playground holds its view weakly where the managed original subscribes a method of the view to a timer the view owns: a cycle a collector frees, and here a leak.
- `Random` of the playground is the subtractive generator of the runtime library, unseeded.

## What the tests do not reach

- The timer of the playground (two ticks a second) runs on the clock of the dispatcher, which a test does not advance: the test finds the scheduled timer and fires it once.
- The flyout of the selection modes of the playground (five check boxes, two of them bound to properties of the list box by its name) is not opened: its bindings are created when it opens.
- The hamburger menu sorts its pages by their headers when it is loaded, as upstream: the pages are in the order Chat, Expanders, Playground, and the tests find a page by its header.
- The developer tools of a debug build have no counterpart (`program.rs`).
