# Bindings of FerroUI

FerroUI is a Rust framework with one C ABI (`ferroui-capi`, docs/c-abi.md). The bindings of
other languages are generated in two stages, so that every language is generated from the
same facts and a new language is one backend:

1. **The API model** (`bindings/model/ferroui-api.json`), language-neutral, built once from the
   metadata of the framework by `scripts/generate_api_model.py`. The script also writes the
   ID table that is compiled into the library (`src/FerroUI.CApi/api_ids.rs`).
2. **Backends**, one per language, that read only the model and write the files of their
   language (`bindings/<language>/`). Osprey is the first (`scripts/generate_osprey_bindings.py`,
   `bindings/osprey/`, docs/osprey.md).

```sh
python3 scripts/generate_api_model.py          # the model and the ID table of the library
python3 scripts/generate_osprey_bindings.py    # the Osprey bindings from the model
python3 scripts/test_generate_bindings.py      # the tests of both
```

Both scripts take `--check`, which CI runs: a change of the framework (a control, a property,
an event, the ABI) that is not generated again fails the build. The design that the model and
the ABI are shaped by (performance and idiom in each language) is docs/bindings-design.md.

## The model

The model is one JSON document (format 1):

| Key | What it holds |
| --- | --- |
| `format` | The version of the format; a change that breaks a backend raises it. |
| `abi_version` | `FERRO_ABI_VERSION` of the header the model was read from. |
| `fingerprint` | A signed 64-bit hash of the ID tables (`ferro_api_fingerprint()` returns the same value in a library built from this model). |
| `abi` | The functions of the C ABI read from `include/ferroui.h`: name, parameters with an abstract type, result, documentation; the callback types; the status codes. |
| `frame` | The byte layout of a frame (section "Frames" of docs/bindings-design.md): the record header and each record with its operation code, fields and the scalar writer function that writes it. |
| `classes` | Each class: stable `id`, `name`, `words`, `base`, `rust_path`, `creatable`, `control`, where it holds `children` (`children`, `items`, `content`, `child`, `none`), its own `properties` and `events` by name, `doc`. |
| `properties` | Each property name the ABI resolves: stable `id`, `name` (`Owner.Name` for an attached property), `words`, and its `declarations`: the owner, the kind (`styled`, `direct`, `attached`), the `value` (`kind`, the Rust type, `nullable`, the enumeration or class), `default` (the Rust expression of the registration, informative), `read_only`, `doc`. |
| `events` | Each routed event name: stable `id`, `name`, `words`, `declarations` (owner, argument type, whether `ferro_raise` can raise it, `doc`), the `value_property` a frame reads as its value (`Text` for `TextChanged`), and the dispatch styles it supports. |
| `enums` | Each enumeration a property uses: `name`, `words`, `rust_path`, `members` (`name`, `words`, `value`, `doc`), `doc`. |

**Names are words.** `FontSize` is stored with `["font", "size"]`, `IsHitTestVisible` with
`["is", "hit", "test", "visible"]`. A backend applies the casing of its language
(`fontSize`, `FontSize`, `font_size`), its reserved words, and the exceptions of its override
file (`bindings/<language>/overrides.json`). The framework names themselves (`name`) are what
the ABI resolves, so a backend never invents a name the library does not know.

**Value kinds.** `bool`, `number` (floating point), `integer`, `string`, `enum` (written by
member name or value), `content` (text or an object), `object` (a handle of a class),
`parsed` (a value the ABI parses from text as markup does: a thickness `"4,2"`, a brush
`"Red"`), and `unsupported` (callbacks, collections, templates: not reachable through scalars
yet). A property name can have declarations of different kinds on different classes
(`Height` is a number on `Layoutable` and a grid length on `RowDefinition`): the backend
decides how to expose both.

**Stable IDs.** Types, properties and events are numbered in the order of the model
(sorted names, starting at 1). The library interns the names of the model in that order when
it starts (`api_ids.rs`), so the ID of a name is the constant of the model and generated code
calls `ferro_set_f64` with the ID of `FontSize` as a literal, with no lookup. A binding compares
`ferro_api_fingerprint()` with the fingerprint it was generated with at start. Adding a
property renumbers the IDs after it and changes the fingerprint; both the library and the
bindings are generated in the same build, and a binding against another library refuses to
start. Name lookup (`ferro_property_id("FontSize", -1)`) keeps working for names the model
does not have.

**Events, both styles.** The ABI calls a C function with a token (`ferro_subscribe`) or with a
`void *` and a release function (`ferro_subscribe_data`); frames bind an event to a message
tag that goes to one dispatcher (`ferro_frame_on`, `ferro_frame_dispatcher`). The model lists
`token`, `user_data` and `frame_tag` for every event: a language with closures generates
handlers over `user_data` (one trampoline, a handler table on its side), a language without
them dispatches tags.

## Adding a backend

1. Write `scripts/generate_<language>_bindings.py` (standard library only, like the others): it
   reads `bindings/model/ferroui-api.json` and writes `bindings/<language>/`, with a header
   comment saying the files are generated, and takes `--check`.
2. Map the abstract types of `abi.functions` to the fastest FFI path of the language (see
   docs/bindings-design.md, section 2), and leave out what the language cannot pass, saying so
   in the generated file.
3. Name with `words` and an override file of the language; escape its reserved words.
4. Call the ABI with the IDs of the model as constants and check the fingerprint at start.
5. Add the `--check` to the step "Generated API model and bindings are up to date" of
   `.github/workflows/ci.yml`, tests to `scripts/test_generate_bindings.py`, samples, and a
   benchmark of the same operations as `src/FerroUI.CApi/benches/call_overhead.rs`.
