//! Native HTTP(S) transport. An application opts in to each permitted origin.
use super::{ComponentReply, ComponentTransport, ResourceError, MAX_DOCUMENT_BYTES};
use std::{
    collections::HashSet,
    io::Read,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use url::Url;

pub struct HttpComponentTransport {
    agent: ureq::Agent,
    origins: HashSet<url::Origin>,
    active: Arc<AtomicUsize>,
}

impl HttpComponentTransport {
    pub fn new(origins: &[Url]) -> Result<Self, crate::Error> {
        if origins.iter().any(|url| {
            !matches!(url.scheme(), "http" | "https")
                || url.host().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
        }) {
            return Err(crate::Error::new(
                "component origins must be HTTP(S) URLs without user information",
            ));
        }
        Ok(Self {
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(15))
                .redirects(0)
                .build(),
            origins: origins.iter().map(Url::origin).collect(),
            active: Default::default(),
        })
    }
}

impl ComponentTransport for HttpComponentTransport {
    fn request(&self, url: &Url, reply: ComponentReply) {
        if !self.origins.contains(&url.origin())
            || !url.username().is_empty()
            || url.password().is_some()
        {
            reply.complete(Err(ResourceError::Failed(
                "origin is not enabled for component loading".into(),
            )));
            return;
        }
        // A cancelled transfer can still be inside TLS or a system read until its timeout.
        if self.active.fetch_add(1, Ordering::Relaxed) >= 16 {
            self.active.fetch_sub(1, Ordering::Relaxed);
            reply.complete(Err(ResourceError::Failed(
                "HTTP component transport is busy".into(),
            )));
            return;
        }
        struct Active(Arc<AtomicUsize>);
        impl Drop for Active {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::Relaxed);
            }
        }
        let guard = Active(self.active.clone());
        let agent = self.agent.clone();
        let url = url.clone();
        // If thread creation fails, dropping the closure drops the guard and completes the reply.
        let _ = std::thread::Builder::new()
            .name("qml-http".into())
            .spawn(move || {
                let _active = guard;
                if !reply.is_pending() {
                    return;
                }
                let result = match agent.get(url.as_str()).call() {
                    Ok(response) if (200..300).contains(&response.status()) => {
                        let mut bytes = Vec::new();
                        response
                            .into_reader()
                            .take(MAX_DOCUMENT_BYTES as u64 + 1)
                            .read_to_end(&mut bytes)
                            .map(|_| bytes)
                            .map_err(|e| ResourceError::Failed(e.to_string()))
                    }
                    Ok(response) => {
                        Err(ResourceError::Failed(format!("HTTP {}", response.status())))
                    }
                    Err(ureq::Error::Status(404, _)) => Err(ResourceError::NotFound),
                    Err(e) => Err(ResourceError::Failed(e.to_string())),
                };
                reply.complete(result);
            });
    }
}
