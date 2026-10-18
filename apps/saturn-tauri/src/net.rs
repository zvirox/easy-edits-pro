use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

/// Reject addresses that are not suitable destinations for an Internet import.
/// All resolved addresses are checked, and the accepted set is pinned for the
/// request to prevent DNS rebinding between validation and connection.
pub fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || octets[0] == 0
                || (octets[0] == 100 && (64..=127).contains(&octets[1]))
                || octets[0] >= 240
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_forbidden_ip(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80
        }
    }
}

pub async fn fetch_public_http(start_url: reqwest::Url) -> Result<reqwest::Response, String> {
    let https_only = start_url.scheme() == "https";
    let mut current = start_url;
    for _ in 0..=5 {
        if !matches!(current.scheme(), "http" | "https") {
            return Err("Only http:// and https:// URLs are supported for network import".into());
        }
        if https_only && current.scheme() != "https" {
            return Err("HTTPS downloads cannot redirect to an insecure URL".into());
        }
        let host = current.host_str().ok_or("URL has no host")?.to_owned();
        let port = current.port_or_known_default().ok_or("URL has no port")?;
        let bare_host = host.trim_start_matches('[').trim_end_matches(']');
        let (addresses, domain): (Vec<SocketAddr>, Option<String>) =
            match bare_host.parse::<IpAddr>() {
                Ok(ip) => (vec![SocketAddr::new(ip, port)], None),
                Err(_) => {
                    let addresses = tokio::net::lookup_host((bare_host, port))
                        .await
                        .map_err(|error| format!("Could not resolve `{bare_host}`: {error}"))?
                        .collect::<Vec<_>>();
                    (addresses, Some(bare_host.to_owned()))
                }
            };
        if addresses.is_empty()
            || addresses
                .iter()
                .any(|address| is_forbidden_ip(address.ip()))
        {
            return Err(
                "Requests to localhost, private or link-local networks are not permitted".into(),
            );
        }

        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .redirect(reqwest::redirect::Policy::none());
        if let Some(domain) = domain.as_deref() {
            builder = builder.resolve_to_addrs(domain, &addresses);
        }
        let response = builder
            .build()
            .map_err(|error| error.to_string())?
            .get(current.clone())
            .send()
            .await
            .map_err(|error| format!("Failed to connect to `{current}`: {error}"))?;

        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or("Redirect without a Location header")?;
            current = current
                .join(location)
                .map_err(|error| format!("Invalid redirect target: {error}"))?;
            continue;
        }
        return Ok(response);
    }
    Err("Too many redirects".into())
}

// Kept as a small hostname helper for callers that only have an unparsed host;
// network requests must use fetch_public_http so DNS and redirects are checked.
pub fn is_forbidden_ip_or_host(host: &str) -> bool {
    let normalized = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    normalized == "localhost"
        || normalized.ends_with(".localhost")
        || normalized.parse::<IpAddr>().is_ok_and(is_forbidden_ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_non_public_addresses() {
        for value in [
            "127.0.0.1",
            "127.0.0.2",
            "10.1.2.3",
            "172.16.0.1",
            "172.31.255.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "::1",
            "::",
            "fc00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
        ] {
            assert!(
                is_forbidden_ip(value.parse().unwrap()),
                "{value} should be blocked"
            );
        }
    }

    #[test]
    fn allows_public_addresses() {
        for value in ["8.8.8.8", "1.1.1.1", "172.32.0.1", "2606:4700:4700::1111"] {
            assert!(
                !is_forbidden_ip(value.parse().unwrap()),
                "{value} should be allowed"
            );
        }
    }

    #[test]
    fn rejects_local_hostname_spellings() {
        for host in [
            "localhost",
            "LOCALHOST",
            "[::1]",
            "127.0.0.1",
            "10.0.0.1",
            "studio.localhost",
        ] {
            assert!(is_forbidden_ip_or_host(host), "{host} should be blocked");
        }
        assert!(!is_forbidden_ip_or_host("example.com"));
    }
}
