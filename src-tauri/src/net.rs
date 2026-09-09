//! Outbound HTTP: SSRF-hardened fetch for untrusted URLs, and a client for known APIs.
//!
//! Spec §6.3 / ADR-aligned split:
//! - [`fetch_untrusted`] — listing pages & enrichment URLs (scheme/IP/size/type guards).
//! - [`api_client`] — SerpAPI / Brave and other hardcoded API hosts (auth headers OK).

use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_TYPE, LOCATION};
use reqwest::redirect::Policy;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use std::time::Duration;

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_REDIRECTS: usize = 5;
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024; // 2 MiB

const API_HOSTS: &[&str] = &["serpapi.com", "api.search.brave.com"];

#[cfg(test)]
thread_local! {
    static ALLOW_LOOPBACK_FOR_TESTS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn allow_loopback_for_tests(allow: bool) {
    ALLOW_LOOPBACK_FOR_TESTS.with(|c| c.set(allow));
}

fn loopback_allowed() -> bool {
    #[cfg(test)]
    {
        return ALLOW_LOOPBACK_FOR_TESTS.with(|c| c.get());
    }
    #[cfg(not(test))]
    {
        false
    }
}

/// Shared timeouts / user-agent for trusted API hosts. Caller attaches auth.
pub fn api_client() -> Result<Client, String> {
    Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .redirect(Policy::limited(5))
        .build()
        .map_err(|e| e.to_string())
}

/// Reject URLs whose host is not an allowlisted API endpoint.
pub fn assert_api_host(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid API URL: {e}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "API URL missing host".to_string())?
        .to_lowercase();
    if API_HOSTS.iter().any(|h| host == *h || host.ends_with(&format!(".{h}"))) {
        Ok(())
    } else {
        Err(format!("Host not on API allowlist: {host}"))
    }
}

fn is_forbidden_ipv4(ip: Ipv4Addr) -> bool {
    if loopback_allowed() && ip.is_loopback() {
        return false;
    }
    let o = ip.octets();
    let cgnat = o[0] == 100 && (64..=127).contains(&o[1]);
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_unspecified()
        || cgnat
}

fn is_forbidden_ipv6(ip: Ipv6Addr) -> bool {
    if loopback_allowed() && ip.is_loopback() {
        return false;
    }
    if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
        return true;
    }
    // Unique local (fc00::/7), link-local (fe80::/10)
    let segments = ip.segments();
    let first = segments[0];
    if (first & 0xfe00) == 0xfc00 {
        return true;
    }
    if (first & 0xffc0) == 0xfe80 {
        return true;
    }
    // IPv4-mapped
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_forbidden_ipv4(v4);
    }
    false
}

pub fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_forbidden_ipv4(v4),
        IpAddr::V6(v6) => is_forbidden_ipv6(v6),
    }
}

/// Scheme + DNS resolve + private/loopback IP checks (spec §6.3).
pub fn validate_url_for_untrusted_fetch(url: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => return Err(format!("Scheme not allowed: {other}")),
    }
    let host = parsed
        .host()
        .ok_or_else(|| "URL missing host".to_string())?;

    match host {
        url::Host::Ipv4(ip) => {
            if is_forbidden_ip(IpAddr::V4(ip)) {
                return Err(format!("Forbidden IP address: {ip}"));
            }
            return Ok(parsed);
        }
        url::Host::Ipv6(ip) => {
            if is_forbidden_ip(IpAddr::V6(ip)) {
                return Err(format!("Forbidden IP address: {ip}"));
            }
            return Ok(parsed);
        }
        url::Host::Domain(domain) => {
            let port = parsed.port_or_known_default().unwrap_or(80);
            let addrs = (domain, port)
                .to_socket_addrs()
                .map_err(|e| format!("DNS resolve failed for {domain}: {e}"))?;
            let mut saw_any = false;
            for addr in addrs {
                saw_any = true;
                if is_forbidden_ip(addr.ip()) {
                    return Err(format!("Forbidden IP address for {domain}: {}", addr.ip()));
                }
            }
            if !saw_any {
                return Err(format!("DNS resolve returned no addresses for {domain}"));
            }
            Ok(parsed)
        }
    }
}

fn content_type_allowed(content_type: &str) -> bool {
    let main = content_type.split(';').next().unwrap_or("").trim().to_lowercase();
    main.starts_with("text/")
        || main == "application/json"
        || main == "application/ld+json"
        || main == "application/xhtml+xml"
        || main == "application/xml"
        || main == "application/javascript" // rare; still text-ish
}

fn read_body_capped(mut res: Response) -> Result<String, String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = res
            .read(&mut chunk)
            .map_err(|e| format!("Failed reading response body: {e}"))?;
        if n == 0 {
            break;
        }
        if buf.len() + n > MAX_BODY_BYTES {
            return Err(format!("Response body exceeds {MAX_BODY_BYTES} bytes"));
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    String::from_utf8(buf).map_err(|_| "Response body is not valid UTF-8".to_string())
}

#[derive(Debug, Clone)]
pub struct UntrustedFetchResult {
    pub final_url: String,
    pub status: u16,
    pub body: String,
}

fn untrusted_client() -> Result<Client, String> {
    Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .redirect(Policy::none()) // we follow manually to re-validate each hop
        .build()
        .map_err(|e| e.to_string())
}

/// Fetch an attacker-influenced URL with SSRF guards. No auth, no cookies.
pub fn fetch_untrusted(url: &str) -> Result<UntrustedFetchResult, String> {
    let client = untrusted_client()?;
    let mut current = validate_url_for_untrusted_fetch(url)?.to_string();

    for hop in 0..=MAX_REDIRECTS {
        let res = client
            .get(&current)
            .send()
            .map_err(|e| format!("Request failed: {e}"))?;
        let status = res.status();
        let code = status.as_u16();

        if status.is_redirection() {
            if hop == MAX_REDIRECTS {
                return Err("Too many redirects".into());
            }
            let loc = res
                .headers()
                .get(LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| "Redirect without Location".to_string())?;
            let next = url::Url::parse(&current)
                .ok()
                .and_then(|base| base.join(loc).ok())
                .or_else(|| url::Url::parse(loc).ok())
                .ok_or_else(|| format!("Invalid redirect Location: {loc}"))?;
            // No scheme downgrade to non-http(s); validate target (re-check private IPs).
            current = validate_url_for_untrusted_fetch(next.as_str())?.to_string();
            continue;
        }

        let ct = res
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html");
        if !content_type_allowed(ct) {
            return Err(format!("Content-Type not allowed: {ct}"));
        }

        let body = read_body_capped(res)?;
        return Ok(UntrustedFetchResult {
            final_url: current,
            status: code,
            body,
        });
    }

    Err("Too many redirects".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::thread;

    #[test]
    fn rejects_file_scheme() {
        let err = validate_url_for_untrusted_fetch("file:///etc/passwd").unwrap_err();
        assert!(err.contains("Scheme"), "{err}");
    }

    #[test]
    fn rejects_ftp_scheme() {
        let err = validate_url_for_untrusted_fetch("ftp://example.com/x").unwrap_err();
        assert!(err.contains("Scheme"), "{err}");
    }

    #[test]
    fn rejects_loopback_literal() {
        let err = validate_url_for_untrusted_fetch("http://127.0.0.1/").unwrap_err();
        assert!(err.contains("Forbidden"), "{err}");
    }

    #[test]
    fn rejects_metadata_link_local() {
        let err = validate_url_for_untrusted_fetch("http://169.254.169.254/latest/meta-data/").unwrap_err();
        assert!(err.contains("Forbidden"), "{err}");
    }

    #[test]
    fn rejects_ipv6_loopback() {
        let err = validate_url_for_untrusted_fetch("http://[::1]/").unwrap_err();
        assert!(err.contains("Forbidden"), "{err}");
    }

    #[test]
    fn rejects_rfc1918() {
        let err = validate_url_for_untrusted_fetch("http://10.0.0.1/").unwrap_err();
        assert!(err.contains("Forbidden"), "{err}");
    }

    #[test]
    fn rejects_cgnat() {
        assert!(is_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
    }

    #[test]
    fn api_host_allowlist() {
        assert!(assert_api_host("https://serpapi.com/search.json").is_ok());
        assert!(assert_api_host("https://api.search.brave.com/res/v1/web/search").is_ok());
        assert!(assert_api_host("https://evil.example/api").is_err());
    }

    #[test]
    fn content_type_gate() {
        assert!(content_type_allowed("text/html; charset=utf-8"));
        assert!(content_type_allowed("application/json"));
        assert!(!content_type_allowed("application/octet-stream"));
        assert!(!content_type_allowed("image/png"));
    }

    fn spawn_http_server(handler: fn(&str) -> Vec<u8>) -> (u16, thread::JoinHandle<()>) {
        static PORT: AtomicU16 = AtomicU16::new(0);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        PORT.store(port, Ordering::SeqCst);
        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let req = String::from_utf8_lossy(&buf);
                let body = handler(&req);
                let _ = stream.write_all(&body);
            }
        });
        (port, handle)
    }

    #[test]
    fn rejects_redirect_to_private() {
        allow_loopback_for_tests(true);
        let (port, join) = spawn_http_server(|_| {
            b"HTTP/1.1 302 Found\r\nLocation: http://169.254.169.254/\r\nContent-Length: 0\r\n\r\n"
                .to_vec()
        });
        let url = format!("http://127.0.0.1:{port}/");
        let err = fetch_untrusted(&url).unwrap_err();
        allow_loopback_for_tests(false);
        let _ = join.join();
        assert!(err.contains("Forbidden") || err.contains("169.254"), "{err}");
    }

    #[test]
    fn rejects_non_text_content_type() {
        allow_loopback_for_tests(true);
        let (port, join) = spawn_http_server(|_| {
            b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 3\r\n\r\nbin"
                .to_vec()
        });
        let url = format!("http://127.0.0.1:{port}/");
        let err = fetch_untrusted(&url).unwrap_err();
        allow_loopback_for_tests(false);
        let _ = join.join();
        assert!(err.contains("Content-Type"), "{err}");
    }

    #[test]
    fn rejects_oversized_body() {
        allow_loopback_for_tests(true);
        let (port, join) = spawn_http_server(|_| {
            let payload = vec![b'a'; MAX_BODY_BYTES + 64];
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                payload.len()
            );
            let mut out = header.into_bytes();
            out.extend_from_slice(&payload);
            out
        });
        let url = format!("http://127.0.0.1:{port}/");
        let err = fetch_untrusted(&url).unwrap_err();
        allow_loopback_for_tests(false);
        let _ = join.join();
        assert!(err.contains("exceeds"), "{err}");
    }

    #[test]
    fn fetch_ok_text() {
        allow_loopback_for_tests(true);
        let (port, join) = spawn_http_server(|_| {
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 5\r\n\r\nhello".to_vec()
        });
        let url = format!("http://127.0.0.1:{port}/");
        let res = fetch_untrusted(&url).unwrap();
        allow_loopback_for_tests(false);
        let _ = join.join();
        assert_eq!(res.body, "hello");
        assert_eq!(res.status, 200);
    }
}
