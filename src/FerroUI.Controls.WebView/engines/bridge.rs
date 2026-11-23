//! The scripts that give engines other than WebView2 its page interface: `window.chrome.webview`
//! with `postMessage` and `addEventListener`, and script results as JSON.

use crate::engine::WebViewError;
use crate::json;

/// The script run at the start of every document. `post` is a JavaScript expression of a
/// function taking the message as a JSON string.
#[cfg_attr(windows, allow(dead_code))]
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
#[cfg_attr(windows, allow(dead_code))]
pub(crate) fn deliver_script(message_json: &str) -> String {
    format!(
        "window.chrome.webview.__ferrouiDeliver({});",
        json::quote(message_json)
    )
}

/// `html` with `script` as its first element: after a doctype, which must stay first for the
/// page to keep its rendering mode.
#[cfg(any(target_os = "android", target_arch = "wasm32", test))]
pub(crate) fn with_document_script(html: &str, script: &str) -> String {
    let start = html.len() - html.trim_start().len();
    let rest = &html[start..];
    let is_doctype = rest.get(..9).is_some_and(|head| head.eq_ignore_ascii_case("<!doctype"));
    let insert = if is_doctype { rest.find('>').map_or(start, |end| start + end + 1) } else { start };
    format!("{}<script>{script}</script>{}", &html[..insert], &html[insert..])
}

/// Runs `script` in the global scope. The value is a string for [`parse_result`], so that a
/// script that throws is reported the same way by every engine.
pub(crate) fn result_script(script: &str) -> String {
    format!(
        "(function(){{try{{var text=JSON.stringify((0,eval)({}));\
return 'v'+(text===undefined?'null':text);\
}}catch(error){{return 'e'+String(error);}}}})()",
        json::quote(script)
    )
}

/// The result of a [`result_script`].
pub(crate) fn parse_result(text: &str) -> Result<String, WebViewError> {
    match text.split_at_checked(1) {
        Some(("v", value)) => Ok(value.to_owned()),
        Some(("e", message)) => Err(WebViewError::Engine(message.to_owned())),
        _ => Err(WebViewError::Engine("the script gave no result".to_owned())),
    }
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

    #[test]
    fn the_document_script_comes_first_but_after_a_doctype() {
        assert_eq!(with_document_script("<p>a</p>", "s"), "<script>s</script><p>a</p>");
        assert_eq!(
            with_document_script("  <!DOCTYPE html><p>a</p>", "s"),
            "  <!DOCTYPE html><script>s</script><p>a</p>"
        );
        assert_eq!(with_document_script("", "s"), "<script>s</script>");
        assert_eq!(with_document_script("<!doctype", "s"), "<script>s</script><!doctype");
        assert_eq!(with_document_script("é<p>", "s"), "<script>s</script>é<p>");
    }

    #[test]
    fn a_result_is_a_value_or_the_error_of_the_script() {
        assert_eq!(parse_result("v{\"a\":1}"), Ok("{\"a\":1}".to_owned()));
        assert_eq!(parse_result("eError: boom"), Err(WebViewError::Engine("Error: boom".to_owned())));
        assert!(parse_result("").is_err());
        assert!(parse_result("x1").is_err());
    }
}
