# 05. Bindings while a container is recycled

Status: the hypotheses are verified against the code, upstream and the counters of design 09 (see
"Hypotheses, verified"). None of the three steps of the design is implemented as written: step 2 is
a path upstream does not have, step 3 is what the port already does, step 4 waited for a cache that
design 04 dropped. Two changes of the port's machinery that the reading of the code turned up are
implemented (see "What is implemented"): a box that carried nothing, and a question asked before
upstream asks it. Both are small. A difference from upstream was found and is recorded, not changed
(see "A difference found"). Written without a build: the session that validates builds, runs the
suites and fills the "measured" column of "Counters".

## Finding

- **[M]** Bindings account for 96 ms of the scrolling profile (17 %): template bindings 59 ms,
  dynamic resource bindings 21 ms, value bindings the rest.
- **[M]** Clearing a cell (`TableViewCell::set_column(None)`) costs 48 ms in total; 35 ms of it are
  template bindings publishing the cleared content into the template of the cell, which removes
  the text block of its content presenter.
- **[M]** Setting the column again costs 19 ms, 14 ms of it in `bind_binding`.

## Cause

A cell binds its content when its column is set and clears the binding when the column is cleared;
the content flows through a template binding into the content presenter, which drops its child for
empty content and builds a new one for the next content. Upstream does exactly this, and the
sequence stays.

**[H]** What can cost less is the machinery of each step:

1. A template binding publishes through the general binding expression path
   (`UntypedBindingExpressionBase::publish_value`, a sink, a property set with a priority), with
   boxed values, although source and target are properties of two known objects.
2. `bind_binding` builds a binding expression from a binding description every time: parsing or
   cloning the path, creating the observer nodes, subscribing. The description is the same for
   every cell of a column.
3. A dynamic resource binding repeats its lookup on every attachment (see design 04 for the lookup
   itself).

## What must not change

The values published, their order and priority, when a binding is instantiated and when it is
disposed, and the error and fallback behaviour of bindings.

## Design

1. **Measure**: per recycled row, bindings created and disposed, values published, and the time in
   expression construction against the time in publishing.
2. **Template bindings on the typed path.** When source and target property have the same value
   type and the binding has no converter, copy the value through the typed property route instead
   of the boxed one. The observable behaviour is the same value at the same priority.
3. **Cheaper instantiation of a compiled binding.** Keep the immutable part of a binding (the
   parsed path and its accessors) shared behind a reference count and build only the per-instance
   state on `bind_binding`. **[H]** Check how upstream shares the compiled path between instances
   and follow it.
4. **Dynamic resource bindings** take the cached lookup of design 04 when it lands.

## Hypotheses, verified

Against `TemplateBindingExpression`, `UntypedBindingExpressionBase`, `TypedBindingExpression`,
`BindingExpression` and its nodes, `CompiledBinding::create_instance`, `CompiledBindingPath`,
`FerroObject::bind_binding_with_anchor`, `DynamicResourceExpression` (read only), `TableViewCell`,
`TableViewCellsPresenter`, `TableView`, `VirtualizingStackPanel::recycle_element` and
`ContentPresenter::update_child_with` of the port; `TemplateBinding.cs`,
`TemplateBindingExpression.cs`, `Core/UntypedBindingExpressionBase.cs`,
`Core/TypedBindingExpression.cs`, `Core/BindingExpression.cs`, `Core/ExpressionNodes/DataContextNode.cs`,
`CompiledBinding.cs`, `DynamicResourceExpression.cs`, `TableViewCell.cs`, `VirtualizingStackPanel.cs`
and `ContentPresenter.cs` of upstream; and the counters per recycled row of design 09 (20 pixels per
step).

**Step 1, the figures. [M]** Per recycled row: 5.00 bindings instanced, 7.00 binding expressions
created, 0.00 template binding expressions created, 1.00 dynamic resource expression created, 86.42
binding values published. **[E]** The 5 bindings are the content bindings of the five cells
(`TableViewCell::set_properties`, the one `bind_binding` of a cell, as `TableViewCell.SetProperties`
has one `Bind`). The 7 expressions are those 5, the dynamic resource expression (its base is counted
with the others) and one more that is not instanced by `bind_binding`: by the page it is the binding
of the cell theme of the GDP column (`Background="{Binding GDP, Converter=...}"`), instanced with the
theme when the cell gets it back (2.00 style instances created per row). Bindings disposed are not
counted (design 09, "what is not counted yet"); by the code every expression created for a row is
disposed when the row is cleared the next time.

**What the figures say about the cause as written.** No template binding expression is created per
row: the templates of the row and of its cells are built once and stay. The 86 publishes are
therefore mostly not the 7 new expressions but the template bindings that already exist, each of
which publishes again whenever the property it reads changes on its templated parent: the content
and the alignment a cell clears and sets, and the inherited values a cell loses and gets back when
its row leaves the panel and returns (84 inherited values differ per row, design 03). So the cost
named in the finding (35 ms of 48 ms in template bindings while a cell is cleared) is publishing,
not construction, and the 14 ms in `bind_binding` are 5 expressions per row.

**Hypothesis 1, the template binding and its path: what it describes is upstream; step 2 is
dropped.** `TemplateBindingExpression` derives from `UntypedBindingExpressionBase` in upstream as in
the port. Its `PublishValue` reads `templatedParent.GetValue(_property)` as an object, converts it
with the default target type converter, and calls `PublishValue(value, error)` of the base, which
compares with the last value, stores it and calls `_sink.OnChanged`; the value store then reads the
value back from the expression as an object. The port does the same statements in the same order
(`publish`, `convert_to_target_type`, `publish_value`, `SinkRef::on_changed`,
`ValueStore::on_expression_changed`). A typed route for template bindings is a second path beside
that one, with its own rule for when it applies; upstream has a typed expression for compiled
bindings (`TypedBindingExpression`) and none for template bindings. That is different logic, not a
different way to carry out the same logic, and it is not implemented. What remains true of the
hypothesis is in "Left for a decision", point 1.

**Hypothesis 2, the instantiation: the port already shares what upstream shares; step 3 has nothing
left to do.** `CompiledBinding.CreateInstance` builds, for every instance, a new list of nodes
(`Path.BuildExpression(nodes, out isRooted)`: a node per path element, and for a property element a
new `PropertyInfoAccessorPlugin` as well), a data context node, the expression and, for a binding
with a converter or a string format, its uncommon fields, in which the string format `N0` becomes
`{0:N0}`. What is shared between instances is the path (its elements with their property
descriptions and accessor factories). The port does the same: `CompiledBindingPath` is a counted
slice of elements that `build_expression` walks; nothing is parsed and no element is copied per
instance; a path of one typed property becomes a `TypedBindingExpression` without any node, as in
upstream. Per instance the port allocates a few things upstream does not; they are listed in "Left
for a profile" and none is on a scale that explains 14 ms.

**Hypothesis 3, the dynamic resource: upstream looks the resource up on every start; step 4 is
dropped.** `DynamicResourceExpression.StartCore` finds the host, subscribes and calls `PublishValue`,
which calls `FindResource`; the port's `on_start` and `publish_value` do the same. The cached lookup
of design 04 was not implemented (upstream has no such cache), so there is nothing to take. Per
recycled row there is one such expression, against 43 resource lookups in all (design 04).

**The recycled row itself: upstream's statements.** `TableViewCell::set_column`, `clear_properties`
and `set_properties` are `Column.set`, `ClearProperties` and `SetProperties` line by line (five
values cleared, three set, the template set and the content bound). `recycle_element` clears the
container, pushes it to the pool, hides it and removes it, as `RecycleElement` does.
`update_child_with` creates the child, removes the old one if it is another, sets the data context
and adds the new one, as `UpdateChild` does.

## What is implemented

Two changes, one commit each, and one commit of tests.

### The data context nodes read the data context without boxing it

- File: `src/FerroUI.Base/data/core/expression_nodes/data_context_node.rs`: `data_context_value_of`,
  used by `DataContextNode::on_source_changed` and its change handler and by
  `ParentDataContextNode::set_parent` and its change handler.
- Machinery: the nodes read the data context (`Option<BoxedValue>`), put it in a new box and passed
  the box to `ValueTypes::normalize` to get the value of the node. The normal form of a boxed
  `Option<BoxedValue>` is, by the first lines of `normalize`, null for `None` and the normal form of
  the contents otherwise. The nodes now take that directly:
  `data_context_of(object).and_then(ValueTypes::normalize)`. One allocation fewer per read.
- Upstream statements preserved: `SetValue(ao.GetValue(StyledElement.DataContextProperty))` in
  `OnSourceChanged`, and the `SetValue` of the change handler. Upstream has no box here at all: the
  data context is an object reference.
- Why behaviour is identical: the value given to `NodeState::set_value` is the same value in both
  forms, for every data context: null for none, the contents for a nullable value that has one, null
  for a nullable value that is null, the value itself otherwise. Nothing else is touched: the
  subscription, the validation of the source and the error for a source that provides no data
  context are as before.

### A conversion asks whom to log against after the converter has failed

- Files: `src/FerroUI.Base/data/core/untyped_binding_expression_base.rs`
  (`UntypedBindingExpressionBase::convert`, `convert_back`), and their four callers in
  `core/binding_expression.rs` (`convert_and_publish_value`, `try_convert_back`) and
  `template_binding_expression.rs` (`publish`, `write_value_to_source`).
- The porting defect: upstream's `Convert` and `ConvertBack` call the converter in a `try` and call
  `ShouldLogError(out var target)` in the `catch`. The port evaluated `should_log_error()` at the
  call, as an argument, so it ran before the converter and on every conversion: an upgrade of the
  target and, for a `BindingExpression`, a read of the value of the source node, for an answer that
  is used only when the converter fails.
- Machinery: `convert` and `convert_back` take the question (`&dyn Fn() -> Option<Ref<FerroObject>>`)
  instead of its answer and ask it where upstream does: after the message of the failure is built.
- Upstream statements preserved: `converter.Convert(...)`; in the handler of its exception the
  message, `if (ShouldLogError(out var target)) Log(target, ...)`, the error, and the unset value.
- Why behaviour is identical: `should_log_error` reads and changes nothing, so for a converter that
  does not itself change the target or the source of the binding the answer is the same before and
  after. Where a converter does, the answer is now the one upstream gets. The value, the error and
  the text of the log are untouched.
- The signature of two public functions of `ferroui-base` changes. They have no callers outside
  `src/FerroUI.Base/data` (the markup crates do not call them).

## A difference found

Upstream's `DataContextNode.OnPropertyChanged` is
`if (sender == Source && e.Property == StyledElement.DataContextProperty) SetValue(e.NewValue);`:
the node takes the value of the notification. The port's handler reads the data context of the
source object again (`data_context_of`). The two are the same value unless the data context changes
again before the handler runs (a handler of the same notification registered earlier that sets
another data context): upstream then sets the value of each notification in turn, the port sets the
latest value each time. `ParentDataContextNode` has the same form.

This was there before this work and is not changed by it: taking the value of the notification is a
change of what the node publishes in that case, and although it is a change towards upstream it is
not one this design can test without a build. It is recorded in `DEVIATIONS.md` (Bindings) with a
comment at the site, for a decision.

## What is not implemented, and why

- **Step 2**: a typed path for template bindings is a path upstream does not have; see above.
- **Step 3**: the port shares the path as upstream does.
- **Step 4**: the cache it refers to does not exist.
- **Fewer publishes** (not publishing the cleared content, or not publishing inherited values that
  come back unchanged) would remove most of the 96 ms, and is exactly what "What must not change"
  rules out: every one of the 86 publishes is a publish upstream makes.

### Left for a decision

1. **The boxes of a template binding publish.** `TemplateBindingExpression::publish` reads the
   property as a box of its exact type (`get_value_untyped`), normalises it (`get_property_value`: a
   nullable value becomes null or a new box of its contents) and converts it to the type of the
   target (`convert_to_target_type`: for a nullable target a third box, of the nullable type again;
   for an untyped target a box of `Option<BoxedValue>`). For a property and a target of the same
   nullable type that is three boxes where the first already holds the value to publish; upstream
   boxes nothing for a reference and once for a value type. Publishing the first box when the two
   types are the same would be machinery only if normalising and converting back is the identity for
   every type, and that depends on what is registered as nullable and with which conversions
   (`ValueTypes`), and on how null is represented for the type (no box for a reference type, a box
   of the null value otherwise), which the comparison with the last published value sees. That could
   not be established by reading, so it is not done. It is the remaining substance of hypothesis 1:
   with 86 publishes per row it is the largest count on this path.

### Left for a profile

Not changed; each is an allocation per expression instance that upstream does not make, at most a
handful per row, and nothing measured points at them yet.

- `PropertyAccessorNode::create` copies the name of the property (`Box<str>`) for every node;
  upstream's node holds the string of the path element. Sharing it needs the name as a counted
  string in the property descriptions and another constructor of the node.
- `CompiledBinding::string_format` returns a copy of the string for every instance (the form of
  every property of a binding class); upstream passes the reference. The `{0:N0}` built from it is
  upstream's.
- The source of an expression is a box of the handle of the target
  (`select_data_context_source`), and `WeakValue::upgrade` makes that box again from the weak handle
  whenever a node asks for its source (`NodeState::source`, in every `set_source`).
- `InpcPropertyAccessor::for_notifying` allocates the closure that views the owner as a notifier
  for every accessor; `PropertyInfoAccessorFactory::create_inpc_property_accessor` could make it
  once. Not on the path of the catalog: its items raise no property change notifications, so their
  accessors are the plain ones.
- `TemplateBindingExpression::on_target_property_changed` asks for the templated parent property on
  every change of any property of the target, once per template binding of the target.

## Expected gain

Small, and to be measured. The 4 to 8 % **[E]** of the design came from steps 2 and 3, which are not
implemented. **[E]** The first change takes away one allocation per read of a data context by a
binding with a data context node: per recycled row 4, one at the start of each of the three content
bindings with a string format and of the binding of the cell theme (the two other content bindings
are typed expressions and have no node). The data context of a row does not change while these are
attached: `ItemsControl::clear_container_for_item_override` leaves it, as upstream's does, the cells
are cleared with the row, and the next item is set before the cells are bound again
(`TableView::prepare_container_for_item_override`). The second change takes away no allocation: per
conversion one upgrade of the target and one read of a node value, once per row on this page (the
one binding with a converter).

## Risks

Low. The first change replaces an expression by its value. The second changes a signature; a caller
that was missed does not compile.

## Verification

- The binding suites of `ferroui-base` (`src/FerroUI.Base/tests/binding_*.rs`,
  `compiled_binding_tests_create.rs`, `ferro_object_tests_binding*.rs`) and of the markup crates
  (among them `template_binding_tests.rs`, `binding_tests_templated_parent.rs` and
  `binding_tests_logging.rs`, which asserts the log of a converter that fails), the control template
  tests, the table view tests.
- New tests, in `src/FerroUI.Base/tests/binding_typed_expression_tests.rs`. The first three hold
  before the changes as well; the fourth is written against the new signature.
  - `recycled_target_takes_the_same_values_from_a_typed_and_an_untyped_binding`: the "recorded
    sequence of published values" the design asked for, at the level of the target: the values a
    target takes when its item leaves, its binding is cleared, the next item arrives and a binding
    of the same description is set, and the subscriptions of both items; the same for the typed and
    for the untyped expression.
  - `binding_to_the_data_context_itself_follows_it_through_null_and_nullable_values`: the value of a
    data context node for no data context, a value, a nullable value with contents and a nullable
    value that is null.
  - `template_binding_publishes_what_the_parent_has_when_the_target_gets_it_back`: a template
    binding whose target leaves its templated parent while the property of the parent changes.
  - `a_conversion_asks_whom_to_log_against_only_after_the_converter_has_failed`: the order of
    `convert` and `convert_back`, with a converter that succeeds and one that fails.
- `scroll-profile.mjs`.

## Counters: before, expected after

Per recycled row, 20 pixels per step. "Before" is the table of design 09, with the allocations as
they are after designs 03 and 04. No counter of what the framework does is expected to move; one
that does is a change of logic and a defect of this work.

| Counter | Before | Expected after | Measured after |
|---|---:|---|---|
| bindings instanced | 5.00 | 5.00 | to be measured |
| binding expressions created | 7.00 | 7.00 | to be measured |
| template binding expressions created | 0.00 | 0.00 | to be measured |
| dynamic resource expressions created | 1.00 | 1.00 | to be measured |
| binding values published | 86.42 | 86.42 | to be measured |
| resource lookups | 43.00 | 43.00 | to be measured |
| property changes raised | 459.24 | 459.24 | to be measured |
| content presenter children replaced | 10.00 | 10.00 | to be measured |
| containers recycled, reused | 1.00, 1.00 | 1.00, 1.00 | to be measured |
| allocations | 1175.0 | lower by 4 **[E]** | to be measured |
| bytes allocated | 104865 | lower by about 130 **[E]** (four boxes of an `Option<BoxedValue>` with their two counts) | to be measured |
| layout microseconds (`release`) | 411.3 (before designs 03 and 04; not measured since) | not expected to be distinguishable | to be measured |

### Commands

The suites:

```sh
cargo test -p ferroui-base --lib
cargo test -p ferroui-base --lib binding_typed_expression_tests
cargo test -p ferroui-base --features perf-counters --lib perf_counters
cargo test -p ferroui-markup --lib
cargo test -p ferroui-markup-xaml --lib
cargo test -p ferroui-controls --lib
cargo test -p control-catalog --lib
```

The figures, with the commands of design 09 (the counters, then the allocations, then the times
without a feature), on the commit of each change and on its parent:

```sh
cargo test -p control-catalog --release --features perf-counters --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --features count-allocations --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --release --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --profile dist --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
```

## Doubts

Most likely first.

1. Nothing was built or run. The new tests were written against the signatures in the source; the
   sequences they assert were derived by reading (for the value a target takes when its binding
   loses its value: `ValueStore::on_expression_changed` and `EffectiveValue::get_entry_value`, which
   give the default value for a typed expression without a value and for an untyped one that
   published the unset value). A test that fails is more likely a mistake of that reading than an
   effect of the two changes: three of the four hold on the parent commit as well, which shows it.
2. The estimate of 4 allocations depends on the seventh expression being the binding of the cell
   theme and on the two content bindings without a string format being typed expressions (the path
   the markup builds for `Name` and `Region` of an item that raises no notifications); neither was
   traced through the style instance and the markup crates.
3. The gain is within the noise of the benchmark for the time; the allocations are counted
   exactly, so four fewer should show.
4. Whether the difference found (the value a data context node takes from a change) should be
   corrected: it needs a decision, not a measurement.
