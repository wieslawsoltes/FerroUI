//! `CombineLatestObservable`.

use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// A tuple of 2 to 16 sequences whose latest values combine into a tuple, once each has produced
/// one, and again on each later value of any of them (the arities of upstream's
/// `CombineLatestObservable.Create`).
pub trait CombineLatestSources: 'static {
    /// The tuple of the latest values.
    type Values: Clone + 'static;

    /// The latest values, combined (`CombineLatest`).
    fn combine_latest(self) -> Rc<dyn IObservable<Self::Values>>;
}

/// Combines the latest value of each of 2 to 16 sequences (`CombineLatestObservable`).
///
/// It delegates to the `combine_latest` operators of `ferroui-reactive`, as upstream delegates to
/// its own engine.
pub struct CombineLatestObservable;

impl CombineLatestObservable {
    /// The latest values of `sources` (a tuple of sequences) combined by `result_selector`, which
    /// takes them as one tuple (`Create<T1, ..., TResult>`).
    pub fn create<S: CombineLatestSources, R: 'static>(sources: S, result_selector: impl Fn(S::Values) -> R + 'static) -> Rc<dyn IObservable<R>> {
        sources.combine_latest().select(result_selector)
    }

    /// The latest values of `sources` as a tuple.
    pub fn create_tuple<S: CombineLatestSources>(sources: S) -> Rc<dyn IObservable<S::Values>> {
        sources.combine_latest()
    }
}

macro_rules! combine_latest_sources {
    ($method:ident; $first_type:ident $first:ident $(, $type_:ident $source:ident)+) => {
        impl<$first_type: Clone + 'static $(, $type_: Clone + 'static)+> CombineLatestSources
            for (Rc<dyn IObservable<$first_type>> $(, Rc<dyn IObservable<$type_>>)+)
        {
            type Values = ($first_type $(, $type_)+);

            fn combine_latest(self) -> Rc<dyn IObservable<Self::Values>> {
                let ($first $(, $source)+) = self;
                $first.$method($($source),+)
            }
        }
    };
}

combine_latest_sources!(combine_latest_tuple; T1 s1, T2 s2);
combine_latest_sources!(combine_latest3_tuple; T1 s1, T2 s2, T3 s3);
combine_latest_sources!(combine_latest4_tuple; T1 s1, T2 s2, T3 s3, T4 s4);
combine_latest_sources!(combine_latest5_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5);
combine_latest_sources!(combine_latest6_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6);
combine_latest_sources!(combine_latest7_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6, T7 s7);
combine_latest_sources!(combine_latest8_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6, T7 s7, T8 s8);
combine_latest_sources!(combine_latest9_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6, T7 s7, T8 s8, T9 s9);
combine_latest_sources!(combine_latest10_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6, T7 s7, T8 s8, T9 s9, T10 s10);
combine_latest_sources!(combine_latest11_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6, T7 s7, T8 s8, T9 s9, T10 s10, T11 s11);
combine_latest_sources!(combine_latest12_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6, T7 s7, T8 s8, T9 s9, T10 s10, T11 s11, T12 s12);
combine_latest_sources!(combine_latest13_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6, T7 s7, T8 s8, T9 s9, T10 s10, T11 s11, T12 s12, T13 s13);
combine_latest_sources!(combine_latest14_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6, T7 s7, T8 s8, T9 s9, T10 s10, T11 s11, T12 s12, T13 s13, T14 s14);
combine_latest_sources!(combine_latest15_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6, T7 s7, T8 s8, T9 s9, T10 s10, T11 s11, T12 s12, T13 s13, T14 s14, T15 s15);
combine_latest_sources!(combine_latest16_tuple; T1 s1, T2 s2, T3 s3, T4 s4, T5 s5, T6 s6, T7 s7, T8 s8, T9 s9, T10 s10, T11 s11, T12 s12, T13 s13, T14 s14, T15 s15, T16 s16);
