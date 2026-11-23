//! Where the bytes of a font come from: `data:`, application assets, `file:` and, behind the
//! feature `http`, `https:`. Applications with another source implement `IFontFaceFetcher`.

use crate::web_font_collection::{FontFetchError, FontFetchResult, IFontFaceFetcher};
use ferroui_base::platform::IAssetLoader;
use ferroui_base::utilities::UriRef;
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::io::Read;
use std::time::Duration;

/// The limits of fetching a font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontFetchOptions {
    /// The largest response, in bytes.
    pub max_size: usize,
    /// The redirects followed.
    pub max_redirects: u32,
    /// The time a whole fetch may take.
    pub timeout: Duration,
    /// Allows `http:`. Off by default: a font is an input to native parsers, and plain HTTP
    /// lets anyone on the network choose it.
    pub allow_insecure_http: bool,
}

impl Default for FontFetchOptions {
    fn default() -> Self {
        Self {
            max_size: ferroui_fonts_web_core::WebFontLimits::DEFAULT.max_encoded_size,
            max_redirects: 5,
            timeout: Duration::from_secs(30),
            allow_insecure_http: false,
        }
    }
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    }
}

fn decode_base64(text: &str, max_size: usize) -> Result<Vec<u8>, FontFetchError> {
    let invalid = || FontFetchError::Network("invalid base64 in a data URL".into());
    let mut output = Vec::with_capacity((text.len() / 4 * 3).min(max_size));
    let (mut bits, mut count) = (0u32, 0u32);
    for byte in text.bytes() {
        if byte == b'=' {
            break;
        }
        if byte.is_ascii_whitespace() {
            continue;
        }
        bits = (bits << 6) | base64_value(byte).ok_or_else(invalid)? as u32;
        count += 6;
        if count >= 8 {
            count -= 8;
            output.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
        if output.len() > max_size {
            return Err(FontFetchError::Forbidden("the font is over the size limit".into()));
        }
    }
    Ok(output)
}

fn decode_percent(text: &str, max_size: usize) -> Result<Vec<u8>, FontFetchError> {
    let bytes = text.as_bytes();
    let mut output = Vec::with_capacity(bytes.len().min(max_size));
    let mut index = 0;
    while index < bytes.len() {
        let mut byte = bytes[index];
        if byte == b'%' {
            let hex = text.get(index + 1..index + 3).and_then(|hex| u8::from_str_radix(hex, 16).ok());
            byte = hex.ok_or_else(|| FontFetchError::Network("invalid escape in a data URL".into()))?;
            index += 2;
        }
        output.push(byte);
        index += 1;
        if output.len() > max_size {
            return Err(FontFetchError::Forbidden("the font is over the size limit".into()));
        }
    }
    Ok(output)
}

/// The bytes of a `data:` URL.
pub(crate) fn decode_data_url(url: &str, max_size: usize) -> FontFetchResult {
    let body = url.get(5..).filter(|_| url[..5].eq_ignore_ascii_case("data:"));
    let body = body.ok_or_else(|| FontFetchError::Network("not a data URL".into()))?;
    let (header, payload) =
        body.split_once(',').ok_or_else(|| FontFetchError::Network("a data URL without data".into()))?;
    if header.to_ascii_lowercase().ends_with(";base64") {
        decode_base64(payload, max_size)
    } else {
        decode_percent(payload, max_size)
    }
}

/// Reads to the end, refusing more than `max_size` bytes, with a bound on what is allocated.
pub(crate) fn read_limited(reader: impl Read, max_size: usize) -> FontFetchResult {
    let mut output = Vec::new();
    let limit = (max_size as u64).saturating_add(1);
    reader.take(limit).read_to_end(&mut output).map_err(|error| FontFetchError::Network(error.to_string()))?;
    if output.len() > max_size {
        return Err(FontFetchError::Forbidden("the font is over the size limit".into()));
    }
    Ok(output)
}

fn read_asset(url: &url::Url, max_size: usize) -> FontFetchResult {
    let loader = FerroLocator::current().get_service::<dyn IAssetLoader>().ok_or(FontFetchError::NotFound)?;
    let stream = loader.open(&UriRef::from(url.clone()), None).map_err(|_| FontFetchError::NotFound)?;
    read_limited(stream, max_size)
}

#[cfg(not(target_family = "wasm"))]
fn read_file(url: &url::Url, max_size: usize) -> FontFetchResult {
    let path = url.to_file_path().map_err(|_| FontFetchError::NotFound)?;
    let file = std::fs::File::open(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => FontFetchError::NotFound,
        _ => FontFetchError::Network(error.to_string()),
    })?;
    read_limited(file, max_size)
}

#[cfg(all(feature = "http", not(target_family = "wasm")))]
pub(crate) fn fetch_http(url: &str, options: &FontFetchOptions) -> FontFetchResult {
    let agent = ureq::AgentBuilder::new()
        .https_only(!options.allow_insecure_http)
        .redirects(options.max_redirects)
        .timeout(options.timeout)
        .build();
    match agent.get(url).call() {
        Ok(response) => {
            let declared = response.header("Content-Length").and_then(|length| length.parse::<usize>().ok());
            if declared.is_some_and(|length| length > options.max_size) {
                return Err(FontFetchError::Forbidden("the font is over the size limit".into()));
            }
            read_limited(response.into_reader(), options.max_size)
        }
        Err(ureq::Error::Status(404 | 410, _)) => Err(FontFetchError::NotFound),
        Err(error) => Err(FontFetchError::Network(error.to_string())),
    }
}

/// Fetches fonts from `data:`, `ferres:`, `resm:`, `file:` and, with the feature `http`, `https:`.
/// What takes time runs on another thread, and the result comes back on the UI thread.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultFontFaceFetcher {
    options: FontFetchOptions,
}

impl DefaultFontFaceFetcher {
    pub fn new(options: FontFetchOptions) -> Self {
        Self { options }
    }
}

impl IFontFaceFetcher for DefaultFontFaceFetcher {
    fn fetch(&self, url: &str, completion: Box<dyn FnOnce(FontFetchResult)>) {
        let options = self.options;
        if url.len() >= 5 && url[..5].eq_ignore_ascii_case("data:") {
            completion(decode_data_url(url, options.max_size));
            return;
        }
        let Ok(parsed) = url::Url::parse(url) else {
            // A relative URL: the page resolves it against its own address.
            #[cfg(all(feature = "browser", target_family = "wasm"))]
            crate::browser_fetcher::fetch(url, options.max_size, completion);
            #[cfg(not(all(feature = "browser", target_family = "wasm")))]
            completion(Err(FontFetchError::NotFound));
            return;
        };
        match parsed.scheme() {
            "ferres" | "resm" => completion(read_asset(&parsed, options.max_size)),
            #[cfg(not(target_family = "wasm"))]
            "file" => {
                let (work, shared) = shared_completion(completion);
                let result = crate::off_thread::run_off_thread(move || read_file(&parsed, options.max_size), work);
                finish_if_not_started(result, shared);
            }
            #[cfg(all(feature = "http", not(target_family = "wasm")))]
            "https" | "http" => {
                if parsed.scheme() == "http" && !options.allow_insecure_http {
                    completion(Err(FontFetchError::Forbidden("plain HTTP is not allowed".into())));
                    return;
                }
                let address = url.to_owned();
                let (work, shared) = shared_completion(completion);
                let result = crate::off_thread::run_off_thread(move || fetch_http(&address, &options), work);
                finish_if_not_started(result, shared);
            }
            #[cfg(all(feature = "browser", target_family = "wasm"))]
            "https" | "http" => {
                if parsed.scheme() == "http" && !options.allow_insecure_http {
                    completion(Err(FontFetchError::Forbidden("plain HTTP is not allowed".into())));
                    return;
                }
                crate::browser_fetcher::fetch(url, options.max_size, completion);
            }
            _ => completion(Err(FontFetchError::Forbidden(format!("the scheme {} is not available", parsed.scheme())))),
        }
    }
}

#[cfg(not(target_family = "wasm"))]
type SharedCompletion = std::rc::Rc<std::cell::RefCell<Option<Box<dyn FnOnce(FontFetchResult)>>>>;

/// The completion, reachable both by the work and by the caller if the thread does not start.
#[cfg(not(target_family = "wasm"))]
fn shared_completion(
    completion: Box<dyn FnOnce(FontFetchResult)>,
) -> (impl FnOnce(FontFetchResult) + 'static, SharedCompletion) {
    let shared: SharedCompletion = std::rc::Rc::new(std::cell::RefCell::new(Some(completion)));
    let for_work = shared.clone();
    let work = move |result| {
        if let Some(completion) = for_work.borrow_mut().take() {
            completion(result);
        }
    };
    (work, shared)
}

#[cfg(not(target_family = "wasm"))]
fn finish_if_not_started(result: std::io::Result<()>, shared: SharedCompletion) {
    if let Err(error) = result {
        if let Some(completion) = shared.borrow_mut().take() {
            completion(Err(FontFetchError::Network(error.to_string())));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_urls() {
        assert_eq!(decode_data_url("data:font/woff2;base64,AQID", 16), Ok(vec![1, 2, 3]));
        assert_eq!(decode_data_url("DATA:font/ttf;BASE64,AQI=", 16), Ok(vec![1, 2]));
        assert_eq!(decode_data_url("data:;base64,AQ\n ID", 16), Ok(vec![1, 2, 3]));
        assert_eq!(decode_data_url("data:text/plain,a%20b", 16), Ok(b"a b".to_vec()));
        for invalid in ["data:", "data:no-comma", "data:;base64,@@@@", "data:,%G1", "data:,%2", "nope"] {
            assert!(decode_data_url(invalid, 16).is_err(), "{invalid}");
        }
    }

    #[test]
    fn a_data_url_over_the_limit_is_refused() {
        let result = decode_data_url("data:;base64,AQIDBAUG", 3);
        assert!(matches!(result, Err(FontFetchError::Forbidden(_))));
        assert!(matches!(decode_data_url("data:,abcd", 3), Err(FontFetchError::Forbidden(_))));
    }

    #[test]
    fn reading_stops_at_the_limit() {
        assert_eq!(read_limited(&[1u8, 2, 3][..], 3), Ok(vec![1, 2, 3]));
        assert!(matches!(read_limited(&[1u8, 2, 3, 4][..], 3), Err(FontFetchError::Forbidden(_))));
    }

    #[cfg(feature = "http")]
    mod http {
        use super::*;
        use std::io::Write;
        use std::net::TcpListener;

        /// A server on the loopback that answers each connection with the next response.
        fn serve(responses: Vec<Vec<u8>>) -> String {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = format!("http://{}/font", listener.local_addr().unwrap());
            std::thread::spawn(move || {
                for response in responses {
                    let Ok((mut stream, _)) = listener.accept() else { return };
                    let mut request = [0u8; 2048];
                    let _ = stream.read(&mut request);
                    let _ = stream.write_all(&response);
                }
            });
            address
        }

        fn ok(body: &[u8]) -> Vec<u8> {
            let mut response =
                format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
            response.extend_from_slice(body);
            response
        }

        fn redirect(to: &str) -> Vec<u8> {
            let head = format!("HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\n");
            format!("{head}Connection: close\r\n\r\n").into_bytes()
        }

        fn insecure() -> FontFetchOptions {
            FontFetchOptions { allow_insecure_http: true, timeout: Duration::from_secs(5), ..Default::default() }
        }

        #[test]
        fn a_response_is_read() {
            let url = serve(vec![ok(b"font bytes")]);
            assert_eq!(fetch_http(&url, &insecure()), Ok(b"font bytes".to_vec()));
        }

        #[test]
        fn plain_http_needs_the_option() {
            let url = serve(vec![ok(b"font")]);
            assert!(fetch_http(&url, &FontFetchOptions::default()).is_err());
        }

        #[test]
        fn a_declared_size_over_the_limit_is_refused() {
            let url = serve(vec![ok(&[0; 64])]);
            let options = FontFetchOptions { max_size: 16, ..insecure() };
            assert!(matches!(fetch_http(&url, &options), Err(FontFetchError::Forbidden(_))));
        }

        #[test]
        fn a_body_over_the_limit_is_refused_without_a_declared_size() {
            let mut response = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
            response.extend_from_slice(&[0; 64]);
            let url = serve(vec![response]);
            let options = FontFetchOptions { max_size: 16, ..insecure() };
            assert!(matches!(fetch_http(&url, &options), Err(FontFetchError::Forbidden(_))));
        }

        #[test]
        fn redirects_are_limited() {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/font", listener.local_addr().unwrap());
            let target = url.clone();
            std::thread::spawn(move || {
                for _ in 0..8 {
                    let Ok((mut stream, _)) = listener.accept() else { return };
                    let mut request = [0u8; 2048];
                    let _ = stream.read(&mut request);
                    let _ = stream.write_all(&redirect(&target));
                }
            });
            let options = FontFetchOptions { max_redirects: 2, ..insecure() };
            assert!(fetch_http(&url, &options).is_err());
        }

        #[test]
        fn a_missing_font_is_not_found() {
            let url = serve(vec![b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()]);
            assert_eq!(fetch_http(&url, &insecure()), Err(FontFetchError::NotFound));
        }
    }
}
