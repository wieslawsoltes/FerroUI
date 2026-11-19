# Osprey samples on the C ABI

The hello world and the seven tasks of [7GUIs](https://eugenkiss.github.io/7guis/tasks) written
in [Osprey](https://www.ospreylang.dev/) over the C ABI of FerroUI (`ferroui-capi`,
docs/c-abi.md): the same tasks as the Rust samples in `samples/SevenGuis`, one Osprey file each,
with no generated bindings. They show what a language without pointers to locals, closures across
its FFI or C structs needs from the ABI, and that the raw ABI is enough for it (#950).

| File | Task | Status |
| --- | --- | --- |
| `hello_world.osp` | Hello, World!: a text and a button whose click changes its text | complete |
| `counter.osp` | Counter | complete |
| `temperature_converter.osp` | Temperature Converter | complete |
| `flight_booker.osp` | Flight Booker | complete |
| `timer.osp` | Timer | complete |
| `crud.osp` | CRUD | complete |
| `circle_drawer.osp` | Circle Drawer | complete |
| `cells.osp` | Cells | complete |

Each task does what its Rust sample does (the scenario of `samples/SevenGuis/Model/scenarios.rs`
is the reference). With `FERROUI_SAMPLE_SCRIPT=1` a program runs the steps of that scenario on
itself once its window is shown (clicks are raised with `ferro_raise`, typing sets `Text`) and
prints one `script: ... ok` line per check, or `FAILED`.

## Building and running

The samples need the Osprey compiler 0.29.0 (a release from
<https://github.com/Nimblesite/osprey/releases/tag/v0.29.0>; Osprey compiles through clang) and the
dynamic library of the ABI with the desktop platform, in the default target directory of the
workspace:

```sh
cargo build --release -p ferroui-capi
```

Compile a sample from the root of the repository: the directive `// @linkdir: target/release` of
each file is read relative to the current directory, not to the file.

```sh
mkdir -p target/osprey
osprey samples/CApi/osprey/counter.osp --compile --memory=arc -o target/osprey/counter
```

Run it with the library on the search path of the loader (`DYLD_LIBRARY_PATH` on macOS,
`LD_LIBRARY_PATH` on Linux):

```sh
DYLD_LIBRARY_PATH=target/release target/osprey/counter
FERROUI_SAMPLE_SCRIPT=1 FERROUI_SMOKE_EXIT_MS=3000 DYLD_LIBRARY_PATH=target/release target/osprey/counter
```

`FERROUI_SMOKE_EXIT_MS=<n>` closes the windows after `n` milliseconds, as for every sample. The
samples run on macOS; Linux needs only the other variable of the loader. Continuous integration
does not build them yet: they are run locally against a build of the library.

## How the programs are written

- **Declarations.** Each file declares the functions of `ferroui.h` it calls with `extern fn`:
  handles and IDs are `int` (`int64_t`), statuses `int32`, strings `string` (NUL-terminated, so
  the length argument is `0 - 1`), and a C `bool` is declared `int8` (`toInt8(1)`, `toInt8(0)`).
- **Callbacks** are named top-level functions written at the call of `ferro_subscribe`,
  `ferro_post`, `ferro_timer_start` or `ferro_app_run`: Osprey passes a function to C only when
  it is known statically (no closure, no function parameter). The integer token tells the
  handler which control it serves (the cell of Cells, the field of the converter).
- **State.** Osprey assigns a `mut` cell only in the arm of an effect handler. The state of a
  task is a few file-scope cells (handles in a record, lists, maps, numbers), and each has a
  setter that installs a handler for one operation and performs it. Some tasks keep their state
  in FerroUI instead and read it back (the flight booker reads the kind and the dates from its
  controls).
- **Arithmetic.** Integer arithmetic may overflow, which is the `Arith` effect: every callback
  installs a handler for it (`handle Arith { ... }`), since a callback entered from C has no
  enclosing one.
- **Strings from FerroUI.** `ferro_get_cstr` returns text the library owns until its next call:
  the programs copy it at once (`ferro_get_cstr(...) + ""`).
- **Values back.** `ferro_get_f64_or`, `ferro_get_i64_or` and `ferro_get_bool_or` return the
  value of a property; the getters with an out-parameter need a pointer to a local, which Osprey
  cannot make.
- **Events.** Text fields use `TextChanging`, raised as soon as the text is set (`TextChanged`
  follows later, from the dispatcher). The circle drawer reads the position and the button of a
  pointer event in its handler (`ferro_event_x`, `ferro_event_y`, `ferro_event_button`), the
  cells the key of a key event (`ferro_event_key`).
- **Environment.** `getenv` of the C library is declared to return `int`: the address of the
  value, `0` when the variable is not set, which is all `FERROUI_SAMPLE_SCRIPT` needs.

## What each task uses

- **Counter:** a cell with the count; `Click`.
- **Temperature Converter:** `TextChanging` of both fields, the text read back, and a cell that
  keeps a field the other one set from being converted back.
- **Flight Booker:** a combo box of `ComboBoxItem`s, `SelectionChanged`, `SelectedIndex` read with
  `ferro_get_i64_or`, the background of an invalid date set from text (`#FFC8C8`) and cleared
  with `ferro_clear`, `IsEnabled` of the return date and of the button.
- **Timer:** `ferro_timer_start` every 50 ms with the elapsed microseconds, the duration read
  from the slider with `ferro_get_f64_or`, a `ProgressBar`, `ValueChanged`.
- **CRUD:** the people in an Osprey list, the items of the list box added and removed
  (`ferro_add_child`, `ferro_remove_child`, `ferro_release`), the selection read back,
  `Grid.Row` and `Grid.Column` as integers.
- **Circle Drawer:** `Ellipse` objects on a `Canvas` placed with `Canvas.Left` and `Canvas.Top`,
  `PointerPressed` and `PointerMoved` with the position and the button of the event, a second
  window with a slider whose `WindowClosed` keeps the adjusted diameter, and undo and redo over
  two Osprey lists of states.
- **Cells:** 2600 cells (a `Border` and a `TextBlock` each) in a `Grid` in a `ScrollViewer`, a
  parser and evaluator of the formulas in Osprey, `DoubleTapped` to edit a cell in a `TextBox`
  set as the child of its border (`ferro_set_handle` of `Child`), `KeyDown` with
  `ferro_event_key`, `LostFocus`, and `ferro_focus` posted with `ferro_post`.

The scripted runs cover the steps of the scenarios. Pointer input, the double click, keys and
the focus are what a user does in the window; the Rust tests of the ABI
(`src/FerroUI.CApi/tests/osprey_tests.rs`) raise a pointer press and a key press with their
arguments to cover the functions that read them.
