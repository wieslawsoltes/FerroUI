//! The scripts that give engines other than WebView2 its page interface: `window.chrome.webview`
//! with `postMessage` and `addEventListener`, and script results as JSON.

use crate::json;

/// The script run at the start of every document. `post` is a JavaScript expression of a
/// function taking the message as a JSON string.
pub(crate) fn document_script(post: &str) -> String {
    format!(
        "(function(){{\
if(window.chrome&&window.chrome.webview)return;\
var post={post},listeners=[];\
var webview={{\
postMessage:function(message){{post(JSON.stringify(message));}},\
addEventListener:function(type,listener){{if(type==='message'&&listeners.indexOf(listener)<0)listeners.push(listener);}},\
removeEventListener:function(type,listener){{var i=listeners.indexOf(listener);if(type==='message'&&i>=0)listeners.splice(i,1);}}\
}};\
Object.defineProperty(webview,'__ferrouiDeliver',{{value:function(text){{\
var event={{data:JSON.parse(text),source:webview}};\
listeners.slice().forEach(function(listener){{listener(event);}});\
}}}});\
window.chrome=window.chrome||{{}};\
window.chrome.webview=webview;\
}})();"
    )
}

/// The script that hands the page a message given as JSON.
pub(crate) fn deliver_script(message_json: &str) -> String {
    format!(
        "window.chrome.webview.__ferrouiDeliver({});",
        json::quote(message_json)
    )
}

/// Runs `script` in the global scope and evaluates to its result as a JSON string.
pub(crate) fn result_script(script: &str) -> String {
    format!(
        "(function(){{var text=JSON.stringify((0,eval)({}));return text===undefined?'null':text;}})()",
        json::quote(script)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scripts_carry_their_text_as_json_strings() {
        assert!(document_script("send").contains("var post=send,"));
        assert_eq!(
            deliver_script("{\"a\":\"b\"}"),
            "window.chrome.webview.__ferrouiDeliver(\"{\\\"a\\\":\\\"b\\\"}\");"
        );
        assert!(result_script("say(\"x\")").contains("(0,eval)(\"say(\\\"x\\\")\")"));
    }
}
