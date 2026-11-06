# Native model configuration and diagnostics

FerroBoard's remaining init-accessor facade and .NET exception-name translations
have been removed. This accompanies the FerroUI main `b7537398` upgrade; the exact
SDK, source hashes and executed checks are in [the receipt](native-main-b7537398.json).
This is an API migration, not a change to keyboard layout, editing or gesture rules.

The 47 `init_*` setter names now use `set_*`, including input attributes, layouts,
keys, selectors, popup mappings, currency sets and punctuation rules. There are no
obsolete aliases. Model metadata uses ordinary setters rather than `InitOnly` /
`IsExternalInit` semantics. Public XAML property names stay the same; required
layout fields retain FerroUI's actual compiler annotations.

| Previous Rust API | Native API |
|---|---|
| `InputAttributes::init_kind` | `InputAttributes::set_kind` |
| `KeyboardLayout::init_type` | `KeyboardLayout::set_type` |
| `TextKeyData::init_label` | `TextKeyData::set_label` |
| `CaseSelector::init_lower` | `CaseSelector::set_lower` |
| `PopupMapping::init_mappings` | `PopupMapping::set_mappings` |

These remain shared configuration models. Existing `Rc` / `Arc` allocations,
`RefCell` / `RwLock` ownership, clone snapshots, notifications and release order
remain intact. Rust callers can also construct their value structs directly.
This migration does not add automatic change notifications to plain properties.
The independent consumer constructs and binds input attributes in both compiled
and runtime XAML, then updates the same allocation through a Rust setter.

Metadata failures now report the native error's `Display` text through FerroUI's
string-valued `MarkupInvokeError::Failed` boundary. Core errors no longer format
.NET exception class names or exception parameter strings. Rust error variants
and their cause chains remain available to native callers. Error messages are
not persistence formats; JSON documents and resource bytes are unchanged.
C# exception labels remain in fixture adapters solely to compare the saved source
outcomes with corresponding native failures.

`ResourceTextListError::InvalidCurrent` is removed with the obsolete managed cursor
contract. The live mutation guard and injectable unsupported-operation errors
remain; their messages describe native resource operations, without fixed-array
or enumerator claims.

Custom text keys use `ITextKeyData::diagnostic_name`, defaulting to Rust's
`std::any::type_name::<Self>()`. The old optional `source_type_name` hook and its
`UnmappedRuntimeTypeName` failure are removed. Built-in diagnostic labels and
explicit overrides retain their existing output. A custom key no longer needs a
runtime type-name registration to format itself.

FerroUI's own interfaces, class macros, metadata and subscription adapters remain
at the control/XAML boundary. UTF-16 buffers still preserve keyboard offsets and
editing semantics. Neither requires a second CLR/BCL runtime in FerroBoard.
This cleanup does not establish complete platform or assistive-device verification.
A-001 remains user-deferred.
