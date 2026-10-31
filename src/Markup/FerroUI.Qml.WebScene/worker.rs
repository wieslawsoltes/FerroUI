//! Private worker executable. Its stdout is exclusively the framed protocol.
mod abi;
mod framing;
use percent_encoding::percent_decode_str;
use serde_json::{json, Value};
use std::{collections::BTreeMap, ffi::{c_char,c_void}, io::{self}, path::Path, sync::Mutex};

const INDEX:&str="https://qml.invalid/index.html";
const RPC:&str="https://qml.invalid/rpc/";
const MODULES:&str="https://qml.invalid/modules/";

#[derive(Default)]
struct Resources {
    files:BTreeMap<String,String>,
    // The ABI may retry an oversized response. Do not invoke the host twice.
    retry:Option<(String,Vec<u8>)>,
}
fn exchange(value:&Value)->Result<Value,String>{
    framing::send(&mut io::stdout().lock(),value).map_err(|e|e.to_string())?;
    framing::receive(&mut io::stdin().lock()).map_err(|e|e.to_string())
}
impl Resources {
    fn content(&mut self,url:&str)->Option<Vec<u8>>{
        let data=if url==INDEX {b"<!doctype html><html></html>".to_vec()}
        else if let Some(text)=url.strip_prefix(RPC) {
            let (_,encoded)=text.split_once('/')?;
            let request:Value=serde_json::from_slice(&percent_decode_str(encoded).collect::<Vec<_>>()).ok()?;
            let response=exchange(&json!({"host":request})).ok()?;
            serde_json::to_vec(&response).ok()?
        } else if url.starts_with(MODULES) {self.files.get(url)?.as_bytes().to_vec()}
        else{return None};
        if data.len()>framing::LIMIT{return None}
        let mut envelope=vec![0;22];envelope[0]=1;envelope.extend(data);
        Some(envelope)
    }
}
unsafe extern "C" fn load(user:*mut c_void,_kind:u32,url:*const c_char,len:usize,_etag:*const c_char,_etag_len:usize,_time:i64,dst:*mut c_char,capacity:usize)->usize{
    // No panic crosses the native boundary; the process owns the callback data.
    std::panic::catch_unwind(||{
        if url.is_null()||len>framing::LIMIT {return 0}
        let Ok(url)=std::str::from_utf8(unsafe{std::slice::from_raw_parts(url.cast::<u8>(),len)}) else{return 0};
        let Ok(mut resources)=(unsafe{&*user.cast::<Mutex<Resources>>()}).lock() else{return 0};
        let cached=resources.retry.take().filter(|(old,_)|old==url).map(|(_,v)|v);
        let Some(data)=cached.or_else(||resources.content(url)) else{return 0};
        let length=data.len();
        if capacity>=length && !dst.is_null(){unsafe{std::ptr::copy_nonoverlapping(data.as_ptr(),dst.cast::<u8>(),length)}}
        else {resources.retry=Some((url.to_owned(),data));}
        length
    }).unwrap_or(0)
}

fn run(library:&Path)->Result<(),String>{
    let resources=Mutex::new(Resources::default());
    let engine=unsafe{abi::Engine::open(library,load,std::ptr::from_ref(&resources).cast_mut().cast())}?;
    // Keep module helpers in a closure. Source text, imports and values always
    // reach the worker as data; only our own bootstrap is concatenated here.
    let bootstrap=format!("globalThis.__ferroQml = ({})({}); 'ready'",include_str!("bridge.js"),include_str!("modules.js"));
    if engine.eval(&bootstrap)?!="ready"{return Err("WebScene bootstrap failed".into())}
    framing::send(&mut io::stdout(),&json!({"ready":true})).map_err(|e|e.to_string())?;
    loop {
        let command=match framing::receive(&mut io::stdin().lock()){
            Ok(v)=>v,Err(e) if e.kind()==io::ErrorKind::UnexpectedEof=>break,Err(e)=>return Err(e.to_string())
        };
        let result=(|| {
            if let Some(files)=command.get("files").and_then(Value::as_object){
                let mut resources=resources.lock().map_err(|_|"poisoned module table")?;
                for (url,text) in files {
                    let text=text.as_str().ok_or("invalid module source")?;
                    if !url.starts_with(MODULES){return Err("invalid module address".into())}
                    if let Some(old)=resources.files.get(url) {
                        if old!=text{return Err(format!("conflicting module source for {url}"))}
                    }else{resources.files.insert(url.clone(),text.into());}
                }
            }
            if let Some(graph)=command.get("prepare"){
                engine.eval(&format!("__ferroQml.prepare({graph}); 'pending'"))?;
                loop {
                    let status=engine.eval("__ferroQml.preparation()")?;
                    if status=="ready"{break}
                    if status!="pending"{return Err(status)}
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
            let script=format!("__ferroQml.run({command})");
            let answer=engine.eval(&script)?;
            serde_json::from_str::<Value>(&answer).map_err(|err|err.to_string())
        })();
        let frame=match result {Ok(answer)=>json!({"result":answer}),Err(error)=>json!({"failure":error})};
        framing::send(&mut io::stdout().lock(),&frame).map_err(|e|e.to_string())?;
    }
    Ok(())
}
fn main(){
    let mut args=std::env::args_os().skip(1);
    let Some(library)=args.next() else{eprintln!("missing WebScene native library path");std::process::exit(2)};
    if let Err(error)=run(Path::new(&library)){eprintln!("WebScene worker: {error}");std::process::exit(1)}
}
