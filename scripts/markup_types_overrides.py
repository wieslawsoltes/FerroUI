"""Hand-maintained input of scripts/generate_markup_types.py: what the `markup:`
part of a class needs and the generator cannot derive from the sources.

One entry per class (by Rust type name), for FerroUI.Base (`BASE`) and
FerroUI.Controls (`CONTROLS`). Each value is a dictionary of optional parts;
every list item is the Rust text of one item of the corresponding part of the
declaration form (see "Markup metadata" in docs/porting/PORTING-GUIDE.md):

    content       the content property, where upstream marks none the
                  generator can read
    parse         the callable of the `parse:` part
    constructors  constructors with arguments: "(A, B) => callable"
    properties    plain properties the generator cannot declare from the
                  getter/setter signatures (adapter closures, write-only):
                  "Name: T { get: .., set: .. } [Attributes]"; these replace
                  the generated entry of the same name
    skip          upstream plain properties that must not be declared (and
                  must not be reported as missing)
    methods       "fn Name(A) => callable" / "static fn .."
    fields        static values (static fields) besides the routed events: "Name: T => callable"
    static_properties
                  static properties of the managed original: "Name: T { get: callable }"
    events        plain events the generator cannot adapt: "Name(A) => callable"
    attributes    type attributes the generator cannot read (constants)
    note          a `//` comment placed above the declaration

After editing, rerun `python3 scripts/generate_markup_types.py` and the tests
of both crates.
"""

BASE = {
    'KeySpline': {
        'parse': "KeySpline::parse",
        'constructors': [
            "(f64, f64, f64, f64) => KeySpline::with_points",
        ],
    },
    'ResourceDictionary': {
        'properties': [
            "Count: i32 { get: |this: &Ref<ResourceDictionary>| this.count() as i32 }",
        ],
        'methods': [
            """try fn Add(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<ResourceDictionary>, key: Option<BoxedValue>, value: Option<BoxedValue>| {
                    new_resource_key(this, key).map(|key| this.add(key, value))
                }""",
            """try fn AddDeferred(Option<BoxedValue>, Rc<dyn IDeferredContent>) =>
                |this: &Ref<ResourceDictionary>, key: Option<BoxedValue>, content: Rc<dyn IDeferredContent>| {
                    new_resource_key(this, key).map(|key| this.add_deferred(key, content))
                }""",
            """try fn AddNotSharedDeferred(Option<BoxedValue>, Rc<dyn IDeferredContent>) =>
                |this: &Ref<ResourceDictionary>, key: Option<BoxedValue>, content: Rc<dyn IDeferredContent>| {
                    new_resource_key(this, key).map(|key| this.add_not_shared_deferred(key, content))
                }""",
            """fn EnsureCapacity(i32) =>
                |this: &Ref<ResourceDictionary>, capacity: i32| this.ensure_capacity(capacity.max(0) as usize)""",
        ],
        'note': """// The key of a resource is any object in the managed original; `resource_key` converts the kinds of key the
// port has.""",
    },
    'NameScope': {
        'methods': [
            """static fn GetNameScope(Ref<StyledElement>) -> Option<NameScopeRef> =>
            |styled: Ref<StyledElement>| NameScope::get_name_scope(&styled)""",
            """static fn SetNameScope(Ref<StyledElement>, Option<NameScopeRef>) =>
            |styled: Ref<StyledElement>, scope: Option<NameScopeRef>| NameScope::set_name_scope(&styled, scope)""",
        ],
    },
    'PullGestureRecognizer': {
        'constructors': [
            "(PullDirection) => PullGestureRecognizer::with_direction",
        ],
    },
    'CombinedGeometry': {
        'constructors': [
            "(Ref<Geometry>, Ref<Geometry>) => CombinedGeometry::with_geometries",
            "(GeometryCombineMode, Option<Ref<Geometry>>, Option<Ref<Geometry>>) => CombinedGeometry::with_mode",
            """(GeometryCombineMode, Option<Ref<Geometry>>, Option<Ref<Geometry>>, Option<Ref<Transform>>) =>
                |mode: GeometryCombineMode, a: Option<Ref<Geometry>>, b: Option<Ref<Geometry>>, t: Option<Ref<Transform>>| {
                    CombinedGeometry::with_mode_and_transform(mode, a, b, t)
                }""",
        ],
    },
    'DashStyle': {
        'constructors': [
            """(Option<MediaCollection<f64>>, f64) => |dashes: Option<MediaCollection<f64>>, offset: f64| {
                DashStyle::with_dash_list(dashes.unwrap_or_default(), offset)
            }""",
        ],
        'static_properties': [
            "Dash: Rc<dyn IDashStyle> { get: || -> Rc<dyn IDashStyle> { DashStyle::dash() } }",
            "Dot: Rc<dyn IDashStyle> { get: || -> Rc<dyn IDashStyle> { DashStyle::dot() } }",
            "DashDot: Rc<dyn IDashStyle> { get: || -> Rc<dyn IDashStyle> { DashStyle::dash_dot() } }",
            "DashDotDot: Rc<dyn IDashStyle> { get: || -> Rc<dyn IDashStyle> { DashStyle::dash_dot_dot() } }",
        ],
    },
    'DrawingBrush': {
        'constructors': [
            "(Ref<Drawing>) => |drawing: Ref<Drawing>| DrawingBrush::with_drawing(drawing)",
        ],
    },
    'DrawingImage': {
        'constructors': [
            "(Ref<Drawing>) => |drawing: Ref<Drawing>| DrawingImage::with_drawing(drawing)",
        ],
    },
    'EllipseGeometry': {
        'constructors': [
            "(Rect) => EllipseGeometry::with_rect",
        ],
    },
    'GradientStop': {
        'constructors': [
            "(Color, f64) => GradientStop::with_color_and_offset",
        ],
    },
    'ImageBrush': {
        'constructors': [
            "(Option<Rc<dyn IImageBrushSource>>) => ImageBrush::with_source",
        ],
    },
    'LineGeometry': {
        'constructors': [
            "(Point, Point) => LineGeometry::with_points",
        ],
    },
    'MatrixTransform': {
        'constructors': [
            "(Matrix) => MatrixTransform::with_matrix",
        ],
    },
    'Pen': {
        'constructors': [
            "(Option<Rc<dyn IBrush>>, f64, Option<Rc<dyn IDashStyle>>, PenLineCap, PenLineJoin, f64) => Pen::with_all",
        ],
    },
    'PolyBezierSegment': {
        'constructors': [
            "(Points, bool) => |points: Points, is_stroked: bool| PolyBezierSegment::with_points(points.iter(), is_stroked)",
        ],
    },
    'PolyLineSegment': {
        'constructors': [
            "(Points) => |points: Points| PolyLineSegment::with_points(points.iter())",
        ],
    },
    'PolylineGeometry': {
        'constructors': [
            "(Points, bool) => |points: Points, is_filled: bool| PolylineGeometry::with_points(points.iter(), is_filled)",
            """(Points, bool, FillRule) => |points: Points, is_filled: bool, fill_rule: FillRule| {
                PolylineGeometry::with_points_and_fill_rule(points.iter(), is_filled, fill_rule)
            }""",
        ],
    },
    'RectangleGeometry': {
        'constructors': [
            "(Rect) => RectangleGeometry::with_rect",
            "(Rect, f64, f64) => RectangleGeometry::with_rect_and_radii",
        ],
    },
    'Rotate3DTransform': {
        'constructors': [
            "(f64, f64, f64, f64, f64, f64, f64) => Rotate3DTransform::with_values",
        ],
    },
    'RotateTransform': {
        'constructors': [
            "(f64) => RotateTransform::with_angle",
            "(f64, f64, f64) => RotateTransform::with_angle_and_center",
        ],
    },
    'ScaleTransform': {
        'constructors': [
            "(f64, f64) => ScaleTransform::with_scale",
        ],
    },
    'SkewTransform': {
        'constructors': [
            "(f64, f64) => SkewTransform::with_angles",
        ],
    },
    'SolidColorBrush': {
        'constructors': [
            "(Color) => SolidColorBrush::with_color",
            "(Color, f64) => SolidColorBrush::with_color_and_opacity",
            "(u32) => SolidColorBrush::from_uint32",
        ],
    },
    'TranslateTransform': {
        'constructors': [
            "(f64, f64) => TranslateTransform::with_offset",
        ],
    },
    'VisualBrush': {
        'constructors': [
            "(Ref<Visual>) => |visual: Ref<Visual>| VisualBrush::with_visual(visual)",
        ],
    },
    'CroppedBitmap': {
        'constructors': [
            "(Rc<dyn IImage>, PixelRect) => CroppedBitmap::with_source",
        ],
    },
    'ControlTheme': {
        'constructors': [
            "(&'static TypeInfo) => ControlTheme::with_target_type",
        ],
        'properties': [
            "TargetType: Option<&'static TypeInfo> { get: ControlTheme::target_type, set: ControlTheme::set_target_type }",
        ],
    },
    'StyleBase': {
        'methods': [
            "fn Add(Rc<dyn SetterBase>) => |this: &Ref<StyleBase>, setter: Rc<dyn SetterBase>| this.add(setter)",
            "fn Add(Rc<dyn IStyle>) => |this: &Ref<StyleBase>, style: Rc<dyn IStyle>| this.add_style(style)",
        ],
    },
    'FerroObject': {
        'methods': [
            """fn Bind(&'static FerroProperty, Rc<dyn BindingBase>) -> Rc<dyn BindingExpressionBase> =>
                |this: &Ref<FerroObject>, property: &'static FerroProperty, binding: Rc<dyn BindingBase>| {
                    this.bind_binding(property, &*binding)
                }""",
            """try fn SetValue(&'static FerroProperty, Option<BoxedValue>, BindingPriority) -> Option<Rc<dyn IDisposable>> =>
                |this: &Ref<FerroObject>, property: &'static FerroProperty, value: Option<BoxedValue>, priority: BindingPriority| {
                    // The value is set as a value of the type of the property.
                    property_value(property, value).map(|value| this.set_value_untyped(property, value.as_any(), priority))
                }""",
        ],
    },
    'Styles': {
        'methods': [
            """try fn Add(Rc<dyn IStyle>) => |this: &Ref<Styles>, style: Rc<dyn IStyle>| {
                // A style that has an owner cannot be added to styles that have one.
                let owned = style.as_object().and_then(|o| o.to_ref().cast::<StyleBase>()).is_some_and(|s| s.owner().is_some());
                if owned && this.owner().is_some() {
                    return Err("The Style already has a parent.");
                }
                Ok(this.add(style))
            }""",
        ],
    },
    'Geometry': {
        'parse': "Geometry::parse",
    },
    # The text content of an element of these classes is the path data (the geometry
    # converter of the managed original applies to the classes derived from `Geometry`).
    'StreamGeometry': {
        'parse': "StreamGeometry::parse",
    },
    'PathGeometry': {
        'parse': "PathGeometry::parse",
    },
    'Transform': {
        'parse': "Transform::parse",
    },
    'CacheMode': {
        'parse': "CacheMode::parse",
    },
}

CONTROLS = {
    # The accessors of the design-time properties that are not plain attached-property
    # accessors, in the declaration order of the managed original (overloads are tried
    # in that order).
    'DataValidationErrors': {
        'methods': [
            """static fn SetError(Ref<Control>, Option<ferroui_base::data::BindingError>) =>
                |control: Ref<Control>, error: Option<ferroui_base::data::BindingError>| {
                    DataValidationErrors::set_error(&control, error.as_ref())
                }""",
        ],
        'note': """// `SetError` has no getter and no registered property in the managed original either: markup sees it as the
// attached property `DataValidationErrors.Error`.""",
    },
    'Design': {
        'methods': [
            """static fn SetDataContext(Rc<dyn IDataTemplate>, Option<BoxedValue>) =>
                |template: Rc<dyn IDataTemplate>, value: Option<BoxedValue>| Design::set_data_context_for_data_template(&template, value)""",
            """static fn GetDataContext(Rc<dyn IDataTemplate>) -> Option<BoxedValue> =>
                |template: Rc<dyn IDataTemplate>| Design::get_data_context_for_data_template(&template)""",
            """static fn SetPreviewWith(Ref<FerroObject>, Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>) =>
                |target: Ref<FerroObject>, template: Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>| Design::set_preview_with_template(&target, template)""",
            """static fn SetPreviewWith(Ref<FerroObject>, Option<Ref<Control>>) =>
                |target: Ref<FerroObject>, control: Option<Ref<Control>>| Design::set_preview_with(&target, control)""",
            """static fn SetPreviewWith(Ref<ResourceDictionary>, Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>) =>
                |target: Ref<ResourceDictionary>, template: Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>| {
                    Design::set_preview_with_template(&target.upcast(), template)
                }""",
            """static fn SetPreviewWith(Ref<ResourceDictionary>, Option<Ref<Control>>) =>
                |target: Ref<ResourceDictionary>, control: Option<Ref<Control>>| Design::set_preview_with(&target.upcast(), control)""",
            """static fn SetPreviewWith(Rc<dyn IDataTemplate>, Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>) =>
                |target: Rc<dyn IDataTemplate>, template: Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>| Design::set_preview_with_template_for_data_template(&target, template)""",
            """static fn SetPreviewWith(Rc<dyn IDataTemplate>, Option<Ref<Control>>) =>
                |target: Rc<dyn IDataTemplate>, control: Option<Ref<Control>>| Design::set_preview_with_for_data_template(&target, control)""",
            """static fn SetPreviewWith(Rc<dyn IStyle>, Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>) =>
                |target: Rc<dyn IStyle>, template: Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>| Design::set_preview_with_template_for_style(&target, template)""",
            """static fn SetPreviewWith(Rc<dyn IStyle>, Option<Ref<Control>>) =>
                |target: Rc<dyn IStyle>, control: Option<Ref<Control>>| Design::set_preview_with_for_style(&target, control)""",
            """static fn GetPreviewWith(Ref<FerroObject>) -> Option<Ref<Control>> =>
                |target: Ref<FerroObject>| Design::get_preview_with(&target)""",
            """static fn GetPreviewWith(Ref<ResourceDictionary>) -> Option<Ref<Control>> =>
                |target: Ref<ResourceDictionary>| Design::get_preview_with(&target.upcast::<FerroObject>())""",
            """static fn GetPreviewWith(Rc<dyn IDataTemplate>) -> Option<Ref<Control>> =>
                |target: Rc<dyn IDataTemplate>| Design::get_preview_with_for_data_template(&target)""",
            """static fn GetPreviewWith(Rc<dyn IStyle>) -> Option<Ref<Control>> =>
                |target: Rc<dyn IStyle>| Design::get_preview_with_for_style(&target)""",
        ],
    },
    'NativeMenuItem': {
        'constructors': [
            "(String) => |header: String| NativeMenuItem::with_header(&header)",
        ],
        'events': [
            """Click(Option<BoxedValue>, EventArgs) => |this: &Ref<NativeMenuItem>, handler: MarkupDelegate| {
                let sender = this.downgrade();
                this.click(move |_item: &NativeMenuItem| {
                    handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(EventArgs::EMPTY)]);
                })
            }""",
        ],
        'note': """// The handler of `Click` takes the sender and empty event arguments in the managed original (`EventHandler`).""",
    },
    'Application': {
        'properties': [
            "Styles: Ref<Styles> { get: Application::styles }",
            "ApplicationLifetime: Option<Rc<dyn IApplicationLifetime>> { set: Application::set_application_lifetime }",
        ],
        'static_properties': [
            "Current: Option<Ref<Application>> { get: Application::current }",
        ],
    },
    'Popup': {
        'properties': [
            "DependencyResolver: Option<Rc<dyn IFerroDependencyResolver>> { set: Popup::set_dependency_resolver }",
        ],
    },
    'TextBox': {
        'properties': [
            """SelectedText: String {
                get: TextBox::selected_text,
                set: |text_box: &Ref<TextBox>, value: String| text_box.set_selected_text(Some(&value))
            }""",
        ],
        'static_properties': [
            "CutGesture: Option<KeyGesture> { get: TextBox::cut_gesture }",
            "CopyGesture: Option<KeyGesture> { get: TextBox::copy_gesture }",
            "PasteGesture: Option<KeyGesture> { get: TextBox::paste_gesture }",
        ],
    },
    'WindowDrawnDecorationsContent': {
        'properties': [
            """Overlay: Option<Ref<Control>> {
                get: WindowDrawnDecorationsContent::overlay,
                set: |content: &Ref<WindowDrawnDecorationsContent>, value: Option<Ref<Control>>| content.set_overlay(value)
            }""",
            """Underlay: Option<Ref<Control>> {
                get: WindowDrawnDecorationsContent::underlay,
                set: |content: &Ref<WindowDrawnDecorationsContent>, value: Option<Ref<Control>>| content.set_underlay(value)
            }""",
            """FullscreenPopover: Option<Ref<Control>> {
                get: WindowDrawnDecorationsContent::fullscreen_popover,
                set: |content: &Ref<WindowDrawnDecorationsContent>, value: Option<Ref<Control>>| {
                    content.set_fullscreen_popover(value)
                }
            }""",
        ],
    },
    'ColumnDefinition': {
        'constructors': [
            "(f64, GridUnitType) => ColumnDefinition::with_value",
            "(GridLength) => ColumnDefinition::with_width",
        ],
    },
    'RowDefinition': {
        'constructors': [
            "(f64, GridUnitType) => RowDefinition::with_value",
            "(GridLength) => RowDefinition::with_height",
        ],
    },
    'InlineUIContainer': {
        'constructors': [
            "(Ref<Control>) => |child: Ref<Control>| InlineUIContainer::with_child(child)",
        ],
    },
    'Run': {
        'constructors': [
            "(Option<String>) => |text: Option<String>| Run::with_text(text.as_deref())",
        ],
    },
    'ToggleSplitButton': {
        'attributes': [
            "PseudoClasses(\":checked\")",
        ],
    },
}
