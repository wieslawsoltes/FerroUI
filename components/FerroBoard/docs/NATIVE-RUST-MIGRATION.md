# Rust-native migration

The native model/accessor and error cleanup is described in
[NATIVE-MODEL-API.md](NATIVE-MODEL-API.md). The old init-only facade,
exception-name adapters, dead cursor state and required custom runtime-name
mapping are removed. Current execution evidence is recorded separately.

The popup, resource collection/dictionary, metadata and subscription migrations
are implemented on the selected FerroUI 0.5 preview main revision.
[Current API changes and source fixture projections](NATIVE-MAIN-20261102.md)
and [exact source/checks](native-upstream-main-20261102.json) describe their scope.

The native implementation uses:

- Vec, IndexMap, Rc/Arc and borrowing for storage/identity.
- Rust Iterator, Option and Result for extensible readers, queries and errors.
- Duration, SystemTime and the existing time crate for keyboard timing/history.
- Closures, native handler tokens, Drop and actual FerroUI Subscription guards.
- Core-owned UTF-16 buffers with checked String bridges at native controls.
- Platform locale services and ICU data through the selected framework boundary.
- Native font bytes/readers and FerroUI font collections.
- Metadata for real controls, domain values and compiled/runtime XAML consumers.

Preserve recognizable public controls and useful extension traits. Document API
migrations while retaining keyboard algorithms, resources, data formats, event
ordering, editing, gestures, prediction, privacy and focus behavior.

Do not add general CLR/BCL services, record generators, reflection, runtime hashes,
string interning, generic managed collections/cursors or delegate equivalence.
Reference C# and its fixtures remain unchanged development inputs.

FerroUI's actual Rx/markup contracts still contain some explicit handles. Their
small adapters are documented at the framework boundary. Core metadata event
scopes retain explicit release functions under the SDK's add-only event metadata
contract. This is a concrete integration limitation, not a request to emulate a
runtime.

Earlier migration JSON receipts and immutable integration archives preserve the
detailed history, intentional API changes and original validation scopes.
Use the current delivery receipt for archive/native execution. Full platform
and accessibility certification is reported separately; A-001 remains deferred.
