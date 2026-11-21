//! Replay the whole-source static EventHandler oracle through the actual service.
use crate::{tests::run, theming::*};
use ferroui_base::{
    BoxedValue,
    data::core::ValueTypes,
    ferro_markup_type,
    metadata::{MarkupDelegate, MarkupInvokeError, MarkupTyped},
    utilities::EventArgs,
};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
ferroui_base::ferro_thread_local! { static TRACE:RefCell<Vec<String>>=const{RefCell::new(Vec::new())}; }
struct Sink {
    name: &'static str,
}
impl PartialEq for Sink {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Sink {
    fn first(&self, sender: Option<BoxedValue>, args: EventArgs) {
        TRACE.with_borrow_mut(|t| {
            t.push(format!(
                "{}:first:{}:{}",
                self.name,
                if sender.is_none() { "True" } else { "False" },
                if args == EventArgs::EMPTY {
                    "True"
                } else {
                    "False"
                }
            ))
        });
    }
    fn second(&self, _: Option<BoxedValue>, _: EventArgs) {
        TRACE.with_borrow_mut(|t| t.push(format!("{}:second", self.name)));
    }
    fn static_(_: Option<BoxedValue>, _: EventArgs) {
        TRACE.with_borrow_mut(|t| t.push("static".into()));
    }
    fn fail(_: Option<BoxedValue>, _: EventArgs) -> Result<(), String> {
        TRACE.with_borrow_mut(|t| t.push("throw".into()));
        Err("InvalidOperationException:event-failed".into())
    }
}
ferro_markup_type!(class Sink as "ThemeEventSink" {
    namespace:"FerroRibbon.Tests",this:Rc<Sink>,handles:[Sink,Rc<Sink>],
    methods:[
        fn First(Option<BoxedValue>,EventArgs)=>|s:&Rc<Sink>,sender:Option<BoxedValue>,args:EventArgs|s.first(sender,args),
        fn Second(Option<BoxedValue>,EventArgs)=>|s:&Rc<Sink>,sender:Option<BoxedValue>,args:EventArgs|s.second(sender,args),
        static fn Static(Option<BoxedValue>,EventArgs)=>Sink::static_,
        static try fn Fail(Option<BoxedValue>,EventArgs)=>Sink::fail,
    ],
});
fn delegate(target: Option<Rc<Sink>>, method: &str) -> MarkupDelegate {
    MarkupDelegate::for_method(
        target.map(|s| s as BoxedValue),
        Sink::MARKUP,
        Sink::MARKUP.find_methods(method).next().unwrap(),
    )
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        RibbonTheme::reset_test_changed();
    }
}
fn prepare() -> Cleanup {
    RibbonTheme::reset_test_cache();
    RibbonTheme::reset_test_changed();
    ValueTypes::register_reference::<Sink>();
    TRACE.with_borrow_mut(Vec::clear);
    Cleanup
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/theme-events.json")).unwrap()
}
#[test]
fn source_changed_null_duplicates_method_identity_and_subsequence_removal() {
    run(|| {
        let _cleanup = prepare();
        let first = Rc::new(Sink { name: "a" });
        let second = Rc::new(Sink { name: "b" });
        let a = delegate(Some(first.clone()), "First");
        let same = delegate(Some(first.clone()), "First");
        let b = delegate(Some(first.clone()), "Second");
        let other = delegate(Some(second.clone()), "First");
        let s1 = delegate(None, "Static");
        let s2 = delegate(None, "Static");
        assert_eq!(
            json!({"same":a==same,"differentMethod":a==b,"differentEqualTarget":a==other,"staticSame":s1==s2,"copy":a.ptr_eq(&same),"targetsEqual":*first==*second}),
            fixture()["equality"]
        );
        let mut rows = Vec::new();
        let mut step = |name: &str, edit: &dyn Fn()| {
            edit();
            let current = RibbonTheme::changed_test_snapshot();
            let count = current.as_ref().map_or(0, |h| h.invocation_list().len());
            TRACE.with_borrow_mut(Vec::clear);
            RibbonTheme::apply_chrome_style(RibbonChromeStyle::Neutral).unwrap();
            let after = RibbonTheme::changed_test_snapshot();
            let same_identity = match (current, after) {
                (None, None) => true,
                (Some(a), Some(b)) => a.ptr_eq(&b),
                _ => false,
            };
            rows.push(json!({"name":name,"count":count,"trace":TRACE.with_borrow(Clone::clone),"sameIdentity":same_identity}));
        };
        step("null-add-remove", &|| {
            RibbonTheme::add_changed(None);
            RibbonTheme::remove_changed(None);
        });
        step("a-b-a", &|| {
            RibbonTheme::add_changed(Some(a.clone()));
            RibbonTheme::add_changed(Some(b.clone()));
            RibbonTheme::add_changed(Some(same.clone()));
        });
        step("remove-last-equal-a", &|| {
            RibbonTheme::remove_changed(Some(delegate(Some(first.clone()), "First")))
        });
        step("remove-missing-equal-valued-target", &|| {
            RibbonTheme::remove_changed(Some(other.clone()))
        });
        RibbonTheme::reset_test_changed();
        let pair = MarkupDelegate::combine(Some(&a), Some(&b)).unwrap();
        step("two-pairs", &|| {
            RibbonTheme::add_changed(Some(pair.clone()));
            RibbonTheme::add_changed(Some(pair.clone()));
        });
        step("remove-last-pair", &|| {
            RibbonTheme::remove_changed(MarkupDelegate::combine(Some(&same), Some(&b)))
        });
        step("remove-whole-pair", &|| {
            RibbonTheme::remove_changed(Some(pair.clone()))
        });
        step("cross-registration-subsequence", &|| {
            RibbonTheme::add_changed(Some(a.clone()));
            RibbonTheme::add_changed(Some(b.clone()));
            RibbonTheme::add_changed(Some(a.clone()));
            RibbonTheme::remove_changed(Some(pair.clone()));
        });
        RibbonTheme::reset_test_changed();
        step("static-equality-remove", &|| {
            RibbonTheme::add_changed(Some(s1.clone()));
            RibbonTheme::remove_changed(Some(s2.clone()));
        });
        assert_eq!(json!(rows), fixture()["rows"]);
        RibbonTheme::add_changed(Some(a));
        RibbonTheme::add_changed(Some(delegate(None, "Fail")));
        RibbonTheme::add_changed(Some(b));
        TRACE.with_borrow_mut(Vec::clear);
        let RibbonThemeApplyError::Handler(error) =
            RibbonTheme::apply_chrome_style(RibbonChromeStyle::Colorful).unwrap_err()
        else {
            panic!("expected handler failure")
        };
        assert_eq!(
            json!({"error":error.to_string(),"trace":TRACE.with_borrow(Clone::clone),"chrome":RibbonTheme::chrome_style().raw()}),
            fixture()["failure"]
        );
        assert_eq!(
            error,
            MarkupInvokeError::Failed("InvalidOperationException:event-failed".into())
        );
    });
}
#[test]
fn source_changed_roots_targets_until_removal_and_tokens_share_the_same_event() {
    run(|| {
        let _cleanup = prepare();
        let sink = Rc::new(Sink { name: "temp" });
        let weak = Rc::downgrade(&sink);
        RibbonTheme::add_changed(Some(delegate(Some(sink), "First")));
        let rooted = weak.upgrade().is_some();
        RibbonTheme::remove_changed(RibbonTheme::changed_test_snapshot());
        let released = weak.upgrade().is_none();
        assert_eq!(
            json!({"rooted":rooted,"released":released}),
            fixture()["lifetime"]
        );
        let first = Rc::new(Sink { name: "a" });
        let handler = delegate(Some(first), "First");
        RibbonTheme::add_changed(Some(handler.clone()));
        let token = RibbonTheme::changed(|_, _| TRACE.with_borrow_mut(|t| t.push("token".into())));
        RibbonTheme::add_changed(Some(handler));
        TRACE.with_borrow_mut(Vec::clear);
        RibbonTheme::apply_chrome_style(RibbonChromeStyle::Neutral).unwrap();
        assert_eq!(
            TRACE.with_borrow(Clone::clone),
            ["a:first:True:True", "token", "a:first:True:True"]
        );
        token.dispose();
        token.dispose();
        TRACE.with_borrow_mut(Vec::clear);
        RibbonTheme::apply_chrome_style(RibbonChromeStyle::Neutral).unwrap();
        assert_eq!(
            TRACE.with_borrow(Clone::clone),
            ["a:first:True:True", "a:first:True:True"]
        );
        // Disposal consumes its closure immediately; an undisposed dropped token
        // leaves the static source subscription installed.
        RibbonTheme::reset_test_changed();
        let payload = Rc::new(());
        let weak = Rc::downgrade(&payload);
        let token = RibbonTheme::changed(move |_, _| {
            std::hint::black_box(&payload);
        });
        assert!(weak.upgrade().is_some());
        token.dispose();
        assert!(weak.upgrade().is_none());
        let payload = Rc::new(());
        let weak = Rc::downgrade(&payload);
        let token = RibbonTheme::changed(move |_, _| {
            std::hint::black_box(&payload);
        });
        drop(token);
        assert!(weak.upgrade().is_some());
        RibbonTheme::reset_test_changed();
        assert!(weak.upgrade().is_none());
        // Removing the source registration releases its callback even when the
        // consumer retains its disposal token, including an owner/token cycle.
        let payload = Rc::new(());
        let weak = Rc::downgrade(&payload);
        let token = RibbonTheme::changed(move |_, _| {
            std::hint::black_box(&payload);
        });
        RibbonTheme::remove_changed(RibbonTheme::changed_test_snapshot());
        assert!(weak.upgrade().is_none());
        token.dispose();
        struct Owner {
            token: RefCell<Option<Rc<dyn ferroui_base::reactive::IDisposable>>>,
        }
        let owner = Rc::new(Owner {
            token: RefCell::new(None),
        });
        let weak = Rc::downgrade(&owner);
        let captured = owner.clone();
        *owner.token.borrow_mut() = Some(RibbonTheme::changed(move |_, _| {
            std::hint::black_box(&captured);
        }));
        drop(owner);
        assert!(weak.upgrade().is_some());
        RibbonTheme::remove_changed(RibbonTheme::changed_test_snapshot());
        assert!(weak.upgrade().is_none());
        // EventArgs.Empty is one source allocation, including across raises.
        let previous = Rc::new(RefCell::new(None::<BoxedValue>));
        let same = Rc::new(std::cell::Cell::new(true));
        let keep = previous.clone();
        let flag = same.clone();
        RibbonTheme::add_changed(Some(MarkupDelegate::new(move |args| {
            let current = args[1].as_ref().unwrap();
            if let Some(old) = keep.borrow().as_ref() {
                flag.set(flag.get() && Rc::ptr_eq(old, current));
            }
            *keep.borrow_mut() = Some(current.clone());
            None
        })));
        RibbonTheme::apply_chrome_style(RibbonChromeStyle::Neutral).unwrap();
        RibbonTheme::apply_chrome_style(RibbonChromeStyle::Neutral).unwrap();
        assert_eq!(json!(same.get()), fixture()["payloadIdentity"]);
    });
}
