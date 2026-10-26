use crate::model::{ensure, ErrorCode, HostError, Result};

fn parse_endpoint(value: &str) -> Result<url::Url> {
    let url = url::Url::parse(value).map_err(HostError::invalid)?;
    let loopback = url.host_str().is_some_and(|host| {
        host == "localhost"
            || host
                .trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    ensure(
        url.host_str().is_some()
            && (url.scheme() == "https" || (url.scheme() == "http" && loopback)),
        ErrorCode::Invalid,
        "Remote collaboration requires HTTPS; HTTP is allowed only on loopback",
    )?;
    ensure(
        url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
            && url.query().is_none(),
        ErrorCode::Invalid,
        "Endpoint must not contain URL credentials, query or fragment",
    )?;
    Ok(url)
}

/// Validate a peer endpoint without opening a socket or writing credentials.
pub fn validate_endpoint_url(value: &str) -> Result<()> {
    parse_endpoint(value).map(|_| ())
}

/// Public invitations require TLS even when the service itself listens on loopback.
pub fn validate_public_endpoint_url(value: &str) -> Result<()> {
    let url = parse_endpoint(value)?;
    ensure(
        url.scheme() == "https",
        ErrorCode::Invalid,
        "Public sharing endpoint must use HTTPS",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_and_host_configuration_share_one_endpoint_policy() {
        for value in ["https://design.example.org/mcp", "https://[::1]:7491/mcp"] {
            validate_public_endpoint_url(value).unwrap();
            validate_endpoint_url(value).unwrap();
        }
        for value in ["http://127.0.0.1:7491/mcp", "http://[::1]:7491/mcp"] {
            validate_endpoint_url(value).unwrap();
            assert!(validate_public_endpoint_url(value).is_err());
        }
        for value in [
            "https://",
            "http://design.example.org/mcp",
            "file:///mcp",
            "https://user:secret@example.org/mcp",
            "https://example.org/mcp?key=value",
            "https://example.org/mcp#fragment",
        ] {
            assert!(validate_endpoint_url(value).is_err());
            assert!(validate_public_endpoint_url(value).is_err());
        }
    }
}
