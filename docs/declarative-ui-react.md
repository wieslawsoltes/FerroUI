# The declarative API of FerroUI compared with React

This document is for developers who know React. It compares `ferroui-view`, the declarative Rust API of FerroUI ([declarative-ui.md](declarative-ui.md)), with React:
- what corresponds to what;
- where the two behave alike and where they differ;
- what React has that FerroUI does not have yet.

It describes FerroUI after phases 1 to 6 of issue #237 (components that run again, hooks, the reconciler; XAML and data templates of components; styles, themes, transitions, popups; hot reload, diagnostics, an inspector; the fewest moves and memo by default; futures, error boundaries and a router) and React 18 and 19.

## 1. Summary

`ferroui-view` has two models of components, and both produce the same elements:

- **Components that run again** (`#[component(rerender)]`, `rerender(..)`) are React's model. A function returns elements of its state. It runs again when its state changes, and a reconciler patches the controls it made into what it returns now. State lives in hooks with React's rules.
- **Components that run once** (`#[component]`) are the model of SolidJS and Leptos. The function runs once, and the properties of controls follow signals: a change updates the properties bound to it, with nothing to compare.

What sets FerroUI apart from React is where the elements end up. React DOM renders into the browser's DOM, styled by CSS and laid out by the browser. FerroUI renders into its own retained controls: a property system with styles and bindings, panels for layout, and themes (Fluent, Simple, WinUI 3), drawn by Skia or Vello on every desktop and mobile platform and in the browser. The nearest React relative is React Native, whose host components are native views. In FerroUI the host components are FerroUI controls, and there is no bridge.

## 2. The architecture side by side

| | React (React DOM) | FerroUI, runs again | FerroUI, runs once |
|---|---|---|---|
| Markup | JSX, compiled to `jsx()` calls | `rsx!`, expanded to builder calls at compile time; or the builders directly | the same |
| Element | a plain object: type, props, key | `Element`: class, key, property values, handlers, children | the same |
| Host elements | DOM nodes | FerroUI controls (`Button`, `StackPanel`, ...) | the same |
| Component | a function of props | a function of its parameters, run in an effect | a function of its parameters, run once |
| State | `useState`, `useReducer` | `use_signal` (a signal kept across runs) | `signal` |
| Update | `setState` schedules a render | a write to a signal the function read runs it again | a write updates the properties that follow the signal |
| Diffing | the reconciler compares the new tree with the fiber tree | the reconciler compares the new elements with the mounted instances | none |
| Scheduling | batched, can be interrupted (concurrent rendering) | synchronous, when the write ends or the batch does | synchronous |
| Layout | CSS (flexbox, grid) | panels: `StackPanel`, `Grid`, `DockPanel`, `Canvas`, ... | the same |
| Styling | CSS, `className`, `style` | styles and themes of the property system, style classes (`class: "accent"`) | the same |
| Types | TypeScript, optional | Rust: properties, parameters and handlers are typed and checked at compile time | the same |

## 3. Concepts, one by one

| React | FerroUI | Notes |
|---|---|---|
| `<Button onClick={f}>Add</Button>` | `rsx! { Button { on_click: f, "Add" } }` or `button().on_click(f).child("Add")` | An attribute is a setter of the builder. A misspelled or mistyped attribute is a compile error that points at it. |
| `{cond && <X/>}`, `cond ? <A/> : <B/>` | `if cond { X {} } else { Y {} }` in `rsx!` | |
| `{items.map(i => <Row key={i.id} .../>)}` | `for i in &items { Row { key: i.id, .. } }` in `rsx!` | |
| `<>...</>` (fragment) | several elements, a `Vec<Element>`, or a tuple; `Element::Fragment` | |
| function component | `#[component(rerender)] fn Name(..) -> Element` | The name is in upper camel case, as in React. The macro also makes a builder function, `name()`. |
| props | the parameters of the function | Each parameter is a setter. Defaults come from `#[prop(default = ..)]` or an `Option` type. A missing required one panics with its name. |
| `children` prop | a parameter `children: Element` | |
| `key` | `key: ..` on any element (numbers or text) | |
| `useState` | `use_signal(|| init)` | It returns a `Signal`, not a value and a setter: `count.get()`, `count.set(..)`, `count.update(|c| ..)`. A `Signal` is `Copy`, so closures take it without a clone. |
| `useReducer` | `use_signal` with `update(|state| reduce(state, action))` | |
| `useRef` | `use_hook(|| Rc::new(RefCell::new(..)))`; for a control, `use_hook(NodeRef::new)` with `node_ref: r` | |
| `useMemo(f, deps)` | `use_memo(|| ..)` | The memo follows the signals it reads, so it needs no dependency list. A value computed from parameters is usually computed in the function, as plain code. |
| `useCallback` | none needed | Handlers are attached again on each run anyway (section 6). |
| `useEffect(f, [])` | `use_effect_with((), |_| { ..; on_cleanup(..) })` | The cleanup is registered with `on_cleanup` instead of being returned. |
| `useEffect(f, [a, b])` | `use_effect_with((a, b), |(a, b)| ..)` | The dependencies are compared with `PartialEq`, as React compares them with `Object.is`. |
| `useEffect(f)` that reads state | `use_effect(|| ..)` | It runs again when the signals it read change, with no dependency list. |
| `useLayoutEffect` | `use_effect_with` | It runs right after the controls of the run are in place, before the next layout and frame (section 7). |
| `createContext`, `<Ctx.Provider value>`, `useContext` | `provide_context(value)`, `use_context::<T>()` | Contexts are found by type, not by an object. |
| `React.memo(C)` | the default of `#[component(rerender)]` for parameters that are `PartialEq + Clone`; `memo` requires it, `always` opts out | React does not memoize by default; FerroUI does, because a run of 2,600 children costs 14 ms and a memoized one 0.23 ms (section 12.3 of declarative-ui.md). |
| controlled input (`value` and `onChange`) | `text: name.get()` with `on_changed(TextBox::text_property(), ..)`, or `text: name.two_way()` | A two-way value is shorter. The hooks temperature converter shows both forms. |
| uncontrolled input with a ref | a control with no bound value, read through a `NodeRef` | |
| `createPortal` | `Popup { .. }` with a child element; `context_menu: rsx! { ContextMenu { .. } }` | The child of a popup is shown above the window, outside the visual tree of its parent, under its owner, as a portal. A second window is `open_window(|| ..)`. |
| CSS modules, CSS-in-JS (styled-components, emotion) | `styles: [style("Button.accent").set(..).nested(style("^:pointerover")..)]` | Styles of the property system with selectors in the syntax of XAML, scoped to the control and the controls below it. |
| `prefers-color-scheme`, a theme context | `use_theme_variant()`, `ThemeVariantScope { requested_theme_variant: .. }` | |
| CSS custom properties (`var(--accent)`) | `use_resource_as::<T>(key)` | Resources of the theme follow the theme variant. |
| CSS transitions, Framer Motion | `transitions: transitions([transition(property, ms(200))])`, `animation(..)`, `use_animation` | Transitions and key-frame animations of the port, on any animatable property. |
| a list virtualized with react-window or react-virtuoso | a virtualized `ListBox` with `item_template: template(|item: T| ..)` | The virtualizing panels of FerroUI realize the visible items. The template keeps a pool of hosts, so scrolling patches existing items instead of mounting new ones (as react-window reuses its rows). |
| a React root inside a page of another framework (`createRoot(el).render(..)`) | `ComponentHost`, in XAML or any tree of controls | |
| React inside a page of another framework, or that page inside React | `use_xaml(..)` puts XAML inside a component; `ComponentHost` puts a component inside XAML | |
| error boundary (`componentDidCatch`, `getDerivedStateFromError`) | `error_boundary(\|\| child, \|info\| fallback)` | A function, not a class. A panic of a component's run below it, on the first run or a later one, shows the fallback; `info.message()` is the panic's message and `info.reset()` mounts the child again. Handlers are not guarded, as in React; unlike React, effects (`use_effect`, `use_effect_with`) are not guarded either. A caught panic is reported as `Diagnostic::ErrorCaught`. |
| `Suspense`, `use(promise)`, data fetching in `useEffect` | `use_future(deps, \|deps\| future)` and `suspense(state, pending, ready, failed)` | The future runs on the thread's executor (`ferroui_system::threading::tasks`) and its state is a signal of `AsyncState` (`Pending`, `Ready`, `Failed`). New dependencies abandon the future before, whose result is then ignored, as an effect's cleanup sets an `ignore` flag in React. `suspense` follows one state; it does not wait for the futures of the components below it as React's `Suspense` does. `sleep(duration)` debounces. |
| `lazy` | none: the code of an application is linked in | |
| React Router (`<Routes>`, `<Route path>`, `useNavigate`, `useParams`) | `router(initial, vec![route("/people/:id", \|m\| ..), ..])`, `use_router()`, `m.param("id")` | Modelled on the routing of ReactiveUI: a stack of paths with `navigate`, `navigate_back`, `navigate_and_reset`, `current` and `can_go_back`, all reactive. The first matching route is shown; `*` matches the rest of a path. There is no URL bar: the stack lives in the window. |
| `useTransition`, `useDeferredValue` | none: rendering is synchronous | |
| Fast Refresh | hot reload of the literals of `rsx!` (feature `hot-reload`) | Literals change while the application runs and components keep their state. A change of structure needs a rebuild; Fast Refresh also swaps code. |
| React DevTools | `inspector()`, `open_inspector()` (Shift+F12 in debug builds) | Components with their runs and hooks, controls with their properties and where each value comes from. |
| `StrictMode`, the warnings of React (keys) | diagnostics in debug builds: lists without keys, duplicate keys, components that run away | `set_diagnostics_handler` sends them elsewhere. |
| Server components, server rendering | none: FerroUI renders on the device | |

## 4. The same programs

### 4.1 A counter

React:

```jsx
function Counter() {
  const [count, setCount] = useState(0);
  return (
    <div className="row">
      <input value={count} readOnly />
      <button onClick={() => setCount(c => c + 1)}>Count</button>
    </div>
  );
}
```

FerroUI, a component that runs again (`samples/SevenGuis/Hooks/Counter`):

```rust
#[component(rerender)]
pub fn Counter() -> Element {
    let count = use_signal(|| 0);
    rsx! {
        StackPanel { orientation: Orientation::Horizontal, spacing: 8.0,
            TextBox { text: count.get().to_string(), is_read_only: true, width: 80.0 }
            Button { on_click: move |_| count.update(|c| *c += 1), "Count" }
        }
    }
}
```

FerroUI, a component that runs once (`samples/SevenGuis/Macros/Counter`). Here the text follows the signal, and the function never runs again:

```rust
#[component]
pub fn Counter() -> Element {
    let count = signal(0);
    rsx! {
        StackPanel { orientation: Orientation::Horizontal, spacing: 8.0,
            TextBox { text: count, is_read_only: true, width: 80.0 }
            Button { on_click: move |_| count.update(|c| *c += 1), "Count" }
        }
    }
}
```

### 4.2 A keyed list with a derived value

React:

```jsx
function People({ people }) {
  const [prefix, setPrefix] = useState("");
  const shown = people.filter(p => p.surname.startsWith(prefix));
  return (
    <>
      <input value={prefix} onChange={e => setPrefix(e.target.value)} />
      <ul>{shown.map(p => <li key={p.id}>{p.label}</li>)}</ul>
    </>
  );
}
```

FerroUI (the CRUD of `samples/SevenGuis/Hooks/Crud`, shortened):

```rust
#[component(rerender)]
fn People(people: Vec<Person>) -> Element {
    let prefix = use_signal(String::new);
    let shown = filter_people(&people, &prefix.get());
    rsx! {
        StackPanel {
            TextBox { text: prefix.two_way() }
            ListBox {
                for p in &shown {
                    ListBoxItem { key: p.id, content: p.label() }
                }
            }
        }
    }
}
```

In both, the item of a person who stays keeps its control (its DOM node in React) when the filter changes.

### 4.3 An effect with a cleanup

React:

```jsx
useEffect(() => {
  const id = setInterval(tick, 50);
  return () => clearInterval(id);
}, []);
```

FerroUI (`samples/SevenGuis/Hooks/Timer`):

```rust
use_effect_with((), move |_| {
    let ticking = DispatcherTimer::run(tick, Duration::from_millis(50), DispatcherPriority::NORMAL);
    on_cleanup(move || ticking.dispose());
});
```

### 4.4 Context

React:

```jsx
const Theme = createContext("light");
<Theme.Provider value="dark"><Toolbar /></Theme.Provider>
function Toolbar() { const theme = useContext(Theme); ... }
```

FerroUI:

```rust
#[derive(Clone)]
struct Theme(&'static str);

#[component(rerender)]
fn App() -> Element {
    provide_context(Theme("dark"));
    rsx! { Toolbar {} }
}

#[component(rerender)]
fn Toolbar() -> Element {
    let theme = use_context::<Theme>().map_or("light", |theme| theme.0);
    ..
}
```

## 5. How a change travels

**React.**
1. `setState` queues an update and schedules a render of the component.
2. The updates of an event, a promise or a timeout are batched (React 18 and later).
3. The render phase calls the component and its children. In concurrent rendering it can be interrupted.
4. The reconciler compares the new elements with the current fiber tree.
5. The commit phase applies the changes to the DOM, runs layout effects, and later passive effects (`useEffect`).

**FerroUI, a component that runs again.**
1. A write to a signal marks what read it.
2. When the write (or the `batch`) ends, the component's effect runs its function. The function reads the signals it needs, and those reads are recorded.
3. The reconciler compares the returned elements with the mounted instances and patches the controls at once.
4. The closures of `use_effect_with` run.
5. A child component whose parent gave it new parameters runs next, in the same propagation of changes.
6. Layout and drawing happen later, on the dispatcher, as for any change of a control.

**FerroUI, a component that runs once.** A write runs the effects of the properties and lists that follow the signal. Nothing else runs, and nothing is compared.

Three differences follow:

- **Dependencies are tracked, not declared.** A component runs again because it read a signal that changed, not because a setter of its own was called. A child that reads a signal its parent owns runs again on its own, without its parent. `use_effect` and `use_memo` need no dependency list. `use_effect_with` keeps one, for effects that depend on parameters.
- **No automatic batching.** Each write outside `batch(|| ..)` propagates at once. This is deliberate: code written for components that run once counts on effects that run in the middle of a handler. Several writes that belong together go in `batch`.
- **Synchronous.** No render is deferred or interrupted. A large update blocks the thread until it is done, as any change to retained controls does.

## 6. Reconciliation compared

| Rule | React | FerroUI |
|---|---|---|
| Another type at the same place | the old subtree is unmounted and a new one mounted | the same: another control class or component type is mounted anew |
| The same type | the props are updated, and the component renders with them | the same; a control keeps its control object, a component its hooks |
| Keys | identify siblings in a list | the same |
| Siblings without keys | matched by their index among the siblings | matched by their place among the siblings without a key, so a keyed sibling that comes or goes does not shift them |
| Props that are equal | not compared for host elements: React compares them and writes what differs to the DOM | plain values are compared by value and left alone when equal |
| Event handlers | React's synthetic event system holds the handler and updates it | a trampoline stays attached and its closure is swapped on each run |
| A prop that is gone | removed from the DOM node | the property is cleared back to its default or style value |
| Moves in a keyed list | one pass over the old indices: a node whose old index falls behind the last one kept in place is moved | the longest increasing subsequence of the old places stays, the rest is moved: the fewest moves (as Vue and Inferno) |
| Text | text nodes | text in a content or an item, a `TextBlock` elsewhere; changed in place |

## 7. Effects and their timing

React runs `useLayoutEffect` after the DOM changes and before the browser paints, and `useEffect` after the paint. FerroUI has no paint in the middle of a run. The closures of `use_effect_with` run right after the reconciler placed the controls of the run, before the dispatcher lays out and draws the next frame. That is the timing of `useLayoutEffect`. An effect that needs the size of a control (after layout) waits for the layout event of the control (`LayoutUpdated`, `SizeChanged`) instead.

`use_effect` is an effect of the reactive runtime. It starts after the first run and then runs whenever the signals it read change, independent of the runs of the component.

## 8. What FerroUI has that React does not

- **Two models in one tree.** A component may run again or run once, and a property may follow a signal even in a component that runs again (`text: count`). The part of the screen that changes most can update without running anything (the gauge of a timer, the value of a cell), while the structure is written as in React.
- **Types end to end.** Every property setter, handler and component parameter is checked by the compiler, including the conversions (`8.0` to a thickness, `"Red"` to a brush). A typo is a compile error, not an `undefined`.
- **Two-way values.** `text: name.two_way()` binds a field to a signal both ways, without an `onChange`.
- **The property system.** Styles, themes, data bindings (`Bind(..)`), attached properties (`Grid.row: 1`), and resources apply to the controls of `rsx!` as they do to controls made by XAML.
- **XAML alongside.** The same controls can be written in XAML, and the two host each other: `ComponentHost` in XAML, `use_xaml` in a component. An MVVM view can show a component that reads its view model with `use_data_context`.
- **Native rendering everywhere.** One code base draws the same controls on macOS, Windows, Linux, iOS, Android and in the browser.

## 9. What React has that FerroUI does not have yet

| React | FerroUI plan |
|---|---|
| `Suspense` that waits for the data of the components below it, streaming | not planned: `suspense` follows the state of one future |
| Concurrent rendering, transitions | not planned: rendering is synchronous by design |
| Server components, server rendering | not planned |

## 10. Other Rust frameworks

| Framework | Model | Compared with `ferroui-view` |
|---|---|---|
| Dioxus | components that run again, `use_signal`, signals that subscribe the component | the closest to `#[component(rerender)]`; Dioxus renders to the web, to the desktop through a web view, and to an experimental native renderer |
| Leptos | components that run once, fine-grained signals | the closest to `#[component]` |
| Yew | components that run again, `use_state` with a setter, a virtual DOM | React's model, for the web |
| Xilem | a view tree rebuilt and diffed on each change, retained widgets | a retained widget tree as in FerroUI, with the view tree typed statically rather than as data |

`ferroui-view` sits on a complete retained framework with an existing control set, themes, XAML and a property system. Its elements are data, so the run-once and run-again models share one tree, one reconciler and one set of builders.

## 11. Advice for React developers

- Start with `#[component(rerender)]` and `use_signal`, and write the function as you would in React. Compute what derives from state in the function.
- Give the items of a list a `key`, as in React.
- Put writes that belong together in `batch(|| ..)`. One write runs the component once; two writes outside a batch run it twice.
- Call hooks at the top of the function, never in a condition or a loop. A run that calls them in another order panics.
- State made with `signal(..)` inside a component that runs again is lost on the next run. Use `use_signal`.
- When a value changes often and drives one property (a progress value, a clock), bind the property to the signal (`value: elapsed`) instead of reading it in the function. Only that property updates.
- Layout is not CSS. A `StackPanel` stacks, a `Grid` has rows and columns (`row_definitions: "Auto,*"`), and alignment and margins are properties of each control.
- The samples are the best reference. `samples/SevenGuis` has every 7GUIs task three times (builders, `rsx!`, hooks), each a small crate with a scenario test.
