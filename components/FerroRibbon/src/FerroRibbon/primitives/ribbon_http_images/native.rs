//! HTTP transport for URI icons. Native image decoding stays on the UI dispatcher.
use super::{MAX_IMAGE_BYTES, TIMEOUT, image_too_large};
use ferroui_base::utilities::UriRef;
use futures_channel::oneshot;
use reqwest::{Client, Url, redirect::Policy};
use std::{io, sync::LazyLock, time::Duration};
use tokio::runtime::{Builder, Runtime};

struct Transport {
    client: Client,
    runtime: Runtime,
}

static TRANSPORT: LazyLock<io::Result<Transport>> = LazyLock::new(|| {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(TIMEOUT)
        .redirect(Policy::limited(10))
        .build()
        .map_err(transport_error)?;
    let runtime = Builder::new_multi_thread()
        .worker_threads(1)
        .thread_name("ferroribbon-http")
        .enable_all()
        .build()?;
    Ok(Transport { client, runtime })
});

pub(in crate::primitives) async fn read_http_image(uri: &UriRef) -> io::Result<Vec<u8>> {
    let url = uri.as_url().expect("resolved HTTP URI").clone();
    let transport = TRANSPORT
        .as_ref()
        .map_err(|error| io::Error::new(error.kind(), error.to_string()))?;
    let (mut sender, receiver) = oneshot::channel();
    let client = transport.client.clone();
    transport.runtime.spawn(async move {
        let result = tokio::select! {
            biased;
            _ = sender.cancellation() => return,
            result = download(client, url) => result,
        };
        let _ = sender.send(result);
    });
    receiver.await.map_err(io::Error::other)?
}

async fn download(client: Client, url: Url) -> io::Result<Vec<u8>> {
    let mut response = client.get(url).send().await.map_err(transport_error)?;
    if !response.status().is_success() {
        return Err(io::Error::other(format!("HTTP {}", response.status())));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_IMAGE_BYTES as u64)
    {
        return Err(image_too_large());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        if chunk.len() > MAX_IMAGE_BYTES - bytes.len() {
            return Err(image_too_large());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn transport_error(error: reqwest::Error) -> io::Error {
    let kind = if error.is_timeout() {
        io::ErrorKind::TimedOut
    } else {
        io::ErrorKind::Other
    };
    io::Error::new(kind, error.without_url())
}
