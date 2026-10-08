# ControlCatalog: open gaps

The documents of the sample that do not load yet, grouped by what they wait for. `excluded.txt` is the machine-readable list (the generated tests of a listed document are ignored with its reason); this page explains each open entry. Every gap of the framework has a minimal reproduction in `tests/gaps*.rs`, ignored with the same identifier, which passes once the gap is closed:

```sh
cargo test -p control-catalog -- --ignored gap_
```

Status: 219 documents, 196 load and show their class; 23 are listed below (one of them, `App.xaml`, loads in the application and in `tests/gaps.rs`).

## Gaps of the framework

| Gap | What is missing | Reproduction | Documents |
|---|---|---|---|
| C102 | `FerroUI.Data.DataValidationException` is not declared for markup (`<data:DataValidationException>` with `x:Arguments`). | `gaps_a::gap_c102_data_validation_exception_in_markup` | `Pages/TextBox/TextBoxValidationPage.xaml` |
| C201 | `System.Collections.ArrayList` is not a type of the markup type system: the element and its children (null included) as an items source. | `gaps_b::gap_c201_array_list_in_markup` | `Pages/ComboBoxPage.xaml` |
| C202 | `OnPlatform` written as an element with `On` children has no content property and is not accepted as the value of the property it is set on. | `gaps_b::gap_c202_on_platform_element_with_on_children` | `Pages/PlatformInfoPage.xaml` |
| C203 | A handler whose second parameter is `EventArgs` is not accepted for `PopupFlyoutBase.Opening`, and the handler cannot reach the cancellable arguments the event passes. | `gaps_b::gap_c203_flyout_opening_handler` | `Pages/ContextFlyoutPage.xaml` |
| C207 | A compiled binding does not stream (`^`) a property of type `IObservable<T>` (the view model declares it as `ObservableValue`): "Compiled bindings do not support stream bindings for objects of type ObservableValue". | `gaps_b::gap_c207_compiled_stream_binding_of_an_observable` | `Pages/ListBoxPage.xaml` |
| C208 | `FontFamily.Name` is not declared in the markup metadata of `FontFamily`, so `{Binding Name}` with `x:DataType="FontFamily"` does not resolve. | `gaps_b::gap_c208_font_family_name_in_a_compiled_binding` | `Pages/ComboBoxPage.xaml` |
| C209 | The methods `TextBox.Cut`, `Copy`, `Paste` and `Clear` are not declared for markup, so they cannot be bound as commands (`{Binding $parent[TextBox].Cut}`). | `gaps_b::gap_c209_text_box_methods_as_commands` | `Pages/ContextFlyoutPage.xaml` |
| C301 | A method name is not accepted for a property of a delegate type (`ToolTip.CustomPopupPlacementCallback`, `PopupFlyoutBase.CustomPopupPlacementCallback`). | `gaps_c::gap_c301_method_name_for_a_delegate_property` | `Pages/FlyoutsPage.xaml`, `Pages/ToolTipPage.xaml` |
| C305 | `System.Collections.Generic.List`1` with `x:TypeArguments` is not a type of the markup type system, so a list created in markup is not accepted by `ItemsControl.ItemsSource`. | `gaps_c::gap_c305_generic_list_element` | `Pages/FocusPage.xaml`, `Pages/RefreshContainerPage.xaml`, `Pages/DialogsPage.xaml` |
| C306 | `Slider.Ticks` (`TickList`) is not converted from text (`Ticks="0,20,25,40,75,100"`). | `gaps_c::gap_c306_slider_ticks_from_text` | `Pages/SliderPage.xaml` |
| C309 | `System.Collections.ArrayList` with enumeration values as children is not a type of the markup type system. | `gaps_c::gap_c309_array_list_element` | `Pages/ViewboxPage.xaml` |
| C310 | `AutoCompleteBox.MinimumPopulateDelay` (`TimeSpan`) is not converted from text (`"00:00:01"`). | `gaps_c::gap_c310_time_span_property_from_text` | `Pages/AutoCompleteBoxPage.xaml` (which also waits for its code-behind) |
| C312 | The event `FerroObject.PropertyChanged` is not declared for markup (`<openGl:GlPageKnobs PropertyChanged="KnobsPropertyChanged"/>`): the events of `FerroObject` in `src/FerroUI.Base/markup_types/classes.rs` do not list it, and its arguments, `FerroPropertyChangedEventArgs<'_>`, borrow the values of the change, so they are not a value a markup handler can be invoked with as they are. | `gaps_c::gap_c312_property_changed_event_in_markup` | `Pages/OpenGl/OpenGlInteropPage.xaml`, `Pages/OpenGl/OpenGlLeasePage.xaml` |

One gap of the framework blocks no document of the list: C101, a reflection binding (`x:CompileBindings="False"`) cannot resolve `$parent[prefix:Type]` for a type of a `using:` namespace (`gaps_a::gap_c101_reflection_binding_parent_of_prefixed_type`). The theme of `SampleGalleryPage` uses such paths with compiled bindings, which resolve them.

## Code-behind that needs framework API the port does not have

| Document | Missing API |
|---|---|
| `Pages/AutoCompleteBoxPage.xaml` | its code-behind (`LogicalExtensions.GetLogicalDescendants`, which it uses, is ported); the document also needs gap C310. |
| `Pages/NumericUpDownPage.xaml` | `CultureInfo.GetCultures(CultureTypes.SpecificCultures)`. |
| `Pages/OpenGl/OpenGlInteropPage.xaml` | Nothing but gap C312: the code-behind is not ported because its handler `KnobsPropertyChanged` cannot be declared for markup until the event is. What it uses besides is ported: `OpenGlContent` (`Pages/OpenGl/open_gl_content.rs`), `GlInterface`, `IGlContext`, the composition interop of `ferroui-opengl` (`OpenGlCompositionInterop::try_create_compatible_gl_context`, `ICompositionGlContext`, `ICompositionGlTexture` and its lease), `Compositor::create_drawing_surface`, `create_surface_visual` and `request_composition_update`, and the events `AttachedToVisualTree` and `DetachedFromVisualTree` of the viewport, which are declared for markup. |
| `Pages/OpenGl/OpenGlLeasePage.xaml` | Gap C312, and `OpenGlFbo` (`Pages/OpenGl/OpenGlFbo.cs`, not ported): it wraps an OpenGL texture in a Ganesh surface (`GRContext.ResetContext` and `Flush`, `GRBackendTexture`, `GRGlTextureInfo`, `SKSurface.Create` over a backend texture). The Skia backend enables the `gl` feature of `skia-safe`, which has these types, for `target_os = "emscripten"` only (`src/Skia/FerroUI.Skia/Cargo.toml`; the desktop build is Graphite on Metal, and no published Skia binary has Graphite and Ganesh together), and `ISkiaGrContext` does not give the Ganesh context (`skia_safe::gpu::DirectContext`) to a lease. The page needs both, and a desktop platform whose leased graphics context is an `IGlContext`. |
| `Pages/ContentPage/ContentPagePerformancePage.xaml`, `Pages/CarouselPage/CarouselPagePerformancePage.xaml`, `Pages/DrawerPage/DrawerPagePerformancePage.xaml`, `Pages/NavigationPage/NavigationPagePerformancePage.xaml`, `Pages/TabbedPage/TabbedPagePerformancePage.xaml` | `NavigationPerformanceMonitorHelper` measures the managed heap (`GC.GetTotalMemory`, `GC.Collect`), which has no counterpart. |

## Classes of the sample that are not ported yet

These documents load without their class (the survey test, `cargo test -p control-catalog -- --ignored survey`, with `CATALOG_SURVEY` set); their code-behind is sample work, not a framework gap:


## Test harness

| Document | Why |
|---|---|
| `App.xaml` | The tray icon of the document loads its icon through the icon loader of the platform, which the unit test application does not have. The document loads in the application, with the styles of the colour picker library it includes; `gaps::the_application_document_loads` loads it with a test icon loader. |
| `Pages/SettingsPage.xaml` | `{x:Static local:App.CurrentTheme}` needs the application of the catalog as the current application. The page loads in the application, which offers it; the unit test application is not that application. |
