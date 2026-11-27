use crate::WinUITheme;
use ferroui_base::{BoxedValue, Ref};
use ferroui_base::metadata::{IServiceProvider, into_markup_value};
use ferroui_base::utilities::UriRef;
use ferroui_markup_xaml::{RuntimeXamlLoaderDocument, RuntimeXamlLoaderConfiguration, XamlLoadError};
use ferroui_markup_xaml::xaml_il::runtime::compiled as rt;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

pub const THEME_CLASS:&str="FerroUI.Themes.WinUI.WinUITheme";
pub const THEME_UNITS:rt::ThemeParts=&[];
pub fn registered_theme_units()->rt::ThemeParts { THEME_UNITS }

fn integer(data:&mut &[u8])->usize {
    let mut value=0;
    for shift in (0..usize::BITS).step_by(7) {
        let (&byte,rest)=data.split_first().unwrap();
        *data=rest;
        value|=((byte&127) as usize)<<shift;
        if byte<128{return value;}
    }
    panic!("invalid integer")
}

fn escaped(xml:&mut String,value:&str,attribute:bool) {
    for c in value.chars() {
        match c {
            '&'=>xml.push_str("&amp;"), '<'=>xml.push_str("&lt;"), '>'=>xml.push_str("&gt;"),
            '"' if attribute=>xml.push_str("&quot;"), '\r'=>xml.push_str("&#13;"),
            '\n' if attribute=>xml.push_str("&#10;"), '\t' if attribute=>xml.push_str("&#9;"),
            _=>xml.push(c)
        }
    }
}
fn documents()->Vec<(String,String)> {
    let (magic,mut data)=include_bytes!("winui.bin").split_at(5);
    assert_eq!(magic,b"FXTA\x01");
    let count=integer(&mut data);
    let pool:Vec<_>=(0..count).map(|_| {
        let n=integer(&mut data);
        let (v,tail)=data.split_at(n);
        data=tail; std::str::from_utf8(v).unwrap()
    }).collect();
    let count=integer(&mut data);
    let mut docs=Vec::with_capacity(count);
    for _ in 0..count {
        let path=pool[integer(&mut data)];
        let mut xml=String::new();
        let mut stack=Vec::new();
        loop {
            let (&op,tail)=data.split_first().unwrap();data=tail;
            match op {
                0=>{assert!(stack.is_empty()); break;}
                1=>{
                    let n=integer(&mut data);
                    let name=pool[integer(&mut data)];
                    stack.push(name);
                    xml.push('<'); xml.push_str(name);
                    for _ in 0..n {
                        xml.push(' ');xml.push_str(pool[integer(&mut data)]);
                        xml.push_str("=\""); escaped(&mut xml,pool[integer(&mut data)],true);xml.push('"');
                    }
                    xml.push('>');
                }
                2=>{xml.push_str("</");xml.push_str(stack.pop().unwrap());xml.push('>');}
                3=>escaped(&mut xml,pool[integer(&mut data)],false),
                _=>panic!("unknown opcode {op}"),
            }
        }
        docs.push((path.to_owned(),xml));
    }
    assert!(data.is_empty());
    docs
}

pub fn populate_with_parts(sp:Option<Rc<dyn IServiceProvider>>,this:&Ref<WinUITheme>,_:rt::ThemeParts)->Result<(),XamlLoadError> {
    FerroRuntimeXamlLoader::register();
    let start=std::time::Instant::now();
    let documents=documents().into_iter().map(|(name,xml)|{
        let uri=UriRef::parse_absolute(&format!("ferres://FerroUI.Themes.WinUI/{name}")).unwrap();
        let mut doc=RuntimeXamlLoaderDocument::with_base_uri(Some(uri),&xml);
        if name=="WinUITheme.xaml" {
            doc.root_instance=into_markup_value(this.clone());
            doc.service_provider=sp.clone();
        }
        doc.document=Some(name);
        doc
    }).collect::<Vec<_>>();
    eprintln!("decoded {} documents in {:?}",documents.len(),start.elapsed());
    let mut config=RuntimeXamlLoaderConfiguration::new();
    config.local_assembly=Some(&crate::ASSEMBLY);
    FerroRuntimeXamlLoader::load_group(documents,Some(config))?;
    Ok(())
}
pub fn try_load(sp:Option<&Rc<dyn IServiceProvider>>,uri:&str)->Result<Option<BoxedValue>,XamlLoadError> {
    if rt::uri_equals(uri,"ferres://ferroui.themes.winui/WinUITheme.xaml",""){
        return Ok(into_markup_value(WinUITheme::with_registered_parts(sp.cloned())));
    }
    Ok(None)
}
