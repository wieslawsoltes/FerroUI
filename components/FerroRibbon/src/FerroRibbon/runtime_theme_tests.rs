use crate::platform_settings;
use crate::{
    tests::run,
    theming::{RibbonTheme, RibbonThemeError, RibbonThemeResources, to_ribbon_color},
};
use ferroribbon_core::theming::{RibbonChromeStyle, RibbonThemePalette, RibbonThemeStyle};
use ferroui_base::{
    DirectProperty, FerroLocator, FerroObjectImpl, FerroProperty, Ref, StyledElement,
    StyledElementImpl,
    controls::ResourceDictionary,
    ferro_class, ferro_impl_classes, ferro_property, instantiate,
    media::{Brush, Color, IBrush, SolidColorBrush},
    platform::{ColorContrastPreference, IPlatformSettings, PlatformColorValues},
    reactive::IDisposable,
    styling::ThemeVariant,
    utilities::EventArgs,
};
use ferroui_controls::{Application, Border, Panel};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/runtime-theme.json")).unwrap()
}
fn hex(brush: Option<Ref<Brush>>) -> Option<String> {
    brush
        .and_then(|b| b.cast::<SolidColorBrush>())
        .map(|b| to_ribbon_color(b.color()).to_hex(true))
}
fn state() -> Value {
    json!({"palette":RibbonTheme::palette().name.as_ref().unwrap().to_utf8().unwrap(),"chrome":RibbonTheme::chrome_style().raw(),"style":RibbonTheme::style().raw()})
}
fn error(e: RibbonThemeError) -> Value {
    json!({"type":if e.empty {"ArgumentException"} else {"ArgumentNullException"},"param":e.parameter})
}
#[test]
fn source_runtime_theme_resource_discovery_cache_and_insertion() {
    // A fresh, independent native registry permits the original no-Application case.
    std::thread::spawn(|| {
        let resolver = FerroLocator::current();
        let mutable = FerroLocator::current_mutable();
        let empty = Rc::new(FerroLocator::new());
        FerroLocator::set_current(empty.clone());FerroLocator::set_current_mutable(empty);
        crate::register_types();
        RibbonTheme::reset_test_cache();let early=RibbonTheme::resources();
        let app=Application::new();Application::bind_current(app.clone());
        assert_eq!(json!({"same":early==RibbonTheme::resources(),"merged":app.resources().merged_dictionaries().count()}),corpus()["beforeApp"]);
        RibbonTheme::reset_test_cache();let nested=ResourceDictionary::new();let first=RibbonThemeResources::new();let second=RibbonThemeResources::new();nested.add_merged_dictionary(first.clone());app.resources().add_merged_dictionary(nested);app.resources().add_merged_dictionary(second);
        assert_eq!(json!({"first":first==RibbonTheme::resources(),"count":app.resources().merged_dictionaries().count()}),corpus()["discovery"]);
        RibbonTheme::reset_test_cache();app.set_resources(ResourceDictionary::new());let created=RibbonTheme::resources();
        let merged=app.resources().merged_dictionaries().get(0).as_object().unwrap().to_ref().cast::<RibbonThemeResources>().unwrap();
        assert_eq!(json!({"count":app.resources().merged_dictionaries().count(),"same":created==merged}),corpus()["insertion"]);
        RibbonTheme::reset_test_cache();FerroLocator::set_current(resolver);FerroLocator::set_current_mutable(mutable);
    }).join().unwrap();
}
#[test]
fn source_runtime_theme_changed_reentry_snapshot_and_null_palette() {
    run(|| {
        RibbonTheme::reset_test_cache();
        let events = Rc::new(RefCell::new(Vec::new()));
        let reentered = Rc::new(Cell::new(false));
        let b = Rc::new(RefCell::new(None::<Rc<dyn IDisposable>>));
        let c = Rc::new(RefCell::new(None::<Rc<dyn IDisposable>>));
        let a = {
            let events = events.clone();
            let reentered = reentered.clone();
            let b = b.clone();
            let c = c.clone();
            RibbonTheme::changed(move |sender, args| {
                events.borrow_mut().push(json!({"handler":"A","senderNull":sender.is_none(),"argsEmpty":*args==EventArgs::EMPTY,"state":state()}));
                if !reentered.replace(true) {
                    b.borrow().as_ref().unwrap().dispose();
                    let events = events.clone();
                    c.replace(Some(RibbonTheme::changed(move |s,e|events.borrow_mut().push(json!({"handler":"C","senderNull":s.is_none(),"argsEmpty":*e==EventArgs::EMPTY,"state":state()})))));
                    RibbonTheme::apply_chrome_style(RibbonChromeStyle::Colorful).unwrap();
                }
            })
        };
        {
            let events = events.clone();
            b.replace(Some(RibbonTheme::changed(move |s,e|events.borrow_mut().push(json!({"handler":"B","senderNull":s.is_none(),"argsEmpty":*e==EventArgs::EMPTY,"state":state()})))));
        }
        RibbonTheme::apply_with_style(
            Some(RibbonThemePalette::excel()),
            RibbonChromeStyle::Neutral,
            RibbonThemeStyle::Cad,
        )
        .unwrap();
        RibbonTheme::apply_palette(Some(RibbonThemePalette::excel())).unwrap();
        let err = RibbonTheme::apply_palette(None).unwrap_err();
        assert_eq!(
            json!({"type":"ArgumentNullException","param":match err {crate::theming::RibbonThemeApplyError::Resources(e)=>e.parameter, other=>panic!("unexpected handler error: {other}")}}),
            corpus()["nullPalette"]
        );
        assert_eq!(json!(*events.borrow()), corpus()["events"]);
        a.dispose();
        b.borrow().as_ref().unwrap().dispose();
        c.borrow().as_ref().unwrap().dispose();
        RibbonTheme::reset_test_cache();
    });
}
#[test]
fn source_runtime_theme_lookup_subtree_high_contrast_fallback_and_errors() {
    run(|| {
        RibbonTheme::reset_test_cache();
        let c = corpus();
        let locator = FerroLocator::enter_scope();
        let settings = platform_settings::Settings::new();
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformSettings>()
            .to_constant(settings.clone());
        let app = Application::current().unwrap();
        let old_resources = app.resources();
        let old_theme = app.requested_theme_variant();
        app.set_resources(ResourceDictionary::new());
        RibbonTheme::apply_with_style(
            Some(RibbonThemePalette::word()),
            RibbonChromeStyle::Neutral,
            RibbonThemeStyle::Office,
        )
        .unwrap();
        app.set_requested_theme_variant(Some(ThemeVariant::dark()));
        let scope = Border::new();
        let fallback = SolidColorBrush::with_color(Color::from_argb(128, 1, 2, 3));
        app.resources()
            .set("Fallback", Some(Rc::new(fallback.clone())));
        app.resources().set("Wrong", Some(Rc::new(42_i32)));
        let mut lookups = Vec::new();
        for dark in [false, true] {
            for hc in [false, true] {
                RibbonTheme::set_theme(
                    Some(&scope),
                    Some(if dark {
                        ThemeVariant::dark()
                    } else {
                        ThemeVariant::light()
                    }),
                )
                .unwrap();
                settings.set_colors(PlatformColorValues::new().with_contrast_preference(if hc {
                    ColorContrastPreference::High
                } else {
                    ColorContrastPreference::NoPreference
                }));
                lookups.push(json!({"dark":dark,"hc":hc,"color":hex(RibbonTheme::get_brush(Some(&scope),Some("RibbonAccentBrush")).unwrap()),"corner":RibbonTheme::get_corner_radius(Some(&scope),"RibbonPopupCornerRadius",99.).top_left}));
            }
        }
        for (actual, expected) in lookups.iter().zip(c["lookups"].as_array().unwrap()) {
            assert_eq!(actual["dark"], expected["dark"]);
            assert_eq!(actual["hc"], expected["hc"]);
            assert_eq!(actual["color"], expected["color"]);
            assert_eq!(actual["corner"].as_f64(), expected["corner"].as_f64());
        }
        settings.set_colors(PlatformColorValues::new());
        assert_eq!(
            json!(hex(
                RibbonTheme::get_brush(None, Some("RibbonAccentBrush")).unwrap()
            )),
            c["appDark"]
        );
        assert_eq!(
            json!({"same":RibbonTheme::get_brush(Some(&scope),Some("Fallback")).unwrap().unwrap()==fallback.upcast::<Brush>(),"wrong":RibbonTheme::get_brush(Some(&scope),Some("Wrong")).unwrap().is_none(),"missing":RibbonTheme::get_brush(Some(&scope),Some("Missing")).unwrap().is_none(),"radius":RibbonTheme::get_corner_radius(Some(&scope),"Missing",12.5).top_left}),
            c["fallbacks"]
        );
        assert_eq!(
            json!({"nullKey":error(RibbonTheme::get_brush(None,None).unwrap_err()),"emptyKey":error(RibbonTheme::get_brush(None,Some("")).unwrap_err()),"nullRoot":error(RibbonTheme::set_theme(None,None).unwrap_err()),"nullTarget":error(RibbonTheme::set_theme_brush(None,None,None).unwrap_err()),"nullProperty":error(RibbonTheme::set_theme_brush(Some(&scope),None,None).unwrap_err()),"nullRegistrationKey":error(RibbonTheme::set_theme_brush(Some(&scope),Some(Border::background_property().as_property()),None).unwrap_err())}),
            c["errors"]
        );
        // Theme inheritance is provided by the selected native engine, separately
        // from the deterministic source adapter's explicit ActualTheme seam.
        // Object-typed native properties retain the actual mutable brush handle.
        RibbonTheme::set_theme_brush(
            Some(&scope),
            Some(ferroui_controls::Control::tag_property().as_property()),
            Some("RibbonAccentBrush"),
        )
        .unwrap();
        let tagged = scope.tag().unwrap();
        let tagged = ferroui_base::data::core::ValueTypes::as_object(&*tagged)
            .unwrap()
            .cast::<Brush>()
            .unwrap();
        assert_eq!(
            tagged,
            RibbonTheme::get_brush(Some(&scope), Some("RibbonAccentBrush"))
                .unwrap()
                .unwrap()
        );
        let root = Panel::new();
        let child = Border::new();
        root.children().add(child.clone());
        RibbonTheme::set_theme(Some(&root), Some(ThemeVariant::dark())).unwrap();
        assert_eq!(child.actual_theme_variant(), Some(ThemeVariant::dark()));
        RibbonTheme::set_theme(Some(&child), Some(ThemeVariant::light())).unwrap();
        assert_eq!(
            hex(RibbonTheme::get_brush(Some(&child), Some("RibbonAccentBrush")).unwrap()),
            Some("#FF185ABD".into())
        );
        app.set_resources(old_resources);
        app.set_requested_theme_variant(old_theme);
        RibbonTheme::reset_test_cache();
        locator.dispose();
    });
}
#[repr(C)]
struct ThemeTarget {
    base: StyledElement,
    first: RefCell<Option<Rc<dyn IBrush>>>,
    second: RefCell<Option<Rc<dyn IBrush>>>,
    writes: RefCell<Vec<String>>,
    reentry: RefCell<Option<Rc<dyn Fn()>>>,
}
ferro_class!(ThemeTarget:StyledElement);
ferro_impl_classes!(ThemeTarget:FerroObjectImpl,StyledElementImpl);
impl ThemeTarget {
    ferro_property!(
        fn first_property() -> DirectProperty<ThemeTarget, Option<Rc<dyn IBrush>>> {
            FerroProperty::register_direct::<ThemeTarget, _>(
                "First",
                |t| t.first.borrow().clone(),
                Some(|t, v| t.assign(true, v)),
                None,
            )
        }
    );
    ferro_property!(
        fn second_property() -> DirectProperty<ThemeTarget, Option<Rc<dyn IBrush>>> {
            FerroProperty::register_direct::<ThemeTarget, _>(
                "Second",
                |t| t.second.borrow().clone(),
                Some(|t, v| t.assign(false, v)),
                None,
            )
        }
    );
    ferro_property!(
        fn third_property() -> DirectProperty<ThemeTarget, Option<Rc<dyn IBrush>>> {
            FerroProperty::register_direct::<ThemeTarget, _>(
                "Third",
                |_| None,
                Some(|t, v| {
                    t.writes.borrow_mut().push(format!(
                        "third:{}",
                        v.as_ref()
                            .and_then(|b| b.as_object())
                            .and_then(|o| o.downcast_ref::<SolidColorBrush>())
                            .map(|b| to_ribbon_color(b.color()).to_hex(true))
                            .unwrap_or("null".into())
                    ));
                }),
                None,
            )
        }
    );
    fn assign(&self, first: bool, value: Option<Rc<dyn IBrush>>) {
        let label = value
            .as_ref()
            .and_then(|b| b.as_object())
            .and_then(|o| o.downcast_ref::<SolidColorBrush>())
            .map(|b| to_ribbon_color(b.color()).to_hex(true))
            .unwrap_or("null".into());
        self.writes.borrow_mut().push(format!(
            "{}:{label}",
            if first { "first" } else { "second" }
        ));
        let reentry = self.reentry.borrow().clone();
        if let Some(reentry) = reentry {
            reentry();
        }
        if first {
            self.first.replace(value);
        } else {
            self.second.replace(value);
        }
    }
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: StyledElement::construct(),
            first: RefCell::new(None),
            second: RefCell::new(None),
            writes: RefCell::new(Vec::new()),
            reentry: RefCell::new(None),
        })
    }
}
#[test]
fn source_runtime_theme_brush_repeated_subscription_order_and_weak_lifetime() {
    run(|| {
        RibbonTheme::reset_test_cache();
        RibbonTheme::apply_with_style(
            Some(RibbonThemePalette::word()),
            RibbonChromeStyle::Neutral,
            RibbonThemeStyle::Office,
        )
        .unwrap();
        let target = ThemeTarget::new();
        RibbonTheme::set_theme(Some(&target), Some(ThemeVariant::light())).unwrap();
        RibbonTheme::set_theme_brush(
            Some(&target),
            Some(ThemeTarget::first_property().as_property()),
            Some("RibbonAccentBrush"),
        )
        .unwrap();
        RibbonTheme::set_theme_brush(
            Some(&target),
            Some(ThemeTarget::first_property().as_property()),
            Some("RibbonBackgroundBrush"),
        )
        .unwrap();
        RibbonTheme::set_theme_brush(
            Some(&target),
            Some(ThemeTarget::second_property().as_property()),
            Some("RibbonAccentBrush"),
        )
        .unwrap();
        assert_eq!(
            json!(target.writes.borrow().clone()),
            corpus()["tracking"]["setup"]
        );
        target.writes.borrow_mut().clear();
        RibbonTheme::set_theme(Some(&target), Some(ThemeVariant::dark())).unwrap();
        assert_eq!(
            json!(target.writes.borrow().clone()),
            corpus()["tracking"]["changed"]
        );
        let weak = target.downgrade();
        drop(target);
        assert_eq!(
            json!(weak.upgrade().is_some()),
            corpus()["tracking"]["targetAlive"]
        );
        RibbonTheme::reset_test_cache();
    });
}
#[test]
fn source_runtime_theme_registration_reentry_and_change_boundaries() {
    run(|| {
        RibbonTheme::reset_test_cache();
        RibbonTheme::apply_with_style(
            Some(RibbonThemePalette::word()),
            RibbonChromeStyle::Neutral,
            RibbonThemeStyle::Office,
        )
        .unwrap();
        let c = corpus();
        for insert in [false, true] {
            let target = ThemeTarget::new();
            RibbonTheme::set_theme(Some(&target), Some(ThemeVariant::light())).unwrap();
            RibbonTheme::set_theme_brush(
                Some(&target),
                Some(ThemeTarget::first_property().as_property()),
                Some("RibbonAccentBrush"),
            )
            .unwrap();
            RibbonTheme::set_theme_brush(
                Some(&target),
                Some(ThemeTarget::second_property().as_property()),
                Some("RibbonAccentBrush"),
            )
            .unwrap();
            target.writes.borrow_mut().clear();
            // Observe actual native direct-property dispatch, including equal
            // writes, just as the source oracle's adapter observes SetValue calls.
            let weak = target.downgrade();
            let once = Rc::new(Cell::new(false));
            let once2 = once.clone();
            target.reentry.replace(Some(Rc::new(move || {
                if !once2.replace(true) {
                    let target = weak.upgrade().unwrap();
                    RibbonTheme::set_theme_brush(
                        Some(&target),
                        Some(if insert {
                            ThemeTarget::third_property().as_property()
                        } else {
                            ThemeTarget::second_property().as_property()
                        }),
                        Some("RibbonBackgroundBrush"),
                    )
                    .unwrap();
                }
            })));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                RibbonTheme::set_theme(Some(&target), Some(ThemeVariant::dark())).unwrap()
            }));
            let expected = &c[if insert {
                "insertionReentry"
            } else {
                "replacement"
            }];
            assert_eq!(result.is_err(), !expected["exception"].is_null());
            assert_eq!(json!(target.writes.borrow().clone()), expected["trace"]);
        }
        let count = Rc::new(Cell::new(0));
        let count2 = count.clone();
        let token = RibbonTheme::changed(move |_, _| count2.set(count2.get() + 1));
        let scope = Border::new();
        RibbonTheme::set_brush_color("RibbonAccentBrush", Color::from_argb(17, 1, 2, 3));
        RibbonTheme::set_theme(Some(&scope), Some(ThemeVariant::light())).unwrap();
        assert_eq!(json!(count.get()), c["noChanges"]);
        RibbonTheme::apply_accent(Color::from_argb(255, 9, 19, 29)).unwrap();
        assert_eq!(
            json!({"state":state(),"color":hex(RibbonTheme::get_brush(Some(&scope),Some("RibbonAccentBrush")).unwrap()),"events":count.get()}),
            c["accent"]
        );
        RibbonTheme::apply_style(RibbonThemeStyle::Cad).unwrap();
        assert_eq!(json!({"state":state(),"events":count.get()}), c["styled"]);
        token.dispose();
        RibbonTheme::reset_test_cache();
    });
}
