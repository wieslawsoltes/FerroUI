//! The probe is invoked by an application Web Worker, exactly as a compiled
//! application creates a QML instance with its browser JavaScript backend.
#![cfg(target_arch = "wasm32")]
use ferroui_qml::{Instance, TypeRegistry, Value};
use ferroui_qml_webscene::WebSceneEngine;
use std::{rc::Rc, time::Duration};
use wasm_bindgen::prelude::*;
include!(concat!(env!("OUT_DIR"), "/browser.rs"));

#[wasm_bindgen]
pub fn has_browser_host() -> bool {
    WebSceneEngine::browser().is_ok()
}

#[wasm_bindgen]
pub async fn probe() -> Result<String, JsValue> {
    exercise()
        .await
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

async fn exercise() -> Result<String, ferroui_qml::Error> {
    let engine = Rc::new(WebSceneEngine::browser()?.with_timeout(Duration::from_secs(3)));
    engine.prepare_browser_instance().await?;
    let source = compiled_qml();
    let registry = TypeRegistry::new();
    let mut app = Instance::with_engine(&source, &registry, Default::default(), engine.clone())?;
    let root = app.root();
    fn check(ok: bool, what: &str) -> Result<(), ferroui_qml::Error> {
        if ok {
            Ok(())
        } else {
            Err(ferroui_qml::Error::new(what))
        }
    }
    check(
        app.get(root, "total")? == Value::Number(11.),
        "initial module binding",
    )?;
    check(
        app.call(root, "callNested", vec![])? == Value::Number(30.),
        "nested method and host invalidation",
    )?;
    app.call(root, "makeBinding", vec![])?;
    app.set(root, "seed", Value::Number(6.))?;
    check(
        app.get(root, "total")? == Value::Number(46.),
        "retained closure binding",
    )?;
    check(
        app.call(root, "fail", vec![]).is_err() && app.get(root, "seed")? == Value::Number(6.),
        "exception rollback",
    )?;
    check(
        app.call_and_await(root, "later", vec![], Duration::from_secs(1))? == Value::Number(48.),
        "promise settlement",
    )?;
    let text = "λ😀".repeat(30_000);
    check(
        app.call(root, "wide", vec![Value::String(text.clone())])? == Value::String(text + "!"),
        "chunked UTF-8 frame",
    )?;
    engine.prepare_browser_instance().await?;
    let other = Instance::with_engine(&source, &registry, Default::default(), engine.clone())?;
    check(
        other.get(other.root(), "total")? == Value::Number(11.),
        "independent browser realm",
    )?;
    let timeout = app
        .call(root, "spin", vec![])
        .expect_err("runaway guest returned");
    check(
        timeout.message.contains("deadline exceeded"),
        "guest deadline",
    )?;
    engine.prepare_browser_instance().await?;
    check(
        app.call(root, "wide", vec![Value::String("new realm".into())])?
            == Value::String("new realm!".into()),
        "fresh worker after guest deadline",
    )?;
    Ok("passed".into())
}

#[wasm_bindgen(
    inline_js = "export function nextTurn() { return new Promise(resolve => setTimeout(resolve, 1)); }"
)]
extern "C" {
    #[wasm_bindgen(js_name = nextTurn)]
    async fn next_turn();
}

#[wasm_bindgen]
pub async fn probe_fetch(url: String) -> Result<(), JsValue> {
    use ferroui_qml::network::BrowserComponentTransport;
    let mut registry = TypeRegistry::new();
    registry.set_component_transport(Rc::new(BrowserComponentTransport::default()));
    let err = |error: ferroui_qml::Error| JsValue::from_str(&error.to_string());
    let engine = Rc::new(WebSceneEngine::browser().map_err(err)?);
    engine.prepare_browser_instance().await.map_err(err)?;
    let doc = ferroui_qml::ir::parse_external("import QtQml\nQtObject {}").unwrap();
    let mut model =
        Instance::with_engine(&doc, &registry, Default::default(), engine).map_err(err)?;
    let factory = model
        .create_component(model.root(), &url, None)
        .map_err(err)?;
    while model.has_pending_component_loads() {
        next_turn().await;
        model.poll_component_loads(8).map_err(err)?;
    }
    if model.get(factory, "status").map_err(err)? != Value::Number(1.) {
        return Err(JsValue::from_str(&format!(
            "remote component error: {:?}",
            model.call(factory, "errorString", vec![])
        )));
    }
    let item = model
        .create_object(factory, None, Default::default())
        .map_err(err)?;
    if model.get(item, "value").map_err(err)? != Value::Number(42.) {
        return Err(JsValue::from_str(
            "browser transport did not instantiate the fetched component",
        ));
    }
    Ok(())
}
