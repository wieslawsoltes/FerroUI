//! `WideArityObservableFixture` (`TestModels/WideArityObservableFixture.cs`).

use super::ManualObservable;
use ferroui_system::IObservable;
use std::rc::Rc;

notifying_model! {
    /// Twelve properties that hold observables, whose setters always notify
    /// (`WideArityObservableFixture`).
    pub struct WideArityObservableFixture (changing: false, check_equality: false) {
        stream1 / set_stream1: Option<Rc<dyn IObservable<String>>> = None,
        stream2 / set_stream2: Option<Rc<dyn IObservable<String>>> = None,
        stream3 / set_stream3: Option<Rc<dyn IObservable<String>>> = None,
        stream4 / set_stream4: Option<Rc<dyn IObservable<String>>> = None,
        stream5 / set_stream5: Option<Rc<dyn IObservable<String>>> = None,
        stream6 / set_stream6: Option<Rc<dyn IObservable<String>>> = None,
        stream7 / set_stream7: Option<Rc<dyn IObservable<String>>> = None,
        stream8 / set_stream8: Option<Rc<dyn IObservable<String>>> = None,
        stream9 / set_stream9: Option<Rc<dyn IObservable<String>>> = None,
        stream10 / set_stream10: Option<Rc<dyn IObservable<String>>> = None,
        stream11 / set_stream11: Option<Rc<dyn IObservable<String>>> = None,
        stream12 / set_stream12: Option<Rc<dyn IObservable<String>>> = None,
    }
}

#[allow(dead_code)]
impl WideArityObservableFixture {
    /// Fills every stream with a manually driven observable and hands them back in order
    /// (`FillStreams`).
    pub fn fill_streams(&self) -> Vec<Rc<ManualObservable<String>>> {
        let streams: Vec<Rc<ManualObservable<String>>> = (0..12).map(|_| ManualObservable::new()).collect();
        self.set_stream1(Some(streams[0].clone() as Rc<dyn IObservable<String>>));
        self.set_stream2(Some(streams[1].clone() as Rc<dyn IObservable<String>>));
        self.set_stream3(Some(streams[2].clone() as Rc<dyn IObservable<String>>));
        self.set_stream4(Some(streams[3].clone() as Rc<dyn IObservable<String>>));
        self.set_stream5(Some(streams[4].clone() as Rc<dyn IObservable<String>>));
        self.set_stream6(Some(streams[5].clone() as Rc<dyn IObservable<String>>));
        self.set_stream7(Some(streams[6].clone() as Rc<dyn IObservable<String>>));
        self.set_stream8(Some(streams[7].clone() as Rc<dyn IObservable<String>>));
        self.set_stream9(Some(streams[8].clone() as Rc<dyn IObservable<String>>));
        self.set_stream10(Some(streams[9].clone() as Rc<dyn IObservable<String>>));
        self.set_stream11(Some(streams[10].clone() as Rc<dyn IObservable<String>>));
        self.set_stream12(Some(streams[11].clone() as Rc<dyn IObservable<String>>));
        streams
    }
}
